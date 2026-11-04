mod app;
mod game;
mod headless;

pub use app::{App, AppControl, FrameUi, WindowConfig};
pub use game::{boot, pause_menu, run};
pub use headless::{Headless, ScriptExt, key_code};

pub mod prelude {
    pub use super::{App, AppControl, Headless, WindowConfig};
}
