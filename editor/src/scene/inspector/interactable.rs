use std::collections::HashSet;

use tungstengine_adventure::{VERB_ACTIONS, verb_defaults};

use super::form::Form;
use super::generic::edit_value;

const VERB_KEYS: [&str; 3] = ["name", "far", "action"];

fn num(t: &toml::Table, key: &str) -> Option<f32> {
    t.get(key)
        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
        .map(|x| x as f32)
}

fn verbs(t: &toml::Table) -> Vec<toml::Table> {
    t.get("verb")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_table().cloned()).collect())
        .unwrap_or_default()
}

fn verb_name(v: &toml::Table) -> String {
    v.get("name")
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
}

fn verb_action(v: &toml::Table) -> String {
    v.get("action")
        .and_then(|x| x.as_str())
        .unwrap_or("none")
        .to_string()
}

fn with_action(v: &toml::Table, action: &str) -> toml::Table {
    let mut out: toml::Table = v
        .iter()
        .filter(|(k, _)| k.as_str() == "name" || k.as_str() == "far")
        .map(|(k, x)| (k.clone(), x.clone()))
        .collect();
    out.insert("action".into(), toml::Value::String(action.into()));
    for (k, x) in verb_defaults(action) {
        out.insert(k, x);
    }
    out
}

fn draw_verb(f: &mut Form, slot: u64, v: &toml::Table) -> Option<toml::Table> {
    let mut out: Option<toml::Table> = None;
    let set = |out: &mut Option<toml::Table>, k: &str, x: toml::Value| {
        out.get_or_insert_with(|| v.clone()).insert(k.into(), x);
    };

    let name0 = verb_name(v);
    let mut name = name0.clone();
    f.text(slot, "name", &mut name);
    if name != name0 {
        set(&mut out, "name", toml::Value::String(name.clone()));
    }

    let far0 = v
        .get("far")
        .and_then(|x| x.as_bool())
        .unwrap_or(name == "look");
    let mut far = far0;
    f.toggle(slot + 1, "far", &mut far);
    if far != far0 {
        set(&mut out, "far", toml::Value::Boolean(far));
    }

    let action = verb_action(v);
    if let Some(a) = f.choice(slot + 2, "action", &action, &VERB_ACTIONS)
        && a != action
    {
        let base = out.clone().unwrap_or_else(|| v.clone());
        return Some(with_action(&base, &a));
    }

    let params: Vec<(String, toml::Value)> = verb_defaults(&action)
        .into_iter()
        .map(|(k, d)| {
            let cur = v.get(&k).cloned().unwrap_or(d);
            (k, cur)
        })
        .chain(
            v.iter()
                .filter(|(k, _)| {
                    !VERB_KEYS.contains(&k.as_str()) && !verb_defaults(&action).contains_key(*k)
                })
                .map(|(k, x)| (k.clone(), x.clone())),
        )
        .collect();
    for (i, (k, x)) in params.iter().enumerate() {
        if let Some(nx) = edit_value(f, slot + 3 + i as u64, k, x) {
            set(&mut out, k, nx);
        }
    }
    out
}

pub fn draw_fields(f: &mut Form, slot_base: u64, t: &toml::Table) -> Option<toml::Table> {
    let mut out: Option<toml::Table> = None;

    for (i, key) in ["radius", "pick"].iter().enumerate() {
        let v0 = num(t, key).unwrap_or(1.0);
        let mut v = v0;
        f.number(slot_base + i as u64, key, &mut v);
        if v != v0 {
            out.get_or_insert_with(|| t.clone())
                .insert(key.to_string(), toml::Value::Float(v as f64));
        }
    }

    let mut list = verbs(t);
    let mut list_changed = false;
    let mut remove = None;
    let mut seen = HashSet::new();
    f.section_action("verbs", None, None);
    for (i, v) in list.iter_mut().enumerate() {
        let slot = slot_base + 100 + i as u64 * 32;
        let name = verb_name(v);
        let dup = !seen.insert(name.clone());
        let title = if dup {
            format!("{name} (duplicate)")
        } else {
            name
        };
        let (open, rm) = f.fold_row(slot, &title, &verb_action(v));
        if rm {
            remove = Some(i);
        }
        if open {
            f.indent(14.0);
            if let Some(nv) = draw_verb(f, slot + 4, v) {
                *v = nv;
                list_changed = true;
            }
            f.indent(-14.0);
            f.gap();
        }
    }
    if let Some(i) = remove {
        list.remove(i);
        list_changed = true;
    }
    if f.add_button(slot_base + 99, "+ verb") {
        let mut v = toml::Table::new();
        v.insert("name".into(), toml::Value::String("verb".into()));
        list.push(with_action(&v, "none"));
        list_changed = true;
    }
    if list_changed {
        out.get_or_insert_with(|| t.clone()).insert(
            "verb".into(),
            toml::Value::Array(list.into_iter().map(toml::Value::Table).collect()),
        );
    }
    out
}
