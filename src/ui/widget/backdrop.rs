use crate::color::Color;
use crate::ui::animation::Anim;
use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::style::{ComputedStyle, Style, Styled};
use crate::ui::widget::Widget;

pub struct Backdrop {
    style: Style,
}

impl Backdrop {
    pub fn new() -> Self {
        let style = Style {
            bg: Some(Color::GRAY_900.with_alpha(0.55)),
            ..Style::default()
        };
        Backdrop { style }
    }
}

impl Default for Backdrop {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Backdrop {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl Widget for Backdrop {
    type Out = ();

    fn presence(&self) -> Option<Anim> {
        Some(Anim::new(0.16).fade())
    }

    fn presence_key(&self) -> Option<&str> {
        Some("ui-backdrop")
    }

    fn render(self, ui: &mut UI) {
        let style = self.style.computed(ComputedStyle::BASE);
        let r = Rect::new(0.0, 0.0, ui.width, ui.height).with_z(style.z);
        ui.fill(r, style.bg.with_alpha(style.bg.a * style.opacity));
        ui.block_pointer(r);
    }
}
