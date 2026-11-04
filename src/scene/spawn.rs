use std::collections::HashMap;

use glam::Vec3;
use uuid::Uuid;

use crate::ecs::prelude::*;
use crate::ecs::set_parent;

use super::def::{
    GameObjDef, TransformDef, apply_child_overrides, component_type, merge_components,
};
use super::file::Prefabs;

#[derive(Component)]
pub struct SceneObject;

#[derive(Resource, Default)]
pub struct DeferredSpawns {
    items: Vec<(GameObjDef, Vec<String>)>,
}

pub type SpawnFn = fn(&mut World, &toml::Value, Entity);
pub type SpawnFilter = fn(&World, &[toml::Value]) -> SpawnDecision;
pub type FinalizeFn = fn(&mut World, Entity, &SpawnInfo);

pub enum SpawnDecision {
    Spawn,
    Skip,
    Defer(Vec<String>),
}

pub struct SpawnInfo<'a> {
    pub key: &'a str,
    pub name: &'a str,
}

#[derive(Resource, Default)]
pub struct SpawnRegistry {
    spawners: HashMap<String, SpawnFn>,
    defaults: HashMap<String, fn() -> toml::Table>,
    pub filter: Option<SpawnFilter>,
    pub finalize: Option<FinalizeFn>,
    pub on_change: Option<fn(&mut World)>,
    pub flags: Option<fn(&World) -> Vec<String>>,
    categories: HashMap<String, String>,
}

impl SpawnRegistry {
    pub fn add(&mut self, tag: &str, f: SpawnFn) {
        self.spawners.insert(tag.to_string(), f);
    }

    pub fn add_with_defaults(&mut self, tag: &str, f: SpawnFn, defaults: fn() -> toml::Table) {
        self.spawners.insert(tag.to_string(), f);
        self.defaults.insert(tag.to_string(), defaults);
    }

    pub fn defaults(&self, tag: &str) -> Option<toml::Table> {
        let mut table = self.defaults.get(tag)?();
        table.remove("type");
        Some(table)
    }

    pub fn categorize(&mut self, category: &str, tags: &[&str]) {
        for t in tags {
            self.categories.insert(t.to_string(), category.to_string());
        }
    }

    pub fn category(&self, tag: &str) -> &str {
        self.categories
            .get(tag)
            .map(String::as_str)
            .unwrap_or("Other")
    }

    pub fn tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self.spawners.keys().cloned().collect();
        tags.sort();
        tags
    }

    fn get(&self, tag: &str) -> Option<SpawnFn> {
        self.spawners.get(tag).copied()
    }
}

pub fn defaults_of<T: Default + serde::Serialize>() -> toml::Table {
    toml::Table::try_from(T::default()).unwrap_or_default()
}

pub fn with_fields(mut table: toml::Table, fields: &[(&str, toml::Value)]) -> toml::Table {
    for (k, v) in fields {
        table.entry(k.to_string()).or_insert_with(|| v.clone());
    }
    table
}

pub fn f32_value(x: f32) -> toml::Value {
    toml::Value::Float(x.to_string().parse().unwrap_or(x as f64))
}

pub fn vec3_value(v: Vec3) -> toml::Value {
    toml::Value::Array(v.to_array().into_iter().map(f32_value).collect())
}

fn transform_defaults() -> toml::Table {
    with_fields(
        defaults_of::<TransformDef>(),
        &[
            ("rotation", vec3_value(Vec3::ZERO)),
            ("scale", vec3_value(Vec3::ONE)),
        ],
    )
}

pub fn register_builtin(reg: &mut SpawnRegistry) {
    reg.categorize("Core", &["transform"]);
    reg.add_with_defaults(
        "transform",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<TransformDef>() {
                w.insert(e, Transform::from(d));
            }
        },
        transform_defaults,
    );
    crate::tilemap::register(reg);
    #[cfg(feature = "2d")]
    super::d2::register(reg);
    #[cfg(feature = "3d")]
    super::d3::spawn::register(reg);
}

pub fn spawn_object(world: &mut World, obj: &GameObjDef) -> Option<Entity> {
    spawn_child(world, obj, None)
}

fn spawn_child(world: &mut World, obj: &GameObjDef, parent: Option<Entity>) -> Option<Entity> {
    let template = obj.from.and_then(|id| {
        world
            .resource::<Prefabs>()
            .ok()
            .and_then(|g| g.get(&id).cloned())
    });
    let name = obj
        .name
        .clone()
        .or_else(|| template.as_ref().and_then(|t| t.name.clone()))
        .unwrap_or_default();
    let key = if !obj.key.is_empty() {
        obj.key.clone()
    } else {
        template.as_ref().map(|t| t.key.clone()).unwrap_or_default()
    };

    let components: Vec<toml::Value> = match &template {
        Some(t) => merge_components(&t.components, &obj.components, &obj.removed),
        None => obj.components.clone(),
    };

    let filter = world
        .resource::<SpawnRegistry>()
        .ok()
        .and_then(|r| r.filter);
    if let Some(filter) = filter {
        match filter(world, &components) {
            SpawnDecision::Skip => return None,
            SpawnDecision::Defer(flags) => {
                if let Ok(d) = world.resource_mut::<DeferredSpawns>() {
                    d.items.push((obj.clone(), flags));
                }
                return None;
            }
            SpawnDecision::Spawn => {}
        }
    }

    let e = world.spawn();
    world.insert(e, SceneObject);
    for raw in &components {
        let Some(tag) = component_type(raw) else {
            continue;
        };
        match world
            .resource::<SpawnRegistry>()
            .ok()
            .and_then(|r| r.get(tag))
        {
            Some(f) => f(world, raw, e),
            None => log::warn!("unknown component type `{tag}` (no spawner registered)"),
        }
    }
    if let Some(p) = parent {
        set_parent(world, e, p);
    }

    let finalize = world
        .resource::<SpawnRegistry>()
        .ok()
        .and_then(|r| r.finalize);
    if let Some(finalize) = finalize {
        finalize(
            world,
            e,
            &SpawnInfo {
                key: &key,
                name: &name,
            },
        );
    }

    let children = match &template {
        Some(t) => apply_child_overrides(&t.children, &obj.children, &obj.removed_children),
        None => obj
            .children
            .iter()
            .filter(|c| c.prefab_child.is_none())
            .cloned()
            .collect(),
    };
    for child in &children {
        spawn_child(world, child, Some(e));
    }

    Some(e)
}

pub fn spawn_gameobj(world: &mut World, from: Uuid, pos: Vec3) -> Option<Entity> {
    let obj = GameObjDef::at(Some(from), pos, Vec3::ZERO, Vec3::ONE);
    spawn_object(world, &obj)
}

pub fn activate_deferred(world: &mut World) -> SysResult {
    let flags_fn = world.resource::<SpawnRegistry>().ok().and_then(|r| r.flags);
    let Some(flags_fn) = flags_fn else {
        return Ok(());
    };
    let flags = flags_fn(world);
    let ready = |show: &[String]| show.iter().all(|f| flags.contains(f));
    let due: Vec<GameObjDef> = {
        let Ok(d) = world.resource::<DeferredSpawns>() else {
            return Ok(());
        };
        d.items
            .iter()
            .filter(|(_, show)| ready(show))
            .map(|(def, _)| def.clone())
            .collect()
    };
    if due.is_empty() {
        return Ok(());
    }
    if let Ok(d) = world.resource_mut::<DeferredSpawns>() {
        d.items.retain(|(_, show)| !ready(show));
    }
    for def in due {
        spawn_object(world, &def);
    }
    Ok(())
}
