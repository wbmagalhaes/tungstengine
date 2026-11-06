use crate::color::Color;
use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::style::Styled;
use crate::ui::widget::{Response, Text, Widget};

const HEIGHT: f32 = 18.0;
const PAD: f32 = 5.0;
const TEXT_SIZE: f32 = 12.0;

pub struct Badge<'a> {
    x: f32,
    y: f32,
    z: f32,
    text: &'a str,
    color: Option<Color>,
}

impl<'a> Badge<'a> {
    pub fn new(x: f32, y: f32, text: &'a str) -> Self {
        Badge {
            x,
            y,
            z: 0.0,
            text,
            color: None,
        }
    }

    pub fn corner_of(rect: Rect, text: &'a str) -> Self {
        Badge {
            x: rect.x + rect.w - HEIGHT / 2.0,
            y: rect.y - HEIGHT / 2.0,
            z: rect.z + 0.01,
            text,
            color: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }
}

impl Widget for Badge<'_> {
    type Out = Response;

    fn render(self, ui: &mut UI) -> Response {
        let t = ui.theme;
        let color = self.color.unwrap_or(t.accent);
        let text_w = ui.text_width(TEXT_SIZE, self.text);
        let width = (text_w + PAD * 2.0).max(HEIGHT);
        let area = Rect::new(self.x, self.y, width, HEIGHT).with_z(self.z);
        ui.fill_round(area, color, HEIGHT / 2.0);
        let x = area.x + (width - text_w) / 2.0;
        let y = area.y + (HEIGHT - TEXT_SIZE) / 2.0;
        ui.add(
            Text::new(x, y, self.text)
                .fg(t.field_bg)
                .font_size(TEXT_SIZE)
                .z(self.z + 0.001),
        );
        Response {
            rect: area,
            ..Response::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    #[test]
    fn short_text_stays_round_and_long_text_grows() {
        let mut ui = UI::new();
        ui.begin(800.0, 600.0, 0.016, Pointer::default());
        let short = ui.add(Badge::new(0.0, 0.0, "1")).rect;
        let long = ui.add(Badge::new(0.0, 0.0, "1234")).rect;
        assert_eq!(short.w, HEIGHT);
        assert!(long.w > short.w);
    }

    #[test]
    fn corner_badge_sits_over_its_owner() {
        let owner = Rect::new(10.0, 10.0, 40.0, 40.0).with_z(5.0);
        let mut ui = UI::new();
        ui.begin(800.0, 600.0, 0.016, Pointer::default());
        let at = ui.add(Badge::corner_of(owner, "3")).rect;
        assert!(at.z > owner.z);
        assert_eq!(at.y, 1.0);
    }
}
