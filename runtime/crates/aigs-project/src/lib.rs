//! Loader, writer and validator of the `.aigs` project format.
//!
//! The `.aigs` format is the AI-Ready contract shared by the editor, the
//! runtime, the exporters and the AI agents. The normative specification
//! lives in `sdk/aigs-format/SPEC.md`; this crate is its reference
//! implementation. Everything the editor can do is expressed as this data.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub use aigs_anim::{Easing, Keyframe};

/// Version of the `.aigs` format implemented by this crate.
pub const FORMAT_VERSION: u32 = 0;

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("io error reading {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported format version {found} (this build supports up to {supported})")]
    UnsupportedVersion { found: u32, supported: u32 },
    #[error("format kind mismatch: expected {expected:?}, found {found:?}")]
    KindMismatch {
        expected: FormatKind,
        found: FormatKind,
    },
}

/// Discriminates the two document types of the format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FormatKind {
    AigsProject,
    AigsScene,
}

/// Header present in every `.aigs` document; drives versioning and migrations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatHeader {
    pub kind: FormatKind,
    pub version: u32,
}

impl FormatHeader {
    fn validate(&self, expected: FormatKind) -> Result<(), FormatError> {
        if self.kind != expected {
            return Err(FormatError::KindMismatch {
                expected,
                found: self.kind,
            });
        }
        if self.version > FORMAT_VERSION {
            return Err(FormatError::UnsupportedVersion {
                found: self.version,
                supported: FORMAT_VERSION,
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Project manifest (`game.aigs`)
// ---------------------------------------------------------------------------

/// Root manifest of a game project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub format: FormatHeader,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Scene shown when the game starts; must be listed in `scenes`.
    pub initial_scene: String,
    /// Scene file paths relative to the project root.
    pub scenes: Vec<String>,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    /// Unique id referenced by components (e.g. `sprite.asset`).
    pub id: String,
    pub kind: AssetKind,
    /// Path relative to the project root.
    pub path: String,
    /// Grid metadata turning an image into a spritesheet (milestone M10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spritesheet: Option<Spritesheet>,
}

/// Fixed-size frame grid over an image, row-major from the top-left.
/// Columns/rows derive from the texture size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spritesheet {
    pub frame_width: u32,
    pub frame_height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Image,
    Audio,
    Font,
    /// A rhai script (milestone M12).
    Script,
    Other,
}

// ---------------------------------------------------------------------------
// Scene documents (`*.scene.aigs`)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub format: FormatHeader,
    pub name: String,
    /// World gravity in units/s² applied to dynamic bodies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gravity: Option<Gravity>,
    /// Background music started when the scene loads (milestone M9).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub music: Option<Music>,
    #[serde(default)]
    pub entities: Vec<EntityNode>,
    #[serde(default)]
    pub animations: Vec<Animation>,
}

/// Scene background music: an `audio` asset id plus playback settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Music {
    /// Id of an `Asset` of kind `audio`.
    pub asset: String,
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Music loops by default.
    #[serde(default = "default_true")]
    pub looped: bool,
}

fn default_volume() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Gravity {
    pub x: f32,
    pub y: f32,
}

impl Default for Gravity {
    fn default() -> Self {
        Self { x: 0.0, y: -980.0 }
    }
}

/// Entity as authored in the editor: an id, a display name, its components
/// and its children (scene tree).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityNode {
    /// Unique id within the scene, referenced by animation tracks.
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub components: Components,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<EntityNode>,
}

/// Component set of an entity. Known components are typed; unknown keys are
/// preserved in `extra` so plugin components (namespaced, e.g. `"my_plugin.foo"`)
/// survive load/save round-trips.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Components {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform2d: Option<Transform2D>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite: Option<Sprite>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera2d: Option<Camera2D>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rigidbody2d: Option<Rigidbody2D>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collider2d: Option<Collider2D>,
    /// Animation state machine (milestone M10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animator: Option<Animator>,
    /// Particle emitter (milestone M11).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub particles: Option<Particles>,
    /// Flat-color box/circle primitive, no image asset needed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
    /// User script (milestone M12): a `script` asset driving this entity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<Script>,
    /// On-screen touch button (milestone M15).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub virtual_button: Option<VirtualButton>,
    /// Code-free event → action rules (see `Behavior`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behaviors: Vec<Behavior>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Animation state machine: named states mapped to scene animations, with
