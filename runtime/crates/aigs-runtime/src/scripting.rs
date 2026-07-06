//! User scripting (milestone M12): rhai scripts attached to entities.
//!
//! A script asset may define four lifecycle functions, all optional:
//!
//! - `fn on_start()` — once, right after the scene loads.
//! - `fn on_update(dt)` — every simulation tick (dt in seconds).
//! - `fn on_collision(other)` — when this entity's collider starts touching
//!   another; `other` is the touched entity's id (`""` if it has none).
//! - `fn on_destroy()` — once, right before the scene is torn down.
//!
//! Scripts read the world through registered functions (see [`api_manifest`]
//! for the full, machine-readable contract) and mutate it through a command
//! queue drained after each call — the engine stays in control.
//!
//! The rhai engine is sandboxed: no file/network access and an operation
//! budget per call, so a runaway script degrades to a warning, not a hang.
//!
//! While running from a project directory (not exported/zipped), the host
//! polls each script file's mtime a couple of times a second and hot-reloads
//! it on change: editing and saving a `.rhai` file updates a running game
//! without restarting Play mode (script-local state resets on reload).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use aigs_ecs::{Entity, World};
use aigs_project::{Asset, AssetKind, EntityNode};
use rhai::{Engine, ImmutableString, Scope, AST};
use serde::Serialize;

use crate::components::{Sprite, Transform2D};
use crate::input::Input;
use crate::player::parse_key;
use crate::scene::SceneInstance;
use crate::KeyCode;

/// How often (in ticks) a running host checks script files for changes.
/// 30 ticks at the fixed 60 Hz step is twice a second.
const RELOAD_CHECK_INTERVAL: u32 = 30;

/// Engine-level effects a script can request; applied by the caller.
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptCommand {
    GotoScene(String),
    PlayAnimation(String),
    PlaySound(String, f32),
    EmitParticles(u32),
}

#[derive(Default)]
struct SelfState {
    x: f32,
    y: f32,
    rotation: f32,
    frame: f32,
}

#[derive(Default)]
struct Shared {
    self_state: RefCell<SelfState>,
    /// Positions of every authored entity in the scene, by id.
    positions: RefCell<HashMap<String, (f32, f32)>>,
    keys: RefCell<(HashSet<KeyCode>, HashSet<KeyCode>, HashSet<KeyCode>)>,
    /// Pending transform mutations (applied to the running entity).
    moves: RefCell<Vec<TransformCommand>>,
    commands: RefCell<Vec<ScriptCommand>>,
    logs: RefCell<Vec<String>>,
    /// The current script instance's persistent state (get_var/set_var),
    /// swapped in before each call and read back after (rhai `fn` bodies
    /// don't see the calling Scope, so this is the supported way for a
    /// script to remember a value across ticks — see `load_memory`).
    memory: RefCell<HashMap<String, f64>>,
}

#[derive(Debug, Clone, Copy)]
enum TransformCommand {
    SetPos(f32, f32),
    MoveBy(f32, f32),
    SetRotation(f32),
    SetFrame(f32),
}

struct RunningScript {
    entity: Entity,
    entity_id: String,
    asset: String,
    scope: Scope<'static>,
    started: bool,
    failed: bool,
    has_start: bool,
    has_update: bool,
    has_on_collision: bool,
    has_on_destroy: bool,
    /// This instance's persistent state (see `get_var`/`set_var`).
    memory: HashMap<String, f64>,
}

impl RunningScript {
    fn refresh_flags(&mut self, ast: &AST) {
        self.has_start = ast.iter_functions().any(|f| f.name == "on_start");
        self.has_update = ast.iter_functions().any(|f| f.name == "on_update");
        self.has_on_collision = ast.iter_functions().any(|f| f.name == "on_collision");
        self.has_on_destroy = ast.iter_functions().any(|f| f.name == "on_destroy");
    }
}

/// Compiles the project's script assets and runs one instance per entity
/// with a `script` component.
pub struct ScriptHost {
    engine: Engine,
    shared: Rc<Shared>,
    asts: HashMap<String, Rc<AST>>,
    /// Absolute path of each script asset, when loaded from disk (enables
    /// hot reload); empty for scripts added via [`ScriptHost::add_source`].
    sources: HashMap<String, PathBuf>,
    mtimes: HashMap<String, SystemTime>,
    running: Vec<RunningScript>,
    warnings: Vec<String>,
    ticks_since_reload_check: u32,
}

