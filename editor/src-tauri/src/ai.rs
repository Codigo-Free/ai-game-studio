//! AI Core (milestones M18-M19): lets the editor's Chat panel answer
//! questions about the currently open project (M18), or propose a concrete,
//! reviewable change to it (M19) — backed by a local model (Ollama) or a
//! cloud one (Claude/OpenAI — implemented against their public APIs but
//! not locally verified, since doing so needs the user's own API key).
//!
//! Provider/model selection is resolved in `settings.rs` (a settings-panel
//! fast-follow of M18): an environment variable, if set, always wins;
//! otherwise a local settings file, with hardcoded defaults under that.

use std::collections::HashSet;

use aigs_project::{ActionSpec, Components, EntityNode, Gravity, Music};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// `"system"`, `"user"` or `"assistant"`.
    pub role: String,
    pub content: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("request to {provider} failed: {source}")]
    Request {
        provider: &'static str,
        source: reqwest::Error,
    },
    #[error("{provider} returned {status}: {body}")]
    Status {
        provider: &'static str,
        status: reqwest::StatusCode,
        body: String,
    },
    #[error("could not parse the provider's response: {0}")]
    Parse(String),
}

/// One model provider, chosen at request time by `settings::resolve_provider`.
/// An `enum` rather than `dyn Trait`: with two or three variants and async
/// calls, a trait object would need `async-trait` (boxed futures) just to
/// get the same dynamic dispatch a `match` already gives us for free.
pub enum Provider {
    Ollama { base_url: String, model: String },
    Claude { api_key: String, model: String },
    OpenAi { api_key: String, model: String },
}

impl std::fmt::Debug for Provider {
    /// Hand-written rather than derived so an API key can never end up in a
    /// panic/log message — e.g. `Result::unwrap_err` prints the `Ok` value's
    /// `Debug` if a test unexpectedly succeeds.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Ollama { base_url, model } => f
                .debug_struct("Ollama")
                .field("base_url", base_url)
                .field("model", model)
                .finish(),
            Provider::Claude { model, .. } => f
                .debug_struct("Claude")
                .field("api_key", &"<redacted>")
                .field("model", model)
                .finish(),
            Provider::OpenAi { model, .. } => f
                .debug_struct("OpenAi")
                .field("api_key", &"<redacted>")
                .field("model", model)
                .finish(),
        }
    }
}

impl Provider {
    /// `json_mode` asks Ollama/OpenAI to constrain their output to
    /// syntactically valid JSON (Ollama's `format: "json"` request field,
    /// OpenAI's `response_format: {"type": "json_object"}`) — used by the
    /// M19 "propose a change" flow. Claude has no equivalent knob in this
    /// design (see `docs/arquitectura.md`), so it's ignored on that branch;
    /// the system prompt's own instructions carry the weight there instead.
    pub async fn chat(
        &self,
        messages: &[ChatMessage],
        json_mode: bool,
    ) -> Result<String, ProviderError> {
        match self {
            Provider::Ollama { base_url, model } => {
                ollama_chat(base_url, model, messages, json_mode).await
            }
            Provider::Claude { api_key, model } => claude_chat(api_key, model, messages).await,
            Provider::OpenAi { api_key, model } => {
                openai_chat(api_key, model, messages, json_mode).await
            }
        }
    }
}

#[derive(Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<&'static str>,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaResponseMessage,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

async fn ollama_chat(
    base_url: &str,
    model: &str,
    messages: &[ChatMessage],
    json_mode: bool,
) -> Result<String, ProviderError> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{base_url}/api/chat"))
        .json(&OllamaRequest {
            model,
            messages,
            stream: false,
            format: if json_mode { Some("json") } else { None },
        })
        .send()
        .await
        .map_err(|source| ProviderError::Request {
            provider: "Ollama",
            source,
        })?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ProviderError::Status {
            provider: "Ollama",
            status,
            body,
        });
    }
    let parsed: OllamaResponse = response
        .json()
        .await
        .map_err(|error| ProviderError::Parse(error.to_string()))?;
    Ok(parsed.message.content)
}

/// Anthropic's Messages API: the system prompt is a top-level field, not a
/// `"system"`-role message in the array.
#[derive(Serialize)]
struct ClaudeRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<ClaudeMessage<'a>>,
}

#[derive(Serialize)]
struct ClaudeMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
}

#[derive(Deserialize)]
struct ClaudeContentBlock {
    text: String,
}

async fn claude_chat(
    api_key: &str,
    model: &str,
    messages: &[ChatMessage],
) -> Result<String, ProviderError> {
    let system = messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.as_str())
        .unwrap_or_default();
    let turns: Vec<ClaudeMessage> = messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| ClaudeMessage {
            role: &m.role,
            content: &m.content,
        })
        .collect();

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&ClaudeRequest {
            model,
            max_tokens: 1024,
            system,
            messages: turns,
        })
        .send()
        .await
        .map_err(|source| ProviderError::Request {
            provider: "Claude",
            source,
        })?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ProviderError::Status {
            provider: "Claude",
            status,
            body,
        });
    }
    let parsed: ClaudeResponse = response
        .json()
        .await
        .map_err(|error| ProviderError::Parse(error.to_string()))?;
    Ok(parsed
        .content
        .into_iter()
        .map(|block| block.text)
        .collect::<Vec<_>>()
        .join(""))
}

