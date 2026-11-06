use crate::color::Color;
use crate::ui::font::{self, Font};
use crate::ui::immediate::{TextRec, UI};
use crate::ui::rect::Rect;
use crate::ui::widget::Widget;

const WAVE_HEIGHT: f32 = 3.0;
const WAVE_SPEED: f32 = 6.0;
const WAVE_SPREAD: f32 = 0.6;
const SHAKE_REACH: f32 = 1.5;
const SHAKE_RATE: f32 = 30.0;
const RAINBOW_SPEED: f32 = 120.0;
const RAINBOW_SPREAD: f32 = 18.0;
const PULSE_SIZE: f32 = 0.12;
const PULSE_SPEED: f32 = 5.0;
const PULSE_SPREAD: f32 = 0.4;
const ICON_MARK: char = '\u{FFFC}';

#[derive(Clone, Debug, Default)]
struct Look {
    color: Option<Color>,
    size: Option<f32>,
    bold: bool,
    italic: bool,
    wave: bool,
    shake: bool,
    rainbow: bool,
    pulse: bool,
    speed: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct RichChar {
    pub ch: char,
    look: Look,
    pause: f32,
    icon: Option<String>,
}

impl RichChar {
    fn font(&self) -> Font {
        match (self.look.bold, self.look.italic) {
            (true, true) => Font::BoldItalic,
            (true, false) => Font::Bold,
            (false, true) => Font::Italic,
            (false, false) => Font::Sans,
        }
    }
}

fn apply(look: &mut Look, name: &str, value: Option<&str>) -> bool {
    match (name, value) {
        ("color", Some(hex)) => {
            look.color = Some(Color::hex(&format!("#{}", hex.trim_start_matches('#'))));
        }
        ("size", Some(px)) => look.size = px.parse().ok(),
        ("speed", Some(rate)) => look.speed = rate.parse().ok(),
        ("b", _) => look.bold = true,
        ("i", _) => look.italic = true,
        ("wave", _) => look.wave = true,
        ("shake", _) => look.shake = true,
        ("rainbow", _) => look.rainbow = true,
        ("pulse", _) => look.pulse = true,
        _ => return false,
    }
    true
}

pub fn parse_rich(text: &str) -> Vec<RichChar> {
    let mut out = Vec::new();
    let mut stack: Vec<(String, Look)> = Vec::new();
    let mut look = Look::default();
    let mut pause = 0.0;
    let mut rest = text;
    while let Some(ch) = rest.chars().next() {
        let tag = (ch == '[').then(|| rest.find(']')).flatten();
        if let Some(end) = tag {
            let body = rest[1..end].trim_end_matches('/').trim();
            let (name, value) = match body.split_once('=') {
                Some((n, v)) => (n.trim(), Some(v.trim())),
                None => (body, None),
            };
            let mut known = true;
            if let Some(closing) = name.strip_prefix('/') {
                if let Some(at) = stack.iter().rposition(|(n, _)| n == closing) {
                    look = stack[at].1.clone();
                    stack.truncate(at);
                }
            } else if name == "pause" {
                pause += value.and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
            } else if name == "icon" {
                out.push(RichChar {
                    ch: ICON_MARK,
                    look: look.clone(),
                    pause,
                    icon: value.map(str::to_string),
                });
                pause = 0.0;
            } else {
                let before = look.clone();
                known = apply(&mut look, name, value);
                if known {
                    stack.push((name.to_string(), before));
                }
            }
            if known {
                rest = &rest[end + 1..];
                continue;
            }
        }
        out.push(RichChar {
            ch,
            look: look.clone(),
            pause,
            icon: None,
        });
        pause = 0.0;
        rest = &rest[ch.len_utf8()..];
    }
    out
}

fn char_time(c: &RichChar, per_second: f32) -> f32 {
    c.pause + 1.0 / (per_second * c.look.speed.unwrap_or(1.0)).max(0.01)
}

pub fn typing_time(chars: &[RichChar], per_second: f32) -> f32 {
    chars.iter().map(|c| char_time(c, per_second)).sum()
}

pub fn typed_count(chars: &[RichChar], elapsed: f32, per_second: f32) -> usize {
    let mut spent = 0.0;
    for (i, c) in chars.iter().enumerate() {
        spent += char_time(c, per_second);
        if spent > elapsed {
            return i;
        }
    }
    chars.len()
}

pub struct RichText<'a> {
    x: f32,
    y: f32,
    text: &'a str,
    width: f32,
    size: f32,
    color: Option<Color>,
    z: f32,
    visible: usize,
}

impl<'a> RichText<'a> {
    pub fn new(x: f32, y: f32, text: &'a str) -> Self {
        RichText {
            x,
            y,
            text,
            width: f32::MAX,
            size: 16.0,
            color: None,
            z: 0.0,
            visible: usize::MAX,
        }
    }

