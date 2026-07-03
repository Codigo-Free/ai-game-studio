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
    mouse_pressed: HashSet<MouseButton>,
    mouse_just_pressed: HashSet<MouseButton>,
    mouse_position: (f32, f32),
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
                self.pressed.remove(&code);
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

    /// Clears per-tick state; called after each simulation tick.
    pub(crate) fn end_tick(&mut self) {
        self.just_pressed.clear();
        self.mouse_just_pressed.clear();
    }
}
