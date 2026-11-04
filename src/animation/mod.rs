mod ease;
mod frames;
#[cfg(feature = "3d")]
mod skeletal;
#[cfg(feature = "3d")]
mod tween;

pub use ease::{Ease, Lerp, Tween};
pub use frames::frame_at;
#[cfg(feature = "3d")]
pub use skeletal::{Animator, Clip, Joint, JointAnim, Skeleton, animator_update};
#[cfg(feature = "3d")]
pub use tween::{Animation, OnFinish, animate};

pub mod prelude {
    #[cfg(feature = "3d")]
    pub use super::{Animation, Animator, OnFinish, Skeleton, animate, animator_update};
    pub use super::{Ease, Lerp, Tween, frame_at};
}
