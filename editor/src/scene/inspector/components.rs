use std::collections::{HashMap, HashSet};

use tungstengine::ecs::prelude::*;
use tungstengine::physics::Collider;
use tungstengine::scene::def::ColliderDef;
use tungstengine::scene::{GameObjDef, component_type};

use super::form::Form;
use super::{collider, generic, interactable};
use crate::scene::node::{self, NodeRef};
use crate::scene::state::EditorState;

const SLOT_BASE: u64 = 2000;
const SLOTS_PER_VIEW: u64 = 64;

pub struct View {
    ty: String,
    table: toml::Table,
    base: toml::Table,
    template: Option<toml::Table>,
    own: Option<usize>,
    original: toml::Table,
    defaulted: HashSet<String>,
    removed: bool,
}

pub enum CompAction {
    Edit(usize, toml::Table),
    Revert(usize, String),
    RemoveTemplate(String),
    Restore(String),
    RemoveOwn(usize),
}

fn table(c: &toml::Value) -> toml::Table {
    c.as_table().cloned().unwrap_or_default()
}

fn is_hidden(c: &toml::Value) -> bool {
    component_type(c) == Some("transform")
}

fn derived_defaults(ty: &str, merged: &mut toml::Table, defaulted: &HashSet<String>) {
    if ty == "interactable"
        && defaulted.contains("pick")
        && let Some(r) = merged.get("radius").cloned()
    {
        merged.insert("pick".into(), r);
    }
}

fn layered(defaults: &toml::Table, over: &[&toml::Table]) -> (toml::Table, HashSet<String>) {
    let mut merged = defaults.clone();
    let mut written = HashSet::new();
    for t in over {
        for (k, v) in t.iter() {
            merged.insert(k.clone(), v.clone());
            written.insert(k.clone());
        }
    }
    let defaulted = defaults
        .keys()
        .filter(|k| !written.contains(*k))
        .cloned()
        .collect();
    (merged, defaulted)
}

pub fn views(
    d: &GameObjDef,
    template: Option<&GameObjDef>,
    defaults: &dyn Fn(&str) -> toml::Table,
) -> Vec<View> {
    let mut out = Vec::new();
    let mut used = HashSet::new();
    if let Some(t) = template {
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for tc in t.components.iter().filter(|c| !is_hidden(c)) {
            let ty = component_type(tc).unwrap_or("?");
            let k = *seen.entry(ty).and_modify(|n| *n += 1).or_insert(0);
            let own = d
                .components
                .iter()
                .enumerate()
                .filter(|(_, c)| component_type(c) == Some(ty))
                .nth(k)
                .map(|(i, _)| i);
            let removed = d.removed.iter().any(|r| r == ty);
            let own = own.filter(|_| !removed);
            let tt = table(tc);
            let original = own.map(|i| table(&d.components[i])).unwrap_or_default();
            if let Some(i) = own {
                used.insert(i);
            }
            let defs = defaults(ty);
            let (mut base, base_defaulted) = layered(&defs, &[&tt]);
            derived_defaults(ty, &mut base, &base_defaulted);
            let (mut merged, defaulted) = layered(&defs, &[&tt, &original]);
            derived_defaults(ty, &mut merged, &defaulted);
            out.push(View {
                ty: ty.to_string(),
                table: merged,
                base,
                template: Some(tt),
                own,
                original,
                defaulted,
                removed,
            });
        }
    }
    for (i, c) in d.components.iter().enumerate() {
        if used.contains(&i) || is_hidden(c) {
            continue;
        }
        let ty = component_type(c).unwrap_or("?");
        if template.is_some() && d.removed.iter().any(|r| r == ty) {
            continue;
        }
        let original = table(c);
        let defs = defaults(ty);
        let (mut merged, defaulted) = layered(&defs, &[&original]);
        derived_defaults(ty, &mut merged, &defaulted);
        let mut base = defs;
        derived_defaults(ty, &mut base, &defaulted);
        if let Some(r) = merged
            .get("radius")
            .filter(|_| ty == "interactable" && defaulted.contains("pick"))
        {
            base.insert("pick".into(), r.clone());
        }
        out.push(View {
            ty: ty.to_string(),
            table: merged,
            base,
            template: None,
            own: Some(i),
            original,
            defaulted,
            removed: false,
        });
    }
    out
}

pub fn present_types(views: &[View]) -> Vec<String> {
    views.iter().map(|v| v.ty.clone()).collect()
}

pub fn draw(f: &mut Form, views: &[View], is_instance: bool) -> Option<CompAction> {
    let mut action = None;
    for (vi, v) in views.iter().enumerate() {
        if v.removed {
            if f.section_action(&v.ty, Some("removed"), Some("↺")) {
                action = Some(CompAction::Restore(v.ty.clone()));
            }
            f.gap();
            continue;
        }
        let tag = match (&v.template, is_instance) {
            (Some(_), _) => Some("prefab"),
            (None, true) => Some("added"),
            (None, false) => None,
        };
        if f.component_header(&v.ty, tag, Some("×")) {
            action = Some(match (&v.template, v.own) {
                (Some(_), _) => CompAction::RemoveTemplate(v.ty.clone()),
                (None, Some(i)) => CompAction::RemoveOwn(i),
                (None, None) => continue,
            });
        }
        let overrides = v.template.as_ref().map(|_| {
            v.own
                .map(|_| {
                    v.table
                        .keys()
                        .filter(|k| {
                            k.as_str() != "type"
                                && v.original.contains_key(*k)
                                && v.base.get(*k) != v.table.get(*k)
                        })
                        .cloned()
                        .collect::<HashSet<_>>()
                })
                .unwrap_or_default()
        });
        f.set_overrides(overrides);
        f.set_defaults(Some(v.defaulted.clone()));
        let slot = SLOT_BASE + vi as u64 * SLOTS_PER_VIEW;
        let edited = if v.ty == "collider" {
            collider::draw_fields(f, slot, &v.table)
        } else if v.ty == "interactable" {
            interactable::draw_fields(f, slot, &v.table)
        } else {
            generic::draw_fields(f, slot, &v.table)
        };
        if let Some(key) = f.take_reverted() {
            action = Some(CompAction::Revert(vi, key));
        }
        if let Some(t) = edited {
            action = Some(CompAction::Edit(vi, t));
        }
        f.set_overrides(None);
        f.set_defaults(None);
        f.gap();
    }
    action
}

