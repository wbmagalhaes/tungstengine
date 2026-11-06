use std::f32::consts::FRAC_PI_2;
use std::hash::Hash;

use crate::animation::Ease;
use crate::ui::immediate::{FIELD_PAD, UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::{ComputedStyle, LinearGradient, Styled, Transition};
use crate::ui::widget::{Response, Text, Widget};

const FILL_INSET: f32 = 2.0;
const FILL_START_ALPHA: f32 = 0.3;
const FILL_END_ALPHA: f32 = 0.85;
const HANDLE_WIDTH: f32 = 4.0;
const HANDLE_INSET: f32 = 7.0;
const HANDLE_ACTIVE_SCALE: f32 = 1.6;
const HANDLE_GLOW: f32 = 7.0;
const HANDLE_GLOW_ALPHA: f32 = 0.3;
const HOVER_SHARE: f32 = 0.35;
const HANDLE_FADE: Transition = Transition::new(0.1, Ease::OutCubic).exit(0.25, Ease::InOutQuad);
const HANDLE_LIGHT: f32 = 0.45;
const HANDLE_TRANSITION: Transition = Transition::new(0.1, Ease::OutBack).exit(0.2, Ease::OutCubic);

pub struct Slider<'a> {
    value: &'a mut f32,
    rect: Rect,
    id: u64,
    label: Option<&'a str>,
    readout: Option<&'a str>,
    min: f32,
    max: f32,
    step: f32,
}

impl<'a> Slider<'a> {
    pub fn new(value: &'a mut f32, rect: Rect, id: impl Hash) -> Self {
        Slider {
            value,
            rect,
            id: key_hash(id),
            label: None,
            readout: None,
            min: 0.0,
            max: 1.0,
            step: 0.0,
        }
    }

    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
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

impl Widget for Slider<'_> {
    type Out = Response;

