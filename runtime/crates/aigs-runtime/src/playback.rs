//! Animation playback (milestone M4): binds the animations authored in an
//! `.aigs` scene to live entities and drives them every simulation tick.
//!
//! Semantics (see `sdk/aigs-format/SPEC.md`): every scene animation starts
//! when the scene loads; looping animations wrap, non-looping ones hold
//! their final value.

use aigs_anim::{sample, Keyframe};
use aigs_ecs::{Entity, World};
use aigs_project::Scene;

use crate::components::{Sprite, Transform2D};
use crate::scene::SceneInstance;

/// Property a track can animate in format v0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnimTarget {
    X,
    Y,
    Rotation,
    ScaleX,
    ScaleY,
    Opacity,
    SpriteFrame,
}

impl AnimTarget {
    fn parse(property: &str) -> Option<Self> {
        match property {
            "transform2d.x" => Some(Self::X),
            "transform2d.y" => Some(Self::Y),
            "transform2d.rotation" => Some(Self::Rotation),
            "transform2d.scale_x" => Some(Self::ScaleX),
            "transform2d.scale_y" => Some(Self::ScaleY),
            "sprite.opacity" => Some(Self::Opacity),
            "sprite.frame" => Some(Self::SpriteFrame),
            _ => None,
        }
    }
}

struct BoundTrack {
    entity: Entity,
    target: AnimTarget,
    keyframes: Vec<Keyframe>,
}

struct PlayingAnimation {
    name: String,
    fps: f32,
    looped: bool,
    duration_frames: f32,
    time: f32,
    finished: bool,
    tracks: Vec<BoundTrack>,
}

/// Live playback state for one scene's animations.
#[derive(Default)]
pub struct AnimationPlayback {
    animations: Vec<PlayingAnimation>,
    warnings: Vec<String>,
}

impl AnimationPlayback {
    /// Binds every scene animation to the instantiated entities. Tracks
    /// pointing to unknown entities or properties are skipped with a warning.
    pub fn bind(scene: &Scene, instance: &SceneInstance) -> Self {
        let mut playback = Self::default();
        for animation in &scene.animations {
            let mut tracks = Vec::new();
            let mut duration = 0u32;
            for track in &animation.tracks {
                let Some(entity) = instance.entity(&track.entity) else {
                    playback.warnings.push(format!(
                        "animation \"{}\": unknown entity \"{}\"",
                        animation.name, track.entity
                    ));
                    continue;
                };
                let Some(target) = AnimTarget::parse(&track.property) else {
                    playback.warnings.push(format!(
                        "animation \"{}\": property \"{}\" is not animatable in format v0",
                        animation.name, track.property
                    ));
                    continue;
                };
                let mut keyframes = track.keyframes.clone();
                keyframes.sort_by_key(|keyframe| keyframe.frame);
                duration = duration.max(keyframes.last().map_or(0, |k| k.frame));
                tracks.push(BoundTrack {
                    entity,
                    target,
                    keyframes,
                });
            }
            if !tracks.is_empty() {
                playback.animations.push(PlayingAnimation {
                    name: animation.name.clone(),
                    fps: animation.fps.max(1) as f32,
                    looped: animation.looped,
                    duration_frames: duration as f32,
                    time: 0.0,
                    finished: false,
                    tracks,
                });
            }
        }
        playback
    }

    /// Advances every playing animation by `dt` seconds and writes the
    /// sampled values into the world. Returns the names of the animations
    /// that finished during this tick (for `animation_end` behaviors).
    pub fn advance(&mut self, world: &World, dt: f32) -> Vec<String> {
        let mut finished_now = Vec::new();
        for animation in &mut self.animations {
            if animation.finished {
                continue;
            }
            animation.time += dt;
            let mut frame = animation.time * animation.fps;
            if frame >= animation.duration_frames {
                if animation.looped && animation.duration_frames > 0.0 {
                    frame %= animation.duration_frames;
                    animation.time = frame / animation.fps;
                } else {
                    frame = animation.duration_frames;
                    animation.finished = true;
                    finished_now.push(animation.name.clone());
                }
            }
            for track in &animation.tracks {
                if let Some(value) = sample(&track.keyframes, frame) {
                    apply(world, track.entity, track.target, value);
                }
            }
        }
        finished_now
    }

    /// Halts an animation by name (its values freeze where they are).
    /// Used by animation state machines when leaving a state.
    pub fn stop(&mut self, name: &str) -> bool {
        let mut found = false;
        for animation in &mut self.animations {
            if animation.name == name {
                animation.finished = true;
                found = true;
            }
        }
        found
    }

