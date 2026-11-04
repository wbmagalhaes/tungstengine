use std::collections::BTreeMap;

use glam::{EulerRot, Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ecs::Transform;

#[cfg(feature = "3d")]
pub use super::d3::def::*;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct GameObjDef {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Uuid>,
    #[serde(default, rename = "component", skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<toml::Value>,
    #[serde(default, rename = "child", skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<GameObjDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<String>,
    #[serde(
        default,
        rename = "removed_child",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub removed_children: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefab_child: Option<usize>,
}

pub fn component_type(c: &toml::Value) -> Option<&str> {
    c.get("type").and_then(|v| v.as_str())
}

pub fn apply_override(base: &GameObjDef, ov: &GameObjDef) -> GameObjDef {
    let mut removed = base.removed.clone();
    for r in &ov.removed {
        if !removed.contains(r) {
            removed.push(r.clone());
        }
    }
    GameObjDef {
        id: base.id,
        key: if ov.key.is_empty() {
            base.key.clone()
        } else {
            ov.key.clone()
        },
        name: ov.name.clone().or_else(|| base.name.clone()),
        from: base.from,
        components: merge_components(&base.components, &ov.components, &ov.removed),
        children: apply_child_overrides(&base.children, &ov.children, &ov.removed_children),
        removed,
        removed_children: base.removed_children.clone(),
        prefab_child: None,
    }
}

pub fn apply_child_overrides(
    base: &[GameObjDef],
    overrides: &[GameObjDef],
    removed: &[usize],
) -> Vec<GameObjDef> {
    base.iter()
        .enumerate()
        .filter(|(i, _)| !removed.contains(i))
        .map(
            |(i, b)| match overrides.iter().find(|o| o.prefab_child == Some(i)) {
                Some(o) => apply_override(b, o),
                None => b.clone(),
            },
        )
        .chain(
            overrides
                .iter()
                .filter(|o| o.prefab_child.is_none())
                .cloned(),
        )
        .collect()
}

pub fn merge_components(
    template: &[toml::Value],
    overrides: &[toml::Value],
    removed: &[String],
) -> Vec<toml::Value> {
    let mut used = vec![false; overrides.len()];
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    let mut out = Vec::with_capacity(template.len() + overrides.len());
    for t in template {
        let ty = component_type(t);
        let occurrence = ty.map(|ty| {
            let n = seen.entry(ty).or_insert(0);
            *n += 1;
            *n - 1
        });
        if ty.is_some_and(|ty| removed.iter().any(|r| r == ty)) {
            continue;
        }
        let matched = ty.zip(occurrence).and_then(|(ty, k)| {
            overrides
                .iter()
                .enumerate()
                .filter(|(_, o)| component_type(o) == Some(ty))
                .nth(k)
                .map(|(i, _)| i)
        });
        let Some(i) = matched else {
            out.push(t.clone());
            continue;
        };
        used[i] = true;
        let merged = match (t, &overrides[i]) {
            (toml::Value::Table(base), toml::Value::Table(over)) => {
                let mut base = base.clone();
                for (k, v) in over {
                    base.insert(k.clone(), v.clone());
                }
                toml::Value::Table(base)
            }
            (_, o) => o.clone(),
        };
        out.push(merged);
    }
    out.extend(
        overrides
            .iter()
            .zip(&used)
            .filter(|(_, u)| !**u)
            .map(|(o, _)| o.clone()),
    );
    out
}

pub fn make_component<T: Serialize>(tag: &str, def: &T) -> toml::Value {
    let mut table = match toml::Value::try_from(def) {
        Ok(toml::Value::Table(t)) => t,
        _ => toml::map::Map::new(),
    };
    table.insert("type".into(), toml::Value::String(tag.to_string()));
    toml::Value::Table(table)
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct TransformDef {
    #[serde(default)]
    pub position: [f32; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look_at: Option<[f32; 3]>,
}

impl From<TransformDef> for Transform {
    fn from(def: TransformDef) -> Self {
        let pos = Vec3::from_array(def.position);
        let mut t = Transform::from_pos(pos);
        if let Some(s) = def.scale {
            t.scale = Vec3::from_array(s);
        }
        if let Some(target) = def.look_at {
            let m = Mat4::look_at_rh(pos, Vec3::from_array(target), Vec3::Y).inverse();
            let (_, rotation, _) = m.to_scale_rotation_translation();
            t.rotation = rotation;
        } else if let Some(r) = def.rotation {
            t.rotation = Quat::from_euler(
                EulerRot::YXZ,
                r[1].to_radians(),
                r[0].to_radians(),
                r[2].to_radians(),
            );
        }
        t
    }
}

impl GameObjDef {
    pub fn at(from: Option<Uuid>, pos: Vec3, rot: Vec3, scale: Vec3) -> Self {
        let mut obj = GameObjDef::empty("");
        obj.name = None;
        obj.from = from;
        obj.set_transform(pos, rot, scale);
        obj
    }

    pub fn empty(name: &str) -> Self {
        GameObjDef {
            id: Uuid::new_v4(),
            key: String::new(),
            name: Some(name.to_string()),
            from: None,
            components: Vec::new(),
            children: Vec::new(),
            removed: Vec::new(),
            removed_children: Vec::new(),
            prefab_child: None,
        }
    }

    pub fn set_transform(&mut self, pos: Vec3, rot: Vec3, scale: Vec3) {
        let prev = self.transform();
        let t = make_component(
            "transform",
            &TransformDef {
                position: pos.to_array(),
                rotation: (rot != Vec3::ZERO).then(|| rot.to_array()),
                scale: (scale != Vec3::ONE).then(|| scale.to_array()),
                look_at: prev.look_at.filter(|_| rot == Vec3::ZERO),
            },
        );
        match self
            .components
            .iter_mut()
            .find(|c| component_type(c) == Some("transform"))
        {
            Some(c) => *c = t,
            None => self.components.insert(0, t),
        }
    }

    pub fn component(&self, tag: &str) -> Option<&toml::Value> {
        self.components
            .iter()
            .find(|c| component_type(c) == Some(tag))
    }

    pub fn transform(&self) -> TransformDef {
        self.component("transform")
            .and_then(|v| v.clone().try_into().ok())
            .unwrap_or_default()
    }
}
