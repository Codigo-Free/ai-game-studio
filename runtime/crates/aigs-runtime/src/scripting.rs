//! User scripting (milestone M12): rhai scripts attached to entities.
//!
//! A script asset may define `fn on_start()` (runs once after the scene
//! loads) and `fn on_update(dt)` (runs every simulation tick). Scripts read
//! the world through registered functions and mutate it through a command
//! queue drained after each call — the engine stays in control.
//!
//! The rhai engine is sandboxed: no file/network access and an operation
//! budget per call, so a runaway script degrades to a warning, not a hang.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use aigs_ecs::{Entity, World};
use aigs_project::{Asset, AssetKind, EntityNode};
use rhai::{Engine, ImmutableString, Scope, AST};

use crate::components::{Sprite, Transform2D};
use crate::input::Input;
use crate::player::parse_key;
use crate::scene::SceneInstance;
use crate::KeyCode;

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
}

/// Compiles the project's script assets and runs one instance per entity
/// with a `script` component.
pub struct ScriptHost {
    engine: Engine,
    shared: Rc<Shared>,
    asts: HashMap<String, Rc<AST>>,
    running: Vec<RunningScript>,
    warnings: Vec<String>,
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
            match std::fs::read_to_string(root.join(&asset.path)) {
                Ok(source) => host.add_source(&asset.id, &source),
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
            running: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Compiles a script from source under an asset id (also used by tests).
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
                        let has_start = ast.iter_functions().any(|f| f.name == "on_start");
                        let has_update = ast.iter_functions().any(|f| f.name == "on_update");
                        if !has_start && !has_update {
                            self.warnings.push(format!(
                                "script \"{}\": defines neither on_start nor on_update",
                                script.asset
                            ));
                        }
                        self.running.push(RunningScript {
                            entity,
                            entity_id: node.id.clone(),
                            asset: script.asset.clone(),
                            scope: Scope::new(),
                            started: false,
                            failed: false,
                            has_start,
                            has_update,
                        });
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

    /// Runs every script for this tick and returns the engine-level
    /// commands (scene switches, sounds, …) to apply.
    pub fn tick(
        &mut self,
        world: &World,
        instance: &SceneInstance,
        input: &Input,
        dt: f32,
    ) -> Vec<(Entity, ScriptCommand)> {
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
            // Per-entity snapshot.
            {
                let mut state = self.shared.self_state.borrow_mut();
                if let Some(transform) = world.get::<Transform2D>(script.entity) {
                    state.x = transform.x;
                    state.y = transform.y;
                    state.rotation = transform.rotation;
                }
                state.frame = world
                    .get::<Sprite>(script.entity)
                    .map(|sprite| sprite.frame)
                    .unwrap_or(0.0);
            }

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
                    continue;
                }
            }

            // Apply transform mutations directly; forward the rest.
            for command in self.shared.moves.borrow_mut().drain(..) {
                match command {
                    TransformCommand::SetPos(x, y) => {
                        if let Some(mut t) = world.get_mut::<Transform2D>(script.entity) {
                            t.x = x;
                            t.y = y;
                        }
                    }
                    TransformCommand::MoveBy(dx, dy) => {
                        if let Some(mut t) = world.get_mut::<Transform2D>(script.entity) {
                            t.x += dx;
                            t.y += dy;
                        }
                    }
                    TransformCommand::SetRotation(rotation) => {
                        if let Some(mut t) = world.get_mut::<Transform2D>(script.entity) {
                            t.rotation = rotation;
                        }
                    }
                    TransformCommand::SetFrame(frame) => {
                        if let Some(mut sprite) = world.get_mut::<Sprite>(script.entity) {
                            sprite.frame = frame;
                        }
                    }
                }
            }
            for command in self.shared.commands.borrow_mut().drain(..) {
                out.push((script.entity, command));
            }
            for message in self.shared.logs.borrow_mut().drain(..) {
                println!("[script:{}] {message}", script.asset);
            }
        }
        out
    }
}

/// Builds the sandboxed engine with the AI Game Studio scripting API.
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
