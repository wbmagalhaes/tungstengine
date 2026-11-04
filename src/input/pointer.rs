use winit::event::MouseButton;

use super::Input;

#[derive(Default, Clone, Copy, Debug)]
pub struct Pointer {
    pub pos: (f32, f32),
    pub pressed: bool,
    pub held: bool,
    pub released: bool,
    pub right: bool,
    pub wheel: f32,
}

impl Input {
    pub fn set_mouse(&mut self, button: MouseButton, pressed: bool) {
        if pressed {
            if self.mouse_down.insert(button) {
                self.mouse_pressed.insert(button);
            }
        } else if self.mouse_down.remove(&button) {
            self.mouse_released.insert(button);
        }
    }

    pub fn add_scroll(&mut self, delta: f32) {
        self.scroll += delta;
    }

    pub fn scroll(&self) -> f32 {
        self.scroll
    }

    pub fn pointer(&self) -> Pointer {
        Pointer {
            pos: self.cursor,
            pressed: self.mouse_pressed(MouseButton::Left),
            held: self.mouse_held(MouseButton::Left),
            released: self.mouse_released(MouseButton::Left),
            right: self.mouse_pressed(MouseButton::Right),
            wheel: self.scroll,
        }
    }

    pub fn mouse_held(&self, button: MouseButton) -> bool {
        self.mouse_down.contains(&button)
    }

    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed.contains(&button)
    }

    pub fn mouse_released(&self, button: MouseButton) -> bool {
        self.mouse_released.contains(&button)
    }
}
