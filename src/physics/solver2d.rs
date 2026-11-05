use glam::Vec2;

use crate::ecs::prelude::*;
use crate::ecs::{FastMap, Transform};

use super::d2::{Body2D, Collider2D, Placed, Shape2D, WallGrid, bounds};

const SLOP: f32 = 0.001;
const CONTACT_HERTZ: f32 = 30.0;
const CONTACT_DAMPING: f32 = 10.0;
const MAX_PUSH_SPEED: f32 = 12.0;
const SPECULATIVE: f32 = 0.02;
const SLEEP_SPEED: f32 = 0.3;
const SLEEP_TIME: f32 = 0.4;
const WAKE_SPEED: f32 = 1.5;
const MAX_CELLS_PER_BODY: usize = 8;
const MAX_BODY_REACH: f32 = 2.0;
const WALL_KEY: u64 = 1 << 63;

#[derive(Resource, Default)]
#[resource(core)]
pub(super) struct ContactCache(FastMap<u64, [f32; 2]>);

struct Contact {
    a: u32,
    b: u32,
    key: u64,
    normal: Vec2,
    separation: f32,
    mass: f32,
    friction: f32,
    normal_impulse: f32,
    tangent_impulse: f32,
}

struct Soft {
    bias_rate: f32,
    mass_scale: f32,
    impulse_scale: f32,
}

impl Soft {
    fn new(h: f32) -> Self {
        let hertz = CONTACT_HERTZ.min(0.25 / h);
        let omega = std::f32::consts::TAU * hertz;
        let a1 = 2.0 * CONTACT_DAMPING + h * omega;
        let a2 = h * omega * a1;
        let a3 = 1.0 / (1.0 + a2);
        Soft {
            bias_rate: omega / a1,
            mass_scale: a2 * a3,
            impulse_scale: a3,
        }
    }
}

fn gap_box(d: Vec2, half: Vec2) -> (Vec2, f32) {
    let sign = |v: f32| if v < 0.0 { -1.0 } else { 1.0 };
    let space = d.abs() - half;
    if space.x > 0.0 && space.y > 0.0 {
        let normal = Vec2::new(space.x * sign(d.x), space.y * sign(d.y));
        let length = normal.length();
        return (normal / length, length);
    }
    if space.x > space.y {
        (Vec2::new(sign(d.x), 0.0), space.x)
    } else {
        (Vec2::new(0.0, sign(d.y)), space.y)
    }
}

fn gap((shape_a, pa): Placed, (shape_b, pb): Placed) -> (Vec2, f32) {
    match (shape_a, shape_b) {
        (Shape2D::Circle(ra), Shape2D::Circle(rb)) => {
            let d = pa - pb;
            let length = d.length();
            let normal = if length > 1e-6 { d / length } else { Vec2::Y };
            (normal, length - ra - rb)
        }
        (Shape2D::Box(sa), Shape2D::Box(sb)) => gap_box(pa - pb, (sa + sb) / 2.0),
        (Shape2D::Circle(r), Shape2D::Box(s)) => {
            let (normal, separation) = gap_box(pa - pb, s / 2.0);
            (normal, separation - r)
        }
        (Shape2D::Box(s), Shape2D::Circle(r)) => {
            let (normal, separation) = gap_box(pb - pa, s / 2.0);
            (-normal, separation - r)
        }
    }
}

struct BodyGrid {
    origin: Vec2,
    cell: f32,
    cols: usize,
    rows: usize,
    starts: Vec<u32>,
    items: Vec<u32>,
}

