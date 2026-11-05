use glam::Vec3;

use crate::ecs::prelude::*;
use crate::ecs::{FastMap, Transform};

use super::d3::{Body3D, Collider3D, Placed, Shape3D};

const SLOP: f32 = 0.001;
const CONTACT_HERTZ: f32 = 30.0;
const CONTACT_DAMPING: f32 = 10.0;
const MAX_PUSH_SPEED: f32 = 12.0;
const SPECULATIVE: f32 = 0.02;
const SLEEP_SPEED: f32 = 0.3;
const SLEEP_TIME: f32 = 0.4;
const WAKE_SPEED: f32 = 1.5;
const MAX_WALL_DEPTH: f32 = 0.01;
const WALL_CELL: f32 = 4.0;
const MAX_WALL_CELLS: i64 = 512;
const WALL_KEY: u64 = 1 << 63;

type Cell = (i32, i32, i32);

#[derive(Resource, Default)]
#[resource(core)]
pub(super) struct ContactCache(FastMap<u64, [f32; 3]>);

fn bounds((shape, pos): Placed) -> (Vec3, Vec3) {
    let half = match shape {
        Shape3D::Box(size) => size / 2.0,
        Shape3D::Sphere(r) => Vec3::splat(r),
    };
    (pos - half, pos + half)
}

fn cell_of(p: Vec3, size: f32) -> Cell {
    let c = (p / size).floor();
    (c.x as i32, c.y as i32, c.z as i32)
}

#[derive(Default)]
pub(super) struct Grid {
    pub(super) items: Vec<Placed>,
    size: f32,
    cells: FastMap<Cell, Vec<u32>>,
    large: Vec<u32>,
}

impl Grid {
    pub(super) fn walls(items: Vec<Placed>) -> Self {
        Self::new(items, WALL_CELL, MAX_WALL_CELLS)
    }

    fn new(items: Vec<Placed>, size: f32, limit: i64) -> Self {
        let mut grid = Grid {
            items,
            size,
            ..Default::default()
        };
        for (i, &item) in grid.items.iter().enumerate() {
            let (lo, hi) = bounds(item);
            let (a, b) = (cell_of(lo, size), cell_of(hi, size));
            let covered = (b.0 - a.0 + 1) as i64 * (b.1 - a.1 + 1) as i64 * (b.2 - a.2 + 1) as i64;
            if covered > limit {
                grid.large.push(i as u32);
                continue;
            }
            for z in a.2..=b.2 {
                for y in a.1..=b.1 {
                    for x in a.0..=b.0 {
                        grid.cells.entry((x, y, z)).or_default().push(i as u32);
                    }
                }
            }
        }
        grid
    }

    fn nearby(&self, (lo, hi): (Vec3, Vec3), out: &mut Vec<u32>) {
        out.clear();
        out.extend_from_slice(&self.large);
        if self.cells.is_empty() {
            return;
        }
        let (a, b) = (cell_of(lo, self.size), cell_of(hi, self.size));
        for z in a.2..=b.2 {
            for y in a.1..=b.1 {
                for x in a.0..=b.0 {
                    if let Some(cell) = self.cells.get(&(x, y, z)) {
                        out.extend_from_slice(cell);
                    }
                }
            }
        }
        out.sort_unstable();
        out.dedup();
    }
}

