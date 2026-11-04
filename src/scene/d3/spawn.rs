use crate::animation::{Animation, Animator};
use crate::color::Color;
use crate::ecs::prelude::*;
use crate::nav::WalkGrid;
use crate::physics::Collider;
use crate::render::d3::{
    Camera, MainCam, Material, MeshRegistry, Outline, Renderer, ShaderRegistry,
};
use crate::scene::spawn::{SpawnRegistry, defaults_of, f32_value, vec3_value, with_fields};

use super::def::{AnimationDef, CameraDef, ColliderDef, OutlineDef, RendererDef, WalkGridDef};
use super::file::Materials;

fn camera_defaults() -> toml::Table {
    let c = Camera::default();
    with_fields(
        defaults_of::<CameraDef>(),
        &[
            ("fov", f32_value(c.fov)),
            ("near", f32_value(c.near)),
            ("far", f32_value(c.far)),
            ("offset", vec3_value(c.offset)),
            ("smooth", f32_value(c.smooth)),
        ],
    )
}

fn collider_defaults() -> toml::Table {
    let c = Collider::default();
    with_fields(
        defaults_of::<ColliderDef>(),
        &[
            ("size", vec3_value(c.size)),
            ("offset", vec3_value(c.offset)),
            ("rotation", vec3_value(c.rotation)),
        ],
    )
}

fn outline_defaults() -> toml::Table {
    let o = Outline::default();
    let byte = |x: f32| toml::Value::Integer((x.clamp(0.0, 1.0) * 255.0).round() as i64);
    with_fields(
        defaults_of::<OutlineDef>(),
        &[
            (
                "color",
                toml::Value::Array(vec![byte(o.color.r), byte(o.color.g), byte(o.color.b)]),
            ),
            ("width", f32_value(o.width)),
        ],
    )
}

fn walk_grid_defaults() -> toml::Table {
    with_fields(
        defaults_of::<WalkGridDef>(),
        &[("blocked", toml::Value::Array(Vec::new()))],
    )
}

pub fn register(reg: &mut SpawnRegistry) {
    reg.categorize(
        "Rendering",
        &["renderer", "outline", "animation", "camera", "main_cam"],
    );
    reg.categorize("Physics", &["collider"]);
    reg.categorize("Scene", &["walk_grid"]);
    reg.add_with_defaults("renderer", spawn_renderer, defaults_of::<RendererDef>);
    reg.add_with_defaults(
        "camera",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<CameraDef>() {
                w.insert(e, Camera::from(d));
            }
        },
        camera_defaults,
    );
    reg.add_with_defaults("main_cam", |w, _, e| w.insert(e, MainCam), toml::Table::new);
    reg.add_with_defaults(
        "walk_grid",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<WalkGridDef>() {
                w.insert(e, WalkGrid::from(d));
            }
        },
        walk_grid_defaults,
    );
    reg.add_with_defaults(
        "collider",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<ColliderDef>() {
                w.insert(e, Collider::from(d));
            }
        },
        collider_defaults,
    );
    reg.add_with_defaults(
        "outline",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<OutlineDef>() {
                w.insert(e, Outline::from(d));
            }
        },
        outline_defaults,
    );
    reg.add_with_defaults(
        "animation",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<AnimationDef>() {
                w.insert(e, Animation::from(d));
            }
        },
        defaults_of::<AnimationDef>,
    );
}

fn spawn_renderer(world: &mut World, v: &toml::Value, e: Entity) {
    let Ok(r) = v.clone().try_into::<RendererDef>() else {
        return;
    };
    let (mut slots, names) = world
        .resource::<MeshRegistry>()
        .map(|reg| (reg.resolve(&r.mesh), reg.slot_names(&r.mesh)))
        .unwrap_or_default();
    if world.resource::<ShaderRegistry>().is_err() {
        world.insert_resource(ShaderRegistry::new());
    }
    let materials = world.resource::<Materials>().cloned().unwrap_or_default();
    if let Ok(shaders) = world.resource_mut::<ShaderRegistry>() {
        r.apply_to(&mut slots, &names, shaders, &materials);
    }
    if let Ok(meshes) = world.resource_mut::<MeshRegistry>() {
        r.apply_textures(&slots, &names, meshes, &materials);
    }
    world.insert(
        e,
        Renderer {
            slots,
            cast_shadows: r.cast_shadows.unwrap_or(true),
            outline_occlude: r.outline_occlude.unwrap_or(true),
        },
    );
    if let Some(skeleton) = world
        .resource::<MeshRegistry>()
        .ok()
        .and_then(|reg| reg.skeleton(&r.mesh))
    {
        world.insert(e, Animator::new(skeleton));
    }
}

pub fn set_mesh(world: &mut World, e: Entity, name: &str) {
    let mut slots = world
        .resource::<MeshRegistry>()
        .map(|reg| reg.resolve(name))
        .unwrap_or_default();
    let prev = world
        .get::<Renderer>(e)
        .and_then(|r| r.slots.first().map(|s| s.material));
    let cast_shadows = world
        .get::<Renderer>(e)
        .map(|r| r.cast_shadows)
        .unwrap_or(true);
    let outline_occlude = world
        .get::<Renderer>(e)
        .map(|r| r.outline_occlude)
        .unwrap_or(true);
    if let Some(slot) = slots.first_mut() {
        slot.material = prev.unwrap_or(Material::LIT.color(Color::GRAY_400));
    }
    world.insert(
        e,
        Renderer {
            slots,
            cast_shadows,
            outline_occlude,
        },
    );
}
