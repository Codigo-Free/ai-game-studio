//! Particle system (milestone M11).
//!
//! Particles are plain ECS entities (`Transform2D` + `Sprite` + `Particle`),
//! so they reuse the instanced renderer, layers and interpolation for free.
//! Emitters live on scene entities as [`ParticleEmitter`] components.

use aigs_ecs::{Entity, World};

use crate::components::{PrevTransform2D, Sprite, Transform2D};
use crate::TextureId;

/// Emitter state attached to a scene entity (built from the `particles`
/// format component at instantiation).
#[derive(Debug, Clone)]
pub struct ParticleEmitter {
    pub texture: TextureId,
    pub base_width: f32,
    pub base_height: f32,
    pub rate: f32,
    pub lifetime: f32,
    pub speed: f32,
    pub direction: f32,
    pub spread: f32,
    pub gravity: f32,
    pub start_scale: f32,
    pub end_scale: f32,
    pub start_opacity: f32,
    pub end_opacity: f32,
    pub layer: i32,
    pub emitting: bool,
    accumulator: f32,
    rng: u64,
}

impl ParticleEmitter {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        texture: TextureId,
        base_width: f32,
        base_height: f32,
        spec: &aigs_project::Particles,
        seed: u64,
    ) -> Self {
        Self {
            texture,
            base_width,
            base_height,
            rate: spec.rate.max(0.0),
            lifetime: spec.lifetime.max(0.05),
            speed: spec.speed,
            direction: spec.direction,
            spread: spec.spread.clamp(0.0, 360.0),
            gravity: spec.gravity,
            start_scale: spec.start_scale,
            end_scale: spec.end_scale,
            start_opacity: spec.start_opacity,
            end_opacity: spec.end_opacity,
            layer: spec.layer,
            emitting: spec.emitting,
            accumulator: 0.0,
            rng: seed | 1,
        }
    }

    fn next_f32(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// A live particle (short-lived entity managed by [`tick`]).
#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub age: f32,
    pub lifetime: f32,
    pub vx: f32,
    pub vy: f32,
    pub gravity: f32,
    pub start_scale: f32,
    pub end_scale: f32,
    pub start_opacity: f32,
    pub end_opacity: f32,
}

/// Advances every particle and emits from continuous emitters.
pub fn tick(world: &mut World, dt: f32) {
    // 1. Age, move and fade the live particles; collect the dead.
    let mut dead = Vec::new();
    world.for_each3::<Particle, Transform2D, Sprite>(|entity, particle, transform, sprite| {
        particle.age += dt;
        if particle.age >= particle.lifetime {
            dead.push(entity);
            return;
        }
        particle.vy += particle.gravity * dt;
        transform.x += particle.vx * dt;
        transform.y += particle.vy * dt;
        let t = (particle.age / particle.lifetime).clamp(0.0, 1.0);
        let scale = particle.start_scale + (particle.end_scale - particle.start_scale) * t;
        transform.scale_x = scale;
        transform.scale_y = scale;
        sprite.opacity =
            particle.start_opacity + (particle.end_opacity - particle.start_opacity) * t;
    });
    for entity in dead {
        world.despawn(entity);
    }

    // 2. Continuous emission from active emitters.
    let mut to_spawn: Vec<(Entity, u32)> = Vec::new();
    world.for_each::<ParticleEmitter>(|entity, emitter| {
        if !emitter.emitting || emitter.rate <= 0.0 {
            return;
        }
        emitter.accumulator += emitter.rate * dt;
        let count = emitter.accumulator.floor() as u32;
        emitter.accumulator -= count as f32;
        if count > 0 {
            to_spawn.push((entity, count));
        }
    });
    for (entity, count) in to_spawn {
        burst(world, entity, count);
    }
}

