use std::hash::Hash;

use crate::ui::immediate::{TextEdit, UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::widget::Widget;

pub struct TagList<'a> {
    rect: Rect,
    tags: &'a mut Vec<String>,
    input: &'a mut String,
    edit: &'a TextEdit,
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
        edit: &'a TextEdit,
        id: impl Hash,
    ) -> Self {
        TagList {
            rect,
            tags,
            input,
            edit,
            id: key_hash(id),
        }
    }
}

impl Widget for TagList<'_> {
    type Out = TagListOut;

    fn render(self, ui: &mut UI) -> TagListOut {
        let t = ui.theme;
        let r = self.rect;
        let (h, gap, size) = (r.h, 6.0, t.font_size * 0.85);
        let (mut x, mut y) = (r.x, r.y);
        let mut remove = None;
        for (n, tag) in self.tags.iter().enumerate() {
            let w = ui.text_width(size, tag) + 34.0;
            if x + w > r.x + r.w && x > r.x {
                x = r.x;
                y += h + gap;
            }
            let chip = Rect::new(x, y, w, h).with_z(r.z);
            ui.surface(chip, t.chip_bg);
            ui.label(
                chip.x + 8.0,
                chip.y + (h - size) / 2.0,
                tag,
                t.chip_fg,
                size,
                r.z + 0.001,
            );
            let close = Rect::new(chip.x + w - 22.0, chip.y + (h - 18.0) / 2.0, 18.0, 18.0)
                .with_z(r.z + 0.001);
            let resp = ui.interact_id(close, (self.id, n));
            ui.surface(close, t.chip_hover.lerp(t.accent, resp.hover()));
            ui.label(
                close.x + 5.0,
                close.y + 1.0,
                "×",
                t.chip_fg,
                size,
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
        if ui
            .text_field((self.id, "add"), field, self.input, self.edit)
            .submitted
        {
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
