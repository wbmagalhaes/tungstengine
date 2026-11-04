use std::path::Path;

use tungstengine::ecs::Transform;
use tungstengine::ecs::prelude::*;
use tungstengine::scene::{self, SceneObject, Scenes};

use crate::{Flags, Ident, Inventory, Player};
use tungstengine::save::SaveRegistry;

pub fn register_defaults(reg: &mut SaveRegistry) {
    reg.register("flags", save_flags, load_flags);
    reg.register("inventory", save_inventory, load_inventory);
    reg.register("scene", save_scene, load_scene);
    reg.register("player", save_player, load_player);
    reg.register("placements", save_placements, load_placements);
}

fn save_scene(world: &World) -> toml::Value {
    let name = world
        .resource::<Scenes>()
        .ok()
        .and_then(|s| s.current_name().map(str::to_string))
        .unwrap_or_default();
    toml::Value::String(name)
}

fn load_scene(world: &mut World, value: &toml::Value) {
    let Some(name) = value.as_str().filter(|n| !n.is_empty()) else {
        return;
    };
    let path = world
        .resource::<Scenes>()
        .ok()
        .and_then(|s| s.path(name).map(Path::to_path_buf));
    if let Some(path) = path {
        scene::load_fresh(world, &path);
    }
}

fn save_placements(world: &World) -> toml::Value {
    let mut table = toml::map::Map::new();
    for (e, ident, t) in world
        .query::<(&Ident, &Transform, &SceneObject)>()
        .map(|(e, i, t, _)| (e, i, t))
    {
        if world.has::<Player>(e) {
            continue;
        }
        let r = t.rotation;
        let arr = [t.position.x, t.position.y, t.position.z, r.x, r.y, r.z, r.w];
        table.insert(
            ident.key.clone(),
            toml::Value::Array(
                arr.into_iter()
                    .map(|v| toml::Value::Float(v as f64))
                    .collect(),
            ),
        );
    }
    toml::Value::Table(table)
}

fn load_placements(world: &mut World, value: &toml::Value) {
    let Some(table) = value.as_table() else {
        return;
    };
    let targets: Vec<(Entity, String)> = world
        .query::<(&Ident, &SceneObject)>()
        .map(|(e, ident, _)| (e, ident.key.clone()))
        .collect();
    for (e, key) in targets {
        let Some(arr) = table.get(&key).and_then(|v| v.as_array()) else {
            continue;
        };
        let f = |i: usize| arr.get(i).and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
        if let Some(t) = world.get_mut::<Transform>(e) {
            t.position = glam::Vec3::new(f(0), f(1), f(2));
            t.rotation = glam::Quat::from_xyzw(f(3), f(4), f(5), f(6)).normalize();
        }
    }
}

fn strings(list: Vec<String>) -> toml::Value {
    toml::Value::Array(list.into_iter().map(toml::Value::String).collect())
}

fn as_strings(value: &toml::Value) -> Vec<String> {
    value
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn save_flags(world: &World) -> toml::Value {
    strings(
        world
            .resource::<Flags>()
            .map(|f| f.all())
            .unwrap_or_default(),
    )
}

fn load_flags(world: &mut World, value: &toml::Value) {
    if let Ok(f) = world.resource_mut::<Flags>() {
        f.load_from(as_strings(value));
    }
}

fn save_inventory(world: &World) -> toml::Value {
    strings(
        world
            .resource::<Inventory>()
            .map(|i| i.items.clone())
            .unwrap_or_default(),
    )
}

fn load_inventory(world: &mut World, value: &toml::Value) {
    if let Ok(inv) = world.resource_mut::<Inventory>() {
        inv.items = as_strings(value);
        inv.selected = None;
    }
}

fn save_player(world: &World) -> toml::Value {
    let pos = world
        .query::<(&Transform, &Player)>()
        .next()
        .map(|(_, t, _)| t.position)
        .unwrap_or_default();
    toml::Value::Array(
        [pos.x, pos.y, pos.z]
            .into_iter()
            .map(|v| toml::Value::Float(v as f64))
            .collect(),
    )
}

fn load_player(world: &mut World, value: &toml::Value) {
    let Some(a) = value.as_array() else {
        return;
    };
    let coord = |i: usize| a.get(i).and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
    let pos = glam::Vec3::new(coord(0), coord(1), coord(2));
    let player = world.query::<(&Player,)>().next().map(|(e, _)| e);
    if let Some(e) = player
        && let Some(t) = world.get_mut::<Transform>(e)
    {
        t.position = pos;
    }
}
