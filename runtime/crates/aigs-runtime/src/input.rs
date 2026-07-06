//! Keyboard and mouse state exposed to game systems.

use std::collections::HashSet;

use winit::event::{ElementState, KeyEvent};
use winit::keyboard::PhysicalKey;

pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

/// Snapshot of the input devices, updated by the runner every frame.
#[derive(Debug, Default)]
pub struct Input {
    pressed: HashSet<KeyCode>,
    just_pressed: HashSet<KeyCode>,
    just_released: HashSet<KeyCode>,
    mouse_pressed: HashSet<MouseButton>,
    mouse_just_pressed: HashSet<MouseButton>,
    mouse_position: (f32, f32),
    viewport: (f32, f32),
}

impl Input {
    /// The key is currently held down.
    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    /// The key went down since the previous simulation tick.
    pub fn key_just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed.contains(&key)
    }

    /// The key went up since the previous simulation tick.
    pub fn key_just_released(&self, key: KeyCode) -> bool {
        self.just_released.contains(&key)
    }

    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed.contains(&button)
    }

    pub fn mouse_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_just_pressed.contains(&button)
    }

    /// Cursor position in window pixels, origin at the top-left corner.
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    /// Current window size in pixels (needed to map the cursor to world
    /// coordinates).
    pub fn viewport(&self) -> (f32, f32) {
        self.viewport
    }

    // -- fed by the runner ---------------------------------------------------

    pub(crate) fn on_key_event(&mut self, event: &KeyEvent) {
        let PhysicalKey::Code(code) = event.physical_key else {
            return;
        };
        match event.state {
            ElementState::Pressed => {
                if self.pressed.insert(code) {
                    self.just_pressed.insert(code);
                }
            }
            ElementState::Released => {
                if self.pressed.remove(&code) {
                    self.just_released.insert(code);
                }
            }
        }
    }

    pub(crate) fn on_mouse_button(&mut self, state: ElementState, button: MouseButton) {
        match state {
            ElementState::Pressed => {
                if self.mouse_pressed.insert(button) {
                    self.mouse_just_pressed.insert(button);
                }
            }
            ElementState::Released => {
                self.mouse_pressed.remove(&button);
            }
        }
    }

    pub(crate) fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }

    pub(crate) fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport = (width, height);
    }

    /// Synthesizes a key press/release exactly like a physical key event —
    /// used by virtual buttons (touch, milestone M15) and by tests to
    /// simulate device input without a window.
    pub(crate) fn simulate_key(&mut self, code: KeyCode, pressed: bool) {
        if pressed {
            if self.pressed.insert(code) {
                self.just_pressed.insert(code);
            }
        } else if self.pressed.remove(&code) {
            self.just_released.insert(code);
        }
    }

    #[cfg(test)]
    pub(crate) fn simulate_mouse(&mut self, button: MouseButton) {
        if self.mouse_pressed.insert(button) {
            self.mouse_just_pressed.insert(button);
        }
    }

    #[cfg(test)]
    pub(crate) fn simulate_end_tick(&mut self) {
        self.end_tick();
        self.mouse_pressed.clear();
    }

    /// Snapshot of the key sets (pressed, just pressed, just released),
    /// used by the script host.
    pub(crate) fn key_snapshot(&self) -> (HashSet<KeyCode>, HashSet<KeyCode>, HashSet<KeyCode>) {
        (
            self.pressed.clone(),
            self.just_pressed.clone(),
            self.just_released.clone(),
        )
    }

    /// Clears per-tick state; called after each simulation tick.
    pub(crate) fn end_tick(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
        self.mouse_just_pressed.clear();
    }
}
