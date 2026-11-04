use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::style::{Style, StylePatch, Styled};
use crate::ui::widget::Widget;

pub struct Panel {
    rect: Rect,
    style: Style,
    patch: StylePatch,
}

impl Panel {
    pub fn new(rect: Rect) -> Self {
        Panel {
            rect,
            style: Style::default(),
            patch: StylePatch::default(),
        }
    }
}

impl Styled for Panel {
    fn patch_mut(&mut self) -> &mut StylePatch {
        &mut self.patch
    }
}

impl Widget for Panel {
    type Out = ();

    fn render(self, ui: &mut UI) {
        ui.fill(self.rect, self.style.with(self.patch).bg);
    }
}
