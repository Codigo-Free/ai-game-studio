//! Milestone M1 deliverable: hundreds of sprites bouncing around a window,
//! rendered by the AI Game Studio runtime with no editor involved.
//!
//! Environment variables:
//! - `AIGS_SPRITES`: number of sprites (default 500).
//! - `AIGS_MAX_FRAMES`: exit after N frames (smoke testing).
//! - Press `Escape` to quit, `Space` to reverse time... well, velocities.

use aigs_runtime::{run, AppConfig, Camera2D, Color, KeyCode, Sprite, Transform2D, World};

const WORLD_WIDTH: f32 = 1280.0;
const WORLD_HEIGHT: f32 = 720.0;

struct Velocity {
    dx: f32,
    dy: f32,
}

/// Tiny deterministic PRNG (xorshift) so the example needs no dependencies.
struct Rng(u64);

impl Rng {
    fn next_f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }
}

/// Generates a soft disc texture procedurally (no asset files needed).
fn disc_pixels(size: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    let center = (size as f32 - 1.0) / 2.0;
    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 - center) / center;
            let dy = (y as f32 - center) / center;
            let distance = (dx * dx + dy * dy).sqrt();
            let alpha = ((1.0 - distance) * 4.0).clamp(0.0, 1.0);
            pixels.extend_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
        }
    }
    pixels
}

fn main() {
    let sprite_count: usize = std::env::var("AIGS_SPRITES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(500);
    let max_frames: Option<u64> = std::env::var("AIGS_MAX_FRAMES")
        .ok()
        .and_then(|value| value.parse().ok());

    let config = AppConfig {
        title: format!("AI Game Studio — bouncing-sprites ({sprite_count})"),
        width: WORLD_WIDTH as u32,
        height: WORLD_HEIGHT as u32,
        clear_color: Color::rgba(0.06, 0.07, 0.11, 1.0),
        max_frames,
        ..AppConfig::default()
    };

    let result = run(
        config,
        move |world: &mut World, renderer| {
            let texture = renderer.create_texture_rgba(32, 32, &disc_pixels(32));

            let camera = world.spawn();
            world.insert(camera, Transform2D::default());
            world.insert(camera, Camera2D::default());

            let mut rng = Rng(0x5DEECE66D);
            for _ in 0..sprite_count {
                let entity = world.spawn();
                let size = rng.range(8.0, 40.0);
                world.insert(
                    entity,
                    Transform2D::at(
                        rng.range(-WORLD_WIDTH / 2.0, WORLD_WIDTH / 2.0),
                        rng.range(-WORLD_HEIGHT / 2.0, WORLD_HEIGHT / 2.0),
                    ),
                );
                let mut sprite = Sprite::new(texture, size, size);
                sprite.opacity = rng.range(0.4, 1.0);
                world.insert(entity, sprite);
                world.insert(
                    entity,
                    Velocity {
                        dx: rng.range(-250.0, 250.0),
                        dy: rng.range(-250.0, 250.0),
                    },
                );
            }
        },
        |world, time, input| {
            if input.key_just_pressed(KeyCode::Escape) {
                std::process::exit(0);
            }
            let reverse = input.key_just_pressed(KeyCode::Space);
            world.for_each2::<Transform2D, Velocity>(|_, transform, velocity| {
                if reverse {
                    velocity.dx = -velocity.dx;
                    velocity.dy = -velocity.dy;
                }
                transform.x += velocity.dx * time.delta;
                transform.y += velocity.dy * time.delta;
                let half_width = WORLD_WIDTH / 2.0;
                let half_height = WORLD_HEIGHT / 2.0;
                if transform.x.abs() > half_width {
                    transform.x = transform.x.clamp(-half_width, half_width);
                    velocity.dx = -velocity.dx;
                }
                if transform.y.abs() > half_height {
                    transform.y = transform.y.clamp(-half_height, half_height);
                    velocity.dy = -velocity.dy;
                }
            });
        },
    );

    if let Err(error) = result {
        eprintln!("bouncing-sprites failed: {error}");
        std::process::exit(1);
    }
}
