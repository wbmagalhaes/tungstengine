use std::hash::Hash;

use crate::ui::immediate::{FieldLook, UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::{Style, Styled};
use crate::ui::widget::Widget;

pub struct TextInput<'a> {
    placeholder: Option<&'a str>,
    rect: Rect,
    id: u64,
    buffer: &'a mut String,
    style: Style,
}

impl<'a> TextInput<'a> {
    pub fn new(rect: Rect, id: impl Hash, buffer: &'a mut String) -> Self {
        TextInput {
            placeholder: None,
            rect,
            id: key_hash(id),
            buffer,
            style: Style::default(),
        }
    }
}

impl<'a> TextInput<'a> {
    pub fn placeholder(mut self, text: &'a str) -> Self {
        self.placeholder = Some(text);
        self
    }
}

impl Styled for TextInput<'_> {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl Widget for TextInput<'_> {
    type Out = bool;

    fn render(self, ui: &mut UI) -> bool {
        let look = self.style.computed(ui.theme.text());
        let field = FieldLook {
            px: look.font_size,
            radii: look.border_radius,
            placeholder: self.placeholder,
        };
        ui.field_line(self.id, self.rect, self.buffer, field)
            .entered
    }
}
