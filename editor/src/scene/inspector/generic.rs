use glam::Vec3;

use super::form::Form;

fn fmt_value(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => format!("\"{s}\""),
        toml::Value::Array(a) if a.iter().any(|x| x.is_table() || x.is_array()) => {
            format!("[{} items]", a.len())
        }
        toml::Value::Array(a) => {
            let parts: Vec<String> = a.iter().map(fmt_value).collect();
            format!("[{}]", parts.join(", "))
        }
        toml::Value::Table(t) => format!("{{{} fields}}", t.len()),
        toml::Value::Float(f) => format!("{f:.2}"),
        other => other.to_string(),
    }
}

fn num(v: &toml::Value) -> Option<f32> {
    v.as_float()
        .or_else(|| v.as_integer().map(|i| i as f64))
        .map(|x| x as f32)
}

fn as_vec3(a: &[toml::Value]) -> Option<Vec3> {
    let n: Vec<f32> = a.iter().filter_map(num).collect();
    (a.len() == 3 && n.len() == 3).then(|| Vec3::new(n[0], n[1], n[2]))
}

fn as_strings(a: &[toml::Value]) -> Option<Vec<String>> {
    a.iter()
        .map(|x| x.as_str().map(String::from))
        .collect::<Option<Vec<_>>>()
}

pub fn edit_value(f: &mut Form, slot: u64, key: &str, v: &toml::Value) -> Option<toml::Value> {
    match v {
        toml::Value::String(s) => {
            let mut s2 = s.clone();
            f.text(slot, key, &mut s2);
            (s2 != *s).then_some(toml::Value::String(s2))
        }
        toml::Value::Integer(i) => {
            let mut x = *i as f32;
            f.number(slot, key, &mut x);
            (x != *i as f32).then(|| {
                if x.fract() == 0.0 {
                    toml::Value::Integer(x as i64)
                } else {
                    toml::Value::Float(x as f64)
                }
            })
        }
        toml::Value::Float(x0) => {
            let mut x = *x0 as f32;
            f.number(slot, key, &mut x);
            (x != *x0 as f32).then_some(toml::Value::Float(x as f64))
        }
        toml::Value::Boolean(b) => {
            let mut b2 = *b;
            f.toggle(slot, key, &mut b2);
            (b2 != *b).then_some(toml::Value::Boolean(b2))
        }
        toml::Value::Array(a) => {
            if let Some(v3) = as_vec3(a) {
                let mut v = v3;
                f.vec3(slot, key, &mut v);
                return (v != v3).then(|| {
                    toml::Value::Array(v.to_array().map(|x| toml::Value::Float(x as f64)).to_vec())
                });
            }
            if key == "lines"
                && let Some(list) = as_strings(a)
            {
                let mut text = list.join("\n");
                f.text_area(slot, key, &mut text);
                let new: Vec<String> = text
                    .lines()
                    .map(str::trim_end)
                    .filter(|l| !l.trim().is_empty())
                    .map(String::from)
                    .collect();
                return (new != list).then(|| {
                    toml::Value::Array(new.into_iter().map(toml::Value::String).collect())
                });
            }
            if let Some(list) = as_strings(a) {
                let mut l = list.clone();
                f.strings(slot, key, &mut l);
                return (l != list)
                    .then(|| toml::Value::Array(l.into_iter().map(toml::Value::String).collect()));
            }
            f.row(key, &fmt_value(v));
            None
        }
        other => {
            f.row(key, &fmt_value(other));
            None
        }
    }
}

pub fn draw_fields(f: &mut Form, slot_base: u64, table: &toml::Table) -> Option<toml::Table> {
    let mut out: Option<toml::Table> = None;
    for (i, (k, v)) in table
        .iter()
        .filter(|(k, _)| k.as_str() != "type")
        .enumerate()
    {
        if let Some(nv) = edit_value(f, slot_base + i as u64, k, v) {
            out.get_or_insert_with(|| table.clone())
                .insert(k.clone(), nv);
        }
    }
    out
}