/// event-driven transitions. Animations referenced by an animator do NOT
/// autostart with the scene; the animator drives them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Animator {
    /// State active when the scene starts.
    pub initial: String,
    /// State name → scene animation name.
    pub states: BTreeMap<String, String>,
    #[serde(default)]
    pub transitions: Vec<Transition>,
}

/// Moves the machine from `from` to `to` when the event fires.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    /// Source state, or `"any"` to transition from every state.
    pub from: String,
    pub to: String,
    pub when: EventSpec,
}

/// Attaches a rhai script asset to the entity. The script may define
/// `fn on_start()` (runs once) and `fn on_update(dt)` (runs every tick).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Script {
    /// Id of an `Asset` of kind `script`.
    pub asset: String,
}

/// On-screen touch button (milestone M15): while held, simulates `key` as
/// pressed for every behavior/script — the same event as a physical key,
/// so existing keyboard-driven projects work on touch screens by adding
/// this component to a sprite, with no other changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualButton {
    /// Key name it simulates, same names `behaviors`/scripts use (e.g. `"ArrowLeft"`).
    pub key: String,
}

/// A code-free rule: when `on` happens, run `do`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Behavior {
    pub on: EventSpec,
    #[serde(rename = "do")]
    pub action: ActionSpec,
}

/// Events a behavior can react to (format v0).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventSpec {
    /// The key is held down (fires every tick).
    KeyDown { key: String },
    /// The key went down this tick.
    KeyPressed { key: String },
    /// The key went up this tick (milestone M10).
    KeyReleased { key: String },
    /// The entity was clicked with the left mouse button.
    Click,
    /// The scene just started (fires once).
    SceneStart,
    /// A non-looping animation of the scene just finished.
    AnimationEnd { animation: String },
    /// This entity started touching another collider (milestone M8).
    Collision {
        /// Optional filter: only fire when touching this entity id.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        with: Option<String>,
    },
}

/// Actions a behavior can run (format v0).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionSpec {
    /// Moves the entity. For continuous events (`key_down`) `dx`/`dy` are
    /// units per second; for discrete events they are an instant offset.
    Move { dx: f32, dy: f32 },
    /// Switches to another scene of the project (path as listed in `scenes`).
    GotoScene { scene: String },
    /// Restarts a scene animation by name.
    PlayAnimation { animation: String },
    /// Plays a sound effect: an `audio` asset id (milestone M9).
    PlaySound {
        asset: String,
        #[serde(default = "default_volume")]
        volume: f32,
    },
    /// Spawns a burst from this entity's particle emitter (milestone M11).
    EmitParticles {
        #[serde(default = "default_burst")]
        count: u32,
    },
}

