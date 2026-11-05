mod app;
mod headless;

pub use app::{App, AppControl, FrameUi, WindowConfig, install_core, tick};
pub use headless::Headless;

pub mod prelude {
    pub use super::{App, AppControl};
}