/// Spawns `count` particles from the emitter of `entity` (the
/// `emit_particles` action). Returns `false` if the entity has no emitter.
pub fn burst(world: &mut World, entity: Entity, count: u32) -> bool {
    let Some(origin) = world.get::<Transform2D>(entity).map(|t| *t) else {
        return false;
    };
    // Copy the spawn parameters out to release the emitter borrow.
    let Some(mut params) = world.get_mut::<ParticleEmitter>(entity).map(|mut emitter| {
        let snapshot = emitter.clone();
        // Burn RNG values on the real emitter so bursts differ.
        for _ in 0..(count * 2) {
            emitter.next_f32();
        }
        snapshot
    }) else {
        return false;
    };

    for _ in 0..count {
        let arc = params.spread.to_radians();
        let angle = params.direction.to_radians() + (params.next_f32() - 0.5) * arc;
        let speed = params.speed * (0.6 + 0.4 * params.next_f32());
        let particle = world.spawn();
        world.insert(
            particle,
            Transform2D {
                x: origin.x,
                y: origin.y,
                rotation: 0.0,
                scale_x: params.start_scale,
                scale_y: params.start_scale,
            },
        );
        world.insert(
            particle,
            PrevTransform2D(Transform2D {
                x: origin.x,
                y: origin.y,
                rotation: 0.0,
                scale_x: params.start_scale,
                scale_y: params.start_scale,
            }),
        );
        let mut sprite = Sprite::new(params.texture, params.base_width, params.base_height);
        sprite.opacity = params.start_opacity;
        sprite.layer = params.layer;
        world.insert(particle, sprite);
        world.insert(
            particle,
            Particle {
                age: 0.0,
                lifetime: params.lifetime,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                gravity: params.gravity,
                start_scale: params.start_scale,
                end_scale: params.end_scale,
                start_opacity: params.start_opacity,
                end_opacity: params.end_opacity,
            },
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_with_emitter(rate: f32, emitting: bool) -> (World, Entity) {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Transform2D::at(10.0, 20.0));
        world.insert(
            entity,
            ParticleEmitter::new(
                TextureId::default(),
                8.0,
                8.0,
                &aigs_project::Particles {
                    rate,
                    lifetime: 0.5,
                    emitting,
                    ..Default::default()
                },
                42,
            ),
        );
        (world, entity)
    }

    fn particle_count(world: &World) -> usize {
        let mut count = 0;
        world.for_each::<Particle>(|_, _| count += 1);
        count
    }

    #[test]
    fn continuous_emitter_spawns_at_rate() {
        let (mut world, _) = world_with_emitter(60.0, true);
        for _ in 0..6 {
            tick(&mut world, 1.0 / 60.0);
        }
        // 60/s for 0.1 s ≈ 6 particles (short lifetime keeps them alive).
        assert_eq!(particle_count(&world), 6);
    }

    #[test]
    fn burst_spawns_and_particles_die() {
        let (mut world, emitter) = world_with_emitter(0.0, false);
        assert!(burst(&mut world, emitter, 15));
        assert_eq!(particle_count(&world), 15);
        // No continuous emission while idle.
        tick(&mut world, 1.0 / 60.0);
        assert_eq!(particle_count(&world), 15);
        // After the lifetime, every particle is gone (entity count back to 1).
        for _ in 0..40 {
            tick(&mut world, 1.0 / 60.0);
        }
        assert_eq!(particle_count(&world), 0);
        assert_eq!(world.len(), 1, "only the emitter remains");
    }

    #[test]
    fn particles_fade_and_shrink() {
        let (mut world, emitter) = world_with_emitter(0.0, false);
        burst(&mut world, emitter, 1);
        for _ in 0..15 {
            tick(&mut world, 1.0 / 60.0);
        }
        let mut checked = false;
        world.for_each3::<Particle, Transform2D, Sprite>(|_, _, transform, sprite| {
            assert!(
                transform.scale_x < 1.0,
                "shrinking, got {}",
                transform.scale_x
            );
            assert!(sprite.opacity < 1.0, "fading, got {}", sprite.opacity);
            checked = true;
        });
        assert!(checked, "particle should still be alive at half life");
    }

    #[test]
    fn burst_without_emitter_is_false() {
        let mut world = World::new();
        let plain = world.spawn();
        world.insert(plain, Transform2D::default());
        assert!(!burst(&mut world, plain, 5));
    }
}
