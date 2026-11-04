use gilrs::{Axis, Button, EventType, Gilrs};

use super::Input;
use super::gamepad::{PadAxis, PadButton};

pub struct PadBackend {
    gilrs: Gilrs,
    announced: bool,
}

impl PadBackend {
    pub fn new() -> Option<PadBackend> {
        match Gilrs::new() {
            Ok(gilrs) => Some(PadBackend {
                gilrs,
                announced: false,
            }),
            Err(e) => {
                log::warn!("gamepads unavailable: {e}");
                None
            }
        }
    }

    pub fn poll(&mut self, input: &mut Input) {
        if !self.announced {
            self.announced = true;
            for (id, _) in self.gilrs.gamepads() {
                input.pad_connected(usize::from(id));
            }
        }
        while let Some(event) = self.gilrs.next_event() {
            let pad = usize::from(event.id);
            match event.event {
                EventType::Connected => input.pad_connected(pad),
                EventType::Disconnected => input.pad_disconnected(pad),
                EventType::ButtonPressed(b, _) => {
                    if let Some(b) = map_button(b) {
                        input.set_pad_button(pad, b, true);
                    }
                }
                EventType::ButtonReleased(b, _) => {
                    if let Some(b) = map_button(b) {
                        input.set_pad_button(pad, b, false);
                    }
                }
                EventType::AxisChanged(a, value, _) => {
                    if let Some(a) = map_axis(a) {
                        input.set_pad_axis(pad, a, value);
                    }
                }
                _ => {}
            }
        }
    }
}

fn map_button(b: Button) -> Option<PadButton> {
    Some(match b {
        Button::South => PadButton::South,
        Button::East => PadButton::East,
        Button::West => PadButton::West,
        Button::North => PadButton::North,
        Button::LeftTrigger => PadButton::LeftBumper,
        Button::RightTrigger => PadButton::RightBumper,
        Button::LeftTrigger2 => PadButton::LeftTrigger,
        Button::RightTrigger2 => PadButton::RightTrigger,
        Button::Select => PadButton::Select,
        Button::Start => PadButton::Start,
        Button::LeftThumb => PadButton::LeftStick,
        Button::RightThumb => PadButton::RightStick,
        Button::DPadUp => PadButton::DpadUp,
        Button::DPadDown => PadButton::DpadDown,
        Button::DPadLeft => PadButton::DpadLeft,
        Button::DPadRight => PadButton::DpadRight,
        _ => return None,
    })
}

fn map_axis(a: Axis) -> Option<PadAxis> {
    Some(match a {
        Axis::LeftStickX => PadAxis::LeftX,
        Axis::LeftStickY => PadAxis::LeftY,
        Axis::RightStickX => PadAxis::RightX,
        Axis::RightStickY => PadAxis::RightY,
        _ => return None,
    })
}
