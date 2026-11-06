use crate::animation::Ease;
use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::style::Transition;
use crate::ui::widget::{Response, Widget};

const BOX_HOVER: Transition = Transition::new(0.1, Ease::OutCubic).exit(0.2, Ease::InOutQuad);
const MARK_TRANSITION: Transition = Transition::new(0.1, Ease::OutCubic).exit(0.2, Ease::InOutQuad);

const BORDER: f32 = 2.0;
const MARK_INSET: f32 = 5.0;
const MAX_RADIUS_SHARE: f32 = 0.25;

pub struct Checkbox<'a> {
    value: &'a mut bool,
    label: &'a str,
    x: f32,
    y: f32,
    z: f32,
}

impl<'a> Checkbox<'a> {
    pub fn new(value: &'a mut bool, label: &'a str) -> Self {
        Checkbox {
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

impl Widget for Checkbox<'_> {
    type Out = Response;

    fn render(self, ui: &mut UI) -> Response {
        let t = ui.theme;
        let size = t.font_size * 0.95;
        let (x, y, z) = (self.x, self.y, self.z);
        let w = ui.text_width(size, self.label) + 34.0;
        let r = Rect::new(x, y, w, 24.0).with_z(z);
        let mut resp = ui.interact_curved(r, ("checkbox", self.label), BOX_HOVER);
        if resp.clicked {
            *self.value = !*self.value;
            resp.changed = true;
        }
        let bx = Rect::new(x, y + 2.0, 20.0, 20.0).with_z(z);
        let edge = t.accent.lerp(t.button_fg, resp.hover() * 0.3);
        let radius = t.border_radius.min(bx.w * MAX_RADIUS_SHARE);
        ui.fill_bordered(bx, t.field_bg, radius, BORDER, edge);
        let target = f32::from(u8::from(*self.value));
        let shown = ui.transition(("checkbox-mark", self.label), target, MARK_TRANSITION);
        if shown > 0.0 {
            let mark = bx.with_padding(MARK_INSET).inner().with_z(z + 0.002);
            let color = t.mark.with_alpha(t.mark.a * shown);
            ui.fill_round(mark, color, (radius - MARK_INSET).max(0.0));
        }
        ui.label(x + 28.0, y + 3.0, self.label, t.label, size, z);
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
    fn click_toggles_bound_value_and_reports_change() {
        let mut ui = UI::new();
        let mut on = false;
        click(&mut ui, (5.0, 10.0));
        let r = ui.add(Checkbox::new(&mut on, "Repetir").at(0.0, 0.0));
        assert!(on && r.changed());
        click(&mut ui, (500.0, 500.0));
        let r = ui.add(Checkbox::new(&mut on, "Repetir").at(0.0, 0.0));
        assert!(on && !r.changed());
    }
}
