//! Game player (milestone M5): drives a whole project — multiple scenes,
//! animation playback, code-free behaviors and scene switching.

use std::collections::HashMap;

use aigs_ecs::{Entity, World};
use aigs_project::{ActionSpec, Animator, EntityNode, EventSpec, Project, Scene};

use crate::audio::AudioPlayer;
use crate::components::{Camera2D, Sprite, Transform2D};
use crate::input::Input;
use crate::particles;
use crate::physics::PhysicsWorld;
use crate::playback::AnimationPlayback;
use crate::scene::{instantiate_scene, ResolveTexture, SceneError, SceneInstance};
use crate::scripting::{ScriptCommand, ScriptHost};
use crate::time::Time;
use crate::KeyCode;

#[derive(Debug, thiserror::Error)]
pub enum PlayerError {
    #[error("scene \"{0}\" is not loaded in the player")]
    UnknownScene(String),
    #[error(transparent)]
    Scene(#[from] SceneError),
}

struct BoundBehavior {
    entity: Entity,
    on: EventSpec,
    action: ActionSpec,
}

/// A running animation state machine (one per entity with an `animator`).
struct RunningAnimator {
    spec: Animator,
    current: String,
}

/// Runs a project: owns the loaded scenes, the current world population,
/// the animation playback and the behavior rules.
pub struct GamePlayer<R: ResolveTexture> {
    scenes: HashMap<String, Scene>,
    textures: R,
    audio: AudioPlayer,
    current: String,
    instance: SceneInstance,
    playback: AnimationPlayback,
    physics: Option<PhysicsWorld>,
    behaviors: Vec<BoundBehavior>,
    animators: Vec<RunningAnimator>,
    scripts: ScriptHost,
    pending_scene: Option<String>,
    scene_started: bool,
    warnings: Vec<String>,
}

impl<R: ResolveTexture> GamePlayer<R> {
    /// Creates the player and loads the project's initial scene into `world`.
    /// `scenes` maps manifest-relative paths to parsed scenes.
    pub fn new(
        project: &Project,
        scenes: HashMap<String, Scene>,
        textures: R,
        audio: AudioPlayer,
        scripts: ScriptHost,
        world: &mut World,
    ) -> Result<Self, PlayerError> {
        let mut player = Self {
            scenes,
            textures,
            audio,
            scripts,
            current: String::new(),
            instance: SceneInstance::default(),
            playback: AnimationPlayback::default(),
            physics: None,
            behaviors: Vec::new(),
            animators: Vec::new(),
            pending_scene: None,
            scene_started: false,
            warnings: Vec::new(),
        };
        player.load_scene(world, &project.initial_scene)?;
        Ok(player)
    }

    /// Path of the scene currently running.
    pub fn current_scene(&self) -> &str {
        &self.current
    }

    /// Problems found while binding the current scene (unknown keys,
    /// entities or properties). Refreshed on every scene switch.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Number of animations in the current scene.
    pub fn animation_count(&self) -> usize {
        self.playback.len()
    }

    fn load_scene(&mut self, world: &mut World, path: &str) -> Result<(), PlayerError> {
        // Give the scene being left a chance to clean up before its
        // entities disappear (e.g. a final log message or sound). Captured
        // separately: `self.warnings` gets reset below for the new scene.
        let destroy_commands = self.scripts.dispatch_destroy(world);
        for (entity, command) in destroy_commands {
            self.apply_destroy_script_command(world, entity, command);
        }
        let destroy_warnings = self.scripts.take_warnings();

        let scene = self
            .scenes
            .get(path)
            .ok_or_else(|| PlayerError::UnknownScene(path.to_string()))?;
        world.clear();
        let instance = instantiate_scene(world, scene, &self.textures)?;
        self.playback = AnimationPlayback::bind(scene, &instance);
        self.warnings = self.playback.warnings().to_vec();
        self.warnings.extend(destroy_warnings);
        self.behaviors = bind_behaviors(&scene.entities, &instance, &mut self.warnings);
        self.physics = PhysicsWorld::build(world, scene.gravity.unwrap_or_default());

        // Animation state machines: their animations don't autostart — the
        // machine drives them, beginning with the initial state.
        self.animators = collect_animators(&scene.entities, &mut self.warnings);
        for animator in &self.animators {
            for animation in animator.spec.states.values() {
                self.playback.stop(animation);
            }
        }
        for animator in &self.animators {
            if let Some(animation) = animator.spec.states.get(&animator.current) {
                self.playback.restart(animation);
            } else {
                self.warnings.push(format!(
                    "animator: initial state \"{}\" is not in states",
                    animator.current
                ));
            }
        }

        self.audio.set_music(scene.music.as_ref());
        self.warnings.extend(self.audio.take_warnings());
        self.scripts.bind(&scene.entities, &instance);
        self.warnings.extend(self.scripts.take_warnings());
        self.instance = instance;
        self.current = path.to_string();
        self.scene_started = false;
        Ok(())
    }