impl ScriptHost {
    /// Loads and compiles every `script` asset. Compile errors become
    /// warnings; the affected scripts simply don't run.
    pub fn load(root: &Path, assets: &[Asset]) -> Self {
        let mut host = Self::empty();
        for asset in assets {
            if asset.kind != AssetKind::Script {
                continue;
            }
            let path = root.join(&asset.path);
            match std::fs::read_to_string(&path) {
                Ok(source) => {
                    host.add_source(&asset.id, &source);
                    if let Ok(modified) = std::fs::metadata(&path).and_then(|m| m.modified()) {
                        host.mtimes.insert(asset.id.clone(), modified);
                    }
                    host.sources.insert(asset.id.clone(), path);
                }
                Err(error) => host.warnings.push(format!(
                    "script asset \"{}\" ({}): {error}",
                    asset.id, asset.path
                )),
            }
        }
        host
    }

    /// A host with no scripts (tests, projects without scripting).
    pub fn empty() -> Self {
        let shared = Rc::new(Shared::default());
        let engine = build_engine(&shared);
        Self {
            engine,
            shared,
            asts: HashMap::new(),
            sources: HashMap::new(),
            mtimes: HashMap::new(),
            running: Vec::new(),
            warnings: Vec::new(),
            ticks_since_reload_check: 0,
        }
    }

    /// Compiles a script from source under an asset id (also used by tests).
    /// Scripts added this way are not file-backed and never hot-reload.
    pub fn add_source(&mut self, id: &str, source: &str) {
        match self.engine.compile(source) {
            Ok(ast) => {
                self.asts.insert(id.to_string(), Rc::new(ast));
            }
            Err(error) => self
                .warnings
                .push(format!("script \"{id}\": compile error: {error}")),
        }
    }

    /// Number of compiled scripts.
    pub fn len(&self) -> usize {
        self.asts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.asts.is_empty()
    }

    /// Problems found while loading/binding/running (drained by the caller).
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// Creates the running instances for a freshly loaded scene.
    pub fn bind(&mut self, entities: &[EntityNode], instance: &SceneInstance) {
        self.running.clear();
        self.bind_rec(entities, instance);
    }

    fn bind_rec(&mut self, entities: &[EntityNode], instance: &SceneInstance) {
        for node in entities {
            if let Some(script) = &node.components.script {
                match (self.asts.get(&script.asset), instance.entity(&node.id)) {
                    (Some(ast), Some(entity)) => {
                        let mut running = RunningScript {
                            entity,
                            entity_id: node.id.clone(),
                            asset: script.asset.clone(),
                            scope: Scope::new(),
                            started: false,
                            failed: false,
                            has_start: false,
                            has_update: false,
                            has_on_collision: false,
                            has_on_destroy: false,
                            memory: HashMap::new(),
                        };
                        running.refresh_flags(ast);
                        if !running.has_start
                            && !running.has_update
                            && !running.has_on_collision
                            && !running.has_on_destroy
                        {
                            self.warnings.push(format!(
                                "script \"{}\": defines no lifecycle function (on_start/on_update/on_collision/on_destroy)",
                                script.asset
                            ));
                        }
                        self.running.push(running);
                    }
                    (None, _) => self.warnings.push(format!(
                        "entity \"{}\": unknown script asset \"{}\"",
                        node.id, script.asset
                    )),
                    _ => {}
                }
            }
            self.bind_rec(&node.children, instance);
        }
    }

    /// Checks every file-backed script for changes and recompiles it,
    /// resetting the local state (`Scope`) of its running instances. Compile
    /// errors keep the previous, working version and become a warning.
    /// Returns the ids that were reloaded.
    pub fn check_reload(&mut self) -> Vec<String> {
        let mut reloaded = Vec::new();
        let ids: Vec<String> = self.sources.keys().cloned().collect();
        for id in ids {
            let path = &self.sources[&id];
            let Ok(modified) = std::fs::metadata(path).and_then(|m| m.modified()) else {
                continue;
            };
            if self.mtimes.get(&id) == Some(&modified) {
                continue;
            }
            self.mtimes.insert(id.clone(), modified);
            let source = match std::fs::read_to_string(path) {
                Ok(source) => source,
                Err(error) => {
                    self.warnings
                        .push(format!("script \"{id}\": hot-reload read error: {error}"));
                    continue;
                }
            };
            match self.engine.compile(&source) {
                Ok(ast) => {
                    let ast = Rc::new(ast);
                    for script in &mut self.running {
                        if script.asset == id {
                            script.refresh_flags(&ast);
                            script.scope = Scope::new();
                            script.memory.clear();
                            script.started = false;
                            script.failed = false;
                        }
                    }
                    self.asts.insert(id.clone(), ast);
                    println!("[script:{id}] hot-reloaded");
                    reloaded.push(id.clone());
                }
                Err(error) => self.warnings.push(format!(
                    "script \"{id}\": hot-reload compile error (keeping previous version): {error}"
                )),
            }
        }
        reloaded
    }

