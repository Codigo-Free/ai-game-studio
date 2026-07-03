//! Timeline animation engine of the AI Game Studio runtime.
//!
//! Milestone M0 seeds the easing curves and interpolation primitives shared
//! with the `.aigs` format. Tracks, keyframes and timeline evaluation land in
//! milestone M4 (see `docs/plan.md`).

use serde::{Deserialize, Serialize};

/// Easing curve applied between two keyframes.
///
/// Serialized in snake_case inside `.aigs` files (e.g. `"ease_in_out"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Easing {
    /// Maps a normalized time `t` in `0.0..=1.0` to eased progress.
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t,
            Easing::EaseOut => t * (2.0 - t),
            Easing::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
        }
    }
}

/// Linear interpolation between `a` and `b` at progress `t`.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Interpolates between two keyframe values applying an easing curve.
pub fn tween(from: f32, to: f32, t: f32, easing: Easing) -> f32 {
    lerp(from, to, easing.apply(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easings_preserve_endpoints() {
        for easing in [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
        ] {
            assert_eq!(easing.apply(0.0), 0.0, "{easing:?} at t=0");
            assert_eq!(easing.apply(1.0), 1.0, "{easing:?} at t=1");
        }
    }

    #[test]
    fn apply_clamps_out_of_range_time() {
        assert_eq!(Easing::Linear.apply(-1.0), 0.0);
        assert_eq!(Easing::Linear.apply(2.0), 1.0);
    }

    #[test]
    fn tween_interpolates_between_keyframes() {
        assert_eq!(tween(0.0, 10.0, 0.5, Easing::Linear), 5.0);
        assert_eq!(tween(0.0, 10.0, 0.0, Easing::EaseInOut), 0.0);
        assert_eq!(tween(0.0, 10.0, 1.0, Easing::EaseInOut), 10.0);
        assert!(tween(0.0, 10.0, 0.25, Easing::EaseIn) < 2.5);
        assert!(tween(0.0, 10.0, 0.25, Easing::EaseOut) > 2.5);
    }
}
