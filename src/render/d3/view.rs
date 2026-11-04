use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};

use crate::animation::{Clip, Joint, JointAnim, Skeleton};
use crate::color::Color;
use crate::ecs::prelude::*;

use crate::texture::Texture2D;

use super::components::{Camera, Faces, Material, Mesh, RenderSlot, Shader};

pub fn propagate_transforms(world: &mut World) {
    let ents: Vec<Entity> = world.query::<(&Transform,)>().map(|(e, _)| e).collect();

    let mut order: Vec<(Entity, u32)> = ents
        .iter()
        .map(|&e| {
            let mut depth = 0u32;
            let mut cur = e;
            while let Some(p) = world.get::<Parent>(cur).map(|p| p.0) {
                depth += 1;
                cur = p;
                if depth > 1024 {
                    break;
                }
            }
            (e, depth)
        })
        .collect();
    order.sort_by_key(|&(_, d)| d);

    for (e, _) in order {
        let local = world
            .get::<Transform>(e)
            .map(|t| t.matrix())
            .unwrap_or(Mat4::IDENTITY);
        let parent_world = world
            .get::<Parent>(e)
            .map(|p| p.0)
            .and_then(|p| world.get::<GlobalTransform>(p).map(|g| g.0))
            .unwrap_or(Mat4::IDENTITY);
        world.insert(e, GlobalTransform(parent_world * local));
    }
}

#[derive(Resource, Default, Clone, Copy)]
pub struct CursorRay {
    pub origin: Vec3,
    pub dir: Vec3,
}

#[derive(Resource, Clone, Copy)]
pub struct Lighting {
    pub sun_dir: Vec3,
    pub sun_color: Color,
    pub sun_intensity: f32,
    pub ambient: Color,
    pub sky: Color,
    pub shininess: f32,
    pub specular: f32,
    pub fresnel: f32,
    pub background: Color,
}

impl Default for Lighting {
    fn default() -> Self {
        Lighting {
            sun_dir: Vec3::new(0.35, 1.0, 0.45),
            sun_color: Color::rgb(255, 244, 224),
            sun_intensity: 0.9,
            ambient: Color::rgb(34, 38, 52),
            sky: Color::rgb(128, 140, 168),
            shininess: 28.0,
            specular: 0.7,
            fresnel: 0.18,
            background: Color::rgb(33, 36, 46),
        }
    }
}

pub struct RenderItem {
    pub mesh: Mesh,
    pub model: Mat4,
    pub color: Color,
    pub shader: Shader,
    pub faces: Faces,
    pub metallic: f32,
    pub roughness: f32,
    pub uv_offset: [f32; 2],
    pub uv_scale: [f32; 2],
    pub params: [f32; 16],
    pub cast_shadows: bool,
    pub outline_occlude: bool,
}

