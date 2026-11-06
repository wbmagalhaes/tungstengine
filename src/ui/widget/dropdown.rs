use crate::animation::Ease;
use crate::ui::animation::Anim;
use crate::ui::style::Styled;
use crate::ui::style::Transition;
use crate::ui::widget::Group;
use crate::ui::widget::TextInput;
use crate::ui::widget::checkbox::check_box;
use std::hash::Hash;

use crate::ui::immediate::{FIELD_PAD, LineIcon, UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::widget::Widget;

const ROW_HOVER: Transition = Transition::new(0.05, Ease::OutQuad).exit(0.14, Ease::OutCubic);
const DROP_GAP: f32 = 4.0;
const LEADING_W: f32 = 22.0;
const LEADING_GAP: f32 = 8.0;
const LEADING_INSET: f32 = 8.0;
const HINT_ALPHA: f32 = 0.45;
const CLEAR_SIZE: f32 = 18.0;
const CLEAR_RIGHT: f32 = 26.0;
const CLEAR_ICON: f32 = 7.0;
const CLEAR_TINT: f32 = 0.3;
const POPOVER_ENTER: f32 = 0.16;
const POPOVER_EXIT: f32 = 0.11;
const POPOVER_SHIFT: f32 = -8.0;
const ARROW_SIZE: f32 = 9.0;
const SCROLL_GUTTER: f32 = 10.0;
const ROW_HOVER_TINT: f32 = 0.08;
const CHECK_SIZE: f32 = 11.0;
const HEAD_HOVER: Transition = Transition::new(0.1, Ease::OutQuad).exit(0.18, Ease::InOutQuad);

const MAX_ROWS: usize = 8;
const DECOR_W: f32 = 56.0;

pub type Decorate<'a> = &'a dyn Fn(&mut UI, Rect, &str);

fn leading_rect(row: Rect, z: f32) -> Rect {
    let height = row.h - LEADING_INSET * 2.0;
    Rect::new(row.x + FIELD_PAD, row.y + LEADING_INSET, LEADING_W, height).with_z(z)
}

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
    pub search_placeholder: Option<String>,
    pub searchable: bool,
}

fn popover_anim() -> Anim {
    let leave = Anim::new(POPOVER_EXIT)
        .ease(Ease::InQuad)
        .fade()
        .rise(POPOVER_SHIFT);
    Anim::new(POPOVER_ENTER)
        .ease(Ease::OutCubic)
        .fade()
        .rise(POPOVER_SHIFT)
        .exit(leave)
}

enum Mark {
    None,
    Current,
    Checked(usize),
    Unchecked,
    Disabled,
}

fn clear_button(ui: &mut UI, r: Rect, id: u64) -> bool {
    let t = ui.theme;
    let size = CLEAR_SIZE;
    let x = r.x + r.w - CLEAR_RIGHT - size;
    let zone = Rect::new(x, r.y + (r.h - size) / 2.0, size, size).with_z(r.z + 0.01);
    let resp = ui.interact_curved(zone, (id, "clear"), HEAD_HOVER);
    if resp.hover() > 0.0 {
        let tint = t.accent.with_alpha(CLEAR_TINT * resp.hover());
        ui.fill_round(zone, tint, size / 2.0);
    }
    let center = (x + size / 2.0, zone.y + size / 2.0);
    ui.icon_line(LineIcon::Cross, center, CLEAR_ICON, t.label, zone.z + 0.001);
    resp.clicked
}

