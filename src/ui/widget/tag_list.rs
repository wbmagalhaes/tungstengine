use crate::animation::Ease;
use crate::ui::style::Transition;
use std::hash::Hash;

use crate::ui::immediate::{LineIcon, UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::widget::TextInput;
use crate::ui::widget::Widget;

const CROSS_SIZE: f32 = 7.0;
const CLOSE_HOVER: Transition = Transition::new(0.08, Ease::OutBack).exit(0.16, Ease::OutCubic);

pub struct TagList<'a> {
    rect: Rect,
    tags: &'a mut Vec<String>,
    input: &'a mut String,
    placeholder: Option<&'a str>,
    id: u64,
}

pub struct TagListOut {
    pub changed: bool,
    pub height: f32,
}

impl<'a> TagList<'a> {
    pub fn new(
        rect: Rect,
        tags: &'a mut Vec<String>,
        input: &'a mut String,
        id: impl Hash,
    ) -> Self {
        TagList {
            rect,
            tags,
            input,
            placeholder: None,
            id: key_hash(id),
        }
    }

    pub fn placeholder(mut self, text: &'a str) -> Self {
        self.placeholder = Some(text);
        self
    }
}

impl Widget for TagList<'_> {
    type Out = TagListOut;

    fn render(self, ui: &mut UI) -> TagListOut {
        let t = ui.theme;
        let r = self.rect;
        let (h, gap, size) = (r.h, 6.0, t.font_size);
        let (mut x, mut y) = (r.x, r.y);
        let mut remove = None;
        for (n, tag) in self.tags.iter().enumerate() {
            let w = ui.text_width(size, tag) + 40.0;
            if x + w > r.x + r.w && x > r.x {
                x = r.x;
                y += h + gap;
            }
            let chip = Rect::new(x, y, w, h).with_z(r.z);
            ui.surface(chip, t.chip_bg);
            ui.label(
                chip.x + 10.0,
                chip.y + (h - size) / 2.0,
                tag,
                t.chip_fg,
                size,
                r.z + 0.001,
            );
            let close = Rect::new(chip.x + w - 22.0, chip.y + (h - 18.0) / 2.0, 18.0, 18.0)
                .with_z(r.z + 0.001);
            let resp = ui.interact_curved(close, (self.id, n), CLOSE_HOVER);
            ui.fill_round(
                close,
                t.chip_hover.lerp(t.accent, resp.hover()),
                close.w / 2.0,
            );
            let (cx, cy) = (close.x + close.w / 2.0, close.y + close.h / 2.0);
            ui.icon_line(
                LineIcon::Cross,
                (cx, cy),
                CROSS_SIZE,
                t.chip_fg,
                r.z + 0.002,
            );
            if resp.clicked {
                remove = Some(n);
            }
            x += w + gap;
        }
        let field_w = 120.0;
        if x + field_w > r.x + r.w && x > r.x {
            x = r.x;
            y += h + gap;
        }
        let field = Rect::new(x, y, field_w, h).with_z(r.z);
        let mut changed = false;
        let field_id = (self.id, "add");
        let input = TextInput::new(field, field_id, self.input)
            .placeholder(self.placeholder.unwrap_or("Add…"));
        if ui.add(input) {
            ui.set_focus(Some(key_hash(field_id)));
            let tag = self.input.trim().to_string();
            if !tag.is_empty() && !self.tags.contains(&tag) {
                self.tags.push(tag);
                changed = true;
            }
            self.input.clear();
        }
        if let Some(n) = remove {
            self.tags.remove(n);
            changed = true;
        }
        TagListOut {
            changed,
            height: y + h - r.y,
        }
    }
}
