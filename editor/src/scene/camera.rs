use glam::{EulerRot, Quat, Vec3};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use tungstengine::ecs::prelude::*;
use tungstengine::input::Input;
use tungstengine::render::d3::prelude::*;

const LOOK_SENS: f32 = 0.004;
const ORBIT_SENS: f32 = 0.006;
const PITCH_LIMIT: f32 = 1.55;
const BASE_SPEED: f32 = 6.0;
const FAST: f32 = 3.0;
const FOCUS_DIST: f32 = 6.0;

#[derive(Resource)]
pub struct EditorCamera {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
    dist: f32,
    speed: f32,
    last_cursor: (f32, f32),
}

impl EditorCamera {
    fn rotation(&self) -> Quat {
        Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0)
    }

    fn forward(&self) -> Vec3 {
        self.rotation() * Vec3::NEG_Z
    }

    fn pivot(&self) -> Vec3 {
        self.pos + self.forward() * self.dist
    }
}

fn main_cam(world: &World) -> Option<Entity> {
    world
        .query::<(&Transform, &MainCam)>()
        .next()
        .map(|(e, _, _)| e)
}

pub fn setup(world: &mut World) {
    let Some(e) = main_cam(world) else {
        return;
    };
    let Some(t) = world.get::<Transform>(e).copied() else {
        return;
    };
    let f = t.rotation * Vec3::NEG_Z;
    let dist = if f.y < -1e-3 {
        (t.position.y / -f.y).clamp(1.0, 100.0)
    } else {
        10.0
    };
    world.insert_resource(EditorCamera {
        pos: t.position,
        yaw: (-f.x).atan2(-f.z),
        pitch: f.y.clamp(-1.0, 1.0).asin(),
        dist,
        speed: BASE_SPEED,
        last_cursor: (0.0, 0.0),
    });
}

pub fn focus(world: &mut World, target: Vec3) {
    if let Ok(c) = world.resource_mut::<EditorCamera>() {
        c.dist = FOCUS_DIST;
        c.pos = target - c.forward() * c.dist;
    }
}

pub struct CamInput {
    pub over_ui: bool,
    pub enabled: bool,
}

pub fn frame(world: &mut World, ci: CamInput) -> bool {
    let dt = world.resource::<Time>().map(|t| t.dt).unwrap_or(1.0 / 60.0);
    let Ok(i) = world.resource::<Input>() else {
        return false;
    };
    let cursor = i.cursor;
    let rmb = i.mouse_held(MouseButton::Right);
    let mmb = i.mouse_held(MouseButton::Middle);
    let alt = i.key_held(KeyCode::AltLeft) || i.key_held(KeyCode::AltRight);
    let lmb = i.mouse_held(MouseButton::Left);
    let lmb_pressed = i.mouse_pressed(MouseButton::Left);
    let fast = i.key_held(KeyCode::ShiftLeft) || i.key_held(KeyCode::ShiftRight);
    let scroll = i.scroll();
    let axis =
        |neg: KeyCode, pos: KeyCode| (i.key_held(pos) as i32 - i.key_held(neg) as i32) as f32;
    let fly = Vec3::new(
        axis(KeyCode::KeyA, KeyCode::KeyD),
        axis(KeyCode::KeyQ, KeyCode::KeyE),
        axis(KeyCode::KeyS, KeyCode::KeyW),
    );

    let Ok(c) = world.resource_mut::<EditorCamera>() else {
        return false;
    };
    let (dx, dy) = (cursor.0 - c.last_cursor.0, cursor.1 - c.last_cursor.1);
    c.last_cursor = cursor;
    if !ci.enabled {
        return false;
    }

    let orbiting = alt && lmb && !(ci.over_ui && lmb_pressed);
    let rot = c.rotation();
    let (right, up, fwd) = (rot * Vec3::X, rot * Vec3::Y, rot * Vec3::NEG_Z);

    if rmb {
        c.yaw -= dx * LOOK_SENS;
        c.pitch = (c.pitch - dy * LOOK_SENS).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        if scroll != 0.0 {
            c.speed = (c.speed * (1.0 + scroll * 0.15)).clamp(0.5, 80.0);
        }
        let mult = if fast { FAST } else { 1.0 };
        let mv = right * fly.x + Vec3::Y * fly.y + fwd * fly.z;
        c.pos += mv * c.speed * mult * dt;
    } else if orbiting {
        let pivot = c.pivot();
        c.yaw -= dx * ORBIT_SENS;
        c.pitch = (c.pitch - dy * ORBIT_SENS).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        c.pos = pivot - c.forward() * c.dist;
    } else if mmb {
        let k = c.dist * 0.0015;
        c.pos += (-right * dx + up * dy) * k;
    } else if scroll != 0.0 && !ci.over_ui {
        let step = (c.dist * 0.15 * scroll).clamp(-c.dist * 0.9, 50.0);
        c.pos += fwd * step;
        c.dist = (c.dist - step).max(0.5);
    }

    let (pos, rot) = (c.pos, c.rotation());
    if let Some(e) = main_cam(world)
        && let Some(t) = world.get_mut::<Transform>(e)
    {
        t.position = pos;
        t.rotation = rot;
    }
    rmb || orbiting || alt
}