fn default_burst() -> u32 {
    20
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Transform2D {
    pub x: f32,
    pub y: f32,
    /// Rotation in degrees, clockwise.
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sprite {
    /// Id of an `Asset` of kind `image` in the project manifest.
    pub asset: String,
    /// Spritesheet frame index (row-major, default 0).
    #[serde(default)]
    pub frame: u32,
    /// Base width in world units; defaults to the texture width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    /// Base height in world units; defaults to the texture height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    /// Draw order; higher layers render on top.
    #[serde(default)]
    pub layer: i32,
}

fn default_opacity() -> f32 {
    1.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Camera2D {
    pub zoom: f32,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self { zoom: 1.0 }
    }
}

/// Physics body (milestone M8). Requires a `collider2d` to interact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rigidbody2D {
    pub body: BodyType,
    /// Multiplier over the scene gravity (dynamic bodies only).
    pub gravity_scale: f32,
    /// Initial linear velocity in units/s.
    pub vx: f32,
    pub vy: f32,
    /// Prevents the physics engine from rotating the body.
    pub fixed_rotation: bool,
}

impl Default for Rigidbody2D {
    fn default() -> Self {
        Self {
            body: BodyType::Dynamic,
            gravity_scale: 1.0,
            vx: 0.0,
            vy: 0.0,
            fixed_rotation: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyType {
    /// Simulated by physics (gravity, collisions).
    #[default]
    Dynamic,
    /// Driven by transforms (behaviors/animations); pushes dynamic bodies.
    Kinematic,
    /// Never moves (floors, walls).
    Static,
}

/// Collision shape (milestone M8). Without a `rigidbody2d` it acts as a
/// static collider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Collider2D {
    pub shape: ColliderShape,
    /// Box extents; default: the sprite size (or 32×32 without sprite).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    /// Circle radius; default: half the sprite's larger side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<f32>,
    /// Sensors detect contacts but don't collide physically.
    pub sensor: bool,
    /// Bounciness, `0.0..=1.0`.
    pub restitution: f32,
    pub friction: f32,
}

impl Default for Collider2D {
    fn default() -> Self {
        Self {
            shape: ColliderShape::Box,
            width: None,
            height: None,
            radius: None,
            sensor: false,
            restitution: 0.0,
            friction: 0.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColliderShape {
    #[default]
    Box,
    Circle,
}

/// Particle emitter (milestone M11). Particles spawn at the entity's
/// position and are simulated by the runtime (not part of the document).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Particles {
    /// Image asset used by each particle.
    pub asset: String,
    /// Particles per second while `emitting` (0 = bursts only).
    pub rate: f32,
    /// Particle lifetime in seconds.
    pub lifetime: f32,
    /// Initial speed in units/s.
    pub speed: f32,
    /// Emission direction in degrees (90 = up).
    pub direction: f32,
    /// Emission arc in degrees centered on `direction` (360 = all around).
    pub spread: f32,
    /// Vertical acceleration applied to particles (units/s²).
    pub gravity: f32,
    pub start_scale: f32,
    pub end_scale: f32,
    pub start_opacity: f32,
    pub end_opacity: f32,
    /// Draw layer of the particles.
    pub layer: i32,
    /// Emit continuously from scene start.
    pub emitting: bool,
}

impl Default for Particles {
    fn default() -> Self {
        Self {
            asset: String::new(),
            rate: 20.0,
            lifetime: 0.8,
            speed: 120.0,
            direction: 90.0,
            spread: 360.0,
            gravity: 0.0,
            start_scale: 1.0,
            end_scale: 0.2,
            start_opacity: 1.0,
            end_opacity: 0.0,
            layer: 5,
            emitting: true,
        }
    }
}

/// Flat-color box/circle primitive: no image asset, drawn as a tinted quad
/// reusing the sprite render pipeline (a shared 1×1 white texture).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Shape {
    pub kind: ShapeKind,
    /// Box extents in world units; used when `kind` is `box`.
    pub width: f32,
    pub height: f32,
    /// Circle radius in world units; used when `kind` is `circle`.
    pub radius: f32,
    /// `"#rrggbb"` or `"#rrggbbaa"` (leading `#` optional).
    pub color: String,
    pub opacity: f32,
    /// Draw order; higher layers render on top.
    pub layer: i32,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            kind: ShapeKind::Box,
            width: 40.0,
            height: 40.0,
            radius: 20.0,
            color: "#7f5af0".to_string(),
            opacity: 1.0,
            layer: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeKind {
    #[default]
    Box,
    Circle,
}

// ---------------------------------------------------------------------------
// Animations (timeline data evaluated by `aigs-anim`)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Animation {
    pub name: String,
    /// Timeline frames per second.
    pub fps: u32,
    #[serde(default, rename = "loop")]
    pub looped: bool,
    #[serde(default)]
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    /// Id of the animated `EntityNode`.
    pub entity: String,
    /// Animated property path, e.g. `"transform2d.x"` or `"sprite.opacity"`.
    pub property: String,
    pub keyframes: Vec<Keyframe>,
}

// ---------------------------------------------------------------------------
// Load / save
// ---------------------------------------------------------------------------

impl Project {
    pub fn from_json(json: &str) -> Result<Self, FormatError> {
        let project: Project = serde_json::from_str(json)?;
        project.format.validate(FormatKind::AigsProject)?;
        Ok(project)
    }

    pub fn to_json(&self) -> Result<String, FormatError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn load(path: &Path) -> Result<Self, FormatError> {
        Self::from_json(&read(path)?)
    }
}

impl Scene {
    pub fn from_json(json: &str) -> Result<Self, FormatError> {
        let scene: Scene = serde_json::from_str(json)?;
        scene.format.validate(FormatKind::AigsScene)?;
        Ok(scene)
    }

    pub fn to_json(&self) -> Result<String, FormatError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn load(path: &Path) -> Result<Self, FormatError> {
        Self::from_json(&read(path)?)
    }
}

fn read(path: &Path) -> Result<String, FormatError> {
    std::fs::read_to_string(path).map_err(|source| FormatError::Io {
        path: path.display().to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_project() -> Project {
        Project {
            format: FormatHeader {
                kind: FormatKind::AigsProject,
                version: FORMAT_VERSION,
            },
            name: "Demo".into(),
            description: Some("Test project".into()),
            initial_scene: "scenes/main.scene.aigs".into(),
            scenes: vec!["scenes/main.scene.aigs".into()],
            assets: vec![Asset {
                id: "hero".into(),
                kind: AssetKind::Image,
                path: "assets/hero.png".into(),
                spritesheet: None,
            }],
        }
    }

    fn sample_scene() -> Scene {
        Scene {
            format: FormatHeader {
                kind: FormatKind::AigsScene,
                version: FORMAT_VERSION,
            },
            name: "main".into(),
            gravity: None,
            music: None,
            entities: vec![EntityNode {
                id: "hero".into(),
                name: "Hero".into(),
                components: Components {
                    transform2d: Some(Transform2D::default()),
                    sprite: Some(Sprite {
                        asset: "hero".into(),
                        frame: 0,
                        width: Some(64.0),
                        height: None,
                        opacity: 1.0,
                        layer: 0,
                    }),
                    ..Components::default()
                },
                children: vec![],
            }],
            animations: vec![Animation {
                name: "intro".into(),
                fps: 30,
                looped: true,
                tracks: vec![Track {
                    entity: "hero".into(),
                    property: "transform2d.x".into(),
                    keyframes: vec![
                        Keyframe {
                            frame: 0,
                            value: 0.0,
                            easing: Easing::Linear,
                        },
                        Keyframe {
                            frame: 30,
                            value: 100.0,
                            easing: Easing::EaseInOut,
                        },
                    ],
                }],
            }],
        }
    }

    #[test]
    fn project_round_trips_without_loss() {
        let project = sample_project();
        let json = project.to_json().unwrap();
        assert_eq!(Project::from_json(&json).unwrap(), project);
    }

    #[test]
    fn scene_round_trips_without_loss() {
        let scene = sample_scene();
        let json = scene.to_json().unwrap();
        assert_eq!(Scene::from_json(&json).unwrap(), scene);
    }

    #[test]
    fn unknown_components_survive_round_trip() {
        let json = r#"{
            "format": { "kind": "aigs-scene", "version": 0 },
            "name": "main",
            "entities": [{
                "id": "e1",
                "name": "Modded",
                "components": {
                    "transform2d": { "x": 5.0 },
                    "my_plugin.magnet": { "strength": 3 }
                }
            }]
        }"#;
        let scene = Scene::from_json(json).unwrap();
        let components = &scene.entities[0].components;
        assert_eq!(components.transform2d.as_ref().unwrap().x, 5.0);
        assert_eq!(components.transform2d.as_ref().unwrap().scale_x, 1.0);
        assert!(components.extra.contains_key("my_plugin.magnet"));
        let saved = scene.to_json().unwrap();
        assert!(saved.contains("my_plugin.magnet"));
    }

    #[test]
    fn rejects_future_format_version() {
        let json = r#"{
            "format": { "kind": "aigs-project", "version": 999 },
            "name": "X",
            "initial_scene": "s",
            "scenes": ["s"]
        }"#;
        assert!(matches!(
            Project::from_json(json),
            Err(FormatError::UnsupportedVersion { found: 999, .. })
        ));
    }

    #[test]
    fn physics_components_round_trip() {
        let json = r#"{
            "format": { "kind": "aigs-scene", "version": 0 },
            "name": "physics",
            "gravity": { "y": -500.0 },
            "entities": [
                {
                    "id": "crate", "name": "Crate",
                    "components": {
                        "sprite": { "asset": "crate" },
                        "rigidbody2d": { "vy": -10.0 },
                        "collider2d": { "restitution": 0.6 },
                        "behaviors": [
                            { "on": { "type": "collision", "with": "floor" },
                              "do": { "type": "play_animation", "animation": "bump" } },
                            { "on": { "type": "collision" },
                              "do": { "type": "move", "dx": 0.0, "dy": 1.0 } }
                        ]
                    }
                },
                {
                    "id": "floor", "name": "Floor",
                    "components": {
                        "collider2d": { "shape": "box", "width": 800.0, "height": 40.0 }
                    }
                }
            ]
        }"#;
        let scene = Scene::from_json(json).unwrap();
        assert_eq!(scene.gravity.unwrap(), Gravity { x: 0.0, y: -500.0 });
        let crate_components = &scene.entities[0].components;
        let body = crate_components.rigidbody2d.as_ref().unwrap();
        assert_eq!(body.body, BodyType::Dynamic);
        assert_eq!(body.vy, -10.0);
        let collider = crate_components.collider2d.as_ref().unwrap();
        assert_eq!(collider.restitution, 0.6);
        assert_eq!(collider.width, None, "size defaults to sprite size");
        assert!(matches!(
            crate_components.behaviors[0].on,
            EventSpec::Collision { with: Some(ref id) } if id == "floor"
        ));
        assert!(matches!(
            crate_components.behaviors[1].on,
            EventSpec::Collision { with: None }
        ));
        let saved = scene.to_json().unwrap();
        assert_eq!(Scene::from_json(&saved).unwrap(), scene);
    }

    #[test]
    fn spritesheet_and_animator_round_trip() {
        let manifest = r#"{
            "format": { "kind": "aigs-project", "version": 0 },
            "name": "X",
            "initial_scene": "s",
            "scenes": ["s"],
            "assets": [{
                "id": "walker", "kind": "image", "path": "a.png",
                "spritesheet": { "frame_width": 32, "frame_height": 48 }
            }]
        }"#;
        let project = Project::from_json(manifest).unwrap();
        let sheet = project.assets[0].spritesheet.unwrap();
        assert_eq!((sheet.frame_width, sheet.frame_height), (32, 48));

        let scene_json = r#"{
            "format": { "kind": "aigs-scene", "version": 0 },
            "name": "s",
            "entities": [{
                "id": "hero", "name": "Hero",
                "components": {
                    "sprite": { "asset": "walker", "frame": 2 },
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
            }]
        }"#;
        let scene = Scene::from_json(scene_json).unwrap();
        let components = &scene.entities[0].components;
        assert_eq!(components.sprite.as_ref().unwrap().frame, 2);
        let animator = components.animator.as_ref().unwrap();
        assert_eq!(animator.initial, "idle");
        assert_eq!(animator.states.len(), 2);
        assert!(matches!(
            animator.transitions[1].when,
            EventSpec::KeyReleased { ref key } if key == "ArrowRight"
        ));
        let saved = scene.to_json().unwrap();
        assert_eq!(Scene::from_json(&saved).unwrap(), scene);
    }

    #[test]
    fn script_component_round_trips() {
        let json = r#"{
            "format": { "kind": "aigs-scene", "version": 0 },
            "name": "s",
            "entities": [{
                "id": "drone", "name": "Drone",
                "components": { "script": { "asset": "patrol" } }
            }]
        }"#;
        let scene = Scene::from_json(json).unwrap();
        assert_eq!(
            scene.entities[0].components.script.as_ref().unwrap().asset,
            "patrol"
        );
        let manifest = r#"{
            "format": { "kind": "aigs-project", "version": 0 },
            "name": "X", "initial_scene": "s", "scenes": ["s"],
            "assets": [{ "id": "patrol", "kind": "script", "path": "scripts/patrol.rhai" }]
        }"#;
        let project = Project::from_json(manifest).unwrap();
        assert_eq!(project.assets[0].kind, AssetKind::Script);
        let saved = scene.to_json().unwrap();
        assert_eq!(Scene::from_json(&saved).unwrap(), scene);
    }

    #[test]
    fn particles_round_trip() {
        let json = r#"{
            "format": { "kind": "aigs-scene", "version": 0 },
            "name": "fx",
            "entities": [{
                "id": "spark", "name": "Spark",
                "components": {
                    "particles": { "asset": "dot", "rate": 0.0, "speed": 200.0, "emitting": false },
                    "behaviors": [
                        { "on": { "type": "click" },
                          "do": { "type": "emit_particles", "count": 30 } },
                        { "on": { "type": "scene_start" },
                          "do": { "type": "emit_particles" } }
                    ]
                }
            }]
        }"#;
        let scene = Scene::from_json(json).unwrap();
        let particles = scene.entities[0].components.particles.as_ref().unwrap();
        assert_eq!(particles.asset, "dot");
        assert_eq!(particles.rate, 0.0);
        assert!(!particles.emitting);
        assert_eq!(particles.lifetime, 0.8, "defaults fill in");
        assert_eq!(
            scene.entities[0].components.behaviors[0].action,
            ActionSpec::EmitParticles { count: 30 }
        );
        assert_eq!(
            scene.entities[0].components.behaviors[1].action,
            ActionSpec::EmitParticles { count: 20 },
            "default burst count"
        );
        let saved = scene.to_json().unwrap();
        assert_eq!(Scene::from_json(&saved).unwrap(), scene);
    }

    #[test]
    fn shape_round_trip() {
        let json = r##"{
            "format": { "kind": "aigs-scene", "version": 0 },
            "name": "hud",
            "entities": [{
                "id": "hunger-bar", "name": "Hunger Bar",
                "components": {
                    "shape": { "kind": "box", "width": 80.0, "height": 10.0, "color": "#e0af68" }
                }
            }, {
                "id": "dot", "name": "Dot",
                "components": { "shape": { "kind": "circle" } }
            }]
        }"##;
        let scene = Scene::from_json(json).unwrap();
        let bar = scene.entities[0].components.shape.as_ref().unwrap();
        assert_eq!(bar.kind, ShapeKind::Box);
        assert_eq!(bar.width, 80.0);
        assert_eq!(bar.color, "#e0af68");
        assert_eq!(bar.opacity, 1.0, "defaults fill in");
        assert_eq!(bar.layer, 0, "defaults fill in");
        let dot = scene.entities[1].components.shape.as_ref().unwrap();
        assert_eq!(dot.kind, ShapeKind::Circle);
        assert_eq!(dot.radius, 20.0, "defaults fill in");
        let saved = scene.to_json().unwrap();
        assert_eq!(Scene::from_json(&saved).unwrap(), scene);
    }

    #[test]
    fn audio_round_trips() {
        let json = r#"{
            "format": { "kind": "aigs-scene", "version": 0 },
            "name": "level",
            "music": { "asset": "theme", "volume": 0.8 },
            "entities": [{
                "id": "coin", "name": "Coin",
                "components": { "behaviors": [
                    { "on": { "type": "click" },
                      "do": { "type": "play_sound", "asset": "pop" } }
                ] }
            }]
        }"#;
        let scene = Scene::from_json(json).unwrap();
        let music = scene.music.as_ref().unwrap();
        assert_eq!(music.asset, "theme");
        assert_eq!(music.volume, 0.8);
        assert!(music.looped, "music loops by default");
        assert_eq!(
            scene.entities[0].components.behaviors[0].action,
            ActionSpec::PlaySound {
                asset: "pop".into(),
                volume: 1.0
            }
        );
        let saved = scene.to_json().unwrap();
        assert_eq!(Scene::from_json(&saved).unwrap(), scene);
    }

    #[test]
    fn behaviors_round_trip() {
        let json = r#"{
            "format": { "kind": "aigs-scene", "version": 0 },
            "name": "menu",
            "entities": [{
                "id": "start-button", "name": "Start",
                "components": {
                    "sprite": { "asset": "button" },
                    "behaviors": [
                        { "on": { "type": "click" },
                          "do": { "type": "goto_scene", "scene": "scenes/level1.scene.aigs" } },
                        { "on": { "type": "key_down", "key": "ArrowRight" },
                          "do": { "type": "move", "dx": 200.0, "dy": 0.0 } },
                        { "on": { "type": "animation_end", "animation": "intro" },
                          "do": { "type": "play_animation", "animation": "idle" } }
                    ]
                }
            }]
        }"#;
        let scene = Scene::from_json(json).unwrap();
        let behaviors = &scene.entities[0].components.behaviors;
        assert_eq!(behaviors.len(), 3);
        assert_eq!(
            behaviors[0].action,
            ActionSpec::GotoScene {
                scene: "scenes/level1.scene.aigs".into()
            }
        );
        assert!(matches!(
            behaviors[1].on,
            EventSpec::KeyDown { ref key } if key == "ArrowRight"
        ));
        let saved = scene.to_json().unwrap();
        assert_eq!(Scene::from_json(&saved).unwrap(), scene);
    }

    #[test]
    fn rejects_kind_mismatch() {
        let json = r#"{
            "format": { "kind": "aigs-scene", "version": 0 },
            "name": "X",
            "initial_scene": "s",
            "scenes": ["s"]
        }"#;
        assert!(matches!(
            Project::from_json(json),
            Err(FormatError::KindMismatch { .. })
        ));
    }
}
