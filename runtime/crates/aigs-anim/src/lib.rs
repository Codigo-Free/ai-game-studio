//! Timeline animation engine of the AI Game Studio runtime.
//!
//! Provides the easing curves, the [`Keyframe`] type shared with the `.aigs`
//! format and [`sample`], the track evaluator used by both the runtime
//! playback (`aigs-runtime`) and mirrored by the editor timeline.

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

/// A keyframe on a timeline track, as authored in `.aigs` files.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    pub frame: u32,
    pub value: f32,
    /// Easing towards the next keyframe.
    #[serde(default)]
    pub easing: Easing,
}

/// Samples a track at `frame` (fractional frames interpolate).
///
/// `keyframes` must be sorted by `frame`. Before the first keyframe the
/// first value holds; after the last one, the last value holds. Returns
/// `None` for an empty track.
pub fn sample(keyframes: &[Keyframe], frame: f32) -> Option<f32> {
    let first = keyframes.first()?;
    if frame <= first.frame as f32 {
        return Some(first.value);
    }
    for pair in keyframes.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        if frame < to.frame as f32 {
            let span = (to.frame - from.frame).max(1) as f32;
            let t = (frame - from.frame as f32) / span;
            return Some(tween(from.value, to.value, t, from.easing));
        }
    }
    keyframes.last().map(|keyframe| keyframe.value)
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

    fn keyframe(frame: u32, value: f32) -> Keyframe {
        Keyframe {
            frame,
            value,
            easing: Easing::Linear,
        }
    }

    #[test]
    fn sample_holds_endpoints_and_interpolates() {
        let track = [keyframe(10, 0.0), keyframe(20, 100.0)];
        assert_eq!(sample(&track, 0.0), Some(0.0), "holds before first");
        assert_eq!(sample(&track, 10.0), Some(0.0));
        assert_eq!(sample(&track, 15.0), Some(50.0), "interpolates");
        assert_eq!(sample(&track, 20.0), Some(100.0));
        assert_eq!(sample(&track, 99.0), Some(100.0), "holds after last");
    }

    #[test]
    fn sample_edge_cases() {
        assert_eq!(sample(&[], 5.0), None, "empty track");
        assert_eq!(sample(&[keyframe(3, 7.0)], 0.0), Some(7.0));
        assert_eq!(sample(&[keyframe(3, 7.0)], 30.0), Some(7.0));
        // Duplicated frames must not divide by zero.
        let dup = [keyframe(5, 1.0), keyframe(5, 2.0)];
        assert!(sample(&dup, 5.0).is_some());
    }

    #[test]
    fn sample_respects_easing_of_leading_keyframe() {
        let track = [
            Keyframe {
                frame: 0,
                value: 0.0,
                easing: Easing::EaseIn,
            },
            keyframe(10, 10.0),
        ];
        let quarter = sample(&track, 2.5).unwrap();
        assert!(quarter < 2.5, "ease-in starts slow, got {quarter}");
    }
}
