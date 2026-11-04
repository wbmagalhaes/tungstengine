mod actor;
mod runner;
mod state;

pub use actor::{Actor, Actors, Voice, load_actors};
pub use runner::{Presentation, YarnRunner};
pub use state::Dialog;

pub const UNKNOWN_LABEL: &str = "?????";

pub mod prelude {
    pub use super::{Dialog, Presentation};
}
