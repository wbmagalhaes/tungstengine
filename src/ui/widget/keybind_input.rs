use std::hash::Hash;

use crate::animation::Ease;
use crate::input::{Binding, KeyCode};
use crate::menus::binding_label;
use crate::ui::immediate::{UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::Transition;
use crate::ui::widget::{Response, Widget};

const HOVER: Transition = Transition::new(0.08, Ease::OutCubic).exit(0.18, Ease::InOutQuad);
const HOVER_TINT: f32 = 0.14;
const HINT_ALPHA: f32 = 0.6;

pub struct KeybindInput<'a> {
    rect: Rect,
    binding: &'a mut Binding,
    listening: &'a mut bool,
    captured: Option<Binding>,
    hint: &'a str,
    id: u64,
}

impl<'a> KeybindInput<'a> {
    pub fn new(
        rect: Rect,
        binding: &'a mut Binding,
        listening: &'a mut bool,
        id: impl Hash,
    ) -> Self {
        KeybindInput {
            rect,
            binding,
            listening,
            captured: None,
            hint: "Press a key…",
            id: key_hash(id),
        }
    }

    pub fn captured(mut self, captured: Option<Binding>) -> Self {
        self.captured = captured;
        self
    }

    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = hint;
        self
    }
}

impl Widget for KeybindInput<'_> {
    type Out = Response;

    fn render(self, ui: &mut UI) -> Response {
        let t = ui.theme;
        let r = self.rect;
        let mut resp = ui.interact_curved(r, (self.id, "keybind"), HOVER);
        let listening = *self.listening;
        if let (true, Some(binding)) = (listening, self.captured) {
            *self.listening = false;
            if binding != Binding::Key(KeyCode::Escape) {
                *self.binding = binding;
                resp.changed = true;
            }
        } else if resp.clicked && !listening {
            *self.listening = true;
        }

        ui.field_box(r, *self.listening);
        if resp.hover() > 0.0 {
            let tint = t.accent.with_alpha(HOVER_TINT * resp.hover());
            ui.fill_round(r.with_z(r.z + 0.001), tint, t.border_radius);
        }
        let size = t.font_size;
        let label = binding_label(*self.binding);
        let (text, ink) = if *self.listening {
            (self.hint, t.label.with_alpha(HINT_ALPHA))
        } else {
            (label.as_str(), t.label)
        };
        let x = r.x + (r.w - ui.text_width(size, text)) / 2.0;
        ui.label(x, r.y + (r.h - size) / 2.0, text, ink, size, r.z + 0.002);
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    const AREA: Rect = Rect {
        x: 0.0,
        y: 0.0,
        w: 120.0,
        h: 30.0,
        padding: 0.0,
        z: 0.0,
    };

    fn frame(
        ui: &mut UI,
        binding: &mut Binding,
        listening: &mut bool,
        released: bool,
        captured: Option<Binding>,
    ) -> bool {
        let pointer = Pointer {
            pos: (10.0, 10.0),
            released,
            ..Default::default()
        };
        ui.begin(800.0, 600.0, 0.016, pointer);
        let input = KeybindInput::new(AREA, binding, listening, "k").captured(captured);
        ui.add(input).changed
    }

    #[test]
    fn click_listens_and_the_next_input_binds() {
        let mut ui = UI::new();
        let mut binding = Binding::Key(KeyCode::KeyA);
        let mut listening = false;
        frame(&mut ui, &mut binding, &mut listening, true, None);
        assert!(listening);
        let next = Some(Binding::Key(KeyCode::KeyB));
        assert!(frame(&mut ui, &mut binding, &mut listening, false, next));
        assert!(!listening);
        assert!(binding == Binding::Key(KeyCode::KeyB));
    }

    #[test]
    fn escape_cancels_and_keeps_the_binding() {
        let mut ui = UI::new();
        let mut binding = Binding::Key(KeyCode::KeyA);
        let mut listening = true;
        let escape = Some(Binding::Key(KeyCode::Escape));
        assert!(!frame(&mut ui, &mut binding, &mut listening, false, escape));
        assert!(!listening);
        assert!(binding == Binding::Key(KeyCode::KeyA));
    }
}