    /// Runs every script's `on_start`/`on_update` for this tick and returns
    /// the engine-level commands (scene switches, sounds, …) to apply.
    pub fn tick(
        &mut self,
        world: &World,
        instance: &SceneInstance,
        input: &Input,
        dt: f32,
    ) -> Vec<(Entity, ScriptCommand)> {
        if !self.sources.is_empty() {
            self.ticks_since_reload_check += 1;
            if self.ticks_since_reload_check >= RELOAD_CHECK_INTERVAL {
                self.ticks_since_reload_check = 0;
                self.check_reload();
            }
        }
        if self.running.is_empty() {
            return Vec::new();
        }

        // Shared snapshots for this tick.
        {
            let mut positions = self.shared.positions.borrow_mut();
            positions.clear();
            for (id, entity) in instance.ids() {
                if let Some(transform) = world.get::<Transform2D>(entity) {
                    positions.insert(id.clone(), (transform.x, transform.y));
                }
            }
            *self.shared.keys.borrow_mut() = input.key_snapshot();
        }

        let mut out = Vec::new();
        for script in &mut self.running {
            if script.failed || !world.is_alive(script.entity) {
                continue;
            }
            snapshot_self(&self.shared, world, script.entity);
            load_memory(&self.shared, script);

            if !script.started {
                script.started = true;
                if script.has_start {
                    if let Err(error) = self.engine.call_fn::<rhai::Dynamic>(
                        &mut script.scope,
                        &self.asts[&script.asset],
                        "on_start",
                        (),
                    ) {
                        script.failed = true;
                        self.warnings.push(format!(
                            "script \"{}\" ({}): on_start: {error}",
                            script.asset, script.entity_id
                        ));
                        save_memory(&self.shared, script);
                        continue;
                    }
                }
            }
            if script.has_update {
                if let Err(error) = self.engine.call_fn::<rhai::Dynamic>(
                    &mut script.scope,
                    &self.asts[&script.asset],
                    "on_update",
                    (f64::from(dt),),
                ) {
                    script.failed = true;
                    self.warnings.push(format!(
                        "script \"{}\" ({}): on_update: {error}",
                        script.asset, script.entity_id
                    ));
                    save_memory(&self.shared, script);
                    continue;
                }
            }

            save_memory(&self.shared, script);
            apply_moves(&self.shared, world, script.entity);
            drain_commands(&self.shared, script.entity, &mut out);
            drain_logs(&self.shared, &script.asset);
        }
        out
    }

    /// Calls `on_collision(other)` on every running script whose entity
    /// appears in `contacts`, once per contacting pair.
    pub fn dispatch_collisions(
        &mut self,
        world: &World,
        instance: &SceneInstance,
        contacts: &[(Entity, Entity)],
    ) -> Vec<(Entity, ScriptCommand)> {
        let mut out = Vec::new();
        if self.running.is_empty() || contacts.is_empty() {
            return out;
        }
        for script in &mut self.running {
            if script.failed || !script.has_on_collision || !world.is_alive(script.entity) {
                continue;
            }
            for &(a, b) in contacts {
                let other = if a == script.entity {
                    b
                } else if b == script.entity {
                    a
                } else {
                    continue;
                };
                snapshot_self(&self.shared, world, script.entity);
                load_memory(&self.shared, script);
                let other_id = instance.id_of(other).unwrap_or_default().to_string();
                if let Err(error) = self.engine.call_fn::<rhai::Dynamic>(
                    &mut script.scope,
                    &self.asts[&script.asset],
                    "on_collision",
                    (other_id,),
                ) {
                    script.failed = true;
                    self.warnings.push(format!(
                        "script \"{}\" ({}): on_collision: {error}",
                        script.asset, script.entity_id
                    ));
                    save_memory(&self.shared, script);
                    continue;
                }
                save_memory(&self.shared, script);
                apply_moves(&self.shared, world, script.entity);
                drain_commands(&self.shared, script.entity, &mut out);
                drain_logs(&self.shared, &script.asset);
            }
        }
        out
    }

