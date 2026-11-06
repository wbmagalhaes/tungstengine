use std::hash::Hash;

use crate::animation::Ease;
use crate::ui::immediate::{LineIcon, UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::Transition;
use crate::ui::widget::{Response, Widget};

const END_INSET: f32 = 3.0;
const END_TINT: f32 = 0.25;
const END_DISABLED: f32 = 0.3;
const ICON_SIZE: f32 = 12.0;
const TRACK_GAP: f32 = 4.0;
const SEGMENT_GAP: f32 = 4.0;
const SEGMENT_HEIGHT: f32 = 8.0;
const FILL_START_ALPHA: f32 = 0.5;
const FILL_END_ALPHA: f32 = 1.0;
const PILL_FADE: Transition = Transition::new(0.1, Ease::OutCubic).exit(0.18, Ease::InOutQuad);
const END_HOVER: Transition = Transition::new(0.08, Ease::OutCubic).exit(0.18, Ease::InOutQuad);
const SEGMENT_HOVER: Transition = Transition::new(0.06, Ease::OutQuad).exit(0.16, Ease::OutCubic);

pub struct Stepper<'a> {
    value: &'a mut f32,
    steps: u32,
    rect: Rect,
    id: u64,
}

impl<'a> Stepper<'a> {
    pub fn new(value: &'a mut f32, steps: u32, rect: Rect) -> Self {
        Stepper {
            value,
            steps: steps.max(1),
            rect,
            id: 0,
        }
    }

    pub fn id(mut self, id: impl Hash) -> Self {
        self.id = key_hash(id);
        self
    }
}

impl Widget for Stepper<'_> {
    type Out = Response;

    fn render(self, ui: &mut UI) -> Response {
        let t = ui.theme;
        let r = self.rect;
        let steps = self.steps as i32;
        let level = (*self.value * steps as f32).round() as i32;
        let mut next = level;
        let side = r.h.min(44.0);
        ui.field_box(r, false);

        let end_button = |ui: &mut UI, x: f32, key: &str, enabled: bool, plus: bool| {
            let zone = Rect::new(x, r.y, side, r.h)
                .with_padding(END_INSET)
                .inner()
                .with_z(r.z + 0.001);
            let resp = ui.interact_curved(zone, (self.id, key), END_HOVER);
            let lit = if enabled { resp.hover() } else { 0.0 };
            if lit > 0.0 {
                let radius = (t.border_radius - END_INSET).max(0.0);
                ui.fill_round(zone, t.accent.with_alpha(END_TINT * lit), radius);
            }
            let ink = t.label.with_alpha(if enabled { 1.0 } else { END_DISABLED });
            let (cx, cy) = (x + side / 2.0, r.y + r.h / 2.0);
            let icon = if plus {
                LineIcon::Plus
            } else {
                LineIcon::Minus
            };
            ui.icon_line(icon, (cx, cy), ICON_SIZE, ink, r.z + 0.002);
            enabled && resp.clicked
        };
        if end_button(ui, r.x, "minus", level > 0, false) {
            next -= 1;
        }
        if end_button(ui, r.x + r.w - side, "plus", level < steps, true) {
            next += 1;
        }

        let x0 = r.x + side + TRACK_GAP;
        let span = r.w - 2.0 * side - TRACK_GAP * 2.0;
        let seg = span / steps as f32;
        for i in 0..steps {
            let hit = Rect::new(x0 + i as f32 * seg, r.y, seg, r.h).with_z(r.z + 0.001);
            let resp = ui.interact_curved(hit, (self.id, "seg", i), SEGMENT_HOVER);
            let target = f32::from(u8::from(i < level));
            let lit = ui.transition((self.id, "lit", i), target, PILL_FADE);
            let ramp = (i + 1) as f32 / steps as f32;
            let strength = FILL_START_ALPHA + (FILL_END_ALPHA - FILL_START_ALPHA) * ramp;
            let empty = t.track_hover.lerp(t.accent, resp.hover() * 0.35);
            let full = t.accent.with_alpha(strength);
            let color = empty
                .lerp(full, lit)
                .lerp(t.button_fg, resp.hover() * lit * 0.2);
            let pill = Rect::new(
                hit.x + SEGMENT_GAP / 2.0,
                r.y + (r.h - SEGMENT_HEIGHT) / 2.0,
                seg - SEGMENT_GAP,
                SEGMENT_HEIGHT,
            )
            .with_z(r.z + 0.002);
            ui.fill_round(pill, color, SEGMENT_HEIGHT / 2.0);
            if resp.clicked {
                next = if level == i + 1 { i } else { i + 1 };
            }
        }
        let mut resp = ui.interact_id(r, (self.id, "stepper"));
        let next = next.clamp(0, steps);
        if next != level {
            *self.value = next as f32 / steps as f32;
            resp.changed = true;
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    fn click(ui: &mut UI, at: (f32, f32)) {
        ui.begin(
            800.0,
            600.0,
            0.016,
            Pointer {
                pos: at,
                released: true,
                ..Default::default()
            },
        );
    }

    #[test]
    fn plus_and_segments_update_bound_value() {
        let mut ui = UI::new();
        let row = Rect::new(0.0, 0.0, 400.0, 40.0);
        let mut v = 0.5;
        click(&mut ui, (380.0, 20.0));
        let r = ui.add(Stepper::new(&mut v, 10, row).id("vol"));
        assert!(r.changed());
        assert!((v - 0.6).abs() < 1e-4);
        click(&mut ui, (70.0, 20.0));
        ui.add(Stepper::new(&mut v, 10, row).id("vol"));
        assert!((v - 0.1).abs() < 1e-4);
    }
}
