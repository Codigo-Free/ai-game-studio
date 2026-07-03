//! Loader, writer and validator of the `.aigs` project format.
//!
//! The `.aigs` format is the AI-Ready contract shared by the editor, the
//! runtime, the exporters and the AI agents. The normative specification
//! lives in `sdk/aigs-format/SPEC.md`; this crate is its reference
//! implementation. Everything the editor can do is expressed as this data.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub use aigs_anim::Easing;

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
    #[serde(default)]
    pub entities: Vec<EntityNode>,
    #[serde(default)]
    pub animations: Vec<Animation>,
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
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    pub frame: u32,
    pub value: f32,
    /// Easing towards the next keyframe.
    #[serde(default)]
    pub easing: Easing,
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