fn diff(edited: &toml::Table, template: &toml::Table, ty: &str) -> toml::Table {
    let mut out = toml::Table::new();
    out.insert("type".into(), toml::Value::String(ty.into()));
    for (k, v) in edited {
        if k != "type" && template.get(k) != Some(v) {
            out.insert(k.clone(), v.clone());
        }
    }
    out
}

fn keep_written(edited: &toml::Table, original: &toml::Table, base: &toml::Table) -> toml::Table {
    edited
        .iter()
        .filter(|(k, v)| {
            k.as_str() == "type" || original.contains_key(*k) || base.get(*k) != Some(*v)
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn set_own(d: &mut GameObjDef, own: Option<usize>, value: Option<toml::Table>) {
    match (own, value) {
        (Some(i), Some(t)) if i < d.components.len() => d.components[i] = toml::Value::Table(t),
        (Some(i), None) if i < d.components.len() => {
            d.components.remove(i);
        }
        (None, Some(t)) => d.components.push(toml::Value::Table(t)),
        _ => {}
    }
}

pub fn apply(world: &mut World, n: &NodeRef, views: &[View], action: CompAction) {
    let mut live_collider = None;
    let mut respawn = true;
    if let Ok(s) = world.resource_mut::<EditorState>()
        && let Some(d) = node::def_mut(s, n)
    {
        match action {
            CompAction::Edit(vi, edited) => {
                let Some(v) = views.get(vi) else {
                    return;
                };
                let value = match &v.template {
                    Some(_) => Some(diff(&edited, &v.base, &v.ty)).filter(|o| o.len() > 1),
                    None => Some(keep_written(&edited, &v.original, &v.base)),
                };
                set_own(d, v.own, value);
                if v.ty == "collider" {
                    live_collider = Some(edited);
                    respawn = false;
                }
            }
            CompAction::Revert(vi, key) => {
                let Some(v) = views.get(vi) else {
                    return;
                };
                if let Some(i) = v.own
                    && let Some(t) = d.components.get_mut(i).and_then(|c| c.as_table_mut())
                {
                    t.remove(&key);
                    if t.len() <= 1 {
                        d.components.remove(i);
                    }
                }
            }
            CompAction::RemoveTemplate(ty) => {
                d.components
                    .retain(|c| component_type(c) != Some(ty.as_str()));
                if !d.removed.contains(&ty) {
                    d.removed.push(ty);
                }
            }
            CompAction::Restore(ty) => d.removed.retain(|r| *r != ty),
            CompAction::RemoveOwn(i) => {
                if i < d.components.len() {
                    d.components.remove(i);
                }
            }
        }
    }
    if let Some(t) = live_collider {
        if let Some(e) = node::entity(world, n)
            && let Ok(def) = toml::Value::Table(t).try_into::<ColliderDef>()
        {
            world.insert(e, Collider::from(def));
        }
        return;
    }
    if respawn {
        node::respawn(world, n.root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comp(src: &str) -> toml::Value {
        toml::Value::Table(toml::from_str(src).unwrap())
    }

    fn prefab() -> GameObjDef {
        let mut t = GameObjDef::empty("p");
        t.components = vec![
            comp("type = \"collider\"\npad = 0.5\nsize = [1.0, 3.0, 1.0]"),
            comp("type = \"interactable\"\nradius = 1.0"),
        ];
        t
    }

    #[test]
    fn override_merges_and_reports_fields() {
        let mut inst = GameObjDef::empty("i");
        inst.components = vec![comp("type = \"collider\"\npad = 0.2")];
        let v = views(&inst, Some(&prefab()), &|_| toml::Table::new());
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].table.get("pad").and_then(|x| x.as_float()), Some(0.2));
        assert!(v[0].table.contains_key("size"));
        assert_eq!(v[0].own, Some(0));
        assert_eq!(v[1].own, None);
    }

    #[test]
    fn removed_template_component_is_flagged() {
        let mut inst = GameObjDef::empty("i");
        inst.removed = vec!["interactable".into()];
        let v = views(&inst, Some(&prefab()), &|_| toml::Table::new());
        assert!(v[1].removed);
    }

    #[test]
    fn added_component_has_no_template() {
        let mut inst = GameObjDef::empty("i");
        inst.components = vec![comp("type = \"outline\"")];
        let v = views(&inst, Some(&prefab()), &|_| toml::Table::new());
        assert_eq!(v.len(), 3);
        assert!(v[2].template.is_none());
    }

    #[test]
    fn diff_keeps_only_changed_fields() {
        let t = table(&comp("type = \"collider\"\npad = 0.5\nnav = true"));
        let mut e = t.clone();
        e.insert("pad".into(), toml::Value::Float(0.1));
        let d = diff(&e, &t, "collider");
        assert_eq!(d.len(), 2);
        assert!(d.contains_key("pad"));
        assert!(!d.contains_key("nav"));
    }
}
