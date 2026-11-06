use std::hash::Hash;

use crate::ui::immediate::{UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::Styled;
use crate::ui::widget::{Button, Response, Widget};

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
        let button = |ui: &mut UI, at: Rect, label: &str, enabled: bool, key: &str| {
            ui.add(
                Button::new(at, label)
                    .id((self.id, key))
                    .enabled(enabled)
                    .z(r.z),
            )
            .clicked()
        };
        if button(ui, Rect::new(r.x, r.y, side, r.h), "-", level > 0, "minus") {
            next -= 1;
        }
        if button(
            ui,
            Rect::new(r.x + r.w - side, r.y, side, r.h),
            "+",
            level < steps,
            "plus",
        ) {
            next += 1;
        }
        let x0 = r.x + side + 12.0;
        let span = r.w - 2.0 * side - 24.0;
        let seg = span / steps as f32;
        for i in 0..steps {
            let cell =
                Rect::new(x0 + i as f32 * seg + 2.0, r.y + 6.0, seg - 4.0, r.h - 12.0).with_z(r.z);
            let hover = cell.contains(ui.cursor);
            let on = i < level;
            let color = if on {
                t.accent.lerp(t.button_fg, if hover { 0.25 } else { 0.0 })
            } else if hover {
                t.track_hover
            } else {
                t.track_off
            };
            ui.surface(cell, color);
            if ui.interact_id(cell, (self.id, "seg", i)).clicked() {
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
