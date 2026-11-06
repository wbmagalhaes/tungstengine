use crate::color::Color;
use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::sprite::SpriteDraw;
use crate::ui::style::Styled;
use crate::ui::widget::{Text, Widget};

pub struct Chip<'a> {
    rect: Rect,
    icon: SpriteDraw,
    text: &'a str,
    accent: Option<Color>,
}

impl<'a> Chip<'a> {
    pub fn new(rect: Rect, icon: SpriteDraw, text: &'a str) -> Self {
        Chip {
            rect,
            icon,
            text,
            accent: None,
        }
    }

    pub fn accent(mut self, color: Color) -> Self {
        self.accent = Some(color);
        self
    }
}

impl Widget for Chip<'_> {
    type Out = bool;

    fn render(self, ui: &mut UI) -> bool {
        let t = ui.theme;
        let r = self.rect;
        let hovered = r.contains(ui.cursor);
        let bg = if hovered { t.chip_hover } else { t.chip_bg };
        let accent = self.accent.unwrap_or(t.accent);
        if t.border_radius > 0.0 {
            ui.fill_bordered(r, bg, r.h / 2.0, 1.0, accent.with_alpha(0.5));
        } else {
            ui.surface(r, bg.with_alpha(0.95));
            ui.surface(
                Rect::new(r.x, r.y + r.h - 2.0, r.w, 2.0).with_z(r.z + 0.001),
                accent.with_alpha(0.85),
            );
        }
        let s = (r.h - 6.0).min(22.0);
        let lift = if t.border_radius > 0.0 { 0.0 } else { 1.0 };
        ui.sprite(
            Rect::new(r.x + 5.0, r.y + (r.h - s) / 2.0 - lift, s, s).with_z(r.z + 0.002),
            self.icon,
            if t.border_radius > 0.0 {
                accent
            } else {
                Color::WHITE
            },
        );
        ui.add(
            Text::new(r.x + 10.0 + s, r.y + (r.h - 15.0) / 2.0 - lift, self.text)
                .fg(t.chip_fg)
                .font_size(15.0)
                .z(r.z + 0.002),
        );
        hovered
    }
}