/// OpenAI's Chat Completions API: unlike Claude, the system message is just
/// another entry in the same array (same shape as Ollama's), and it has a
/// native JSON-forcing knob (`response_format`).
#[derive(Serialize)]
struct OpenAiRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<OpenAiResponseFormat>,
}

#[derive(Serialize)]
struct OpenAiResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

#[derive(Deserialize)]
struct OpenAiResponseMessage {
    content: String,
}

async fn openai_chat(
    api_key: &str,
    model: &str,
    messages: &[ChatMessage],
    json_mode: bool,
) -> Result<String, ProviderError> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&OpenAiRequest {
            model,
            messages,
            response_format: if json_mode {
                Some(OpenAiResponseFormat {
                    kind: "json_object",
                })
            } else {
                None
            },
        })
        .send()
        .await
        .map_err(|source| ProviderError::Request {
            provider: "OpenAI",
            source,
        })?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ProviderError::Status {
            provider: "OpenAI",
            status,
            body,
        });
    }
    let parsed: OpenAiResponse = response
        .json()
        .await
        .map_err(|error| ProviderError::Parse(error.to_string()))?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .ok_or_else(|| ProviderError::Parse("OpenAI returned no choices".to_string()))
}

/// The scripting API manifest (milestone M12), embedded at compile time so
/// the "propose a change" prompt always ships the same contract a human
/// reading `sdk/aigs-format/scripting-api.json` would see — no separate
/// copy to keep in sync, no runtime path to get wrong.
const SCRIPTING_API_MANIFEST: &str = include_str!("../../../sdk/aigs-format/scripting-api.json");

/// A reference to a project asset (id + kind), just enough for the prompt
/// to tell the model what it may point `sprite`/`script`/`particles` at.
#[derive(Debug, Clone, Deserialize)]
pub struct AssetRef {
    pub id: String,
    pub kind: String,
}

/// One entity to insert into the current scene, at `parent_id` (root if
/// `None`). `entity` deserializes straight into the format's own type, so a
/// hallucinated component or wrong field type is rejected here rather than
/// silently accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityToAdd {
    pub parent_id: Option<String>,
    pub entity: EntityNode,
}

/// A partial component patch merged onto an existing entity. `Components`'
/// fields are all optional, so a JSON object naming only the fields that
/// change deserializes cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityToUpdate {
    pub id: String,
    pub components_patch: Components,
}

/// A new `.rhai` script asset the proposal wants to add, referenced by
/// `asset_id` from any `script: { asset }` component in the same proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptToWrite {
    pub asset_id: String,
    pub filename: String,
    pub content: String,
}

/// Scene-level fields a proposal may patch, as opposed to per-entity
/// components (milestone M20 — `gravity`/`music` live on `Scene`, not on
/// any entity, so they need their own patch shape).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gravity: Option<Gravity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub music: Option<Music>,
}

/// A concrete, reviewable change to the currently open scene (milestone
/// M19; `scene_patch` added in M20). Applying it is the frontend's job (it
/// owns the document/undo stack); this struct is just the validated,
/// parsed shape of what the model proposed. `scene_patch` always
/// serializes (even when empty) so the frontend can treat it as always
/// present, the same way the list fields are always `[]` rather than
/// absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeProposal {
    #[serde(default = "default_summary")]
    pub summary: String,
    #[serde(default)]
    pub entities_to_add: Vec<EntityToAdd>,
    #[serde(default)]
    pub entities_to_update: Vec<EntityToUpdate>,
    #[serde(default)]
    pub entities_to_remove: Vec<String>,
    #[serde(default)]
    pub scripts: Vec<ScriptToWrite>,
    #[serde(default)]
    pub scene_patch: ScenePatch,
}

/// Fallback for `summary`-shaped fields the model sometimes omits under
/// token pressure or plain instruction-following slips. `summary` is pure
/// descriptive metadata — unlike a scope violation, a bad asset reference
/// or a script that fails to compile, a missing one says nothing about
/// whether the rest of the proposal is safe to apply, so it doesn't
/// deserve the same hard rejection.
pub(crate) fn default_summary() -> String {
    "(no summary provided)".to_string()
}

