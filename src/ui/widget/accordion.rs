use std::hash::Hash;

use crate::animation::Ease;
use std::f32::consts::FRAC_PI_2;

use crate::ui::immediate::{LineIcon, UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::Transition;
use crate::ui::widget::Divider;
use crate::ui::widget::Widget;

const BODY_PAD: f32 = 8.0;
const TITLE_GAP: f32 = 10.0;
const CHEVRON_SIZE: f32 = 9.0;
const UNFOLD: Transition = Transition::new(0.22, Ease::OutCubic).exit(0.16, Ease::InOutQuad);
const HOVER: Transition = Transition::new(0.08, Ease::OutCubic).exit(0.18, Ease::InOutQuad);

#[derive(Default, Clone, Copy)]
pub struct AccordionState {
    pub open: bool,
    height: f32,
}

pub struct Accordion<'a, F> {
    rect: Rect,
    title: &'a str,
    id: u64,
    state: &'a mut AccordionState,
    body: F,
}

impl<'a, F: FnOnce(&mut UI, f32) -> f32> Accordion<'a, F> {
    pub fn new(
        rect: Rect,
        title: &'a str,
        id: impl Hash,
        state: &'a mut AccordionState,
        body: F,
    ) -> Self {
        Accordion {
            rect,
            title,
            id: key_hash(id),
            state,
            body,
        }
    }
}

impl<F: FnOnce(&mut UI, f32) -> f32> Widget for Accordion<'_, F> {
    type Out = f32;

    fn render(self, ui: &mut UI) -> f32 {
        let t = ui.theme;
        let r = self.rect;
        let resp = ui.interact_curved(r, (self.id, "head"), HOVER);
        if resp.clicked {
            self.state.open = !self.state.open;
        }
        let target = f32::from(u8::from(self.state.open));
        let unfolded = ui.transition((self.id, "unfold"), target, UNFOLD);
        let size = t.font_size;
        let lit = resp.hover().max(unfolded.clamp(0.0, 1.0));
        let ink = t.label.lerp(t.button_fg, lit);
        let center = (r.x + CHEVRON_SIZE / 2.0 + 2.0, r.y + r.h / 2.0);
        let start = ui.draws.len();
        ui.icon_line(
            LineIcon::ChevronDown,
            center,
            CHEVRON_SIZE,
            ink,
            r.z + 0.002,
        );
        let turn = -FRAC_PI_2 * (1.0 - unfolded);
        ui.transform_since(start, center, (1.0, 1.0), (0.0, 0.0), turn);
        let text_x = r.x + CHEVRON_SIZE + TITLE_GAP;
        ui.label(
            text_x,
            r.y + (r.h - size) / 2.0,
            self.title,
            ink,
            size,
            r.z + 0.002,
        );
        let line = r.y + r.h - 1.0;
        ui.add(Divider::horizontal(r.x, line, r.w).z(r.z + 0.001));

        let visible = self.state.height * unfolded;
        if unfolded > 0.0 {
            let top = r.y + r.h;
            ui.push_clip(Rect::new(r.x, top, r.w, visible.max(1.0)).with_z(r.z));
            let bottom = (self.body)(ui, top + BODY_PAD);
            ui.pop_clip();
            self.state.height = (bottom - top + BODY_PAD).max(0.0);
        }
        r.h + visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    fn frame(ui: &mut UI, state: &mut AccordionState, released: bool) -> f32 {
        let pointer = Pointer {
            pos: (10.0, 10.0),
            released,
            ..Default::default()
        };
        ui.begin(800.0, 600.0, 0.05, pointer);
        let head = Rect::new(0.0, 0.0, 200.0, 30.0);
        ui.add(Accordion::new(head, "A", "a", state, |_, y| y + 50.0))
    }

    #[test]
    fn click_unfolds_gradually_then_folds() {
        let mut ui = UI::new();
        let mut state = AccordionState::default();
        assert_eq!(frame(&mut ui, &mut state, false), 30.0);
        frame(&mut ui, &mut state, true);
        assert!(state.open);
        let early = frame(&mut ui, &mut state, false);
        let mut full = early;
        for _ in 0..20 {
            full = frame(&mut ui, &mut state, false);
        }
        assert!(early < full);
        assert_eq!(full, 30.0 + 50.0 + BODY_PAD * 2.0);
        frame(&mut ui, &mut state, true);
        for _ in 0..20 {
            full = frame(&mut ui, &mut state, false);
        }
        assert_eq!(full, 30.0);
    }
}
