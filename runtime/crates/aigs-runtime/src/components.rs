//! Base runtime components (milestone M1).
//!
//! These mirror the authoring components of the `.aigs` format (see
//! `sdk/aigs-format/SPEC.md`); the scene-to-world instantiation that connects
//! both arrives in milestone M2.

use aigs_render::TextureId;

/// Position, rotation and scale in world units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    pub x: f32,
    pub y: f32,
    /// Rotation in degrees, clockwise (same convention as the `.aigs` format).
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

impl Transform2D {
    pub fn at(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            ..Self::default()
        }
    }
}

/// Snapshot of the previous simulation tick, written automatically by the
/// runner and used to interpolate rendering between fixed updates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrevTransform2D(pub Transform2D);

/// A textured quad.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sprite {
    pub texture: TextureId,
    /// Base size in world units, before the transform scale.
    pub width: f32,
    pub height: f32,
    pub opacity: f32,
    /// Higher layers draw on top.
    pub layer: i32,
}

impl Sprite {
    pub fn new(texture: TextureId, width: f32, height: f32) -> Self {
        Self {
            texture,
            width,
            height,
            opacity: 1.0,
            layer: 0,
        }
    }
}

/// Marks the entity whose view renders the scene. The first live entity with
/// both a `Camera2D` and a `Transform2D` wins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera2D {
    pub zoom: f32,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self { zoom: 1.0 }
    }
}

/// Entities with `Visibility(false)` are skipped by the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visibility(pub bool);

impl Default for Visibility {
    fn default() -> Self {
        Self(true)
    }
}

/// Human-readable name shown by the editor and used in logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name(pub String);