    /// Runs one simulation tick: animations, behaviors and scene switches.
    pub fn update(&mut self, world: &mut World, time: &Time, input: &Input) {
        let started = !self.scene_started;
        self.scene_started = true;

        let finished = self.playback.advance(world, time.delta);

        let clicked = if input.mouse_just_pressed(crate::MouseButton::Left) {
            hit_test(world, input)
        } else {
            None
        };

        // Collect first (checking events borrows the behavior list), then run.
        let mut to_run: Vec<(Entity, ActionSpec, bool)> = Vec::new();
        for behavior in &self.behaviors {
            if !world.is_alive(behavior.entity) {
                continue;
            }
            let (fires, continuous) = match &behavior.on {
                EventSpec::KeyDown { key } => (
                    parse_key(key).is_some_and(|code| input.key_pressed(code)),
                    true,
                ),
                EventSpec::KeyPressed { key } => (
                    parse_key(key).is_some_and(|code| input.key_just_pressed(code)),
                    false,
                ),
                EventSpec::KeyReleased { key } => (
                    parse_key(key).is_some_and(|code| input.key_just_released(code)),
                    false,
                ),
                EventSpec::Click => (clicked == Some(behavior.entity), false),
                EventSpec::SceneStart => (started, false),
                EventSpec::AnimationEnd { animation } => {
                    (finished.iter().any(|name| name == animation), false)
                }
                EventSpec::Collision { .. } => (false, false),
            };
            if fires {
                to_run.push((behavior.entity, behavior.action.clone(), continuous));
            }
        }
        for (entity, action, continuous) in to_run {
            if let ActionSpec::EmitParticles { count } = &action {
                if !particles::burst(world, entity, *count) {
                    self.warnings
                        .push("emit_particles: entity has no particle emitter".to_string());
                }
                continue;
            }
            self.run_action(world, entity, &action, continuous, time.delta);
        }

        // Animation state machines: evaluate transitions on input events.
        for animator in &mut self.animators {
            for transition in &animator.spec.transitions {
                if transition.from != animator.current && transition.from != "any" {
                    continue;
                }
                if transition.to == animator.current {
                    continue;
                }
                let fires = match &transition.when {
                    EventSpec::KeyDown { key } => {
                        parse_key(key).is_some_and(|code| input.key_pressed(code))
                    }
                    EventSpec::KeyPressed { key } => {
                        parse_key(key).is_some_and(|code| input.key_just_pressed(code))
                    }
                    EventSpec::KeyReleased { key } => {
                        parse_key(key).is_some_and(|code| input.key_just_released(code))
                    }
                    EventSpec::SceneStart => started,
                    EventSpec::AnimationEnd { animation } => {
                        finished.iter().any(|name| name == animation)
                    }
                    // Click/collision transitions are not supported in v0.
                    _ => false,
                };
                if fires {
                    if let Some(old) = animator.spec.states.get(&animator.current) {
                        self.playback.stop(old);
                    }
                    animator.current = transition.to.clone();
                    if let Some(new) = animator.spec.states.get(&animator.current) {
                        self.playback.restart(new);
                    }
                    break;
                }
            }
        }

        // User scripts (after behaviors/animators, before physics so their
        // movement drives kinematic bodies this same tick).
        let script_commands = self.scripts.tick(world, &self.instance, input, time.delta);
        for (entity, command) in script_commands {
            self.apply_script_command(world, entity, command);
        }
        self.warnings.extend(self.scripts.take_warnings());
        self.warnings.extend(self.audio.take_warnings());

        // Physics step (after behaviors so kinematic bodies follow input),
        // then collision behaviors and scripts fire within the same tick.
        if let Some(physics) = self.physics.as_mut() {
            let contacts = physics.step(world, time.delta);
            if !contacts.is_empty() {
                let script_commands =
                    self.scripts
                        .dispatch_collisions(world, &self.instance, &contacts);
                for (entity, command) in script_commands {
                    self.apply_script_command(world, entity, command);
                }
                self.warnings.extend(self.scripts.take_warnings());

                let mut collision_actions: Vec<(Entity, ActionSpec)> = Vec::new();
                for behavior in &self.behaviors {
                    let EventSpec::Collision { with } = &behavior.on else {
                        continue;
                    };
                    if !world.is_alive(behavior.entity) {
                        continue;
                    }
                    let filter = with.as_ref().and_then(|id| self.instance.entity(id));
                    let touched = contacts.iter().any(|(a, b)| {
                        let other = if *a == behavior.entity {
                            Some(*b)
                        } else if *b == behavior.entity {
                            Some(*a)
                        } else {
                            None
                        };
                        match (other, with) {
                            (Some(other), Some(_)) => filter == Some(other),
                            (Some(_), None) => true,
                            (None, _) => false,
                        }
                    });
                    if touched {
                        collision_actions.push((behavior.entity, behavior.action.clone()));
                    }
                }
                for (entity, action) in collision_actions {
                    if let ActionSpec::EmitParticles { count } = &action {
                        if !particles::burst(world, entity, *count) {
                            self.warnings
                                .push("emit_particles: entity has no particle emitter".to_string());
                        }
                        continue;
                    }
                    self.run_action(world, entity, &action, false, time.delta);
                }
            }
        }

        particles::tick(world, time.delta);

        if let Some(path) = self.pending_scene.take() {
            if let Err(error) = self.load_scene(world, &path) {
                self.warnings.push(error.to_string());
            }
        }
    }