impl BodyGrid {
    fn new(placed: &[Placed], reach: f32) -> Self {
        let mut lo = Vec2::splat(f32::MAX);
        let mut hi = Vec2::splat(f32::MIN);
        let mut widest = 0.01f32;
        for &p in placed {
            let (a, b) = bounds(p);
            lo = lo.min(a);
            hi = hi.max(b);
            widest = widest.max((b - a).max_element());
        }
        let span = (hi - lo).max(Vec2::splat(widest));
        let budget = (placed.len() * MAX_CELLS_PER_BODY).max(1) as f32;
        let cell = (widest + reach).max((span.x * span.y / budget).sqrt());
        let cols = (span.x / cell) as usize + 1;
        let rows = (span.y / cell) as usize + 1;
        let mut grid = BodyGrid {
            origin: lo,
            cell,
            cols,
            rows,
            starts: vec![0; cols * rows + 1],
            items: vec![0; placed.len()],
        };
        let slots: Vec<usize> = placed.iter().map(|p| grid.slot(p.1)).collect();
        for &slot in &slots {
            grid.starts[slot + 1] += 1;
        }
        for i in 1..grid.starts.len() {
            grid.starts[i] += grid.starts[i - 1];
        }
        let mut cursor = grid.starts.clone();
        for (i, &slot) in slots.iter().enumerate() {
            grid.items[cursor[slot] as usize] = i as u32;
            cursor[slot] += 1;
        }
        grid
    }

    fn coords(&self, pos: Vec2) -> (usize, usize) {
        let c = ((pos - self.origin) / self.cell).max(Vec2::ZERO);
        (
            (c.x as usize).min(self.cols - 1),
            (c.y as usize).min(self.rows - 1),
        )
    }

    fn slot(&self, pos: Vec2) -> usize {
        let (x, y) = self.coords(pos);
        y * self.cols + x
    }

    fn neighbours(&self, pos: Vec2, reach: f32, mut visit: impl FnMut(usize)) {
        let (x, y) = self.coords(pos);
        let ring = 1 + (reach / self.cell).ceil() as usize;
        for cy in y.saturating_sub(ring)..=(y + ring).min(self.rows - 1) {
            let (from, to) = (x.saturating_sub(ring), (x + ring).min(self.cols - 1));
            let start = self.starts[cy * self.cols + from] as usize;
            let end = self.starts[cy * self.cols + to + 1] as usize;
            for &i in &self.items[start..end] {
                visit(i as usize);
            }
        }
    }
}

pub(super) struct Sim {
    entities: Vec<Entity>,
    colliders: Vec<Collider2D>,
    bodies: Vec<Body2D>,
    pos: Vec<Vec2>,
    vel: Vec<Vec2>,
    moved: Vec<Vec2>,
    inv_mass: Vec<f32>,
    dormant: Vec<bool>,
    contacts: Vec<Contact>,
}

impl Sim {
    pub(super) fn load(world: &World) -> Self {
        let mut sim = Sim {
            entities: Vec::new(),
            colliders: Vec::new(),
            bodies: Vec::new(),
            pos: Vec::new(),
            vel: Vec::new(),
            moved: Vec::new(),
            inv_mass: Vec::new(),
            dormant: Vec::new(),
            contacts: Vec::new(),
        };
        for (e, body, collider, t) in world.query::<(&Body2D, &Collider2D, &Transform)>() {
            let mut body = *body;
            let pos = t.position.truncate();
            if body.asleep && (body.velocity != Vec2::ZERO || pos != body.previous) {
                body.asleep = false;
                body.still = 0.0;
            }
            sim.dormant.push(body.asleep);
            sim.entities.push(e);
            sim.colliders.push(*collider);
            sim.pos.push(pos);
            sim.vel.push(body.velocity);
            sim.moved.push(Vec2::ZERO);
            sim.inv_mass.push(if body.asleep {
                0.0
            } else {
                1.0 / body.mass.max(1e-4)
            });
            sim.bodies.push(body);
        }
        sim.pos.push(Vec2::ZERO);
        sim.vel.push(Vec2::ZERO);
        sim.moved.push(Vec2::ZERO);
        sim.inv_mass.push(0.0);
        sim
    }

    pub(super) fn store(&self, world: &mut World) {
        for (i, body) in self.bodies.iter().enumerate() {
            if body.asleep && self.dormant[i] {
                continue;
            }
            let e = self.entities[i];
            if let Some(t) = world.get_mut::<Transform>(e) {
                t.position.x = self.pos[i].x;
                t.position.y = self.pos[i].y;
            }
            if let Some(current) = world.get_mut::<Body2D>(e) {
                *current = *body;
                current.velocity = self.vel[i];
            }
        }
    }

    fn count(&self) -> usize {
        self.bodies.len()
    }