/// Shared core of every "propose a JSON change" prompt (M19's single
/// unrestricted Programmer and M20's scoped specialists alike): the JSON
/// schema, the assets/entity/animation ids already in play (so the model
/// doesn't invent or collide with them), and the project context itself
/// (built by the frontend, same as M18's read-only chat). `role_preamble`
/// carries whatever's specific to who's asking; `include_scripting_api`
/// skips the (sizeable) scripting manifest for agents that never write
/// scripts, saving them prompt space and latency.
fn build_json_proposal_prompt(
    role_preamble: &str,
    context: &str,
    known_assets: &[AssetRef],
    known_entity_ids: &[String],
    known_animation_names: &[String],
    include_scripting_api: bool,
) -> String {
    let assets_list = if known_assets.is_empty() {
        "(none yet)".to_string()
    } else {
        known_assets
            .iter()
            .map(|a| format!("{} ({})", a.id, a.kind))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let entity_ids_list = if known_entity_ids.is_empty() {
        "(none yet)".to_string()
    } else {
        known_entity_ids.join(", ")
    };
    let animations_list = if known_animation_names.is_empty() {
        "(none yet)".to_string()
    } else {
        known_animation_names.join(", ")
    };
    let scripting_section = if include_scripting_api {
        format!(
            "To give an entity a script: (1) add ONE entry to the top-level \"scripts\" list \
             with a fresh \"asset_id\" (a plain string, not in the asset list above) and the \
             rhai \"content\"; (2) on the entity's \"script\" component, set \"asset\" to that \
             SAME plain string id — never the script's content or an object. Example: \
             top-level `\"scripts\": [{{\"asset_id\": \"door-script\", \"filename\": \"door.rhai\", \"content\": \"fn on_start() {{}}\"}}]` \
             plus, on the entity, `\"components\": {{\"script\": {{\"asset\": \"door-script\"}}}}` — \
             note \"asset\" is the bare string \"door-script\", NOT `{{\"content\": ...}}`. The same \
             rule applies to \"sprite\"/\"particles\": their \"asset\" is always a plain string id, \
             never a nested object. Script content must be valid rhai using only the API below \
             (functions not listed here do not exist):\n\n{SCRIPTING_API_MANIFEST}\n\n"
        )
    } else {
        String::new()
    };
    format!(
        "{role_preamble} Given the project state below and an instruction, respond with \
         ONLY a single JSON object — no markdown code fences, no commentary before or \
         after it — describing a proposed change to the CURRENTLY OPEN SCENE. \
         A human will review your proposal and confirm it before anything is \
         written to disk, so prefer a working, conservative change over a \
         speculative one.\n\n\
         JSON schema (all list fields optional, default to an empty list/object):\n\
         {{\n\
         \x20 \"summary\": \"one-line, human-readable description of the change\",\n\
         \x20 \"entities_to_add\": [ {{ \"parent_id\": string|null, \"entity\": {{ \"id\": string, \"name\": string, \"components\": {{ ... }} }} }} ],\n\
         \x20 \"entities_to_update\": [ {{ \"id\": string, \"components_patch\": {{ ... }} }} ],\n\
         \x20 \"entities_to_remove\": [ string ],\n\
         \x20 \"scripts\": [ {{ \"asset_id\": string, \"filename\": string (must end in \\\".rhai\\\"), \"content\": string }} ],\n\
         \x20 \"scene_patch\": {{ \"gravity\": {{x,y}}, \"music\": {{asset,volume,looped}} }}\n\
         }}\n\n\
         Component shapes you may use inside \"components\"/\"components_patch\" (all fields optional \
         unless noted; exact string values matter, there is no synonym matching): \
         transform2d {{x,y,rotation,scale_x,scale_y}}, sprite {{asset,frame,width,height,opacity,layer}}, \
         rigidbody2d {{body: \"dynamic\"|\"kinematic\"|\"static\",gravity_scale,vx,vy,fixed_rotation}}, \
         collider2d {{shape: \"box\"|\"circle\" (NOT \"rectangle\"/\"square\"/\"sphere\"),width,height,radius,sensor,restitution,friction}}, \
         animator {{initial,states,transitions}}, script {{asset}}.\n\n\
         behaviors is a list of {{\"on\": EventSpec, \"do\": ActionSpec}} rules. EventSpec and \
         ActionSpec are each a JSON OBJECT with a \"type\" field naming the exact variant below \
         plus that variant's own fields at the same level (NEVER a plain string):\n\
         EventSpec: {{\"type\":\"key_down\",\"key\":string}} | {{\"type\":\"key_pressed\",\"key\":string}} | \
         {{\"type\":\"key_released\",\"key\":string}} | {{\"type\":\"click\"}} | {{\"type\":\"scene_start\"}} | \
         {{\"type\":\"animation_end\",\"animation\":string}} | {{\"type\":\"collision\",\"with\":string (optional)}}.\n\
         ActionSpec: {{\"type\":\"move\",\"dx\":number,\"dy\":number}} | {{\"type\":\"goto_scene\",\"scene\":string}} | \
         {{\"type\":\"play_animation\",\"animation\":string}} | {{\"type\":\"play_sound\",\"asset\":string,\"volume\":number (optional)}} | \
         {{\"type\":\"emit_particles\",\"count\":number (optional)}}.\n\
         Example: {{\"on\":{{\"type\":\"scene_start\"}},\"do\":{{\"type\":\"play_sound\",\"asset\":\"theme\"}}}}.\n\n\
         Entity ids already in the current scene (this may include entities another step of the \
         same plan just added): {entity_ids_list}\n\
         If you need to change one of THESE entities (e.g. add a collider or a script to an \
         entity that already exists), use \"entities_to_update\" with its existing id — do NOT \
         put that same id in \"entities_to_add\", that would try to create a duplicate and gets \
         rejected. Only use \"entities_to_add\" for a genuinely new entity, with a fresh id not \
         in this list.\n\n\
         Existing project assets you may reference by id from \"sprite\"/\"particles\"/\"script\", \
         \"scene_patch.music\" and the \"play_sound\" action: {assets_list}\n\
         Every one of those components is OPTIONAL — an entity can have just \"transform2d\" and \
         nothing else. If the asset you'd need (e.g. an image for \"sprite\", an audio asset for \
         \"scene_patch.music\" or \"play_sound\") is NOT in the list above, DO NOT invent an id \
         and DO NOT set \"asset\" to `null` — instead leave that WHOLE component/field/behavior \
         out of your JSON entirely (e.g. an entity with no fitting image gets no \"sprite\" key at \
         all; omit \"scene_patch\" if there's no music asset; drop a behavior if it needs a sound \
         that doesn't exist) and mention the gap in \"summary\" instead. A smaller, honest proposal \
         that only uses what actually exists is always better than one that references or nulls \
         out something that isn't there.\n\n\
         Existing scene animations you may reference by name from an \"animator\" component \
         (do not invent new ones — authoring keyframes is a separate, manual step): {animations_list}\n\n\
         NEVER write `null` as the value of ANY field, of ANY type (string, number, or boolean) — \
         not \"id\"/\"name\"/\"asset\"/\"filename\"/\"content\", and not numeric ones either like \
         \"frame\"/\"width\"/\"height\"/\"layer\"/\"volume\"/\"count\". If you don't want to set a \
         field, or don't have a real value for it, DO NOT INCLUDE THAT KEY AT ALL — leaving a key \
         out is always valid and uses its default; writing `null` for it is a syntax error and \
         gets the whole proposal rejected. The ONLY field allowed to be `null` is \"parent_id\" \
         (which means \"no parent\", a real, meaningful value there).\n\n\
         {scripting_section}\
         Current project/scene state:\n{context}"
    )
}

/// Builds the system prompt for M19's "propose a change" mode: a single,
/// unrestricted "Programmer" agent with full access to entity structure,
/// components, behaviors and scripts (no per-agent scope — see M20's
/// `agents::build_scoped_agent_prompt` for the restricted version used by
/// orchestration specialists).
pub fn build_propose_system_prompt(
    context: &str,
    known_assets: &[AssetRef],
    known_entity_ids: &[String],
    known_animation_names: &[String],
) -> String {
    build_json_proposal_prompt(
        "You are the \"Programmer\" agent embedded in AI Game Studio's editor.",
        context,
        known_assets,
        known_entity_ids,
        known_animation_names,
        true,
    )
}

/// Builds the system prompt for one scoped specialist within an M20
/// orchestration plan: same JSON schema as M19, but told (and, separately,
/// enforced by `parse_and_validate_proposal`'s `scope`) which components it
/// may set. The scripting manifest is only included for the Programmer —
/// every other specialist never writes a script, so it would just be
/// unused prompt weight.
pub(crate) fn build_scoped_agent_prompt(
    agent: AgentKind,
    context: &str,
    known_assets: &[AssetRef],
    known_entity_ids: &[String],
    known_animation_names: &[String],
) -> String {
    let scene_patch_fields = agent.allowed_scene_patch_fields();
    let scene_patch_rule = if scene_patch_fields.is_empty() {
        "You may NOT set \"scene_patch\" at all — omit it entirely.".to_string()
    } else {
        format!(
            "You may ONLY set these fields in \"scene_patch\": {scene_patch_fields:?} — omit any \
             other scene_patch field entirely."
        )
    };
    let preamble = format!(
        "You are the \"{label}\" specialist in a team of agents working on a game project \
         in AI Game Studio, coordinated by an Architect that already broke a larger \
         instruction into steps — the instruction below is YOUR step. You may ONLY set these \
         components on entities you add or update: {component_keys:?}. {scene_patch_rule} \
         Anything else (any other component, any other scene_patch field) is another \
         specialist's job; leave it out of your proposal.",
        label = agent.label(),
        component_keys = agent.allowed_component_keys(),
    );
    build_json_proposal_prompt(
        &preamble,
        context,
        known_assets,
        known_entity_ids,
        known_animation_names,
        agent == AgentKind::Programmer,
    )
}

/// Pulls the first balanced `{...}` object out of `raw`, tolerating any
/// prose or markdown fences a model adds around it (Claude in particular
/// tends to narrate before answering, despite being told not to).
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

/// A specialist's area of responsibility within a `ChangeProposal` — which
/// component keys (and scene-level fields) it may set. The system prompt
/// already tells each specialist what it owns, but a model can ignore
/// that, so this is enforced server-side too (milestone M20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Architect,
    LevelDesigner,
    Programmer,
    Physics,
    Audio,
    Animator,
}

