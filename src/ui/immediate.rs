use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash, Hasher};

use super::animation::{Anim, Presence, apply_anim, bbox_center};
use super::font;
use super::input::EditInput;
use super::layout::{Dir, Layout};
use super::rect::Rect;
use super::sprite::{DrawRect, SpriteDraw};
use super::style::Styled;
use super::theme::Theme;
use super::widget::{Backdrop, Button, Response, Text, Widget};
use crate::color::Color;
use crate::input::Pointer;
use crate::sprites::SpriteArg;

type IdMap<V> = HashMap<u64, V, BuildHasherDefault<IdHasher>>;

#[derive(Default)]
pub struct IdHasher(u64);

impl Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = self.0.rotate_left(8) ^ b as u64;
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = n;
    }
}

#[derive(Default, Clone)]
pub struct TextEdit {
    pub typed: String,
    pub backspace: bool,
    pub delete: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub home: bool,
    pub end: bool,
    pub enter: bool,
    pub shift: bool,
    pub select_all: bool,
    pub copy: bool,
    pub cut: bool,
    pub paste: Option<String>,
}

const MULTI_CLICK_SECS: f32 = 0.35;

fn byte_at(s: &str, chars: usize) -> usize {
    s.char_indices().nth(chars).map_or(s.len(), |(b, _)| b)
}

fn word_at(s: &str, at: usize) -> (usize, usize) {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return (0, 0);
    }
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let pivot = at.min(chars.len() - 1);
    let kind = word(chars[pivot]);
    let mut a = pivot;
    while a > 0 && word(chars[a - 1]) == kind {
        a -= 1;
    }
    let mut b = pivot + 1;
    while b < chars.len() && word(chars[b]) == kind {
        b += 1;
    }
    (a, b)
}

#[derive(Default, Clone, Copy)]
pub struct FieldResponse {
    pub changed: bool,
    pub submitted: bool,
    pub focused: bool,
}

#[derive(Clone, Debug)]
pub struct TextRec {
    pub text: String,
    pub rect: Rect,
    pub alpha: f32,
}

#[derive(Default)]
pub struct UI {
    pub draws: Vec<DrawRect>,
    pub texts: Vec<TextRec>,
    pub width: f32,
    pub height: f32,
    pub cursor: (f32, f32),
    pub clicked: bool,
    pub pressed: bool,
    down: bool,
    wheel: f32,
    scrolls: IdMap<(f32, Option<f32>)>,
    repeats: IdMap<(f32, f32)>,
    states: IdMap<Box<dyn std::any::Any>>,
    pub theme: Theme,
    holds: IdMap<(f32, bool)>,
    focus: Option<u64>,
    keyboard_used: bool,
    edit: EditInput,
    focused: Option<u64>,
    caret: usize,
    sel_anchor: usize,
    clipboard_out: Option<String>,
    caret_phase: f32,
    carets: IdMap<(usize, usize)>,
    drag_field: Option<u64>,
    focus_text: String,
    last_click: Option<(u64, f32, u32)>,
    pub copied: Option<String>,
    dt: f32,
    time: f32,
    anims: IdMap<f32>,
    anims_seen: std::collections::HashSet<u64>,
    hovers: IdMap<bool>,
    tween_seq: u64,
    id_counts: IdMap<u32>,
    presence: IdMap<Presence>,
    presence_stack: Vec<(u64, usize, Anim, [f32; 4])>,
    pub pointer_used: bool,
    block_z: f32,
    capture_z: f32,
    clip_stack: Vec<Rect>,
    wrap_cache: RefCell<WrapCache>,
    wrap_scratch: Vec<(usize, usize)>,
}

#[derive(Default)]
struct WrapCache {
    entries: Vec<(u64, Vec<(usize, usize)>)>,
}

const WRAP_CACHE_CAP: usize = 8;

impl UI {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_edit(&mut self, edit: EditInput) {
        self.keyboard_used = false;
        self.edit = edit;
        if self.clicked {
            self.focused = None;
        }
    }

    pub fn keyboard_used(&self) -> bool {
        self.keyboard_used
    }

    pub fn focused_id(&self) -> Option<u64> {
        self.focused
    }

    pub fn set_focus(&mut self, focus: Option<u64>) {
        self.focused = focus;
    }

    pub fn caret_sel(&self) -> (usize, usize) {
        (self.caret, self.sel_anchor)
    }

    pub fn set_caret_sel(&mut self, caret: usize, anchor: usize) {
        self.caret = caret;
        self.sel_anchor = anchor;
    }

    pub fn edit_input(&self) -> EditInput {
        self.edit.clone()
    }

    pub fn mark_keyboard(&mut self) {
        self.keyboard_used = true;
    }

    pub fn set_clipboard_out(&mut self, text: String) {
        self.clipboard_out = Some(text);
    }

    pub fn reset_blink(&mut self) {
        self.caret_phase = self.time;
    }

    pub fn caret_on(&self) -> bool {
        (self.time - self.caret_phase).rem_euclid(1.0) < 0.5
    }

    pub fn take_clipboard(&mut self) -> Option<String> {
        self.clipboard_out.take()
    }

    pub fn focus(&mut self, id: impl Hash) {
        self.focused = Some(key_hash(id));
        self.caret = usize::MAX;
        self.sel_anchor = usize::MAX;
    }

    pub fn is_focused(&self, id: impl Hash) -> bool {
        self.focused == Some(key_hash(id))
    }

    pub fn backdrop(&mut self) {
        self.add(Backdrop::new());
    }

    pub fn text(&mut self, x: f32, y: f32, s: &str) {
        self.add(Text::new(x, y, s))
    }

    pub fn text_in(&mut self, r: Rect, s: &str) {
        self.add(Text::centered(r, s))
    }

    pub fn button(&mut self, r: Rect, label: &str) -> Response {
        self.add(Button::new(r, label))
    }

    pub fn begin(&mut self, w: f32, h: f32, dt: f32, cursor: Pointer) {
        self.draws.clear();
        self.texts.clear();
        self.width = w;
        self.height = h;
        self.dt = dt;
        self.time += dt;
        self.tween_seq = 0;
        let seen = std::mem::take(&mut self.anims_seen);
        self.anims.retain(|id, _| seen.contains(id));
        self.id_counts.clear();
        self.clip_stack.clear();
        self.pointer_used = false;
        self.capture_z = self.block_z;
        self.block_z = 0.0;

        self.cursor = cursor.pos;
        self.clicked = cursor.released;
        self.pressed = cursor.held;
        self.down = cursor.pressed;
        self.wheel = cursor.wheel;
    }

