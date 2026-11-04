use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::ecs::prelude::*;

pub const MATERIAL_FLOATS: usize = 16;

pub const UNLIT_GLSL: &str = r#"
vec4 frag(Surface s) {
    vec4 c = s.color * sample_base(s.uv);
    if (c.a < 0.5) { discard; }
    return c;
}
"#;

pub const LIT_GLSL: &str = r#"
vec4 frag(Surface s) {
    s.color *= sample_base(s.uv);
    if (s.color.a < 0.5) { discard; }
    vec3 n = normalize(s.normal);
    vec3 l = normalize(light.sun_dir.xyz);
    vec3 v = normalize(u_cam - s.world);
    vec3 h = normalize(l + v);

    float t = n.y * 0.5 + 0.5;
    vec3 ambient = mix(light.ambient.rgb, light.sky.rgb, t);

    float ndl = max(dot(n, l), 0.0);
    float shadow = shadow_factor(s.world, n, ndl);
    vec3 diffuse = light.sun_color.rgb * light.sun_color.w * ndl * shadow;

    float rough = clamp(s.roughness, 0.04, 1.0);
    float gloss = mix(4.0, 256.0, 1.0 - rough);
    float f0 = mix(0.04, 1.0, s.metallic);
    float spec = pow(max(dot(n, h), 0.0), gloss) * f0 * ndl * shadow;
    vec3 spec_color = mix(light.sun_color.rgb, s.color.rgb, s.metallic);

    vec3 lit = s.color.rgb * (ambient + diffuse) + spec_color * spec * light.sun_color.w;
    return vec4(lit, s.color.a);
}
"#;

const SHADER_COMMON: &str = r#"#version 450
layout(set = 0, binding = 1) uniform Light {
    vec4 sun_dir;
    vec4 sun_color;
    vec4 ambient;
    vec4 sky;
    vec4 cam_pos;
} light;
struct Surface { vec3 normal; vec3 world; vec3 local; vec4 color; vec2 uv; float metallic; float roughness; };
#define u_time (light.cam_pos.w)
#define u_cam (light.cam_pos.xyz)
"#;

const VERTEX_HEAD: &str = r#"
layout(set = 0, binding = 0) uniform Camera { mat4 view_proj; } camera;
layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec4 m0;
layout(location = 3) in vec4 m1;
layout(location = 4) in vec4 m2;
layout(location = 5) in vec4 m3;
layout(location = 6) in vec4 in_color;
layout(location = 8) in vec4 in_vcolor;
layout(location = 9) in vec2 in_uv;
layout(location = 10) in vec2 in_mr;
layout(location = 11) in vec4 in_uvt;
layout(location = 0) out vec3 v_normal;
layout(location = 1) out vec4 v_color;
layout(location = 2) out vec3 v_world;
layout(location = 3) out vec3 v_local;
layout(location = 4) out vec2 v_uv;
layout(location = 5) out vec2 v_mr;
vec4 sample_base(vec2 uv) { return vec4(1.0); }
float shadow_factor(vec3 world, vec3 n, float ndl) { return 1.0; }
"#;

const VERTEX_MAIN: &str = r#"
void main() {
    vec3 p = vert(in_position);
    mat4 model = mat4(m0, m1, m2, m3);
    vec4 world = model * vec4(p, 1.0);
    gl_Position = camera.view_proj * world;
    v_normal = normalize((model * vec4(in_normal, 0.0)).xyz);
    v_color = in_color * in_vcolor;
    v_world = world.xyz;
    v_local = in_position;
    v_uv = in_uv * in_uvt.xy + in_uvt.zw;
    v_mr = in_mr;
}
"#;

const FRAGMENT_HEAD: &str = r#"
layout(location = 0) in vec3 v_normal;
layout(location = 1) in vec4 v_color;
layout(location = 2) in vec3 v_world;
layout(location = 3) in vec3 v_local;
layout(location = 4) in vec2 v_uv;
layout(location = 5) in vec2 v_mr;
layout(location = 0) out vec4 frag_out;
layout(location = 1) out vec4 depth_out;
layout(set = 2, binding = 0) uniform texture2D u_basecolor;
layout(set = 2, binding = 1) uniform sampler u_basecolor_sampler;
vec4 sample_base(vec2 uv) {
    return texture(sampler2D(u_basecolor, u_basecolor_sampler), uv);
}
layout(set = 0, binding = 2) uniform Shadow { mat4 light_vp; } shadowm;
layout(set = 0, binding = 3) uniform texture2D shadow_map;
layout(set = 0, binding = 4) uniform sampler shadow_samp;
float shadow_factor(vec3 world, vec3 n, float ndl) {
    float slope = clamp(1.0 - ndl, 0.0, 1.0);
    vec3 wp = world + n * (0.015 + 0.06 * slope);
    vec4 lc = shadowm.light_vp * vec4(wp, 1.0);
    vec3 ndc = lc.xyz / lc.w;
    vec2 uv = ndc.xy * vec2(0.5, -0.5) + 0.5;
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || ndc.z > 1.0) {
        return 1.0;
    }
    float cur = ndc.z - 0.0004;
    vec2 texel = vec2(1.0 / 2048.0);
    float sum = 0.0;
    for (int x = -1; x <= 1; x++) {
        for (int y = -1; y <= 1; y++) {
            float d = texture(sampler2D(shadow_map, shadow_samp), uv + vec2(x, y) * texel).r;
            sum += cur <= d ? 1.0 : 0.0;
        }
    }
    return sum / 9.0;
}
"#;

