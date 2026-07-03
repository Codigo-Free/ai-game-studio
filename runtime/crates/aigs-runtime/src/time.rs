//! Simulation clock exposed to game systems.

/// Time state for the current simulation tick.
#[derive(Debug, Clone, Copy, Default)]
pub struct Time {
    /// Fixed simulation step in seconds (see [`crate::FIXED_DT`]).
    pub delta: f32,
    /// Total simulated time in seconds.
    pub elapsed: f64,
    /// Number of simulation ticks since startup.
    pub tick: u64,
    /// Rendered frames per second, measured over the last second.
    pub fps: f32,
}
