use glam::Vec3;

use tungstengine::color::Color;
use tungstengine::ecs::prelude::*;
use tungstengine::render::d3::prelude::*;

const HALF: i32 = 20;
const HEIGHT: f32 = 0.002;
const MAJOR_W: f32 = 0.012;
const MINOR_W: f32 = 0.006;
const MINOR_MAX_HEIGHT: f32 = 7.0;
const MAJOR_COLOR: Color = Color::SLATE_300;
const MINOR_COLOR: Color = Color::SLATE_400;
const AXIS_X_COLOR: Color = Color::RED_500;
const BACKDROP: Color = Color::rgb(33, 36, 46);
const MAJOR_ALPHA: f32 = 0.35;
const MINOR_ALPHA: f32 = 0.2;
const AXIS_Z_COLOR: Color = Color::BLUE_500;

#[derive(Resource)]
pub struct EditorGrid {
    minor: Vec<(Entity, Vec3)>,
    minor_visible: bool,
}

fn line(world: &mut World, along_x: bool, offset: f32, width: f32, color: Color) -> (Entity, Vec3) {
    let len = HALF as f32 * 2.0;
    let (pos, scale) = if along_x {
        (
            Vec3::new(0.0, HEIGHT * 0.5, offset),
            Vec3::new(len, HEIGHT, width),
        )
    } else {
        (
            Vec3::new(offset, HEIGHT * 0.5, 0.0),
            Vec3::new(width, HEIGHT, len),
        )
    };
    let e = world.spawn();
    let mut t = Transform::from_pos(pos);
    t.scale = scale;
    world.insert(e, t);
    let mut r = Renderer::new(Mesh::CUBE, Material::UNLIT.color(color)).no_outline_occlude();
    r.cast_shadows = false;
    world.insert(e, r);
    (e, scale)
}

pub fn setup(world: &mut World) {
    let mut minor = Vec::new();
    for along_x in [true, false] {
        for i in -HALF..=HALF {
            let color = match (i, along_x) {
                (0, true) => AXIS_X_COLOR,
                (0, false) => AXIS_Z_COLOR,
                _ => MAJOR_COLOR,
            };
            let alpha = if i == 0 { 0.7 } else { MAJOR_ALPHA };
            line(
                world,
                along_x,
                i as f32,
                MAJOR_W,
                BACKDROP.lerp(color, alpha),
            );
            if i < HALF {
                minor.push(line(
                    world,
                    along_x,
                    i as f32 + 0.5,
                    MINOR_W,
                    BACKDROP.lerp(MINOR_COLOR, MINOR_ALPHA),
                ));
            }
        }
    }
    world.insert_resource(EditorGrid {
        minor,
        minor_visible: true,
    });
}

pub fn frame(world: &mut World) {
    let height = world
        .query::<(&GlobalTransform, &MainCam)>()
        .next()
        .map(|(_, g, _)| g.position().y.abs())
        .unwrap_or(f32::INFINITY);
    let show = height < MINOR_MAX_HEIGHT;
    let Ok(grid) = world.resource::<EditorGrid>() else {
        return;
    };
    if grid.minor_visible == show {
        return;
    }
    let minor = grid.minor.clone();
    for (e, scale) in minor {
        if let Some(t) = world.get_mut::<Transform>(e) {
            t.scale = if show { scale } else { Vec3::ZERO };
        }
    }
    if let Ok(grid) = world.resource_mut::<EditorGrid>() {
        grid.minor_visible = show;
    }
}
