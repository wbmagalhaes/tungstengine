use std::f32::consts::{FRAC_PI_2, PI};

use crate::color::Color;
use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::style::{ComputedStyle, LinearGradient};
use crate::ui::widget::Widget;

const THICKNESS: f32 = 1.0;
const LABEL_GAP: f32 = 10.0;
const LINE_ALPHA: f32 = 0.45;
const LABEL_SIZE: f32 = 13.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    Horizontal,
    Vertical,
}

pub struct Divider<'a> {
    axis: Axis,
    x: f32,
    y: f32,
    length: f32,
    z: f32,
    color: Option<Color>,
    label: Option<&'a str>,
    fade: bool,
}

impl<'a> Divider<'a> {
    pub fn horizontal(x: f32, y: f32, length: f32) -> Self {
        Self::new(Axis::Horizontal, x, y, length)
    }

    pub fn vertical(x: f32, y: f32, length: f32) -> Self {
        Self::new(Axis::Vertical, x, y, length)
    }

    fn new(axis: Axis, x: f32, y: f32, length: f32) -> Self {
        Divider {
            axis,
            x,
            y,
            length,
            z: 0.0,
            color: None,
            label: None,
            fade: true,
        }
    }

    pub fn label(mut self, text: &'a str) -> Self {
        self.label = Some(text);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn solid(mut self) -> Self {
        self.fade = false;
        self
    }

    pub fn z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }
}

impl Widget for Divider<'_> {
    type Out = ();

    fn render(self, ui: &mut UI) {
        let color = self.color.unwrap_or(ui.theme.accent);
        let (mut x, mut length) = (self.x, self.length);
        let mut y = self.y;
        if let (Some(text), Axis::Horizontal) = (self.label, self.axis) {
            ui.label(x, y, text, color, LABEL_SIZE, self.z);
            let offset = ui.text_width(LABEL_SIZE, text) + LABEL_GAP;
            x += offset;
            length = (length - offset).max(0.0);
            y += LABEL_SIZE / 2.0;
        }
        let (x, y) = (x.round(), y.round());
        let (line, angle) = match self.axis {
            Axis::Horizontal => (Rect::new(x, y, length, THICKNESS), FRAC_PI_2),
            Axis::Vertical => (Rect::new(x, y, THICKNESS, length), PI),
        };
        let start = color.with_alpha(color.a * LINE_ALPHA);
        let end = if self.fade {
            start.with_alpha(0.0)
        } else {
            start
        };
        let look = ComputedStyle {
            gradient: Some(LinearGradient {
                angle,
                from: start,
                via: None,
                to: end,
            }),
            ..ComputedStyle::BASE
        };
        ui.fill_styled(line.with_z(self.z), start, &look);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    fn shape(divider: Divider) -> crate::ui::RoundShape {
        let mut ui = UI::new();
        ui.begin(800.0, 600.0, 0.016, Pointer::default());
        ui.add(divider);
        ui.draws.iter().find_map(|d| d.round).unwrap()
    }

    #[test]
    fn axis_sets_the_long_side() {
        let across = shape(Divider::horizontal(0.0, 0.0, 100.0));
        assert_eq!(across.half, [50.0, 0.5]);
        let down = shape(Divider::vertical(0.0, 0.0, 100.0));
        assert_eq!(down.half, [0.5, 50.0]);
    }

    #[test]
    fn line_sits_on_whole_pixels_in_both_axes() {
        let across = shape(Divider::horizontal(10.3, 20.6, 100.0));
        assert_eq!(across.center[1], 21.5);
        let down = shape(Divider::vertical(10.3, 20.6, 100.0));
        assert_eq!(down.center[0], 10.5);
    }
}