pub struct CameraView {
    pub world: Mat4,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for CameraView {
    fn default() -> Self {
        let c = Camera::default();
        CameraView {
            world: Mat4::IDENTITY,
            fov: c.fov,
            near: c.near,
            far: c.far,
        }
    }
}

#[derive(Resource, Default, Clone, Copy)]
pub struct RenderStats {
    pub draw_calls: u32,
    pub instances: u32,
    pub triangles: u32,
    pub vram_bytes: u64,
}

#[derive(Default)]
pub struct Scene {
    pub camera: CameraView,
    pub items: Vec<RenderItem>,
    pub outlines: Vec<RenderItem>,
    pub outline_color: [f32; 4],
    pub outline_width: f32,
    pub lighting: Lighting,
    pub time: f32,
    pub post: Vec<super::post::PostPass>,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

impl Vertex {
    fn new(position: [f32; 3], normal: [f32; 3]) -> Self {
        Vertex {
            position,
            normal,
            color: [1.0, 1.0, 1.0, 1.0],
            uv: [0.0, 0.0],
        }
    }
}

fn quad(v: &mut Vec<Vertex>, i: &mut Vec<u32>, corners: [[f32; 3]; 4], normal: [f32; 3]) {
    let base = v.len() as u32;
    for &position in &corners {
        v.push(Vertex::new(position, normal));
    }
    i.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn tri(v: &mut Vec<Vertex>, i: &mut Vec<u32>, a: Vec3, b: Vec3, c: Vec3) {
    let normal = (b - a).cross(c - a).normalize().to_array();
    let base = v.len() as u32;
    for p in [a, b, c] {
        v.push(Vertex::new(p.to_array(), normal));
    }
    i.extend_from_slice(&[base, base + 1, base + 2]);
}

fn cube() -> (Vec<Vertex>, Vec<u32>) {
    let mut v = Vec::new();
    let mut i = Vec::new();
    let s = 0.5;
    quad(
        &mut v,
        &mut i,
        [[s, -s, -s], [s, s, -s], [s, s, s], [s, -s, s]],
        [1.0, 0.0, 0.0],
    );
    quad(
        &mut v,
        &mut i,
        [[-s, -s, s], [-s, s, s], [-s, s, -s], [-s, -s, -s]],
        [-1.0, 0.0, 0.0],
    );
    quad(
        &mut v,
        &mut i,
        [[-s, s, -s], [-s, s, s], [s, s, s], [s, s, -s]],
        [0.0, 1.0, 0.0],
    );
    quad(
        &mut v,
        &mut i,
        [[-s, -s, s], [-s, -s, -s], [s, -s, -s], [s, -s, s]],
        [0.0, -1.0, 0.0],
    );
    quad(
        &mut v,
        &mut i,
        [[s, -s, s], [s, s, s], [-s, s, s], [-s, -s, s]],
        [0.0, 0.0, 1.0],
    );
    quad(
        &mut v,
        &mut i,
        [[-s, -s, -s], [-s, s, -s], [s, s, -s], [s, -s, -s]],
        [0.0, 0.0, -1.0],
    );
    (v, i)
}

fn pyramid() -> (Vec<Vertex>, Vec<u32>) {
    let mut v = Vec::new();
    let mut i = Vec::new();
    let s = 0.5;
    let apex = Vec3::new(0.0, 0.5, 0.0);
    let b0 = Vec3::new(-s, -0.5, -s);
    let b1 = Vec3::new(s, -0.5, -s);
    let b2 = Vec3::new(s, -0.5, s);
    let b3 = Vec3::new(-s, -0.5, s);
    quad(
        &mut v,
        &mut i,
        [b0.to_array(), b1.to_array(), b2.to_array(), b3.to_array()],
        [0.0, -1.0, 0.0],
    );
    tri(&mut v, &mut i, b0, apex, b1);
    tri(&mut v, &mut i, b1, apex, b2);
    tri(&mut v, &mut i, b2, apex, b3);
    tri(&mut v, &mut i, b3, apex, b0);
    (v, i)
}

fn midpoint(a: u32, b: u32, pos: &mut Vec<Vec3>, cache: &mut HashMap<(u32, u32), u32>) -> u32 {
    let key = if a < b { (a, b) } else { (b, a) };
    if let Some(&m) = cache.get(&key) {
        return m;
    }
    let m = ((pos[a as usize] + pos[b as usize]) * 0.5).normalize();
    let idx = pos.len() as u32;
    pos.push(m);
    cache.insert(key, idx);
    idx
}

fn sphere() -> (Vec<Vertex>, Vec<u32>) {
    let t = (1.0 + 5.0f32.sqrt()) / 2.0;
    let mut pos: Vec<Vec3> = [
        [-1.0, t, 0.0],
        [1.0, t, 0.0],
        [-1.0, -t, 0.0],
        [1.0, -t, 0.0],
        [0.0, -1.0, t],
        [0.0, 1.0, t],
        [0.0, -1.0, -t],
        [0.0, 1.0, -t],
        [t, 0.0, -1.0],
        [t, 0.0, 1.0],
        [-t, 0.0, -1.0],
        [-t, 0.0, 1.0],
    ]
    .iter()
    .map(|c| Vec3::from_array(*c).normalize())
    .collect();
    let mut faces: Vec<[u32; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];

    for _ in 0..2 {
        let mut cache = HashMap::new();
        let mut next = Vec::with_capacity(faces.len() * 4);
        for f in &faces {
            let a = midpoint(f[0], f[1], &mut pos, &mut cache);
            let b = midpoint(f[1], f[2], &mut pos, &mut cache);
            let c = midpoint(f[2], f[0], &mut pos, &mut cache);
            next.push([f[0], a, c]);
            next.push([f[1], b, a]);
            next.push([f[2], c, b]);
            next.push([a, b, c]);
        }
        faces = next;
    }

    let v: Vec<Vertex> = pos
        .iter()
        .map(|p| Vertex::new((*p * 0.5).to_array(), p.to_array()))
        .collect();
    let i: Vec<u32> = faces.iter().flatten().copied().collect();
    (v, i)
}

fn disk() -> (Vec<Vertex>, Vec<u32>) {
    let segments = 32u32;
    let r = 0.5;
    let normal = [0.0, 1.0, 0.0];
    let mut v = vec![Vertex::new([0.0, 0.0, 0.0], normal)];
    for k in 0..=segments {
        let a = k as f32 / segments as f32 * std::f32::consts::TAU;
        v.push(Vertex::new([r * a.cos(), 0.0, r * a.sin()], normal));
    }
    let mut i = Vec::new();
    for k in 1..=segments {
        i.extend_from_slice(&[0, k + 1, k]);
    }
    (v, i)
}

fn cylinder() -> (Vec<Vertex>, Vec<u32>) {
    let segments = 32u32;
    let r = 0.5;
    let h = 0.5;
    let mut v = Vec::new();
    let mut i = Vec::new();

    // top cap (+Y)
    let top_c = v.len() as u32;
    v.push(Vertex::new([0.0, h, 0.0], [0.0, 1.0, 0.0]));
    let top_rim = v.len() as u32;
    for k in 0..=segments {
        let a = k as f32 / segments as f32 * std::f32::consts::TAU;
        v.push(Vertex::new([r * a.cos(), h, r * a.sin()], [0.0, 1.0, 0.0]));
    }
    for k in 0..segments {
        i.extend_from_slice(&[top_c, top_rim + k + 1, top_rim + k]);
    }

    // bottom cap (-Y)
    let bot_c = v.len() as u32;
    v.push(Vertex::new([0.0, -h, 0.0], [0.0, -1.0, 0.0]));
    let bot_rim = v.len() as u32;
    for k in 0..=segments {
        let a = k as f32 / segments as f32 * std::f32::consts::TAU;
        v.push(Vertex::new(
            [r * a.cos(), -h, r * a.sin()],
            [0.0, -1.0, 0.0],
        ));
    }
    for k in 0..segments {
        i.extend_from_slice(&[bot_c, bot_rim + k, bot_rim + k + 1]);
    }

    // side (radial normals)
    let side = v.len() as u32;
    for k in 0..=segments {
        let a = k as f32 / segments as f32 * std::f32::consts::TAU;
        let n = [a.cos(), 0.0, a.sin()];
        v.push(Vertex::new([r * a.cos(), h, r * a.sin()], n));
        v.push(Vertex::new([r * a.cos(), -h, r * a.sin()], n));
    }
    for k in 0..segments {
        let t0 = side + k * 2;
        let b0 = side + k * 2 + 1;
        let t1 = side + (k + 1) * 2;
        let b1 = side + (k + 1) * 2 + 1;
        i.extend_from_slice(&[t0, t1, b0, t1, b1, b0]);
    }
    (v, i)
}

fn capsule() -> (Vec<Vertex>, Vec<u32>) {
    let sectors = 24u32;
    let stacks = 8u32;
    let r = 0.5f32;
    let hc = 0.5f32;

    // rings, top pole -> bottom pole: (y, radius, hemisphere center y)
    let mut rings: Vec<(f32, f32, f32)> = Vec::new();
    for i in 0..=stacks {
        let phi = i as f32 / stacks as f32 * std::f32::consts::FRAC_PI_2;
        rings.push((hc + r * phi.cos(), r * phi.sin(), hc));
    }
    for i in 0..=stacks {
        let phi =
            std::f32::consts::FRAC_PI_2 + i as f32 / stacks as f32 * std::f32::consts::FRAC_PI_2;
        rings.push((-hc + r * phi.cos(), r * phi.sin(), -hc));
    }

    let mut v = Vec::new();
    for &(y, radius, cy) in &rings {
        for j in 0..=sectors {
            let th = j as f32 / sectors as f32 * std::f32::consts::TAU;
            let pos = [radius * th.cos(), y, radius * th.sin()];
            let normal = Vec3::new(pos[0], y - cy, pos[2]).normalize().to_array();
            v.push(Vertex::new(pos, normal));
        }
    }

    let mut i = Vec::new();
    let row_len = sectors + 1;
    for row in 0..(rings.len() as u32 - 1) {
        for j in 0..sectors {
            let a = row * row_len + j;
            let b = a + row_len;
            i.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    (v, i)
}

fn plane() -> (Vec<Vertex>, Vec<u32>) {
    let mut v = Vec::new();
    let mut i = Vec::new();
    let s = 0.5;
    quad(
        &mut v,
        &mut i,
        [[-s, 0.0, -s], [-s, 0.0, s], [s, 0.0, s], [s, 0.0, -s]],
        [0.0, 1.0, 0.0],
    );
    (v, i)
}

pub fn builtin_meshes() -> [(Vec<Vertex>, Vec<u32>); 7] {
    [
        cube(),
        sphere(),
        pyramid(),
        disk(),
        cylinder(),
        capsule(),
        plane(),
    ]
}

pub const BUILTIN_MESH_NAMES: [&str; 7] = [
    "cube", "sphere", "pyramid", "disk", "cylinder", "capsule", "plane",
];

#[derive(Resource, Default)]
pub struct MeshRegistry {
    data: Vec<(Vec<Vertex>, Vec<u32>)>,
    mesh_tex: Vec<Option<usize>>,
    images: Vec<Texture2D>,
    models: HashMap<String, Vec<RenderSlot>>,
    names: HashMap<String, Vec<String>>,
    skeletons: HashMap<String, Arc<Skeleton>>,
    tex_paths: HashMap<String, usize>,
}

impl MeshRegistry {
    pub fn new() -> Self {
        let mut reg = MeshRegistry::default();
        let material = Material::LIT.color(Color::GRAY);
        for (name, (verts, indices)) in BUILTIN_MESH_NAMES.iter().zip(builtin_meshes()) {
            let mesh = reg.push_mesh(verts, indices);
            reg.models.insert(
                name.to_string(),
                vec![RenderSlot {
                    mesh,
                    material,
                    bone: None,
                }],
            );
        }
        reg
    }

    fn push_mesh(&mut self, verts: Vec<Vertex>, indices: Vec<u32>) -> Mesh {
        let mesh = Mesh(self.data.len() as u32);
        self.data.push((verts, indices));
        self.mesh_tex.push(None);
        mesh
    }

    pub fn load(&mut self, path: &str) -> Option<()> {
        if self.models.contains_key(path) {
            return Some(());
        }
        let model = load_glb(path)?;
        let image_base = self.images.len();
        log::debug!(
            "model {path}: {} primitives, {} image(s)",
            model.prims.len(),
            model.images.len()
        );
        for (i, img) in model.images.iter().enumerate() {
            log::debug!(
                "  image {i}: {}x{} ({} KiB)",
                img.width,
                img.height,
                img.rgba.len() / 1024
            );
        }
        self.images.extend(model.images);
        let mut slots = Vec::with_capacity(model.prims.len());
        let mut names = Vec::with_capacity(model.prims.len());
        for prim in model.prims {
            log::debug!(
                "  prim node='{}' material='{}' verts={} tex={:?}",
                prim.node,
                prim.name,
                prim.verts.len(),
                prim.image.map(|i| image_base + i)
            );
            let mesh = self.push_mesh(prim.verts, prim.indices);
            self.mesh_tex[mesh.index()] = prim.image.map(|i| image_base + i);
            slots.push(RenderSlot {
                mesh,
                material: prim.material,
                bone: prim.bone,
            });
            names.push(prim.name);
        }
        self.models.insert(path.to_string(), slots);
        self.names.insert(path.to_string(), names);
        if let Some(skeleton) = model.skeleton {
            log::debug!(
                "  skeleton: {} joints, clips: {}",
                skeleton.joints.len(),
                skeleton
                    .clips
                    .iter()
                    .map(|c| format!("{}({:.2}s)", c.name, c.duration))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            self.skeletons.insert(path.to_string(), Arc::new(skeleton));
        }
        Some(())
    }

    pub fn skeleton(&self, path: &str) -> Option<Arc<Skeleton>> {
        self.skeletons.get(path).cloned()
    }

    pub fn images(&self) -> &[Texture2D] {
        &self.images
    }

    pub fn mesh_textures(&self) -> &[Option<usize>] {
        &self.mesh_tex
    }

    pub fn load_texture(&mut self, path: &str) -> Option<usize> {
        if let Some(&i) = self.tex_paths.get(path) {
            return Some(i);
        }
        let bytes = std::fs::read(path).ok()?;
        let tex = Texture2D::from_png(&bytes)?;
        let i = self.images.len();
        self.images.push(tex);
        self.tex_paths.insert(path.to_string(), i);
        Some(i)
    }

    pub fn set_mesh_texture(&mut self, mesh: Mesh, image: usize) {
        if let Some(slot) = self.mesh_tex.get_mut(mesh.index()) {
            *slot = Some(image);
        }
    }

    pub fn resolve(&self, name: &str) -> Vec<RenderSlot> {
        self.models.get(name).cloned().unwrap_or_else(|| {
            vec![RenderSlot {
                mesh: Mesh::CUBE,
                material: Material::LIT,
                bone: None,
            }]
        })
    }

    pub fn slot_names(&self, name: &str) -> Vec<String> {
        self.names.get(name).cloned().unwrap_or_default()
    }

    pub fn data(&self) -> &[(Vec<Vertex>, Vec<u32>)] {
        &self.data
    }
}

pub struct LoadedPrimitive {
    pub verts: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material: Material,
    pub name: String,
    pub node: String,
    pub image: Option<usize>,
    pub bone: Option<usize>,
}

pub struct LoadedModel {
    pub prims: Vec<LoadedPrimitive>,
    pub images: Vec<Texture2D>,
    pub skeleton: Option<Skeleton>,
}

pub fn load_glb(path: &str) -> Option<LoadedModel> {
    let bytes = std::fs::read(path).ok()?;
    let gltf = gltf::Gltf::from_slice(&bytes).ok()?;
    let blob = gltf.blob.as_deref();
    let buffers: Vec<&[u8]> = gltf
        .buffers()
        .map(|b| match b.source() {
            gltf::buffer::Source::Bin => blob.unwrap_or(&[]),
            gltf::buffer::Source::Uri(_) => &[],
        })
        .collect();

    // node hierarchy: local transforms + parent links + world transforms
    let node_count = gltf.nodes().count();
    let mut local = vec![Mat4::IDENTITY; node_count];
    let mut parent = vec![None; node_count];
    for node in gltf.nodes() {
        local[node.index()] = Mat4::from_cols_array_2d(&node.transform().matrix());
        for child in node.children() {
            parent[child.index()] = Some(node.index());
        }
    }
    let mut world = vec![Mat4::IDENTITY; node_count];
    for (i, slot) in world.iter_mut().enumerate() {
        let mut chain = Vec::new();
        let mut cur = Some(i);
        while let Some(c) = cur {
            chain.push(c);
            cur = parent[c];
        }
        let mut m = Mat4::IDENTITY;
        for &c in chain.iter().rev() {
            m *= local[c];
        }
        *slot = m;
    }

    // animated node set -> compact joint list
    let mut animated: Vec<usize> = Vec::new();
    for anim in gltf.animations() {
        for ch in anim.channels() {
            let n = ch.target().node().index();
            if !animated.contains(&n) {
                animated.push(n);
            }
        }
    }

    animated.sort_unstable();
    let mut is_animated = vec![false; node_count];
    for &n in &animated {
        is_animated[n] = true;
    }

    // order joints parent-before-child: sort by count of animated ancestors
    let depth = |start: usize| -> usize {
        let mut d = 0;
        let mut cur = parent[start];
        while let Some(p) = cur {
            if is_animated[p] {
                d += 1;
            }
            cur = parent[p];
        }
        d
    };
    animated.sort_by_key(|&n| depth(n));

    let mut node_joint = vec![None; node_count];
    for (j, &n) in animated.iter().enumerate() {
        node_joint[n] = Some(j);
    }
    let nearest_joint = |start: usize| -> Option<usize> {
        let mut cur = Some(start);
        while let Some(c) = cur {
            if let Some(j) = node_joint[c] {
                return Some(j);
            }
            cur = parent[c];
        }
        None
    };

    let skeleton = (!animated.is_empty()).then(|| {
        let joints = animated
            .iter()
            .map(|&n| Joint {
                parent: parent[n].and_then(nearest_joint),
                base: parent[n].map(|p| world[p]).unwrap_or(Mat4::IDENTITY),
                bind_local: local[n],
                inv_bind_world: world[n].inverse(),
            })
            .collect();
        let clips = gltf
            .animations()
            .map(|anim| {
                let mut joints: Vec<JointAnim> =
                    (0..animated.len()).map(|_| JointAnim::default()).collect();
                let mut duration = 0.0f32;
                for ch in anim.channels() {
                    let Some(j) = node_joint[ch.target().node().index()] else {
                        continue;
                    };
                    let reader = ch.reader(|b| buffers.get(b.index()).copied());
                    let Some(times) = reader.read_inputs() else {
                        continue;
                    };
                    let times: Vec<f32> = times.collect();
                    if let Some(&last) = times.last() {
                        duration = duration.max(last);
                    }
                    match reader.read_outputs() {
                        Some(gltf::animation::util::ReadOutputs::Translations(it)) => {
                            joints[j].trans = times
                                .iter()
                                .copied()
                                .zip(it.map(Vec3::from_array))
                                .collect();
                        }
                        Some(gltf::animation::util::ReadOutputs::Rotations(it)) => {
                            joints[j].rot = times
                                .iter()
                                .copied()
                                .zip(it.into_f32().map(Quat::from_array))
                                .collect();
                        }
                        Some(gltf::animation::util::ReadOutputs::Scales(it)) => {
                            joints[j].scale = times
                                .iter()
                                .copied()
                                .zip(it.map(Vec3::from_array))
                                .collect();
                        }
                        _ => {}
                    }
                }
                Clip {
                    name: anim.name().unwrap_or("clip").to_string(),
                    duration,
                    joints,
                }
            })
            .collect();
        Skeleton { joints, clips }
    });

    let mut images: Vec<Texture2D> = Vec::new();
    let mut image_index: HashMap<usize, usize> = HashMap::new();
    let mut prims = Vec::new();
    for node in gltf.nodes() {
        let Some(mesh) = node.mesh() else {
            continue;
        };
        let nidx = node.index();
        let wm = world[nidx];
        let normal_mat = glam::Mat3::from_mat4(wm).inverse().transpose();
        let bone = nearest_joint(nidx);
        let node_name = node.name().unwrap_or_default().to_string();
        for prim in mesh.primitives() {
            let reader = prim.reader(|b| buffers.get(b.index()).copied());
            let positions: Vec<[f32; 3]> = match reader.read_positions() {
                Some(p) => p.collect(),
                None => continue,
            };
            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|n| n.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);
            let mat = prim.material();
            let base = mat.pbr_metallic_roughness().base_color_texture();
            let tex_coord = base.as_ref().map(|b| b.tex_coord()).unwrap_or(0);
            let (uv_offset, uv_scale) = base
                .as_ref()
                .and_then(|b| b.texture_transform())
                .map(|t| (t.offset(), t.scale()))
                .unwrap_or(([0.0, 0.0], [1.0, 1.0]));
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(tex_coord)
                .map(|t| t.into_f32().collect())
                .unwrap_or_default();
            let mut verts = Vec::with_capacity(positions.len());
            for (idx, position) in positions.iter().enumerate() {
                let p = wm.transform_point3(Vec3::from_array(*position)).to_array();
                let n = normals.get(idx).copied().unwrap_or([0.0, 1.0, 0.0]);
                let n = (normal_mat * Vec3::from_array(n))
                    .normalize_or_zero()
                    .to_array();
                let mut vert = Vertex::new(p, n);
                vert.uv = uvs.get(idx).copied().unwrap_or([0.0, 0.0]);
                verts.push(vert);
            }
            let indices: Vec<u32> = match reader.read_indices() {
                Some(idx) => idx.into_u32().collect(),
                None => (0..positions.len() as u32).collect(),
            };
            let image = base.and_then(|b| {
                load_image(
                    b.texture().source().source(),
                    &buffers,
                    &mut images,
                    &mut image_index,
                )
            });
            let pbr = mat.pbr_metallic_roughness();
            let material = Material {
                color: Color::from_linear(pbr.base_color_factor()),
                shader: Shader::LIT,
                faces: if mat.double_sided() {
                    Faces::Both
                } else {
                    Faces::Front
                },
                metallic: pbr.metallic_factor(),
                roughness: pbr.roughness_factor(),
                uv_offset,
                uv_scale,
                params: [0.0; 16],
            };
            let name = mat.name().unwrap_or_default().to_string();
            prims.push(LoadedPrimitive {
                verts,
                indices,
                material,
                name,
                node: node_name.clone(),
                image,
                bone,
            });
        }
    }
    Some(LoadedModel {
        prims,
        images,
        skeleton,
    })
}

fn load_image(
    source: gltf::image::Source,
    buffers: &[&[u8]],
    images: &mut Vec<Texture2D>,
    image_index: &mut HashMap<usize, usize>,
) -> Option<usize> {
    let gltf::image::Source::View { view, mime_type } = source else {
        return None;
    };
    if mime_type != "image/png" {
        return None;
    }
    let key = view.index();
    if let Some(&i) = image_index.get(&key) {
        return Some(i);
    }
    let buffer = buffers.get(view.buffer().index()).copied()?;
    let start = view.offset();
    let png = buffer.get(start..start + view.length())?;
    let tex = Texture2D::from_png(png)?;
    let i = images.len();
    images.push(tex);
    image_index.insert(key, i);
    Some(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_world_composes_with_parent() {
        let mut world = World::new();

        let parent = world.spawn();
        world.insert(parent, Transform::from_pos(Vec3::new(10.0, 0.0, 0.0)));

        let child = world.spawn();
        world.insert(child, Transform::from_pos(Vec3::new(5.0, 0.0, 0.0)));
        world.insert(child, Parent(parent));

        propagate_transforms(&mut world);

        let g = world.get::<GlobalTransform>(child).unwrap();
        assert_eq!(g.position(), Vec3::new(15.0, 0.0, 0.0));
    }

    #[test]
    fn nested_world_composes_through_chain() {
        let mut world = World::new();

        let a = world.spawn();
        world.insert(a, Transform::from_pos(Vec3::new(10.0, 0.0, 0.0)));

        let b = world.spawn();
        world.insert(b, Transform::from_pos(Vec3::new(5.0, 0.0, 0.0)));
        world.insert(b, Parent(a));

        let c = world.spawn();
        world.insert(c, Transform::from_pos(Vec3::new(2.0, 0.0, 0.0)));
        world.insert(c, Parent(b));

        propagate_transforms(&mut world);

        let g = world.get::<GlobalTransform>(c).unwrap();
        assert_eq!(g.position(), Vec3::new(17.0, 0.0, 0.0));
    }
}

#[derive(Resource, Clone, Copy)]
pub struct OverlayHook {
    pub scene: fn(&World) -> Option<Scene>,
    pub intro: fn(&World) -> f32,
}

pub fn load_models(reg: &mut MeshRegistry, dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            load_models(reg, &path);
        } else if path.extension().and_then(|e| e.to_str()) == Some("glb")
            && let Some(s) = path.to_str()
            && reg.load(s).is_none()
        {
            log::error!("failed to load model {}", path.display());
        }
    }
}
