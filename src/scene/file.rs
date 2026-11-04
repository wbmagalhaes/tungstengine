use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::ecs::prelude::*;

use super::def::{GameObjDef, component_type};

fn find_item_sprite(node: &GameObjDef) -> Option<SpriteSpec> {
    for c in &node.components {
        if component_type(c) == Some("item") {
            return c.get("sprite").and_then(|v| v.clone().try_into().ok());
        }
    }
    node.children.iter().find_map(find_item_sprite)
}
use super::spawn::spawn_object;
use crate::sprites::SpriteSpec;

#[derive(Resource, Default)]
pub struct Scenes {
    map: HashMap<String, PathBuf>,
    pending: Option<String>,
    pub current: Option<PathBuf>,
}

impl Scenes {
    pub fn path(&self, name: &str) -> Option<&Path> {
        self.map.get(name).map(|p| p.as_path())
    }

    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.map.keys().cloned().collect();
        v.sort();
        v
    }

    pub fn current_name(&self) -> Option<&str> {
        let current = self.current.as_ref()?;
        self.map
            .iter()
            .find(|(_, p)| *p == current)
            .map(|(n, _)| n.as_str())
    }

    pub fn request(&mut self, name: &str) {
        self.pending = Some(name.to_string());
    }

    pub fn pending(&self) -> Option<&str> {
        self.pending.as_deref()
    }

    pub fn pending_path(&self) -> Option<PathBuf> {
        self.pending.as_ref().and_then(|n| self.map.get(n).cloned())
    }

    pub fn clear_pending(&mut self) {
        self.pending = None;
    }
}

pub fn load_scenes(path: &Path) -> Scenes {
    match fs::read_to_string(path).map(|t| toml::from_str::<HashMap<String, PathBuf>>(&t)) {
        Ok(Ok(map)) => Scenes {
            map,
            pending: None,
            current: None,
        },
        Ok(Err(e)) => {
            log::error!("scenes `{}`: {e}", path.display());
            Scenes::default()
        }
        Err(e) => {
            log::error!("scenes `{}`: {e}", path.display());
            Scenes::default()
        }
    }
}

#[derive(Resource, Default)]
pub struct Prefabs {
    by_id: HashMap<Uuid, GameObjDef>,
    names: HashMap<String, String>,
    ids: HashMap<String, Uuid>,
}

impl Prefabs {
    pub fn get(&self, id: &Uuid) -> Option<&GameObjDef> {
        self.by_id.get(id)
    }

    pub fn name_of(&self, key: &str) -> Option<&str> {
        self.names.get(key).map(String::as_str)
    }

    pub fn sprite_of(&self, key: &str) -> Option<SpriteSpec> {
        let id = self.ids.get(key)?;
        find_item_sprite(self.by_id.get(id)?)
    }

    pub fn id_of(&self, key: &str) -> Option<Uuid> {
        self.ids.get(key).copied()
    }

    pub fn list(&self) -> Vec<(Uuid, String)> {
        let mut v: Vec<_> = self
            .by_id
            .values()
            .map(|g| (g.id, g.name.clone().unwrap_or_default()))
            .collect();
        v.sort_by(|a, b| a.1.cmp(&b.1));
        v
    }
}

fn register_names(node: &GameObjDef, names: &mut HashMap<String, String>) {
    if let Some(name) = node.name.as_ref().filter(|n| !n.is_empty())
        && !node.key.is_empty()
    {
        names.insert(node.key.clone(), name.clone());
    }
    for child in &node.children {
        register_names(child, names);
    }
}

pub fn load_prefabs(dir: &Path) -> Prefabs {
    let mut by_id = HashMap::new();
    let mut names = HashMap::new();
    let mut ids = HashMap::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            match fs::read_to_string(&path).map(|t| toml::from_str::<GameObjDef>(&t)) {
                Ok(Ok(mut def)) => {
                    if def.key.is_empty() {
                        def.key = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or_default()
                            .to_string();
                    }
                    if ids.contains_key(&def.key) {
                        log::warn!("prefab `{}`: duplicate key `{}`", path.display(), def.key);
                    }
                    ids.insert(def.key.clone(), def.id);
                    register_names(&def, &mut names);
                    by_id.insert(def.id, def);
                }
                Ok(Err(e)) => log::error!("prefab `{}`: {e}", path.display()),
                Err(e) => log::error!("prefab `{}`: {e}", path.display()),
            }
        }
    }
    Prefabs { by_id, names, ids }
}

#[derive(Default)]
pub struct SceneFile {
    pub header: toml::Table,
    pub objects: Vec<GameObjDef>,
}

pub fn read_scene_file(scene_path: &Path) -> SceneFile {
    let Ok(text) = fs::read_to_string(scene_path) else {
        return SceneFile::default();
    };
    let mut header = match toml::from_str::<toml::Table>(&text) {
        Ok(t) => t,
        Err(e) => {
            log::error!("scene `{}`: {e}", scene_path.display());
            return SceneFile::default();
        }
    };
    let objects = header
        .remove("object")
        .map(|v| v.try_into::<Vec<GameObjDef>>())
        .transpose()
        .unwrap_or_else(|e| {
            log::error!("scene `{}`: {e}", scene_path.display());
            None
        })
        .unwrap_or_default();
    SceneFile { header, objects }
}

pub fn read_scene(scene_path: &Path) -> Vec<GameObjDef> {
    read_scene_file(scene_path).objects
}

pub fn write_scene_file(scene_path: &Path, scene: &SceneFile) -> std::io::Result<()> {
    let mut table = scene.header.clone();
    let objects = toml::Value::try_from(&scene.objects).map_err(std::io::Error::other)?;
    table.insert("object".into(), objects);
    let text = toml::to_string_pretty(&table).map_err(std::io::Error::other)?;
    fs::write(scene_path, text)
}

pub fn load_scene(world: &mut World, scene_path: &Path) {
    world.insert_resource(super::spawn::DeferredSpawns::default());
    if let Ok(s) = world.resource_mut::<Scenes>() {
        s.current = Some(scene_path.to_path_buf());
    }
    let scene = read_scene_file(scene_path);
    #[cfg(feature = "3d")]
    super::d3::file::apply_header(world, &scene.header);
    for o in scene.objects {
        spawn_object(world, &o);
    }
}
