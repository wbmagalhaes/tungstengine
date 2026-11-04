use glam::{Mat4, Quat, Vec2, Vec3};

use super::components::input_captured;
use super::interact::interactable_hit;
use crate::prelude::*;
use tungstengine::animation::Animation;
use tungstengine::animation::Animator;
use tungstengine::audio::queue_sound;
use tungstengine::ecs::prelude::*;
use tungstengine::input::Input;
use tungstengine::nav::WalkGrid;
use tungstengine::physics::{Collider, Footprint};
use tungstengine::render::d3::prelude::*;
use tungstengine::scene::{GameObjDef, Prefabs, Scenes, spawn_object};

pub fn ground_point(ray: &CursorRay) -> Vec3 {
    if ray.dir.y.abs() > 1e-6 {
        ray.origin + ray.dir * (-ray.origin.y / ray.dir.y)
    } else {
        Vec3::ZERO
    }
}

pub const CURSOR_SCALE: Vec3 = Vec3::new(0.2, 1.0, 0.2);

pub fn update_cursor(world: &mut World) -> SysResult {
    let blocked = input_captured(world);
    let ground = ground_point(world.resource::<CursorRay>()?);
    for (_e, t, _) in world.query_mut::<(&mut Transform, &CursorTag)>() {
        if blocked {
            t.scale = Vec3::ZERO;
        } else {
            t.scale = CURSOR_SCALE;
            t.position = ground + Vec3::Y * 0.001;
        }
    }
    Ok(())
}

pub const PLAYER_RADIUS: f32 = 0.28;

pub fn rasterize_colliders(world: &mut World) -> SysResult {
    let prints: Vec<(Footprint, f32)> = world
        .query::<(&GlobalTransform, &Collider)>()
        .filter(|(_, _, c)| c.nav)
        .map(|(_, g, c)| (c.footprint(g.0), c.pad + PLAYER_RADIUS))
        .collect();

    let Some(e) = world.query::<(&WalkGrid,)>().next().map(|(e, _)| e) else {
        return Ok(());
    };
    if let Some(grid) = world.get_mut::<WalkGrid>(e) {
        grid.clear_dynamic();
        for (fp, inflate) in &prints {
            grid.stamp_footprint(fp, *inflate);
        }
    }
    Ok(())
}

pub fn collide_slide(world: &mut World) -> SysResult {
    let prints: Vec<(Footprint, f32)> = world
        .query::<(&GlobalTransform, &Collider)>()
        .map(|(_, g, c)| (c.footprint(g.0), c.pad))
        .collect();
    if prints.is_empty() {
        return Ok(());
    }

    let Some((e, mut p)) = world
        .query::<(&Transform, &Player)>()
        .next()
        .map(|(e, t, _)| (e, Vec2::new(t.position.x, t.position.z)))
    else {
        return Ok(());
    };

    for (fp, pad) in &prints {
        let reach = pad + PLAYER_RADIUS;
        let q = fp.closest_on_boundary(p);
        let delta = p - q;
        let d = delta.length();
        if fp.contains(p) {
            let out = if d > 1e-5 { -delta / d } else { Vec2::X };
            p = q + out * reach;
        } else if d < reach && d > 1e-5 {
            p = q + delta / d * reach;
        }
    }

    if let Some(t) = world.get_mut::<Transform>(e) {
        t.position.x = p.x;
        t.position.z = p.y;
    }
    Ok(())
}

const WALK_ACCEL: f32 = 30.0;
const TURN_SPEED: f32 = 18.0;

pub fn player_animation(world: &mut World) -> SysResult {
    let walking = !world.resource::<Destination>()?.path.is_empty();
    let clip = if walking { "walk" } else { "idle" };
    let ents: Vec<Entity> = world
        .query::<(&Player, &Animator)>()
        .map(|(e, _, _)| e)
        .collect();
    for e in ents {
        if let Some(a) = world.get_mut::<Animator>(e) {
            a.play(clip);
        }
    }
    Ok(())
}