    fn placed(&self, i: usize) -> Placed {
        (
            self.colliders[i].shape,
            self.pos[i] + self.colliders[i].offset,
        )
    }

    fn wake(&mut self, i: usize) {
        if self.bodies[i].asleep {
            self.bodies[i].asleep = false;
            self.bodies[i].still = 0.0;
            self.inv_mass[i] = 1.0 / self.bodies[i].mass.max(1e-4);
        }
    }

    fn add_contact(
        &mut self,
        (a, b): (usize, usize),
        key: u64,
        (normal, separation): (Vec2, f32),
        friction: f32,
        cache: &ContactCache,
    ) {
        let total = self.inv_mass[a] + self.inv_mass[b];
        if total <= 0.0 {
            return;
        }
        let [normal_impulse, tangent_impulse] = cache.0.get(&key).copied().unwrap_or_default();
        self.contacts.push(Contact {
            a: a as u32,
            b: b as u32,
            key,
            normal,
            separation,
            mass: 1.0 / total,
            friction,
            normal_impulse,
            tangent_impulse,
        });
    }

    fn find_contacts(&mut self, grids: [&WallGrid; 2], step: f32, cache: &ContactCache) {
        self.contacts.clear();
        let n = self.count();
        let anchor = n;
        let solid: Vec<usize> = (0..n).filter(|&i| !self.colliders[i].sensor).collect();
        let placed: Vec<Placed> = solid.iter().map(|&i| self.placed(i)).collect();
        let grid = BodyGrid::new(&placed, SPECULATIVE);
        let mut near = Vec::new();
        let mut wake = Vec::new();

        for i in 0..n {
            if self.bodies[i].asleep {
                continue;
            }
            let margin = self.vel[i].length() * step + SPECULATIVE;
            let (lo, hi) = bounds(self.placed(i));
            let area = (lo - margin, hi + margin);
            let id = self.entities[i].id as u64;
            for (g, walls) in grids.iter().enumerate() {
                if walls.walls.is_empty() {
                    continue;
                }
                walls.nearby(area, &mut near);
                for &w in near.iter() {
                    let found = gap(self.placed(i), walls.walls[w as usize]);
                    if found.1 < margin {
                        let key = WALL_KEY | (g as u64) << 62 | id << 30 | w as u64;
                        let grip = self.bodies[i].friction;
                        self.add_contact((i, anchor), key, found, grip, cache);
                    }
                }
            }
            if self.colliders[i].sensor {
                continue;
            }
            let mine = self.placed(i);
            let mut pairs = std::mem::take(&mut wake);
            let speed = self.vel[i].length();
            let (mine_lo, mine_hi) = bounds(mine);
            let size = (mine_hi - mine_lo).max_element();
            let reach = (margin * 2.0).min(size * MAX_BODY_REACH);
            grid.neighbours(mine.1, reach, |k| {
                let j = solid[k];
                if j == i {
                    return;
                }
                let theirs = self.vel[j].length();
                let owned = speed > theirs || (speed == theirs && i < j);
                if !owned && !self.bodies[j].asleep {
                    return;
                }
                let found = gap(mine, placed[k]);
                let other = self.vel[j].length() * step;
                if found.1 < (margin + other).min(reach) {
                    pairs.push((j, found));
                }
            });
            for (j, found) in pairs.drain(..) {
                if self.bodies[j].asleep && speed > WAKE_SPEED && found.1 < margin {
                    self.wake(j);
                }
                let (lo_id, hi_id) = {
                    let (x, y) = (self.entities[i].id as u64, self.entities[j].id as u64);
                    (x.min(y), x.max(y))
                };
                let key = lo_id << 31 | hi_id;
                let sign = if self.entities[i].id as u64 == lo_id {
                    1.0
                } else {
                    -1.0
                };
                let grip = (self.bodies[i].friction + self.bodies[j].friction) / 2.0;
                let (a, b) = if sign > 0.0 { (i, j) } else { (j, i) };
                self.add_contact((a, b), key, (found.0 * sign, found.1), grip, cache);
            }
            wake = pairs;
        }
    }

