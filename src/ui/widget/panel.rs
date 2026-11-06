use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::style::{Style, Styled};
use crate::ui::widget::Widget;

pub struct Panel {
    rect: Rect,
    style: Style,
}

impl Panel {
    pub fn new(rect: Rect) -> Self {
        Panel {
            rect,
            style: Style::default(),
        }
    }
}

impl Styled for Panel {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl Widget for Panel {
    type Out = ();

    fn render(self, ui: &mut UI) {
        let style = self.style.computed(ui.theme.base());
        ui.fill_styled(self.rect, style.bg, &style);
    }
}