pub fn walk(world: &mut World) -> SysResult {
    let dt = world.resource::<Time>()?.dt;
    let path = world.resource::<Destination>()?.path.clone();

    let Some((e, pos, speed, vel)) = world
        .query::<(&Transform, &Player)>()
        .next()
        .map(|(e, t, p)| (e, t.position, p.speed, p.vel))
    else {
        return Ok(());
    };

    let Some(&target) = path.first() else {
        if let Some(p) = world.get_mut::<Player>(e) {
            p.vel = (vel - WALK_ACCEL * dt).max(0.0);
        }
        return Ok(());
    };

    let mut remaining = 0.0;
    let mut prev = Vec3::new(pos.x, 0.0, pos.z);
    for wp in &path {
        let flat = Vec3::new(wp.x, 0.0, wp.z);
        remaining += (flat - prev).length();
        prev = flat;
    }

    let cruise = (2.0 * WALK_ACCEL * remaining).sqrt();
    let target_speed = speed.min(cruise);
    let new_vel = if vel < target_speed {
        (vel + WALK_ACCEL * dt).min(target_speed)
    } else {
        target_speed
    };

    let goal = Vec3::new(target.x, pos.y, target.z);
    let to = goal - pos;
    let dist = to.length();
    let step = new_vel * dt;
    let (new_pos, reached) = if dist <= step || dist < 1e-4 {
        (goal, true)
    } else {
        (pos + to / dist * step, false)
    };

    if let Some(p) = world.get_mut::<Player>(e) {
        p.vel = new_vel;
    }
    if let Some(t) = world.get_mut::<Transform>(e) {
        t.position = new_pos;
        let flat = Vec3::new(to.x, 0.0, to.z);
        if new_vel > 0.05 && flat.length_squared() > 1e-6 {
            let yaw = flat.x.atan2(flat.z);
            let target = Quat::from_rotation_y(yaw);
            t.rotation = t.rotation.slerp(target, (TURN_SPEED * dt).min(1.0));
        }
    }

    if reached
        && let Ok(dest) = world.resource_mut::<Destination>()
        && !dest.path.is_empty()
    {
        dest.path.remove(0);
    }

    Ok(())
}

pub fn hover_outline(world: &mut World) -> SysResult {
    let captured = input_captured(world);

    let ray = *world.resource::<CursorRay>()?;
    let mut hit: Option<(Entity, f32)> = None;
    if !captured {
        let ents: Vec<Entity> = world.query::<(&Interactable,)>().map(|(e, _)| e).collect();
        for e in ents {
            if let Some(d) = interactable_hit(world, e, &ray)
                && hit.is_none_or(|(_, hd)| d < hd)
            {
                hit = Some((e, d));
            }
        }
    }

    let hit = hit.map(|(e, _)| e);

    let prev: Vec<Entity> = world.query::<(&Hovered,)>().map(|(e, _)| e).collect();
    for e in prev {
        if Some(e) != hit {
            world.remove::<Hovered>(e);
        }
    }

    if let Some(e) = hit
        && !world.has::<Hovered>(e)
    {
        world.insert(e, Hovered);
    }

    Ok(())
}

pub const WALK_MARK_SCALE: Vec3 = Vec3::new(0.3, 1.0, 0.3);

pub fn walkmark_visibility(world: &mut World) -> SysResult {
    let scale = if world.resource::<Destination>()?.path.is_empty() {
        Vec3::ZERO
    } else {
        WALK_MARK_SCALE
    };
    for (_e, t, _) in world.query_mut::<(&mut Transform, &WalkMarkTag)>() {
        t.scale = scale;
    }
    Ok(())
}

const ZOOM_MIN: f32 = 0.85;
const ZOOM_MAX: f32 = 2.50;
const ZOOM_SENS: f32 = 0.10;

pub fn camera_follow(world: &mut World) -> SysResult {
    let dt = world.resource::<Time>()?.dt;
    let blocked = input_captured(world);
    let scroll = if blocked {
        0.0
    } else {
        world.resource::<Input>().map(|i| i.scroll()).unwrap_or(0.0)
    };

    let Some(target) = world
        .query::<(&Transform, &Player)>()
        .next()
        .map(|(_, t, _)| t.position)
    else {
        return Ok(());
    };

    let Some((cam_e, mut cam)) = world
        .query::<(&Camera, &MainCam)>()
        .next()
        .map(|(e, c, _)| (e, *c))
    else {
        return Ok(());
    };

    if scroll != 0.0 {
        cam.zoom = (cam.zoom - scroll * ZOOM_SENS).clamp(ZOOM_MIN, ZOOM_MAX);
        if let Some(c) = world.get_mut::<Camera>(cam_e) {
            c.zoom = cam.zoom;
        }
    }

    let focus = target + Vec3::Y * 1.5;
    let eye = focus + cam.offset * cam.zoom;

    let cam_world = Mat4::look_at_rh(eye, focus, Vec3::Y).inverse();
    let (_, desired_rot, desired_pos) = cam_world.to_scale_rotation_translation();

    let alpha = 1.0 - (-cam.smooth * dt).exp();

    for (_e, t, _) in world.query_mut::<(&mut Transform, &MainCam)>() {
        t.position = t.position.lerp(desired_pos, alpha);
        t.rotation = t.rotation.slerp(desired_rot, alpha);
    }
    Ok(())
}