impl AgentKind {
    pub fn label(&self) -> &'static str {
        match self {
            AgentKind::Architect => "Architect",
            AgentKind::LevelDesigner => "Level Designer",
            AgentKind::Programmer => "Programmer",
            AgentKind::Physics => "Physics",
            AgentKind::Audio => "Audio",
            AgentKind::Animator => "Animator",
        }
    }

    /// Component keys this agent may set on an entity it adds or updates.
    pub fn allowed_component_keys(&self) -> &'static [&'static str] {
        match self {
            AgentKind::Architect => &["transform2d", "sprite"],
            AgentKind::LevelDesigner => &["transform2d", "sprite", "collider2d"],
            AgentKind::Programmer => &["script", "behaviors"],
            AgentKind::Physics => &["rigidbody2d", "collider2d"],
            AgentKind::Audio => &["behaviors"],
            AgentKind::Animator => &["animator"],
        }
    }

    fn may_set_gravity(&self) -> bool {
        matches!(self, AgentKind::Physics)
    }

    fn may_set_music(&self) -> bool {
        matches!(self, AgentKind::Audio)
    }

    /// `scene_patch` field names this agent may set — a completely
    /// different vocabulary from `allowed_component_keys()` (component
    /// names like "sprite" aren't scene_patch fields at all), so it needs
    /// its own list rather than reusing that one in the prompt.
    pub fn allowed_scene_patch_fields(&self) -> &'static [&'static str] {
        match self {
            AgentKind::Physics => &["gravity"],
            AgentKind::Audio => &["music"],
            _ => &[],
        }
    }
}

