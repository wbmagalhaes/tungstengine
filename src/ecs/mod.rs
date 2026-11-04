pub mod components;
mod entity;
mod hash;
mod hierarchy;
mod marker;
mod query;
mod time;
mod world;

pub mod prelude;

pub use components::{Baked, Children, GlobalTransform, Parent, Static, Transform};
pub use ecs_macros::{Component, Resource};
pub use entity::Entity;
pub use hash::{FastHasher, FastMap};
pub use hierarchy::set_parent;
pub use marker::{Component, Resource, ResourceCore};
pub use query::{Query, QueryMut};
pub use time::Time;
pub use world::{SysResult, World, in_state};
