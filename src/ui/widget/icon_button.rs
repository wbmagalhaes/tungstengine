use std::hash::Hash;

use crate::ui::SpriteDraw;
use crate::ui::immediate::{LineIcon, UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::{Stateful, Style, Styled};
use crate::ui::widget::{Button, Response, Widget};

const ICON_SHARE: f32 = 0.4;
const SPRITE_SHARE: f32 = 0.65;
const DISABLED_ALPHA: f32 = 0.35;
const STILL: Style = Style {
    scale: Some((1.0, 1.0)),
    ..Style::NONE
};

#[derive(Clone, Copy)]
pub enum Icon {
    Line(LineIcon),
    Sprite(SpriteDraw),
}

pub struct IconButton {
    rect: Rect,
    icon: Icon,
    id: u64,
    enabled: bool,
}

impl IconButton {
    pub fn new(rect: Rect, icon: Icon, id: impl Hash) -> Self {
        IconButton {
            rect,
            icon,
            id: key_hash(id),
            enabled: true,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Widget for IconButton {
    type Out = Response;

    fn render(self, ui: &mut UI) -> Response {
        let r = self.rect;
        let button = Button::new(r, "")
            .id(self.id)
            .enabled(self.enabled)
            .hover(STILL)
            .active(STILL)
            .z(r.z);
        let resp = ui.add(button);
        let ink = ui.theme.label;
        let alpha = if self.enabled { 1.0 } else { DISABLED_ALPHA };
        let side = r.w.min(r.h);
        let center = (r.x + r.w / 2.0, r.y + r.h / 2.0);
        let z = r.z + 0.002;
        match self.icon {
            Icon::Line(icon) => {
                let color = ink.with_alpha(ink.a * alpha);
                ui.icon_line(icon, center, side * ICON_SHARE, color, z);
            }
            Icon::Sprite(sprite) => {
                let size = side * SPRITE_SHARE;
                let spot = Rect::new(center.0 - size / 2.0, center.1 - size / 2.0, size, size);
                let color = crate::color::Color::WHITE.with_alpha(alpha);
                ui.sprite_smooth(spot.with_z(z), sprite, color);
            }
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    fn frame(ui: &mut UI, released: bool, enabled: bool) -> bool {
        let pointer = Pointer {
            pos: (10.0, 10.0),
            released,
            ..Default::default()
        };
        ui.begin(800.0, 600.0, 0.016, pointer);
        ui.block_pointer(Rect::new(0.0, 0.0, 100.0, 100.0).with_z(5.0));
        let area = Rect::new(0.0, 0.0, 32.0, 32.0).with_z(6.0);
        let icon = Icon::Line(LineIcon::Plus);
        ui.add(IconButton::new(area, icon, "plus").enabled(enabled))
            .clicked()
    }

    #[test]
    fn click_passes_a_lower_blocker_through_its_own_z() {
        let mut ui = UI::new();
        assert!(frame(&mut ui, true, true));
        assert!(!frame(&mut ui, true, false));
    }
}
