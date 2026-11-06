use std::hash::Hash;

use crate::animation::Ease;
use crate::ui::immediate::{UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::Transition;
use crate::ui::widget::Widget;

const RING_SIZE: f32 = 20.0;
const RING_BORDER: f32 = 2.0;
const DOT_SIZE: f32 = 10.0;
const LABEL_GAP: f32 = 8.0;
const DOT: Transition = Transition::new(0.14, Ease::OutBack).exit(0.16, Ease::InOutQuad);
const HOVER: Transition = Transition::new(0.1, Ease::OutCubic).exit(0.2, Ease::InOutQuad);

pub struct RadioGroup<'a> {
    rect: Rect,
    options: &'a [&'a str],
    selected: &'a mut usize,
    id: u64,
}

impl<'a> RadioGroup<'a> {
    pub fn new(rect: Rect, options: &'a [&'a str], selected: &'a mut usize, id: impl Hash) -> Self {
        RadioGroup {
            rect,
            options,
            selected,
            id: key_hash(id),
        }
    }
}

impl Widget for RadioGroup<'_> {
    type Out = bool;

    fn render(self, ui: &mut UI) -> bool {
        let t = ui.theme;
        let r = self.rect;
        let size = t.font_size;
        let mut changed = false;
        for (k, label) in self.options.iter().enumerate() {
            let row = Rect::new(r.x, r.y + k as f32 * r.h, r.w, r.h).with_z(r.z);
            let resp = ui.interact_curved(row, (self.id, k), HOVER);
            if resp.clicked && *self.selected != k {
                *self.selected = k;
                changed = true;
            }
            let ring_y = row.y + (row.h - RING_SIZE) / 2.0;
            let ring = Rect::new(row.x, ring_y, RING_SIZE, RING_SIZE).with_z(r.z);
            let edge = t.accent.lerp(t.button_fg, resp.hover() * 0.3);
            ui.fill_bordered(ring, t.field_bg, RING_SIZE / 2.0, RING_BORDER, edge);
            let target = f32::from(u8::from(*self.selected == k));
            let shown = ui.transition((self.id, "dot", k), target, DOT);
            if shown > 0.0 {
                let dot = DOT_SIZE * shown;
                let (cx, cy) = (row.x + RING_SIZE / 2.0, row.y + row.h / 2.0);
                let mark = Rect::new(cx - dot / 2.0, cy - dot / 2.0, dot, dot).with_z(r.z + 0.002);
                ui.fill_round(mark, t.mark, dot / 2.0);
            }
            let x = row.x + RING_SIZE + LABEL_GAP;
            let y = row.y + (row.h - size) / 2.0;
            ui.label(x, y, label, t.label, size, r.z + 0.002);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    #[test]
    fn click_picks_only_the_row_under_the_pointer() {
        let mut ui = UI::new();
        let mut selected = 0;
        let pointer = Pointer {
            pos: (10.0, 40.0),
            released: true,
            ..Default::default()
        };
        ui.begin(800.0, 600.0, 0.016, pointer);
        let area = Rect::new(0.0, 0.0, 200.0, 30.0);
        let changed = ui.add(RadioGroup::new(area, &["a", "b", "c"], &mut selected, "r"));
        assert!(changed);
        assert_eq!(selected, 1);
    }
}
