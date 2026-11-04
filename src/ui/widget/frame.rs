use crate::color::Color;
use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::widget::Widget;

pub struct Frame {
    rect: Rect,
    accent: Option<Color>,
}

impl Frame {
    pub fn new(rect: Rect) -> Self {
        Frame { rect, accent: None }
    }

    pub fn accent(mut self, color: Color) -> Self {
        self.accent = Some(color);
        self
    }
}

impl Widget for Frame {
    type Out = ();

    fn render(self, ui: &mut UI) {
        let t = ui.theme;
        let r = self.rect;
        ui.fill(
            Rect::new(r.x + 3.0, r.y + 4.0, r.w, r.h).with_z(r.z - 0.002),
            Color::BLACK.with_alpha(0.35),
        );
        ui.fill(
            Rect::new(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0).with_z(r.z - 0.001),
            t.panel_border.with_alpha(0.9),
        );
        ui.fill(r, t.panel_bg.with_alpha(0.92));
        ui.fill(
            Rect::new(r.x, r.y, r.w, 2.0).with_z(r.z + 0.001),
            self.accent.unwrap_or(t.accent).with_alpha(0.55),
        );
    }
}