pub fn scene_trigger(world: &mut World) -> SysResult {
    let Some(pp) = world
        .query::<(&Transform, &Player)>()
        .next()
        .map(|(_, t, _)| t.position)
    else {
        return Ok(());
    };
    let flags = world.resource::<Flags>()?.all();

    let mut go: Option<String> = None;
    for (_e, t, trig) in world.query::<(&Transform, &ChangeSceneTrigger)>() {
        let enabled = trig
            .flag
            .as_ref()
            .is_none_or(|f| flags.iter().any(|x| x == f));
        if !enabled {
            continue;
        }
        let dx = pp.x - t.position.x;
        let dz = pp.z - t.position.z;
        if (dx * dx + dz * dz).sqrt() < trig.radius {
            go = Some(trig.to.clone());
        }
    }

    if let Some(to) = go
        && let Ok(s) = world.resource_mut::<Scenes>()
    {
        s.request(&to);
    }

    Ok(())
}

pub fn play_on_flag(world: &mut World) -> SysResult {
    let flags = world.resource::<Flags>()?.all();
    let fire: Vec<(Entity, Animation)> = world
        .query::<(&PlayOnFlag,)>()
        .filter(|(_, p)| flags.iter().any(|f| f == &p.flag))
        .map(|(e, p)| (e, p.anim.clone()))
        .collect();
    for (e, anim) in fire {
        world.insert(e, anim);
        world.remove::<PlayOnFlag>(e);
    }
    Ok(())
}

pub fn disable_on_flag(world: &mut World) -> SysResult {
    let flags = world.resource::<Flags>()?.all();
    let fire: Vec<(Entity, bool)> = world
        .query::<(&DisableOnFlag,)>()
        .filter(|(_, d)| flags.iter().any(|f| f == &d.flag))
        .map(|(e, d)| (e, d.hide))
        .collect();
    for (e, hide) in fire {
        if hide {
            world.despawn(e);
        } else {
            world.remove::<Interactable>(e);
            world.remove::<DisableOnFlag>(e);
        }
    }
    Ok(())
}

pub fn look_on_flag(world: &mut World) -> SysResult {
    let flags = world.resource::<Flags>()?.all();
    let fire: Vec<(Entity, Vec<String>)> = world
        .query::<(&LookOnFlag,)>()
        .filter(|(_, l)| flags.iter().any(|f| f == &l.flag))
        .map(|(e, l)| (e, l.look.clone()))
        .collect();
    for (e, look) in fire {
        if let Some(i) = world.get_mut::<Interactable>(e) {
            i.set_look(look);
        }
        world.remove::<LookOnFlag>(e);
    }
    Ok(())
}

pub fn sfx_on_flag(world: &mut World) -> SysResult {
    let flags = world.resource::<Flags>()?.all();
    let fire: Vec<(Entity, String)> = world
        .query::<(&SfxOnFlag,)>()
        .filter(|(_, s)| flags.iter().any(|f| f == &s.flag))
        .map(|(e, s)| (e, s.sound.clone()))
        .collect();
    for (e, sound) in fire {
        world.remove::<SfxOnFlag>(e);
        queue_sound(&sound);
    }
    Ok(())
}

pub fn spawn_on_flag(world: &mut World) -> SysResult {
    let flags = world.resource::<Flags>()?.all();
    let fire: Vec<(Entity, String, Vec3, Vec3)> = world
        .query::<(&SpawnOnFlag,)>()
        .filter(|(_, s)| flags.iter().any(|f| f == &s.flag))
        .map(|(e, s)| (e, s.prefab.clone(), s.offset, s.scale))
        .collect();
    for (e, prefab, offset, scale) in fire {
        world.remove::<SpawnOnFlag>(e);
        let pos = world
            .get::<GlobalTransform>(e)
            .map(|g| g.position())
            .unwrap_or(Vec3::ZERO)
            + offset;
        let id = world
            .resource::<Prefabs>()
            .ok()
            .and_then(|p| p.id_of(&prefab));
        if let Some(id) = id {
            let obj = GameObjDef::instance(Some(id), None, pos, Vec3::ZERO, scale);
            spawn_object(world, &obj);
        }
    }
    Ok(())
}
