use std::f32::consts::{FRAC_PI_2, PI};
use std::hash::Hash;

use crate::color::Color;
use crate::ui::immediate::{UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::{ComputedStyle, LinearGradient};
use crate::ui::widget::Widget;

const HUE_BANDS: usize = 6;
const MARK: f32 = 14.0;
const MARK_RING: f32 = 2.0;
const SHADE_OVERLAP: f32 = 1.0;

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
        let radius = ui.theme.border_radius;
        let paint =
            |ui: &mut UI, r: Rect, corners: [f32; 4], angle: f32, from: Color, to: Color| {
                let look = ComputedStyle {
                    border_radius: corners,
                    gradient: Some(LinearGradient {
                        angle,
                        from,
                        via: None,
                        to,
                    }),
                    ..ComputedStyle::BASE
                };
                ui.fill_styled(r, from, &look);
            };
        let pure = Color::hsv(hsv.0, 1.0, 1.0);
        let base = Rect::new(
            sv.x + SHADE_OVERLAP,
            sv.y + SHADE_OVERLAP,
            sv.w - SHADE_OVERLAP * 2.0,
            sv.h - SHADE_OVERLAP * 2.0,
        )
        .with_z(sv.z);
        let base_radius = (radius - SHADE_OVERLAP).max(0.0);
        paint(ui, base, [base_radius; 4], FRAC_PI_2, Color::WHITE, pure);
        let shade = sv.with_z(rect.z + 0.001);
        let clear = Color::BLACK.with_alpha(0.0);
        paint(ui, shade, [radius; 4], PI, clear, Color::BLACK);

        let band = hue.h / HUE_BANDS as f32;
        for i in 0..HUE_BANDS {
            let r = Rect::new(hue.x, hue.y + i as f32 * band, bar, band).with_z(rect.z);
            let corners = match i {
                0 => [radius, radius, 0.0, 0.0],
                last if last == HUE_BANDS - 1 => [0.0, 0.0, radius, radius],
                _ => [0.0; 4],
            };
            let at =
                |step: usize| Color::hsv(step as f32 * 360.0 / HUE_BANDS as f32 % 360.0, 1.0, 1.0);
            paint(ui, r, corners, PI, at(i), at(i + 1));
        }

        let (mx, my) = (sv.x + hsv.1 * sv.w, sv.y + (1.0 - hsv.2) * sv.h);
        let dot = Rect::new(mx - MARK / 2.0, my - MARK / 2.0, MARK, MARK).with_z(rect.z + 0.01);
        let chosen = Color::hsv(hsv.0, hsv.1, hsv.2);
        ui.fill_bordered(dot, chosen, MARK / 2.0, MARK_RING, Color::WHITE);
        let hy = hue.y + hsv.0 / 359.0 * hue.h;
        let handle = Rect::new(hue.x - 2.0, hy - 3.0, bar + 4.0, 6.0).with_z(rect.z + 0.01);
        ui.fill_bordered(handle, pure, 3.0, MARK_RING, Color::WHITE);
        changed
    }
}
