use std::collections::HashSet;

use glam::Vec3;

use tungstengine::color::Color;
use tungstengine::ui::prelude::*;

use crate::scene::menu::{flat_button, list_item};

pub const LINE_H: f32 = 18.0;
pub const EDIT_H: f32 = 24.0;
pub const FONT: f32 = 13.0;
pub const KEY_W: f32 = 72.0;
pub const OVERRIDE_COLOR: Color = Color::AMBER_300;

pub struct Form<'a> {
    pub ui: &'a mut UI,
    pub body: Rect,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub hidden_gizmos: HashSet<String>,
    pub open: HashSet<u64>,
    buf: Option<(u64, String)>,
    focused: bool,
    multiline: bool,
    locks: HashSet<u64>,
    overrides: Option<HashSet<String>>,
    defaults: Option<HashSet<String>>,
    reverted: Option<String>,
}

impl<'a> Form<'a> {
    pub fn new(
        ui: &'a mut UI,
        body: Rect,
        offset: f32,
        buf: Option<(u64, String)>,
        locks: HashSet<u64>,
    ) -> Self {
        Form {
            ui,
            body,
            x: body.x + 10.0,
            y: body.y + 2.0 - offset,
            w: body.w - 20.0,
            hidden_gizmos: HashSet::new(),
            open: HashSet::new(),
            buf,
            focused: false,
            multiline: false,
            locks,
            overrides: None,
            defaults: None,
            reverted: None,
        }
    }

    pub fn finish(self) -> (Option<(u64, String)>, bool, HashSet<u64>) {
        (self.buf, self.focused, self.locks)
    }

    pub fn multiline_focused(&self) -> bool {
        self.multiline
    }

    pub fn text_area(&mut self, slot: u64, label: &str, v: &mut String) {
        if self.visible(LINE_H) {
            self.label(label, self.y + 2.0);
        }
        self.y += LINE_H;
        let id = ("inspector-area", slot);
        let key = slot * 64 + 63;
        let was_focused = self.ui.is_focused(id);
        let mut text = match &self.buf {
            Some((k, b)) if was_focused && *k == key => b.clone(),
            _ => v.clone(),
        };
        let w = self.w - 12.0;
        let h = TextArea::height(&text, w, FONT, 2);
        if self.visible(h) || was_focused {
            let r = Rect::new(self.x + 12.0, self.y, w, h);
            self.ui.add(TextArea::new(r, id, &mut text).font_size(FONT));
        }
        self.y += h + 4.0;
        if self.ui.is_focused(id) {
            self.focused = true;
            self.multiline = true;
            self.buf = Some((key, text.clone()));
            *v = text;
        }
    }

    pub fn visible(&self, h: f32) -> bool {
        self.y + h >= self.body.y && self.y <= self.body.y + self.body.h
    }

    pub fn gap(&mut self) {
        self.y += LINE_H * 0.5;
    }

    pub fn section_action(&mut self, name: &str, tag: Option<&str>, action: Option<&str>) -> bool {
        self.header(name, tag, action, false)
    }

    pub fn component_header(
        &mut self,
        name: &str,
        tag: Option<&str>,
        action: Option<&str>,
    ) -> bool {
        self.header(name, tag, action, true)
    }

    fn header(&mut self, name: &str, tag: Option<&str>, action: Option<&str>, gizmo: bool) -> bool {
        let mut clicked = false;
        if self.visible(LINE_H) {
            let r = Rect::new(self.body.x + 4.0, self.y, self.body.w - 8.0, LINE_H - 1.0);
            self.ui.fill(r, Color::SLATE_800);
            let mut text_x = self.x;
            if gizmo {
                let on = !self.hidden_gizmos.contains(name);
                let b = Rect::new(r.x + 4.0, r.y + 3.0, 11.0, 11.0);
                if self.checkbox(b, ("gizmo", name.len() as u64 ^ self.y as u64), on)
                    && !self.hidden_gizmos.remove(name)
                {
                    self.hidden_gizmos.insert(name.to_string());
                }
                text_x = b.x + b.w + 6.0;
            }
            self.ui.add(
                Text::new(text_x, self.y + 2.0, name)
                    .font_size(FONT)
                    .fg(Color::SKY_300),
            );
            let mut right = r.x + r.w - 6.0;
            if let Some(label) = action {
                let w = self.ui.text_width(11.0, label) + 12.0;
                right -= w;
                let b = Rect::new(right, r.y + 1.0, w, r.h - 2.0);
                clicked = flat_button(
                    self.ui,
                    b,
                    ("section-action", name, self.y as i32),
                    label,
                    false,
                );
                right -= 6.0;
            }
            if let Some(tag) = tag {
                let tw = self.ui.text_width(11.0, tag);
                self.ui.add(
                    Text::new(right - tw - 2.0, self.y + 3.0, tag)
                        .font_size(11.0)
                        .fg(Color::GRAY_500),
                );
            }
        }
        self.y += LINE_H;
        clicked
    }