    fn run_action(
        &mut self,
        world: &World,
        entity: Entity,
        action: &ActionSpec,
        continuous: bool,
        dt: f32,
    ) {
        match action {
            ActionSpec::Move { dx, dy } => {
                if let Some(mut transform) = world.get_mut::<Transform2D>(entity) {
                    let factor = if continuous { dt } else { 1.0 };
                    transform.x += dx * factor;
                    transform.y += dy * factor;
                }
            }
            ActionSpec::GotoScene { scene } => {
                self.pending_scene = Some(scene.clone());
            }
            ActionSpec::PlayAnimation { animation } => {
                if !self.playback.restart(animation) {
                    self.warnings
                        .push(format!("play_animation: unknown animation \"{animation}\""));
                }
            }
            ActionSpec::PlaySound { asset, volume } => {
                self.audio.play_sound(asset, *volume);
                self.warnings.extend(self.audio.take_warnings());
            }
            ActionSpec::EmitParticles { .. } => {
                // Needs &mut World; handled by the caller (see `update`).
            }
        }
    }

    /// Applies an engine-level command a script queued (`goto_scene`,
    /// `play_animation`, `play_sound`, `emit_particles`). Shared by
    /// `on_update`/`on_collision` dispatch.
    fn apply_script_command(&mut self, world: &mut World, entity: Entity, command: ScriptCommand) {
        match command {
            ScriptCommand::GotoScene(path) => self.pending_scene = Some(path),
            ScriptCommand::PlayAnimation(name) => {
                if !self.playback.restart(&name) {
                    self.warnings
                        .push(format!("script: unknown animation \"{name}\""));
                }
            }
            ScriptCommand::PlaySound(name, volume) => {
                self.audio.play_sound(&name, volume);
            }
            ScriptCommand::EmitParticles(count) => {
                if !particles::burst(world, entity, count) {
                    self.warnings
                        .push("script: entity has no particle emitter".to_string());
                }
            }
        }
    }

