use std::f32::consts::{FRAC_PI_2, TAU};
use std::hash::Hash;

use crate::animation::Ease;
use crate::color::Color;
use crate::ui::drag::DragDrop;
use crate::ui::immediate::{UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::sprite::SpriteDraw;
use crate::ui::style::Transition;
use crate::ui::widget::{Response, Widget};

const HOVER: Transition = Transition::new(0.08, Ease::OutCubic).exit(0.2, Ease::InOutQuad);
const ICON_INSET: f32 = 7.0;
const HOVER_TINT: f32 = 0.14;
const SELECTED_BORDER: f32 = 2.0;
const DRAGGED_ALPHA: f32 = 0.3;
const COOLDOWN_ALPHA: f32 = 0.6;
const COOLDOWN_STEPS: usize = 32;
const COUNT_PAD: f32 = 5.0;
const COUNT_SCALE: f32 = 0.85;

pub struct Slot {
    rect: Rect,
    id: u64,
    sprite: Option<SpriteDraw>,
    count: u32,
    rarity: Option<Color>,
    selected: bool,
    cooldown: f32,
    dragged: bool,
}

impl Slot {
    pub fn new(rect: Rect, id: impl Hash) -> Self {
        Slot {
            rect,
            id: key_hash(id),
            sprite: None,
            count: 0,
            rarity: None,
            selected: false,
            cooldown: 0.0,
            dragged: false,
        }
    }

    pub fn sprite(mut self, sprite: SpriteDraw) -> Self {
        self.sprite = Some(sprite);
        self
    }

    pub fn count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }

    pub fn rarity(mut self, color: Color) -> Self {
        self.rarity = Some(color);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn cooldown(mut self, remaining: f32) -> Self {
        self.cooldown = remaining.clamp(0.0, 1.0);
        self
    }

    pub fn dragged(mut self, dragged: bool) -> Self {
        self.dragged = dragged;
        self
    }
}

pub fn slot_pick<T: Copy>(ui: &UI, drag: &mut DragDrop<T>, resp: &Response, payload: T) {
    if resp.hovered && ui.pointer_edge() && drag.held.is_none() {
        drag.pick(payload, ui.cursor(), (resp.rect.x, resp.rect.y));
    }
}

pub fn slot_ghost<T: Copy>(ui: &mut UI, drag: &DragDrop<T>, sprite: SpriteDraw, size: f32, z: f32) {
    let Some(held) = drag.held else {
        return;
    };
    let cursor = ui.cursor();
    let (x, y) = if held.sticky {
        (cursor.0 - size / 2.0, cursor.1 - size / 2.0)
    } else {
        (cursor.0 - held.grab.0, cursor.1 - held.grab.1)
    };
    let icon = Rect::new(x, y, size, size)
        .with_padding(ICON_INSET)
        .inner()
        .with_z(z);
    ui.sprite(icon, sprite, Color::WHITE);
}

fn cooldown_fan(r: Rect, remaining: f32) -> Vec<(f32, f32)> {
    let center = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    let (half_w, half_h) = (r.w / 2.0, r.h / 2.0);
    let mut fan = vec![center];
    for step in 0..=COOLDOWN_STEPS {
        let share = step as f32 / COOLDOWN_STEPS as f32;
        let angle = -FRAC_PI_2 + TAU * (1.0 - remaining) + TAU * remaining * share;
        let (sin, cos) = angle.sin_cos();
        let reach = (half_w / cos.abs().max(1e-4)).min(half_h / sin.abs().max(1e-4));
        fan.push((center.0 + cos * reach, center.1 + sin * reach));
    }
    fan
}

impl Widget for Slot {
    type Out = Response;

    fn render(self, ui: &mut UI) -> Response {
        let t = ui.theme;
        let r = self.rect;
        let resp = ui.interact_curved(r, (self.id, "slot"), HOVER);
        let edge = match (self.selected, self.rarity) {
            (true, _) => t.accent,
            (false, Some(rarity)) => rarity,
            (false, None) => t.panel_border,
        };
        let border = if self.selected || self.rarity.is_some() {
            SELECTED_BORDER
        } else {
            1.0
        };
        let radius = t.border_radius;
        ui.fill_bordered(r, t.field_bg, radius, border, edge);
        if resp.hover() > 0.0 {
            let tint = t.accent.with_alpha(HOVER_TINT * resp.hover());
            ui.fill_round(r.with_z(r.z + 0.001), tint, radius);
        }
        if let Some(sprite) = self.sprite {
            let alpha = if self.dragged { DRAGGED_ALPHA } else { 1.0 };
            let icon = r.with_padding(ICON_INSET).inner().with_z(r.z + 0.002);
            ui.sprite(icon, sprite, Color::WHITE.with_alpha(alpha));
        }
        if self.cooldown > 0.0 {
            ui.push_clip_round(r, radius);
            let shade = Color::BLACK.with_alpha(COOLDOWN_ALPHA);
            ui.poly(&cooldown_fan(r, self.cooldown), shade, r.z + 0.003);
            ui.pop_clip();
        }
        if self.count > 1 {
            let text = self.count.to_string();
            let size = t.font_size * COUNT_SCALE;
            let x = r.x + r.w - COUNT_PAD - ui.text_width(size, &text);
            let y = r.y + r.h - COUNT_PAD - size;
            ui.label(x, y, &text, t.button_fg, size, r.z + 0.004);
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    #[test]
    fn full_cooldown_sweeps_all_around_and_stays_in_the_slot() {
        let r = Rect::new(0.0, 0.0, 40.0, 40.0);
        let fan = cooldown_fan(r, 1.0);
        assert!(fan.iter().all(|p| (0.0..=40.01).contains(&p.0)));
        assert!(fan.iter().all(|p| (0.0..=40.01).contains(&p.1)));
        let first = fan[1];
        let last = fan[fan.len() - 1];
        assert!((first.0 - last.0).abs() < 0.01 && (first.1 - last.1).abs() < 0.01);
    }

    #[test]
    fn pressing_a_slot_picks_its_payload_once() {
        let mut ui = UI::new();
        let pointer = Pointer {
            pos: (10.0, 10.0),
            pressed: true,
            held: true,
            ..Default::default()
        };
        ui.begin(800.0, 600.0, 0.016, pointer);
        let resp = ui.add(Slot::new(Rect::new(0.0, 0.0, 40.0, 40.0), "s"));
        let mut drag = DragDrop::<u32>::default();
        slot_pick(&ui, &mut drag, &resp, 7);
        slot_pick(&ui, &mut drag, &resp, 9);
        assert_eq!(drag.held.unwrap().payload, 7);
    }
}
