use glam::{Vec2, Vec3};

use crate::ecs::prelude::*;
use crate::ecs::{RenderOffset, Transform};
use crate::fixed_step::FixedStep;
use crate::tilemap::Tilemap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape2D {
    Box(Vec2),
    Circle(f32),
}

pub const EARTH_GRAVITY: f32 = 9.8;
pub const UNITS_PER_METER: f32 = 3.0;

#[derive(Component, Clone, Copy, Debug)]
pub struct Collider2D {
    pub shape: Shape2D,
    pub offset: Vec2,
    pub sensor: bool,
}

impl Collider2D {
    pub fn rect(size: Vec2) -> Self {
        Collider2D {
            shape: Shape2D::Box(size),
            offset: Vec2::ZERO,
            sensor: false,
        }
    }

    pub fn circle(radius: f32) -> Self {
        Collider2D {
            shape: Shape2D::Circle(radius),
            offset: Vec2::ZERO,
            sensor: false,
        }
    }

    pub fn sensor(mut self) -> Self {
        self.sensor = true;
        self
    }

    fn reach(&self) -> f32 {
        match self.shape {
            Shape2D::Box(size) => size.min_element() / 2.0,
            Shape2D::Circle(r) => r,
        }
        .max(0.01)
    }

    fn placed(&self, t: &Transform) -> Placed {
        (self.shape, t.position.truncate() + self.offset)
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Body2D {
    pub velocity: Vec2,
    pub gravity_scale: f32,
    pub on_ground: bool,
    previous: Vec2,
}

impl Default for Body2D {
    fn default() -> Self {
        Body2D {
            velocity: Vec2::ZERO,
            gravity_scale: 1.0,
            on_ground: false,
            previous: Vec2::ZERO,
        }
    }
}

#[derive(Resource, Clone, Copy, Debug, Default)]
#[resource(core)]
pub struct Physics2D {
    pub gravity: Vec2,
    pub timing: FixedStep,
}

impl Physics2D {
    pub fn earth() -> Self {
        Self::with_gravity(Vec2::Y * EARTH_GRAVITY * UNITS_PER_METER)
    }

    pub fn jump_speed(&self, height: f32) -> f32 {
        (2.0 * self.gravity.length() * height).sqrt()
    }

    pub fn with_gravity(gravity: Vec2) -> Self {
        Physics2D {
            gravity,
            ..Default::default()
        }
    }
}

#[derive(Resource, Clone, Debug, Default)]
pub struct TileCollider(Vec<Placed>);

impl TileCollider {
    pub fn new(map: &Tilemap) -> Self {
        TileCollider(blocked_tiles(map))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit2D {
    pub entity: Entity,
    pub distance: f32,
    pub point: Vec2,
}

type Placed = (Shape2D, Vec2);

fn box_push(a: Vec2, ha: Vec2, b: Vec2, hb: Vec2) -> Option<Vec2> {
    let d = a - b;
    let overlap = ha + hb - d.abs();
    if overlap.x <= 0.0 || overlap.y <= 0.0 {
        return None;
    }
    let sign = |v: f32| if v < 0.0 { -1.0 } else { 1.0 };
    Some(if overlap.x < overlap.y {
        Vec2::new(overlap.x * sign(d.x), 0.0)
    } else {
        Vec2::new(0.0, overlap.y * sign(d.y))
    })
}

fn circle_box_push(c: Vec2, r: f32, b: Vec2, half: Vec2) -> Option<Vec2> {
    let closest = c.clamp(b - half, b + half);
    let d = c - closest;
    if d == Vec2::ZERO {
        return box_push(c, Vec2::splat(r), b, half);
    }
    let len = d.length();
    (len < r).then(|| d / len * (r - len))
}

pub fn push_out(a: Placed, b: Placed) -> Option<Vec2> {
    match (a, b) {
        ((Shape2D::Box(sa), pa), (Shape2D::Box(sb), pb)) => box_push(pa, sa / 2.0, pb, sb / 2.0),
        ((Shape2D::Circle(r), pa), (Shape2D::Box(s), pb)) => circle_box_push(pa, r, pb, s / 2.0),
        ((Shape2D::Box(s), pa), (Shape2D::Circle(r), pb)) => {
            circle_box_push(pb, r, pa, s / 2.0).map(|p| -p)
        }
        ((Shape2D::Circle(ra), pa), (Shape2D::Circle(rb), pb)) => {
            let d = pa - pb;
            let len = d.length();
            let depth = ra + rb - len;
            (depth > 0.0).then(|| {
                let normal = if len > 0.0 { d / len } else { Vec2::Y };
                normal * depth
            })
        }
    }
}

fn solids(world: &World) -> Vec<(Entity, Placed)> {
    world
        .query::<(&Collider2D, &Transform)>()
        .filter(|(e, c, _)| !c.sensor && !world.has::<Body2D>(*e))
        .map(|(e, c, t)| (e, c.placed(t)))
        .collect()
}

fn blocked_tiles(map: &Tilemap) -> Vec<Placed> {
    let size = Vec2::splat(map.cell);
    let height = map.layers.iter().map(|l| l.rows.len()).max().unwrap_or(0);
    let width = map
        .layers
        .iter()
        .flat_map(|l| l.rows.iter().map(|r| r.chars().count()))
        .max()
        .unwrap_or(0);
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .filter(|&(x, y)| map.is_blocked(x, y))
        .map(|(x, y)| {
            let p = map.position(x as f32, y as f32, 0.0);
            (Shape2D::Box(size), Vec2::new(p.x, p.y))
        })
        .collect()
}

pub fn physics2d_step(world: &mut World) -> SysResult {
    let dt = world.dt();
    let physics = world.resource_core_mut::<Physics2D>();
    let steps = physics.timing.advance(dt);
    let (gravity, step, alpha) = (physics.gravity, physics.timing.step, physics.timing.alpha());
    if steps == 0 {
        interpolate(world, alpha, step);
        return Ok(());
    }
    let mut walls: Vec<Placed> = solids(world).into_iter().map(|(_, p)| p).collect();
    if let Ok(tiles) = world.resource::<TileCollider>() {
        walls.extend(tiles.0.iter().copied());
    }
    for _ in 0..steps {
        simulate(world, &walls, gravity, step);
    }
    interpolate(world, alpha, step);
    Ok(())
}

fn interpolate(world: &mut World, alpha: f32, step: f32) {
    let offsets: Vec<(Entity, Vec3)> = world
        .query::<(&Body2D, &Transform)>()
        .map(|(e, b, t)| {
            let back = b.previous - t.position.truncate();
            let teleported = back.length() > b.velocity.length() * step * 2.0 + 1e-4;
            let offset = if teleported {
                Vec2::ZERO
            } else {
                back * (1.0 - alpha)
            };
            (e, offset.extend(0.0))
        })
        .collect();
    for (e, offset) in offsets {
        world.insert(e, RenderOffset(offset));
    }
}

fn simulate(world: &mut World, walls: &[Placed], gravity: Vec2, dt: f32) {
    let bodies: Vec<(Entity, Collider2D)> = world
        .query::<(&Body2D, &Collider2D)>()
        .map(|(e, _, c)| (e, *c))
        .collect();
    for (e, collider) in bodies {
        let Some(mut body) = world.get::<Body2D>(e).copied() else {
            continue;
        };
        let Some(t) = world.get_mut::<Transform>(e) else {
            continue;
        };
        body.previous = t.position.truncate();
        body.velocity += gravity * body.gravity_scale * dt;
        body.on_ground = false;
        let step = body.velocity * dt;
        let count = (step.length() / collider.reach() / 0.5).ceil().max(1.0);
        for _ in 0..count as u32 {
            let part = body.velocity * dt / count;
            t.position.x += part.x;
            t.position.y += part.y;
            for wall in walls {
                let Some(push) = push_out(collider.placed(t), *wall) else {
                    continue;
                };
                t.position.x += push.x;
                t.position.y += push.y;
                let normal = push.normalize_or_zero();
                let into = body.velocity.dot(normal);
                if into < 0.0 {
                    body.velocity -= normal * into;
                }
                if normal.dot(gravity) < 0.0 {
                    body.on_ground = true;
                }
            }
        }
        world.insert(e, body);
    }
}

pub fn overlaps(world: &World, shape: Shape2D, pos: Vec2) -> Vec<Entity> {
    world
        .query::<(&Collider2D, &Transform)>()
        .filter(|(_, c, t)| push_out((shape, pos), c.placed(t)).is_some())
        .map(|(e, _, _)| e)
        .collect()
}

fn ray_box(origin: Vec2, dir: Vec2, center: Vec2, half: Vec2) -> Option<f32> {
    let inv = dir.recip();
    let a = (center - half - origin) * inv;
    let b = (center + half - origin) * inv;
    let near = a.min(b).max_element();
    let far = a.max(b).min_element();
    (far >= near.max(0.0)).then_some(near.max(0.0))
}

fn ray_circle(origin: Vec2, dir: Vec2, center: Vec2, r: f32) -> Option<f32> {
    let to = origin - center;
    let b = to.dot(dir);
    let disc = b * b - (to.length_squared() - r * r);
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    let far = -b + disc.sqrt();
    (far >= 0.0).then_some(t.max(0.0))
}

pub fn raycast2d(world: &World, origin: Vec2, dir: Vec2, max: f32) -> Option<RayHit2D> {
    let dir = dir.normalize_or_zero();
    world
        .query::<(&Collider2D, &Transform)>()
        .filter(|(_, c, _)| !c.sensor)
        .filter_map(|(entity, c, t)| {
            let (shape, pos) = c.placed(t);
            let distance = match shape {
                Shape2D::Box(size) => ray_box(origin, dir, pos, size / 2.0),
                Shape2D::Circle(r) => ray_circle(origin, dir, pos, r),
            }?;
            (distance <= max).then_some(RayHit2D {
                entity,
                distance,
                point: origin + dir * distance,
            })
        })
        .min_by(|a, b| a.distance.total_cmp(&b.distance))
}

pub mod prelude {
    pub use super::{
        Body2D, Collider2D, EARTH_GRAVITY, Physics2D, RayHit2D, Shape2D, TileCollider,
        UNITS_PER_METER, overlaps, physics2d_step, raycast2d,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Time;
    use crate::tilemap::{TileCollision, TileDef, TileLayer};
    use std::collections::BTreeMap;

    fn world(gravity: Vec2) -> World {
        let mut w = World::new();
        w.insert_resource(Time {
            dt: 0.1,
            ..Default::default()
        });
        w.insert_resource(Physics2D::with_gravity(gravity));
        w
    }

    fn spawn(w: &mut World, pos: Vec2, c: Collider2D) -> Entity {
        let e = w.spawn();
        w.insert(e, Transform::from_pos(pos.extend(0.0)));
        w.insert(e, c);
        e
    }

    fn body(w: &mut World, pos: Vec2) -> Entity {
        let e = spawn(w, pos, Collider2D::rect(Vec2::ONE));
        w.insert(e, Body2D::default());
        e
    }

    fn pos(w: &World, e: Entity) -> Vec3 {
        w.get::<Transform>(e).unwrap().position
    }

    #[test]
    fn body_falls_and_lands_on_a_solid() {
        let mut w = world(Vec2::new(0.0, 10.0));
        let b = body(&mut w, Vec2::ZERO);
        spawn(
            &mut w,
            Vec2::new(0.0, 3.0),
            Collider2D::rect(Vec2::new(10.0, 1.0)),
        );
        for _ in 0..40 {
            physics2d_step(&mut w).unwrap();
        }
        assert!((pos(&w, b).y - 2.0).abs() < 1e-3);
        let state = w.get::<Body2D>(b).unwrap();
        assert!(state.on_ground);
        assert!(state.velocity.y.abs() < 1.5);
    }

    #[test]
    fn same_time_gives_the_same_result_at_any_frame_rate() {
        let fall = |frames: u32, dt: f32| {
            let mut w = world(Vec2::new(0.0, 10.0));
            w.insert_resource(Time {
                dt,
                ..Default::default()
            });
            let b = body(&mut w, Vec2::ZERO);
            for _ in 0..frames {
                physics2d_step(&mut w).unwrap();
            }
            pos(&w, b).y
        };
        assert!((fall(30, 1.0 / 30.0) - fall(240, 1.0 / 240.0)).abs() < 0.1);
    }

    #[test]
    fn render_offset_trails_the_moving_body_between_steps() {
        let mut w = world(Vec2::ZERO);
        w.insert_resource(Time {
            dt: 1.5 / 120.0,
            ..Default::default()
        });
        let b = body(&mut w, Vec2::ZERO);
        w.get_mut::<Body2D>(b).unwrap().velocity = Vec2::new(120.0, 0.0);
        physics2d_step(&mut w).unwrap();
        let offset = w.get::<RenderOffset>(b).unwrap().0.x;
        assert!((offset + 0.5).abs() < 1e-3, "{offset}");
    }

    #[test]
    fn teleporting_does_not_smear_the_body() {
        let mut w = world(Vec2::ZERO);
        let b = body(&mut w, Vec2::ZERO);
        physics2d_step(&mut w).unwrap();
        w.get_mut::<Transform>(b).unwrap().position.x = 500.0;
        w.insert_resource(Time::default());
        physics2d_step(&mut w).unwrap();
        assert_eq!(w.get::<RenderOffset>(b).unwrap().0, Vec3::ZERO);
    }

    #[test]
    fn wall_stops_only_the_blocked_axis() {
        let mut w = world(Vec2::ZERO);
        let b = body(&mut w, Vec2::ZERO);
        w.get_mut::<Body2D>(b).unwrap().velocity = Vec2::new(10.0, 5.0);
        spawn(
            &mut w,
            Vec2::new(2.0, 0.0),
            Collider2D::rect(Vec2::new(1.0, 100.0)),
        );
        for _ in 0..5 {
            physics2d_step(&mut w).unwrap();
        }
        let p = pos(&w, b);
        assert!((p.x - 1.0).abs() < 1e-3);
        assert!(p.y > 2.0);
    }

    #[test]
    fn blocked_tiles_stop_bodies() {
        let mut w = world(Vec2::new(0.0, 10.0));
        let b = body(&mut w, Vec2::ZERO);
        w.insert_resource(TileCollider::new(&Tilemap {
            origin: [0.0, 3.0],
            layers: vec![TileLayer {
                rows: vec!["#".into()],
                ..Default::default()
            }],
            tiles: BTreeMap::from([(
                "#".to_string(),
                TileDef {
                    collision: TileCollision::Block,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        }));
        for _ in 0..40 {
            physics2d_step(&mut w).unwrap();
        }
        assert!((pos(&w, b).y - 2.0).abs() < 1e-3);
    }

    #[test]
    fn sensors_overlap_but_do_not_block() {
        let mut w = world(Vec2::new(0.0, 10.0));
        let b = body(&mut w, Vec2::ZERO);
        let sensor = Collider2D::rect(Vec2::new(10.0, 1.0)).sensor();
        let s = spawn(&mut w, Vec2::new(0.0, 3.0), sensor);
        for _ in 0..40 {
            physics2d_step(&mut w).unwrap();
        }
        assert!(pos(&w, b).y > 3.0);
        assert!(overlaps(&w, Shape2D::Circle(0.5), Vec2::new(0.0, 3.0)).contains(&s));
        assert!(raycast2d(&w, Vec2::new(0.0, 1.0), Vec2::Y, 3.0).is_none());
    }

    #[test]
    fn jump_speed_reaches_the_requested_height() {
        let physics = Physics2D::earth();
        let v = physics.jump_speed(3.0);
        let height = v * v / (2.0 * physics.gravity.y);
        assert!((height - 3.0).abs() < 1e-3);
    }

    #[test]
    fn circles_and_boxes_overlap() {
        let mut w = world(Vec2::ZERO);
        let e = spawn(&mut w, Vec2::ZERO, Collider2D::rect(Vec2::splat(2.0)));
        assert_eq!(overlaps(&w, Shape2D::Circle(1.0), Vec2::new(1.5, 0.0)), [e]);
        assert!(overlaps(&w, Shape2D::Circle(0.4), Vec2::new(1.5, 0.0)).is_empty());
    }

    #[test]
    fn raycast_returns_the_nearest_hit() {
        let mut w = world(Vec2::ZERO);
        let far = spawn(&mut w, Vec2::new(10.0, 0.0), Collider2D::circle(1.0));
        let near = spawn(
            &mut w,
            Vec2::new(5.0, 0.0),
            Collider2D::rect(Vec2::splat(2.0)),
        );
        let hit = raycast2d(&w, Vec2::ZERO, Vec2::X, 20.0).unwrap();
        assert_eq!(hit.entity, near);
        assert!((hit.distance - 4.0).abs() < 1e-3);
        assert_ne!(hit.entity, far);
        assert!(raycast2d(&w, Vec2::ZERO, Vec2::X, 2.0).is_none());
    }
}
