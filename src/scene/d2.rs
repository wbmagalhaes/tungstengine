use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::ecs::prelude::*;
use crate::render::d2::SpriteRenderer;
use crate::sprites::Sprites;

use super::spawn::{SpawnRegistry, defaults_of};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SpriteDef {
    pub sprite: String,
    pub size: [f32; 2],
    pub anchor: [f32; 2],
    pub color: Option<String>,
    pub layer: i32,
    pub flip_x: bool,
}

impl Default for SpriteDef {
    fn default() -> Self {
        SpriteDef {
            sprite: String::new(),
            size: [1.0, 1.0],
            anchor: [0.5, 0.5],
            color: None,
            layer: 0,
            flip_x: false,
        }
    }
}

pub fn register(reg: &mut SpawnRegistry) {
    reg.categorize("Rendering", &["sprite"]);
    reg.add_with_defaults("sprite", spawn_sprite, defaults_of::<SpriteDef>);
}

fn spawn_sprite(world: &mut World, v: &toml::Value, e: Entity) {
    let Ok(d) = v.clone().try_into::<SpriteDef>() else {
        return;
    };
    let sprite = world.resource_core_mut::<Sprites>().get(&d.sprite);
    let mut r = SpriteRenderer::new(sprite, Vec2::from_array(d.size));
    r.anchor = Vec2::from_array(d.anchor);
    r.layer = d.layer;
    r.flip_x = d.flip_x;
    if let Some(color) = d.color.as_deref() {
        r.color = Color::hex(color);
    }
    world.insert(e, r);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Transform;
    use crate::scene::{GameObjDef, register_builtin, spawn_object};
    use crate::texture::Texture2D;

    fn world() -> World {
        let mut world = World::new();
        let mut reg = SpawnRegistry::default();
        register_builtin(&mut reg);
        world.insert_resource(reg);
        world.resource_core_mut::<Sprites>().insert(
            "cat",
            Texture2D {
                width: 1,
                height: 1,
                rgba: vec![255; 4],
            },
        );
        world
    }

    #[test]
    fn spawns_a_sprite_from_a_scene_object() {
        let mut world = world();
        let obj: GameObjDef = toml::from_str(
            "[[component]]\ntype = \"transform\"\nposition = [2.0, 3.0, 0.0]\n\n[[component]]\ntype = \"sprite\"\nsprite = \"cat\"\nlayer = 4\n",
        )
        .unwrap();
        let e = spawn_object(&mut world, &obj).unwrap();
        assert_eq!(world.get::<SpriteRenderer>(e).map(|s| s.layer), Some(4));
        assert_eq!(world.get::<Transform>(e).map(|t| t.position.x), Some(2.0));
    }
}