    fn field_input(
        &mut self,
        key: u64,
        rect: Rect,
        value: &mut String,
        edit: &TextEdit,
        hit: impl Fn(&Self, &str) -> usize,
        vertical: impl Fn(&Self, &str, usize, i32) -> usize,
    ) -> (FieldResponse, bool, usize, usize) {
        let hover = rect.contains(self.cursor);
        let len = value.chars().count();
        let (mut caret, mut anchor) = self.carets.get(&key).copied().unwrap_or((len, len));
        caret = caret.min(len);
        anchor = anchor.min(len);
        let mut blurred = false;
        if self.down {
            if hover {
                let at = hit(self, value);
                let count = match self.last_click {
                    Some((k, t, n)) if k == key && self.time - t < MULTI_CLICK_SECS => n + 1,
                    _ => 1,
                };
                self.last_click = Some((key, self.time, count));
                let extend = edit.shift && self.focus == Some(key);
                if self.focus != Some(key) {
                    self.focus_text = value.clone();
                }
                self.focus = Some(key);
                match (count - 1) % 3 {
                    0 => {
                        caret = at;
                        if !extend {
                            anchor = at;
                        }
                        self.drag_field = Some(key);
                    }
                    1 => (anchor, caret) = word_at(value, at),
                    _ => (anchor, caret) = (0, len),
                }
            } else if self.focus == Some(key) {
                self.focus = None;
                blurred = *value != self.focus_text;
            }
        } else if self.pressed && self.drag_field == Some(key) {
            caret = hit(self, value);
        }
        if !self.pressed && self.drag_field == Some(key) {
            self.drag_field = None;
        }
        let focused = self.focus == Some(key);
        let mut out = FieldResponse {
            submitted: blurred,
            ..Default::default()
        };
        if focused {
            let range = |c: usize, a: usize| (c.min(a), c.max(a));
            if edit.select_all {
                (anchor, caret) = (0, len);
            }
            if (edit.copy || edit.cut) && caret != anchor {
                let (a, b) = range(caret, anchor);
                self.copied = Some(value[byte_at(value, a)..byte_at(value, b)].to_string());
            }
            let mut insert = edit.typed.clone();
            if let Some(text) = &edit.paste {
                insert.push_str(&text.replace(['\r', '\n'], " "));
            }
            let erase = edit.cut || edit.backspace || edit.delete || !insert.is_empty();
            if erase && caret != anchor {
                let (a, b) = range(caret, anchor);
                value.replace_range(byte_at(value, a)..byte_at(value, b), "");
                (caret, anchor) = (a, a);
                out.changed = true;
            } else if edit.backspace && caret > 0 {
                value.replace_range(byte_at(value, caret - 1)..byte_at(value, caret), "");
                caret -= 1;
                anchor = caret;
                out.changed = true;
            } else if edit.delete && caret < value.chars().count() {
                value.replace_range(byte_at(value, caret)..byte_at(value, caret + 1), "");
                anchor = caret;
                out.changed = true;
            }
            if !insert.is_empty() {
                value.insert_str(byte_at(value, caret), &insert);
                caret += insert.chars().count();
                anchor = caret;
                out.changed = true;
            }
            let len = value.chars().count();
            let collapse = caret != anchor && !edit.shift;
            let moved = if edit.left {
                Some(if collapse {
                    caret.min(anchor)
                } else {
                    caret.saturating_sub(1)
                })
            } else if edit.right {
                Some(if collapse {
                    caret.max(anchor)
                } else {
                    (caret + 1).min(len)
                })
            } else if edit.up {
                Some(vertical(self, value, caret, -1))
            } else if edit.down {
                Some(vertical(self, value, caret, 1))
            } else if edit.home {
                Some(vertical(self, value, caret, i32::MIN))
            } else if edit.end {
                Some(vertical(self, value, caret, i32::MAX))
            } else {
                None
            };
            if let Some(c) = moved {
                caret = c;
                if !edit.shift {
                    anchor = c;
                }
            }
            if edit.enter {
                out.submitted = true;
                self.focus = None;
            }
        }
        self.carets.insert(key, (caret, anchor));
        let border = if focused {
            Color::hex("#f0c040")
        } else if hover {
            Color::hex("#8a80b0")
        } else {
            Color::hex("#4a4060")
        };
        self.fill(rect, border);
        self.fill(
            rect.with_padding(1.0).inner().with_z(rect.z + 0.001),
            Color::hex("#14101e"),
        );
        out.focused = focused;
        (out, focused, caret, anchor)
    }

    fn selection(&mut self, x: f32, y: f32, px: f32, text: &str, span: (usize, usize), z: f32) {
        let (from, to) = span;
        if from >= to {
            return;
        }
        let x0 = self.text_width(px, &text[..byte_at(text, from)]);
        let x1 = self.text_width(px, &text[..byte_at(text, to)]);
        self.fill(
            Rect::new(x + x0, y, (x1 - x0).max(2.0), px * 1.15).with_z(z),
            Color::hex("#3a5fa8").with_alpha(0.75),
        );
    }

    fn nearest_char(&self, px: f32, text: &str, x: f32) -> usize {
        let mut best = (0, x.abs());
        for (n, (b, c)) in text.char_indices().enumerate() {
            let w = self.text_width(px, &text[..b + c.len_utf8()]);
            let d = (w - x).abs();
            if d < best.1 {
                best = (n + 1, d);
            }
        }
        best.0
    }

    fn wrapped_spans(&self, px: f32, text: &str, width: f32) -> Vec<(usize, String)> {
        let mut off = 0;
        self.wrap_text(px, text, width)
            .into_iter()
            .map(|line| {
                let start = off;
                off += line.chars().count() + 1;
                (start, line)
            })
            .collect()
    }

    fn caret(&mut self, x: f32, y: f32, px: f32, z: f32) {
        if (self.time * 2.0).fract() < 0.5 {
            self.fill(Rect::new(x, y, 2.0, px).with_z(z), Color::WHITE);
        }
    }

    pub fn text_field(
        &mut self,
        id: impl Hash,
        rect: Rect,
        value: &mut String,
        edit: &TextEdit,
    ) -> FieldResponse {
        let key = key_hash(id);
        let px = (rect.h * 0.55).min(15.0);
        let shift_of = |ui: &Self, value: &str, caret: usize| {
            let cx = ui.text_width(px, &value[..byte_at(value, caret)]);
            (cx - (rect.w - 12.0)).max(0.0)
        };
        let prev = self.carets.get(&key).map_or(usize::MAX, |c| c.0);
        let (out, focused, caret, anchor) = self.field_input(
            key,
            rect,
            value,
            edit,
            |ui, v| {
                let shift = shift_of(ui, v, prev.min(v.chars().count()));
                ui.nearest_char(px, v, ui.cursor.0 - rect.x - 6.0 + shift)
            },
            |_, v, c, dir| match dir {
                i32::MIN => 0,
                i32::MAX => v.chars().count(),
                _ => c,
            },
        );
        let ty = rect.y + (rect.h - px) / 2.0;
        self.push_clip(rect);
        let cx = self.text_width(px, &value[..byte_at(value, caret)]);
        let shift = shift_of(self, value, caret);
        self.glyphs(
            rect.x + 6.0 - shift,
            ty,
            px,
            value,
            Color::WHITE,
            rect.z + 0.002,
        );
        if focused {
            let (from, to) = (caret.min(anchor), caret.max(anchor));
            self.selection(
                rect.x + 6.0 - shift,
                ty,
                px,
                value,
                (from, to),
                rect.z + 0.0015,
            );
            self.caret(rect.x + 6.0 - shift + cx, ty, px, rect.z + 0.003);
        }
        self.pop_clip();
        out
    }

