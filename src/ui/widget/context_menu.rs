use std::hash::Hash;

use crate::animation::Ease;
use crate::ui::immediate::{FIELD_PAD, LineIcon, UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::Transition;
use crate::ui::widget::{Group, Widget};

const WIDTH: f32 = 200.0;
const ROW: f32 = 32.0;
const CHECK_SIZE: f32 = 11.0;
const HOVER_TINT: f32 = 0.08;
const DISABLED_ALPHA: f32 = 0.4;
const ENTER: f32 = 0.13;
const POP: f32 = 0.96;
const DEFAULT_Z: f32 = 200.0;
const ROW_HOVER: Transition = Transition::new(0.05, Ease::OutQuad).exit(0.14, Ease::OutCubic);

#[derive(Default)]
pub struct ContextState {
    pub open: bool,
    at: (f32, f32),
    fresh: bool,
}

impl ContextState {
    pub fn open_at(&mut self, at: (f32, f32)) {
        self.open = true;
        self.at = at;
        self.fresh = true;
    }
}

#[derive(Clone, Copy)]
pub struct MenuItem<'a> {
    pub label: &'a str,
    pub checked: bool,
    pub disabled: bool,
}

impl<'a> MenuItem<'a> {
    pub fn new(label: &'a str) -> Self {
        MenuItem {
            label,
            checked: false,
            disabled: false,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

pub struct ContextMenu<'a> {
    state: &'a mut ContextState,
    items: &'a [MenuItem<'a>],
    id: u64,
    z: f32,
}

impl<'a> ContextMenu<'a> {
    pub fn new(state: &'a mut ContextState, items: &'a [MenuItem<'a>], id: impl Hash) -> Self {
        ContextMenu {
            state,
            items,
            id: key_hash(id),
            z: DEFAULT_Z,
        }
    }

    pub fn z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }
}

impl Widget for ContextMenu<'_> {
    type Out = Option<usize>;

    fn render(self, ui: &mut UI) -> Option<usize> {
        if !self.state.open {
            return None;
        }
        let t = ui.theme;
        let height = ROW * self.items.len() as f32;
        let x = self.state.at.0.min(ui.width - WIDTH).max(0.0);
        let y = self.state.at.1.min(ui.height - height).max(0.0);
        let card = Rect::new(x, y, WIDTH, height).with_z(self.z);
        let (id, items) = (self.id, self.items);
        let key = format!("context-{id}");
        let picked = ui.add(
            Group::new(|ui: &mut UI| {
                ui.fill_bordered(card, t.field_bg, t.border_radius, 1.0, t.panel_border);
                ui.push_clip_round(card, t.border_radius);
                let mut picked = None;
                for (n, item) in items.iter().enumerate() {
                    let row = Rect::new(x, y + ROW * n as f32, WIDTH, ROW).with_z(card.z + 0.01);
                    let resp = ui.interact_curved(row, (id, n), ROW_HOVER);
                    if !item.disabled && resp.hover() > 0.0 {
                        let soft = t.button_fg.with_alpha(HOVER_TINT * resp.hover());
                        ui.fill(row, soft);
                    }
                    let ink = if item.disabled {
                        t.label.with_alpha(DISABLED_ALPHA)
                    } else {
                        t.label
                    };
                    let text_y = row.y + (ROW - t.font_size) / 2.0;
                    let tz = row.z + 0.01;
                    ui.label(x + FIELD_PAD, text_y, item.label, ink, t.font_size, tz);
                    if item.checked {
                        let center = (x + WIDTH - FIELD_PAD - CHECK_SIZE / 2.0, row.y + ROW / 2.0);
                        ui.icon_line(LineIcon::Check, center, CHECK_SIZE, t.accent, tz);
                    }
                    if resp.clicked && !item.disabled {
                        picked = Some(n);
                    }
                }
                ui.pop_clip();
                ui.block_pointer(card);
                picked
            })
            .pop_in(ENTER, POP)
            .key(&key),
        );
        if picked.is_some() {
            self.state.open = false;
        } else if self.state.fresh {
            self.state.fresh = false;
        } else if ui.clicked_outside(card) {
            self.state.open = false;
        }
        picked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    fn frame(
        ui: &mut UI,
        state: &mut ContextState,
        pos: (f32, f32),
        released: bool,
    ) -> Option<usize> {
        let pointer = Pointer {
            pos,
            released,
            ..Default::default()
        };
        ui.begin(800.0, 600.0, 0.016, pointer);
        let items = [
            MenuItem::new("Copy"),
            MenuItem::new("Paste").disabled(true),
            MenuItem::new("Grid").checked(true),
        ];
        ui.add(ContextMenu::new(state, &items, "menu"))
    }

    #[test]
    fn picking_closes_and_disabled_does_nothing() {
        let mut ui = UI::new();
        let mut state = ContextState::default();
        state.open_at((100.0, 100.0));
        frame(&mut ui, &mut state, (0.0, 0.0), false);
        assert_eq!(frame(&mut ui, &mut state, (120.0, 148.0), true), None);
        assert!(state.open);
        assert_eq!(frame(&mut ui, &mut state, (120.0, 112.0), true), Some(0));
        assert!(!state.open);
    }

    #[test]
    fn click_outside_closes_but_not_the_opening_click() {
        let mut ui = UI::new();
        let mut state = ContextState::default();
        state.open_at((100.0, 100.0));
        frame(&mut ui, &mut state, (5.0, 5.0), true);
        assert!(state.open);
        frame(&mut ui, &mut state, (5.0, 5.0), true);
        assert!(!state.open);
    }

    #[test]
    fn stays_inside_the_screen() {
        let mut ui = UI::new();
        let mut state = ContextState::default();
        state.open_at((790.0, 590.0));
        frame(&mut ui, &mut state, (0.0, 0.0), false);
        let first = ui.draws.iter().find_map(|d| d.round).unwrap();
        assert!(first.center[0] + first.half[0] <= 800.0);
    }
}
