mod backdrop;
mod button;
mod checkbox;
mod chip;
mod color_list;
mod color_picker;
mod dropdown;
mod frame;
mod group;
mod panel;
mod response;
mod stepper;
mod tabs;
mod tag_list;
mod text;
mod text_area;
mod text_input;

pub use backdrop::Backdrop;
pub use button::{Button, button, button_outlined, hold_button, repeat_button};
pub use checkbox::Checkbox;
pub use chip::Chip;
pub use color_list::ColorList;
pub use color_picker::ColorPicker;
pub use dropdown::{DropState, Dropdown, MultiSelect};
pub use frame::Frame;
pub use group::Group;
pub use panel::Panel;
pub use response::Response;
pub use stepper::Stepper;
pub use tabs::Tabs;
pub use tag_list::TagList;
pub use text::Text;
pub use text_area::TextArea;
pub use text_input::TextInput;

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
