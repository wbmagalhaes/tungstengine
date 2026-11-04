use std::collections::BTreeMap;

use glam::{EulerRot, Quat, Vec3};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::animation::{Animation, Ease, OnFinish};
use crate::color::Color;
use crate::nav::WalkGrid;
use crate::render::d3::{
    Camera, Faces, Lighting, Material, MeshRegistry, Outline, ParamSpec, RenderSlot, Shader,
    ShaderRegistry,
};

use super::super::def::*;
use super::file::Materials;

fn unit_component(tag: &str) -> toml::Value {
    let mut table = toml::map::Map::new();
    table.insert("type".into(), toml::Value::String(tag.to_string()));
    toml::Value::Table(table)
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PostPassDef {
    pub shader: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<BTreeMap<String, ParamSpec>>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct LightingDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sun_dir: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sun_color: Option<[u8; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sun_intensity: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambient: Option<[u8; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sky: Option<[u8; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shininess: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specular: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fresnel: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<[u8; 3]>,
}

impl From<LightingDef> for Lighting {
    fn from(d: LightingDef) -> Self {
        let base = Lighting::default();
        let rgb = |c: [u8; 3]| Color::rgb(c[0], c[1], c[2]);
        Lighting {
            sun_dir: d.sun_dir.map(Vec3::from).unwrap_or(base.sun_dir),
            sun_color: d.sun_color.map(rgb).unwrap_or(base.sun_color),
            sun_intensity: d.sun_intensity.unwrap_or(base.sun_intensity),
            ambient: d.ambient.map(rgb).unwrap_or(base.ambient),
            sky: d.sky.map(rgb).unwrap_or(base.sky),
            shininess: d.shininess.unwrap_or(base.shininess),
            specular: d.specular.unwrap_or(base.specular),
            fresnel: d.fresnel.unwrap_or(base.fresnel),
            background: d.background.map(rgb).unwrap_or(base.background),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ColorSpec {
    Name(String),
    Rgb([u8; 3]),
}

impl ColorSpec {
    fn to_color(&self) -> Option<Color> {
        match self {
            ColorSpec::Rgb(c) => Some(Color::rgb(c[0], c[1], c[2])),
            ColorSpec::Name(s) => Color::parse(s),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RendererDef {
    pub mesh: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast_shadows: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outline_occlude: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<ColorSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shader: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faces: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<BTreeMap<String, ParamSpec>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materials: Vec<MaterialDef>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct MaterialDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<ColorSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shader: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faces: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metallic: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roughness: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uv_offset: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uv_scale: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<BTreeMap<String, ParamSpec>>,
}

impl MaterialDef {
    fn merged(&self, materials: &Materials) -> MaterialDef {
        let base = self
            .material
            .as_deref()
            .and_then(|n| materials.get(n))
            .cloned()
            .unwrap_or_default();
        MaterialDef {
            name: self.name.clone(),
            material: None,
            color: self.color.clone().or(base.color),
            shader: self.shader.clone().or(base.shader),
            faces: self.faces.clone().or(base.faces),
            metallic: self.metallic.or(base.metallic),
            roughness: self.roughness.or(base.roughness),
            uv_offset: self.uv_offset.or(base.uv_offset),
            uv_scale: self.uv_scale.or(base.uv_scale),
            texture: self.texture.clone().or(base.texture),
            params: self.params.clone().or(base.params),
        }
    }

    fn apply(
        &self,
        base: Material,
        shaders: &mut ShaderRegistry,
        materials: &Materials,
    ) -> Material {
        let def = self.merged(materials);
        let shader = def
            .shader
            .as_deref()
            .map(|s| Shader(shaders.resolve(s)))
            .unwrap_or(base.shader);
        let params = match &def.params {
            Some(values) => shaders.pack(shader.0, values),
            None => base.params,
        };
        Material {
            color: def.color.and_then(|c| c.to_color()).unwrap_or(base.color),
            shader,
            faces: def.faces.as_deref().map(faces_from).unwrap_or(base.faces),
            metallic: def.metallic.unwrap_or(base.metallic),
            roughness: def.roughness.unwrap_or(base.roughness),
            uv_offset: def.uv_offset.unwrap_or(base.uv_offset),
            uv_scale: def.uv_scale.unwrap_or(base.uv_scale),
            params,
        }
    }
}

impl Default for RendererDef {
    fn default() -> Self {
        RendererDef::of_mesh("cube".into())
    }
}

impl RendererDef {
    fn of_mesh(mesh: String) -> Self {
        RendererDef {
            mesh,
            cast_shadows: None,
            outline_occlude: None,
            material: None,
            color: None,
            shader: None,
            faces: None,
            params: None,
            materials: Vec::new(),
        }
    }

    pub(super) fn apply_to(
        &self,
        slots: &mut [RenderSlot],
        names: &[String],
        shaders: &mut ShaderRegistry,
        materials: &Materials,
    ) {
        let top = MaterialDef {
            material: self.material.clone(),
            color: self.color.clone(),
            shader: self.shader.clone(),
            faces: self.faces.clone(),
            metallic: None,
            roughness: None,
            uv_offset: None,
            uv_scale: None,
            texture: None,
            params: self.params.clone(),
            name: None,
        };

        for slot in slots.iter_mut() {
            slot.material = top.apply(slot.material, shaders, materials);
        }

        for (i, def) in self.materials.iter().enumerate() {
            match &def.name {
                Some(want) => {
                    for (si, slot) in slots.iter_mut().enumerate() {
                        if names.get(si).is_some_and(|n| n == want) {
                            slot.material = def.apply(slot.material, shaders, materials);
                        }
                    }
                }
                None => {
                    if let Some(slot) = slots.get_mut(i) {
                        slot.material = def.apply(slot.material, shaders, materials);
                    }
                }
            }
        }
    }

    pub(super) fn apply_textures(
        &self,
        slots: &[RenderSlot],
        names: &[String],
        meshes: &mut MeshRegistry,
        materials: &Materials,
    ) {
        for (i, def) in self.materials.iter().enumerate() {
            let Some(path) = def.merged(materials).texture else {
                continue;
            };
            let Some(img) = meshes.load_texture(&path) else {
                continue;
            };
            match &def.name {
                Some(want) => {
                    for (si, slot) in slots.iter().enumerate() {
                        if names.get(si).is_some_and(|n| n == want) {
                            meshes.set_mesh_texture(slot.mesh, img);
                        }
                    }
                }
                None => {
                    if let Some(slot) = slots.get(i) {
                        meshes.set_mesh_texture(slot.mesh, img);
                    }
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CameraDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fov: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub near: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub far: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smooth: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AnimationDef {
    #[serde(default, rename = "track")]
    tracks: Vec<TrackDef>,
    #[serde(default)]
    looping: bool,
    #[serde(default = "remove_finish")]
    on_finish: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    finish_sound: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TrackDef {
    prop: String,
    #[serde(default)]
    to: [f32; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<[u8; 3]>,
    duration: f32,
    #[serde(default)]
    delay: f32,
    #[serde(default = "default_ease")]
    ease: Ease,
}

impl Default for AnimationDef {
    fn default() -> Self {
        AnimationDef {
            tracks: Vec::new(),
            looping: false,
            on_finish: remove_finish(),
            finish_sound: None,
        }
    }
}

fn remove_finish() -> String {
    "remove".into()
}

fn default_ease() -> Ease {
    Ease::OutQuad
}

impl From<AnimationDef> for Animation {
    fn from(def: AnimationDef) -> Self {
        let mut anim = Animation::new().on_finish(match def.on_finish.as_str() {
            "despawn" => OnFinish::Despawn,
            "keep" => OnFinish::Keep,
            _ => OnFinish::Remove,
        });
        if def.looping {
            anim = anim.looping();
        }
        if let Some(sound) = &def.finish_sound {
            anim = anim.finish_sound(sound);
        }
        for tr in def.tracks {
            let v = Vec3::from_array(tr.to);
            anim = match tr.prop.as_str() {
                "position" => anim.position(v, tr.duration),
                "rotation" => anim.rotation(
                    Quat::from_euler(
                        EulerRot::YXZ,
                        v.y.to_radians(),
                        v.x.to_radians(),
                        v.z.to_radians(),
                    ),
                    tr.duration,
                ),
                "color" => {
                    let c = tr.color.unwrap_or([255, 255, 255]);
                    anim.color(Color::rgb(c[0], c[1], c[2]), tr.duration)
                }
                _ => anim.scale(v, tr.duration),
            };
            anim = anim.ease(tr.ease).delay(tr.delay);
        }
        anim
    }
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ColliderDef {
    #[serde(default)]
    pad: f32,
    #[serde(default = "default_true")]
    nav: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    size: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    offset: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rotation: Option<[f32; 3]>,
}

use crate::physics::Collider;

impl Default for ColliderDef {
    fn default() -> Self {
        ColliderDef {
            pad: 0.0,
            nav: default_true(),
            size: None,
            offset: None,
            rotation: None,
        }
    }
}

impl From<ColliderDef> for Collider {
    fn from(def: ColliderDef) -> Self {
        let base = Collider::default();
        Collider {
            pad: def.pad,
            nav: def.nav,
            size: def.size.map(Vec3::from_array).unwrap_or(base.size),
            offset: def.offset.map(Vec3::from_array).unwrap_or(base.offset),
            rotation: def.rotation.map(Vec3::from_array).unwrap_or(base.rotation),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct OutlineDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<[u8; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    width: Option<f32>,
}

impl From<OutlineDef> for Outline {
    fn from(def: OutlineDef) -> Self {
        let base = Outline::default();
        Outline {
            color: def
                .color
                .map(|c| Color::rgb(c[0], c[1], c[2]))
                .unwrap_or(base.color),
            width: def.width.unwrap_or(base.width),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WalkGridDef {
    cell: f32,
    cols: usize,
    rows: usize,
    origin: [f32; 2],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    blocked: Vec<[usize; 2]>,
}

impl Default for WalkGridDef {
    fn default() -> Self {
        WalkGridDef {
            cell: 1.0,
            cols: 10,
            rows: 10,
            origin: [0.0, 0.0],
            blocked: Vec::new(),
        }
    }
}

impl From<WalkGridDef> for WalkGrid {
    fn from(def: WalkGridDef) -> Self {
        let mut grid = WalkGrid::new(def.cell, def.cols, def.rows, def.origin);
        for [cx, cz] in def.blocked {
            grid.set(cx, cz, true);
        }
        grid
    }
}

impl From<&WalkGrid> for WalkGridDef {
    fn from(g: &WalkGrid) -> Self {
        WalkGridDef {
            cell: g.cell,
            cols: g.cols,
            rows: g.rows,
            origin: g.origin,
            blocked: (0..g.rows)
                .flat_map(|z| (0..g.cols).map(move |x| (x, z)))
                .filter(|&(x, z)| g.authored_blocked(x, z))
                .map(|(x, z)| [x, z])
                .collect(),
        }
    }
}

pub const MESH_NAMES: [&str; 7] = [
    "cube", "sphere", "pyramid", "disk", "cylinder", "capsule", "plane",
];

fn faces_from(s: &str) -> Faces {
    match s {
        "back" => Faces::Back,
        "both" => Faces::Both,
        _ => Faces::Front,
    }
}

impl From<CameraDef> for Camera {
    fn from(def: CameraDef) -> Self {
        let base = Camera::default();
        Camera {
            fov: def.fov.unwrap_or(base.fov),
            near: def.near.unwrap_or(base.near),
            far: def.far.unwrap_or(base.far),
            offset: def.offset.map(Vec3::from_array).unwrap_or(base.offset),
            smooth: def.smooth.unwrap_or(base.smooth),
            zoom: base.zoom,
        }
    }
}

impl GameObjDef {
    pub fn instance(
        from: Option<Uuid>,
        mesh: Option<String>,
        pos: Vec3,
        rot: Vec3,
        scale: Vec3,
    ) -> Self {
        let mut components = vec![make_component(
            "transform",
            &TransformDef {
                position: pos.to_array(),
                rotation: (rot != Vec3::ZERO).then(|| rot.to_array()),
                scale: (scale != Vec3::ONE).then(|| scale.to_array()),
                look_at: None,
            },
        )];
        if let Some(m) = mesh {
            components.push(make_component("renderer", &RendererDef::of_mesh(m)));
        }
        GameObjDef {
            id: Uuid::new_v4(),
            key: String::new(),
            name: None,
            from,
            components,
            children: Vec::new(),
            removed: Vec::new(),
            removed_children: Vec::new(),
            prefab_child: None,
        }
    }

    pub fn set_mesh(&mut self, mesh: &str) {
        match self
            .components
            .iter_mut()
            .find(|c| component_type(c) == Some("renderer"))
        {
            Some(toml::Value::Table(t)) => {
                t.insert("mesh".into(), toml::Value::String(mesh.to_string()));
            }
            _ => self.components.push(make_component(
                "renderer",
                &RendererDef::of_mesh(mesh.to_string()),
            )),
        }
    }

    pub fn mesh(&self) -> Option<String> {
        self.component("renderer")
            .and_then(|v| v.get("mesh"))
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    pub fn is_camera(&self) -> bool {
        self.component("camera").is_some() || self.component("main_cam").is_some()
    }

    pub fn walk_grid(&self) -> Option<WalkGrid> {
        self.component("walk_grid")
            .and_then(|v| v.clone().try_into::<WalkGridDef>().ok())
            .map(WalkGrid::from)
    }
}

pub fn walk_grid(grid: &WalkGrid) -> GameObjDef {
    GameObjDef {
        id: Uuid::new_v4(),
        key: String::new(),
        name: None,
        from: None,
        components: vec![make_component("walk_grid", &WalkGridDef::from(grid))],
        children: Vec::new(),
        removed: Vec::new(),
        removed_children: Vec::new(),
        prefab_child: None,
    }
}

pub fn camera(pos: [f32; 3], look_at: [f32; 3]) -> GameObjDef {
    GameObjDef {
        id: Uuid::new_v4(),
        key: String::new(),
        name: None,
        from: None,
        components: vec![
            make_component(
                "transform",
                &TransformDef {
                    position: pos,
                    look_at: Some(look_at),
                    ..Default::default()
                },
            ),
            make_component("camera", &CameraDef::default()),
            unit_component("main_cam"),
        ],
        children: Vec::new(),
        removed: Vec::new(),
        removed_children: Vec::new(),
        prefab_child: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{GameObjDef, apply_child_overrides, merge_components};

    fn comps(src: &str) -> Vec<toml::Value> {
        #[derive(serde::Deserialize)]
        struct W {
            component: Vec<toml::Value>,
        }
        toml::from_str::<W>(src).unwrap().component
    }

    #[test]
    fn override_keeps_template_fields() {
        let t = comps("[[component]]\ntype = \"renderer\"\nmesh = \"cube\"\ncolor = \"red\"\n");
        let o = comps("[[component]]\ntype = \"renderer\"\ncolor = \"blue\"\n");
        let m = merge_components(&t, &o, &[]);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0]["mesh"].as_str(), Some("cube"));
        assert_eq!(m[0]["color"].as_str(), Some("blue"));
        assert_eq!(m[0]["type"].as_str(), Some("renderer"));
    }

    #[test]
    fn removed_type_dropped() {
        let t = comps("[[component]]\ntype = \"a\"\n[[component]]\ntype = \"b\"\n");
        let m = merge_components(&t, &[], &["a".to_string()]);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0]["type"].as_str(), Some("b"));
    }

    #[test]
    fn removed_type_with_unmatched_override_appended() {
        let t = comps("[[component]]\ntype = \"a\"\nx = 1\n");
        let o = comps("[[component]]\ntype = \"a\"\ny = 2\n");
        let m = merge_components(&t, &o, &["a".to_string()]);
        assert_eq!(m.len(), 1);
        assert!(m[0].get("x").is_none());
        assert_eq!(m[0]["y"].as_integer(), Some(2));
    }

    #[test]
    fn duplicates_matched_by_occurrence() {
        let t = comps(
            "[[component]]\ntype = \"a\"\nv = 1\nk = 0\n[[component]]\ntype = \"a\"\nv = 2\nk = 1\n",
        );
        let o = comps(
            "[[component]]\ntype = \"a\"\nv = 10\n[[component]]\ntype = \"a\"\nv = 20\n[[component]]\ntype = \"a\"\nv = 30\n",
        );
        let m = merge_components(&t, &o, &[]);
        assert_eq!(m.len(), 3);
        assert_eq!(m[0]["v"].as_integer(), Some(10));
        assert_eq!(m[0]["k"].as_integer(), Some(0));
        assert_eq!(m[1]["v"].as_integer(), Some(20));
        assert_eq!(m[1]["k"].as_integer(), Some(1));
        assert_eq!(m[2]["v"].as_integer(), Some(30));
    }

    #[test]
    fn unmatched_override_appended() {
        let t = comps("[[component]]\ntype = \"a\"\n");
        let o = comps("[[component]]\ntype = \"z\"\n[[component]]\ntype = \"a\"\nq = 1\n");
        let m = merge_components(&t, &o, &[]);
        assert_eq!(m.len(), 2);
        assert_eq!(m[0]["q"].as_integer(), Some(1));
        assert_eq!(m[1]["type"].as_str(), Some("z"));
    }

    #[test]
    fn removed_fields_round_trip() {
        let d: GameObjDef = toml::from_str("name = \"x\"\n").unwrap();
        assert!(d.removed.is_empty() && d.removed_children.is_empty());
        let s = toml::to_string(&d).unwrap();
        assert!(!s.contains("removed"));

        let d: GameObjDef =
            toml::from_str("removed = [\"renderer\"]\nremoved_child = [0, 2]\n").unwrap();
        assert_eq!(d.removed, vec!["renderer".to_string()]);
        assert_eq!(d.removed_children, vec![0, 2]);
        let s = toml::to_string(&d).unwrap();
        assert!(s.contains("removed = [\"renderer\"]"));
        assert!(s.contains("removed_child = [0, 2]"));
        let back: GameObjDef = toml::from_str(&s).unwrap();
        assert_eq!(back.removed, d.removed);
        assert_eq!(back.removed_children, d.removed_children);
    }

    fn child(name: &str, src: &str) -> GameObjDef {
        let mut d = GameObjDef::empty(name);
        d.components = comps(src);
        d
    }

    #[test]
    fn child_override_merges_in_place() {
        let base = vec![
            child(
                "a",
                "[[component]]\ntype = \"transform\"\nposition = [1.0, 0.0, 0.0]\n",
            ),
            child("b", "[[component]]\ntype = \"renderer\"\nmesh = \"cube\"\n"),
        ];
        let mut ov = child(
            "",
            "[[component]]\ntype = \"renderer\"\nmesh = \"sphere\"\n",
        );
        ov.name = None;
        ov.prefab_child = Some(1);
        let extra = GameObjDef::empty("extra");
        let out = apply_child_overrides(&base, &[ov, extra], &[]);
        assert_eq!(out.len(), 3);
        assert_eq!(out[1].name.as_deref(), Some("b"));
        assert_eq!(out[1].components[0]["mesh"].as_str(), Some("sphere"));
        assert_eq!(out[2].name.as_deref(), Some("extra"));
    }

    #[test]
    fn child_override_respects_removed() {
        let base = vec![GameObjDef::empty("a"), GameObjDef::empty("b")];
        let mut ov = GameObjDef::empty("renamed");
        ov.prefab_child = Some(1);
        let out = apply_child_overrides(&base, &[ov], &[0]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name.as_deref(), Some("renamed"));
    }
}