    fn render(self, ui: &mut UI) -> Response {
        let t = ui.theme;
        let r = self.rect;
        let mut resp = ui.interact_id(r, (self.id, "slider"));
        if resp.pressed {
            ui.active_slider = Some(self.id);
        }
        if !ui.pointer_down() && ui.active_slider == Some(self.id) {
            ui.active_slider = None;
        }
        let dragging = ui.active_slider == Some(self.id);
        if dragging {
            let share = ((ui.cursor().0 - r.x) / r.w.max(1.0)).clamp(0.0, 1.0);
            let raw = self.min + (self.max - self.min) * share;
            let next = if self.step > 0.0 {
                let steps = ((raw - self.min) / self.step).round();
                (self.min + steps * self.step).clamp(self.min, self.max)
            } else {
                raw
            };
            resp.changed = next != *self.value;
            *self.value = next;
        }
        let span = (self.max - self.min).max(f32::EPSILON);
        let share = ((*self.value - self.min) / span).clamp(0.0, 1.0);
        let at = r.x + r.w * share;
        let radius = t.border_radius;
        ui.fill_bordered(r, t.field_bg, radius, 1.0, t.panel_border);

        let inner = r.with_padding(FILL_INSET).inner().with_z(r.z);
        ui.push_clip_round(inner, (radius - FILL_INSET).max(0.0));
        let fill = Rect::new(inner.x, inner.y, at - inner.x, inner.h).with_z(r.z + 0.001);
        let look = ComputedStyle {
            gradient: Some(LinearGradient {
                angle: FRAC_PI_2,
                from: t.accent.with_alpha(FILL_START_ALPHA),
                via: None,
                to: t.accent.with_alpha(FILL_END_ALPHA),
            }),
            ..ComputedStyle::BASE
        };
        ui.fill_styled(fill, t.accent, &look);

        let target = match (dragging, resp.hovered) {
            (true, _) => 1.0,
            (false, true) => HOVER_SHARE,
            (false, false) => 0.0,
        };
        let grow = ui.transition((self.id, "handle"), target, HANDLE_TRANSITION);
        let width = HANDLE_WIDTH * (1.0 + (HANDLE_ACTIVE_SCALE - 1.0) * grow);
        let height = r.h - HANDLE_INSET * 2.0;
        let glow = HANDLE_GLOW * grow;
        if glow > 0.0 {
            let halo = Rect::new(at - width / 2.0 - glow, r.y, width + glow * 2.0, r.h)
                .with_z(r.z + 0.002);
            ui.fill_round(halo, t.accent.with_alpha(HANDLE_GLOW_ALPHA), glow);
        }
        ui.pop_clip();
        let handle =
            Rect::new(at - width / 2.0, r.y + HANDLE_INSET, width, height).with_z(r.z + 0.003);
        let shown_target = f32::from(u8::from(dragging || resp.hovered));
        let shown = ui.transition((self.id, "handle-shown"), shown_target, HANDLE_FADE);
        if shown > 0.0 {
            let color = t.accent.lerp(t.button_fg, HANDLE_LIGHT);
            ui.fill_round(handle, color.with_alpha(color.a * shown), width / 2.0);
        }

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
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    fn frame(ui: &mut UI, value: &mut f32, x: f32, held: bool) {
        let pointer = Pointer {
            pos: (x, 10.0),
            pressed: held,
            held,
            ..Default::default()
        };
        ui.begin(800.0, 600.0, 0.016, pointer);
        ui.add(Slider::new(value, Rect::new(0.0, 0.0, 118.0, 20.0), "s"));
    }

    #[test]
    fn dragging_keeps_control_outside_the_track() {
        let mut ui = UI::new();
        let mut value = 0.0;
        frame(&mut ui, &mut value, 59.0, true);
        assert!((value - 0.5).abs() < 0.01);
        frame(&mut ui, &mut value, 500.0, true);
        assert_eq!(value, 1.0);
        let knob = |ui: &UI| ui.draws.iter().rev().find_map(|d| d.round).unwrap().half[0];
        for _ in 0..30 {
            frame(&mut ui, &mut value, 500.0, true);
        }
        assert!((knob(&ui) - HANDLE_WIDTH * HANDLE_ACTIVE_SCALE / 2.0).abs() < 0.01);
        frame(&mut ui, &mut value, 59.0, false);
        frame(&mut ui, &mut value, 20.0, false);
        assert_eq!(value, 1.0);
    }

    #[test]
    fn rounded_clip_reaches_children_and_ends_with_the_pop() {
        let mut ui = UI::new();
        ui.begin(800.0, 600.0, 0.016, Pointer::default());
        let area = Rect::new(0.0, 0.0, 100.0, 40.0);
        ui.push_clip_round(area, 200.0);
        ui.push_clip(Rect::new(10.0, 10.0, 20.0, 20.0));
        ui.fill(Rect::new(0.0, 0.0, 5.0, 5.0), crate::color::Color::WHITE);
        let inner = ui.draws.last().unwrap().clip_round.unwrap();
        assert_eq!(inner.radii, [20.0; 4]);
        ui.pop_clip();
        ui.pop_clip();
        ui.fill(Rect::new(0.0, 0.0, 5.0, 5.0), crate::color::Color::WHITE);
        assert!(ui.draws.last().unwrap().clip_round.is_none());
    }

    #[test]
    fn range_and_step_use_real_units() {
        let mut ui = UI::new();
        let mut value = 7.0;
        let pointer = Pointer {
            pos: (61.0, 10.0),
            pressed: true,
            held: true,
            ..Default::default()
        };
        ui.begin(800.0, 600.0, 0.016, pointer);
        let area = Rect::new(0.0, 0.0, 100.0, 20.0);
        ui.add(
            Slider::new(&mut value, area, "n")
                .range(0.0, 20.0)
                .step(1.0),
        );
        assert_eq!(value, 12.0);
    }
}
