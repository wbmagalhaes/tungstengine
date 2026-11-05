use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::ecs::prelude::*;
use crate::input::Input;
use crate::ui::{TextRec, UI};

use super::app::{FrameUi, install_core, tick};

pub type ScriptExt = fn(&mut World, &[String]) -> Option<String>;

pub struct Headless {
    pub world: World,
    ui: UI,
    frame_ui: FrameUi,
    pub width: f32,
    pub height: f32,
    pub dt: f32,
    pub draw_ui: bool,
    frames: u64,
}

impl Headless {
    pub fn new(mut world: World, frame_ui: FrameUi, width: f32, height: f32) -> Self {
        install_core(&mut world);
        let mut ui = UI::new();
        ui.record_texts = true;
        let mut h = Headless {
            world,
            ui,
            frame_ui,
            width,
            height,
            dt: 1.0 / 60.0,
            draw_ui: true,
            frames: 0,
        };
        h.frame();
        h
    }

    pub fn frame(&mut self) {
        tick(
            &mut self.world,
            &mut self.ui,
            self.draw_ui.then_some(self.frame_ui),
            self.width,
            self.height,
            self.dt,
        );
        self.frames += 1;
    }

    pub fn frames(&mut self, n: u64) {
        for _ in 0..n {
            self.frame();
        }
    }

    pub fn wait(&mut self, secs: f32) {
        self.frames((secs / self.dt).ceil().max(1.0) as u64);
    }

    fn input(&mut self) -> Option<&mut Input> {
        self.world.resource_mut::<Input>().ok()
    }

    pub fn key(&mut self, key: KeyCode, down: bool) {
        if let Some(i) = self.input() {
            i.set_key(key, down);
        }
    }

    pub fn tap(&mut self, key: KeyCode) {
        self.key(key, true);
        self.frame();
        self.key(key, false);
        self.frame();
    }

    pub fn hold(&mut self, key: KeyCode, secs: f32) {
        self.key(key, true);
        self.wait(secs);
        self.key(key, false);
        self.frame();
    }

    pub fn move_cursor(&mut self, x: f32, y: f32) {
        if let Some(i) = self.input() {
            i.cursor = (x, y);
        }
        self.frame();
    }

    pub fn click(&mut self, x: f32, y: f32) {
        self.move_cursor(x, y);
        if let Some(i) = self.input() {
            i.set_mouse(MouseButton::Left, true);
        }
        self.frame();
        if let Some(i) = self.input() {
            i.set_mouse(MouseButton::Left, false);
        }
        self.frame();
    }