const FRAGMENT_MAIN: &str = r#"
void main() {
    Surface s;
    s.normal = normalize(v_normal);
    s.world = v_world;
    s.local = v_local;
    s.color = v_color;
    s.uv = v_uv;
    s.metallic = v_mr.x;
    s.roughness = v_mr.y;
    frag_out = frag(s);
    depth_out = vec4(length(v_world - u_cam), 0.0, 0.0, 1.0);
}
"#;

const DEFAULT_VERT: &str = "vec3 vert(vec3 pos) { return pos; }\n";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ParamSpec {
    Scalar(f32),
    Vec(Vec<f32>),
}

impl ParamSpec {
    fn floats(&self) -> Vec<f32> {
        match self {
            ParamSpec::Scalar(x) => vec![*x],
            ParamSpec::Vec(v) => v.clone(),
        }
    }
}

pub struct ParamField {
    name: String,
    offset: u32,
    components: u8,
}

pub fn pack_params(
    fields: &[ParamField],
    mut out: [f32; MATERIAL_FLOATS],
    values: &BTreeMap<String, ParamSpec>,
) -> [f32; MATERIAL_FLOATS] {
    for field in fields {
        if let Some(spec) = values.get(&field.name) {
            let base = (field.offset / 4) as usize;
            for (i, x) in spec
                .floats()
                .iter()
                .take(field.components as usize)
                .enumerate()
            {
                if base + i < MATERIAL_FLOATS {
                    out[base + i] = *x;
                }
            }
        }
    }
    out
}

struct ShaderEntry {
    vertex: String,
    fragment: String,
    params: Vec<ParamField>,
    path: Option<String>,
    mtime: Option<std::time::SystemTime>,
}

fn file_mtime(path: &str) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn compile(source: String) -> (String, String, Vec<ParamField>) {
    let (vertex, fragment) = if source.contains("#version") {
        (source.clone(), source)
    } else {
        let body = if source.contains("vert(") {
            source
        } else {
            format!("{DEFAULT_VERT}{source}")
        };
        (
            format!("{SHADER_COMMON}{VERTEX_HEAD}{body}{VERTEX_MAIN}"),
            format!("{SHADER_COMMON}{FRAGMENT_HEAD}{body}{FRAGMENT_MAIN}"),
        )
    };
    let params = reflect_params(&fragment);
    (vertex, fragment, params)
}

#[derive(Resource)]
pub struct ShaderRegistry {
    entries: Vec<ShaderEntry>,
    by_name: HashMap<String, u32>,
}

impl Default for ShaderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ShaderRegistry {
    pub fn new() -> Self {
        let mut reg = ShaderRegistry {
            entries: Vec::new(),
            by_name: HashMap::new(),
        };
        reg.intern("unlit", UNLIT_GLSL.to_string());
        reg.intern("lit", LIT_GLSL.to_string());
        reg
    }

    fn intern(&mut self, name: &str, source: String) -> u32 {
        if let Some(&handle) = self.by_name.get(name) {
            return handle;
        }
        let (vertex, fragment, params) = compile(source);
        let path = name.ends_with(".glsl").then(|| name.to_string());
        let mtime = path.as_deref().and_then(file_mtime);
        let handle = self.entries.len() as u32;
        self.entries.push(ShaderEntry {
            vertex,
            fragment,
            params,
            path,
            mtime,
        });
        self.by_name.insert(name.to_string(), handle);
        handle
    }

    pub fn reload_changed(&mut self) -> Vec<usize> {
        let mut changed = Vec::new();
        for (i, entry) in self.entries.iter_mut().enumerate() {
            let Some(path) = entry.path.clone() else {
                continue;
            };
            let mtime = file_mtime(&path);
            if mtime == entry.mtime {
                continue;
            }
            entry.mtime = mtime;
            if let Ok(source) = std::fs::read_to_string(&path) {
                let (vertex, fragment, params) = compile(source);
                entry.vertex = vertex;
                entry.fragment = fragment;
                entry.params = params;
                changed.push(i);
            }
        }
        changed
    }