    pub fn button(&mut self, label: &str) -> bool {
        let h = EDIT_H;
        let mut clicked = false;
        if self.visible(h) {
            let r = Rect::new(self.x, self.y + 2.0, self.w, h - 4.0);
            self.ui.fill(r, Color::SLATE_800);
            clicked = flat_button(self.ui, r, ("form-button", label), label, false);
        }
        self.y += h;
        clicked
    }

    pub fn set_overrides(&mut self, keys: Option<HashSet<String>>) {
        self.overrides = keys;
    }

    pub fn set_defaults(&mut self, keys: Option<HashSet<String>>) {
        self.defaults = keys;
    }

    pub fn take_reverted(&mut self) -> Option<String> {
        self.reverted.take()
    }

    pub fn label(&mut self, key: &str, y: f32) {
        let overridden = self.overrides.as_ref().is_some_and(|o| o.contains(key));
        if overridden {
            self.ui.fill(
                Rect::new(self.body.x + 4.0, y - 2.0, 2.0, LINE_H),
                OVERRIDE_COLOR,
            );
        }
        self.ui.add(
            Text::new(self.x + 6.0, y, key)
                .font_size(FONT)
                .fg(if overridden {
                    OVERRIDE_COLOR
                } else if self.defaults.as_ref().is_some_and(|d| d.contains(key)) {
                    Color::GRAY_600
                } else {
                    Color::GRAY_400
                })
                .truncate(KEY_W - 30.0),
        );
        if overridden {
            let r = Rect::new(self.x + KEY_W - 32.0, y - 2.0, 16.0, 18.0);
            if flat_button(self.ui, r, ("revert", key, y as i32), "↺", false) {
                self.reverted = Some(key.to_string());
            }
        }
    }

    pub fn text(&mut self, slot: u64, label: &str, v: &mut String) {
        let h = EDIT_H + 4.0;
        if self.visible(h) {
            self.label(label, self.y + 6.0);
            let r = Rect::new(self.x + KEY_W, self.y + 2.0, self.w - KEY_W, EDIT_H);
            if let Some(t) = self.input(slot * 3, r, v.clone()) {
                *v = t;
            }
        }
        self.y += h;
    }

    pub fn strings(&mut self, slot: u64, label: &str, v: &mut Vec<String>) {
        if self.visible(LINE_H) {
            self.label(label, self.y + 2.0);
        }
        self.y += LINE_H;
        let mut remove = None;
        for (i, item) in v.iter_mut().enumerate() {
            let h = EDIT_H + 2.0;
            if self.visible(h) {
                let r = Rect::new(self.x + 12.0, self.y + 1.0, self.w - 38.0, EDIT_H);
                if let Some(t) = self.input(slot * 64 + i as u64, r, item.clone()) {
                    *item = t;
                }
                let b = Rect::new(self.x + self.w - 22.0, self.y + 3.0, 20.0, EDIT_H - 4.0);
                if flat_button(self.ui, b, ("list-remove", slot, i as u64), "×", false) {
                    remove = Some(i);
                }
            }
            self.y += h;
        }
        if let Some(i) = remove {
            v.remove(i);
        }
        let h = EDIT_H;
        if self.visible(h) {
            let r = Rect::new(self.x + 12.0, self.y + 1.0, 80.0, h - 4.0);
            if list_item(self.ui, r, ("list-add", slot), "+ item", false) {
                v.push(String::new());
            }
        }
        self.y += h;
    }

    pub fn row(&mut self, key: &str, value: &str) {
        self.row_colored(key, value, Color::GRAY_100);
    }