struct Contact {
    a: u32,
    b: u32,
    key: u64,
    normal: Vec3,
    tangents: [Vec3; 2],
    separation: f32,
    mass: f32,
    friction: f32,
    normal_impulse: f32,
    tangent_impulse: [f32; 2],
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

fn sign(v: f32) -> f32 {
    if v < 0.0 { -1.0 } else { 1.0 }
}

fn gap_box(d: Vec3, half: Vec3) -> (Vec3, f32) {
    let space = d.abs() - half;
    let outside = space.max(Vec3::ZERO);
    if outside.length_squared() > 0.0 {
        let length = outside.length();
        let normal = outside * Vec3::new(sign(d.x), sign(d.y), sign(d.z)) / length;
        return (normal, length);
    }
    if space.x >= space.y && space.x >= space.z {
        (Vec3::X * sign(d.x), space.x)
    } else if space.y >= space.z {
        (Vec3::Y * sign(d.y), space.y)
    } else {
        (Vec3::Z * sign(d.z), space.z)
    }
}

fn gap((shape_a, pa): Placed, (shape_b, pb): Placed) -> (Vec3, f32) {
    match (shape_a, shape_b) {
        (Shape3D::Sphere(ra), Shape3D::Sphere(rb)) => {
            let d = pa - pb;
            let length = d.length();
            let normal = if length > 1e-6 { d / length } else { Vec3::Y };
            (normal, length - ra - rb)
        }
        (Shape3D::Box(sa), Shape3D::Box(sb)) => gap_box(pa - pb, (sa + sb) / 2.0),
        (Shape3D::Sphere(r), Shape3D::Box(s)) => {
            let (normal, separation) = gap_box(pa - pb, s / 2.0);
            (normal, separation - r)
        }
        (Shape3D::Box(s), Shape3D::Sphere(r)) => {
            let (normal, separation) = gap_box(pb - pa, s / 2.0);
            (-normal, separation - r)
        }
    }
}

pub(super) struct Sim {
    entities: Vec<Entity>,
    colliders: Vec<Collider3D>,
    bodies: Vec<Body3D>,
    pos: Vec<Vec3>,
    vel: Vec<Vec3>,
    moved: Vec<Vec3>,
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
        for (e, body, collider, t) in world.query::<(&Body3D, &Collider3D, &Transform)>() {
            let mut body = *body;
            if body.asleep && (body.velocity != Vec3::ZERO || t.position != body.previous) {
                body.asleep = false;
                body.still = 0.0;
            }
            sim.dormant.push(body.asleep);
            sim.entities.push(e);
            sim.colliders.push(*collider);
            sim.pos.push(t.position);
            sim.vel.push(body.velocity);
            sim.moved.push(Vec3::ZERO);
            sim.inv_mass.push(if body.asleep {
                0.0
            } else {
                1.0 / body.mass.max(1e-4)
            });
            sim.bodies.push(body);
        }
        sim.pos.push(Vec3::ZERO);
        sim.vel.push(Vec3::ZERO);
        sim.moved.push(Vec3::ZERO);
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
                t.position = self.pos[i];
            }
            if let Some(current) = world.get_mut::<Body3D>(e) {
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
        (normal, separation): (Vec3, f32),
        friction: f32,
        cache: &ContactCache,
    ) {
        let total = self.inv_mass[a] + self.inv_mass[b];
        if total <= 0.0 {
            return;
        }
        let [normal_impulse, t0, t1] = cache.0.get(&key).copied().unwrap_or_default();
        let (u, v) = normal.any_orthonormal_pair();
        self.contacts.push(Contact {
            a: a as u32,
            b: b as u32,
            key,
            normal,
            tangents: [u, v],
            separation,
            mass: 1.0 / total,
            friction,
            normal_impulse,
            tangent_impulse: [t0, t1],
        });
    }

