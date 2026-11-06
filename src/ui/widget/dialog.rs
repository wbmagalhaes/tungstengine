use std::hash::Hash;

use crate::ui::font::Font;
use crate::ui::immediate::{UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::Styled;
use crate::ui::widget::{Backdrop, Button, Group, Text, Widget};

const WIDTH: f32 = 420.0;
const PAD: f32 = 20.0;
const GAP: f32 = 12.0;
const TITLE_SIZE: f32 = 19.0;
const BUTTON_HEIGHT: f32 = 34.0;
const BUTTON_WIDTH: f32 = 104.0;
const CARD_RADIUS: f32 = 14.0;
const ENTER: f32 = 0.2;
const POP: f32 = 0.94;
const FADE: f32 = 0.16;
const DEFAULT_Z: f32 = 100.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DialogEvent {
    Button(usize),
    Dismissed,
}

pub struct Dialog<'a> {
    title: &'a str,
    text: &'a str,
    buttons: &'a [&'a str],
    id: u64,
    z: f32,
    dismissable: bool,
}

impl<'a> Dialog<'a> {
    pub fn new(title: &'a str, text: &'a str, buttons: &'a [&'a str]) -> Self {
        Dialog {
            title,
            text,
            buttons,
            id: key_hash(title),
            z: DEFAULT_Z,
            dismissable: false,
        }
    }

    pub fn id(mut self, id: impl Hash) -> Self {
        self.id = key_hash(id);
        self
    }

    pub fn z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }

    pub fn dismissable(mut self) -> Self {
        self.dismissable = true;
        self
    }
}

impl Widget for Dialog<'_> {
    type Out = Option<DialogEvent>;

    fn render(self, ui: &mut UI) -> Option<DialogEvent> {
        let t = ui.theme;
        let z = self.z;
        let key = format!("dialog-{}", self.id);
        let ready = ui.since_shown((self.id, "ready"), ENTER) >= 1.0;
        ui.add(
            Group::new(|ui: &mut UI| {
                ui.add(Backdrop::new().z(z));
            })
            .fade_in(FADE)
            .key(&format!("{key}-back")),
        );

        let inner = WIDTH - PAD * 2.0;
        let (_, text_h) = ui.measure(Font::Sans, t.font_size, self.text, Some(inner));
        let height = PAD * 2.0 + TITLE_SIZE + GAP + text_h + GAP * 2.0 + BUTTON_HEIGHT;
        let (x, y) = ((ui.width - WIDTH) / 2.0, (ui.height - height) / 2.0);
        let card = Rect::new(x, y, WIDTH, height).with_z(z + 1.0);
        let picked = ui.add(
            Group::new(|ui: &mut UI| {
                ui.fill_bordered(card, t.panel_bg, CARD_RADIUS, 1.0, t.panel_border);
                let tz = card.z + 0.01;
                ui.label(x + PAD, y + PAD, self.title, t.button_fg, TITLE_SIZE, tz);
                let text_y = y + PAD + TITLE_SIZE + GAP;
                ui.add(Text::new(x + PAD, text_y, self.text).wrapped(inner).z(tz));
                let row_y = y + height - PAD - BUTTON_HEIGHT;
                let mut picked = None;
                let count = self.buttons.len();
                for (i, label) in self.buttons.iter().enumerate() {
                    let from_right = (count - 1 - i) as f32;
                    let bx = x + WIDTH - PAD - BUTTON_WIDTH - from_right * (BUTTON_WIDTH + GAP);
                    let area = Rect::new(bx, row_y, BUTTON_WIDTH, BUTTON_HEIGHT).with_z(tz);
                    let mut button = Button::new(area, label).id((self.id, i)).z(tz);
                    if i + 1 == count {
                        button = button.bg(t.accent.with_alpha(0.55));
                    }
                    if ui.add(button).clicked() && ready {
                        picked = Some(i);
                    }
                }
                picked
            })
            .pop_in(ENTER, POP)
            .key(&key),
        );
        if let Some(i) = picked {
            return Some(DialogEvent::Button(i));
        }
        (ready && self.dismissable && ui.edit_input().escape).then_some(DialogEvent::Dismissed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;
    use crate::ui::input::EditInput;

    fn frame(ui: &mut UI, pos: (f32, f32), released: bool, escape: bool) -> Option<DialogEvent> {
        let pointer = Pointer {
            pos,
            released,
            ..Default::default()
        };
        ui.begin(800.0, 600.0, 0.5, pointer);
        ui.set_edit(EditInput {
            escape,
            ..Default::default()
        });
        ui.add(Dialog::new("Quit", "Sure?", &["No", "Yes"]).dismissable())
    }

    #[test]
    fn last_button_sits_right_and_reports_its_index() {
        let mut ui = UI::new();
        frame(&mut ui, (0.0, 0.0), false, false);
        frame(&mut ui, (0.0, 0.0), false, false);
        let x = (800.0 + WIDTH) / 2.0 - PAD - 10.0;
        let hit = (0..600)
            .map(|y| (x, y as f32))
            .find_map(|pos| frame(&mut ui, pos, true, false));
        assert_eq!(hit, Some(DialogEvent::Button(1)));
    }

    #[test]
    fn click_during_the_entrance_is_ignored() {
        let x = (800.0 + WIDTH) / 2.0 - PAD - 10.0;
        for y in 0..600 {
            let pointer = Pointer {
                pos: (x, y as f32),
                released: true,
                ..Default::default()
            };
            let mut fresh = UI::new();
            fresh.begin(800.0, 600.0, 0.016, pointer);
            let dialog = Dialog::new("Quit", "Sure?", &["No", "Yes"]);
            assert_eq!(fresh.add(dialog), None);
        }
    }

    #[test]
    fn escape_only_dismisses_when_allowed() {
        let mut ui = UI::new();
        assert_eq!(
            frame(&mut ui, (0.0, 0.0), false, true),
            Some(DialogEvent::Dismissed)
        );
        ui.begin(800.0, 600.0, 0.1, Pointer::default());
        ui.set_edit(EditInput {
            escape: true,
            ..Default::default()
        });
        assert_eq!(ui.add(Dialog::new("A", "B", &["Ok"])), None);
    }
}
