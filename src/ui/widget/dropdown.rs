use crate::animation::Ease;
use crate::ui::style::Transition;
use std::hash::Hash;

use crate::ui::immediate::{TextEdit, UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::widget::Widget;

const ROW_HOVER: Transition = Transition::new(0.05, Ease::OutQuad).exit(0.14, Ease::OutCubic);
const HEAD_HOVER: Transition = Transition::new(0.1, Ease::OutQuad).exit(0.18, Ease::InOutQuad);

const MAX_ROWS: usize = 8;
const DECOR_W: f32 = 56.0;

pub type Decorate<'a> = &'a dyn Fn(&mut UI, Rect, &str);

fn decor_rect(row: Rect, right: f32) -> Rect {
    Rect::new(
        row.x + row.w - right - DECOR_W,
        row.y + 3.0,
        DECOR_W,
        row.h - 6.0,
    )
}

#[derive(Default)]
pub struct DropState {
    pub open: bool,
    pub search: String,
}

enum Mark {
    None,
    Current,
    Checked(usize),
    Unchecked,
    Disabled,
}

fn head(ui: &mut UI, r: Rect, id: u64, text: &str, state: &mut DropState) -> bool {
    let t = ui.theme;
    let size = t.font_size * 0.9;
    let resp = ui.interact_curved(r, (id, "head"), HEAD_HOVER);
    ui.surface(r, t.accent.lerp(t.button_fg, resp.hover() * 0.3));
    ui.surface(r.with_padding(1.0).inner().with_z(r.z + 0.001), t.field_bg);
    ui.label(
        r.x + 8.0,
        r.y + (r.h - size) / 2.0,
        text,
        t.label,
        size,
        r.z + 0.002,
    );
    let arrow = if state.open { "▴" } else { "▾" };
    ui.label(
        r.x + r.w - 18.0,
        r.y + (r.h - size) / 2.0,
        arrow,
        t.label,
        size,
        r.z + 0.002,
    );
    if resp.clicked {
        state.open = !state.open;
        state.search.clear();
        return true;
    }
    false
}

fn popup(
    ui: &mut UI,
    (r, id): (Rect, u64),
    options: &[&str],
    (state, edit): (&mut DropState, &TextEdit),
    decorate: Option<Decorate>,
    mark: impl Fn(&str) -> Mark,
) -> Option<String> {
    let t = ui.theme;
    let size = t.font_size * 0.9;
    let z = r.z + 1.0;
    let search = Rect::new(r.x, r.y + r.h, r.w, r.h).with_z(z);
    ui.surface(search, t.field_bg);
    ui.text_field((id, "search"), search, &mut state.search, edit);
    if state.search.is_empty() {
        ui.label(
            search.x + 8.0,
            search.y + (r.h - size) / 2.0,
            "search…",
            t.label.with_alpha(0.4),
            size,
            z + 0.01,
        );
    }
    let needle = state.search.to_lowercase();
    let shown: Vec<&str> = options
        .iter()
        .copied()
        .filter(|o| needle.is_empty() || o.to_lowercase().contains(&needle))
        .collect();
    let view = Rect::new(
        r.x,
        search.y + r.h,
        r.w,
        r.h * shown.len().clamp(1, MAX_ROWS) as f32,
    )
    .with_z(z);
    let offset = ui.scroll_area((id, "scroll"), view, r.h * shown.len() as f32, t.accent);
    ui.surface(view, t.field_bg);
    let mut picked = None;
    ui.push_clip(view);
    for (n, opt) in shown.iter().enumerate() {
        let row =
            Rect::new(r.x, view.y + r.h * n as f32 - offset, r.w - 10.0, r.h).with_z(z + 0.001);
        if row.y + row.h < view.y || row.y > view.y + view.h {
            continue;
        }
        let resp = ui.interact_curved(row.intersect(view).with_z(z + 0.001), (id, *opt), ROW_HOVER);
        let m = mark(opt);
        let bg = if matches!(m, Mark::Current) {
            t.accent
        } else {
            t.field_bg
        };
        ui.surface(row, bg.lerp(t.button_fg, resp.hover() * 0.3));
        let mut tx = row.x + 8.0;
        if matches!(m, Mark::Checked(_) | Mark::Unchecked | Mark::Disabled) {
            let bx =
                Rect::new(row.x + 6.0, row.y + (row.h - 14.0) / 2.0, 14.0, 14.0).with_z(z + 0.002);
            ui.surface(bx, t.accent);
            ui.surface(bx.with_padding(2.0).inner().with_z(z + 0.003), t.field_bg);
            if let Mark::Checked(p) = m {
                ui.surface(bx.with_padding(4.0).inner().with_z(z + 0.004), t.mark);
                ui.label(
                    row.x + row.w - 22.0,
                    row.y + (row.h - size) / 2.0,
                    &(p + 1).to_string(),
                    t.label,
                    size,
                    z + 0.003,
                );
            }
            tx = row.x + 28.0;
        }
        let fg = if matches!(m, Mark::Disabled) {
            t.label.with_alpha(0.4)
        } else {
            t.label
        };
        ui.label(tx, row.y + (row.h - size) / 2.0, opt, fg, size, z + 0.003);
        if let Some(f) = decorate {
            f(ui, decor_rect(row, 28.0).with_z(z + 0.003), opt);
        }
        if resp.clicked {
            picked = Some(opt.to_string());
        }
    }
    ui.pop_clip();
    ui.block_pointer(Rect::new(r.x, r.y, r.w, view.y + view.h - r.y).with_z(z));
    let area = Rect::new(r.x, r.y, r.w, view.y + view.h - r.y);
    if ui.clicked_outside(area) {
        state.open = false;
    }
    picked
}

pub struct Dropdown<'a> {
    rect: Rect,
    options: &'a [&'a str],
    selected: &'a mut String,
    state: &'a mut DropState,
    edit: &'a TextEdit,
    decorate: Option<Decorate<'a>>,
    id: u64,
}

