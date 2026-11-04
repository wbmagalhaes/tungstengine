use std::path::Path;
use std::time::SystemTime;

use crate::ecs::prelude::*;
#[cfg(feature = "3d")]
use crate::render::d3::Transition;

use super::spawn::{SceneObject, SpawnRegistry};
use super::{Scenes, load_prefabs, load_scene};

#[derive(Resource, Default)]
pub struct SceneWatch {
    assets: String,
    primed: bool,
    scene: Option<SystemTime>,
    prefabs: Option<SystemTime>,
    materials: Option<SystemTime>,
}

impl SceneWatch {
    pub fn new(assets: &str) -> Self {
        SceneWatch {
            assets: assets.to_string(),
            ..Default::default()
        }
    }
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn dir_mtime(dir: &Path) -> Option<SystemTime> {
    let mut latest = file_mtime(dir);
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(m) = entry.metadata().and_then(|m| m.modified()) {
                latest = Some(latest.map_or(m, |l| l.max(m)));
            }
        }
    }
    latest
}

pub fn load_fresh(world: &mut World, path: &Path) {
    let entities: Vec<Entity> = world.query::<(&SceneObject,)>().map(|(e, _)| e).collect();
    for e in entities {
        world.despawn(e);
    }
    load_scene(world, path);
}

pub fn hot_reload(world: &mut World) -> SysResult {
    let assets = world.resource::<SceneWatch>()?.assets.clone();
    let scene_path = world.resource::<Scenes>()?.current.clone();

    let scene_m = scene_path.as_deref().and_then(file_mtime);
    let prefabs_m = dir_mtime(Path::new(&format!("{assets}/prefabs")));
    let materials_m = dir_mtime(Path::new(&format!("{assets}/materials")));

    let (primed, last_scene, last_prefabs, last_materials) = {
        let w = world.resource::<SceneWatch>()?;
        (w.primed, w.scene, w.prefabs, w.materials)
    };

    if let Ok(w) = world.resource_mut::<SceneWatch>() {
        w.primed = true;
        w.scene = scene_m;
        w.prefabs = prefabs_m;
        w.materials = materials_m;
    }
    if !primed {
        return Ok(());
    }

    let mut reload = scene_m != last_scene;
    if prefabs_m != last_prefabs {
        log::debug!("hot-reload prefabs");
        world.insert_resource(load_prefabs(Path::new(&format!("{assets}/prefabs"))));
        reload = true;
    }
    if materials_m != last_materials {
        log::debug!("hot-reload materials");
        #[cfg(feature = "3d")]
        world.insert_resource(super::load_materials(Path::new(&format!(
            "{assets}/materials"
        ))));
        reload = true;
    }

    if reload && let Some(path) = scene_path {
        log::debug!("hot-reload scene {}", path.display());
        load_fresh(world, &path);
    }
    Ok(())
}

pub fn change_scene(world: &mut World) {
    if world
        .resource::<Scenes>()
        .map(|s| s.pending().is_none())
        .unwrap_or(true)
    {
        return;
    }

    if !transition_covered(world) {
        return;
    }

    let path = world
        .resource::<Scenes>()
        .ok()
        .and_then(|s| s.pending_path());
    if let Ok(s) = world.resource_mut::<Scenes>() {
        s.clear_pending();
    }
    let Some(path) = path else {
        return;
    };

    let entities: Vec<Entity> = world.query::<(&SceneObject,)>().map(|(e, _)| e).collect();
    for e in entities {
        world.despawn(e);
    }

    let on_change = world
        .resource::<SpawnRegistry>()
        .ok()
        .and_then(|r| r.on_change);
    if let Some(on_change) = on_change {
        on_change(world);
    }

    load_scene(world, &path);
}

#[cfg(not(feature = "3d"))]
fn transition_covered(_world: &mut World) -> bool {
    true
}

#[cfg(feature = "3d")]
fn transition_covered(world: &mut World) -> bool {
    let active = world
        .resource::<Transition>()
        .map(|t| t.active)
        .unwrap_or(false);
    if !active {
        if let Ok(t) = world.resource_mut::<Transition>() {
            t.start();
        }
        return false;
    }

    let covered = world
        .resource::<Transition>()
        .map(|t| t.covered())
        .unwrap_or(true);
    if !covered {
        return false;
    }
    true
}
