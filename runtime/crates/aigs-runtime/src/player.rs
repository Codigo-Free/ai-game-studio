//! Game player (milestone M5): drives a whole project — multiple scenes,
//! animation playback, code-free behaviors and scene switching.

use std::collections::HashMap;

use aigs_ecs::{Entity, World};
use aigs_project::{ActionSpec, EntityNode, EventSpec, Project, Scene};

use crate::audio::AudioPlayer;
use crate::components::{Camera2D, Sprite, Transform2D};
use crate::input::Input;
use crate::physics::PhysicsWorld;
use crate::playback::AnimationPlayback;
use crate::scene::{instantiate_scene, ResolveTexture, SceneError, SceneInstance};
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
        world: &mut World,
    ) -> Result<Self, PlayerError> {
        let mut player = Self {
            scenes,
            textures,
            audio,
            current: String::new(),
            instance: SceneInstance::default(),
            playback: AnimationPlayback::default(),
            physics: None,
            behaviors: Vec::new(),
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
        let scene = self
            .scenes
            .get(path)
            .ok_or_else(|| PlayerError::UnknownScene(path.to_string()))?;
        world.clear();
        let instance = instantiate_scene(world, scene, &self.textures)?;
        self.playback = AnimationPlayback::bind(scene, &instance);
        self.warnings = self.playback.warnings().to_vec();
        self.behaviors = bind_behaviors(&scene.entities, &instance, &mut self.warnings);
        self.physics = PhysicsWorld::build(world, scene.gravity.unwrap_or_default());
        self.audio.set_music(scene.music.as_ref());
        self.warnings.extend(self.audio.take_warnings());
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
            self.run_action(world, entity, &action, continuous, time.delta);
        }

        // Physics step (after behaviors so kinematic bodies follow input),
        // then collision behaviors fire within the same tick.
        if let Some(physics) = self.physics.as_mut() {
            let contacts = physics.step(world, time.delta);
            if !contacts.is_empty() {
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
                    self.run_action(world, entity, &action, false, time.delta);
                }
            }
        }

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
        }
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
    fn parse_key_accepts_common_names() {
        assert_eq!(parse_key("ArrowLeft"), Some(KeyCode::ArrowLeft));
        assert_eq!(parse_key("Space"), Some(KeyCode::Space));
        assert_eq!(parse_key("a"), Some(KeyCode::KeyA));
        assert_eq!(parse_key("KeyZ"), Some(KeyCode::KeyZ));
        assert_eq!(parse_key("Digit3"), Some(KeyCode::Digit3));
        assert_eq!(parse_key("NotAKey"), None);
    }
}
