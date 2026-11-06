use std::hash::Hash;

use crate::animation::Ease;
use crate::ui::immediate::{UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::Transition;
use crate::ui::widget::Widget;

const BAR_WIDTH: f32 = 4.0;
const BAR_MARGIN: f32 = 3.0;
const MIN_THUMB: f32 = 30.0;
const THUMB_ALPHA: f32 = 0.75;
const BAR_FADE: Transition = Transition::new(0.12, Ease::OutCubic).exit(0.35, Ease::InOutQuad);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ScrollAxis {
    #[default]
    Vertical,
    Horizontal,
    Both,
}

pub struct ScrollView<'a, F> {
    rect: Rect,
    id: u64,
    content: &'a mut (f32, f32),
    axis: ScrollAxis,
    body: F,
}

impl<'a, F: FnOnce(&mut UI, (f32, f32)) -> (f32, f32)> ScrollView<'a, F> {
    pub fn new(rect: Rect, id: impl Hash, content: &'a mut (f32, f32), body: F) -> Self {
        ScrollView {
            rect,
            id: key_hash(id),
            content,
            axis: ScrollAxis::Vertical,
            body,
        }
    }

    pub fn axis(mut self, axis: ScrollAxis) -> Self {
        self.axis = axis;
        self
    }
}

fn thumb(ui: &mut UI, track: Rect, offset: f32, max: f32, content: f32, shown: f32, across: bool) {
    let long = if across { track.w } else { track.h };
    let size = (long * long / content).max(MIN_THUMB);
    let at = offset / max * (long - size);
    let rect = if across {
        Rect::new(track.x + at, track.y, size, BAR_WIDTH)
    } else {
        Rect::new(track.x, track.y + at, BAR_WIDTH, size)
    };
    let color = ui.theme.accent.with_alpha(THUMB_ALPHA * shown);
    ui.fill_round(rect.with_z(track.z), color, BAR_WIDTH / 2.0);
}

impl<F: FnOnce(&mut UI, (f32, f32)) -> (f32, f32)> Widget for ScrollView<'_, F> {
    type Out = (f32, f32);

    fn render(self, ui: &mut UI) -> (f32, f32) {
        let t = ui.theme;
        let r = self.rect;
        let horizontal = self.axis != ScrollAxis::Vertical;
        let vertical = self.axis != ScrollAxis::Horizontal;
        let offset = ui.scroll_view(self.id, r, *self.content, (horizontal, vertical));
        let origin = (r.x - offset.0, r.y - offset.1);
        ui.push_clip_round(r, t.border_radius);
        let end = (self.body)(ui, origin);
        ui.pop_clip();
        *self.content = ((end.0 - origin.0).max(0.0), (end.1 - origin.1).max(0.0));

        let (width, height) = *self.content;
        let max_x = if horizontal {
            (width - r.w).max(0.0)
        } else {
            0.0
        };
        let max_y = if vertical {
            (height - r.h).max(0.0)
        } else {
            0.0
        };
        let target = f32::from(u8::from(
            r.contains(ui.cursor()) && (max_x > 0.0 || max_y > 0.0),
        ));
        let shown = ui.transition((self.id, "bar"), target, BAR_FADE);
        if shown > 0.0 {
            let z = r.z + 0.2;
            if max_y > 0.0 {
                let x = r.x + r.w - BAR_MARGIN - BAR_WIDTH;
                let track = Rect::new(x, r.y, BAR_WIDTH, r.h).with_z(z);
                thumb(ui, track, offset.1, max_y, height, shown, false);
            }
            if max_x > 0.0 {
                let y = r.y + r.h - BAR_MARGIN - BAR_WIDTH;
                let track = Rect::new(r.x, y, r.w, BAR_WIDTH).with_z(z);
                thumb(ui, track, offset.0, max_x, width, shown, true);
            }
        }
        offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    fn frame(ui: &mut UI, content: &mut (f32, f32), axis: ScrollAxis, wheel: f32) -> (f32, f32) {
        let pointer = Pointer {
            pos: (10.0, 10.0),
            wheel,
            ..Default::default()
        };
        ui.begin(800.0, 600.0, 0.016, pointer);
        let area = Rect::new(0.0, 0.0, 100.0, 100.0);
        let view = ScrollView::new(area, "s", content, |_, at| (at.0 + 300.0, at.1 + 400.0));
        ui.add(view.axis(axis))
    }

    #[test]
    fn vertical_learns_its_size_and_stays_in_limits() {
        let mut ui = UI::new();
        let mut content = (0.0, 0.0);
        frame(&mut ui, &mut content, ScrollAxis::Vertical, 0.0);
        assert_eq!(content, (300.0, 400.0));
        assert_eq!(
            frame(&mut ui, &mut content, ScrollAxis::Vertical, -1.0),
            (0.0, 60.0)
        );
        assert_eq!(
            frame(&mut ui, &mut content, ScrollAxis::Vertical, -100.0),
            (0.0, 300.0)
        );
    }

    #[test]
    fn horizontal_uses_the_wheel_sideways() {
        let mut ui = UI::new();
        let mut content = (0.0, 0.0);
        frame(&mut ui, &mut content, ScrollAxis::Horizontal, 0.0);
        assert_eq!(
            frame(&mut ui, &mut content, ScrollAxis::Horizontal, -1.0),
            (60.0, 0.0)
        );
    }

    #[test]
    fn both_scrolls_down_unless_shift_is_held() {
        let mut ui = UI::new();
        let mut content = (0.0, 0.0);
        frame(&mut ui, &mut content, ScrollAxis::Both, 0.0);
        assert_eq!(
            frame(&mut ui, &mut content, ScrollAxis::Both, -1.0),
            (0.0, 60.0)
        );
        ui.set_edit(crate::ui::input::EditInput {
            shift: true,
            ..Default::default()
        });
        let pointer = Pointer {
            pos: (10.0, 10.0),
            wheel: -1.0,
            ..Default::default()
        };
        ui.begin(800.0, 600.0, 0.016, pointer);
        ui.set_edit(crate::ui::input::EditInput {
            shift: true,
            ..Default::default()
        });
        let area = Rect::new(0.0, 0.0, 100.0, 100.0);
        let view = ScrollView::new(area, "s", &mut content, |_, at| {
            (at.0 + 300.0, at.1 + 400.0)
        });
        assert_eq!(ui.add(view.axis(ScrollAxis::Both)), (60.0, 60.0));
    }
}
