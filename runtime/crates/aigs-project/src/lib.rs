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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Image,
    Audio,
    Font,
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
    /// Code-free event → action rules (see `Behavior`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behaviors: Vec<Behavior>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
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
