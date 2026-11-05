use winit::keyboard::KeyCode;

use super::Input;

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