fn used_component_keys(components: &Components) -> Vec<&'static str> {
    let mut used = Vec::new();
    if components.transform2d.is_some() {
        used.push("transform2d");
    }
    if components.sprite.is_some() {
        used.push("sprite");
    }
    if components.camera2d.is_some() {
        used.push("camera2d");
    }
    if components.rigidbody2d.is_some() {
        used.push("rigidbody2d");
    }
    if components.collider2d.is_some() {
        used.push("collider2d");
    }
    if components.animator.is_some() {
        used.push("animator");
    }
    if components.particles.is_some() {
        used.push("particles");
    }
    if components.script.is_some() {
        used.push("script");
    }
    if components.virtual_button.is_some() {
        used.push("virtual_button");
    }
    if !components.behaviors.is_empty() {
        used.push("behaviors");
    }
    if !components.extra.is_empty() {
        used.push("extra (plugin components)");
    }
    used
}

fn check_components_scope(components: &Components, agent: AgentKind) -> Result<(), String> {
    let allowed = agent.allowed_component_keys();
    for key in used_component_keys(components) {
        if !allowed.contains(&key) {
            return Err(format!(
                "the \"{}\" agent isn't allowed to set component \"{key}\" (allowed: {allowed:?})",
                agent.label()
            ));
        }
    }
    Ok(())
}

fn check_scene_patch_scope(patch: &ScenePatch, agent: AgentKind) -> Result<(), String> {
    if patch.gravity.is_some() && !agent.may_set_gravity() {
        return Err(format!(
            "the \"{}\" agent isn't allowed to set scene gravity",
            agent.label()
        ));
    }
    if patch.music.is_some() && !agent.may_set_music() {
        return Err(format!(
            "the \"{}\" agent isn't allowed to set scene music",
            agent.label()
        ));
    }
    Ok(())
}

fn check_components_asset_refs(
    components: &Components,
    available: &HashSet<String>,
) -> Result<(), String> {
    if let Some(sprite) = &components.sprite {
        if !available.contains(&sprite.asset) {
            return Err(format!("references unknown asset \"{}\"", sprite.asset));
        }
    }
    if let Some(script) = &components.script {
        if !available.contains(&script.asset) {
            return Err(format!("references unknown asset \"{}\"", script.asset));
        }
    }
    if let Some(particles) = &components.particles {
        if !available.contains(&particles.asset) {
            return Err(format!("references unknown asset \"{}\"", particles.asset));
        }
    }
    for behavior in &components.behaviors {
        if let ActionSpec::PlaySound { asset, .. } = &behavior.action {
            if !available.contains(asset) {
                return Err(format!(
                    "references unknown asset \"{asset}\" in a play_sound behavior"
                ));
            }
        }
    }
    Ok(())
}

fn check_scene_patch_asset_refs(
    patch: &ScenePatch,
    available: &HashSet<String>,
) -> Result<(), String> {
    if let Some(music) = &patch.music {
        if !available.contains(&music.asset) {
            return Err(format!(
                "scene_patch.music references unknown asset \"{}\"",
                music.asset
            ));
        }
    }
    Ok(())
}

fn check_components_animator_refs(
    components: &Components,
    known_animations: &HashSet<String>,
) -> Result<(), String> {
    let Some(animator) = &components.animator else {
        return Ok(());
    };
    if !known_animations.contains(&animator.initial) {
        return Err(format!(
            "animator references unknown animation \"{}\"",
            animator.initial
        ));
    }
    for name in animator.states.values() {
        if !known_animations.contains(name) {
            return Err(format!("animator references unknown animation \"{name}\""));
        }
    }
    Ok(())
}

fn check_entity_asset_refs(entity: &EntityNode, available: &HashSet<String>) -> Result<(), String> {
    check_components_asset_refs(&entity.components, available)
        .map_err(|e| format!("entity \"{}\" {e}", entity.id))?;
    for child in &entity.children {
        check_entity_asset_refs(child, available)?;
    }
    Ok(())
}

fn check_entity_animator_refs(
    entity: &EntityNode,
    known_animations: &HashSet<String>,
) -> Result<(), String> {
    check_components_animator_refs(&entity.components, known_animations)
        .map_err(|e| format!("entity \"{}\" {e}", entity.id))?;
    for child in &entity.children {
        check_entity_animator_refs(child, known_animations)?;
    }
    Ok(())
}

fn check_entity_scope(entity: &EntityNode, agent: AgentKind) -> Result<(), String> {
    check_components_scope(&entity.components, agent)
        .map_err(|e| format!("entity \"{}\" {e}", entity.id))?;
    for child in &entity.children {
        check_entity_scope(child, agent)?;
    }
    Ok(())
}

pub(crate) fn collect_entity_ids(entity: &EntityNode, into: &mut Vec<String>) {
    into.push(entity.id.clone());
    for child in &entity.children {
        collect_entity_ids(child, into);
    }
}

