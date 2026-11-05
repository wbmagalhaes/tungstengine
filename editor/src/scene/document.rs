use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use glam::Vec3;
use uuid::Uuid;

use tungstengine::ecs::prelude::*;
use tungstengine::scene::{self, GameObjDef, SceneHeader};

use super::node::{self, NodeRef};
use super::state::{DocKind, EditorState, PREFABS_DIR, Placed, default_grid, file_name, file_stem};
use super::viewport::{apply_transform, spawn_marker};

fn set_status(world: &mut World, msg: String) {
    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.status = msg;
    }
}

fn clear(world: &mut World) {
    let Ok(s) = world.resource_mut::<EditorState>() else {
        return;
    };
    let ents: Vec<Entity> = s
        .placed
        .iter()
        .map(|p| p.entity)
        .chain(s.markers.values().copied())
        .collect();
    s.placed.clear();
    s.reserved.clear();
    s.markers.clear();
    s.selected = None;
    s.sel_path.clear();
    s.gizmo = None;
    s.expanded.clear();
    s.tree_scroll = 0.0;
    s.paint_mode = false;
    s.dialog_mode = false;
    for e in ents {
        world.despawn(e);
    }
}

fn def_xf(def: &GameObjDef) -> (Vec3, Vec3, Vec3) {
    let t = def.transform();
    (
        Vec3::from_array(t.position),
        t.rotation.map(Vec3::from_array).unwrap_or(Vec3::ZERO),
        t.scale.map(Vec3::from_array).unwrap_or(Vec3::ONE),
    )
}

pub fn spawn_placed(world: &mut World, def: &GameObjDef) -> Option<Placed> {
    let e = scene::spawn_object(world, def)?;
    let (pos, rot, scale) = def_xf(def);
    if !world.has::<Transform>(e) {
        world.insert(e, Transform::default());
        apply_transform(world, e, pos, rot, scale);
    }
    Some(Placed {
        entity: e,
        def: def.clone(),
        mesh: def.mesh(),
        pos,
        rot,
        scale,
    })
}

pub fn empty_placed(world: &mut World, def: GameObjDef) -> Placed {
    let e = world.spawn();
    world.insert(e, Transform::default());
    let (pos, rot, scale) = def_xf(&def);
    apply_transform(world, e, pos, rot, scale);
    Placed {
        entity: e,
        mesh: def.mesh(),
        def,
        pos,
        rot,
        scale,
    }
}

pub fn placed_def(p: &Placed) -> GameObjDef {
    let mut d = p.def.clone();
    if def_xf(&d) != (p.pos, p.rot, p.scale) {
        d.set_transform(p.pos, p.rot, p.scale);
    }
    d
}

fn saved_def(p: &Placed) -> GameObjDef {
    let mut d = placed_def(p);
    node::prune_overrides(&mut d);
    d
}

pub fn open_scene(world: &mut World, path: &Path) {
    let objects = scene::read_scene(path);
    let header = scene::read_scene_header(path);
    clear(world);

    let grid = objects
        .iter()
        .find_map(|o| o.walk_grid())
        .unwrap_or_else(default_grid);
    let mut markers = HashMap::new();
    for cz in 0..grid.rows {
        for cx in 0..grid.cols {
            if grid.is_blocked_cell(cx, cz) {
                markers.insert((cx, cz), spawn_marker(world, &grid, cx, cz));
            }
        }
    }

    let mut placed = Vec::new();
    let mut reserved = Vec::new();
    for o in objects {
        if o.walk_grid().is_some() {
            continue;
        }
        if o.is_camera() {
            reserved.push(o);
            continue;
        }
        match spawn_placed(world, &o) {
            Some(p) => placed.push(p),
            None => reserved.push(o),
        }
    }

    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.status = format!("{}: {} object(s)", file_name(Some(path)), placed.len());
        s.doc = DocKind::Scene;
        s.path = Some(path.to_path_buf());
        s.header = header;
        s.grid = grid;
        s.markers = markers;
        s.placed = placed;
        s.reserved = reserved;
    }
}

pub fn new_scene(world: &mut World) {
    clear(world);
    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.doc = DocKind::Scene;
        s.path = None;
        s.header = SceneHeader::default();
        s.grid = default_grid();
        s.status = "new scene".into();
    }
}

