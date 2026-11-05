use glam::{Vec2, Vec3};

use crate::ecs::FastMap;
use crate::ecs::prelude::*;
use crate::ecs::{RenderOffset, Transform};
use crate::fixed_step::FixedStep;
use crate::tilemap::Tilemap;

use super::solver2d::{ContactCache, Sim};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape2D {
    Box(Vec2),
    Circle(f32),
}

const GRID_CELL: f32 = 4.0;
const DEFAULT_SUBSTEPS: u32 = 2;
const MAX_WALL_CELLS: i64 = 64;

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

    fn placed(&self, t: &Transform) -> Placed {
        (self.shape, t.position.truncate() + self.offset)
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Body2D {
    pub velocity: Vec2,
    pub gravity_scale: f32,
    pub mass: f32,
    pub friction: f32,
    pub on_ground: bool,
    pub can_sleep: bool,
    pub asleep: bool,
    pub(super) still: f32,
    pub(super) previous: Vec2,
}

impl Default for Body2D {
    fn default() -> Self {
        Body2D {
            velocity: Vec2::ZERO,
            gravity_scale: 1.0,
            mass: 1.0,
            friction: 0.0,
            on_ground: false,
            can_sleep: false,
            asleep: false,
            still: 0.0,
            previous: Vec2::ZERO,
        }
    }
}

impl Body2D {
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
pub struct Physics2D {
    pub gravity: Vec2,
    pub timing: FixedStep,
    pub substeps: u32,
}

impl Default for Physics2D {
    fn default() -> Self {
        Physics2D {
            gravity: Vec2::ZERO,
            timing: FixedStep::default(),
            substeps: DEFAULT_SUBSTEPS,
        }
    }
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
pub struct TileCollider(WallGrid);

impl TileCollider {
    pub fn new(map: &Tilemap) -> Self {
        TileCollider(WallGrid::new(blocked_tiles(map)))
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct WallGrid {
    pub(super) walls: Vec<Placed>,
    cells: FastMap<(i32, i32), Vec<u32>>,
    large: Vec<u32>,
}

impl WallGrid {
    fn new(walls: Vec<Placed>) -> Self {
        let mut grid = WallGrid {
            walls,
            ..Default::default()
        };
        for (i, &wall) in grid.walls.iter().enumerate() {
            let (lo, hi) = cell_range(bounds(wall));
            let covered = (hi.0 - lo.0 + 1) as i64 * (hi.1 - lo.1 + 1) as i64;
            if covered > MAX_WALL_CELLS {
                grid.large.push(i as u32);
                continue;
            }
            for y in lo.1..=hi.1 {
                for x in lo.0..=hi.0 {
                    grid.cells.entry((x, y)).or_default().push(i as u32);
                }
            }
        }
        grid
    }

    pub(super) fn nearby(&self, area: (Vec2, Vec2), out: &mut Vec<u32>) {
        out.clear();
        out.extend_from_slice(&self.large);
        if self.cells.is_empty() {
            return;
        }
        let (lo, hi) = cell_range(area);
        for y in lo.1..=hi.1 {
            for x in lo.0..=hi.0 {
                if let Some(cell) = self.cells.get(&(x, y)) {
                    out.extend_from_slice(cell);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
    }
}

pub(super) fn bounds((shape, pos): Placed) -> (Vec2, Vec2) {
    let half = match shape {
        Shape2D::Box(size) => size / 2.0,
        Shape2D::Circle(r) => Vec2::splat(r),
    };
    (pos - half, pos + half)
}

fn cell_range((lo, hi): (Vec2, Vec2)) -> ((i32, i32), (i32, i32)) {
    let cell = |p: Vec2| {
        let c = (p / GRID_CELL).floor();
        (c.x as i32, c.y as i32)
    };
    (cell(lo), cell(hi))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit2D {
    pub entity: Entity,
    pub distance: f32,
    pub point: Vec2,
}

pub(super) type Placed = (Shape2D, Vec2);

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

impl World {
    pub fn add_physics2d(&mut self) {
        self.add_system_to(Stage::Physics, physics2d_step);
    }
}

pub fn physics2d_step(world: &mut World) -> SysResult {
    let dt = world.dt();
    let physics = world.resource_core_mut::<Physics2D>();
    let steps = physics.timing.advance(dt);
    let (gravity, step, alpha) = (physics.gravity, physics.timing.step, physics.timing.alpha());
    let substeps = physics.substeps.max(1);
    if steps == 0 {
        interpolate(world, alpha, step);
        return Ok(());
    }
    let walls = WallGrid::new(solids(world).into_iter().map(|(_, p)| p).collect());
    let tiles = world.resource_mut::<TileCollider>().map(std::mem::take);
    let empty = TileCollider::default();
    let tile_grid = &tiles.as_ref().unwrap_or(&empty).0;
    let mut cache = std::mem::take(world.resource_core_mut::<ContactCache>());
    let mut sim = Sim::load(world);
    for _ in 0..steps {
        sim.step([&walls, tile_grid], gravity, step, substeps, &mut cache);
    }
    sim.store(world);
    *world.resource_core_mut::<ContactCache>() = cache;
    if let Ok(tiles) = tiles {
        world.insert_resource(tiles);
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
        match world.get_mut::<RenderOffset>(e) {
            Some(current) => current.0 = offset,
            None => world.insert(e, RenderOffset(offset)),
        }
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
        assert!((pos(&w, b).y - 2.0).abs() < 1e-2, "{}", pos(&w, b).y);
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
        assert!((pos(&w, b).y - 2.0).abs() < 1e-2, "{}", pos(&w, b).y);
    }

    #[test]
    fn grid_finds_walls_across_cells_and_huge_walls() {
        let small = (Shape2D::Box(Vec2::new(10.0, 1.0)), Vec2::new(0.0, 0.0));
        let far = (Shape2D::Box(Vec2::ONE), Vec2::new(100.0, 100.0));
        let huge = (Shape2D::Box(Vec2::new(4000.0, 1.0)), Vec2::new(0.0, 50.0));
        let grid = WallGrid::new(vec![small, far, huge]);
        assert_eq!(grid.large, [2]);
        let mut near = Vec::new();
        grid.nearby((Vec2::new(4.5, -0.5), Vec2::new(4.6, 0.5)), &mut near);
        assert_eq!(near, [0, 2]);
        grid.nearby((Vec2::new(99.0, 99.0), Vec2::new(101.0, 101.0)), &mut near);
        assert_eq!(near, [1, 2]);
    }

    #[test]
    fn body_lands_on_a_floor_much_wider_than_the_grid() {
        let mut w = world(Vec2::new(0.0, 10.0));
        let b = body(&mut w, Vec2::new(1500.0, 0.0));
        spawn(
            &mut w,
            Vec2::new(0.0, 3.0),
            Collider2D::rect(Vec2::new(4000.0, 1.0)),
        );
        for _ in 0..40 {
            physics2d_step(&mut w).unwrap();
        }
        assert!((pos(&w, b).y - 2.0).abs() < 1e-2, "{}", pos(&w, b).y);
    }

    #[test]
    fn equal_bodies_stop_each_other_head_on() {
        let mut w = world(Vec2::ZERO);
        let a = body(&mut w, Vec2::new(-3.0, 0.0));
        let b = body(&mut w, Vec2::new(3.0, 0.0));
        w.get_mut::<Body2D>(a).unwrap().velocity.x = 5.0;
        w.get_mut::<Body2D>(b).unwrap().velocity.x = -5.0;
        for _ in 0..20 {
            physics2d_step(&mut w).unwrap();
        }
        assert!((pos(&w, b).x - pos(&w, a).x - 1.0).abs() < 1e-2);
        assert!(w.get::<Body2D>(a).unwrap().velocity.x.abs() < 1e-3);
    }

    #[test]
    fn heavy_bodies_shove_light_ones() {
        let mut w = world(Vec2::ZERO);
        let heavy = body(&mut w, Vec2::ZERO);
        let light = body(&mut w, Vec2::new(2.0, 0.0));
        w.get_mut::<Body2D>(heavy).unwrap().mass = 9.0;
        w.get_mut::<Body2D>(heavy).unwrap().velocity.x = 10.0;
        for _ in 0..10 {
            physics2d_step(&mut w).unwrap();
        }
        assert!(w.get::<Body2D>(heavy).unwrap().velocity.x > 8.0);
        assert!(pos(&w, light).x > 3.0);
    }

    #[test]
    fn bodies_stack_and_the_top_one_counts_as_grounded() {
        let mut w = world(Vec2::new(0.0, 10.0));
        spawn(
            &mut w,
            Vec2::new(0.0, 3.0),
            Collider2D::rect(Vec2::new(10.0, 1.0)),
        );
        let lower = body(&mut w, Vec2::new(0.0, 2.0));
        let upper = body(&mut w, Vec2::new(0.0, 0.0));
        for _ in 0..60 {
            physics2d_step(&mut w).unwrap();
        }
        assert!((pos(&w, lower).y - 2.0).abs() < 0.05);
        assert!((pos(&w, upper).y - 1.0).abs() < 0.1);
        assert!(w.get::<Body2D>(upper).unwrap().on_ground);
    }

    #[test]
    fn a_shoved_body_does_not_go_through_a_wall() {
        let mut w = world(Vec2::ZERO);
        let pusher = body(&mut w, Vec2::ZERO);
        let pushed = body(&mut w, Vec2::new(1.5, 0.0));
        spawn(
            &mut w,
            Vec2::new(3.0, 0.0),
            Collider2D::rect(Vec2::new(1.0, 100.0)),
        );
        w.get_mut::<Body2D>(pusher).unwrap().mass = 2.0;
        for _ in 0..20 {
            w.get_mut::<Body2D>(pusher).unwrap().velocity.x = 20.0;
            physics2d_step(&mut w).unwrap();
        }
        assert!(pos(&w, pushed).x <= 2.05, "{}", pos(&w, pushed).x);
    }

    #[test]
    fn sensor_bodies_pass_through_other_bodies() {
        let mut w = world(Vec2::ZERO);
        let a = body(&mut w, Vec2::ZERO);
        let b = spawn(
            &mut w,
            Vec2::new(2.0, 0.0),
            Collider2D::rect(Vec2::ONE).sensor(),
        );
        w.insert(b, Body2D::default());
        w.get_mut::<Body2D>(a).unwrap().velocity.x = 10.0;
        for _ in 0..10 {
            physics2d_step(&mut w).unwrap();
        }
        assert!(pos(&w, a).x > 5.0);
    }

    #[test]
    fn distant_bodies_keep_the_dense_grid_small() {
        let mut w = world(Vec2::ZERO);
        let a = body(&mut w, Vec2::ZERO);
        let b = body(&mut w, Vec2::new(0.5, 0.0));
        let far = body(&mut w, Vec2::new(1.0e6, 1.0e6));
        for _ in 0..10 {
            physics2d_step(&mut w).unwrap();
        }
        assert!(pos(&w, b).x - pos(&w, a).x > 0.9);
        assert_eq!(pos(&w, far).x, 1.0e6);
    }

    #[test]
    fn resting_sleepy_bodies_sleep_and_wake_when_hit() {
        let mut w = world(Vec2::new(0.0, 10.0));
        spawn(
            &mut w,
            Vec2::new(0.0, 3.0),
            Collider2D::rect(Vec2::new(10.0, 1.0)),
        );
        let a = body(&mut w, Vec2::new(0.0, 2.0));
        let b = body(&mut w, Vec2::new(4.0, 2.0));
        w.insert(a, Body2D::default().sleepy());
        for _ in 0..20 {
            physics2d_step(&mut w).unwrap();
        }
        assert!(w.get::<Body2D>(a).unwrap().asleep);
        assert!(!w.get::<Body2D>(b).unwrap().asleep);
        w.get_mut::<Body2D>(b).unwrap().velocity.x = -20.0;
        for _ in 0..4 {
            physics2d_step(&mut w).unwrap();
        }
        assert!(pos(&w, a).x < -0.05, "{}", pos(&w, a).x);
    }

    #[test]
    fn setting_velocity_wakes_a_sleeper() {
        let mut w = world(Vec2::ZERO);
        let a = body(&mut w, Vec2::ZERO);
        let other = body(&mut w, Vec2::new(50.0, 0.0));
        w.insert(a, Body2D::default().sleepy());
        for _ in 0..20 {
            physics2d_step(&mut w).unwrap();
        }
        assert!(w.get::<Body2D>(a).unwrap().asleep);
        w.get_mut::<Body2D>(a).unwrap().velocity.x = 5.0;
        physics2d_step(&mut w).unwrap();
        assert!(pos(&w, a).x > 0.0);
        assert!(pos(&w, other).x == 50.0);
    }

    #[test]
    fn a_symmetric_pile_does_not_drift_sideways() {
        let mut w = world(Vec2::new(0.0, 30.0));
        spawn(
            &mut w,
            Vec2::new(0.0, 12.0),
            Collider2D::rect(Vec2::new(200.0, 1.0)),
        );
        let mut grains = Vec::new();
        for row in 0..12 {
            for col in 0..31 {
                let x = (col as f32 - 15.0) * 1.05;
                grains.push(body(&mut w, Vec2::new(x, 10.0 - row as f32 * 1.05)));
            }
        }
        let center =
            |w: &World| grains.iter().map(|&g| pos(w, g).x).sum::<f32>() / grains.len() as f32;
        for _ in 0..120 {
            physics2d_step(&mut w).unwrap();
        }
        assert!(center(&w).abs() < 0.3, "{}", center(&w));
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