    fn find_contacts(&mut self, walls: &Grid, step: f32, cache: &ContactCache) {
        self.contacts.clear();
        let n = self.count();
        let solid: Vec<usize> = (0..n).filter(|&i| !self.colliders[i].sensor).collect();
        let placed: Vec<Placed> = solid.iter().map(|&i| self.placed(i)).collect();
        let widest = placed
            .iter()
            .map(|&p| {
                let (lo, hi) = bounds(p);
                (hi - lo).max_element()
            })
            .fold(0.01, f32::max);
        let bodies = Grid::new(placed, widest + SPECULATIVE, 27);
        let mut near = Vec::new();
        let mut hits: Vec<(usize, (Vec3, f32))> = Vec::new();

        for i in 0..n {
            if self.bodies[i].asleep {
                continue;
            }
            let margin = self.vel[i].length() * step + SPECULATIVE;
            let mine = self.placed(i);
            let (lo, hi) = bounds(mine);
            let area = (lo - margin, hi + margin);
            let id = self.entities[i].id as u64;
            walls.nearby(area, &mut near);
            for &w in near.iter() {
                let found = gap(mine, walls.items[w as usize]);
                if found.1 < margin {
                    let key = WALL_KEY | id << 31 | w as u64;
                    self.add_contact((i, n), key, found, self.bodies[i].friction, cache);
                }
            }
            if self.colliders[i].sensor {
                continue;
            }
            bodies.nearby(area, &mut near);
            hits.clear();
            for &k in near.iter() {
                let j = solid[k as usize];
                if j == i {
                    continue;
                }
                let speed = self.vel[i].length();
                let theirs = self.vel[j].length();
                let owned = speed > theirs || (speed == theirs && i < j);
                if !owned && !self.bodies[j].asleep {
                    continue;
                }
                let found = gap(mine, bodies.items[k as usize]);
                if found.1 < margin + theirs * step {
                    hits.push((j, found));
                }
            }
            let speed = self.vel[i].length();
            for &(j, found) in &hits {
                if self.bodies[j].asleep && speed > WAKE_SPEED && found.1 < margin {
                    self.wake(j);
                }
                let (x, y) = (self.entities[i].id as u64, self.entities[j].id as u64);
                let key = x.min(y) << 31 | x.max(y);
                let (pair, normal) = if x < y {
                    ((i, j), found.0)
                } else {
                    ((j, i), -found.0)
                };
                let grip = (self.bodies[i].friction + self.bodies[j].friction) / 2.0;
                self.add_contact(pair, key, (normal, found.1), grip, cache);
            }
        }
    }

    fn warm_start(&mut self) {
        for c in &self.contacts {
            let (a, b) = (c.a as usize, c.b as usize);
            let impulse = c.normal * c.normal_impulse
                + c.tangents[0] * c.tangent_impulse[0]
                + c.tangents[1] * c.tangent_impulse[1];
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

            let limit = c.friction * c.normal_impulse;
            for (tangent, applied) in c.tangents.iter().zip(&mut c.tangent_impulse) {
                let slide = (va - vb).dot(*tangent);
                let total = (*applied - c.mass * slide).clamp(-limit, limit);
                let delta = *tangent * (total - *applied);
                *applied = total;
                va += delta * inv_a;
                vb -= delta * inv_b;
            }
            self.vel[a] = va;
            self.vel[b] = vb;
        }
    }

    fn keep_out_of_walls(&mut self, anchor: usize) {
        for c in self.contacts.iter().filter(|c| c.b as usize == anchor) {
            let a = c.a as usize;
            let depth = -(c.separation + self.moved[a].dot(c.normal));
            if depth > MAX_WALL_DEPTH {
                self.moved[a] += c.normal * (depth - MAX_WALL_DEPTH);
                let into = self.vel[a].dot(c.normal).min(0.0);
                self.vel[a] -= c.normal * into;
            }
        }
    }

    fn update_sleep(&mut self, gravity: Vec3, dt: f32) {
        for i in 0..self.count() {
            let body = &mut self.bodies[i];
            if body.asleep || !body.can_sleep {
                continue;
            }
            let held = body.on_ground || gravity == Vec3::ZERO;
            let still = held && self.vel[i].length() < SLEEP_SPEED;
            body.still = if still { body.still + dt } else { 0.0 };
            if body.still >= SLEEP_TIME {
                body.asleep = true;
                body.previous = self.pos[i];
                self.vel[i] = Vec3::ZERO;
                self.inv_mass[i] = 0.0;
            }
        }
    }

    pub(super) fn step(
        &mut self,
        walls: &Grid,
        gravity: Vec3,
        dt: f32,
        substeps: u32,
        cache: &mut ContactCache,
    ) {
        let n = self.count();
        self.find_contacts(walls, dt, cache);
        self.contacts.sort_by_key(|c| c.b as usize == n);
        let h = dt / substeps as f32;
        let soft = Soft::new(h);
        for i in 0..n {
            self.moved[i] = Vec3::ZERO;
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

        self.keep_out_of_walls(n);
        cache.0.clear();
        for i in 0..n {
            self.pos[i] += self.moved[i];
            self.bodies[i].on_ground = self.bodies[i].asleep && self.bodies[i].on_ground;
        }
        for c in &self.contacts {
            if c.normal_impulse <= 0.0 {
                continue;
            }
            let [t0, t1] = c.tangent_impulse;
            cache.0.insert(c.key, [c.normal_impulse, t0, t1]);
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
