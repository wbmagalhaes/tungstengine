use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::widget::{Response, Widget};

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
        let mut resp = ui.interact_id(r, ("checkbox", self.label));
        if resp.clicked {
            *self.value = !*self.value;
            resp.changed = true;
        }
        let bx = Rect::new(x, y + 2.0, 20.0, 20.0).with_z(z);
        ui.fill(bx, t.accent.lerp(t.button_fg, resp.hover() * 0.3));
        ui.fill(bx.with_padding(2.0).inner().with_z(z + 0.001), t.field_bg);
        if *self.value {
            ui.fill(bx.with_padding(5.0).inner().with_z(z + 0.002), t.mark);
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
