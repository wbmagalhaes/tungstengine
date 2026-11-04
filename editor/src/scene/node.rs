use std::collections::HashMap;

use glam::{Mat4, Vec3};

use tungstengine::ecs::prelude::*;
use tungstengine::scene::{GameObjDef, Prefabs, component_type};

use super::document::{empty_placed, placed_def, spawn_placed};
use super::gizmo::{Tool, Xf};
use super::state::EditorState;
use super::viewport::{apply_transform, euler_quat, quat_euler};

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct NodeRef {
    pub root: usize,
    pub path: Vec<usize>,
}

impl NodeRef {
    pub fn root(root: usize) -> Self {
        NodeRef {
            root,
            path: Vec::new(),
        }
    }

    pub fn key(&self) -> Vec<usize> {
        std::iter::once(self.root)
            .chain(self.path.iter().copied())
            .collect()
    }
}

pub fn selected(s: &EditorState) -> Option<NodeRef> {
    s.selected.map(|root| NodeRef {
        root,
        path: s.sel_path.clone(),
    })
}

pub fn select(s: &mut EditorState, n: Option<NodeRef>) {
    s.selected = n.as_ref().map(|n| n.root);
    s.sel_path = n.map(|n| n.path).unwrap_or_default();
    s.gizmo = None;
}

pub fn def<'a>(s: &'a EditorState, n: &NodeRef) -> Option<&'a GameObjDef> {
    let mut d = &s.placed.get(n.root)?.def;
    for &i in &n.path {
        d = d.children.get(i)?;
    }
    Some(d)
}

pub fn def_mut<'a>(s: &'a mut EditorState, n: &NodeRef) -> Option<&'a mut GameObjDef> {
    let mut d = &mut s.placed.get_mut(n.root)?.def;
    for &i in &n.path {
        d = d.children.get_mut(i)?;
    }
    Some(d)
}

pub fn template(world: &World, d: &GameObjDef) -> Option<GameObjDef> {
    let id = d.from?;
    world.resource::<Prefabs>().ok()?.get(&id).cloned()
}

fn parent_of(n: &NodeRef) -> Option<NodeRef> {
    let (_, path) = n.path.split_last()?;
    Some(NodeRef {
        root: n.root,
        path: path.to_vec(),
    })
}

pub fn base(world: &World, n: &NodeRef) -> Option<GameObjDef> {
    let s = world.resource::<EditorState>().ok()?;
    let d = def(s, n)?;
    if let Some(i) = d.prefab_child {
        return base(world, &parent_of(n)?)?.children.get(i).cloned();
    }
    template(world, d)
}

fn child_slot(base: Option<&GameObjDef>, parent: &GameObjDef, j: usize) -> Option<usize> {
    let kept = |upto: usize| {
        (0..upto)
            .filter(|i| !parent.removed_children.contains(i))
            .count()
    };
    let child = parent.children.get(j)?;
    let base_len = base.map(|b| b.children.len()).unwrap_or(0);
    match (child.prefab_child, base) {
        (Some(i), Some(_)) => {
            (!parent.removed_children.contains(&i) && i < base_len).then(|| kept(i))
        }
        (Some(_), None) => None,
        (None, _) => {
            let own_rank = parent.children[..j]
                .iter()
                .filter(|c| c.prefab_child.is_none())
                .count();
            Some(kept(base_len) + own_rank)
        }
    }
}

pub fn entity(world: &World, n: &NodeRef) -> Option<Entity> {
    let s = world.resource::<EditorState>().ok()?;
    let mut e = s.placed.get(n.root)?.entity;
    let mut cur = NodeRef::root(n.root);
    for &j in &n.path {
        let parent = def(s, &cur)?;
        let b = base(world, &cur);
        let slot = child_slot(b.as_ref(), parent, j)?;
        e = *world.get::<Children>(e)?.0.get(slot)?;
        cur.path.push(j);
    }
    Some(e)
}

fn has_transform(d: &GameObjDef) -> bool {
    d.components
        .iter()
        .any(|c| component_type(c) == Some("transform"))
}

pub fn local(world: &World, n: &NodeRef) -> Option<(Vec3, Vec3, Vec3)> {
    let s = world.resource::<EditorState>().ok()?;
    if n.path.is_empty() {
        let p = s.placed.get(n.root)?;
        return Some((p.pos, p.rot, p.scale));
    }
    let d = def(s, n)?;
    let t = if has_transform(d) {
        d.transform()
    } else {
        base(world, n).map(|b| b.transform()).unwrap_or_default()
    };
    Some((
        Vec3::from_array(t.position),
        t.rotation.map(Vec3::from_array).unwrap_or(Vec3::ZERO),
        t.scale.map(Vec3::from_array).unwrap_or(Vec3::ONE),
    ))
}

pub fn set_local(world: &mut World, n: &NodeRef, pos: Vec3, rot: Vec3, scale: Vec3) {
    let e = entity(world, n);
    if let Ok(s) = world.resource_mut::<EditorState>() {
        if n.path.is_empty() {
            if let Some(p) = s.placed.get_mut(n.root) {
                p.pos = pos;
                p.rot = rot;
                p.scale = scale;
            }
        } else if let Some(d) = def_mut(s, n) {
            d.set_transform(pos, rot, scale);
        }
    }
    if let Some(e) = e {
        if !world.has::<Transform>(e) {
            world.insert(e, Transform::default());
        }
        apply_transform(world, e, pos, rot, scale);
    }
}

fn parent_global(world: &World, n: &NodeRef) -> Mat4 {
    let Some(parent) = parent_of(n) else {
        return Mat4::IDENTITY;
    };
    entity(world, &parent)
        .and_then(|e| world.get::<GlobalTransform>(e))
        .map(|g| g.0)
        .unwrap_or(Mat4::IDENTITY)
}

