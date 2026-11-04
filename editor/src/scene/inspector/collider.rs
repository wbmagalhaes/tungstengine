use glam::Vec3;

use super::form::Form;

fn num(v: &toml::Value) -> Option<f32> {
    v.as_float()
        .or_else(|| v.as_integer().map(|i| i as f64))
        .map(|x| x as f32)
}

fn get_vec3(t: &toml::Table, key: &str, default: Vec3) -> Vec3 {
    t.get(key)
        .and_then(|v| v.as_array())
        .and_then(|a| {
            let n: Vec<f32> = a.iter().filter_map(num).collect();
            (n.len() == 3).then(|| Vec3::new(n[0], n[1], n[2]))
        })
        .unwrap_or(default)
}

fn vec3_value(v: Vec3) -> toml::Value {
    toml::Value::Array(v.to_array().map(|x| toml::Value::Float(x as f64)).to_vec())
}

pub fn draw_fields(f: &mut Form, slot_base: u64, t: &toml::Table) -> Option<toml::Table> {
    let size0 = get_vec3(t, "size", Vec3::ONE);
    let offset0 = get_vec3(t, "offset", Vec3::ZERO);
    let rotation0 = get_vec3(t, "rotation", Vec3::ZERO);
    let pad0 = t.get("pad").and_then(num).unwrap_or(0.0);
    let nav0 = t.get("nav").and_then(|v| v.as_bool()).unwrap_or(true);
    let (mut size, mut offset, mut rotation, mut pad, mut nav) =
        (size0, offset0, rotation0, pad0, nav0);

    f.vec3_lockable(slot_base, "size", &mut size);
    f.vec3(slot_base + 1, "offset", &mut offset);
    f.vec3(slot_base + 2, "rotation", &mut rotation);
    f.number(slot_base + 3, "pad", &mut pad);
    f.toggle(slot_base + 4, "nav", &mut nav);

    let mut out: Option<toml::Table> = None;
    let mut set = |k: &str, v: toml::Value| {
        out.get_or_insert_with(|| t.clone()).insert(k.into(), v);
    };
    if size != size0 {
        set("size", vec3_value(size));
    }
    if offset != offset0 {
        set("offset", vec3_value(offset));
    }
    if rotation != rotation0 {
        set("rotation", vec3_value(rotation));
    }
    if pad != pad0 {
        set("pad", toml::Value::Float(pad as f64));
    }
    if nav != nav0 {
        set("nav", toml::Value::Boolean(nav));
    }
    out
}
