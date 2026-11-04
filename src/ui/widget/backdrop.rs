use crate::color::Color;
use crate::ui::animation::Anim;
use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::style::{Style, StylePatch, Styled};
use crate::ui::widget::Widget;

pub struct Backdrop {
    style: Style,
    patch: StylePatch,
}

impl Backdrop {
    pub fn new() -> Self {
        let style = Style {
            bg: Color::GRAY_900.with_alpha(0.55),
            ..Style::default()
        };
        Backdrop {
            style,
            patch: StylePatch::default(),
        }
    }
}

impl Default for Backdrop {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Backdrop {
    fn patch_mut(&mut self) -> &mut StylePatch {
        &mut self.patch
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
        let a = self.style.with(self.patch).bg.a * self.style.with(self.patch).opacity;
        let r = Rect::new(0.0, 0.0, ui.width, ui.height).with_z(self.style.with(self.patch).z);
        ui.fill(r, self.style.with(self.patch).bg.with_alpha(a));
        ui.block_pointer(r);
    }
}
