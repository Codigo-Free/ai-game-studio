//! Multi-agent orchestration (milestone M20): a high-level instruction is
//! first broken into steps by an "Architect" planner, each step then runs
//! through a scoped specialist (`AgentKind`, defined in `ai.rs` alongside
//! the `ChangeProposal` type it scopes), and the resulting per-step
//! proposals are merged into one. This reuses M19's `ChangeProposal` type,
//! its Rust-side validation and the frontend's apply/undo path unchanged —
//! see `docs/arquitectura.md` for why this is a deterministic two-phase
//! pipeline rather than a multi-turn tool-calling agent loop.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::ai::{
    build_scoped_agent_prompt, collect_entity_ids, parse_and_validate_proposal, AgentKind,
    AssetRef, ChangeProposal, ChatMessage, Provider,
};

/// One step of an `OrchestrationPlan`: run `instruction` through `agent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub agent: AgentKind,
    pub instruction: String,
}

#[derive(Debug, Clone, Deserialize)]
struct OrchestrationPlan {
    summary: String,
    #[serde(default)]
    steps: Vec<AgentStep>,
}

/// Keeps a runaway plan (or a confused model looping the same step) from
/// turning one user instruction into dozens of sequential model calls.
const MAX_STEPS: usize = 8;

fn agent_roster_description() -> String {
    [
        AgentKind::Architect,
        AgentKind::LevelDesigner,
        AgentKind::Programmer,
        AgentKind::Physics,
        AgentKind::Audio,
        AgentKind::Animator,
    ]
    .iter()
    .map(|agent| {
        format!(
            "- {}: owns {:?}.",
            match agent {
                AgentKind::Architect => "architect (creates/places basic entities)",
                AgentKind::LevelDesigner =>
                    "level_designer (composes several entities into a level layout)",
                AgentKind::Programmer => "programmer (writes rhai scripts and behaviors)",
                AgentKind::Physics => "physics (rigidbody2d/collider2d and scene gravity)",
                AgentKind::Audio => "audio (scene music and play_sound behaviors)",
                AgentKind::Animator =>
                    "animator (wires up EXISTING scene animations, does not author new ones)",
            },
            agent.allowed_component_keys()
        )
    })
    .collect::<Vec<_>>()
    .join("\n")
}

fn build_planning_prompt(context: &str) -> String {
    format!(
        "You are the \"Architect\", coordinating a team of specialists to fulfill a \
         high-level instruction about a game project in AI Game Studio. Break the \
         instruction into an ordered list of steps, each assigned to exactly one \
         specialist from this roster:\n\n{roster}\n\n\
         Respond with ONLY a single JSON object — no markdown code fences, no commentary \
         before or after it:\n\
         {{\n\
         \x20 \"summary\": \"one-line description of the overall plan\",\n\
         \x20 \"steps\": [ {{ \"agent\": \"architect\"|\"level_designer\"|\"programmer\"|\"physics\"|\"audio\"|\"animator\", \"instruction\": \"concrete instruction for that specialist\" }} ]\n\
         }}\n\n\
         Keep the plan as short as possible: only include steps that are actually needed \
         (at most {MAX_STEPS}). Steps run in order, so a later step may refer to an entity \
         an earlier step creates.\n\n\
         Current project/scene state:\n{context}",
        roster = agent_roster_description(),
    )
}

