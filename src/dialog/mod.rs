mod actor;
mod conversation;
mod runner;
mod state;

pub use actor::{Actor, Actors, PortraitSide, Voice, load_actors, parse_actors};
pub use conversation::conversation;
pub use runner::{Presentation, YarnRunner};
pub use state::Dialog;

pub const UNKNOWN_LABEL: &str = "?????";

pub mod prelude {
    pub use super::{Dialog, Presentation};
}