    pub fn wrap_text(&self, px: f32, text: &str, width: f32) -> Vec<String> {
        let mut lines = vec![String::new()];
        for word in text.split(' ') {
            let line = lines.last_mut().expect("lines starts non-empty");
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if line.is_empty() || self.text_width(px, &candidate) <= width {
                *line = candidate;
            } else {
                lines.push(word.to_string());
            }
        }
        lines
    }

    pub fn text_area(
        &mut self,
        id: impl Hash,
        rect: Rect,
        value: &mut String,
        edit: &TextEdit,
    ) -> FieldResponse {
        let key = key_hash(id);
        let px = 14.0;
        let line_h = px * 1.35;
        let width = rect.w - 12.0;
        let visible = ((rect.h - 8.0) / line_h).floor().max(1.0) as usize;
        let line_of = |spans: &[(usize, String)], caret: usize| {
            spans
                .iter()
                .rposition(|(start, _)| *start <= caret)
                .unwrap_or(0)
        };
        let first_of = |spans: &[(usize, String)], caret: usize| {
            let line = line_of(spans, caret);
            let tail = spans.len().saturating_sub(visible);
            tail.min(line).max((line + 1).saturating_sub(visible))
        };
        let prev = self.carets.get(&key).map_or(usize::MAX, |c| c.0);
        let (out, focused, caret, anchor) = self.field_input(
            key,
            rect,
            value,
            edit,
            |ui, v| {
                let spans = ui.wrapped_spans(px, v, width);
                let first = first_of(&spans, prev.min(v.chars().count()));
                let row = ((ui.cursor.1 - rect.y - 4.0) / line_h).floor().max(0.0) as usize;
                let (start, line) = &spans[(first + row).min(spans.len() - 1)];
                start + ui.nearest_char(px, line, ui.cursor.0 - rect.x - 6.0)
            },
            |ui, v, c, dir| {
                let spans = ui.wrapped_spans(px, v, width);
                let line = line_of(&spans, c);
                let (start, text) = &spans[line];
                let len = text.chars().count();
                match dir {
                    i32::MIN => *start,
                    i32::MAX => start + len,
                    _ => {
                        let target = line as i32 + dir;
                        if target < 0 || target as usize >= spans.len() {
                            return c;
                        }
                        let x = ui.text_width(px, &text[..byte_at(text, (c - start).min(len))]);
                        let (s2, t2) = &spans[target as usize];
                        s2 + ui.nearest_char(px, t2, x)
                    }
                }
            },
        );
        let spans = self.wrapped_spans(px, value, width);
        let first = first_of(&spans, caret);
        self.push_clip(rect);
        for (n, (_, line)) in spans.iter().skip(first).take(visible).enumerate() {
            let y = rect.y + 4.0 + n as f32 * line_h;
            self.glyphs(rect.x + 6.0, y, px, line, Color::WHITE, rect.z + 0.002);
        }
        if focused {
            let (from, to) = (caret.min(anchor), caret.max(anchor));
            for (n, (start, text)) in spans.iter().enumerate().skip(first).take(visible) {
                let end = start + text.chars().count();
                let (a, b) = (from.max(*start), to.min(end));
                if a < b {
                    let y = rect.y + 4.0 + (n - first) as f32 * line_h;
                    self.selection(
                        rect.x + 6.0,
                        y,
                        px,
                        text,
                        (a - start, b - start),
                        rect.z + 0.0015,
                    );
                }
            }
            let line = line_of(&spans, caret);
            let (start, text) = &spans[line];
            let col = (caret - start).min(text.chars().count());
            let x = rect.x + 6.0 + self.text_width(px, &text[..byte_at(text, col)]);
            let n = (line - first) as f32;
            self.caret(x, rect.y + 4.0 + n * line_h, px, rect.z + 0.003);
        }
        self.pop_clip();
        out
    }

    pub fn has_focus(&self) -> bool {
        self.focus.is_some()
    }

    pub fn hold_progress(&mut self, id: impl Hash, rect: Rect, secs: f32) -> (f32, bool) {
        let key = key_hash(id);
        if !self.pressed {
            self.holds.remove(&key);
            return (0.0, false);
        }
        let (held, done) = self.holds.get(&key).copied().unwrap_or((0.0, false));
        if done || !rect.contains(self.cursor) {
            self.holds.insert(key, (0.0, done));
            return (0.0, false);
        }
        let held = held + self.dt;
        let fire = held >= secs;
        self.holds
            .insert(key, (if fire { 0.0 } else { held }, fire));
        ((held / secs).min(1.0), fire)
    }

    pub fn hold_repeat(&mut self, id: impl Hash, rect: Rect, delay: f32, interval: f32) -> bool {
        let key = key_hash(id);
        if !(self.pressed && rect.contains(self.cursor)) {
            self.repeats.remove(&key);
            return false;
        }
        let (held, next) = self.repeats.get(&key).copied().unwrap_or((0.0, delay));
        let held = held + self.dt;
        let fire = held >= next;
        let next = if fire { next + interval } else { next };
        self.repeats.insert(key, (held, next));
        fire
    }