    pub fn wrapped(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn font_size(mut self, px: f32) -> Self {
        self.size = px;
        self
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }

    pub fn visible(mut self, count: usize) -> Self {
        self.visible = count;
        self
    }
}

fn width_of(c: &RichChar, base: f32) -> f32 {
    let px = c.look.size.unwrap_or(base);
    if c.icon.is_some() {
        px
    } else {
        font::advance(c.font(), px, c.ch)
    }
}

fn layout(chars: &[RichChar], base: f32, max_w: f32) -> Vec<(f32, f32)> {
    let line_h = font::line_height(Font::Sans, base);
    let mut spots = Vec::with_capacity(chars.len());
    let (mut x, mut y) = (0.0, 0.0);
    let mut i = 0;
    while i < chars.len() {
        if chars[i].ch == '\n' {
            spots.push((x, y));
            x = 0.0;
            y += line_h;
            i += 1;
            continue;
        }
        let end = chars[i..]
            .iter()
            .position(|c| c.ch == ' ' || c.ch == '\n')
            .map_or(chars.len(), |p| i + p + 1);
        let word: f32 = chars[i..end].iter().map(|c| width_of(c, base)).sum();
        if x > 0.0 && x + word > max_w {
            x = 0.0;
            y += line_h;
        }
        for c in &chars[i..end] {
            spots.push((x, y));
            x += width_of(c, base);
        }
        i = end;
    }
    spots
}

fn jitter(seed: u32) -> f32 {
    let bits = seed.wrapping_mul(0x9E37_79B9) ^ (seed >> 7);
    (bits.wrapping_mul(0x85EB_CA6B) >> 8) as f32 / (1 << 24) as f32 * 2.0 - 1.0
}

impl Widget for RichText<'_> {
    type Out = ();

    fn render(self, ui: &mut UI) {
        let chars = parse_rich(self.text);
        let spots = layout(&chars, self.size, self.width);
        let time = ui.elapsed();
        let base = self.color.unwrap_or(ui.theme.label);
        let recording = std::mem::replace(&mut ui.record_texts, false);
        let shown = chars.len().min(self.visible);
        for (i, (c, &(cx, cy))) in chars.iter().zip(&spots).enumerate().take(shown) {
            let step = i as f32;
            let mut px = c.look.size.unwrap_or(self.size);
            let (mut x, mut y) = (self.x + cx, self.y + cy);
            if c.look.wave {
                y += (time * WAVE_SPEED + step * WAVE_SPREAD).sin() * WAVE_HEIGHT;
            }
            if c.look.shake {
                let seed = (time * SHAKE_RATE) as u32 * 977 + i as u32;
                x += jitter(seed) * SHAKE_REACH;
                y += jitter(seed + 1) * SHAKE_REACH;
            }
            if c.look.pulse {
                px *= 1.0 + PULSE_SIZE * (time * PULSE_SPEED + step * PULSE_SPREAD).sin();
            }
            let color = if c.look.rainbow {
                Color::hsv(
                    (time * RAINBOW_SPEED + step * RAINBOW_SPREAD) % 360.0,
                    0.7,
                    1.0,
                )
            } else {
                c.look.color.unwrap_or(base)
            };
            if let Some(name) = &c.icon {
                let spot = Rect::new(x, y, px, px).with_z(self.z);
                ui.sprite(spot, name.as_str(), Color::WHITE);
            } else if !c.ch.is_whitespace() {
                let mut buffer = [0; 4];
                let glyph = c.ch.encode_utf8(&mut buffer);
                ui.glyphs_font(x, y, c.font(), px, glyph, color, self.z);
            }
        }
        ui.record_texts = recording;
        if recording && shown > 0 {
            let text: String = chars[..shown].iter().map(|c| c.ch).collect();
            let rect = Rect::new(self.x, self.y, self.width.min(4096.0), self.size).with_z(self.z);
            ui.texts.push(TextRec {
                text,
                rect,
                alpha: base.a,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(chars: &[RichChar]) -> String {
        chars.iter().map(|c| c.ch).collect()
    }

    #[test]
    fn tags_leave_only_visible_text_and_nest() {
        let chars = parse_rich("a[b]c[wave]d[/wave]e[/b]f");
        assert_eq!(plain(&chars), "acdef");
        assert!(chars[1].look.bold && !chars[1].look.wave);
        assert!(chars[2].look.bold && chars[2].look.wave);
        assert!(!chars[4].look.bold);
    }

    #[test]
    fn unknown_tags_stay_as_text() {
        assert_eq!(plain(&parse_rich("[nope]x")), "[nope]x");
    }

    #[test]
    fn pause_and_speed_change_typing_time() {
        let plain_time = typing_time(&parse_rich("ab"), 10.0);
        let paused = typing_time(&parse_rich("a[pause=0.5/]b"), 10.0);
        assert!((paused - plain_time - 0.5).abs() < 1e-4);
        let fast = typing_time(&parse_rich("[speed=2]ab[/speed]"), 10.0);
        assert!((fast - plain_time / 2.0).abs() < 1e-4);
    }

    #[test]
    fn typing_stops_at_the_pause() {
        let chars = parse_rich("a[pause=1]b");
        assert_eq!(typed_count(&chars, 0.5, 10.0), 1);
        assert_eq!(typed_count(&chars, 1.3, 10.0), 2);
    }

    #[test]
    fn long_text_wraps_by_word() {
        let chars = parse_rich("hello world");
        let wide = layout(&chars, 16.0, 1000.0);
        let thin = layout(&chars, 16.0, 60.0);
        assert_eq!(wide[6].1, 0.0);
        assert!(thin[6].1 > 0.0 && thin[6].0 == 0.0);
    }
}