    pub fn drag(&mut self, from: (f32, f32), to: (f32, f32)) {
        self.move_cursor(from.0, from.1);
        if let Some(i) = self.input() {
            i.set_mouse(MouseButton::Left, true);
        }
        self.frame();
        for k in 1..=8 {
            let t = k as f32 / 8.0;
            self.move_cursor(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
        }
        if let Some(i) = self.input() {
            i.set_mouse(MouseButton::Left, false);
        }
        self.frame();
    }

    pub fn texts(&self) -> &[TextRec] {
        &self.ui.texts
    }

    pub fn find_text(&self, label: &str) -> Option<TextRec> {
        let exact = self.ui.texts.iter().filter(|t| t.text == label);
        let partial = self.ui.texts.iter().filter(|t| t.text.contains(label));
        exact
            .max_by(|a, b| a.rect.z.total_cmp(&b.rect.z))
            .or_else(|| partial.max_by(|a, b| a.rect.z.total_cmp(&b.rect.z)))
            .cloned()
    }

    pub fn click_text(&mut self, label: &str) -> bool {
        match self.find_text(label) {
            Some(t) => {
                self.click(t.rect.x + t.rect.w / 2.0, t.rect.y + t.rect.h / 2.0);
                true
            }
            None => false,
        }
    }

    pub fn screen_texts(&self) -> Vec<String> {
        let mut seen: Vec<(String, i32, i32)> = Vec::new();
        for t in &self.ui.texts {
            let key = (t.text.clone(), t.rect.x as i32 / 6, t.rect.y as i32 / 6);
            if !seen.contains(&key) {
                seen.push(key);
            }
        }
        seen.sort_by_key(|(_, x, y)| (*y, *x));
        seen.into_iter()
            .map(|(t, x, y)| format!("[{:>4},{:>4}] {t}", x * 6, y * 6))
            .collect()
    }

    pub fn run_script(&mut self, src: &str, ext: ScriptExt) -> (String, bool) {
        let mut out = String::new();
        let mut ok = true;
        for (n, raw) in src.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let args = split_args(line);
            out.push_str(&format!("> {line}\n"));
            match self.command(&args, ext) {
                Ok(Some(text)) if !text.is_empty() => {
                    out.push_str(&text);
                    if !text.ends_with('\n') {
                        out.push('\n');
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    ok = false;
                    out.push_str(&format!("FAILED (line {}): {e}\n", n + 1));
                }
            }
        }
        out.push_str(&format!("# {} frames simulated\n", self.frames));
        (out, ok)
    }

    fn command(&mut self, a: &[String], ext: ScriptExt) -> Result<Option<String>, String> {
        let num = |i: usize| -> Result<f32, String> {
            a.get(i)
                .and_then(|v| v.parse::<f32>().ok())
                .ok_or_else(|| format!("invalid argument {i}"))
        };
        let key = |i: usize| -> Result<KeyCode, String> {
            a.get(i)
                .and_then(|v| key_code(v))
                .ok_or_else(|| format!("unknown key: {:?}", a.get(i)))
        };
        match a.first().map(String::as_str) {
            Some("wait") => self.wait(num(1)?),
            Some("frames") => self.frames(num(1)? as u64),
            Some("tap") => self.tap(key(1)?),
            Some("press") => {
                self.key(key(1)?, true);
                self.frame();
            }
            Some("release") => {
                self.key(key(1)?, false);
                self.frame();
            }
            Some("hold") => self.hold(key(1)?, num(2)?),
            Some("move") => self.move_cursor(num(1)?, num(2)?),
            Some("click") if a.len() == 2 => {
                if !self.click_text(&a[1]) {
                    return Err(format!("text not found on screen: {:?}", a[1]));
                }
            }
            Some("click") => self.click(num(1)?, num(2)?),
            Some(cmd @ ("mouse_down" | "mouse_up")) => {
                let down = cmd == "mouse_down";
                if let Some(i) = self.input() {
                    i.set_mouse(MouseButton::Left, down);
                }
                self.frame();
            }
            Some("drag") => self.drag((num(1)?, num(2)?), (num(3)?, num(4)?)),
            Some("drag_text") => {
                let from = self
                    .find_text(&a[1])
                    .ok_or_else(|| format!("text not found: {:?}", a[1]))?;
                let to = self
                    .find_text(&a[2])
                    .ok_or_else(|| format!("text not found: {:?}", a[2]))?;
                let c = |t: &TextRec| (t.rect.x + t.rect.w / 2.0, t.rect.y + t.rect.h / 2.0);
                self.drag(c(&from), c(&to));
            }
            Some("texts") => return Ok(Some(self.screen_texts().join("\n"))),
            Some("expect") => {
                let want = a.get(1).ok_or("expect without text")?;
                if self.find_text(want).is_none() {
                    return Err(format!("expected to see {want:?} on screen"));
                }
            }
            Some("expect_not") => {
                let want = a.get(1).ok_or("expect_not without text")?;
                if self.find_text(want).is_some() {
                    return Err(format!("did not expect to see {want:?} on screen"));
                }
            }
            _ => {
                return match ext(&mut self.world, a) {
                    Some(text) => {
                        self.frame();
                        if let Some(err) = text.strip_prefix("ERR ") {
                            Err(err.to_string())
                        } else {
                            Ok(Some(text))
                        }
                    }
                    None => Err(format!("unknown command: {}", a[0])),
                };
            }
        }
        Ok(None)
    }
}

fn split_args(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for ch in line.chars() {
        match ch {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub fn key_code(name: &str) -> Option<KeyCode> {
    let upper = name.to_ascii_uppercase();
    if upper.len() == 1 {
        let c = upper.chars().next()?;
        if c.is_ascii_alphabetic() {
            let i = c as u8 - b'A';
            const LETTERS: [KeyCode; 26] = [
                KeyCode::KeyA,
                KeyCode::KeyB,
                KeyCode::KeyC,
                KeyCode::KeyD,
                KeyCode::KeyE,
                KeyCode::KeyF,
                KeyCode::KeyG,
                KeyCode::KeyH,
                KeyCode::KeyI,
                KeyCode::KeyJ,
                KeyCode::KeyK,
                KeyCode::KeyL,
                KeyCode::KeyM,
                KeyCode::KeyN,
                KeyCode::KeyO,
                KeyCode::KeyP,
                KeyCode::KeyQ,
                KeyCode::KeyR,
                KeyCode::KeyS,
                KeyCode::KeyT,
                KeyCode::KeyU,
                KeyCode::KeyV,
                KeyCode::KeyW,
                KeyCode::KeyX,
                KeyCode::KeyY,
                KeyCode::KeyZ,
            ];
            return Some(LETTERS[i as usize]);
        }
        if c.is_ascii_digit() {
            const DIGITS: [KeyCode; 10] = [
                KeyCode::Digit0,
                KeyCode::Digit1,
                KeyCode::Digit2,
                KeyCode::Digit3,
                KeyCode::Digit4,
                KeyCode::Digit5,
                KeyCode::Digit6,
                KeyCode::Digit7,
                KeyCode::Digit8,
                KeyCode::Digit9,
            ];
            return Some(DIGITS[(c as u8 - b'0') as usize]);
        }
    }
    Some(match upper.as_str() {
        "SPACE" => KeyCode::Space,
        "ESC" | "ESCAPE" => KeyCode::Escape,
        "ENTER" => KeyCode::Enter,
        "TAB" => KeyCode::Tab,
        "SHIFT" => KeyCode::ShiftLeft,
        "CTRL" => KeyCode::ControlLeft,
        "UP" => KeyCode::ArrowUp,
        "DOWN" => KeyCode::ArrowDown,
        "LEFT" => KeyCode::ArrowLeft,
        "RIGHT" => KeyCode::ArrowRight,
        _ => return None,
    })
}
