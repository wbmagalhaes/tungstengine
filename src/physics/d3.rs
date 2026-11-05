use glam::Vec3;

use crate::ecs::prelude::*;
use crate::ecs::{RenderOffset, Transform};
use crate::fixed_step::FixedStep;

use super::solver3d::{ContactCache, Grid, Sim};

const DEFAULT_SUBSTEPS: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape3D {
    Box(Vec3),
    Sphere(f32),
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Collider3D {
    pub shape: Shape3D,
    pub offset: Vec3,
    pub sensor: bool,
}

impl Collider3D {
    pub fn cuboid(size: Vec3) -> Self {
        Collider3D {
            shape: Shape3D::Box(size),
            offset: Vec3::ZERO,
            sensor: false,
        }
    }

    pub fn sphere(radius: f32) -> Self {
        Collider3D {
            shape: Shape3D::Sphere(radius),
            offset: Vec3::ZERO,
            sensor: false,
        }
    }

    pub fn offset(mut self, offset: Vec3) -> Self {
        self.offset = offset;
        self
    }

    pub fn sensor(mut self) -> Self {
        self.sensor = true;
        self
    }

    fn placed(&self, t: &Transform) -> Placed {
        (self.shape, t.position + self.offset)
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Body3D {
    pub velocity: Vec3,
    pub gravity_scale: f32,
    pub mass: f32,
    pub friction: f32,
    pub on_ground: bool,
    pub can_sleep: bool,
    pub asleep: bool,
    pub(super) still: f32,
    pub(super) previous: Vec3,
}

impl Default for Body3D {
    fn default() -> Self {
        Body3D {
            velocity: Vec3::ZERO,
            gravity_scale: 1.0,
            mass: 1.0,
            friction: 0.0,
            on_ground: false,
            can_sleep: false,
            asleep: false,
            still: 0.0,
            previous: Vec3::ZERO,
        }
    }
}

impl Body3D {
    pub fn friction(mut self, friction: f32) -> Self {
        self.friction = friction;
        self
    }

    pub fn sleepy(mut self) -> Self {
        self.can_sleep = true;
        self
    }
}

#[derive(Resource, Clone, Copy, Debug)]
#[resource(core)]
pub struct Physics3D {
    pub gravity: Vec3,
    pub timing: FixedStep,
    pub substeps: u32,
}

impl Default for Physics3D {
    fn default() -> Self {
        Physics3D {
            gravity: Vec3::ZERO,
            timing: FixedStep::default(),
            substeps: DEFAULT_SUBSTEPS,
        }
    }
}

impl Physics3D {
    pub fn with_gravity(gravity: Vec3) -> Self {
        Physics3D {
            gravity,
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit3D {
    pub entity: Entity,
    pub distance: f32,
    pub point: Vec3,
}

pub(super) type Placed = (Shape3D, Vec3);

fn box_push(a: Vec3, ha: Vec3, b: Vec3, hb: Vec3) -> Option<Vec3> {
    let d = a - b;
    let overlap = ha + hb - d.abs();
    if overlap.cmple(Vec3::ZERO).any() {
        return None;
    }
    let sign = |v: f32| if v < 0.0 { -1.0 } else { 1.0 };
    Some(if overlap.x <= overlap.y && overlap.x <= overlap.z {
        Vec3::X * overlap.x * sign(d.x)
    } else if overlap.y <= overlap.z {
        Vec3::Y * overlap.y * sign(d.y)
    } else {
        Vec3::Z * overlap.z * sign(d.z)
    })
}

fn sphere_box_push(c: Vec3, r: f32, b: Vec3, half: Vec3) -> Option<Vec3> {
    let closest = c.clamp(b - half, b + half);
    let d = c - closest;
    if d == Vec3::ZERO {
        return box_push(c, Vec3::splat(r), b, half);
    }
    let len = d.length();
    (len < r).then(|| d / len * (r - len))
}

pub fn push_out(a: Placed, b: Placed) -> Option<Vec3> {
    match (a, b) {
        ((Shape3D::Box(sa), pa), (Shape3D::Box(sb), pb)) => box_push(pa, sa / 2.0, pb, sb / 2.0),
        ((Shape3D::Sphere(r), pa), (Shape3D::Box(s), pb)) => sphere_box_push(pa, r, pb, s / 2.0),
        ((Shape3D::Box(s), pa), (Shape3D::Sphere(r), pb)) => {
            sphere_box_push(pb, r, pa, s / 2.0).map(|p| -p)
        }
        ((Shape3D::Sphere(ra), pa), (Shape3D::Sphere(rb), pb)) => {
            let d = pa - pb;
            let len = d.length();
            let depth = ra + rb - len;
            (depth > 0.0).then(|| {
                let normal = if len > 0.0 { d / len } else { Vec3::Y };
                normal * depth
            })
        }
    }
}

fn solids(world: &World) -> Vec<Placed> {
    world
        .query::<(&Collider3D, &Transform)>()
        .filter(|(e, c, _)| !c.sensor && !world.has::<Body3D>(*e))
        .map(|(_, c, t)| c.placed(t))
        .collect()
}

impl World {
    pub fn add_physics3d(&mut self) {
        self.add_system_to(Stage::Physics, physics3d_step);
    }

    pub fn add_physics3d_if(&mut self, condition: impl Fn(&World) -> bool + 'static) {
        self.add_system_to_if(Stage::Physics, condition, physics3d_step);
    }
}

pub fn physics3d_step(world: &mut World) -> SysResult {
    let dt = world.dt();
    let physics = world.resource_core_mut::<Physics3D>();
    let steps = physics.timing.advance(dt);
    let (gravity, step, alpha) = (physics.gravity, physics.timing.step, physics.timing.alpha());
    let substeps = physics.substeps.max(1);
    if steps > 0 {
        let walls = Grid::walls(solids(world));
        let mut cache = std::mem::take(world.resource_core_mut::<ContactCache>());
        let mut sim = Sim::load(world);
        for _ in 0..steps {
            sim.step(&walls, gravity, step, substeps, &mut cache);
        }
        sim.store(world);
        *world.resource_core_mut::<ContactCache>() = cache;
    }
    interpolate(world, alpha, step);
    Ok(())
}

fn interpolate(world: &mut World, alpha: f32, step: f32) {
    let offsets: Vec<(Entity, Vec3)> = world
        .query::<(&Body3D, &Transform)>()
        .map(|(e, b, t)| {
            let back = b.previous - t.position;
            let teleported = back.length() > b.velocity.length() * step * 2.0 + 1e-4;
            let offset = if teleported {
                Vec3::ZERO
            } else {
                back * (1.0 - alpha)
            };
            (e, offset)
        })
        .collect();
    for (e, offset) in offsets {
        match world.get_mut::<RenderOffset>(e) {
            Some(current) => current.0 = offset,
            None => world.insert(e, RenderOffset(offset)),
        }
    }
}

pub fn overlaps3d(world: &World, shape: Shape3D, pos: Vec3) -> Vec<Entity> {
    world
        .query::<(&Collider3D, &Transform)>()
        .filter(|(_, c, t)| push_out((shape, pos), c.placed(t)).is_some())
        .map(|(e, _, _)| e)
        .collect()
}

fn ray_box(origin: Vec3, dir: Vec3, center: Vec3, half: Vec3) -> Option<f32> {
    let inv = dir.recip();
    let a = (center - half - origin) * inv;
    let b = (center + half - origin) * inv;
    let near = a.min(b).max_element();
    let far = a.max(b).min_element();
    (far >= near.max(0.0)).then_some(near.max(0.0))
}

fn ray_sphere(origin: Vec3, dir: Vec3, center: Vec3, r: f32) -> Option<f32> {
    let to = origin - center;
    let b = to.dot(dir);
    let disc = b * b - (to.length_squared() - r * r);
    if disc < 0.0 {
        return None;
    }
    let far = -b + disc.sqrt();
    (far >= 0.0).then_some((-b - disc.sqrt()).max(0.0))
}

pub fn raycast3d(world: &World, origin: Vec3, dir: Vec3, max: f32) -> Option<RayHit3D> {
    let dir = dir.normalize_or_zero();
    world
        .query::<(&Collider3D, &Transform)>()
        .filter(|(_, c, _)| !c.sensor)
        .filter_map(|(entity, c, t)| {
            let (shape, pos) = c.placed(t);
            let distance = match shape {
                Shape3D::Box(size) => ray_box(origin, dir, pos, size / 2.0),
                Shape3D::Sphere(r) => ray_sphere(origin, dir, pos, r),
            }?;
            (distance <= max).then_some(RayHit3D {
                entity,
                distance,
                point: origin + dir * distance,
            })
        })
        .min_by(|a, b| a.distance.total_cmp(&b.distance))
}

pub mod prelude {
    pub use super::{
        Body3D, Collider3D, Physics3D, RayHit3D, Shape3D, overlaps3d, physics3d_step, raycast3d,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Time;

    fn world(gravity: Vec3) -> World {
        let mut w = World::new();
        w.insert_resource(Time {
            dt: 0.1,
            ..Default::default()
        });
        w.insert_resource(Physics3D::with_gravity(gravity));
        w
    }

    fn spawn(w: &mut World, pos: Vec3, c: Collider3D) -> Entity {
        let e = w.spawn();
        w.insert(e, Transform::from_pos(pos));
        w.insert(e, c);
        e
    }

    fn body(w: &mut World, pos: Vec3) -> Entity {
        let e = spawn(w, pos, Collider3D::cuboid(Vec3::ONE));
        w.insert(e, Body3D::default());
        e
    }

    #[test]
    fn body_falls_and_lands_on_the_ground() {
        let mut w = world(Vec3::new(0.0, -10.0, 0.0));
        let b = body(&mut w, Vec3::new(0.0, 3.0, 0.0));
        spawn(
            &mut w,
            Vec3::new(0.0, -0.5, 0.0),
            Collider3D::cuboid(Vec3::new(20.0, 1.0, 20.0)),
        );
        for _ in 0..40 {
            physics3d_step(&mut w).unwrap();
        }
        let y = w.get::<Transform>(b).unwrap().position.y;
        assert!((y - 0.5).abs() < 1e-2, "{y}");
        assert!(w.get::<Body3D>(b).unwrap().on_ground);
    }

    #[test]
    fn bodies_stack_push_and_sleep() {
        let mut w = world(Vec3::new(0.0, -10.0, 0.0));
        spawn(
            &mut w,
            Vec3::new(0.0, -0.5, 0.0),
            Collider3D::cuboid(Vec3::new(10.0, 1.0, 10.0)),
        );
        let lower = body(&mut w, Vec3::new(0.0, 0.6, 0.0));
        let upper = body(&mut w, Vec3::new(0.0, 1.8, 0.0));
        w.insert(upper, Body3D::default().sleepy());
        for _ in 0..30 {
            physics3d_step(&mut w).unwrap();
        }
        let gap = w.get::<Transform>(upper).unwrap().position.y
            - w.get::<Transform>(lower).unwrap().position.y;
        assert!((gap - 1.0).abs() < 0.05, "{gap}");
        assert!(w.get::<Body3D>(upper).unwrap().on_ground);
        assert!(w.get::<Body3D>(upper).unwrap().asleep);
    }

    #[test]
    fn sensors_do_not_block_bodies() {
        let mut w = world(Vec3::new(0.0, -10.0, 0.0));
        let b = body(&mut w, Vec3::new(0.0, 2.0, 0.0));
        spawn(
            &mut w,
            Vec3::ZERO,
            Collider3D::cuboid(Vec3::new(10.0, 1.0, 10.0)).sensor(),
        );
        for _ in 0..20 {
            physics3d_step(&mut w).unwrap();
        }
        assert!(w.get::<Transform>(b).unwrap().position.y < -1.0);
    }

    #[test]
    fn fast_bodies_do_not_pass_through_walls() {
        let mut w = world(Vec3::ZERO);
        let b = body(&mut w, Vec3::ZERO);
        w.get_mut::<Body3D>(b).unwrap().velocity = Vec3::X * 500.0;
        spawn(
            &mut w,
            Vec3::X * 5.0,
            Collider3D::cuboid(Vec3::new(1.0, 50.0, 50.0)),
        );
        for _ in 0..5 {
            physics3d_step(&mut w).unwrap();
        }
        assert!(w.get::<Transform>(b).unwrap().position.x < 5.0);
    }

    #[test]
    fn sphere_and_box_overlap_and_ray_hits_nearest() {
        let mut w = world(Vec3::ZERO);
        let near = spawn(&mut w, Vec3::X * 5.0, Collider3D::cuboid(Vec3::splat(2.0)));
        spawn(&mut w, Vec3::X * 10.0, Collider3D::sphere(1.0));
        assert_eq!(overlaps3d(&w, Shape3D::Sphere(1.0), Vec3::X * 3.5), [near]);
        let hit = raycast3d(&w, Vec3::ZERO, Vec3::X, 20.0).unwrap();
        assert_eq!(hit.entity, near);
        assert!((hit.distance - 4.0).abs() < 1e-3);
    }
}