    pub fn scroll_area(&mut self, id: impl Hash, view: Rect, content_h: f32, color: Color) -> f32 {
        let key = key_hash(id);
        let max = (content_h - view.h).max(0.0);
        let (mut offset, mut drag) = self.scrolls.get(&key).copied().unwrap_or((0.0, None));
        if view.contains(self.cursor) && self.wheel != 0.0 {
            offset -= self.wheel * 60.0;
        }
        let bar_w = 8.0;
        let track = Rect::new(view.x + view.w - bar_w, view.y, bar_w, view.h).with_z(view.z + 0.1);
        let thumb_h = if content_h > 0.0 {
            (view.h * view.h / content_h).max(30.0)
        } else {
            view.h
        };
        if max > 0.0 {
            if self.down && track.contains(self.cursor) {
                let thumb_y = view.y + (offset / max) * (view.h - thumb_h);
                let grab = self.cursor.1 - thumb_y;
                drag = Some(if (0.0..=thumb_h).contains(&grab) {
                    grab
                } else {
                    thumb_h / 2.0
                });
            }
            if let Some(grab) = drag {
                let t = (self.cursor.1 - grab - view.y) / (view.h - thumb_h).max(1.0);
                offset = t * max;
            }
        }
        if !self.pressed {
            drag = None;
        }
        offset = offset.clamp(0.0, max);
        self.scrolls.insert(key, (offset, drag));
        if max > 0.0 {
            self.fill(track, color.with_alpha(0.2));
            let thumb_y = view.y + (offset / max) * (view.h - thumb_h);
            let hot = drag.is_some() || track.contains(self.cursor);
            self.fill(
                Rect::new(track.x, thumb_y, bar_w, thumb_h).with_z(track.z + 0.01),
                color.with_alpha(if hot { 0.9 } else { 0.6 }),
            );
        }
        offset
    }

    pub fn time(&self) -> f32 {
        self.time
    }

    pub fn block_pointer(&mut self, rect: Rect) {
        if rect.contains(self.cursor) {
            self.pointer_used = true;
            self.block_z = self.block_z.max(rect.z);
        }
    }

    fn current_clip(&self) -> [f32; 4] {
        match self.clip_stack.last() {
            Some(r) => [r.x, r.y, r.w, r.h],
            None => [0.0, 0.0, f32::INFINITY, f32::INFINITY],
        }
    }

    pub fn push_clip(&mut self, rect: Rect) {
        let r = match self.clip_stack.last() {
            Some(p) => p.intersect(rect),
            None => rect,
        };
        self.clip_stack.push(r);
    }

    pub fn pop_clip(&mut self) {
        self.clip_stack.pop();
    }

    pub fn clip<R>(&mut self, rect: Rect, body: impl FnOnce(&mut Self) -> R) -> R {
        self.push_clip(rect);
        let out = body(self);
        self.pop_clip();
        out
    }

    pub fn pointer_used(&self) -> bool {
        self.pointer_used
    }

    fn next_auto_key(&mut self) -> u64 {
        self.tween_seq += 1;
        self.tween_seq.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1
    }

    fn anim_range(&mut self, start: usize, anim: Anim, key: Option<u64>, ancestor: [f32; 4]) {
        let id = key.unwrap_or_else(|| self.next_auto_key());
        let dt = self.dt;
        let st = self
            .presence
            .entry(id)
            .or_insert_with(|| Presence::new(anim, key.is_some()));
        st.present = true;
        st.anim = anim;
        st.ancestor = ancestor;
        st.retarget(1.0);
        st.tween.tick(dt);
        if st.exitable {
            st.draws.clear();
            st.draws.extend_from_slice(&self.draws[start..]);
        }
        if !st.tween.finished() {
            let frame = st.frame();
            let center = if st.exitable {
                st.snapshot_center()
            } else {
                bbox_center(&self.draws[start..])
            };
            apply_anim(&mut self.draws[start..], frame, center, ancestor);
        }
    }

    pub fn pointer_over(&mut self, rect: Rect) -> bool {
        let blocker = self.capture_z.max(self.block_z);
        let over = rect.contains(self.cursor) && rect.z >= blocker;
        if over {
            self.pointer_used = true;
            self.block_z = self.block_z.max(rect.z);
        }
        over
    }

    pub fn center_x(&self, r: Rect) -> Rect {
        r.center_x(self.width)
    }

    pub fn center_y(&self, r: Rect) -> Rect {
        r.center_y(self.height)
    }

    pub fn center(&self, r: Rect) -> Rect {
        r.center(self.width, self.height)
    }

    pub fn vertical(&self, area: Rect, gap: f32) -> Layout {
        Layout::new(Dir::Vertical, area, gap)
    }

    pub fn horizontal(&self, area: Rect, gap: f32) -> Layout {
        Layout::new(Dir::Horizontal, area, gap)
    }