    pub fn row_colored(&mut self, key: &str, value: &str, color: Color) {
        if self.visible(LINE_H) {
            let y = self.y + 2.0;
            self.label(key, y);
            self.ui.add(
                Text::new(self.x + KEY_W + 10.0, y, value)
                    .font_size(FONT)
                    .fg(color)
                    .truncate(self.w - KEY_W - 14.0),
            );
        }
        self.y += LINE_H;
    }

    pub fn input(&mut self, slot: u64, r: Rect, shown: String) -> Option<String> {
        let id = ("inspector-field", slot);
        let was_focused = self.ui.is_focused(id);
        let mut text = match &self.buf {
            Some((k, b)) if was_focused && *k == slot => b.clone(),
            _ => shown,
        };
        self.ui
            .add(TextInput::new(r, id, &mut text).font_size(FONT));
        if !self.ui.is_focused(id) {
            return None;
        }
        self.focused = true;
        self.buf = Some((slot, text.clone()));
        Some(text)
    }

    pub fn vec3(&mut self, slot: u64, label: &str, v: &mut Vec3) {
        self.vec3_row(slot, label, v, false);
    }

    pub fn vec3_lockable(&mut self, slot: u64, label: &str, v: &mut Vec3) {
        self.vec3_row(slot, label, v, true);
    }

    fn checkbox(&mut self, r: Rect, id: (&str, u64), on: bool) -> bool {
        let hit = self.ui.interact_id(r, id);
        self.ui.fill(
            r,
            if hit.hovered() {
                Color::GRAY_500
            } else {
                Color::GRAY_600
            },
        );
        let inner = Rect::new(r.x + 2.0, r.y + 2.0, r.w - 4.0, r.h - 4.0);
        self.ui
            .fill(inner, if on { Color::SKY_400 } else { Color::GRAY_800 });
        hit.clicked()
    }

    fn lock_box(&mut self, slot: u64, y: f32) -> bool {
        let r = Rect::new(self.x + KEY_W - 14.0, y + 5.0, 13.0, 13.0);
        let on = self.locks.contains(&slot);
        if self.checkbox(r, ("lock", slot), on) && !self.locks.remove(&slot) {
            self.locks.insert(slot);
        }
        self.locks.contains(&slot)
    }

    fn vec3_row(&mut self, slot: u64, label: &str, v: &mut Vec3, lockable: bool) {
        let h = EDIT_H + 4.0;
        if self.visible(h) {
            self.label(label, self.y + 6.0);
            let locked = lockable && self.lock_box(slot, self.y);
            let cell = (self.w - KEY_W) / 3.0;
            for (axis, name) in ["X", "Y", "Z"].iter().enumerate() {
                let cx = self.x + KEY_W + axis as f32 * cell;
                self.ui.add(
                    Text::new(cx, self.y + 6.0, name)
                        .font_size(11.0)
                        .fg(Color::GRAY_500),
                );
                let r = Rect::new(cx + 10.0, self.y + 2.0, cell - 14.0, EDIT_H);
                if let Some(t) = self.input(slot * 3 + axis as u64, r, fmt_num(v[axis]))
                    && let Some(n) = parse_num(&t)
                    && n != v[axis]
                {
                    if locked {
                        *v = Vec3::splat(n);
                    } else {
                        v[axis] = n;
                    }
                }
            }
        }
        self.y += h;
    }

    fn value_rect(&self) -> Rect {
        Rect::new(
            self.x + KEY_W + 10.0,
            self.y + 2.0,
            (self.w - KEY_W) / 3.0 - 4.0,
            EDIT_H,
        )
    }

    pub fn number(&mut self, slot: u64, label: &str, v: &mut f32) {
        let h = EDIT_H + 4.0;
        if self.visible(h) {
            self.label(label, self.y + 6.0);
            let r = self.value_rect();
            if let Some(t) = self.input(slot * 3, r, fmt_num(*v))
                && let Some(n) = parse_num(&t)
            {
                *v = n;
            }
        }
        self.y += h;
    }

    pub fn toggle(&mut self, slot: u64, label: &str, v: &mut bool) {
        let h = EDIT_H + 4.0;
        if self.visible(h) {
            self.label(label, self.y + 6.0);
            let r = Rect::new(self.x + KEY_W + 10.0, self.y + 6.0, 15.0, 15.0);
            if self.checkbox(r, ("toggle", slot), *v) {
                *v = !*v;
            }
        }
        self.y += h;
    }

    pub fn indent(&mut self, dx: f32) {
        self.x += dx;
        self.w -= dx;
    }