    /// Restarts an animation by name (used by the `play_animation` action).
    /// Returns `false` if no animation has that name.
    pub fn restart(&mut self, name: &str) -> bool {
        let mut found = false;
        for animation in &mut self.animations {
            if animation.name == name {
                animation.time = 0.0;
                animation.finished = false;
                found = true;
            }
        }
        found
    }

    /// Number of animations that still advance.
    pub fn active(&self) -> usize {
        self.animations.iter().filter(|a| !a.finished).count()
    }

    pub fn len(&self) -> usize {
        self.animations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.animations.is_empty()
    }

    /// Problems found while binding (unknown entities/properties).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

fn apply(world: &World, entity: Entity, target: AnimTarget, value: f32) {
    match target {
        AnimTarget::Opacity => {
            if let Some(mut sprite) = world.get_mut::<Sprite>(entity) {
                sprite.opacity = value;
            }
        }
        AnimTarget::SpriteFrame => {
            if let Some(mut sprite) = world.get_mut::<Sprite>(entity) {
                sprite.frame = value;
            }
        }
        _ => {
            if let Some(mut transform) = world.get_mut::<Transform2D>(entity) {
                match target {
                    AnimTarget::X => transform.x = value,
                    AnimTarget::Y => transform.y = value,
                    AnimTarget::Rotation => transform.rotation = value,
                    AnimTarget::ScaleX => transform.scale_x = value,
                    AnimTarget::ScaleY => transform.scale_y = value,
                    AnimTarget::Opacity | AnimTarget::SpriteFrame => unreachable!(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{instantiate_scene, ResolveTexture};
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

    fn scene() -> Scene {
        Scene::from_json(
            r#"{
                "format": { "kind": "aigs-scene", "version": 0 },
                "name": "main",
                "entities": [{
                    "id": "hero", "name": "Hero",
                    "components": {
                        "transform2d": {},
                        "sprite": { "asset": "hero" }
                    }
                }],
                "animations": [
                    {
                        "name": "slide", "fps": 10, "loop": false,
                        "tracks": [{
                            "entity": "hero", "property": "transform2d.x",
                            "keyframes": [
                                { "frame": 0, "value": 0.0 },
                                { "frame": 10, "value": 100.0 }
                            ]
                        }]
                    },
                    {
                        "name": "pulse", "fps": 10, "loop": true,
                        "tracks": [{
                            "entity": "hero", "property": "sprite.opacity",
                            "keyframes": [
                                { "frame": 0, "value": 0.0 },
                                { "frame": 10, "value": 1.0 }
                            ]
                        }]
                    },
                    {
                        "name": "broken", "fps": 10, "loop": false,
                        "tracks": [
                            { "entity": "ghost", "property": "transform2d.x", "keyframes": [] },
                            { "entity": "hero", "property": "physics.mass", "keyframes": [] }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap()
    }

    fn setup() -> (World, SceneInstance, AnimationPlayback) {
        let mut world = World::new();
        let instance = instantiate_scene(&mut world, &scene(), &AnyTexture).unwrap();
        let playback = AnimationPlayback::bind(&scene(), &instance);
        (world, instance, playback)
    }

    #[test]
    fn bind_reports_broken_tracks_and_keeps_valid_ones() {
        let (_, _, playback) = setup();
        assert_eq!(playback.len(), 2, "broken animation has no valid tracks");
        assert_eq!(playback.warnings().len(), 2);
    }

    #[test]
    fn advance_moves_animated_values() {
        let (world, instance, mut playback) = setup();
        let hero = instance.entity("hero").unwrap();
        playback.advance(&world, 0.5); // frame 5 of 10 at 10 fps
        let x = world.get::<Transform2D>(hero).unwrap().x;
        assert!((x - 50.0).abs() < 0.01, "expected midpoint, got {x}");
    }

    #[test]
    fn non_looping_animation_holds_final_value() {
        let (world, instance, mut playback) = setup();
        let hero = instance.entity("hero").unwrap();
        playback.advance(&world, 5.0); // way past the end
        assert_eq!(world.get::<Transform2D>(hero).unwrap().x, 100.0);
        assert_eq!(playback.active(), 1, "only the looping one stays active");
        // Further advances keep the final value.
        playback.advance(&world, 1.0);
        assert_eq!(world.get::<Transform2D>(hero).unwrap().x, 100.0);
    }

    #[test]
    fn looping_animation_wraps() {
        let (world, instance, mut playback) = setup();
        let hero = instance.entity("hero").unwrap();
        playback.advance(&world, 1.25); // 12.5 frames -> wraps to 2.5 of 10
        let opacity = world.get::<Sprite>(hero).unwrap().opacity;
        assert!(
            (opacity - 0.25).abs() < 0.01,
            "expected wrapped value, got {opacity}"
        );
    }
}
