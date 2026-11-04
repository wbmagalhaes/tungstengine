mod change;
#[cfg(feature = "2d")]
pub mod d2;
#[cfg(feature = "3d")]
pub mod d3;
pub mod def;
pub mod file;
pub mod spawn;

pub use change::{SceneWatch, change_scene, hot_reload, load_fresh};

pub use def::{
    GameObjDef, apply_child_overrides, apply_override, component_type, merge_components,
};
pub use file::{
    Prefabs, SceneFile, Scenes, load_prefabs, load_scene, load_scenes, read_scene, read_scene_file,
    write_scene_file,
};
pub use spawn::{
    DeferredSpawns, FinalizeFn, SceneObject, SpawnDecision, SpawnFilter, SpawnFn, SpawnInfo,
    SpawnRegistry, activate_deferred, defaults_of, register_builtin, spawn_gameobj, spawn_object,
};

#[cfg(feature = "3d")]
pub use d3::def::{MESH_NAMES, camera, walk_grid};
#[cfg(feature = "3d")]
pub use d3::spawn::set_mesh;
#[cfg(feature = "3d")]
pub use d3::{
    Materials, PostProfiles, SceneHeader, load_materials, load_post_profiles, read_scene_header,
    save_scene, save_scene_with,
};

pub mod prelude {
    #[cfg(feature = "3d")]
    pub use super::Materials;
    pub use super::{Prefabs, SceneObject, Scenes, load_scene, spawn_gameobj, spawn_object};
}
