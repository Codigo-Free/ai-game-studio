//! 2D renderer of the AI Game Studio runtime.
//!
//! [`Renderer`] owns the WGPU device and draws batched, instanced sprites
//! sorted by layer. Milestone M1 couples it to a winit window; rendering to
//! an offscreen texture for the editor viewport arrives in M3.

mod renderer;

pub use renderer::{CameraView, RenderError, Renderer, SpriteInstance, TextureId};
pub use wgpu::SurfaceError;

/// RGBA color with components in `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Color = Color::rgba(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Color = Color::rgba(0.0, 0.0, 0.0, 1.0);
    pub const TRANSPARENT: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Parses `#rrggbb` or `#rrggbbaa` (leading `#` optional).
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        let (r, g, b, a) = match hex.len() {
            6 => (byte(0)?, byte(2)?, byte(4)?, 255),
            8 => (byte(0)?, byte(2)?, byte(4)?, byte(6)?),
            _ => return None,
        };
        Some(Self::rgba(
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            f32::from(a) / 255.0,
        ))
    }
}

/// Logical size of the render target in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

impl Viewport {
    pub fn aspect_ratio(self) -> f32 {
        self.width as f32 / self.height.max(1) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_colors() {
        assert_eq!(Color::from_hex("#ffffff"), Some(Color::WHITE));
        assert_eq!(Color::from_hex("000000ff"), Some(Color::BLACK));
        let semi = Color::from_hex("#ff000080").unwrap();
        assert_eq!((semi.r, semi.g, semi.b), (1.0, 0.0, 0.0));
        assert!((semi.a - 0.5).abs() < 0.01);
    }

    #[test]
    fn rejects_malformed_hex() {
        assert_eq!(Color::from_hex("#fff"), None);
        assert_eq!(Color::from_hex("zzzzzz"), None);
        assert_eq!(Color::from_hex(""), None);
    }

    #[test]
    fn viewport_aspect_ratio_never_divides_by_zero() {
        let vp = Viewport {
            width: 1920,
            height: 0,
        };
        assert!(vp.aspect_ratio().is_finite());
    }
}
