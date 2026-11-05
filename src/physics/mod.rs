pub mod d2;
#[cfg(feature = "3d")]
pub mod d3;
#[cfg(feature = "3d")]
mod footprint;
mod shapes2d;
mod solver2d;
#[cfg(feature = "3d")]
mod solver3d;

#[cfg(feature = "3d")]
pub use footprint::*;