pub fn world_xf(world: &World, n: &NodeRef) -> Option<Xf> {
    let (pos, rot, scale) = local(world, n)?;
    let m = parent_global(world, n)
        * Mat4::from_scale_rotation_translation(scale, euler_quat(rot), pos);
    let (scale, rot, pos) = m.to_scale_rotation_translation();
    Some(Xf { pos, rot, scale })
}

pub fn set_world(world: &mut World, n: &NodeRef, xf: Xf, only: Option<Tool>) {
    let Some((pos0, rot0, scale0)) = local(world, n) else {
        return;
    };
    let m = parent_global(world, n).inverse()
        * Mat4::from_scale_rotation_translation(xf.scale, xf.rot, xf.pos);
    let (scale, rot, pos) = m.to_scale_rotation_translation();
    let rot = quat_euler(rot);
    let (pos, rot, scale) = match only {
        Some(Tool::Move) => (pos, rot0, scale0),
        Some(Tool::Rotate) => (pos0, rot, scale0),
        Some(Tool::Scale) => (pos0, rot0, scale),
        None => (pos, rot, scale),
    };
    set_local(world, n, pos, rot, scale);
}

pub fn respawn(world: &mut World, root: usize) {
    let Some(p) = world
        .resource::<EditorState>()
        .ok()
        .and_then(|s| s.placed.get(root).cloned())
    else {
        return;
    };
    let def = placed_def(&p);
    world.despawn(p.entity);
    let np = spawn_placed(world, &def).unwrap_or_else(|| empty_placed(world, def));
    if let Ok(s) = world.resource_mut::<EditorState>()
        && let Some(slot) = s.placed.get_mut(root)
    {
        *slot = np;
        s.gizmo = None;
    }
}

fn collect(world: &World, n: NodeRef, d: &GameObjDef, out: &mut HashMap<Entity, Vec<usize>>) {
    if let Some(e) = entity(world, &n) {
        out.insert(e, n.path.clone());
    }
    for (i, c) in d.children.iter().enumerate() {
        let mut path = n.path.clone();
        path.push(i);
        collect(world, NodeRef { root: n.root, path }, c, out);
    }
}

pub fn path_of(world: &World, root: usize, target: Entity) -> Vec<usize> {
    let Some(d) = world
        .resource::<EditorState>()
        .ok()
        .and_then(|s| s.placed.get(root).map(|p| p.def.clone()))
    else {
        return Vec::new();
    };
    let mut map = HashMap::new();
    collect(world, NodeRef::root(root), &d, &mut map);
    let mut e = target;
    loop {
        if let Some(p) = map.get(&e) {
            return p.clone();
        }
        match world.get::<Parent>(e) {
            Some(p) => e = p.0,
            None => return Vec::new(),
        }
    }
}

pub fn override_child(world: &mut World, parent: &NodeRef, index: usize) -> Option<NodeRef> {
    let s = world.resource_mut::<EditorState>().ok()?;
    let d = def_mut(s, parent)?;
    let j = match d
        .children
        .iter()
        .position(|c| c.prefab_child == Some(index))
    {
        Some(j) => j,
        None => {
            let mut c = GameObjDef::empty("");
            c.name = None;
            c.prefab_child = Some(index);
            d.children.push(c);
            d.children.len() - 1
        }
    };
    let mut path = parent.path.clone();
    path.push(j);
    Some(NodeRef {
        root: parent.root,
        path,
    })
}

pub fn prune_overrides(d: &mut GameObjDef) {
    for c in &mut d.children {
        prune_overrides(c);
    }
    d.children.retain(|c| {
        c.prefab_child.is_none()
            || c.name.is_some()
            || !c.components.is_empty()
            || !c.children.is_empty()
            || !c.removed.is_empty()
            || !c.removed_children.is_empty()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str) -> GameObjDef {
        GameObjDef::empty(name)
    }

    fn override_of(i: usize) -> GameObjDef {
        let mut c = GameObjDef::empty("");
        c.name = None;
        c.prefab_child = Some(i);
        c
    }

    #[test]
    fn override_children_map_to_template_slots() {
        let mut base = named("base");
        base.children = vec![named("a"), named("b"), named("c")];
        let mut parent = named("inst");
        parent.children = vec![named("own"), override_of(2)];
        assert_eq!(child_slot(Some(&base), &parent, 1), Some(2));
        assert_eq!(child_slot(Some(&base), &parent, 0), Some(3));
    }

    #[test]
    fn removed_template_children_shift_slots() {
        let mut base = named("base");
        base.children = vec![named("a"), named("b"), named("c")];
        let mut parent = named("inst");
        parent.removed_children = vec![0];
        parent.children = vec![override_of(2), named("own"), override_of(0)];
        assert_eq!(child_slot(Some(&base), &parent, 0), Some(1));
        assert_eq!(child_slot(Some(&base), &parent, 1), Some(2));
        assert_eq!(child_slot(Some(&base), &parent, 2), None);
    }

    #[test]
    fn plain_children_without_base() {
        let mut parent = named("p");
        parent.children = vec![named("x"), named("y")];
        assert_eq!(child_slot(None, &parent, 1), Some(1));
    }

    #[test]
    fn prune_drops_empty_overrides_only() {
        let mut d = named("p");
        let mut kept = override_of(1);
        kept.name = Some("renamed".into());
        d.children = vec![override_of(0), kept, named("own")];
        prune_overrides(&mut d);
        assert_eq!(d.children.len(), 2);
        assert_eq!(d.children[0].prefab_child, Some(1));
    }
}
