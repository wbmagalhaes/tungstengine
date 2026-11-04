use std::hash::Hash;

use crate::color::Color;
use crate::ui::immediate::{UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::widget::Widget;

pub struct ColorPicker<'a> {
    hsv: &'a mut (f32, f32, f32),
    rect: Rect,
    id: u64,
}

impl<'a> ColorPicker<'a> {
    pub fn new(hsv: &'a mut (f32, f32, f32), rect: Rect, id: impl Hash) -> Self {
        ColorPicker {
            hsv,
            rect,
            id: key_hash(id),
        }
    }
}

impl Widget for ColorPicker<'_> {
    type Out = bool;

    fn render(self, ui: &mut UI) -> bool {
        const STEPS: usize = 16;
        let (rect, hsv, id) = (self.rect, self.hsv, self.id);
        let bar = 18.0;
        let side = (rect.w - bar - 8.0).min(rect.h);
        let sv = Rect::new(rect.x, rect.y, side, side).with_z(rect.z);
        let hue = Rect::new(rect.x + side + 8.0, rect.y, bar, side).with_z(rect.z);
        let sv_resp = ui.interact_id(sv, (id, 1));
        let hue_resp = ui.interact_id(hue, (id, 2));
        let (cx, cy) = ui.cursor();
        let down = ui.pointer_down();
        let mut changed = false;
        if down && hue_resp.hovered {
            hsv.0 = ((cy - hue.y) / hue.h).clamp(0.0, 1.0) * 359.0;
            changed = true;
        }
        if down && sv_resp.hovered {
            hsv.1 = ((cx - sv.x) / sv.w).clamp(0.0, 1.0);
            hsv.2 = 1.0 - ((cy - sv.y) / sv.h).clamp(0.0, 1.0);
            changed = true;
        }
        let cell = side / STEPS as f32;
        for yi in 0..STEPS {
            for xi in 0..STEPS {
                let s = (xi as f32 + 0.5) / STEPS as f32;
                let v = 1.0 - (yi as f32 + 0.5) / STEPS as f32;
                let r = Rect::new(
                    sv.x + xi as f32 * cell,
                    sv.y + yi as f32 * cell,
                    cell + 0.5,
                    cell + 0.5,
                )
                .with_z(rect.z);
                ui.fill(r, Color::hsv(hsv.0, s, v));
            }
            let h = (yi as f32 + 0.5) / STEPS as f32 * 359.0;
            let r = Rect::new(hue.x, hue.y + yi as f32 * cell, bar, cell + 0.5).with_z(rect.z);
            ui.fill(r, Color::hsv(h, 1.0, 1.0));
        }
        let mark = |x: f32, y: f32, w: f32, h: f32| Rect::new(x, y, w, h).with_z(rect.z + 0.01);
        let (mx, my) = (sv.x + hsv.1 * sv.w, sv.y + (1.0 - hsv.2) * sv.h);
        ui.fill(mark(mx - 4.0, my - 4.0, 8.0, 8.0), Color::WHITE);
        ui.fill(
            mark(mx - 2.0, my - 2.0, 4.0, 4.0),
            Color::hsv(hsv.0, hsv.1, hsv.2),
        );
        let hy = hue.y + hsv.0 / 359.0 * hue.h;
        ui.fill(mark(hue.x - 2.0, hy - 2.0, bar + 4.0, 4.0), Color::WHITE);
        changed
    }
}
