use std::f32::consts::FRAC_PI_2;
use std::hash::Hash;

use crate::animation::Ease;
use crate::color::Color;
use crate::ui::immediate::{FIELD_PAD, UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::{ComputedStyle, LinearGradient, Styled, Transition};
use crate::ui::widget::{Text, Widget};

const FILL_INSET: f32 = 2.0;
const FILL_START_ALPHA: f32 = 0.3;
const FILL_END_ALPHA: f32 = 0.85;
const TRAIL_ALPHA: f32 = 0.55;
const FILL_CURVE: Transition = Transition::new(0.22, Ease::OutCubic).exit(0.14, Ease::OutQuad);
const TRAIL_CURVE: Transition = Transition::new(0.14, Ease::OutQuad).exit(0.7, Ease::InOutCubic);

pub struct ProgressBar<'a> {
    value: f32,
    rect: Rect,
    id: u64,
    min: f32,
    max: f32,
    trail: bool,
    color: Option<Color>,
    label: Option<&'a str>,
    readout: Option<&'a str>,
}

impl<'a> ProgressBar<'a> {
    pub fn new(value: f32, rect: Rect, id: impl Hash) -> Self {
        ProgressBar {
            value,
            rect,
            id: key_hash(id),
            min: 0.0,
            max: 1.0,
            trail: false,
            color: None,
            label: None,
            readout: None,
        }
    }

    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn trail(mut self) -> Self {
        self.trail = true;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn readout(mut self, readout: &'a str) -> Self {
        self.readout = Some(readout);
        self
    }
}

impl Widget for ProgressBar<'_> {
    type Out = ();

    fn render(self, ui: &mut UI) {
        let t = ui.theme;
        let r = self.rect;
        let color = self.color.unwrap_or(t.accent);
        let span = (self.max - self.min).max(f32::EPSILON);
        let share = ((self.value - self.min) / span).clamp(0.0, 1.0);
        let shown = ui.transition((self.id, "fill"), share, FILL_CURVE);
        ui.field_box(r, false);

        let inner = r.with_padding(FILL_INSET).inner().with_z(r.z);
        ui.push_clip_round(inner, (t.border_radius - FILL_INSET).max(0.0));
        if self.trail {
            let behind = ui.transition((self.id, "trail"), share, TRAIL_CURVE);
            if behind > shown {
                let tail = Rect::new(inner.x, inner.y, inner.w * behind, inner.h);
                let pale = color.lerp(t.button_fg, 0.5).with_alpha(TRAIL_ALPHA);
                ui.fill(tail.with_z(r.z + 0.001), pale);
            }
        }
        let fill = Rect::new(inner.x, inner.y, inner.w * shown, inner.h).with_z(r.z + 0.002);
        let look = ComputedStyle {
            gradient: Some(LinearGradient {
                angle: FRAC_PI_2,
                from: color.with_alpha(FILL_START_ALPHA),
                via: None,
                to: color.with_alpha(FILL_END_ALPHA),
            }),
            ..ComputedStyle::BASE
        };
        ui.fill_styled(fill, color, &look);
        ui.pop_clip();

        let text_y = r.y + (r.h - t.font_size) / 2.0;
        let text_z = r.z + 0.004;
        if let Some(label) = self.label {
            ui.add(
                Text::new(r.x + FIELD_PAD, text_y, label)
                    .fg(t.button_fg)
                    .z(text_z),
            );
        }
        if let Some(readout) = self.readout {
            let x = r.x + r.w - FIELD_PAD - ui.text_width(t.font_size, readout);
            ui.add(Text::new(x, text_y, readout).fg(t.label).z(text_z));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    fn frame(ui: &mut UI, value: f32) -> Vec<f32> {
        ui.begin(800.0, 600.0, 0.05, Pointer::default());
        let area = Rect::new(0.0, 0.0, 104.0, 20.0).with_z(3.0);
        ui.add(ProgressBar::new(value, area, "hp").range(0.0, 10.0).trail());
        let widths = ui.draws.iter().map(|d| d.max[0] - d.min[0]);
        widths.collect()
    }

    #[test]
    fn trail_stays_behind_after_a_drop() {
        let mut ui = UI::new();
        for _ in 0..30 {
            frame(&mut ui, 10.0);
        }
        for _ in 0..6 {
            frame(&mut ui, 2.0);
        }
        let draws = frame(&mut ui, 2.0);
        assert!(draws.len() >= 3, "{draws:?}");
        assert!(ui.draws.iter().all(|d| d.z >= 3.0));
    }
}