    /// Same as [`Self::apply_script_command`], but for `on_destroy`: a
    /// scene switch is already underway, so `goto_scene` from on_destroy is
    /// rejected (with a warning) instead of chaining into another switch.
    fn apply_destroy_script_command(
        &mut self,
        world: &mut World,
        entity: Entity,
        command: ScriptCommand,
    ) {
        if matches!(command, ScriptCommand::GotoScene(_)) {
            self.warnings
                .push("script: goto_scene from on_destroy is ignored".to_string());
            return;
        }
        self.apply_script_command(world, entity, command);
    }
}

fn collect_animators(entities: &[EntityNode], warnings: &mut Vec<String>) -> Vec<RunningAnimator> {
    let mut animators = Vec::new();
    collect_animators_rec(entities, warnings, &mut animators);
    animators
}

fn collect_animators_rec(
    entities: &[EntityNode],
    warnings: &mut Vec<String>,
    into: &mut Vec<RunningAnimator>,
) {
    for node in entities {
        if let Some(spec) = &node.components.animator {
            for transition in &spec.transitions {
                if transition.from != "any" && !spec.states.contains_key(&transition.from) {
                    warnings.push(format!(
                        "animator \"{}\": transition from unknown state \"{}\"",
                        node.id, transition.from
                    ));
                }
                if !spec.states.contains_key(&transition.to) {
                    warnings.push(format!(
                        "animator \"{}\": transition to unknown state \"{}\"",
                        node.id, transition.to
                    ));
                }
            }
            into.push(RunningAnimator {
                current: spec.initial.clone(),
                spec: spec.clone(),
            });
        }
        collect_animators_rec(&node.children, warnings, into);
    }
}

fn bind_behaviors(
    entities: &[EntityNode],
    instance: &SceneInstance,
    warnings: &mut Vec<String>,
) -> Vec<BoundBehavior> {
    let mut bound = Vec::new();
    collect_behaviors(entities, instance, warnings, &mut bound);
    bound
}

fn collect_behaviors(
    entities: &[EntityNode],
    instance: &SceneInstance,
    warnings: &mut Vec<String>,
    into: &mut Vec<BoundBehavior>,
) {
    for node in entities {
        if let Some(entity) = instance.entity(&node.id) {
            for behavior in &node.components.behaviors {
                if let EventSpec::KeyDown { key } | EventSpec::KeyPressed { key } = &behavior.on {
                    if parse_key(key).is_none() {
                        warnings.push(format!(
                            "entity \"{}\": unknown key name \"{key}\"",
                            node.id
                        ));
                    }
                }
                into.push(BoundBehavior {
                    entity,
                    on: behavior.on.clone(),
                    action: behavior.action.clone(),
                });
            }
        }
        collect_behaviors(&node.children, instance, warnings, into);
    }
}

/// Topmost sprite under the cursor, mapped through the scene camera.
fn hit_test(world: &World, input: &Input) -> Option<Entity> {
    let (view_w, view_h) = input.viewport();
    if view_w <= 0.0 || view_h <= 0.0 {
        return None;
    }
    let mut camera = (0.0f32, 0.0f32, 1.0f32);
    let mut found_camera = false;
    world.for_each2::<Transform2D, Camera2D>(|_, transform, cam| {
        if !found_camera {
            camera = (transform.x, transform.y, cam.zoom.max(0.0001));
            found_camera = true;
        }
    });
    let (mx, my) = input.mouse_position();
    let world_x = camera.0 + (mx - view_w / 2.0) / camera.2;
    let world_y = camera.1 + (view_h / 2.0 - my) / camera.2;

    let mut best: Option<(i32, u32, Entity)> = None;
    world.for_each2::<Transform2D, Sprite>(|entity, transform, sprite| {
        let radians = transform.rotation.to_radians();
        let (sin, cos) = radians.sin_cos();
        let dx = world_x - transform.x;
        let dy = world_y - transform.y;
        let local_x = (dx * cos - dy * sin) / transform.scale_x.max(0.0001);
        let local_y = (dx * sin + dy * cos) / transform.scale_y.max(0.0001);
        if local_x.abs() <= sprite.width / 2.0 && local_y.abs() <= sprite.height / 2.0 {
            let key = (sprite.layer, entity.index());
            if best.is_none_or(|(layer, index, _)| key >= (layer, index)) {
                best = Some((sprite.layer, entity.index(), entity));
            }
        }
    });
    best.map(|(_, _, entity)| entity)
}

/// Maps `.aigs` key names to winit key codes. Accepts winit-style names
/// (`ArrowLeft`, `Space`, `KeyA`, `Digit1`) plus single characters (`a`, `1`).
pub fn parse_key(name: &str) -> Option<KeyCode> {
    use KeyCode::*;
    let key = match name {
        "ArrowLeft" | "Left" => ArrowLeft,
        "ArrowRight" | "Right" => ArrowRight,
        "ArrowUp" | "Up" => ArrowUp,
        "ArrowDown" | "Down" => ArrowDown,
        "Space" | " " => Space,
        "Enter" => Enter,
        "Escape" => Escape,
        "Tab" => Tab,
        "ShiftLeft" => ShiftLeft,
        "ShiftRight" => ShiftRight,
        "ControlLeft" => ControlLeft,
        "ControlRight" => ControlRight,
        other => {
            let normalized = other.strip_prefix("Key").unwrap_or(other);
            if normalized.len() == 1 {
                let ch = normalized.chars().next().unwrap().to_ascii_uppercase();
                return match ch {
                    'A' => Some(KeyA),
                    'B' => Some(KeyB),
                    'C' => Some(KeyC),
                    'D' => Some(KeyD),
                    'E' => Some(KeyE),
                    'F' => Some(KeyF),
                    'G' => Some(KeyG),
                    'H' => Some(KeyH),
                    'I' => Some(KeyI),
                    'J' => Some(KeyJ),
                    'K' => Some(KeyK),
                    'L' => Some(KeyL),
                    'M' => Some(KeyM),
                    'N' => Some(KeyN),
                    'O' => Some(KeyO),
                    'P' => Some(KeyP),
                    'Q' => Some(KeyQ),
                    'R' => Some(KeyR),
                    'S' => Some(KeyS),
                    'T' => Some(KeyT),
                    'U' => Some(KeyU),
                    'V' => Some(KeyV),
                    'W' => Some(KeyW),
                    'X' => Some(KeyX),
                    'Y' => Some(KeyY),
                    'Z' => Some(KeyZ),
                    '0' => Some(Digit0),
                    '1' => Some(Digit1),
                    '2' => Some(Digit2),
                    '3' => Some(Digit3),
                    '4' => Some(Digit4),
                    '5' => Some(Digit5),
                    '6' => Some(Digit6),
                    '7' => Some(Digit7),
                    '8' => Some(Digit8),
                    '9' => Some(Digit9),
                    _ => None,
                };
            }
            let digit = other.strip_prefix("Digit").and_then(|d| d.chars().next());
            if let Some(ch) = digit {
                return parse_key(&ch.to_string());
            }
            return None;
        }
    };
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TextureInfo;

    struct AnyTexture;

    impl ResolveTexture for AnyTexture {
        fn resolve(&self, _: &str) -> Option<TextureInfo> {
            Some(TextureInfo {
                id: Default::default(),
                width: 32.0,
                height: 32.0,
                sheet: None,
                frame_width: 32.0,
                frame_height: 32.0,
            })
        }
    }

    fn project() -> (Project, HashMap<String, Scene>) {
        let project = Project::from_json(
            r#"{
                "format": { "kind": "aigs-project", "version": 0 },
                "name": "Demo",
                "initial_scene": "menu",
                "scenes": ["menu", "level"]
            }"#,
        )
        .unwrap();
        let menu = Scene::from_json(
            r#"{
                "format": { "kind": "aigs-scene", "version": 0 },
                "name": "menu",
                "entities": [
                    { "id": "camera", "name": "Cam",
                      "components": { "transform2d": {}, "camera2d": {} } },
                    { "id": "start", "name": "Start",
                      "components": {
                        "transform2d": { "x": 0.0, "y": 0.0 },
                        "sprite": { "asset": "button" },
                        "behaviors": [
                            { "on": { "type": "click" },
                              "do": { "type": "goto_scene", "scene": "level" } },
                            { "on": { "type": "scene_start" },
                              "do": { "type": "move", "dx": 7.0, "dy": 0.0 } }
                        ]
                      } }
                ]
            }"#,
        )
        .unwrap();
        let level = Scene::from_json(
            r#"{
                "format": { "kind": "aigs-scene", "version": 0 },
                "name": "level",
                "entities": [
                    { "id": "robot", "name": "Robot",
                      "components": {
                        "transform2d": {},
                        "sprite": { "asset": "robot" },
                        "behaviors": [
                            { "on": { "type": "key_down", "key": "ArrowRight" },
                              "do": { "type": "move", "dx": 100.0, "dy": 0.0 } },
                            { "on": { "type": "key_pressed", "key": "Escape" },
                              "do": { "type": "goto_scene", "scene": "menu" } }
                        ]
                      } }
                ]
            }"#,
        )
        .unwrap();
        let mut scenes = HashMap::new();
        scenes.insert("menu".to_string(), menu);
        scenes.insert("level".to_string(), level);
        (project, scenes)
    }

    fn tick_time() -> Time {
        Time {
            delta: 0.1,
            ..Time::default()
        }
    }

    #[test]
    fn scene_start_fires_once() {
        let (project, scenes) = project();
        let mut world = World::new();
        let mut player = GamePlayer::new(
            &project,
            scenes,
            AnyTexture,
            AudioPlayer::disabled(),
            ScriptHost::empty(),
            &mut world,
        )
        .unwrap();
        let input = Input::default();
        player.update(&mut world, &tick_time(), &input);
        player.update(&mut world, &tick_time(), &input);
        // Discrete move: applied exactly once, as an instant offset.
        let mut x = None;
        world.for_each::<Transform2D>(|_, t| {
            if t.x != 0.0 {
                x = Some(t.x);
            }
        });
        assert_eq!(x, Some(7.0));
    }

    #[test]
    fn key_down_moves_continuously_and_escape_switches_back() {
        let (project, scenes) = project();
        let mut world = World::new();
        let mut player = GamePlayer::new(
            &project,
            scenes,
            AnyTexture,
            AudioPlayer::disabled(),
            ScriptHost::empty(),
            &mut world,
        )
        .unwrap();

        // Go to the level first.
        let mut input = Input::default();
        input.set_viewport(800.0, 600.0);
        input.set_mouse_position(400.0 + 7.0, 300.0); // over the button (moved by scene_start... not yet: click happens after start on same tick)
        player.update(&mut world, &tick_time(), &input); // scene_start moves button
        input.simulate_mouse(crate::MouseButton::Left);
        player.update(&mut world, &tick_time(), &input);
        assert_eq!(player.current_scene(), "level");
        input.simulate_end_tick();

        // Hold ArrowRight for two ticks: 100 units/s * 0.2 s.
        input.simulate_key(KeyCode::ArrowRight, true);
        player.update(&mut world, &tick_time(), &input);
        input.simulate_end_tick();
        input.simulate_key(KeyCode::ArrowRight, true);
        player.update(&mut world, &tick_time(), &input);
        let mut x = 0.0;
        world.for_each::<Transform2D>(|_, t| x = t.x);
        assert!((x - 20.0).abs() < 0.01, "expected 20, got {x}");
        input.simulate_end_tick();

        // Escape returns to the menu.
        input.simulate_key(KeyCode::Escape, true);
        player.update(&mut world, &tick_time(), &input);
        assert_eq!(player.current_scene(), "menu");
        assert_eq!(world.len(), 2, "menu entities repopulated");
    }

    #[test]
    fn unknown_key_names_are_reported() {
        let (project, mut scenes) = project();
        scenes.insert(
            "menu".to_string(),
            Scene::from_json(
                r#"{
                    "format": { "kind": "aigs-scene", "version": 0 },
                    "name": "menu",
                    "entities": [{ "id": "e", "name": "E",
                      "components": { "behaviors": [
                        { "on": { "type": "key_down", "key": "NotAKey" },
                          "do": { "type": "move", "dx": 1.0, "dy": 0.0 } }
                      ] } }]
                }"#,
            )
            .unwrap(),
        );
        let mut world = World::new();
        let player = GamePlayer::new(
            &project,
            scenes,
            AnyTexture,
            AudioPlayer::disabled(),
            ScriptHost::empty(),
            &mut world,
        )
        .unwrap();
        assert_eq!(player.warnings().len(), 1);
    }

    fn physics_project(crate_behaviors: &str, sensor: bool) -> (Project, HashMap<String, Scene>) {
        let project = Project::from_json(
            r#"{
                "format": { "kind": "aigs-project", "version": 0 },
                "name": "Physics",
                "initial_scene": "fall",
                "scenes": ["fall", "done"]
            }"#,
        )
        .unwrap();
        let fall = Scene::from_json(&format!(
            r#"{{
                "format": {{ "kind": "aigs-scene", "version": 0 }},
                "name": "fall",
                "gravity": {{ "x": 0.0, "y": -980.0 }},
                "entities": [
                    {{ "id": "crate", "name": "Crate",
                      "components": {{
                        "transform2d": {{ "x": 0.0, "y": 200.0 }},
                        "sprite": {{ "asset": "crate", "width": 32.0, "height": 32.0 }},
                        "rigidbody2d": {{ "fixed_rotation": true }},
                        "collider2d": {{}},
                        "behaviors": [{crate_behaviors}]
                      }} }},
                    {{ "id": "floor", "name": "Floor",
                      "components": {{
                        "transform2d": {{ "x": 0.0, "y": 0.0 }},
                        "collider2d": {{ "width": 800.0, "height": 40.0, "sensor": {sensor} }}
                      }} }}
                ]
            }}"#
        ))
        .unwrap();
        let done = Scene::from_json(
            r#"{
                "format": { "kind": "aigs-scene", "version": 0 },
                "name": "done",
                "entities": []
            }"#,
        )
        .unwrap();
        let mut scenes = HashMap::new();
        scenes.insert("fall".to_string(), fall);
        scenes.insert("done".to_string(), done);
        (project, scenes)
    }

    fn crate_y(world: &World) -> f32 {
        let mut y = f32::NAN;
        world.for_each2::<Transform2D, crate::Sprite>(|_, t, _| y = t.y);
        y
    }

    #[test]
    fn dynamic_body_falls_and_rests_on_the_floor() {
        let (project, scenes) = physics_project("", false);
        let mut world = World::new();
        let mut player = GamePlayer::new(
            &project,
            scenes,
            AnyTexture,
            AudioPlayer::disabled(),
            ScriptHost::empty(),
            &mut world,
        )
        .unwrap();
        let input = Input::default();
        let time = Time {
            delta: 1.0 / 60.0,
            ..Time::default()
        };
        for _ in 0..240 {
            player.update(&mut world, &time, &input);
        }
        let y = crate_y(&world);
        // Rests on the floor: floor top (20) + half box (16) = 36.
        assert!((y - 36.0).abs() < 2.0, "crate should rest at ~36, got {y}");
    }

    #[test]
    fn collision_behavior_switches_scene() {
        let behaviors = r#"{ "on": { "type": "collision", "with": "floor" },
                             "do": { "type": "goto_scene", "scene": "done" } }"#;
        let (project, scenes) = physics_project(behaviors, false);
        let mut world = World::new();
        let mut player = GamePlayer::new(
            &project,
            scenes,
            AnyTexture,
            AudioPlayer::disabled(),
            ScriptHost::empty(),
            &mut world,
        )
        .unwrap();
        let input = Input::default();
        let time = Time {
            delta: 1.0 / 60.0,
            ..Time::default()
        };
        for _ in 0..240 {
            player.update(&mut world, &time, &input);
            if player.current_scene() == "done" {
                break;
            }
        }
        assert_eq!(player.current_scene(), "done", "collision must fire");
    }

    #[test]
    fn sensor_detects_without_blocking() {
        let behaviors = r#"{ "on": { "type": "collision" },
                             "do": { "type": "move", "dx": 500.0, "dy": 0.0 } }"#;
        let (project, scenes) = physics_project(behaviors, true);
        let mut world = World::new();
        let mut player = GamePlayer::new(
            &project,
            scenes,
            AnyTexture,
            AudioPlayer::disabled(),
            ScriptHost::empty(),
            &mut world,
        )
        .unwrap();
        let input = Input::default();
        let time = Time {
            delta: 1.0 / 60.0,
            ..Time::default()
        };
        for _ in 0..240 {
            player.update(&mut world, &time, &input);
        }
        let y = crate_y(&world);
        assert!(y < -100.0, "sensor must not block the fall, y = {y}");
    }

    #[test]
    fn animator_switches_states_on_input() {
        let project = Project::from_json(
            r#"{
                "format": { "kind": "aigs-project", "version": 0 },
                "name": "Anim", "initial_scene": "s", "scenes": ["s"]
            }"#,
        )
        .unwrap();
        let scene = Scene::from_json(
            r#"{
                "format": { "kind": "aigs-scene", "version": 0 },
                "name": "s",
                "entities": [{
                    "id": "walker", "name": "Walker",
                    "components": {
                        "transform2d": {},
                        "sprite": { "asset": "walker" },
                        "animator": {
                            "initial": "idle",
                            "states": { "idle": "anim-idle", "walk": "anim-walk" },
                            "transitions": [
                                { "from": "idle", "to": "walk",
                                  "when": { "type": "key_down", "key": "ArrowRight" } },
                                { "from": "walk", "to": "idle",
                                  "when": { "type": "key_released", "key": "ArrowRight" } }
                            ]
                        }
                    }
                }],
                "animations": [
                    { "name": "anim-idle", "fps": 10, "loop": true,
                      "tracks": [{ "entity": "walker", "property": "sprite.frame",
                        "keyframes": [ { "frame": 0, "value": 0.0 }, { "frame": 2, "value": 0.0 } ] }] },
                    { "name": "anim-walk", "fps": 10, "loop": true,
                      "tracks": [{ "entity": "walker", "property": "sprite.frame",
                        "keyframes": [ { "frame": 0, "value": 2.0 }, { "frame": 4, "value": 5.9 } ] }] }
                ]
            }"#,
        )
        .unwrap();
        let mut scenes = HashMap::new();
        scenes.insert("s".to_string(), scene);
        let mut world = World::new();
        let mut player = GamePlayer::new(
            &project,
            scenes,
            AnyTexture,
            AudioPlayer::disabled(),
            ScriptHost::empty(),
            &mut world,
        )
        .unwrap();
        let mut input = Input::default();
        let time = tick_time();

        let frame = |world: &World| {
            let mut value = -1.0;
            world.for_each::<crate::Sprite>(|_, sprite| value = sprite.frame);
            value
        };

        // Initial state: idle animation runs (frame stays 0).
        player.update(&mut world, &time, &input);
        assert_eq!(frame(&world), 0.0, "idle keeps frame 0");

        // Hold right: walk state animates frames >= 2.
        input.simulate_key(KeyCode::ArrowRight, true);
        player.update(&mut world, &time, &input);
        input.simulate_end_tick();
        player.update(&mut world, &time, &input);
        assert!(
            frame(&world) >= 2.0,
            "walk frames start at 2, got {}",
            frame(&world)
        );

        // Release: back to idle. The transition happens this tick; the idle
        // animation applies its values from the next advance on.
        input.simulate_key(KeyCode::ArrowRight, false);
        player.update(&mut world, &time, &input);
        input.simulate_end_tick();
        player.update(&mut world, &time, &input);
        assert_eq!(frame(&world), 0.0, "released -> idle resets frame");
    }

    #[test]
    fn script_drives_an_entity_and_reports_errors() {
        let project = Project::from_json(
            r#"{
                "format": { "kind": "aigs-project", "version": 0 },
                "name": "Scripted", "initial_scene": "s", "scenes": ["s", "done"]
            }"#,
        )
        .unwrap();
        let scene = Scene::from_json(
            r#"{
                "format": { "kind": "aigs-scene", "version": 0 },
                "name": "s",
                "entities": [
                    { "id": "drone", "name": "Drone",
                      "components": { "transform2d": { "x": 0.0 }, "script": { "asset": "patrol" } } },
                    { "id": "broken", "name": "Broken",
                      "components": { "transform2d": {}, "script": { "asset": "boom" } } },
                    { "id": "target", "name": "Target",
                      "components": { "transform2d": { "x": 500.0, "y": 0.0 } } }
                ]
            }"#,
        )
        .unwrap();
        let done = Scene::from_json(
            r#"{ "format": { "kind": "aigs-scene", "version": 0 }, "name": "done", "entities": [] }"#,
        )
        .unwrap();
        let mut scenes = HashMap::new();
        scenes.insert("s".to_string(), scene);
        scenes.insert("done".to_string(), done);

        let mut host = ScriptHost::empty();
        host.add_source(
            "patrol",
            r#"
                fn on_start() { set_pos(100.0, 0.0); }
                fn on_update(dt) {
                    move_by(60.0 * dt, 0.0);
                    if distance_to("target") < 340.0 { goto_scene("done"); }
                }
            "#,
        );
        host.add_source("boom", r#"fn on_update(dt) { this_does_not_exist(); }"#);

        let mut world = World::new();
        let mut player = GamePlayer::new(
            &project,
            scenes,
            AnyTexture,
            AudioPlayer::disabled(),
            host,
            &mut world,
        )
        .unwrap();
        let input = Input::default();
        let time = Time {
            delta: 0.1,
            ..Time::default()
        };

        // First tick: on_start teleports to 100, then moves 6 units.
        player.update(&mut world, &time, &input);
        let mut x = 0.0;
        world.for_each2::<Transform2D, crate::Sprite>(|_, _, _| {});
        world.for_each::<Transform2D>(|_, t| {
            if t.x > 50.0 && t.x < 200.0 {
                x = t.x;
            }
        });
        assert!((x - 106.0).abs() < 0.01, "expected 106, got {x}");
        // Broken script must warn exactly once and not crash the player.
        assert!(
            player.warnings().iter().any(|w| w.contains("boom")),
            "broken script warning expected: {:?}",
            player.warnings()
        );

        // Keep walking right until distance_to(target) < 340 -> goto done.
        for _ in 0..120 {
            player.update(&mut world, &time, &input);
            if player.current_scene() == "done" {
                break;
            }
        }
        assert_eq!(player.current_scene(), "done", "script goto_scene works");
    }

    #[test]
    fn script_on_collision_and_on_destroy_lifecycle_events() {
        let project = Project::from_json(
            r#"{
                "format": { "kind": "aigs-project", "version": 0 },
                "name": "Lifecycle", "initial_scene": "fall", "scenes": ["fall", "next"]
            }"#,
        )
        .unwrap();
        let fall = Scene::from_json(
            r#"{
                "format": { "kind": "aigs-scene", "version": 0 },
                "name": "fall",
                "gravity": { "x": 0.0, "y": -980.0 },
                "entities": [
                    { "id": "crate", "name": "Crate",
                      "components": {
                        "transform2d": { "x": 0.0, "y": 100.0 },
                        "sprite": { "asset": "crate", "width": 32.0, "height": 32.0 },
                        "rigidbody2d": { "fixed_rotation": true },
                        "collider2d": {},
                        "script": { "asset": "watcher" }
                      } },
                    { "id": "floor", "name": "Floor",
                      "components": {
                        "transform2d": {},
                        "collider2d": { "width": 800.0, "height": 40.0 }
                      } },
                    { "id": "switch", "name": "Switch",
                      "components": { "behaviors": [
                        { "on": { "type": "key_pressed", "key": "Enter" },
                          "do": { "type": "goto_scene", "scene": "next" } }
                      ] } }
                ]
            }"#,
        )
        .unwrap();
        let next = Scene::from_json(
            r#"{ "format": { "kind": "aigs-scene", "version": 0 }, "name": "next", "entities": [] }"#,
        )
        .unwrap();
        let mut scenes = HashMap::new();
        scenes.insert("fall".to_string(), fall);
        scenes.insert("next".to_string(), next);

        let mut host = ScriptHost::empty();
        host.add_source(
            "watcher",
            r#"
                fn on_collision(other) {
                    set_var("hit", 1.0);
                    log("touched " + other);
                }
                fn on_destroy() {
                    set_var("destroyed", 1.0);
                    this_does_not_exist();
                }
            "#,
        );

        let mut world = World::new();
        let mut player = GamePlayer::new(
            &project,
            scenes,
            AnyTexture,
            AudioPlayer::disabled(),
            host,
            &mut world,
        )
        .unwrap();
        let mut input = Input::default();
        let time = Time {
            delta: 1.0 / 60.0,
            ..Time::default()
        };
        for _ in 0..120 {
            player.update(&mut world, &time, &input);
        }
        // The crate must have hit the floor and fired on_collision; the
        // scene hasn't switched yet, so on_destroy must not have run.
        assert!(
            !player.warnings().iter().any(|w| w.contains("on_destroy")),
            "on_destroy must not fire while still in the same scene: {:?}",
            player.warnings()
        );

        // Now force a scene switch: on_destroy must fire for the scripted
        // entity being left behind, and its deliberate error must surface
        // as a warning without crashing the player.
        input.simulate_key(KeyCode::Enter, true);
        player.update(&mut world, &time, &input);
        assert_eq!(player.current_scene(), "next");
        assert!(
            player
                .warnings()
                .iter()
                .any(|w| w.contains("on_destroy") && w.contains("watcher")),
            "on_destroy must have run (and reported its error): {:?}",
            player.warnings()
        );
    }

    #[test]
    fn parse_key_accepts_common_names() {
        assert_eq!(parse_key("ArrowLeft"), Some(KeyCode::ArrowLeft));
        assert_eq!(parse_key("Space"), Some(KeyCode::Space));
        assert_eq!(parse_key("a"), Some(KeyCode::KeyA));
        assert_eq!(parse_key("KeyZ"), Some(KeyCode::KeyZ));
        assert_eq!(parse_key("Digit3"), Some(KeyCode::Digit3));
        assert_eq!(parse_key("NotAKey"), None);
    }
}