impl<'a> Dropdown<'a> {
    pub fn new(
        rect: Rect,
        options: &'a [&'a str],
        selected: &'a mut String,
        state: &'a mut DropState,
        edit: &'a TextEdit,
        id: impl Hash,
    ) -> Self {
        Dropdown {
            rect,
            options,
            selected,
            state,
            edit,
            decorate: None,
            id: key_hash(id),
        }
    }

    pub fn decorate(mut self, f: Decorate<'a>) -> Self {
        self.decorate = Some(f);
        self
    }
}

impl Widget for Dropdown<'_> {
    type Out = bool;

    fn render(self, ui: &mut UI) -> bool {
        let text = self.selected.clone();
        if let Some(f) = self.decorate {
            f(
                ui,
                decor_rect(self.rect, 24.0).with_z(self.rect.z + 0.003),
                &text,
            );
        }
        if head(ui, self.rect, self.id, &text, self.state) || !self.state.open {
            return false;
        }
        let current = self.selected.clone();
        let picked = popup(
            ui,
            (self.rect, self.id),
            self.options,
            (self.state, self.edit),
            self.decorate,
            |o| {
                if o == current {
                    Mark::Current
                } else {
                    Mark::None
                }
            },
        );
        match picked {
            Some(p) => {
                self.state.open = false;
                let changed = p != *self.selected;
                *self.selected = p;
                changed
            }
            None => false,
        }
    }
}

pub struct MultiSelect<'a> {
    rect: Rect,
    options: &'a [&'a str],
    selected: &'a mut Vec<String>,
    state: &'a mut DropState,
    edit: &'a TextEdit,
    max: usize,
    id: u64,
}

impl<'a> MultiSelect<'a> {
    pub fn new(
        rect: Rect,
        options: &'a [&'a str],
        selected: &'a mut Vec<String>,
        state: &'a mut DropState,
        edit: &'a TextEdit,
        id: impl Hash,
    ) -> Self {
        MultiSelect {
            rect,
            options,
            selected,
            state,
            edit,
            max: usize::MAX,
            id: key_hash(id),
        }
    }

    pub fn max(mut self, max: usize) -> Self {
        self.max = max;
        self
    }
}

impl Widget for MultiSelect<'_> {
    type Out = bool;

    fn render(self, ui: &mut UI) -> bool {
        let summary = self.selected.join(", ");
        if head(ui, self.rect, self.id, &summary, self.state) || !self.state.open {
            return false;
        }
        let chosen = self.selected.clone();
        let full = chosen.len() >= self.max;
        let picked = popup(
            ui,
            (self.rect, self.id),
            self.options,
            (self.state, self.edit),
            None,
            |o| match chosen.iter().position(|s| s == o) {
                Some(p) => Mark::Checked(p),
                None if full => Mark::Disabled,
                None => Mark::Unchecked,
            },
        );
        let Some(p) = picked else {
            return false;
        };
        match self.selected.iter().position(|s| *s == p) {
            Some(n) => {
                self.selected.remove(n);
                true
            }
            None if !full => {
                self.selected.push(p);
                true
            }
            None => false,
        }
    }
}
