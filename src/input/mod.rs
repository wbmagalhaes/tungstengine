use std::collections::{HashMap, HashSet};

use crate::ecs::Resource;

mod actions;
mod gamepad;
mod kbd;
mod pad_backend;
mod pointer;

pub use actions::{Action, Actions, Binding, InputWorld};
pub use ecs_macros::Action;
pub use gamepad::{AxisDir, PadAxis, PadButton, PadState, PlayerInput, PlayerSlot};
pub use pad_backend::PadBackend;
pub use pointer::Pointer;
pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

#[derive(Resource, Default)]
#[resource(core)]
pub struct Input {
    pub cursor: (f32, f32),
    pub typed: String,
    pub actions: Actions,
    pads: Vec<PadState>,
    main_pad: Option<usize>,
    extra_players: Vec<PlayerSlot>,
    scroll: f32,
    mouse_down: HashSet<MouseButton>,
    mouse_pressed: HashSet<MouseButton>,
    mouse_released: HashSet<MouseButton>,
    keys_down: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
    keys_struck: HashSet<KeyCode>,
    keys_released: HashSet<KeyCode>,
    key_held_time: HashMap<KeyCode, f32>,
}

impl Input {
    pub fn release_all(&mut self) {
        self.mouse_down.clear();
        self.keys_released.extend(self.keys_down.drain());
        self.mouse_pressed.clear();
        self.keys_pressed.clear();
        self.keys_struck.clear();
        self.key_held_time.clear();
        self.typed.clear();
        self.scroll = 0.0;
    }

    pub fn end_frame(&mut self) {
        self.pads_end_frame();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.keys_pressed.clear();
        self.keys_struck.clear();
        self.keys_released.clear();
        self.typed.clear();
        self.scroll = 0.0;
    }
}

pub mod prelude {
    pub use super::{Action, Actions, Binding, Input, InputWorld, KeyCode, MouseButton, PadButton};
}