/// Runs the Architect's plan for `instruction`, then each specialist step
/// in order, merging every validated step proposal into one. Sequential
/// (not parallel/concurrent) on purpose: each step's known ids/assets grow
/// to include what earlier steps just proposed, so step 2 can reference an
/// entity step 1 creates, and two steps can't silently collide on the same
/// new id. All or nothing — if the plan or any single step fails to
/// validate, the whole orchestration is rejected with a message naming
/// which step/agent failed; nothing partial is ever returned.
pub async fn orchestrate(
    provider: &Provider,
    context: &str,
    instruction: String,
    known_assets: Vec<AssetRef>,
    known_entity_ids: Vec<String>,
    known_animation_names: Vec<String>,
) -> Result<ChangeProposal, String> {
    let planning_messages = [
        ChatMessage {
            role: "system".to_string(),
            content: build_planning_prompt(context),
        },
        ChatMessage {
            role: "user".to_string(),
            content: instruction,
        },
    ];
    let raw_plan = provider
        .chat(&planning_messages, true)
        .await
        .map_err(|e| e.to_string())?;
    let plan = parse_plan(&raw_plan)?;

    let mut known_asset_ids: Vec<String> = known_assets.iter().map(|a| a.id.clone()).collect();
    let mut known_entity_ids = known_entity_ids;

    let mut merged = ChangeProposal {
        summary: plan.summary.clone(),
        entities_to_add: Vec::new(),
        entities_to_update: Vec::new(),
        entities_to_remove: Vec::new(),
        scripts: Vec::new(),
        scene_patch: Default::default(),
    };
    let mut step_summaries = Vec::new();

    for (index, step) in plan.steps.iter().enumerate() {
        let system = build_scoped_agent_prompt(
            step.agent,
            context,
            &known_assets,
            &known_entity_ids,
            &known_animation_names,
        );
        let messages = [
            ChatMessage {
                role: "system".to_string(),
                content: system,
            },
            ChatMessage {
                role: "user".to_string(),
                content: step.instruction.clone(),
            },
        ];
        let raw = provider
            .chat(&messages, true)
            .await
            .map_err(|e| format!("step {} ({}) failed: {e}", index + 1, step.agent.label()))?;
        let step_proposal = parse_and_validate_proposal(
            &raw,
            &known_asset_ids,
            &known_entity_ids,
            &known_animation_names,
            Some(step.agent),
        )
        .map_err(|e| format!("step {} ({}) {e}", index + 1, step.agent.label()))?;

        for add in &step_proposal.entities_to_add {
            collect_entity_ids(&add.entity, &mut known_entity_ids);
        }
        for script in &step_proposal.scripts {
            known_asset_ids.push(script.asset_id.clone());
        }
        step_summaries.push(format!("{}: {}", step.agent.label(), step_proposal.summary));

        merged.entities_to_add.extend(step_proposal.entities_to_add);
        merged
            .entities_to_update
            .extend(step_proposal.entities_to_update);
        merged
            .entities_to_remove
            .extend(step_proposal.entities_to_remove);
        merged.scripts.extend(step_proposal.scripts);
        if step_proposal.scene_patch.gravity.is_some() {
            merged.scene_patch.gravity = step_proposal.scene_patch.gravity;
        }
        if step_proposal.scene_patch.music.is_some() {
            merged.scene_patch.music = step_proposal.scene_patch.music;
        }
    }

    // Dedupe entities_to_remove in case more than one step proposed removing
    // the same id — harmless to apply once, confusing to list twice.
    let mut seen = HashSet::new();
    merged
        .entities_to_remove
        .retain(|id| seen.insert(id.clone()));

    if !step_summaries.is_empty() {
        merged.summary = format!(
            "{}\n{}",
            merged.summary,
            step_summaries
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{}. {s}", i + 1))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    Ok(merged)
}

fn parse_plan(raw: &str) -> Result<OrchestrationPlan, String> {
    let json = extract_json_object(raw)?;
    let plan: OrchestrationPlan = serde_json::from_str(json)
        .map_err(|e| format!("the Architect's plan wasn't valid JSON: {e}"))?;
    if plan.steps.is_empty() {
        return Err("the Architect's plan has no steps — nothing to do".to_string());
    }
    if plan.steps.len() > MAX_STEPS {
        return Err(format!(
            "the Architect's plan has {} steps, more than the {MAX_STEPS} allowed",
            plan.steps.len()
        ));
    }
    Ok(plan)
}

// --- Producer: whole-game/whole-scene generation (milestone M21) ---
//
// A "Producer" plans which SCENES a high-level instruction needs — the one
// already open in the editor, brand-new ones, or both — before handing each
// scene's goal to `orchestrate` exactly as M20 already does. This closes a
// gap M20 left open (its own example, "create a second harder level",
// actually needed a new scene, which `orchestrate` alone never supported):
// see `docs/arquitectura.md` for why this is a thin planning layer on top
// of the existing engine rather than a new generation mechanism.

/// One scene an instruction needs: either the one already open in the
/// editor (`is_new: false`) or a brand-new one to create (`is_new: true`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenePlan {
    pub name: String,
    #[serde(default)]
    pub is_new: bool,
    pub goal: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ProducerPlan {
    summary: String,
    #[serde(default)]
    scenes: Vec<ScenePlan>,
}

/// Keeps a runaway plan from turning "make a game" into dozens of scenes,
/// each itself a multi-step orchestration — this is the outermost of two
/// step-count guards (see `MAX_STEPS` for the inner one).
const MAX_SCENES: usize = 6;

fn build_producer_prompt(context: &str, known_scene_names: &[String]) -> String {
    let scenes_list = if known_scene_names.is_empty() {
        "(none yet — this is a brand new project)".to_string()
    } else {
        known_scene_names.join(", ")
    };
    format!(
        "You are the \"Producer\" for a game project in AI Game Studio. Given a high-level \
         instruction, decide which scenes it needs and what each one should accomplish. A \
         scene is either the one ALREADY OPEN in the editor (\"is_new\": false — use this for \
         changes to what already exists; at most one scene in your plan may have \
         \"is_new\": false) or a BRAND NEW one you name (\"is_new\": true).\n\n\
         Existing scene names already in the project (do not reuse these for a new scene): \
         {scenes_list}\n\n\
         Respond with ONLY a single JSON object — no markdown code fences, no commentary \
         before or after it:\n\
         {{\n\
         \x20 \"summary\": \"one-line description of the overall game\",\n\
         \x20 \"scenes\": [ {{ \"name\": string, \"is_new\": boolean, \"goal\": \"what this scene should contain and do, in enough detail for a team to build it\" }} ]\n\
         }}\n\n\
         Keep it as short as the instruction actually needs (at most {MAX_SCENES} scenes) — a \
         simple single-scene game needs only one entry with \"is_new\": false.\n\n\
         Current project state:\n{context}"
    )
}

/// One scene's validated result within a `ProjectProposal`.
#[derive(Debug, Clone, Serialize)]
pub struct ScenedProposal {
    pub name: String,
    pub is_new: bool,
    pub proposal: ChangeProposal,
}

/// A complete-game (or complete-new-scene) generation result: one or more
/// scenes, each with its own validated `ChangeProposal`. Meant to be
/// applied together as a single atomic change — see `docs/arquitectura.md`
/// on why undoing a generated game should be one `Ctrl+Z`, not one per
/// scene.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectProposal {
    pub summary: String,
    pub scenes: Vec<ScenedProposal>,
}

/// Generates a whole game, or a whole new scene within one, from a
/// high-level `instruction`. The Producer decides which scenes are needed;
/// each scene is then built by `orchestrate` exactly as in M20 — a new
/// scene starts with no entities/animations of its own, the "current" one
/// keeps whatever `current_entity_ids`/`current_animation_names` says it
/// already has. Scenes run sequentially and share the growing asset list
/// (a script one scene writes is visible to the next), same rationale as
/// `orchestrate`'s sequential steps.
pub async fn generate_project(
    provider: &Provider,
    context: &str,
    instruction: String,
    known_assets: Vec<AssetRef>,
    known_scene_names: Vec<String>,
    current_entity_ids: Vec<String>,
    current_animation_names: Vec<String>,
) -> Result<ProjectProposal, String> {
    let planning_messages = [
        ChatMessage {
            role: "system".to_string(),
            content: build_producer_prompt(context, &known_scene_names),
        },
        ChatMessage {
            role: "user".to_string(),
            content: instruction,
        },
    ];
    let raw_plan = provider
        .chat(&planning_messages, true)
        .await
        .map_err(|e| e.to_string())?;
    let plan = parse_producer_plan(&raw_plan)?;

    let mut known_assets = known_assets;
    let mut used_names: HashSet<String> = known_scene_names.iter().cloned().collect();
    let mut seen_current = false;
    let mut scenes = Vec::new();

    for scene_plan in &plan.scenes {
        if scene_plan.is_new {
            if !used_names.insert(scene_plan.name.clone()) {
                return Err(format!(
                    "the Producer's plan reuses scene name \"{}\"",
                    scene_plan.name
                ));
            }
        } else if !seen_current {
            seen_current = true;
        } else {
            return Err(
                "the Producer's plan targets the currently open scene more than once".to_string(),
            );
        }

        let (entity_ids, animation_names) = if scene_plan.is_new {
            (Vec::new(), Vec::new())
        } else {
            (current_entity_ids.clone(), current_animation_names.clone())
        };

        let proposal = orchestrate(
            provider,
            context,
            scene_plan.goal.clone(),
            known_assets.clone(),
            entity_ids,
            animation_names,
        )
        .await
        .map_err(|e| format!("scene \"{}\" {e}", scene_plan.name))?;

        for script in &proposal.scripts {
            known_assets.push(AssetRef {
                id: script.asset_id.clone(),
                kind: "script".to_string(),
            });
        }

        scenes.push(ScenedProposal {
            name: scene_plan.name.clone(),
            is_new: scene_plan.is_new,
            proposal,
        });
    }

    Ok(ProjectProposal {
        summary: plan.summary,
        scenes,
    })
}

fn parse_producer_plan(raw: &str) -> Result<ProducerPlan, String> {
    let json = extract_json_object(raw)?;
    let plan: ProducerPlan = serde_json::from_str(json)
        .map_err(|e| format!("the Producer's plan wasn't valid JSON: {e}"))?;
    if plan.scenes.is_empty() {
        return Err("the Producer's plan has no scenes — nothing to do".to_string());
    }
    if plan.scenes.len() > MAX_SCENES {
        return Err(format!(
            "the Producer's plan has {} scenes, more than the {MAX_SCENES} allowed",
            plan.scenes.len()
        ));
    }
    for scene in &plan.scenes {
        if scene.name.trim().is_empty()
            || scene.name.contains('/')
            || scene.name.contains('\\')
            || scene.name.contains("..")
        {
            return Err(format!("invalid scene name \"{}\"", scene.name));
        }
    }
    Ok(plan)
}

/// Same tolerant extraction as `ai::parse_and_validate_proposal` uses —
/// duplicated rather than shared because it's a tiny, self-contained
/// parser and pulling it across the module boundary isn't worth a `pub`.
fn extract_json_object(raw: &str) -> Result<&str, String> {
    let start = raw
        .find('{')
        .ok_or_else(|| "no JSON object found in the model's response".to_string())?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (i, c) in raw.char_indices().skip(start) {
        if in_string {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(&raw[start..=i]);
                }
            }
            _ => {}
        }
    }
    Err("unterminated JSON object in the model's response".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plan_rejects_an_empty_plan() {
        let raw = serde_json::json!({ "summary": "nothing to do", "steps": [] }).to_string();
        let error = parse_plan(&raw).unwrap_err();
        assert!(error.contains("no steps"), "unexpected error: {error}");
    }

    #[test]
    fn parse_plan_rejects_too_many_steps() {
        let steps: Vec<_> = (0..MAX_STEPS + 1)
            .map(|_| serde_json::json!({ "agent": "architect", "instruction": "do something" }))
            .collect();
        let raw = serde_json::json!({ "summary": "too much", "steps": steps }).to_string();
        let error = parse_plan(&raw).unwrap_err();
        assert!(error.contains("more than"), "unexpected error: {error}");
    }

    #[test]
    fn parse_plan_rejects_an_unknown_agent() {
        let raw = serde_json::json!({
            "summary": "plan",
            "steps": [{ "agent": "optimizer", "instruction": "make it faster" }]
        })
        .to_string();
        assert!(parse_plan(&raw).is_err());
    }

    #[test]
    fn parse_plan_accepts_a_valid_plan_wrapped_in_prose() {
        let inner = serde_json::json!({
            "summary": "Adds a platform with a collider",
            "steps": [
                { "agent": "architect", "instruction": "place a platform sprite at (0, -100)" },
                { "agent": "physics", "instruction": "give the platform a static collider" }
            ]
        })
        .to_string();
        let raw = format!("Sure!\n```json\n{inner}\n```");
        let plan = parse_plan(&raw).unwrap();
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].agent, AgentKind::Architect);
        assert_eq!(plan.steps[1].agent, AgentKind::Physics);
    }

    #[test]
    fn parse_producer_plan_rejects_an_empty_plan() {
        let raw = serde_json::json!({ "summary": "nothing", "scenes": [] }).to_string();
        let error = parse_producer_plan(&raw).unwrap_err();
        assert!(error.contains("no scenes"), "unexpected error: {error}");
    }

    #[test]
    fn parse_producer_plan_rejects_too_many_scenes() {
        let scenes: Vec<_> = (0..MAX_SCENES + 1)
            .map(|i| serde_json::json!({ "name": format!("scene-{i}"), "is_new": true, "goal": "x" }))
            .collect();
        let raw = serde_json::json!({ "summary": "too much", "scenes": scenes }).to_string();
        let error = parse_producer_plan(&raw).unwrap_err();
        assert!(error.contains("more than"), "unexpected error: {error}");
    }

    #[test]
    fn parse_producer_plan_rejects_a_path_like_scene_name() {
        let raw = serde_json::json!({
            "summary": "plan",
            "scenes": [{ "name": "../escape", "is_new": true, "goal": "x" }]
        })
        .to_string();
        let error = parse_producer_plan(&raw).unwrap_err();
        assert!(
            error.contains("invalid scene name"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn parse_producer_plan_accepts_a_valid_multi_scene_plan() {
        let raw = serde_json::json!({
            "summary": "A menu and a level",
            "scenes": [
                { "name": "menu", "is_new": true, "goal": "a title screen with a start button" },
                { "name": "level", "is_new": true, "goal": "a simple platform to stand on" }
            ]
        })
        .to_string();
        let plan = parse_producer_plan(&raw).unwrap();
        assert_eq!(plan.scenes.len(), 2);
        assert!(plan.scenes[0].is_new);
    }

    /// Manual, exploratory probe (see `#[ignore]`): does a real local model
    /// actually plan and execute a two-step orchestration end to end? Run
    /// with `cargo test -- --ignored orchestrate_end_to_end`. Expect
    /// several minutes: one planning call plus one call per step, all
    /// against a CPU-only local model.
    #[ignore]
    #[tokio::test]
    async fn orchestrate_end_to_end_with_real_ollama() {
        let base_url = "http://localhost:11434".to_string();
        if reqwest::Client::new()
            .get(format!("{base_url}/api/tags"))
            .send()
            .await
            .is_err()
        {
            eprintln!("skipping: no Ollama reachable at {base_url}");
            return;
        }
        let model =
            std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:7b".to_string());
        let provider = Provider::Ollama { base_url, model };
        let known_assets = vec![AssetRef {
            id: "platform".to_string(),
            kind: "image".to_string(),
        }];
        let context =
            r#"{"project":{"name":"Test"},"current_scene":{"name":"main","entities":[]}}"#;
        let proposal = orchestrate(
            &provider,
            context,
            "Add a static platform at (0, -100) using the existing \"platform\" sprite, \
             and give it a collider so the player can stand on it."
                .to_string(),
            known_assets,
            vec![],
            vec![],
        )
        .await
        .expect("orchestration should produce a valid merged proposal");
        eprintln!("merged proposal: {proposal:?}");
        assert!(
            !proposal.entities_to_add.is_empty() || !proposal.entities_to_update.is_empty(),
            "expected the plan to touch at least one entity"
        );
    }

    /// Manual, exploratory probe (see `#[ignore]`): the M21 case study —
    /// does a real local model turn one instruction into a brand-new,
    /// multi-scene mini-game (menu + level)? Run with
    /// `cargo test -- --ignored generate_project_end_to_end`. Expect this
    /// to be slow: one Producer call plus a full `orchestrate` (its own
    /// planning + step calls) per scene, all against a CPU-only local
    /// model — tens of minutes, not seconds.
    #[ignore]
    #[tokio::test]
    async fn generate_project_end_to_end_with_real_ollama() {
        let base_url = "http://localhost:11434".to_string();
        if reqwest::Client::new()
            .get(format!("{base_url}/api/tags"))
            .send()
            .await
            .is_err()
        {
            eprintln!("skipping: no Ollama reachable at {base_url}");
            return;
        }
        let model =
            std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:7b".to_string());
        let provider = Provider::Ollama { base_url, model };
        let known_assets = vec![AssetRef {
            id: "hero".to_string(),
            kind: "image".to_string(),
        }];
        let context =
            r#"{"project":{"name":"Coin Runner","assets":[{"id":"hero","kind":"image"}]}}"#;
        let project = generate_project(
            &provider,
            context,
            "Make a tiny game with exactly two scenes, each as simple as possible: \
             (1) a title screen scene with just one entity, the hero sprite centered \
             on screen, nothing else; (2) a level scene with just one entity, a \
             static platform using the hero sprite with a collider so the player can \
             stand on it, nothing else."
                .to_string(),
            known_assets,
            vec![],
            vec![],
            vec![],
        )
        .await
        .expect("project generation should produce a valid multi-scene proposal");
        eprintln!("generated project: {project:?}");
        assert!(!project.scenes.is_empty(), "expected at least one scene");
        for scene in &project.scenes {
            assert!(
                !scene.proposal.entities_to_add.is_empty()
                    || !scene.proposal.entities_to_update.is_empty(),
                "expected scene \"{}\" to touch at least one entity",
                scene.name
            );
        }
    }
}
