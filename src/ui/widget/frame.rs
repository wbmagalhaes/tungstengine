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
        let radius = t.border_radius;
        let shadow = Rect::new(r.x + 3.0, r.y + 4.0, r.w, r.h).with_z(r.z - 0.002);
        ui.fill_round(shadow, Color::BLACK.with_alpha(0.35), radius);
        let edge = t.panel_border.with_alpha(0.9);
        let outer = Rect::new(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0).with_z(r.z);
        ui.fill_bordered(outer, t.panel_bg.with_alpha(0.92), radius + 2.0, 2.0, edge);
        ui.push_clip_round(r, radius);
        ui.fill(
            Rect::new(r.x, r.y, r.w, 2.0).with_z(r.z + 0.001),
            self.accent.unwrap_or(t.accent).with_alpha(0.55),
        );
        ui.pop_clip();
    }
}