    pub fn resolve(&mut self, name: &str) -> u32 {
        if let Some(&handle) = self.by_name.get(name) {
            return handle;
        }
        if name.ends_with(".glsl") {
            match std::fs::read_to_string(name) {
                Ok(source) => return self.intern(name, source),
                Err(e) => log::error!("shader `{name}`: {e}"),
            }
        }
        *self.by_name.get("lit").unwrap_or(&1)
    }

    pub fn pack(
        &self,
        handle: u32,
        values: &BTreeMap<String, ParamSpec>,
    ) -> [f32; MATERIAL_FLOATS] {
        match self.entries.get(handle as usize) {
            Some(entry) => pack_params(&entry.params, [0.0; MATERIAL_FLOATS], values),
            None => [0.0; MATERIAL_FLOATS],
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn vertex_source(&self, handle: usize) -> &str {
        &self.entries[handle].vertex
    }

    pub fn fragment_source(&self, handle: usize) -> &str {
        &self.entries[handle].fragment
    }

    pub fn has_params(&self, handle: usize) -> bool {
        !self.entries[handle].params.is_empty()
    }
}

pub fn reflect_params(source: &str) -> Vec<ParamField> {
    use wgpu::naga;
    let mut opts = naga::front::glsl::Options::from(naga::ShaderStage::Fragment);
    opts.defines.insert("FRAGMENT".to_string(), "1".to_string());
    let Ok(module) = naga::front::glsl::Frontend::default().parse(&opts, source) else {
        return Vec::new();
    };
    for (_, var) in module.global_variables.iter() {
        if var.space != naga::AddressSpace::Uniform {
            continue;
        }
        let Some(binding) = &var.binding else {
            continue;
        };
        if binding.group != 1 {
            continue;
        }
        if let naga::TypeInner::Struct { members, .. } = &module.types[var.ty].inner {
            return members
                .iter()
                .filter_map(|m| {
                    let name = m.name.clone()?;
                    let components = match &module.types[m.ty].inner {
                        naga::TypeInner::Scalar(_) => 1u8,
                        naga::TypeInner::Vector { size, .. } => *size as u8,
                        _ => return None,
                    };
                    Some(ParamField {
                        name,
                        offset: m.offset,
                        components,
                    })
                })
                .collect();
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::naga;

    fn compiles(src: &str, stage: naga::ShaderStage, define: &str) {
        let mut opts = naga::front::glsl::Options::from(stage);
        opts.defines.insert(define.to_string(), "1".to_string());
        let module = naga::front::glsl::Frontend::default()
            .parse(&opts, src)
            .unwrap_or_else(|e| panic!("glsl parse failed ({define}): {e:?}"));
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator
            .validate(&module)
            .unwrap_or_else(|e| panic!("glsl validation failed ({define}): {e:?}"));
    }

    #[test]
    fn builtins_have_no_params() {
        let reg = ShaderRegistry::new();
        assert_eq!(reg.len(), 2);
        assert!(!reg.has_params(0));
        assert!(!reg.has_params(1));
    }

    #[test]
    fn vert_frag_snippet_is_wrapped_and_compiles() {
        let mut reg = ShaderRegistry::new();
        let handle = reg.intern(
            "snippet",
            "layout(set = 1, binding = 0) uniform Material { vec4 tint; } mat;\n\
             vec3 vert(vec3 p) { return p + vec3(0.0, sin(u_time), 0.0); }\n\
             vec4 frag(Surface s) { return s.color * mat.tint + vec4(u_cam, u_time); }"
                .to_string(),
        );
        assert!(reg.has_params(handle as usize));
        compiles(
            reg.vertex_source(handle as usize),
            naga::ShaderStage::Vertex,
            "VERTEX",
        );
        compiles(
            reg.fragment_source(handle as usize),
            naga::ShaderStage::Fragment,
            "FRAGMENT",
        );
    }

    #[test]
    fn reflects_named_params() {
        let src = r#"
#version 450
#ifdef FRAGMENT
layout(set = 1, binding = 0) uniform Material {
    vec3 tint;
    float speed;
} mat;
layout(location = 0) out vec4 frag;
void main() { frag = vec4(mat.tint * mat.speed, 1.0); }
#endif
"#;
        let fields = reflect_params(src);
        assert_eq!(fields.len(), 2);
        let mut reg = ShaderRegistry::new();
        let handle = reg.intern("test", src.to_string());
        assert!(reg.has_params(handle as usize));
        let mut vals = BTreeMap::new();
        vals.insert("tint".to_string(), ParamSpec::Vec(vec![0.5, 0.25, 0.75]));
        vals.insert("speed".to_string(), ParamSpec::Scalar(2.0));
        let packed = reg.pack(handle, &vals);
        assert_eq!(&packed[0..3], &[0.5, 0.25, 0.75]);
        assert_eq!(packed[3], 2.0);
    }
}
