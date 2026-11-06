use winit::keyboard::KeyCode;

use super::Input;
use crate::ui::EditInput;

const REPEAT_DELAY: f32 = 0.2;
const REPEAT_INTERVAL: f32 = 0.025;

impl Input {
    pub fn set_key(&mut self, key: KeyCode, pressed: bool) {
        if pressed {
            if self.keys_down.insert(key) {
                self.keys_pressed.insert(key);
                self.keys_struck.insert(key);
                self.key_held_time.insert(key, 0.0);
            }
        } else if self.keys_down.remove(&key) {
            self.keys_released.insert(key);
            self.key_held_time.remove(&key);
        }
    }

    pub fn tick(&mut self, dt: f32) {
        for (key, t) in self.key_held_time.iter_mut() {
            let before = *t;
            *t += dt;
            let fired = if before < REPEAT_DELAY {
                *t >= REPEAT_DELAY
            } else {
                let prev = ((before - REPEAT_DELAY) / REPEAT_INTERVAL).floor();
                let now = ((*t - REPEAT_DELAY) / REPEAT_INTERVAL).floor();
                now > prev
            };
            if fired {
                self.keys_pressed.insert(*key);
            }
        }
    }

    fn ctrl(&self) -> bool {
        self.key_held(KeyCode::ControlLeft) || self.key_held(KeyCode::ControlRight)
    }

    pub fn edit_input(&self) -> EditInput {
        let ctrl = self.ctrl();
        EditInput {
            typed: if ctrl {
                String::new()
            } else {
                self.typed.clone()
            },
            backspace: self.key_pressed(KeyCode::Backspace),
            delete: self.key_pressed(KeyCode::Delete),
            enter: self.key_pressed(KeyCode::Enter),
            left: self.key_pressed(KeyCode::ArrowLeft),
            right: self.key_pressed(KeyCode::ArrowRight),
            up: self.key_pressed(KeyCode::ArrowUp),
            down: self.key_pressed(KeyCode::ArrowDown),
            home: self.key_pressed(KeyCode::Home),
            end: self.key_pressed(KeyCode::End),
            shift: self.key_held(KeyCode::ShiftLeft) || self.key_held(KeyCode::ShiftRight),
            ctrl,
            select_all: ctrl && self.key_pressed(KeyCode::KeyA),
            copy: ctrl && self.key_pressed(KeyCode::KeyC),
            cut: ctrl && self.key_pressed(KeyCode::KeyX),
            paste: (ctrl && self.key_pressed(KeyCode::KeyV))
                .then(crate::clipboard::get)
                .flatten(),
        }
    }

    pub fn push_text(&mut self, text: &str) {
        for c in text.chars() {
            if !c.is_control() {
                self.typed.push(c);
            }
        }
    }

    pub fn key_held(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn consume_key(&mut self, key: KeyCode) {
        self.keys_pressed.remove(&key);
        self.keys_struck.remove(&key);
    }

    pub fn key_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_input_carries_typed_text_and_held_keys() {
        let mut input = Input::default();
        input.push_text("ab");
        input.set_key(KeyCode::Backspace, true);
        let edit = input.edit_input();
        assert_eq!(edit.typed, "ab");
        assert!(edit.backspace && !edit.ctrl);
        input.set_key(KeyCode::ControlLeft, true);
        assert_eq!(input.edit_input().typed, "");
    }
}
