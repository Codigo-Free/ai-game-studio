//! 2D physics (milestone M8): rapier2d driven at the fixed simulation step.
//!
//! Body semantics (see `sdk/aigs-format/SPEC.md`):
//! - **dynamic**: simulated (gravity, collisions); its `Transform2D` is
//!   written back after every step.
//! - **kinematic**: driven by the ECS transform (behaviors/animations);
//!   pushes dynamic bodies but is not affected by them.
//! - **static** (or a collider without a body): never moves.

use std::collections::HashMap;
use std::sync::Mutex;

use aigs_ecs::{Entity, World};
use aigs_project::Gravity;
use rapier2d::prelude::*;

use crate::components::{BodyType, Collider2DShape, ColliderShape, RigidBody2D, Transform2D};

pub struct PhysicsWorld {
    gravity: Vector,
    integration: IntegrationParameters,
    pipeline: PhysicsPipeline,
    islands: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd: CCDSolver,
    entity_of_collider: HashMap<ColliderHandle, Entity>,
    kinematic: Vec<(Entity, RigidBodyHandle)>,
    dynamic: Vec<(Entity, RigidBodyHandle)>,
}

impl PhysicsWorld {
    /// Builds the physics state from every entity with a collider. Returns
    /// `None` when the scene has no physics at all.
    pub fn build(world: &World, gravity: Gravity) -> Option<Self> {
        let mut physics = Self {
            gravity: Vector::new(gravity.x, gravity.y),
            integration: IntegrationParameters::default(),
            pipeline: PhysicsPipeline::new(),
            islands: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd: CCDSolver::new(),
            entity_of_collider: HashMap::new(),
            kinematic: Vec::new(),
            dynamic: Vec::new(),
        };

        let mut any = false;
        world.for_each::<Collider2DShape>(|entity, collider| {
            any = true;
            let transform = world
                .get::<Transform2D>(entity)
                .map(|t| *t)
                .unwrap_or_default();
            let body_spec = world.get::<RigidBody2D>(entity).map(|b| *b);
            let position = Pose::new(
                Vector::new(transform.x, transform.y),
                -transform.rotation.to_radians(),
            );

            let body_handle = match body_spec {
                Some(spec) => {
                    let mut builder = match spec.body {
                        BodyType::Dynamic => RigidBodyBuilder::dynamic(),
                        BodyType::Kinematic => RigidBodyBuilder::kinematic_position_based(),
                        BodyType::Static => RigidBodyBuilder::fixed(),
                    }
                    .pose(position)
                    .linvel(Vector::new(spec.vx, spec.vy))
                    .gravity_scale(spec.gravity_scale);
                    if spec.fixed_rotation {
                        builder = builder.lock_rotations();
                    }
                    let handle = physics.bodies.insert(builder);
                    match spec.body {
                        BodyType::Dynamic => physics.dynamic.push((entity, handle)),
                        BodyType::Kinematic => physics.kinematic.push((entity, handle)),
                        BodyType::Static => {}
                    }
                    Some(handle)
                }
                None => None,
            };

            let mut collider_builder = match collider.shape {
                ColliderShape::Box => {
                    ColliderBuilder::cuboid(collider.half_width, collider.half_height)
                }
                ColliderShape::Circle => ColliderBuilder::ball(collider.radius),
            }
            .sensor(collider.sensor)
            .restitution(collider.restitution)
            .friction(collider.friction)
            .active_events(ActiveEvents::COLLISION_EVENTS);
            if body_handle.is_none() {
                collider_builder = collider_builder.position(position);
            }
            let collider_handle = match body_handle {
                Some(body) => physics.colliders.insert_with_parent(
                    collider_builder,
                    body,
                    &mut physics.bodies,
                ),
                None => physics.colliders.insert(collider_builder),
            };
            physics.entity_of_collider.insert(collider_handle, entity);
        });

        any.then_some(physics)
    }

    /// Advances the simulation one fixed step and returns the entity pairs
    /// that started touching during this step.
    pub fn step(&mut self, world: &World, dt: f32) -> Vec<(Entity, Entity)> {
        // Kinematic bodies follow the ECS transform.
        for (entity, handle) in &self.kinematic {
            if let (Some(transform), Some(body)) = (
                world.get::<Transform2D>(*entity),
                self.bodies.get_mut(*handle),
            ) {
                body.set_next_kinematic_position(Pose::new(
                    Vector::new(transform.x, transform.y),
                    -transform.rotation.to_radians(),
                ));
            }
        }

        self.integration.dt = dt;
        let events = CollisionCollector::default();
        self.pipeline.step(
            self.gravity,
            &self.integration,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd,
            &(),
            &events,
        );

        // Dynamic bodies write their simulated pose back into the ECS.
        for (entity, handle) in &self.dynamic {
            if let (Some(mut transform), Some(body)) = (
                world.get_mut::<Transform2D>(*entity),
                self.bodies.get(*handle),
            ) {
                let position = body.position();
                transform.x = position.translation.x;
                transform.y = position.translation.y;
                transform.rotation = -position.rotation.angle().to_degrees();
            }
        }

        events
            .started
            .into_inner()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(a, b)| {
                let entity_a = self.entity_of_collider.get(&a)?;
                let entity_b = self.entity_of_collider.get(&b)?;
                Some((*entity_a, *entity_b))
            })
            .collect()
    }
}

/// Collects contact-started events during a step.
#[derive(Default)]
struct CollisionCollector {
    started: Mutex<Vec<(ColliderHandle, ColliderHandle)>>,
}

impl EventHandler for CollisionCollector {
    fn handle_collision_event(
        &self,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        event: CollisionEvent,
        _contact_pair: Option<&ContactPair>,
    ) {
        if let CollisionEvent::Started(a, b, _) = event {
            if let Ok(mut started) = self.started.lock() {
                started.push((a, b));
            }
        }
    }

    fn handle_contact_force_event(
        &self,
        _dt: Real,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        _contact_pair: &ContactPair,
        _total_force_magnitude: Real,
    ) {
    }
}
