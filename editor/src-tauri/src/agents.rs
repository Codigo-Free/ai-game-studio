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
}