    pub fn state_mut<T: Default + 'static>(&mut self, id: impl Hash) -> &mut T {
        let slot = self
            .states
            .entry(key_hash(id))
            .or_insert_with(|| Box::new(T::default()));
        if !slot.is::<T>() {
            *slot = Box::new(T::default());
        }
        slot.downcast_mut::<T>()
            .expect("widget state type mismatch")
    }

    pub fn anim(&mut self, id: u64, target: f32) -> f32 {
        let dt = self.dt;
        self.anims_seen.insert(id);
        let cur = self.anims.entry(id).or_insert(target);
        let k = 1.0 - (-14.0 * dt).exp();
        *cur += (target - *cur) * k;
        *cur
    }

    pub fn smooth_rect(&mut self, id: impl Hash, target: Rect) -> Rect {
        let id = key_hash(id);
        Rect::new(
            self.anim(id ^ 0x11, target.x),
            self.anim(id ^ 0x22, target.y),
            self.anim(id ^ 0x33, target.w),
            self.anim(id ^ 0x44, target.h),
        )
        .with_z(target.z)
    }

    pub fn hover_edges(&mut self, id: u64, hovered: bool) -> (bool, bool) {
        let prev = self.hovers.insert(id, hovered).unwrap_or(false);
        (hovered && !prev, !hovered && prev)
    }

    pub fn text_width(&self, px: f32, s: &str) -> f32 {
        self.text_width_font(font::Font::Sans, px, s)
    }

    pub fn text_width_font(&self, f: font::Font, px: f32, s: &str) -> f32 {
        s.chars().map(|ch| font::advance(f, px, ch)).sum()
    }

    pub fn wrap(&self, f: font::Font, px: f32, text: &str, max_w: f32) -> Vec<(usize, usize)> {
        self.with_wrap(f, px, text, max_w, |lines| lines.to_vec())
    }

    pub fn wrap_into(
        &self,
        f: font::Font,
        px: f32,
        text: &str,
        max_w: f32,
        out: &mut Vec<(usize, usize)>,
    ) {
        self.with_wrap(f, px, text, max_w, |lines| {
            out.clear();
            out.extend_from_slice(lines);
        });
    }

    pub fn take_wrap_scratch(&mut self) -> Vec<(usize, usize)> {
        std::mem::take(&mut self.wrap_scratch)
    }

    pub fn put_wrap_scratch(&mut self, scratch: Vec<(usize, usize)>) {
        self.wrap_scratch = scratch;
    }

    fn with_wrap<R>(
        &self,
        f: font::Font,
        px: f32,
        text: &str,
        max_w: f32,
        use_lines: impl FnOnce(&[(usize, usize)]) -> R,
    ) -> R {
        let key = key_hash((text, f, px.to_bits(), max_w.to_bits()));
        {
            let mut cache = self.wrap_cache.borrow_mut();
            if let Some(i) = cache.entries.iter().position(|(k, _)| *k == key) {
                let e = cache.entries.remove(i);
                cache.entries.push(e);
                return use_lines(&cache.entries.last().unwrap().1);
            }
        }
        let lines = self.wrap_compute(f, px, text, max_w);
        let out = use_lines(&lines);
        let mut cache = self.wrap_cache.borrow_mut();
        if cache.entries.len() >= WRAP_CACHE_CAP {
            cache.entries.remove(0);
        }
        cache.entries.push((key, lines));
        out
    }

    fn wrap_compute(&self, f: font::Font, px: f32, text: &str, max_w: f32) -> Vec<(usize, usize)> {
        let mut lines = Vec::new();
        for para in text.split('\n') {
            let base = para.as_ptr() as usize - text.as_ptr() as usize;
            let mut start = base;
            let mut end = base;
            let mut line_w = 0.0;
            let mut push = |s: usize, e: usize| lines.push((s, e));
            for word in para.split_inclusive(' ') {
                let woff = word.as_ptr() as usize - text.as_ptr() as usize;
                let w = self.text_width_font(f, px, word);
                if line_w + w > max_w && end > start {
                    push(start, end);
                    start = woff;
                    line_w = 0.0;
                }
                if w > max_w {
                    for (ci, ch) in word.char_indices() {
                        let cw = self.text_width_font(f, px, ch.encode_utf8(&mut [0; 4]));
                        if line_w + cw > max_w && end > start {
                            push(start, end);
                            start = woff + ci;
                            line_w = 0.0;
                        }
                        end = woff + ci + ch.len_utf8();
                        line_w += cw;
                    }
                } else {
                    end = woff + word.len();
                    line_w += w;
                }
            }
            push(start, end);
        }
        lines
    }

    pub fn measure(&self, f: font::Font, px: f32, text: &str, max_w: Option<f32>) -> (f32, f32) {
        let lh = font::line_height(f, px);
        let (w, n) = match max_w {
            Some(mw) => self.with_wrap(f, px, text, mw, |lines| {
                let w = lines
                    .iter()
                    .map(|&(a, b)| self.text_width_font(f, px, &text[a..b]))
                    .fold(0.0, f32::max);
                (w, lines.len())
            }),
            None => {
                let mut w = 0.0;
                let mut n = 0;
                for line in text.split('\n') {
                    w = f32::max(w, self.text_width_font(f, px, line));
                    n += 1;
                }
                (w, n)
            }
        };
        (w, n.max(1) as f32 * lh)
    }

    pub fn fill(&mut self, r: Rect, color: Color) {
        let (uv_min, uv_max) = font::white_uv();
        let clip = self.current_clip();
        self.draws.push(DrawRect {
            min: r.min(),
            max: r.max(),
            uv_min,
            uv_max,
            color: color.to_array(),
            z: r.z,
            tex: 0,
            clip,
            quad: None,
            uvs: None,
        });
    }

    pub fn line(&mut self, a: (f32, f32), b: (f32, f32), color: Color, thickness: f32) {
        self.line_z(a, b, color, thickness, 0.0);
    }

    pub fn line_z(&mut self, a: (f32, f32), b: (f32, f32), color: Color, thickness: f32, z: f32) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-4 {
            return;
        }
        let h = thickness * 0.5;
        let (nx, ny) = (-dy / len * h, dx / len * h);
        let (ex, ey) = (dx / len * h, dy / len * h);
        let (ax, ay) = (a.0 - ex, a.1 - ey);
        let (bx, by) = (b.0 + ex, b.1 + ey);
        let quad = [
            [ax + nx, ay + ny],
            [bx + nx, by + ny],
            [ax - nx, ay - ny],
            [bx - nx, by - ny],
        ];
        let (uv_min, uv_max) = font::white_uv();
        let clip = self.current_clip();
        self.draws.push(DrawRect {
            min: [a.0.min(b.0) - h, a.1.min(b.1) - h],
            max: [a.0.max(b.0) + h, a.1.max(b.1) + h],
            uv_min,
            uv_max,
            color: color.to_array(),
            z,
            tex: 0,
            clip,
            quad: Some(quad),
            uvs: None,
        });
    }

    pub fn quad(&mut self, pts: [(f32, f32); 4], color: Color) {
        self.quad_z(pts, color, 0.0);
    }

    pub fn quad_z(&mut self, pts: [(f32, f32); 4], color: Color, z: f32) {
        let xs = pts.map(|p| p.0);
        let ys = pts.map(|p| p.1);
        let min = [
            xs.into_iter().fold(f32::INFINITY, f32::min),
            ys.into_iter().fold(f32::INFINITY, f32::min),
        ];
        let max = [
            xs.into_iter().fold(f32::NEG_INFINITY, f32::max),
            ys.into_iter().fold(f32::NEG_INFINITY, f32::max),
        ];
        let (uv_min, uv_max) = font::white_uv();
        let clip = self.current_clip();
        let [a, b, c, d] = pts.map(|p| [p.0, p.1]);
        self.draws.push(DrawRect {
            min,
            max,
            uv_min,
            uv_max,
            color: color.to_array(),
            z,
            tex: 0,
            clip,
            quad: Some([a, b, d, c]),
            uvs: None,
        });
    }

    pub fn sprite_quad(
        &mut self,
        pts: [(f32, f32); 4],
        sprite: impl SpriteArg,
        uv: ([f32; 2], [f32; 2]),
        color: Color,
        z: f32,
    ) {
        let sprite = sprite.resolve();
        let xs = pts.map(|p| p.0);
        let ys = pts.map(|p| p.1);
        let lerp = |t: [f32; 2]| {
            [
                sprite.uv_min[0] + (sprite.uv_max[0] - sprite.uv_min[0]) * t[0],
                sprite.uv_min[1] + (sprite.uv_max[1] - sprite.uv_min[1]) * t[1],
            ]
        };
        let clip = self.current_clip();
        let [a, b, c, d] = pts.map(|p| [p.0, p.1]);
        self.draws.push(DrawRect {
            min: [
                xs.into_iter().fold(f32::INFINITY, f32::min),
                ys.into_iter().fold(f32::INFINITY, f32::min),
            ],
            max: [
                xs.into_iter().fold(f32::NEG_INFINITY, f32::max),
                ys.into_iter().fold(f32::NEG_INFINITY, f32::max),
            ],
            uv_min: lerp(uv.0),
            uv_max: lerp(uv.1),
            color: color.to_array(),
            z,
            tex: sprite.tex,
            clip,
            quad: Some([a, b, d, c]),
            uvs: None,
        });
    }

    pub fn tex_poly(&mut self, pts: &[(f32, f32, f32, f32)], tex: u32, color: Color, z: f32) {
        if pts.len() < 3 {
            return;
        }
        let clip = self.current_clip();
        let (x0, y0, u0, v0) = pts[0];
        for w in pts[1..].windows(2) {
            let ((x1, y1, u1, v1), (x2, y2, u2, v2)) = (w[0], w[1]);
            self.draws.push(DrawRect {
                min: [x0.min(x1).min(x2), y0.min(y1).min(y2)],
                max: [x0.max(x1).max(x2), y0.max(y1).max(y2)],
                uv_min: [0.0, 0.0],
                uv_max: [1.0, 1.0],
                color: color.to_array(),
                z,
                tex,
                clip,
                quad: Some([[x0, y0], [x1, y1], [x2, y2], [x2, y2]]),
                uvs: Some([[u0, v0], [u1, v1], [u2, v2], [u2, v2]]),
            });
        }
    }

    pub fn poly(&mut self, pts: &[(f32, f32)], color: Color, z: f32) {
        let (uv_min, _) = font::white_uv();
        let pts: Vec<_> = pts
            .iter()
            .map(|&(x, y)| (x, y, uv_min[0], uv_min[1]))
            .collect();
        self.tex_poly(&pts, 0, color, z);
    }

    pub fn triangle(&mut self, a: (f32, f32), b: (f32, f32), c: (f32, f32), color: Color) {
        self.quad([a, b, c, c], color);
    }

    pub fn sprite(&mut self, r: Rect, sprite: impl SpriteArg, color: Color) {
        let sprite = sprite.resolve();
        self.sprite_smooth(sprite.pixel_rect(r), sprite, color);
    }

    pub fn sprite_smooth(&mut self, r: Rect, sprite: impl SpriteArg, color: Color) {
        let sprite = sprite.resolve();
        let clip = self.current_clip();
        self.draws.push(DrawRect {
            min: r.min(),
            max: r.max(),
            uv_min: sprite.uv_min,
            uv_max: sprite.uv_max,
            color: color.to_array(),
            z: r.z,
            tex: sprite.tex,
            clip,
            quad: None,
            uvs: None,
        });
    }

    pub fn text_outlined(&mut self, r: Rect, text: &str, color: Color, size: f32, z: f32) {
        let o = (size / 12.0).max(2.0);
        let shadow = Color::new(0.07, 0.04, 0.11, color.a);
        for (dx, dy) in [
            (-o, 0.0),
            (o, 0.0),
            (0.0, -o),
            (0.0, o),
            (-o, -o),
            (o, -o),
            (-o, o),
            (o, o),
        ] {
            self.add(
                Text::centered(Rect::new(r.x + dx, r.y + dy, r.w, r.h), text)
                    .fg(shadow)
                    .font_size(size)
                    .z(z),
            );
        }
        self.add(
            Text::centered(r, text)
                .fg(color)
                .font_size(size)
                .z(z + 0.001),
        );
    }

    pub fn cross(&mut self, center: (f32, f32), half: f32, color: Color, thickness: f32, z: f32) {
        let (cx, cy) = center;
        self.line_z(
            (cx - half, cy - half),
            (cx + half, cy + half),
            color,
            thickness,
            z,
        );
        self.line_z(
            (cx - half, cy + half),
            (cx + half, cy - half),
            color,
            thickness,
            z,
        );
    }

    pub fn sprite_rotated(
        &mut self,
        r: Rect,
        sprite: impl SpriteArg,
        color: Color,
        quarter_turns: u8,
    ) {
        let sprite = sprite.resolve();
        let r = sprite.pixel_rect(r);
        let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
        let corners = [
            (-r.w / 2.0, -r.h / 2.0),
            (r.w / 2.0, -r.h / 2.0),
            (-r.w / 2.0, r.h / 2.0),
            (r.w / 2.0, r.h / 2.0),
        ];
        let quad = corners.map(|(mut dx, mut dy)| {
            for _ in 0..quarter_turns % 4 {
                (dx, dy) = (-dy, dx);
            }
            [cx + dx, cy + dy]
        });
        let clip = self.current_clip();
        self.draws.push(DrawRect {
            min: r.min(),
            max: r.max(),
            uv_min: sprite.uv_min,
            uv_max: sprite.uv_max,
            color: color.to_array(),
            z: r.z,
            tex: sprite.tex,
            clip,
            quad: Some(quad),
            uvs: None,
        });
    }

    pub fn sprite_pivot(
        &mut self,
        r: Rect,
        sprite: impl SpriteArg,
        color: Color,
        angle: f32,
        pivot: (f32, f32),
    ) {
        let sprite = sprite.resolve();
        let (sin, cos) = angle.sin_cos();
        let quad = [
            (r.x, r.y),
            (r.x + r.w, r.y),
            (r.x, r.y + r.h),
            (r.x + r.w, r.y + r.h),
        ]
        .map(|(x, y)| {
            let (dx, dy) = (x - pivot.0, y - pivot.1);
            [pivot.0 + dx * cos - dy * sin, pivot.1 + dx * sin + dy * cos]
        });
        let min = [
            quad.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min),
            quad.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min),
        ];
        let max = [
            quad.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max),
            quad.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max),
        ];
        let clip = self.current_clip();
        self.draws.push(DrawRect {
            min,
            max,
            uv_min: sprite.uv_min,
            uv_max: sprite.uv_max,
            color: color.to_array(),
            z: r.z,
            tex: sprite.tex,
            clip,
            quad: Some(quad),
            uvs: None,
        });
    }

    pub fn fill_pivot(&mut self, r: Rect, color: Color, angle: f32, pivot: (f32, f32)) {
        let (uv_min, uv_max) = font::white_uv();
        self.sprite_pivot(
            r,
            SpriteDraw {
                tex: 0,
                uv_min,
                uv_max,
                px: [0, 0],
            },
            color,
            angle,
            pivot,
        );
    }

    pub fn sprite_outline(&mut self, r: Rect, sprite: impl SpriteArg, color: Color, width: f32) {
        let sprite = sprite.resolve();
        let r = sprite.pixel_rect(r);
        let c = color.to_array();
        let clip = self.current_clip();
        for k in 0..8 {
            let ang = k as f32 / 8.0 * std::f32::consts::TAU;
            let (dx, dy) = (ang.cos() * width, ang.sin() * width);
            self.draws.push(DrawRect {
                min: [r.x + dx, r.y + dy],
                max: [r.x + r.w + dx, r.y + r.h + dy],
                uv_min: sprite.uv_min,
                uv_max: sprite.uv_max,
                color: c,
                z: r.z,
                tex: sprite.tex,
                clip,
                quad: None,
                uvs: None,
            });
        }
    }

    pub fn glyphs(&mut self, x: f32, y: f32, px: f32, s: &str, color: Color, z: f32) {
        self.glyphs_font(x, y, font::Font::Sans, px, s, color, z);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn glyphs_font(
        &mut self,
        x: f32,
        y: f32,
        f: font::Font,
        px: f32,
        s: &str,
        color: Color,
        z: f32,
    ) {
        let baseline = (y + font::top_to_baseline(f, px)).round();
        let c = color.to_array();
        let clip = self.current_clip();
        if color.a > 0.05 && !s.trim().is_empty() {
            let w = self.text_width_font(f, px, s);
            let r = Rect::new(x, y, w, px).with_z(z);
            let visible = r.x + r.w > clip[0]
                && r.x < clip[0] + clip[2]
                && r.y + r.h > clip[1]
                && r.y < clip[1] + clip[3];
            if visible {
                self.texts.push(TextRec {
                    text: s.to_string(),
                    rect: r,
                    alpha: color.a,
                });
            }
        }
        let mut pen = x;
        for ch in s.chars() {
            match font::glyph(f, ch, px) {
                Some(g) => {
                    if g.size[0] > 0.0 && g.size[1] > 0.0 {
                        let gx = (pen + g.bearing[0]).round();
                        let gy = baseline - g.bearing[1];
                        self.draws.push(DrawRect {
                            min: [gx, gy],
                            max: [gx + g.size[0], gy + g.size[1]],
                            uv_min: g.uv_min,
                            uv_max: g.uv_max,
                            color: c,
                            z,
                            tex: 0,
                            clip,
                            quad: None,
                            uvs: None,
                        });
                    }
                    pen += g.advance;
                }
                None => pen += font::advance(f, px, ch),
            }
        }
    }

    pub fn add<Out>(&mut self, widget: impl Widget<Out = Out>) -> Out {
        let anim = widget.presence();
        let key = widget.presence_key().map(key_hash);
        let ancestor = self.current_clip();
        let start = self.draws.len();
        let out = widget.render(self);
        if let Some(anim) = anim {
            self.anim_range(start, anim, key, ancestor);
        }
        out
    }

    pub fn presence_begin(&mut self, key: &str, anim: Anim) {
        let ancestor = self.current_clip();
        self.presence_stack
            .push((key_hash(key), self.draws.len(), anim, ancestor));
    }

    pub fn presence_end(&mut self) {
        let Some((key, start, anim, ancestor)) = self.presence_stack.pop() else {
            return;
        };
        let dt = self.dt;
        let st = self
            .presence
            .entry(key)
            .or_insert_with(|| Presence::new(anim, true));
        st.present = true;
        st.anim = anim;
        st.ancestor = ancestor;
        st.retarget(1.0);
        st.tween.tick(dt);
        st.draws.clear();
        st.draws.extend_from_slice(&self.draws[start..]);
        if !st.tween.finished() {
            let frame = st.frame();
            let center = st.snapshot_center();
            apply_anim(&mut self.draws[start..], frame, center, ancestor);
        }
    }

    pub fn finish(&mut self) {
        let dt = self.dt;
        let mut replay: Vec<DrawRect> = Vec::new();
        let mut remove: Vec<u64> = Vec::new();
        for (key, st) in self.presence.iter_mut() {
            if st.present {
                st.present = false;
                continue;
            }
            if !st.exitable {
                remove.push(*key);
                continue;
            }
            st.retarget(0.0);
            let t = st.tween.tick(dt);
            if t <= 0.001 {
                remove.push(*key);
                continue;
            }
            let frame = st.frame();
            let center = st.snapshot_center();
            let ancestor = st.ancestor;
            let mut copy = st.draws.clone();
            apply_anim(&mut copy, frame, center, ancestor);
            replay.extend(copy);
        }
        for key in remove {
            self.presence.remove(&key);
        }
        self.draws.extend(replay);
    }

    pub fn label(&mut self, x: f32, y: f32, text: &str, color: Color, size: f32, z: f32) {
        self.add(Text::new(x, y, text).fg(color).font_size(size).z(z));
    }

    pub fn icon(&mut self, r: Rect, sprite: impl SpriteArg) {
        let sprite = sprite.resolve();
        self.sprite(r, sprite, Color::WHITE);
    }

    // Two widget-identity models coexist. `widget_id`/`interact`/`Button` derive the id from the
    // label plus a per-label call-order counter - ergonomic (no manual ids) and stable across
    // frames as long as render order is stable. `interact_id`/`Button::id`/`TextInput` take an
    // explicit `impl Hash` instead - use those in dynamic lists where a same-label widget can
    // appear/disappear conditionally and would otherwise shift the counter of later widgets.
    pub fn widget_id(&mut self, label: &str) -> u64 {
        let lh = key_hash(label);
        let n = self.id_counts.entry(lh).or_insert(0);
        let id = key_hash((lh, *n));
        *n += 1;
        id
    }

    pub fn interact(&mut self, rect: Rect, label: &str) -> Response {
        let id = self.widget_id(label);
        self.interact_hashed(rect, id)
    }

    pub fn interact_id(&mut self, rect: Rect, id: impl Hash) -> Response {
        let id = key_hash(id);
        self.interact_hashed(rect, id)
    }

    pub fn interact_hashed(&mut self, rect: Rect, id: u64) -> Response {
        let hover = self.pointer_over(rect);
        let (entered, exited) = self.hover_edges(id, hover);
        let t = self.anim(id, if hover { 1.0 } else { 0.0 });
        Response {
            hovered: hover,
            clicked: hover && self.clicked,
            pressed: hover && self.pressed,
            hover: t,
            entered,
            exited,
            changed: false,
            rect,
        }
    }

    pub fn fill_aa(&mut self, r: Rect, color: Color) {
        let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.w, r.y + r.h);
        let (ix0, iy0, ix1, iy1) = (x0.ceil(), y0.ceil(), x1.floor(), y1.floor());
        if ix1 > ix0 && iy1 > iy0 {
            self.fill(Rect::new(ix0, iy0, ix1 - ix0, iy1 - iy0).with_z(r.z), color);
        }
        let edge = |cover: f32| color.with_alpha(color.a * cover.clamp(0.0, 1.0));
        let (top, bottom) = (iy0 - y0, y1 - iy1);
        let (left, right) = (ix0 - x0, x1 - ix1);
        let w = (ix1 - ix0).max(0.0);
        let h = (iy1 - iy0).max(0.0);
        if top > 0.0 {
            self.fill(Rect::new(ix0, iy0 - 1.0, w, 1.0).with_z(r.z), edge(top));
        }
        if bottom > 0.0 {
            self.fill(Rect::new(ix0, iy1, w, 1.0).with_z(r.z), edge(bottom));
        }
        if left > 0.0 {
            self.fill(Rect::new(ix0 - 1.0, iy0, 1.0, h).with_z(r.z), edge(left));
        }
        if right > 0.0 {
            self.fill(Rect::new(ix1, iy0, 1.0, h).with_z(r.z), edge(right));
        }
    }

    pub fn corners(&mut self, r: Rect, len: f32, thick: f32, color: Color) {
        let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.w, r.y + r.h);
        for (x, y, dx, dy) in [
            (x0, y0, 1.0, 1.0),
            (x1, y0, -1.0, 1.0),
            (x0, y1, 1.0, -1.0),
            (x1, y1, -1.0, -1.0),
        ] {
            let hx = if dx > 0.0 { x } else { x - len };
            let vy = if dy > 0.0 { y } else { y - len };
            let hy = if dy > 0.0 { y } else { y - thick };
            let vx = if dx > 0.0 { x } else { x - thick };
            self.fill_aa(Rect::new(hx, hy, len, thick).with_z(r.z), color);
            self.fill_aa(
                Rect::new(
                    vx,
                    vy + if dy > 0.0 { thick } else { 0.0 },
                    thick,
                    len - thick,
                )
                .with_z(r.z),
                color,
            );
        }
    }

    pub fn clicked_outside(&self, rect: Rect) -> bool {
        self.clicked && !rect.contains(self.cursor)
    }

    pub fn cursor(&self) -> (f32, f32) {
        self.cursor
    }

    pub fn pointer_down(&self) -> bool {
        self.pressed
    }

    pub fn sprite_button(
        &mut self,
        rect: Rect,
        id: &str,
        sprite: impl SpriteArg,
        tint: Color,
    ) -> Response {
        let sprite = sprite.resolve();
        let resp = self.interact(rect, id);
        let bg = Color::GRAY_900
            .with_alpha(0.85)
            .lerp(Color::SLATE_700, resp.hover());
        self.fill(rect, bg);
        let inner = rect.with_padding(8.0);
        if resp.hover() > 0.01 {
            let outline = Color::WHITE.with_alpha(resp.hover());
            self.sprite_outline(inner, sprite, outline, 3.0);
        }
        self.sprite(inner, sprite, tint);
        resp
    }
}

