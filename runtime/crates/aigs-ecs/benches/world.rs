//! ECS benchmarks: the numbers that guard the 60 FPS budget.
//! Baselines are recorded in `docs/testing.md`.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use aigs_ecs::World;

struct Position {
    x: f32,
    y: f32,
}
struct Velocity {
    dx: f32,
    dy: f32,
}

fn populated_world(entities: usize) -> World {
    let mut world = World::new();
    for i in 0..entities {
        let entity = world.spawn();
        world.insert(
            entity,
            Position {
                x: i as f32,
                y: 0.0,
            },
        );
        // Half the entities also move.
        if i % 2 == 0 {
            world.insert(entity, Velocity { dx: 1.0, dy: 2.0 });
        }
    }
    world
}

fn bench_world(c: &mut Criterion) {
    c.bench_function("spawn_insert_10k", |b| {
        b.iter(|| black_box(populated_world(10_000)))
    });

    let world = populated_world(10_000);
    c.bench_function("query2_10k", |b| {
        b.iter(|| {
            world.for_each2::<Position, Velocity>(|_, pos, vel| {
                pos.x += vel.dx * (1.0 / 60.0);
                pos.y += vel.dy * (1.0 / 60.0);
            });
        })
    });
}

criterion_group!(benches, bench_world);
criterion_main!(benches);