pub fn save_scene(world: &mut World, path: PathBuf) {
    let Ok(s) = world.resource::<EditorState>() else {
        return;
    };
    let objects: Vec<GameObjDef> = s
        .placed
        .iter()
        .map(saved_def)
        .chain(s.reserved.iter().cloned())
        .chain(std::iter::once(scene::walk_grid(&s.grid)))
        .collect();
    let msg = match scene::save_scene_with(&path, &s.header, &objects) {
        Ok(_) => format!(
            "saved to {} ({} obj)",
            file_name(Some(&path)),
            objects.len()
        ),
        Err(e) => format!("error: {e}"),
    };
    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.status = msg;
        s.path = Some(path);
    }
}

pub fn open_prefab(world: &mut World, path: &Path) {
    let def = fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|t| toml::from_str::<GameObjDef>(&t).map_err(|e| e.to_string()));
    let def = match def {
        Ok(d) => d,
        Err(e) => return set_status(world, format!("error: {e}")),
    };
    clear(world);
    let p = spawn_placed(world, &def).unwrap_or_else(|| empty_placed(world, def));
    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.status = format!("prefab: {}", file_name(Some(path)));
        s.doc = DocKind::Prefab;
        s.path = Some(path.to_path_buf());
        s.placed = vec![p];
        s.selected = Some(0);
    }
}

pub fn new_prefab(world: &mut World) {
    clear(world);
    let p = empty_placed(world, GameObjDef::empty("New prefab"));
    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.doc = DocKind::Prefab;
        s.path = None;
        s.placed = vec![p];
        s.selected = Some(0);
        s.status = "new prefab".into();
    }
}

pub fn save_prefab(world: &mut World, path: PathBuf, save_as: bool) {
    let Some(p) = world
        .resource::<EditorState>()
        .ok()
        .and_then(|s| s.placed.first().cloned())
    else {
        return set_status(world, "no object open".into());
    };
    let mut def = saved_def(&p);
    if save_as {
        def.id = Uuid::new_v4();
        def.key = file_stem(Some(&path));
    }
    let saved = toml::to_string_pretty(&def)
        .map_err(|e| e.to_string())
        .and_then(|text| crate::write_atomic(&path, &text));
    if let Err(e) = saved {
        return set_status(world, format!("error: {e}"));
    }
    let registry = scene::load_prefabs(Path::new(PREFABS_DIR));
    let gameobjs = registry.list();
    world.insert_resource(registry);
    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.status = format!("saved to {}", file_name(Some(&path)));
        s.gameobjs = gameobjs;
        s.path = Some(path);
        if let Some(pp) = s.placed.first_mut() {
            pp.def = def;
        }
    }
}

pub fn add_object(world: &mut World, from: Option<Uuid>) {
    let def = match from {
        Some(id) => GameObjDef::instance(Some(id), None, Vec3::ZERO, Vec3::ZERO, Vec3::ONE),
        None => {
            let mut d = GameObjDef::empty("Empty");
            d.set_transform(Vec3::ZERO, Vec3::ZERO, Vec3::ONE);
            d
        }
    };
    let p = spawn_placed(world, &def).unwrap_or_else(|| empty_placed(world, def));
    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.placed.push(p);
        let root = s.placed.len() - 1;
        node::select(s, Some(NodeRef::root(root)));
    }
}

pub fn delete_selected(world: &mut World) {
    let Some((doc, n)) = world
        .resource::<EditorState>()
        .ok()
        .and_then(|s| node::selected(s).map(|n| (s.doc, n)))
    else {
        return;
    };
    if let Some((last, parent_path)) = n.path.split_last() {
        let parent = NodeRef {
            root: n.root,
            path: parent_path.to_vec(),
        };
        if let Ok(s) = world.resource_mut::<EditorState>()
            && let Some(d) = node::def_mut(s, &parent)
            && *last < d.children.len()
        {
            let removed = d.children.remove(*last);
            if let Some(i) = removed.prefab_child
                && !d.removed_children.contains(&i)
            {
                d.removed_children.push(i);
                d.removed_children.sort_unstable();
            }
        }
        node::respawn(world, n.root);
        if let Ok(s) = world.resource_mut::<EditorState>() {
            node::select(s, Some(parent));
        }
        return;
    }
    if doc != DocKind::Scene {
        return;
    }
    let Some(e) = world
        .resource::<EditorState>()
        .ok()
        .and_then(|s| s.placed.get(n.root).map(|p| p.entity))
    else {
        return;
    };
    world.despawn(e);
    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.placed.remove(n.root);
        node::select(s, None);
    }
}