fn head(
    ui: &mut UI,
    r: Rect,
    id: u64,
    (text, hint, leading): (&str, Option<&str>, Option<Decorate>),
    state: &mut DropState,
) -> bool {
    let t = ui.theme;
    let size = t.font_size;
    let resp = ui.interact_curved(r, (id, "head"), HEAD_HOVER);
    ui.field_box(r, state.open || resp.hovered);
    let empty = text.is_empty();
    let (shown, ink) = match hint.filter(|_| empty) {
        Some(hint) => (hint, t.label.with_alpha(HINT_ALPHA)),
        None => (text, t.label),
    };
    let mut text_x = r.x + FIELD_PAD;
    if let Some(draw) = leading.filter(|_| !empty) {
        draw(ui, leading_rect(r, r.z + 0.002), text);
        text_x += LEADING_W + LEADING_GAP;
    }
    ui.label(
        text_x,
        r.y + (r.h - size) / 2.0,
        shown,
        ink,
        size,
        r.z + 0.002,
    );
    let arrow = if state.open {
        LineIcon::ChevronUp
    } else {
        LineIcon::ChevronDown
    };
    let center = (r.x + r.w - 14.0, r.y + r.h / 2.0);
    ui.icon_line(arrow, center, ARROW_SIZE, t.label, r.z + 0.002);
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
    state: &mut DropState,
    (decorate, leading): (Option<Decorate>, Option<Decorate>),
    mark: impl Fn(&str) -> Mark,
) -> Option<String> {
    let t = ui.theme;
    let size = t.font_size;
    let z = r.z + 1.0;
    let needle = state.search.to_lowercase();
    let shown: Vec<&str> = options
        .iter()
        .copied()
        .filter(|o| needle.is_empty() || o.to_lowercase().contains(&needle))
        .collect();
    let hide = !state.searchable;
    let search_h = if hide { 0.0 } else { r.h };
    let stack = search_h + r.h * shown.len().clamp(1, MAX_ROWS) as f32;
    let below = r.y + r.h + DROP_GAP;
    let opens_up = below + stack > ui.height;
    let stack_top = if opens_up {
        r.y - DROP_GAP - stack
    } else {
        below
    };
    let search = Rect::new(r.x, stack_top, r.w, r.h).with_z(z);
    if !hide {
        let hint = state.search_placeholder.as_deref().unwrap_or("Search…");
        let top = t.border_radius;
        let input = TextInput::new(search, (id, "search"), &mut state.search)
            .placeholder(hint)
            .border_radius_corners(top, top, 0.0, 0.0);
        ui.add(input);
    }
    let view = Rect::new(
        r.x,
        if hide { search.y } else { search.y + r.h },
        r.w,
        r.h * shown.len().clamp(1, MAX_ROWS) as f32,
    )
    .with_z(z);
    let offset = ui.scroll_y((id, "scroll"), view, r.h * shown.len() as f32);
    let edge = if hide { t.border_radius } else { 0.0 };
    let bottom = [edge, edge, t.border_radius, t.border_radius];
    ui.fill_corners(view, t.field_bg, bottom, 1.0, t.panel_border);
    let mut picked = None;
    ui.push_clip_corners(view, bottom);
    let scrolls = r.h * shown.len() as f32 > view.h;
    let row_w = r.w - if scrolls { SCROLL_GUTTER } else { 0.0 };
    for (n, opt) in shown.iter().enumerate() {
        let y = view.y + r.h * n as f32 - offset;
        let row = Rect::new(r.x, y, row_w, r.h).with_z(z + 0.001);
        if row.y + row.h < view.y || row.y > view.y + view.h {
            continue;
        }
        let resp = ui.interact_curved(row.intersect(view).with_z(z + 0.001), (id, *opt), ROW_HOVER);
        let m = mark(opt);
        if resp.hover() > 0.0 {
            let soft = t.button_fg.with_alpha(ROW_HOVER_TINT * resp.hover());
            ui.fill(row, soft);
        }
        let current = matches!(m, Mark::Current);
        if current {
            let center = (
                row.x + row.w - FIELD_PAD - CHECK_SIZE / 2.0,
                row.y + row.h / 2.0,
            );
            ui.icon_line(LineIcon::Check, center, CHECK_SIZE, t.accent, row.z + 0.002);
        }
        let mut tx = row.x + FIELD_PAD;
        if matches!(m, Mark::Checked(_) | Mark::Unchecked | Mark::Disabled) {
            let bx =
                Rect::new(row.x + 6.0, row.y + (row.h - 14.0) / 2.0, 14.0, 14.0).with_z(z + 0.002);
            let checked = matches!(m, Mark::Checked(_));
            check_box(ui, bx, (id, "check", *opt), checked, t.accent);
            if let Mark::Checked(p) = m {
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
        let fg = match m {
            Mark::Disabled => t.label.with_alpha(0.4),
            Mark::Current => t.button_fg,
            Mark::Checked(_) => t.button_fg,
            _ => t.label,
        };
        if let Some(draw) = leading {
            draw(ui, leading_rect(row, z + 0.003), opt);
            tx += LEADING_W + LEADING_GAP;
        }
        ui.label(tx, row.y + (row.h - size) / 2.0, opt, fg, size, z + 0.003);
        if let Some(f) = decorate {
            f(ui, decor_rect(row, 28.0).with_z(z + 0.003), opt);
        }
        if resp.clicked {
            picked = Some(opt.to_string());
        }
    }
    ui.pop_clip();
    ui.block_pointer(Rect::new(r.x, search.y, r.w, view.y + view.h - search.y).with_z(z));
    let area_top = r.y.min(search.y);
    let area_bottom = (r.y + r.h).max(view.y + view.h);
    let area = Rect::new(r.x, area_top, r.w, area_bottom - area_top);
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
    decorate: Option<Decorate<'a>>,
    leading: Option<Decorate<'a>>,
    placeholder: Option<&'a str>,
    clearable: bool,
    id: u64,
}

impl<'a> Dropdown<'a> {
    pub fn new(
        rect: Rect,
        options: &'a [&'a str],
        selected: &'a mut String,
        state: &'a mut DropState,
        id: impl Hash,
    ) -> Self {
        Dropdown {
            rect,
            options,
            selected,
            state,
            decorate: None,
            leading: None,
            placeholder: None,
            clearable: false,
            id: key_hash(id),
        }
    }

    pub fn clearable(mut self) -> Self {
        self.clearable = true;
        self
    }

    pub fn placeholder(mut self, text: &'a str) -> Self {
        self.placeholder = Some(text);
        self
    }

    pub fn leading(mut self, f: Decorate<'a>) -> Self {
        self.leading = Some(f);
        self
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
        if self.clearable && !self.selected.is_empty() && clear_button(ui, self.rect, self.id) {
            self.selected.clear();
            self.state.open = false;
            return true;
        }
        if head(
            ui,
            self.rect,
            self.id,
            (&text, self.placeholder, self.leading),
            self.state,
        ) || !self.state.open
        {
            return false;
        }
        let current = self.selected.clone();
        let key = format!("dropdown-{}", self.id);
        let picked = ui.add(
            Group::new(|ui: &mut UI| {
                popup(
                    ui,
                    (self.rect, self.id),
                    self.options,
                    self.state,
                    (self.decorate, self.leading),
                    |o| {
                        if o == current {
                            Mark::Current
                        } else {
                            Mark::None
                        }
                    },
                )
            })
            .anim(popover_anim())
            .key(&key),
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
    max: usize,
    placeholder: Option<&'a str>,
    clearable: bool,
    id: u64,
}

impl<'a> MultiSelect<'a> {
    pub fn new(
        rect: Rect,
        options: &'a [&'a str],
        selected: &'a mut Vec<String>,
        state: &'a mut DropState,
        id: impl Hash,
    ) -> Self {
        MultiSelect {
            rect,
            options,
            selected,
            state,
            max: usize::MAX,
            placeholder: None,
            clearable: false,
            id: key_hash(id),
        }
    }

    pub fn clearable(mut self) -> Self {
        self.clearable = true;
        self
    }

    pub fn placeholder(mut self, text: &'a str) -> Self {
        self.placeholder = Some(text);
        self
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
        if self.clearable && !self.selected.is_empty() && clear_button(ui, self.rect, self.id) {
            self.selected.clear();
            self.state.open = false;
            return true;
        }
        if head(
            ui,
            self.rect,
            self.id,
            (&summary, self.placeholder, None),
            self.state,
        ) || !self.state.open
        {
            return false;
        }
        let chosen = self.selected.clone();
        let full = chosen.len() >= self.max;
        let key = format!("dropdown-{}", self.id);
        let picked = ui.add(
            Group::new(|ui: &mut UI| {
                popup(
                    ui,
                    (self.rect, self.id),
                    self.options,
                    self.state,
                    (None, None),
                    |o| match chosen.iter().position(|s| s == o) {
                        Some(p) => Mark::Checked(p),
                        None if full => Mark::Disabled,
                        None => Mark::Unchecked,
                    },
                )
            })
            .anim(popover_anim())
            .key(&key),
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
