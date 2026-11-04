mod app;

pub use app::{App, AppControl, FrameUi, WindowConfig};

pub mod prelude {
    pub use super::{App, AppControl};
}