pub fn key_hash(key: impl Hash) -> u64 {
    let mut h = FxHasher::default();
    key.hash(&mut h);
    h.finish()
}

#[derive(Default)]
struct FxHasher(u64);

impl FxHasher {
    fn mix(&mut self, word: u64) {
        const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for FxHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        let (chunks, rest) = bytes.as_chunks::<8>();
        for c in chunks {
            self.mix(u64::from_le_bytes(*c));
        }
        let mut tail = 0u64;
        for (i, &b) in rest.iter().enumerate() {
            tail |= (b as u64) << (i * 8);
        }
        if !rest.is_empty() {
            self.mix(tail);
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.mix(n);
    }
    fn write_usize(&mut self, n: usize) {
        self.mix(n as u64);
    }
}

pub fn clip_half_plane(
    pts: &[(f32, f32, f32, f32)],
    normal: (f32, f32),
    limit: f32,
) -> Vec<(f32, f32, f32, f32)> {
    let side = |p: &(f32, f32, f32, f32)| p.0 * normal.0 + p.1 * normal.1 - limit;
    let mut out = Vec::new();
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        let (da, db) = (side(&a), side(&b));
        if da <= 0.0 {
            out.push(a);
        }
        if (da <= 0.0) != (db <= 0.0) {
            let t = da / (da - db);
            out.push((
                a.0 + (b.0 - a.0) * t,
                a.1 + (b.1 - a.1) * t,
                a.2 + (b.2 - a.2) * t,
                a.3 + (b.3 - a.3) * t,
            ));
        }
    }
    out
}

