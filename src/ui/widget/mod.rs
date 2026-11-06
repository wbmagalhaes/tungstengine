mod accordion;
mod backdrop;
mod badge;
mod button;
mod checkbox;
mod chip;
mod color_list;
mod color_picker;
mod context_menu;
mod dialog;
mod divider;
mod dropdown;
mod frame;
mod group;
mod icon_button;
mod keybind_input;
mod panel;
mod progress_bar;
mod radio_group;
mod response;
mod scroll_view;
mod slider;
mod slot;
mod spinner;
mod stepper;
mod switch;
mod tabs;
mod tag_list;
mod text;
mod text_area;
mod text_input;
mod toast;

pub use accordion::{Accordion, AccordionState};
pub use backdrop::Backdrop;
pub use button::{Button, button, button_outlined, hold_button, repeat_button};
pub use checkbox::Checkbox;
pub use chip::Chip;
pub use color_list::ColorList;
pub use color_picker::ColorPicker;
pub use context_menu::{ContextMenu, ContextState, MenuItem};
pub use dialog::{Dialog, DialogEvent};
pub use divider::Divider;
pub use dropdown::{DropState, Dropdown, MultiSelect};
pub use frame::Frame;
pub use group::Group;
pub use keybind_input::KeybindInput;
pub use panel::Panel;
pub use radio_group::RadioGroup;
pub use response::Response;
pub use scroll_view::{ScrollAxis, ScrollView};
pub use slider::Slider;
pub use slot::{Slot, slot_ghost, slot_pick};
pub use stepper::Stepper;
pub use switch::Switch;
pub use tabs::Tabs;
pub use tag_list::TagList;
pub use text::Text;
pub use text_area::TextArea;
pub use text_input::TextInput;
pub use toast::Toasts;

use super::animation::Anim;
use super::immediate::UI;

pub trait Widget {
    type Out;

    fn render(self, ui: &mut UI) -> Self::Out;

    fn presence(&self) -> Option<Anim> {
        None
    }

    fn presence_key(&self) -> Option<&str> {
        None
    }
}
pub use badge::Badge;
pub use icon_button::{Icon, IconButton};
pub use progress_bar::ProgressBar;
pub use spinner::Spinner;
