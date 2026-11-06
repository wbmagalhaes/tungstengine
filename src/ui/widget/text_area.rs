use std::hash::Hash;

use crate::ui::font;
use crate::ui::immediate::{FieldLook, UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::{Style, Styled};
use crate::ui::widget::Widget;

const PAD: f32 = 6.0;

pub struct TextArea<'a> {
    rect: Rect,
    id: u64,
    buffer: &'a mut String,
    style: Style,
    placeholder: Option<&'a str>,
}

#[cfg_attr(not(test), allow(dead_code))]
struct Row {
    start: usize,
    end: usize,
}

fn layout(chars: &[char], face: font::Font, px: f32, max_w: f32) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut start = 0;
    let mut x = 0.0;
    let mut last_space = None;
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\n' {
            rows.push(Row { start, end: i });
            start = i + 1;
            x = 0.0;
            last_space = None;
            i += 1;
            continue;
        }
        let adv = font::advance(face, px, ch);
        if x + adv > max_w && i > start {
            let cut = match last_space {
                Some(s) if s > start => s + 1,
                _ => i,
            };
            rows.push(Row { start, end: cut });
            start = cut;
            x = chars[start..i]
                .iter()
                .map(|c| font::advance(face, px, *c))
                .sum();
            last_space = None;
        }
        if ch == ' ' {
            last_space = Some(i);
        }
        x += adv;
        i += 1;
    }
    rows.push(Row {
        start,
        end: chars.len(),
    });
    rows
}

impl<'a> TextArea<'a> {
    pub fn new(rect: Rect, id: impl Hash, buffer: &'a mut String) -> Self {
        TextArea {
            rect,
            id: key_hash(id),
            buffer,
            style: Style::default(),
            placeholder: None,
        }
    }

    pub fn placeholder(mut self, text: &'a str) -> Self {
        self.placeholder = Some(text);
        self
    }

    pub fn height(text: &str, width: f32, px: f32, min_rows: usize) -> f32 {
        let chars: Vec<char> = text.chars().collect();
        let rows = layout(&chars, font::Font::Sans, px, width - PAD * 2.0)
            .len()
            .max(min_rows);
        rows as f32 * font::line_height(font::Font::Sans, px) + PAD * 2.0
    }
}

impl Styled for TextArea<'_> {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl Widget for TextArea<'_> {
    type Out = bool;

    fn render(self, ui: &mut UI) -> bool {
        let look = self.style.computed(ui.theme.text());
        let field = FieldLook {
            px: look.font_size,
            radii: look.border_radius,
            placeholder: self.placeholder,
        };
        ui.field_area(self.id, self.rect, self.buffer, field, true)
            .changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn newlines_split_rows() {
        let c = text("ab\ncd\n");
        let rows = layout(&c, font::Font::Sans, 14.0, 1000.0);
        assert_eq!(rows.len(), 3);
        assert_eq!((rows[1].start, rows[1].end), (3, 5));
        assert_eq!((rows[2].start, rows[2].end), (6, 6));
    }

    #[test]
    fn wraps_at_word_boundary() {
        let c = text("hello world again");
        let w = font::advance(font::Font::Sans, 14.0, 'h') * 8.0;
        let rows = layout(&c, font::Font::Sans, 14.0, w);
        assert!(rows.len() >= 2);
        assert_eq!(rows[1].start, 6);
    }
}
