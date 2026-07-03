//! Game loop, base components and window runner of the AI Game Studio
//! runtime (milestone M1).
//!
//! [`run`] opens a window and drives a fixed-timestep simulation (60 Hz) with
//! interpolated rendering on top of [`aigs_ecs`] and [`aigs_render`]. Loading
//! worlds from `.aigs` scenes arrives in milestone M2 (see `docs/plan.md`).

mod app;
mod components;
mod input;
mod time;

pub use aigs_ecs::{Entity, Schedule, World};
pub use aigs_render::{Color, Renderer, TextureId, Viewport};
pub use app::{run, AppConfig, RunError, FIXED_DT};
pub use components::{Camera2D, Name, PrevTransform2D, Sprite, Transform2D, Visibility};
pub use input::{Input, KeyCode, MouseButton};
pub use time::Time;