    /// Calls `on_destroy()` on every running script, right before the scene
    /// that owns them is torn down. `world` must still contain the entities
    /// (call before clearing it).
    pub fn dispatch_destroy(&mut self, world: &World) -> Vec<(Entity, ScriptCommand)> {
        let mut out = Vec::new();
        for script in &mut self.running {
            if script.failed || !script.has_on_destroy || !world.is_alive(script.entity) {
                continue;
            }
            snapshot_self(&self.shared, world, script.entity);
            load_memory(&self.shared, script);
            if let Err(error) = self.engine.call_fn::<rhai::Dynamic>(
                &mut script.scope,
                &self.asts[&script.asset],
                "on_destroy",
                (),
            ) {
                self.warnings.push(format!(
                    "script \"{}\" ({}): on_destroy: {error}",
                    script.asset, script.entity_id
                ));
            }
            save_memory(&self.shared, script);
            apply_moves(&self.shared, world, script.entity);
            drain_commands(&self.shared, script.entity, &mut out);
            drain_logs(&self.shared, &script.asset);
        }
        out
    }
}

/// Swaps a script instance's persistent state into `Shared` before a call.
fn load_memory(shared: &Shared, script: &RunningScript) {
    *shared.memory.borrow_mut() = script.memory.clone();
}

/// Reads a script instance's persistent state back out after a call.
fn save_memory(shared: &Shared, script: &mut RunningScript) {
    script.memory = shared.memory.borrow().clone();
}

/// Refreshes the per-entity read snapshot (`x()`, `y()`, `rotation()`, `frame()`).
fn snapshot_self(shared: &Shared, world: &World, entity: Entity) {
    let mut state = shared.self_state.borrow_mut();
    if let Some(transform) = world.get::<Transform2D>(entity) {
        state.x = transform.x;
        state.y = transform.y;
        state.rotation = transform.rotation;
    }
    state.frame = world
        .get::<Sprite>(entity)
        .map(|sprite| sprite.frame)
        .unwrap_or(0.0);
}

/// Applies the transform/frame mutations a script queued for `entity`.
fn apply_moves(shared: &Shared, world: &World, entity: Entity) {
    for command in shared.moves.borrow_mut().drain(..) {
        match command {
            TransformCommand::SetPos(x, y) => {
                if let Some(mut t) = world.get_mut::<Transform2D>(entity) {
                    t.x = x;
                    t.y = y;
                }
            }
            TransformCommand::MoveBy(dx, dy) => {
                if let Some(mut t) = world.get_mut::<Transform2D>(entity) {
                    t.x += dx;
                    t.y += dy;
                }
            }
            TransformCommand::SetRotation(rotation) => {
                if let Some(mut t) = world.get_mut::<Transform2D>(entity) {
                    t.rotation = rotation;
                }
            }
            TransformCommand::SetFrame(frame) => {
                if let Some(mut sprite) = world.get_mut::<Sprite>(entity) {
                    sprite.frame = frame;
                }
            }
        }
    }
}

/// Moves the engine-level commands a script queued into `out`.
fn drain_commands(shared: &Shared, entity: Entity, out: &mut Vec<(Entity, ScriptCommand)>) {
    for command in shared.commands.borrow_mut().drain(..) {
        out.push((entity, command));
    }
}

/// Flushes a script's `log()` calls to stdout.
fn drain_logs(shared: &Shared, asset: &str) {
    for message in shared.logs.borrow_mut().drain(..) {
        println!("[script:{asset}] {message}");
    }
}

// ---------------------------------------------------------------------------
// Typed, machine-readable API manifest (for AI agents and editor tooling).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ApiParam {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

