use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::ecs::prelude::*;
use crate::render::d3::{Lighting, PostChain, PostPass, PostRegistry};
use crate::scene::GameObjDef;
use crate::scene::file::{SceneFile, read_scene_file, write_scene_file};

use super::def::{LightingDef, MaterialDef, PostPassDef};

#[derive(Resource, Default, Clone)]
pub struct Materials(HashMap<String, MaterialDef>);

impl Materials {
    pub fn get(&self, name: &str) -> Option<&MaterialDef> {
        self.0.get(name)
    }

    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.0.keys().cloned().collect();
        v.sort();
        v
    }
}

pub fn load_materials(dir: &Path) -> Materials {
    let mut map = HashMap::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            match fs::read_to_string(&path).map(|t| toml::from_str::<MaterialDef>(&t)) {
                Ok(Ok(def)) => {
                    map.insert(stem.to_string(), def);
                }
                Ok(Err(e)) => log::error!("material `{}`: {e}", path.display()),
                Err(e) => log::error!("material `{}`: {e}", path.display()),
            }
        }
    }
    Materials(map)
}

#[derive(Resource, Default)]
pub struct PostProfiles(pub HashMap<String, Vec<PostPassDef>>);

#[derive(Deserialize, Default)]
struct PostProfileFile {
    #[serde(default)]
    pass: Vec<PostPassDef>,
}

pub fn load_post_profiles(dir: &Path) -> PostProfiles {
    let mut map = HashMap::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            match fs::read_to_string(&path).map(|t| toml::from_str::<PostProfileFile>(&t)) {
                Ok(Ok(profile)) => {
                    map.insert(stem.to_string(), profile.pass);
                }
                Ok(Err(e)) => log::error!("post profile `{}`: {e}", path.display()),
                Err(e) => log::error!("post profile `{}`: {e}", path.display()),
            }
        }
    }
    PostProfiles(map)
}

fn build_post_chain(world: &mut World, profile: Option<&str>) -> PostChain {
    let Some(name) = profile else {
        return PostChain::default();
    };
    let defs = match world.resource::<PostProfiles>() {
        Ok(p) => p.0.get(name).cloned().unwrap_or_default(),
        Err(_) => return PostChain::default(),
    };
    let mut passes = Vec::new();
    for def in &defs {
        let Ok(reg) = world.resource_mut::<PostRegistry>() else {
            break;
        };
        if let Some(shader) = reg.resolve(&def.shader) {
            let params = def
                .params
                .as_ref()
                .map(|v| reg.pack(shader, v))
                .unwrap_or([0.0; 16]);
            passes.push(PostPass { shader, params });
        }
    }
    PostChain {
        passes: passes.into(),
    }
}

#[derive(Clone, Default)]
pub struct SceneHeader {
    pub lighting: Option<LightingDef>,
    pub post: Option<String>,
}

fn parse_header(table: &toml::Table) -> SceneHeader {
    SceneHeader {
        lighting: table
            .get("lighting")
            .and_then(|v| v.clone().try_into().ok()),
        post: table.get("post").and_then(|v| v.as_str()).map(String::from),
    }
}

pub fn apply_header(world: &mut World, table: &toml::Table) {
    let header = parse_header(table);
    world.insert_resource(Lighting::from(header.lighting.unwrap_or_default()));
    let chain = build_post_chain(world, header.post.as_deref());
    world.insert_resource(chain);
}

pub fn read_scene_header(scene_path: &Path) -> SceneHeader {
    parse_header(&read_scene_file(scene_path).header)
}

pub fn save_scene(scene_path: &Path, objects: &[GameObjDef]) -> std::io::Result<()> {
    save_scene_with(scene_path, &SceneHeader::default(), objects)
}

pub fn save_scene_with(
    scene_path: &Path,
    header: &SceneHeader,
    objects: &[GameObjDef],
) -> std::io::Result<()> {
    let mut table = toml::Table::new();
    if let Some(lighting) = &header.lighting {
        let value = toml::Value::try_from(lighting).map_err(std::io::Error::other)?;
        table.insert("lighting".into(), value);
    }
    if let Some(post) = &header.post {
        table.insert("post".into(), toml::Value::String(post.clone()));
    }
    write_scene_file(
        scene_path,
        &SceneFile {
            header: table,
            objects: objects.to_vec(),
        },
    )
}
