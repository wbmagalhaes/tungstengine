use std::collections::{BTreeMap, HashMap};

use crate::ecs::prelude::*;

use super::shader::{MATERIAL_FLOATS, ParamField, ParamSpec, pack_params, reflect_params};

pub const FULLSCREEN_VERTEX: &str = r#"
#version 450
layout(location = 0) out vec2 v_uv;
void main() {
    vec2 p = vec2(float((gl_VertexIndex << 1) & 2), float(gl_VertexIndex & 2));
    gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
    v_uv = vec2(p.x, 1.0 - p.y);
}
"#;

pub const PASSTHROUGH_FRAGMENT: &str = r#"
#version 450
layout(set = 0, binding = 0) uniform texture2D scene_tex;
layout(set = 0, binding = 1) uniform sampler scene_samp;
layout(location = 0) in vec2 v_uv;
layout(location = 0) out vec4 frag_out;
void main() {
    frag_out = texture(sampler2D(scene_tex, scene_samp), v_uv);
}
"#;

const POST_PRELUDE: &str = r#"#version 450
layout(set = 0, binding = 0) uniform texture2D scene_tex;
layout(set = 0, binding = 1) uniform sampler scene_samp;
layout(set = 0, binding = 2) uniform Globals { vec4 res_time; vec4 cam; } globals;
layout(set = 0, binding = 3) uniform texture2D depth_tex;
layout(location = 0) in vec2 v_uv;
layout(location = 0) out vec4 frag_out;
#define scene sampler2D(scene_tex, scene_samp)
#define u_resolution (globals.res_time.xy)
#define u_texel (1.0 / globals.res_time.xy)
#define u_time (globals.res_time.z)
#define u_near (globals.cam.x)
#define u_far (globals.cam.y)
float depth_raw(vec2 uv) {
    return texture(sampler2D(depth_tex, scene_samp), uv).r;
}
float depth_linear(vec2 uv) {
    return depth_raw(uv);
}
"#;

const POST_MAIN: &str = r#"
void main() {
    frag_out = post(v_uv);
}
"#;

struct PostEntry {
    fragment: String,
    params: Vec<ParamField>,
    path: String,
    mtime: Option<std::time::SystemTime>,
}

fn post_mtime(path: &str) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

#[derive(Resource, Default)]
pub struct PostRegistry {
    entries: Vec<PostEntry>,
    by_name: HashMap<String, u32>,
}

impl PostRegistry {
    pub fn resolve(&mut self, path: &str) -> Option<u32> {
        if let Some(&handle) = self.by_name.get(path) {
            return Some(handle);
        }
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                log::error!("post shader `{path}`: {e}");
                return None;
            }
        };
        let fragment = format!("{POST_PRELUDE}\n{source}{POST_MAIN}");
        let params = reflect_params(&fragment);
        let handle = self.entries.len() as u32;
        self.entries.push(PostEntry {
            fragment,
            params,
            mtime: post_mtime(path),
            path: path.to_string(),
        });
        self.by_name.insert(path.to_string(), handle);
        Some(handle)
    }

    pub fn reload_changed(&mut self) -> Vec<usize> {
        let mut changed = Vec::new();
        for (i, entry) in self.entries.iter_mut().enumerate() {
            let mtime = post_mtime(&entry.path);
            if mtime == entry.mtime {
                continue;
            }
            entry.mtime = mtime;
            if let Ok(source) = std::fs::read_to_string(&entry.path) {
                let fragment = format!("{POST_PRELUDE}\n{source}{POST_MAIN}");
                entry.params = reflect_params(&fragment);
                entry.fragment = fragment;
                changed.push(i);
            }
        }
        changed
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

    pub fn fragment_source(&self, handle: usize) -> &str {
        &self.entries[handle].fragment
    }

    pub fn has_params(&self, handle: usize) -> bool {
        !self.entries[handle].params.is_empty()
    }
}

#[derive(Clone, Copy)]
pub struct PostPass {
    pub shader: u32,
    pub params: [f32; MATERIAL_FLOATS],
}

#[derive(Resource, Default)]
pub struct PostChain {
    pub passes: Vec<PostPass>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::naga;

    fn validate(src: &str, stage: naga::ShaderStage) {
        let opts = naga::front::glsl::Options::from(stage);
        let m = naga::front::glsl::Frontend::default()
            .parse(&opts, src)
            .unwrap_or_else(|e| panic!("parse: {e:?}"));
        let mut v = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        v.validate(&m).unwrap_or_else(|e| panic!("validate: {e:?}"));
    }

    #[test]
    fn post_snippet_is_wrapped_and_compiles() {
        validate(FULLSCREEN_VERTEX, naga::ShaderStage::Vertex);
        let user = "layout(set = 1, binding = 0) uniform Material { float amount; } mat;\n\
                    vec4 post(vec2 uv) { return texture(scene, uv) * mat.amount + u_time + u_texel.x; }";
        let fragment = format!("{POST_PRELUDE}\n{user}{POST_MAIN}");
        let fields = reflect_params(&fragment);
        assert_eq!(fields.len(), 1);
        validate(&fragment, naga::ShaderStage::Fragment);
    }
}
