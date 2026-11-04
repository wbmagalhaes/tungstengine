mod components;
pub mod gfx;
mod post;
mod shader;
mod transition;
pub mod view;

pub use components::{
    Camera, Faces, Hovered, MainCam, Material, Mesh, Outline, RenderSlot, Renderer, Shader,
};
pub use gfx::GfxConfig;
pub use post::{PostChain, PostPass, PostRegistry};
pub use shader::{MATERIAL_FLOATS, ParamSpec, ShaderRegistry};
pub use transition::{Transition, TransitionDef, TransitionMask, TransitionView};
pub use view::{
    CameraView, CursorRay, Lighting, MeshRegistry, OverlayHook, RenderItem, RenderStats, Scene,
    Vertex, propagate_transforms,
};

pub mod prelude {
    pub use super::{
        Camera, CursorRay, Faces, Hovered, Lighting, MainCam, Material, Mesh, MeshRegistry,
        Outline, RenderSlot, RenderStats, Renderer, Shader, propagate_transforms,
    };
}