    fn warm_start(&mut self) {
        for c in &self.contacts {
            let (a, b) = (c.a as usize, c.b as usize);
            let impulse = c.normal * c.normal_impulse + c.normal.perp() * c.tangent_impulse;
            self.vel[a] += impulse * self.inv_mass[a];
            self.vel[b] -= impulse * self.inv_mass[b];
        }
    }

    fn solve(&mut self, soft: &Soft, h: f32, use_bias: bool) {
        let inv_h = 1.0 / h;
        for c in &mut self.contacts {
            let (a, b) = (c.a as usize, c.b as usize);
            let (inv_a, inv_b) = (self.inv_mass[a], self.inv_mass[b]);
            let (mut va, mut vb) = (self.vel[a], self.vel[b]);
            let separation = c.separation + (self.moved[a] - self.moved[b]).dot(c.normal);

            let (bias, mass_scale, impulse_scale) = if separation > 0.0 {
                (separation * inv_h, 1.0, 0.0)
            } else if use_bias {
                let push = (soft.bias_rate * (separation + SLOP)).max(-MAX_PUSH_SPEED);
                (push.min(0.0), soft.mass_scale, soft.impulse_scale)
            } else {
                (0.0, 1.0, 0.0)
            };
            let speed = (va - vb).dot(c.normal);
            let impulse = -c.mass * mass_scale * (speed + bias) - impulse_scale * c.normal_impulse;
            let total = (c.normal_impulse + impulse).max(0.0);
            let delta = c.normal * (total - c.normal_impulse);
            c.normal_impulse = total;
            va += delta * inv_a;
            vb -= delta * inv_b;

            let tangent = c.normal.perp();
            let slide = (va - vb).dot(tangent);
            let limit = c.friction * c.normal_impulse;
            let total = (c.tangent_impulse - c.mass * slide).clamp(-limit, limit);
            let delta = tangent * (total - c.tangent_impulse);
            c.tangent_impulse = total;
            self.vel[a] = va + delta * inv_a;
            self.vel[b] = vb - delta * inv_b;
        }
    }

    fn update_sleep(&mut self, gravity: Vec2, dt: f32) {
        for i in 0..self.count() {
            let body = &mut self.bodies[i];
            if body.asleep || !body.can_sleep {
                continue;
            }
            let held = body.on_ground || gravity == Vec2::ZERO;
            let still = held && self.vel[i].length() < SLEEP_SPEED;
            body.still = if still { body.still + dt } else { 0.0 };
            if body.still >= SLEEP_TIME {
                body.asleep = true;
                body.previous = self.pos[i];
                self.vel[i] = Vec2::ZERO;
                self.inv_mass[i] = 0.0;
            }
        }
    }

    pub(super) fn step(
        &mut self,
        grids: [&WallGrid; 2],
        gravity: Vec2,
        dt: f32,
        substeps: u32,
        cache: &mut ContactCache,
    ) {
        let n = self.count();
        self.find_contacts(grids, dt, cache);
        let h = dt / substeps as f32;
        let soft = Soft::new(h);
        for i in 0..n {
            self.moved[i] = Vec2::ZERO;
            if !self.bodies[i].asleep {
                self.bodies[i].previous = self.pos[i];
            }
        }
        for _ in 0..substeps {
            for i in 0..n {
                if !self.bodies[i].asleep {
                    self.vel[i] += gravity * self.bodies[i].gravity_scale * h;
                }
            }
            self.warm_start();
            self.solve(&soft, h, true);
            for i in 0..n {
                self.moved[i] += self.vel[i] * h;
            }
            self.solve(&soft, h, false);
        }

        cache.0.clear();
        for i in 0..n {
            self.pos[i] += self.moved[i];
            self.bodies[i].on_ground = self.bodies[i].asleep && self.bodies[i].on_ground;
        }
        for c in &self.contacts {
            if c.normal_impulse <= 0.0 {
                continue;
            }
            cache.0.insert(c.key, [c.normal_impulse, c.tangent_impulse]);
            let facing = c.normal.dot(gravity);
            if facing < 0.0 {
                self.bodies[c.a as usize].on_ground = true;
            }
            if facing > 0.0 && (c.b as usize) < n {
                self.bodies[c.b as usize].on_ground = true;
            }
        }
        self.update_sleep(gravity, dt);
    }
}