/// Parses the model's raw text response into a `ChangeProposal` and rejects
/// it (with a message fit to show the user) unless every asset/animation
/// reference resolves, every updated/removed id already exists, no new id
/// collides with an existing one, every generated script actually compiles,
/// and — when `scope` is given (milestone M20's per-agent write scope) —
/// every component/scene field it touches is one that agent is allowed to
/// set. "All or nothing": a proposal either fully checks out and can be
/// shown for confirmation, or it doesn't get shown at all — no
/// partial/best-effort application.
pub fn parse_and_validate_proposal(
    raw: &str,
    known_asset_ids: &[String],
    known_entity_ids: &[String],
    known_animation_names: &[String],
    scope: Option<AgentKind>,
) -> Result<ChangeProposal, String> {
    let json = extract_json_object(raw)?;
    let proposal: ChangeProposal = serde_json::from_str(json)
        .map_err(|e| format!("the model's response wasn't a valid change proposal: {e}"))?;

    if let Some(agent) = scope {
        check_scene_patch_scope(&proposal.scene_patch, agent)?;
    }

    let mut available: HashSet<String> = known_asset_ids.iter().cloned().collect();
    check_scene_patch_asset_refs(&proposal.scene_patch, &available)?;
    for script in &proposal.scripts {
        if !script.filename.ends_with(".rhai")
            || script.filename.contains('/')
            || script.filename.contains('\\')
            || script.filename.contains("..")
        {
            return Err(format!(
                "proposed script filename \"{}\" is not a plain \"name.rhai\"",
                script.filename
            ));
        }
        if available.contains(&script.asset_id) {
            return Err(format!(
                "proposed script asset id \"{}\" collides with an existing asset",
                script.asset_id
            ));
        }
        rhai::Engine::new().compile(&script.content).map_err(|e| {
            format!(
                "generated script \"{}\" doesn't compile: {e}",
                script.filename
            )
        })?;
        available.insert(script.asset_id.clone());
    }

    let known_animations: HashSet<String> = known_animation_names.iter().cloned().collect();
    let existing_entities: HashSet<String> = known_entity_ids.iter().cloned().collect();
    let mut new_ids: HashSet<String> = HashSet::new();
    for add in &proposal.entities_to_add {
        let mut ids = Vec::new();
        collect_entity_ids(&add.entity, &mut ids);
        for id in ids {
            if existing_entities.contains(&id) || !new_ids.insert(id.clone()) {
                return Err(format!(
                    "proposed entity id \"{id}\" collides with an existing or another proposed entity"
                ));
            }
        }
        if let Some(parent_id) = &add.parent_id {
            if !existing_entities.contains(parent_id) && !new_ids.contains(parent_id) {
                return Err(format!("references unknown parent entity \"{parent_id}\""));
            }
        }
        check_entity_asset_refs(&add.entity, &available)?;
        check_entity_animator_refs(&add.entity, &known_animations)?;
        if let Some(agent) = scope {
            check_entity_scope(&add.entity, agent)?;
        }
    }
    for update in &proposal.entities_to_update {
        if !existing_entities.contains(&update.id) {
            return Err(format!("wants to update unknown entity \"{}\"", update.id));
        }
        check_components_asset_refs(&update.components_patch, &available)?;
        check_components_animator_refs(&update.components_patch, &known_animations)?;
        if let Some(agent) = scope {
            check_components_scope(&update.components_patch, agent)?;
        }
    }
    for id in &proposal.entities_to_remove {
        if !existing_entities.contains(id) {
            return Err(format!("wants to remove unknown entity \"{id}\""));
        }
    }

    Ok(proposal)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real call to a local Ollama (no mocking) — skipped, not failed, if
    /// nothing is listening on the default port, matching how the audio
    /// player degrades to a no-op without a device instead of erroring.
    #[tokio::test]
    async fn ollama_answers_a_simple_question_when_available() {
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
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:latest".to_string());
        let provider = Provider::Ollama { base_url, model };
        let answer = provider
            .chat(
                &[ChatMessage {
                    role: "user".to_string(),
                    content: "Reply with exactly the word: pong".to_string(),
                }],
                false,
            )
            .await
            .unwrap();
        assert!(
            !answer.trim().is_empty(),
            "expected a non-empty answer, got {answer:?}"
        );
    }

    /// Manual, exploratory probe (not part of the regular suite — see
    /// `#[ignore]`): does a real local model actually follow the "propose a
    /// change" JSON schema end to end? Run with
    /// `cargo test -- --ignored propose_change_end_to_end`.
    #[ignore]
    #[tokio::test]
    async fn propose_change_end_to_end_with_real_ollama() {
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
        let known_entity_ids = vec!["hero".to_string()];
        let context = r#"{"project":{"name":"Test"},"current_scene":{"name":"main","entities":[{"id":"hero","name":"Hero","components":{"transform2d":{"x":0,"y":0},"sprite":{"asset":"hero"}}}]}}"#;
        let system = build_propose_system_prompt(context, &known_assets, &known_entity_ids, &[]);
        let messages = [
            ChatMessage {
                role: "system".to_string(),
                content: system,
            },
            ChatMessage {
                role: "user".to_string(),
                content: "Add a coin entity at position (50, 50) using the existing \"hero\" sprite. No script needed.".to_string(),
            },
        ];
        let raw = provider.chat(&messages, true).await.unwrap();
        eprintln!("raw model response:\n{raw}");
        let known_asset_ids: Vec<String> = known_assets.into_iter().map(|a| a.id).collect();
        let proposal =
            parse_and_validate_proposal(&raw, &known_asset_ids, &known_entity_ids, &[], None)
                .expect("model's proposal should pass validation");
        assert_eq!(proposal.entities_to_add.len(), 1);
        eprintln!("parsed proposal: {proposal:?}");
    }

    fn drone_proposal_json() -> serde_json::Value {
        serde_json::json!({
            "summary": "Adds a patrolling drone",
            "entities_to_add": [{
                "parent_id": null,
                "entity": {
                    "id": "patrol-drone",
                    "name": "Patrol Drone",
                    "components": {
                        "transform2d": { "x": 100.0, "y": 200.0 },
                        "sprite": { "asset": "drone" },
                        "script": { "asset": "patrol-drone-script" }
                    }
                }
            }],
            "scripts": [{
                "asset_id": "patrol-drone-script",
                "filename": "patrol-drone.rhai",
                "content": "fn on_start() { set_var(\"dir\", 1.0); }\nfn on_update(dt) { move_by(get_var(\"dir\") * 100.0 * dt, 0.0); }"
            }]
        })
    }

    #[test]
    fn valid_proposal_with_new_entity_and_script_is_accepted() {
        let raw = drone_proposal_json().to_string();
        let known_assets = vec!["drone".to_string()];
        let known_entities = vec!["player".to_string()];
        let proposal =
            parse_and_validate_proposal(&raw, &known_assets, &known_entities, &[], None).unwrap();
        assert_eq!(proposal.entities_to_add.len(), 1);
        assert_eq!(proposal.scripts.len(), 1);
    }

    #[test]
    fn proposal_missing_summary_is_accepted_with_a_default() {
        // Real failure observed live: a model omitted "summary" entirely
        // (not null — absent) while the rest of the proposal was valid.
        // Unlike scope/asset/script violations, a missing summary says
        // nothing about safety, so it shouldn't sink an otherwise-good
        // proposal.
        let mut json = drone_proposal_json();
        json.as_object_mut().unwrap().remove("summary");
        let proposal =
            parse_and_validate_proposal(&json.to_string(), &["drone".to_string()], &[], &[], None)
                .unwrap();
        assert_eq!(proposal.summary, default_summary());
    }

    #[test]
    fn proposal_with_a_well_formed_behavior_is_accepted() {
        // Real failure observed live: a model set "on" to a plain string
        // ("space_background") instead of a tagged EventSpec object,
        // because the prompt only said `{on, do}` without spelling out
        // the variant shapes. This proves the documented example shape
        // (see build_json_proposal_prompt) actually round-trips.
        let raw = serde_json::json!({
            "summary": "Plays music when the scene starts",
            "entities_to_update": [{
                "id": "player",
                "components_patch": {
                    "behaviors": [
                        { "on": { "type": "scene_start" }, "do": { "type": "play_sound", "asset": "theme" } }
                    ]
                }
            }]
        })
        .to_string();
        let proposal = parse_and_validate_proposal(
            &raw,
            &["theme".to_string()],
            &["player".to_string()],
            &[],
            None,
        )
        .unwrap();
        assert_eq!(proposal.entities_to_update.len(), 1);
    }

    #[test]
    fn proposal_with_a_play_sound_of_an_unknown_asset_is_rejected() {
        let raw = serde_json::json!({
            "summary": "Adds a sound effect",
            "entities_to_update": [{
                "id": "player",
                "components_patch": {
                    "behaviors": [
                        { "on": { "type": "click" }, "do": { "type": "play_sound", "asset": "made-up-sfx" } }
                    ]
                }
            }]
        })
        .to_string();
        let error =
            parse_and_validate_proposal(&raw, &[], &["player".to_string()], &[], None).unwrap_err();
        assert!(error.contains("made-up-sfx"), "unexpected error: {error}");
    }

    #[test]
    fn proposal_setting_scene_music_to_an_unknown_asset_is_rejected() {
        // Real failure observed live: the Audio agent had no real audio
        // asset to reference (the project had none) and emitted `null`
        // instead — this test covers the sibling case where it instead
        // hallucinates an asset id, which must be rejected the same way
        // sprite/script/particles asset references already are.
        let raw = serde_json::json!({
            "summary": "Adds background music",
            "scene_patch": { "music": { "asset": "made-up-theme" } }
        })
        .to_string();
        let error = parse_and_validate_proposal(&raw, &[], &[], &[], None).unwrap_err();
        assert!(error.contains("made-up-theme"), "unexpected error: {error}");
    }

    #[test]
    fn proposal_setting_scene_music_to_a_known_asset_is_accepted() {
        let raw = serde_json::json!({
            "summary": "Adds background music",
            "scene_patch": { "music": { "asset": "theme" } }
        })
        .to_string();
        let proposal =
            parse_and_validate_proposal(&raw, &["theme".to_string()], &[], &[], None).unwrap();
        assert_eq!(
            proposal.scene_patch.music.map(|m| m.asset),
            Some("theme".to_string())
        );
    }

    #[test]
    fn proposal_wrapped_in_prose_and_code_fences_is_still_extracted() {
        let inner = drone_proposal_json().to_string();
        let raw =
            format!("Sure, here you go:\n```json\n{inner}\n```\nLet me know if you need changes!");
        let known_assets = vec!["drone".to_string()];
        let proposal = parse_and_validate_proposal(&raw, &known_assets, &[], &[], None).unwrap();
        assert_eq!(proposal.summary, "Adds a patrolling drone");
    }

    #[test]
    fn proposal_referencing_unknown_asset_is_rejected() {
        let raw = drone_proposal_json().to_string();
        // "drone" is deliberately absent from the known assets.
        let error = parse_and_validate_proposal(&raw, &[], &[], &[], None).unwrap_err();
        assert!(error.contains("unknown asset"), "unexpected error: {error}");
    }

    #[test]
    fn proposal_with_broken_script_is_rejected() {
        let mut json = drone_proposal_json();
        json["scripts"][0]["content"] = serde_json::json!("fn on_start( { this is not rhai");
        let error =
            parse_and_validate_proposal(&json.to_string(), &["drone".to_string()], &[], &[], None)
                .unwrap_err();
        assert!(
            error.contains("doesn't compile"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn proposal_reusing_an_existing_entity_id_is_rejected() {
        let raw = drone_proposal_json().to_string();
        let known_entities = vec!["patrol-drone".to_string()];
        let error =
            parse_and_validate_proposal(&raw, &["drone".to_string()], &known_entities, &[], None)
                .unwrap_err();
        assert!(error.contains("collides"), "unexpected error: {error}");
    }

    #[test]
    fn proposal_updating_an_unknown_entity_is_rejected() {
        let raw = serde_json::json!({
            "summary": "Move the boss",
            "entities_to_update": [{ "id": "no-such-entity", "components_patch": { "transform2d": { "x": 1.0 } } }]
        })
        .to_string();
        let error = parse_and_validate_proposal(&raw, &[], &[], &[], None).unwrap_err();
        assert!(
            error.contains("unknown entity"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn proposal_with_path_traversal_in_script_filename_is_rejected() {
        let mut json = drone_proposal_json();
        json["scripts"][0]["filename"] = serde_json::json!("../../etc/passwd.rhai");
        let error =
            parse_and_validate_proposal(&json.to_string(), &["drone".to_string()], &[], &[], None)
                .unwrap_err();
        assert!(error.contains("plain"), "unexpected error: {error}");
    }

    #[test]
    fn proposal_setting_a_component_outside_the_agents_scope_is_rejected() {
        // The drone proposal sets transform2d/sprite/script, none of which
        // the Physics agent (rigidbody2d/collider2d only) may touch.
        let raw = drone_proposal_json().to_string();
        let error = parse_and_validate_proposal(
            &raw,
            &["drone".to_string()],
            &[],
            &[],
            Some(AgentKind::Physics),
        )
        .unwrap_err();
        assert!(error.contains("Physics"), "unexpected error: {error}");
        assert!(error.contains("isn't allowed"), "unexpected error: {error}");
    }

    #[test]
    fn proposal_setting_a_component_within_the_agents_scope_is_accepted() {
        // Architect owns transform2d/sprite, both present on "coin".
        let coin = serde_json::json!({
            "summary": "Adds a coin",
            "entities_to_add": [{
                "parent_id": null,
                "entity": {
                    "id": "coin",
                    "name": "Coin",
                    "components": { "transform2d": { "x": 50.0 }, "sprite": { "asset": "hero" } }
                }
            }]
        })
        .to_string();
        let proposal = parse_and_validate_proposal(
            &coin,
            &["hero".to_string()],
            &[],
            &[],
            Some(AgentKind::Architect),
        )
        .unwrap();
        assert_eq!(proposal.entities_to_add.len(), 1);
    }

    #[test]
    fn proposal_referencing_unknown_animation_is_rejected() {
        let raw = serde_json::json!({
            "summary": "Wire up idle/walk",
            "entities_to_add": [{
                "parent_id": null,
                "entity": {
                    "id": "robot",
                    "name": "Robot",
                    "components": { "animator": { "initial": "idle", "states": { "idle": "idle", "walk": "no-such-anim" } } }
                }
            }]
        })
        .to_string();
        let error =
            parse_and_validate_proposal(&raw, &[], &[], &["idle".to_string()], None).unwrap_err();
        assert!(error.contains("no-such-anim"), "unexpected error: {error}");
    }

    #[test]
    fn proposal_setting_scene_gravity_from_a_non_physics_agent_is_rejected() {
        let raw = serde_json::json!({
            "summary": "Change gravity",
            "scene_patch": { "gravity": { "x": 0.0, "y": -500.0 } }
        })
        .to_string();
        let error =
            parse_and_validate_proposal(&raw, &[], &[], &[], Some(AgentKind::Audio)).unwrap_err();
        assert!(error.contains("gravity"), "unexpected error: {error}");
    }

    #[test]
    fn scoped_agent_prompt_tells_each_agent_its_real_scene_patch_fields() {
        // Real failure observed live: the prompt used to reuse
        // `allowed_component_keys()` (component names like "sprite") for
        // the scene_patch sentence too, so an Architect's prompt claimed
        // it could set scene_patch fields "transform2d"/"sprite" — neither
        // of which is a real scene_patch field — and never actually told
        // it gravity/music were off limits. It then set gravity anyway.
        let architect_prompt = build_scoped_agent_prompt(AgentKind::Architect, "", &[], &[], &[]);
        assert!(
            architect_prompt.contains("You may NOT set \"scene_patch\""),
            "Architect's prompt should forbid scene_patch entirely: {architect_prompt}"
        );

        let physics_prompt = build_scoped_agent_prompt(AgentKind::Physics, "", &[], &[], &[]);
        assert!(
            physics_prompt.contains("[\"gravity\"]"),
            "Physics's prompt should mention gravity as its scene_patch field: {physics_prompt}"
        );

        let audio_prompt = build_scoped_agent_prompt(AgentKind::Audio, "", &[], &[], &[]);
        assert!(
            audio_prompt.contains("[\"music\"]"),
            "Audio's prompt should mention music as its scene_patch field: {audio_prompt}"
        );
    }
}