    pub fn fold_row(&mut self, slot: u64, title: &str, detail: &str) -> (bool, bool) {
        let h = EDIT_H;
        let mut open = self.open.contains(&slot);
        let mut remove = false;
        if self.visible(h) {
            let r = Rect::new(self.x, self.y + 1.0, self.w, h - 2.0);
            let hit = self.ui.interact_id(r, ("fold", slot));
            self.ui.fill(
                r,
                if hit.hovered() {
                    Color::SLATE_700
                } else {
                    Color::SLATE_800
                },
            );
            let (cx, cy) = (r.x + 9.0, r.y + r.h * 0.5);
            if open {
                self.ui.triangle(
                    (cx - 4.0, cy - 2.0),
                    (cx + 4.0, cy - 2.0),
                    (cx, cy + 3.0),
                    Color::GRAY_400,
                );
            } else {
                self.ui.triangle(
                    (cx - 2.0, cy - 4.0),
                    (cx + 3.0, cy),
                    (cx - 2.0, cy + 4.0),
                    Color::GRAY_400,
                );
            }
            self.ui.add(
                Text::new(r.x + 20.0, r.y + 5.0, title)
                    .font_size(FONT)
                    .fg(Color::GRAY_100)
                    .truncate(r.w * 0.5),
            );
            let dw = self.ui.text_width(11.0, detail);
            self.ui.add(
                Text::new(r.x + r.w - dw - 30.0, r.y + 6.0, detail)
                    .font_size(11.0)
                    .fg(Color::GRAY_500),
            );
            let b = Rect::new(r.x + r.w - 22.0, r.y + 2.0, 20.0, r.h - 4.0);
            let on_button = b.contains(self.ui.cursor);
            if flat_button(self.ui, b, ("fold-remove", slot), "×", false) {
                remove = true;
            }
            if hit.clicked() && !on_button {
                open = !open;
                if open {
                    self.open.insert(slot);
                } else {
                    self.open.remove(&slot);
                }
            }
        }
        self.y += h;
        (open, remove)
    }

    pub fn add_button(&mut self, slot: u64, label: &str) -> bool {
        let h = EDIT_H;
        let mut clicked = false;
        if self.visible(h) {
            let r = Rect::new(self.x, self.y + 2.0, self.w, h - 4.0);
            self.ui.fill(r, Color::SLATE_800.with_alpha(0.6));
            clicked = list_item(self.ui, r, ("add-button", slot), label, false);
        }
        self.y += h;
        clicked
    }

    pub fn choice(
        &mut self,
        slot: u64,
        label: &str,
        current: &str,
        options: &[&str],
    ) -> Option<String> {
        let h = EDIT_H + 4.0;
        let mut picked = None;
        let open = self.open.contains(&slot);
        if self.visible(h) {
            self.label(label, self.y + 6.0);
            let r = Rect::new(self.x + KEY_W, self.y + 2.0, self.w - KEY_W, EDIT_H);
            self.ui.fill(r, Color::GRAY_800);
            if list_item(self.ui, r, ("choice", slot), current, open) {
                if open {
                    self.open.remove(&slot);
                } else {
                    self.open.insert(slot);
                }
            }
            let (cx, cy) = (r.x + r.w - 12.0, r.y + r.h * 0.5);
            self.ui.triangle(
                (cx - 4.0, cy - 2.0),
                (cx + 4.0, cy - 2.0),
                (cx, cy + 3.0),
                Color::GRAY_400,
            );
        }
        self.y += h;
        if open {
            for (i, opt) in options.iter().enumerate() {
                if self.visible(EDIT_H) {
                    let r = Rect::new(self.x + KEY_W, self.y, self.w - KEY_W, EDIT_H - 2.0);
                    self.ui.fill(r, Color::SLATE_800);
                    if list_item(
                        self.ui,
                        r,
                        ("choice-opt", slot, i as u64),
                        opt,
                        *opt == current,
                    ) {
                        picked = Some(opt.to_string());
                        self.open.remove(&slot);
                    }
                }
                self.y += EDIT_H;
            }
        }
        picked
    }
}

pub fn fmt_num(v: f32) -> String {
    let r = (v * 1000.0).round() / 1000.0;
    if r == 0.0 { "0".into() } else { r.to_string() }
}

fn parse_num(t: &str) -> Option<f32> {
    t.trim().replace(',', ".").parse().ok()
}
