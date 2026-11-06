use crate::animation::Ease;
use crate::ui::style::Transition;
use std::hash::Hash;

use crate::color::Color;
use crate::ui::immediate::{UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::widget::{ColorPicker, Widget};

const SWATCH_HOVER: Transition = Transition::new(0.12, Ease::OutBack).exit(0.2, Ease::OutCubic);

pub struct ColorList<'a> {
    rect: Rect,
    colors: &'a mut [String],
    editing: &'a mut Option<(usize, (f32, f32, f32))>,
    labels: &'a [&'a str],
    id: u64,
}

pub struct ColorListOut {
    pub changed: bool,
    pub height: f32,
}

impl<'a> ColorList<'a> {
    pub fn new(
        rect: Rect,
        colors: &'a mut [String],
        editing: &'a mut Option<(usize, (f32, f32, f32))>,
        id: impl Hash,
    ) -> Self {
        ColorList {
            rect,
            colors,
            editing,
            labels: &[],
            id: key_hash(id),
        }
    }

    pub fn labels(mut self, labels: &'a [&'a str]) -> Self {
        self.labels = labels;
        self
    }
}

impl Widget for ColorList<'_> {
    type Out = ColorListOut;

    fn render(self, ui: &mut UI) -> ColorListOut {
        let t = ui.theme;
        let r = self.rect;
        let size = t.font_size * 0.8;
        let cell = 96.0;
        let mut height = r.h;
        for (n, hex) in self.colors.iter().enumerate() {
            let x = r.x + n as f32 * (cell + 8.0);
            let swatch = Rect::new(x, r.y, r.h, r.h).with_z(r.z);
            let open = self.editing.is_some_and(|(e, _)| e == n);
            let resp = ui.interact_curved(swatch, (self.id, n), SWATCH_HOVER);
            let edge = if open {
                t.mark
            } else {
                t.accent.lerp(t.button_fg, resp.hover() * 0.4)
            };
            ui.fill_bordered(swatch, Color::hex(hex), t.border_radius, 2.0, edge);
            let label = self.labels.get(n).copied().unwrap_or(hex.as_str());
            ui.label(x + r.h + 6.0, r.y + 2.0, label, t.label, size, r.z);
            if !self.labels.is_empty() {
                ui.label(
                    x + r.h + 6.0,
                    r.y + 2.0 + size,
                    hex,
                    t.label.with_alpha(0.6),
                    size,
                    r.z,
                );
            }
            if resp.clicked {
                *self.editing = if open {
                    None
                } else {
                    Some((n, Color::hex(hex).to_hsv()))
                };
            }
        }
        let mut changed = false;
        if let Some((n, hsv)) = self.editing.as_mut()
            && *n < self.colors.len()
        {
            let pr = Rect::new(r.x, r.y + r.h + 8.0, 200.0, 170.0).with_z(r.z);
            if ui.add(ColorPicker::new(hsv, pr, (self.id, "picker"))) {
                self.colors[*n] = Color::hsv(hsv.0, hsv.1, hsv.2).to_hex();
                changed = true;
            }
            height += 178.0;
        }
        ColorListOut { changed, height }
    }
}
