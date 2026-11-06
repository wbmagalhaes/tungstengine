extern crate self as tungstengine;

pub use glam as math;

pub mod animation;
pub mod audio;
pub mod clipboard;
pub mod color;
#[cfg(feature = "dialog")]
pub mod dialog;
pub mod ecs;
pub mod fixed_step;
pub mod flags;
pub mod grid;
pub mod input;
pub mod localization;
pub mod manifest;
pub mod menus;
#[cfg(feature = "3d")]
pub mod nav;
pub mod num;
pub mod particles;
pub mod physics;
#[cfg(feature = "3d")]
pub mod raycast;
pub mod render;
pub mod rng;
#[cfg(feature = "3d")]
pub mod rules;
pub mod runtime;
pub mod save;
#[cfg(feature = "scene")]
pub mod scene;
pub mod settings;
pub mod sprites;
pub mod storage;
pub mod texture;
pub mod tilemap;
pub mod ui;

#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen;

#[macro_export]
macro_rules! web_entry {
    ($setup:expr, $frame:expr) => {
        #[cfg(target_arch = "wasm32")]
        #[$crate::wasm_bindgen::prelude::wasm_bindgen(start, wasm_bindgen = $crate::wasm_bindgen)]
        pub fn web_start() {
            $crate::runtime::d2::run($setup, $frame).expect("event loop");
        }
    };
}

#[macro_export]
macro_rules! boot {
    ($world:expr) => {
        $crate::runtime::d2::boot($world, $crate::manifest::embed_game!())
    };
    ($world:expr, $manifest:literal) => {
        $crate::runtime::d2::boot($world, $crate::manifest::embed_game!($manifest))
    };
}

pub mod prelude {
    pub use super::animation::prelude::*;
    pub use super::audio::prelude::*;
    pub use super::color::prelude::*;
    pub use super::ecs::prelude::*;
    pub use super::input::prelude::*;
    pub use super::localization::prelude::*;
    pub use super::math::{Mat4, Quat, Vec2, Vec3};
    pub use super::num::BigNumber;
    pub use super::particles::prelude::*;
    pub use super::physics::d2::prelude::*;
    #[cfg(feature = "3d")]
    pub use super::physics::d3::prelude::*;
    pub use super::render::ClearColor;
    #[cfg(feature = "2d")]
    pub use super::render::d2::prelude::*;
    pub use super::rng::prelude::*;
    pub use super::save::prelude::*;
    pub use super::settings::GameSettings;
    pub use super::sprites::prelude::*;
    pub use super::texture::prelude::*;
    pub use super::ui::prelude::*;
}