#[cfg(test)]
mod field_tests {
    use super::*;

    fn frame(
        ui: &mut UI,
        value: &mut String,
        at: (f32, f32),
        press: bool,
        edit: TextEdit,
    ) -> FieldResponse {
        ui.begin(
            800.0,
            600.0,
            0.016,
            Pointer {
                pos: at,
                pressed: press,
                held: press,
                released: false,
                right: false,
                wheel: 0.0,
            },
        );
        let r = ui.text_field("f", Rect::new(10.0, 10.0, 300.0, 28.0), value, &edit);
        ui.finish();
        r
    }

    #[test]
    fn click_then_edit_and_submit() {
        let mut ui = UI::new();
        let mut v = String::from("[[\"anvil\", 2], [\"fusion_forge\", 1]]");
        frame(&mut ui, &mut v, (20.0, 20.0), true, TextEdit::default());
        frame(
            &mut ui,
            &mut v,
            (20.0, 20.0),
            false,
            TextEdit {
                select_all: true,
                ..Default::default()
            },
        );
        frame(
            &mut ui,
            &mut v,
            (20.0, 20.0),
            false,
            TextEdit {
                typed: "[[\"fusion_forge\", 1]]".into(),
                ..Default::default()
            },
        );
        assert_eq!(v, "[[\"fusion_forge\", 1]]");
        frame(
            &mut ui,
            &mut v,
            (20.0, 20.0),
            false,
            TextEdit {
                home: true,
                ..Default::default()
            },
        );
        frame(
            &mut ui,
            &mut v,
            (20.0, 20.0),
            false,
            TextEdit {
                delete: true,
                ..Default::default()
            },
        );
        assert_eq!(v, "[\"fusion_forge\", 1]]");
        frame(
            &mut ui,
            &mut v,
            (20.0, 20.0),
            false,
            TextEdit {
                end: true,
                ..Default::default()
            },
        );
        frame(
            &mut ui,
            &mut v,
            (20.0, 20.0),
            false,
            TextEdit {
                backspace: true,
                ..Default::default()
            },
        );
        assert_eq!(v, "[\"fusion_forge\", 1]");
        let r = frame(
            &mut ui,
            &mut v,
            (20.0, 20.0),
            false,
            TextEdit {
                enter: true,
                ..Default::default()
            },
        );
        assert!(r.submitted);
    }

    #[test]
    fn blur_submits_only_when_changed() {
        let mut ui = UI::new();
        let mut v = String::from("abc");
        frame(&mut ui, &mut v, (20.0, 20.0), true, TextEdit::default());
        frame(&mut ui, &mut v, (20.0, 20.0), false, TextEdit::default());
        assert!(!frame(&mut ui, &mut v, (500.0, 500.0), true, TextEdit::default()).submitted);
        frame(&mut ui, &mut v, (20.0, 20.0), true, TextEdit::default());
        frame(
            &mut ui,
            &mut v,
            (20.0, 20.0),
            false,
            TextEdit {
                typed: "d".into(),
                ..Default::default()
            },
        );
        assert!(frame(&mut ui, &mut v, (500.0, 500.0), true, TextEdit::default()).submitted);
    }
}
