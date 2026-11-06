use std::f32::consts::FRAC_PI_2;

use crate::animation::Ease;
use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::style::{ComputedStyle, LinearGradient, Transition};
use crate::ui::widget::{Response, Widget};

const TRACK_WIDTH: f32 = 40.0;
const TRACK_HEIGHT: f32 = 22.0;
const KNOB_INSET: f32 = 3.0;
const LABEL_GAP: f32 = 10.0;
const FILL_START_ALPHA: f32 = 0.45;
const SLIDE: Transition = Transition::new(0.18, Ease::OutBack).exit(0.16, Ease::OutCubic);
const HOVER: Transition = Transition::new(0.1, Ease::OutCubic).exit(0.2, Ease::InOutQuad);
const HOVER_GROW: f32 = 1.5;

pub struct Switch<'a> {
    value: &'a mut bool,
    label: &'a str,
    x: f32,
    y: f32,
    z: f32,
}

impl<'a> Switch<'a> {
    pub fn new(value: &'a mut bool, label: &'a str) -> Self {
        Switch {
            value,
            label,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    pub fn z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }
}

impl Widget for Switch<'_> {
    type Out = Response;

    fn render(self, ui: &mut UI) -> Response {
        let t = ui.theme;
        let (x, y, z) = (self.x, self.y, self.z);
        let size = t.font_size;
        let width = TRACK_WIDTH + LABEL_GAP + ui.text_width(size, self.label);
        let area = Rect::new(x, y, width, TRACK_HEIGHT).with_z(z);
        let mut resp = ui.interact_curved(area, ("switch", self.label), HOVER);
        if resp.clicked {
            *self.value = !*self.value;
            resp.changed = true;
        }
        let target = f32::from(u8::from(*self.value));
        let on = ui.transition(("switch-slide", self.label), target, SLIDE);

        let track = Rect::new(x, y, TRACK_WIDTH, TRACK_HEIGHT).with_z(z);
        let radius = TRACK_HEIGHT / 2.0;
        ui.fill_bordered(track, t.field_bg, radius, 1.0, t.panel_border);
        let shown = on.clamp(0.0, 1.0);
        if shown > 0.0 {
            let look = ComputedStyle {
                border_radius: [radius; 4],
                gradient: Some(LinearGradient {
                    angle: FRAC_PI_2,
                    from: t.accent.with_alpha(FILL_START_ALPHA * shown),
                    via: None,
                    to: t.accent.with_alpha(shown),
                }),
                ..ComputedStyle::BASE
            };
            ui.fill_styled(track.with_z(z + 0.001), t.accent, &look);
        }

        let base = TRACK_HEIGHT - KNOB_INSET * 2.0;
        let knob_size = base + HOVER_GROW * resp.hover();
        let travel = TRACK_WIDTH - KNOB_INSET * 2.0 - base;
        let cx = x + KNOB_INSET + base / 2.0 + travel * on;
        let cy = y + TRACK_HEIGHT / 2.0;
        let knob = Rect::new(
            cx - knob_size / 2.0,
            cy - knob_size / 2.0,
            knob_size,
            knob_size,
        );
        ui.fill_round(knob.with_z(z + 0.002), t.button_fg, knob_size / 2.0);

        let label_y = y + (TRACK_HEIGHT - size) / 2.0;
        ui.label(
            x + TRACK_WIDTH + LABEL_GAP,
            label_y,
            self.label,
            t.label,
            size,
            z,
        );
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    #[test]
    fn click_toggles_and_the_knob_travels() {
        let mut ui = UI::new();
        let mut value = false;
        let frame = |ui: &mut UI, value: &mut bool, released: bool| {
            let pointer = Pointer {
                pos: (10.0, 10.0),
                released,
                ..Default::default()
            };
            ui.begin(800.0, 600.0, 0.05, pointer);
            ui.add(Switch::new(value, "Luz"));
            let shapes: Vec<_> = ui.draws.iter().filter_map(|d| d.round).collect();
            shapes.last().unwrap().center[0]
        };
        let off = frame(&mut ui, &mut value, false);
        frame(&mut ui, &mut value, true);
        assert!(value);
        let mut on = off;
        for _ in 0..10 {
            on = frame(&mut ui, &mut value, false);
        }
        assert!(on > off + 10.0, "{off} {on}");
    }
}