fn param(name: &str, ty: &str) -> ApiParam {
    ApiParam {
        name: name.to_string(),
        ty: ty.to_string(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiFunction {
    pub name: String,
    pub params: Vec<ApiParam>,
    pub returns: Option<String>,
    /// One of: `entity_state`, `other_entities`, `input`, `transform`, `engine`, `utility`.
    pub category: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiLifecycle {
    pub name: String,
    pub params: Vec<ApiParam>,
    pub description: String,
}

/// The full scripting contract: lifecycle functions a script may define,
/// plus every function the engine registers for scripts to call. Kept next
/// to the registrations in [`build_engine`] so it can't silently drift; see
/// `sdk/aigs-format/scripting-api.json` for a checked-in snapshot and
/// `aigs script-api` to print it live.
#[derive(Debug, Clone, Serialize)]
pub struct ApiManifest {
    pub version: u32,
    pub language: String,
    pub lifecycle: Vec<ApiLifecycle>,
    pub functions: Vec<ApiFunction>,
}

pub fn api_manifest() -> ApiManifest {
    ApiManifest {
        version: 1,
        language: "rhai".to_string(),
        lifecycle: vec![
            ApiLifecycle {
                name: "on_start".to_string(),
                params: vec![],
                description: "Called once, right after the scene loads.".to_string(),
            },
            ApiLifecycle {
                name: "on_update".to_string(),
                params: vec![param("dt", "float")],
                description:
                    "Called every simulation tick (60/s). dt is the fixed step in seconds."
                        .to_string(),
            },
            ApiLifecycle {
                name: "on_collision".to_string(),
                params: vec![param("other", "string")],
                description: "Called when this entity's collider starts touching another. \
                    'other' is the id of the touched entity, or \"\" if it has none."
                    .to_string(),
            },
            ApiLifecycle {
                name: "on_destroy".to_string(),
                params: vec![],
                description:
                    "Called once, right before the scene is torn down (e.g. a scene switch)."
                        .to_string(),
            },
        ],
        functions: vec![
            ApiFunction {
                name: "x".to_string(),
                params: vec![],
                returns: Some("float".to_string()),
                category: "entity_state".to_string(),
                description: "This entity's world X position.".to_string(),
            },
            ApiFunction {
                name: "y".to_string(),
                params: vec![],
                returns: Some("float".to_string()),
                category: "entity_state".to_string(),
                description: "This entity's world Y position.".to_string(),
            },
            ApiFunction {
                name: "rotation".to_string(),
                params: vec![],
                returns: Some("float".to_string()),
                category: "entity_state".to_string(),
                description: "This entity's rotation in degrees, clockwise.".to_string(),
            },
            ApiFunction {
                name: "frame".to_string(),
                params: vec![],
                returns: Some("float".to_string()),
                category: "entity_state".to_string(),
                description: "This entity's current spritesheet frame index.".to_string(),
            },
            ApiFunction {
                name: "get_var".to_string(),
                params: vec![param("name", "string")],
                returns: Some("float".to_string()),
                category: "state".to_string(),
                description: "Reads this script instance's persistent state (0.0 if never set). \
                    rhai `fn` bodies can't see outer scope variables between calls, so use \
                    get_var/set_var — not a captured variable — to remember values across ticks."
                    .to_string(),
            },
            ApiFunction {
                name: "set_var".to_string(),
                params: vec![param("name", "string"), param("value", "float")],
                returns: None,
                category: "state".to_string(),
                description: "Writes this script instance's persistent state, readable in later ticks via get_var. Reset when the script hot-reloads or the scene restarts.".to_string(),
            },
            ApiFunction {
                name: "x_of".to_string(),
                params: vec![param("id", "string")],
                returns: Some("float".to_string()),
                category: "other_entities".to_string(),
                description: "World X position of another entity in the scene, by its authored id (0.0 if unknown).".to_string(),
            },
            ApiFunction {
                name: "y_of".to_string(),
                params: vec![param("id", "string")],
                returns: Some("float".to_string()),
                category: "other_entities".to_string(),
                description: "World Y position of another entity in the scene, by its authored id (0.0 if unknown).".to_string(),
            },
            ApiFunction {
                name: "distance_to".to_string(),
                params: vec![param("id", "string")],
                returns: Some("float".to_string()),
                category: "other_entities".to_string(),
                description: "Euclidean distance from this entity to another by id (a very large number if unknown).".to_string(),
            },
            ApiFunction {
                name: "key_down".to_string(),
                params: vec![param("name", "string")],
                returns: Some("bool".to_string()),
                category: "input".to_string(),
                description: "True while the named key is held down (same key names as behaviors, e.g. \"ArrowRight\").".to_string(),
            },
            ApiFunction {
                name: "key_pressed".to_string(),
                params: vec![param("name", "string")],
                returns: Some("bool".to_string()),
                category: "input".to_string(),
                description: "True on the single tick the named key went down.".to_string(),
            },
            ApiFunction {
                name: "key_released".to_string(),
                params: vec![param("name", "string")],
                returns: Some("bool".to_string()),
                category: "input".to_string(),
                description: "True on the single tick the named key went up.".to_string(),
            },
            ApiFunction {
                name: "set_pos".to_string(),
                params: vec![param("x", "float"), param("y", "float")],
                returns: None,
                category: "transform".to_string(),
                description: "Sets this entity's world position immediately.".to_string(),
            },
            ApiFunction {
                name: "move_by".to_string(),
                params: vec![param("dx", "float"), param("dy", "float")],
                returns: None,
                category: "transform".to_string(),
                description: "Offsets this entity's world position by (dx, dy).".to_string(),
            },
            ApiFunction {
                name: "set_rotation".to_string(),
                params: vec![param("degrees", "float")],
                returns: None,
                category: "transform".to_string(),
                description: "Sets this entity's rotation in degrees, clockwise.".to_string(),
            },
            ApiFunction {
                name: "set_frame".to_string(),
                params: vec![param("frame", "float")],
                returns: None,
                category: "transform".to_string(),
                description: "Sets this entity's spritesheet frame index.".to_string(),
            },
            ApiFunction {
                name: "goto_scene".to_string(),
                params: vec![param("path", "string")],
                returns: None,
                category: "engine".to_string(),
                description: "Switches to another scene of the project (path as listed in the manifest). Ignored if called from on_destroy.".to_string(),
            },
            ApiFunction {
                name: "play_animation".to_string(),
                params: vec![param("name", "string")],
                returns: None,
                category: "engine".to_string(),
                description: "Restarts a scene animation by name.".to_string(),
            },
            ApiFunction {
                name: "play_sound".to_string(),
                params: vec![param("asset", "string")],
                returns: None,
                category: "engine".to_string(),
                description: "Plays a sound effect at full volume (overload of play_sound(asset, volume)).".to_string(),
            },
            ApiFunction {
                name: "play_sound".to_string(),
                params: vec![param("asset", "string"), param("volume", "float")],
                returns: None,
                category: "engine".to_string(),
                description: "Plays a sound effect (audio asset id) at the given linear volume (0.0-1.0).".to_string(),
            },
            ApiFunction {
                name: "emit_particles".to_string(),
                params: vec![param("count", "int")],
                returns: None,
                category: "engine".to_string(),
                description: "Spawns a burst from this entity's particle emitter (requires a particles component).".to_string(),
            },
            ApiFunction {
                name: "log".to_string(),
                params: vec![param("message", "string")],
                returns: None,
                category: "utility".to_string(),
                description: "Writes a line to the game console, prefixed with [script:<asset>].".to_string(),
            },
        ],
    }
}

/// Builds the sandboxed engine with the AI Game Studio scripting API. Keep
/// in sync with [`api_manifest`] (a test enforces every documented function
/// here is actually callable).
fn build_engine(shared: &Rc<Shared>) -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(200_000);
    engine.set_max_call_levels(32);

    // -- reads ---------------------------------------------------------------
    let s = Rc::clone(shared);
    engine.register_fn("x", move || f64::from(s.self_state.borrow().x));
    let s = Rc::clone(shared);
    engine.register_fn("y", move || f64::from(s.self_state.borrow().y));
    let s = Rc::clone(shared);
    engine.register_fn("rotation", move || {
        f64::from(s.self_state.borrow().rotation)
    });
    let s = Rc::clone(shared);
    engine.register_fn("frame", move || f64::from(s.self_state.borrow().frame));
    let s = Rc::clone(shared);
    engine.register_fn("x_of", move |id: ImmutableString| {
        f64::from(s.positions.borrow().get(id.as_str()).map_or(0.0, |p| p.0))
    });
    let s = Rc::clone(shared);
    engine.register_fn("y_of", move |id: ImmutableString| {
        f64::from(s.positions.borrow().get(id.as_str()).map_or(0.0, |p| p.1))
    });
    let s = Rc::clone(shared);
    engine.register_fn("distance_to", move |id: ImmutableString| {
        let positions = s.positions.borrow();
        let state = s.self_state.borrow();
        positions.get(id.as_str()).map_or(f64::MAX, |(x, y)| {
            f64::from(((x - state.x).powi(2) + (y - state.y).powi(2)).sqrt())
        })
    });

    // -- persistent instance state --------------------------------------------
    let s = Rc::clone(shared);
    engine.register_fn("get_var", move |name: ImmutableString| {
        *s.memory.borrow().get(name.as_str()).unwrap_or(&0.0)
    });
    let s = Rc::clone(shared);
    engine.register_fn("set_var", move |name: ImmutableString, value: f64| {
        s.memory.borrow_mut().insert(name.to_string(), value);
    });

    // -- input ---------------------------------------------------------------
    let s = Rc::clone(shared);
    engine.register_fn("key_down", move |name: ImmutableString| {
        parse_key(name.as_str()).is_some_and(|code| s.keys.borrow().0.contains(&code))
    });
    let s = Rc::clone(shared);
    engine.register_fn("key_pressed", move |name: ImmutableString| {
        parse_key(name.as_str()).is_some_and(|code| s.keys.borrow().1.contains(&code))
    });
    let s = Rc::clone(shared);
    engine.register_fn("key_released", move |name: ImmutableString| {
        parse_key(name.as_str()).is_some_and(|code| s.keys.borrow().2.contains(&code))
    });

    // -- writes (queued) -----------------------------------------------------
    let s = Rc::clone(shared);
    engine.register_fn("set_pos", move |x: f64, y: f64| {
        s.moves
            .borrow_mut()
            .push(TransformCommand::SetPos(x as f32, y as f32));
    });
    let s = Rc::clone(shared);
    engine.register_fn("move_by", move |dx: f64, dy: f64| {
        s.moves
            .borrow_mut()
            .push(TransformCommand::MoveBy(dx as f32, dy as f32));
    });
    let s = Rc::clone(shared);
    engine.register_fn("set_rotation", move |rotation: f64| {
        s.moves
            .borrow_mut()
            .push(TransformCommand::SetRotation(rotation as f32));
    });
    let s = Rc::clone(shared);
    engine.register_fn("set_frame", move |frame: f64| {
        s.moves
            .borrow_mut()
            .push(TransformCommand::SetFrame(frame as f32));
    });
    let s = Rc::clone(shared);
    engine.register_fn("goto_scene", move |path: ImmutableString| {
        s.commands
            .borrow_mut()
            .push(ScriptCommand::GotoScene(path.to_string()));
    });
    let s = Rc::clone(shared);
    engine.register_fn("play_animation", move |name: ImmutableString| {
        s.commands
            .borrow_mut()
            .push(ScriptCommand::PlayAnimation(name.to_string()));
    });
    let s = Rc::clone(shared);
    engine.register_fn("play_sound", move |name: ImmutableString| {
        s.commands
            .borrow_mut()
            .push(ScriptCommand::PlaySound(name.to_string(), 1.0));
    });
    let s = Rc::clone(shared);
    engine.register_fn("play_sound", move |name: ImmutableString, volume: f64| {
        s.commands
            .borrow_mut()
            .push(ScriptCommand::PlaySound(name.to_string(), volume as f32));
    });
    let s = Rc::clone(shared);
    engine.register_fn("emit_particles", move |count: i64| {
        s.commands
            .borrow_mut()
            .push(ScriptCommand::EmitParticles(count.max(0) as u32));
    });
    let s = Rc::clone(shared);
    engine.register_fn("log", move |message: ImmutableString| {
        s.logs.borrow_mut().push(message.to_string());
    });

    engine
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercises every function documented in [`api_manifest`] once, proving
    /// the manifest and the actual registrations agree on name and arity.
    const SMOKE_SCRIPT: &str = r#"
        fn on_start() {
            let a = x(); let b = y(); let c = rotation(); let d = frame();
            let e = x_of("other"); let f = y_of("other"); let g = distance_to("other");
            let h = key_down("Space"); let i = key_pressed("Space"); let j = key_released("Space");
            set_var("n", get_var("n") + 1.0);
            set_pos(1.0, 2.0);
            move_by(1.0, 1.0);
            set_rotation(90.0);
            set_frame(3.0);
            goto_scene("scenes/x.scene.aigs");
            play_animation("idle");
            play_sound("blip");
            play_sound("blip", 0.5);
            emit_particles(3);
            log("smoke ok");
        }
    "#;

    #[test]
    fn every_documented_function_is_actually_callable() {
        let mut host = ScriptHost::empty();
        host.add_source("smoke", SMOKE_SCRIPT);
        assert!(
            host.warnings.is_empty(),
            "smoke script must compile: {:?}",
            host.warnings
        );
        let ast = host.asts.get("smoke").unwrap().clone();
        let mut scope = Scope::new();
        let _: rhai::Dynamic = host
            .engine
            .call_fn(&mut scope, &ast, "on_start", ())
            .expect("every manifest function must be callable with its documented arity");
    }

    #[test]
    fn api_manifest_lists_every_function_the_smoke_script_calls() {
        let manifest = api_manifest();
        let names: HashSet<&str> = manifest.functions.iter().map(|f| f.name.as_str()).collect();
        for expected in [
            "x",
            "y",
            "rotation",
            "frame",
            "get_var",
            "set_var",
            "x_of",
            "y_of",
            "distance_to",
            "key_down",
            "key_pressed",
            "key_released",
            "set_pos",
            "move_by",
            "set_rotation",
            "set_frame",
            "goto_scene",
            "play_animation",
            "play_sound",
            "emit_particles",
            "log",
        ] {
            assert!(
                names.contains(expected),
                "manifest is missing \"{expected}\""
            );
        }
        let lifecycle: HashSet<&str> = manifest.lifecycle.iter().map(|f| f.name.as_str()).collect();
        for expected in ["on_start", "on_update", "on_collision", "on_destroy"] {
            assert!(
                lifecycle.contains(expected),
                "manifest is missing lifecycle \"{expected}\""
            );
        }
    }

    #[test]
    fn hot_reload_recompiles_and_resets_state() {
        let dir =
            std::env::temp_dir().join(format!("aigs-script-reload-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script_path = dir.join("counter.rhai");
        std::fs::write(
            &script_path,
            r#"
                fn on_start() { set_var("n", 1.0); }
                fn on_update(dt) { set_frame(get_var("n")); }
            "#,
        )
        .unwrap();

        let asset = Asset {
            id: "counter".to_string(),
            kind: AssetKind::Script,
            path: "counter.rhai".to_string(),
            spritesheet: None,
        };
        let mut host = ScriptHost::load(&dir, std::slice::from_ref(&asset));
        assert!(
            host.warnings.is_empty(),
            "should load cleanly: {:?}",
            host.warnings
        );

        struct AnyTexture;
        impl crate::ResolveTexture for AnyTexture {
            fn resolve(&self, _: &str) -> Option<crate::TextureInfo> {
                Some(crate::TextureInfo {
                    id: Default::default(),
                    width: 8.0,
                    height: 8.0,
                    sheet: None,
                    frame_width: 8.0,
                    frame_height: 8.0,
                })
            }
        }
        let scene = aigs_project::Scene {
            format: aigs_project::FormatHeader {
                kind: aigs_project::FormatKind::AigsScene,
                version: 0,
            },
            name: "s".to_string(),
            gravity: None,
            music: None,
            entities: vec![EntityNode {
                id: "counter-entity".to_string(),
                name: "Counter".to_string(),
                components: aigs_project::Components {
                    transform2d: Some(aigs_project::Transform2D::default()),
                    sprite: Some(aigs_project::Sprite {
                        asset: "sprite".to_string(),
                        frame: 0,
                        width: Some(8.0),
                        height: Some(8.0),
                        opacity: 1.0,
                        layer: 0,
                    }),
                    script: Some(aigs_project::Script {
                        asset: "counter".to_string(),
                    }),
                    ..Default::default()
                },
                children: vec![],
            }],
            animations: vec![],
        };
        let mut world = World::new();
        let scene_instance = crate::instantiate_scene(&mut world, &scene, &AnyTexture).unwrap();
        host.bind(&scene.entities, &scene_instance);

        let input = Input::default();
        host.tick(&world, &scene_instance, &input, 1.0 / 60.0);
        let mut frame = -1.0;
        world.for_each::<Sprite>(|_, sprite| frame = sprite.frame);
        assert_eq!(frame, 1.0, "first run sets frame to 1");

        // Change the script's behavior and force a newer mtime (filesystem
        // mtime resolution can be coarse, so push it comfortably forward).
        std::fs::write(
            &script_path,
            r#"
                fn on_start() { set_var("n", 9.0); }
                fn on_update(dt) { set_frame(get_var("n")); }
            "#,
        )
        .unwrap();
        // Open with write access: on Windows, setting file times needs
        // FILE_WRITE_ATTRIBUTES, which a read-only handle doesn't carry.
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&script_path)
            .unwrap();
        file.set_modified(SystemTime::now() + std::time::Duration::from_secs(5))
            .unwrap();

        let reloaded = host.check_reload();
        assert_eq!(reloaded, vec!["counter".to_string()]);

        host.tick(&world, &scene_instance, &input, 1.0 / 60.0);
        let mut frame_after = -1.0;
        world.for_each::<Sprite>(|_, sprite| frame_after = sprite.frame);
        assert_eq!(
            frame_after, 9.0,
            "reloaded script runs the new code with reset state"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
