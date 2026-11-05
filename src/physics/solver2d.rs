use glam::Vec2;

use crate::ecs::prelude::*;
use crate::ecs::{Children, FastMap, Transform};

use super::d2::{Body2D, Collider2D, Placed, Shape2D, WallGrid, angle_of, bounds};
use super::shapes2d::{Manifold, manifold};

const SLOP: f32 = 0.001;
const CONTACT_HERTZ: f32 = 30.0;
const CONTACT_DAMPING: f32 = 10.0;
const MAX_PUSH_SPEED: f32 = 12.0;
const SPECULATIVE: f32 = 0.02;
const SLEEP_SPEED: f32 = 0.3;
const SLEEP_TIME: f32 = 0.4;
const WAKE_SPEED: f32 = 1.5;
const MAX_CELLS_PER_BODY: usize = 8;
const MAX_SWEEP_SAMPLES: u32 = 64;
const WALL_KEY: u64 = 1 << 63;
const MAX_WALL_DEPTH: f32 = 0.01;
const LARGE_BODY: f32 = 4.0;

#[derive(Resource, Default)]
#[resource(core)]
pub(super) struct ContactCache(FastMap<u64, [f32; 2]>);

struct Contact {
    a: u32,
    b: u32,
    key: u64,
    normal: Vec2,
    ra: Vec2,
    rb: Vec2,
    separation: f32,
    mass: f32,
    tangent_mass: f32,
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

fn gap((shape_a, pa, _): Placed, (shape_b, pb, _): Placed) -> (Vec2, f32) {
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
        _ => (Vec2::Y, 0.0),
    }
}

fn is_simple(p: Placed) -> bool {
    !matches!(p.0, Shape2D::Polygon { .. }) && p.2 == 0.0
}

struct BodyGrid {
    origin: Vec2,
    cell: f32,
    cols: usize,
    rows: usize,
    starts: Vec<u32>,
    items: Vec<u32>,
    large: Vec<u32>,
}

impl BodyGrid {
    fn new(placed: &[Placed], reach: f32) -> Self {
        let width = |p: Placed| {
            let (a, b) = bounds(p);
            (b - a).max_element()
        };
        let mean = placed.iter().map(|&p| width(p)).sum::<f32>() / placed.len().max(1) as f32;
        let limit = mean * LARGE_BODY;
        let mut lo = Vec2::splat(f32::MAX);
        let mut hi = Vec2::splat(f32::MIN);
        let mut widest = 0.01f32;
        let mut large = Vec::new();
        for (i, &p) in placed.iter().enumerate() {
            let (a, b) = bounds(p);
            if width(p) > limit {
                large.push(i as u32);
                continue;
            }
            lo = lo.min(a);
            hi = hi.max(b);
            widest = widest.max((b - a).max_element());
        }
        if lo.x > hi.x {
            lo = Vec2::ZERO;
            hi = Vec2::ONE;
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
            items: vec![0; placed.len() - large.len()],
            large,
        };
        let slots: Vec<(usize, usize)> = placed
            .iter()
            .enumerate()
            .filter(|&(_, &p)| width(p) <= limit)
            .map(|(i, p)| (i, grid.slot(p.1)))
            .collect();
        for &(_, slot) in &slots {
            grid.starts[slot + 1] += 1;
        }
        for i in 1..grid.starts.len() {
            grid.starts[i] += grid.starts[i - 1];
        }
        let mut cursor = grid.starts.clone();
        for &(i, slot) in &slots {
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
        for &i in &self.large {
            visit(i as usize);
        }
    }
}

fn inverse_inertia(body: &Body2D, collider: &Collider2D) -> f32 {
    if !body.rotates || body.asleep {
        return 0.0;
    }
    let mass = body.mass.max(1e-4);
    let inertia = match collider.shape {
        Shape2D::Circle(r) => 0.5 * mass * r * r,
        Shape2D::Box(size) => mass * size.length_squared() / 12.0,
        Shape2D::Polygon { points, count } => {
            let reach = points[..count as usize]
                .iter()
                .map(|p| p.length_squared())
                .fold(0.0, f32::max);
            0.5 * mass * reach
        }
    };
    1.0 / inertia.max(1e-5)
}

#[derive(Clone, Copy)]
struct Part {
    shape: Shape2D,
    offset: Vec2,
    angle: f32,
}

fn part_size(shape: Shape2D) -> (f32, f32) {
    match shape {
        Shape2D::Circle(r) => (std::f32::consts::PI * r * r, r),
        Shape2D::Box(size) => (size.x * size.y, size.length() / 2.0),
        Shape2D::Polygon { points, count } => {
            let points = &points[..count as usize];
            let area = (0..points.len())
                .map(|i| points[i].perp_dot(points[(i + 1) % points.len()]))
                .sum::<f32>()
                .abs()
                / 2.0;
            let reach = points.iter().map(|p| p.length()).fold(0.0, f32::max);
            (area, reach)
        }
    }
}

fn child_parts(world: &World, e: Entity) -> Vec<Part> {
    let mut parts = Vec::new();
    let mut stack: Vec<(Entity, Vec2, f32)> = world
        .get::<Children>(e)
        .map(|c| c.0.iter().map(|&k| (k, Vec2::ZERO, 0.0)).collect())
        .unwrap_or_default();
    while let Some((node, base, turn)) = stack.pop() {
        let Some(t) = world.get::<Transform>(node) else {
            continue;
        };
        let at = base + Vec2::from_angle(turn).rotate(t.position.truncate());
        let angle = turn + angle_of(t);
        if let Some(c) = world.get::<Collider2D>(node)
            && !c.sensor
        {
            parts.push(Part {
                shape: c.shape,
                offset: at + Vec2::from_angle(angle).rotate(c.offset),
                angle,
            });
        }
        if let Some(children) = world.get::<Children>(node) {
            stack.extend(children.0.iter().map(|&k| (k, at, angle)));
        }
    }
    parts
}

fn compound_inverse_inertia(body: &Body2D, parts: &[Part]) -> f32 {
    if !body.rotates {
        return 0.0;
    }
    let total: f32 = parts.iter().map(|p| part_size(p.shape).0).sum();
    let inertia: f32 = parts
        .iter()
        .map(|p| {
            let (area, reach) = part_size(p.shape);
            let share = body.mass.max(1e-4) * area / total.max(1e-6);
            share * (0.5 * reach * reach + p.offset.length_squared())
        })
        .sum();
    1.0 / inertia.max(1e-5)
}

pub(super) struct Sim {
    entities: Vec<Entity>,
    colliders: Vec<Collider2D>,
    compounds: FastMap<u32, Vec<Part>>,
    base_inertia: Vec<f32>,
    bodies: Vec<Body2D>,
    pos: Vec<Vec2>,
    vel: Vec<Vec2>,
    moved: Vec<Vec2>,
    inv_mass: Vec<f32>,
    inv_inertia: Vec<f32>,
    angle: Vec<f32>,
    spin: Vec<f32>,
    turned: Vec<f32>,
    dormant: Vec<bool>,
    contacts: Vec<Contact>,
}

impl Sim {
    pub(super) fn load(world: &World) -> Self {
        let mut sim = Sim {
            entities: Vec::new(),
            colliders: Vec::new(),
            compounds: FastMap::default(),
            base_inertia: Vec::new(),
            bodies: Vec::new(),
            pos: Vec::new(),
            vel: Vec::new(),
            moved: Vec::new(),
            inv_mass: Vec::new(),
            inv_inertia: Vec::new(),
            angle: Vec::new(),
            spin: Vec::new(),
            turned: Vec::new(),
            dormant: Vec::new(),
            contacts: Vec::new(),
        };
        for (e, body, t) in world.query::<(&Body2D, &Transform)>() {
            let own = world.get::<Collider2D>(e);
            let mut parts = child_parts(world, e);
            if !parts.is_empty()
                && let Some(c) = own
            {
                parts.push(Part {
                    shape: c.shape,
                    offset: c.offset,
                    angle: 0.0,
                });
            }
            let bound = parts
                .iter()
                .map(|p| p.offset.length() + part_size(p.shape).1)
                .fold(0.0, f32::max);
            let wide = Collider2D::circle(bound);
            let collider = match (parts.is_empty(), own) {
                (false, _) => &wide,
                (true, Some(c)) => c,
                (true, None) => continue,
            };
            let mut body = *body;
            let pos = t.position.truncate();
            if body.asleep
                && (body.velocity != Vec2::ZERO || body.spin != 0.0 || pos != body.previous)
            {
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
            let awake = Body2D {
                asleep: false,
                ..body
            };
            let inertia = if parts.is_empty() {
                inverse_inertia(&awake, collider)
            } else {
                compound_inverse_inertia(&awake, &parts)
            };
            sim.base_inertia.push(inertia);
            sim.inv_inertia
                .push(if body.asleep { 0.0 } else { inertia });
            if !parts.is_empty() {
                sim.compounds.insert(sim.entities.len() as u32 - 1, parts);
            }
            sim.angle.push(angle_of(t));
            sim.spin.push(body.spin);
            sim.turned.push(0.0);
            sim.bodies.push(body);
        }
        sim.pos.push(Vec2::ZERO);
        sim.vel.push(Vec2::ZERO);
        sim.moved.push(Vec2::ZERO);
        sim.inv_mass.push(0.0);
        sim.inv_inertia.push(0.0);
        sim.angle.push(0.0);
        sim.spin.push(0.0);
        sim.turned.push(0.0);
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
                if body.rotates {
                    t.rotation = glam::Quat::from_rotation_z(self.angle[i]);
                }
            }
            if let Some(current) = world.get_mut::<Body2D>(e) {
                *current = *body;
                current.velocity = self.vel[i];
                current.spin = self.spin[i];
            }
        }
    }

    fn count(&self) -> usize {
        self.bodies.len()
    }

    fn placed(&self, i: usize) -> Placed {
        let offset = self.colliders[i].offset;
        let offset = if offset == Vec2::ZERO {
            offset
        } else {
            Vec2::from_angle(self.angle[i]).rotate(offset)
        };
        (self.colliders[i].shape, self.pos[i] + offset, self.angle[i])
    }

    fn place_part(&self, i: usize, part: &Part) -> Placed {
        let turn = Vec2::from_angle(self.angle[i]);
        (
            part.shape,
            self.pos[i] + turn.rotate(part.offset),
            self.angle[i] + part.angle,
        )
    }

    fn shapes(&self, i: usize) -> Vec<Placed> {
        match self.compounds.get(&(i as u32)) {
            Some(parts) => parts.iter().map(|p| self.place_part(i, p)).collect(),
            None => vec![self.placed(i)],
        }
    }

    fn wake(&mut self, i: usize) {
        if self.bodies[i].asleep {
            self.bodies[i].asleep = false;
            self.bodies[i].still = 0.0;
            self.inv_mass[i] = 1.0 / self.bodies[i].mass.max(1e-4);
            self.inv_inertia[i] = self.base_inertia[i];
        }
    }

    fn collide(&self, i: usize, mine: Placed, other: Placed, other_turns: bool) -> Manifold {
        let turning = self.bodies[i].rotates || other_turns;
        if !turning && is_simple(mine) && is_simple(other) {
            let (normal, separation) = gap(mine, other);
            return Manifold::single(normal, mine.1, separation);
        }
        manifold(mine, other)
    }

    fn add_contacts(
        &mut self,
        (a, b): (usize, usize),
        key: u64,
        found: &Manifold,
        friction: f32,
        cache: &ContactCache,
    ) {
        for (index, &(point, separation)) in found.contacts().iter().enumerate() {
            let (ra, rb) = (point - self.pos[a], point - self.pos[b]);
            let (ia, ib) = (self.inv_inertia[a], self.inv_inertia[b]);
            let linear = self.inv_mass[a] + self.inv_mass[b];
            if linear <= 0.0 {
                return;
            }
            let share = |axis: Vec2| {
                let (ta, tb) = (ra.perp_dot(axis), rb.perp_dot(axis));
                1.0 / (linear + ta * ta * ia + tb * tb * ib)
            };
            let normal = found.normal;
            let key = key ^ (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
            let [normal_impulse, tangent_impulse] = cache.0.get(&key).copied().unwrap_or_default();
            let contact = Contact {
                a: a as u32,
                b: b as u32,
                key,
                normal,
                ra,
                rb,
                separation,
                mass: share(normal),
                tangent_mass: share(normal.perp()),
                friction,
                normal_impulse,
                tangent_impulse,
            };
            self.contacts.push(contact);
        }
    }

    fn find_contacts(&mut self, grids: [&WallGrid; 2], step: f32, cache: &ContactCache) {
        self.contacts.clear();
        let n = self.count();
        let anchor = n;
        let solid: Vec<usize> = (0..n).filter(|&i| !self.colliders[i].sensor).collect();
        let placed: Vec<Placed> = solid.iter().map(|&i| self.placed(i)).collect();
        let grid = BodyGrid::new(&placed, SPECULATIVE);
        let mut near = Vec::new();
        let mut wake: Vec<(usize, Manifold)> = Vec::new();
        let mut parts: Vec<(usize, usize, Manifold)> = Vec::new();
        let mut swept_pairs: FastMap<u64, ()> = FastMap::default();

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
                    for (part, shape) in self.shapes(i).into_iter().enumerate() {
                        let found = self.collide(i, shape, walls.walls[w as usize], false);
                        if found.separation() < margin {
                            let base = WALL_KEY | (g as u64) << 62 | id << 30 | w as u64;
                            let key = base ^ (part as u64 + 1) << 52;
                            let grip = self.bodies[i].friction;
                            self.add_contacts((i, anchor), key, &found, grip, cache);
                        }
                    }
                }
            }
            if self.colliders[i].sensor {
                continue;
            }
            let mine = self.placed(i);
            let mut pairs: Vec<(usize, Manifold)> = std::mem::take(&mut wake);
            let speed = self.vel[i].length();
            let (mine_lo, mine_hi) = bounds(mine);
            let reach = (mine_hi - mine_lo).max_element() / 2.0;
            let travel = self.vel[i] * step * 2.0;
            let fast = travel.length() > grid.cell * 2.0;
            let samples = if fast {
                ((travel.length() / grid.cell).ceil() as u32).min(MAX_SWEEP_SAMPLES)
            } else {
                0
            };
            let mut visit = |k: usize| {
                let j = solid[k];
                if j == i {
                    return;
                }
                let theirs = self.vel[j].length();
                let their_fast = theirs * step * 2.0 > grid.cell * 2.0;
                let owned = speed > theirs || (speed == theirs && i < j);
                if !owned && !self.bodies[j].asleep && !fast && !their_fast {
                    return;
                }
                let compound = self.compounds.contains_key(&(i as u32))
                    || self.compounds.contains_key(&(j as u32));
                if compound {
                    for (pa, shape) in self.shapes(i).into_iter().enumerate() {
                        for (pb, other) in self.shapes(j).into_iter().enumerate() {
                            let found = self.collide(i, shape, other, self.bodies[j].rotates);
                            if found.separation() < SPECULATIVE {
                                parts.push((j, pa * 64 + pb, found));
                            }
                        }
                    }
                    return;
                }
                let found = self.collide(i, mine, placed[k], self.bodies[j].rotates);
                let closing = -(self.vel[i] - self.vel[j]).dot(found.normal);
                if found.separation() < SPECULATIVE + closing.max(0.0) * step {
                    pairs.push((j, found));
                }
            };
            for sample in 0..=samples {
                let along = sample as f32 / samples.max(1) as f32;
                let ring = if sample == 0 { reach } else { 0.0 };
                grid.neighbours(mine.1 + travel * along, ring, &mut visit);
            }
            if fast {
                pairs.sort_unstable_by_key(|p| p.0);
                pairs.dedup_by_key(|p| p.0);
            }
            parts.sort_unstable_by_key(|p| (p.0, p.1));
            parts.dedup_by_key(|p| (p.0, p.1));
            for (j, part, found) in parts.drain(..) {
                let (x, y) = (self.entities[i].id as u64, self.entities[j].id as u64);
                let key = (x.min(y) << 31 | x.max(y)) ^ (part as u64 + 1) << 50;
                let grip = (self.bodies[i].friction + self.bodies[j].friction) / 2.0;
                let mut found = found;
                let pair = if x < y {
                    (i, j)
                } else {
                    found.normal = -found.normal;
                    (j, i)
                };
                self.add_contacts(pair, key, &found, grip, cache);
            }
            for (j, found) in pairs.drain(..) {
                if self.bodies[j].asleep && speed > WAKE_SPEED && found.separation() < margin {
                    self.wake(j);
                }
                let (lo_id, hi_id) = {
                    let (x, y) = (self.entities[i].id as u64, self.entities[j].id as u64);
                    (x.min(y), x.max(y))
                };
                let key = lo_id << 31 | hi_id;
                let swept = fast || self.vel[j].length() * step > grid.cell;
                if swept && swept_pairs.insert(key, ()).is_some() {
                    continue;
                }
                let sign = if self.entities[i].id as u64 == lo_id {
                    1.0
                } else {
                    -1.0
                };
                let grip = (self.bodies[i].friction + self.bodies[j].friction) / 2.0;
                let (a, b) = if sign > 0.0 { (i, j) } else { (j, i) };
                let mut found = found;
                found.normal *= sign;
                self.add_contacts((a, b), key, &found, grip, cache);
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
            self.spin[a] += c.ra.perp_dot(impulse) * self.inv_inertia[a];
            self.spin[b] -= c.rb.perp_dot(impulse) * self.inv_inertia[b];
        }
    }

    fn solve(&mut self, soft: &Soft, h: f32, use_bias: bool) {
        let inv_h = 1.0 / h;
        for c in &mut self.contacts {
            let (a, b) = (c.a as usize, c.b as usize);
            let (inv_a, inv_b) = (self.inv_mass[a], self.inv_mass[b]);
            let (ia, ib) = (self.inv_inertia[a], self.inv_inertia[b]);
            let (mut va, mut vb) = (self.vel[a], self.vel[b]);
            let (mut wa, mut wb) = (self.spin[a], self.spin[b]);
            let (pa, pb) = (c.ra.perp(), c.rb.perp());
            let shift_a = self.moved[a] + pa * self.turned[a];
            let shift_b = self.moved[b] + pb * self.turned[b];
            let separation = c.separation + (shift_a - shift_b).dot(c.normal);

            let (bias, mass_scale, impulse_scale) = if separation > 0.0 {
                (separation * inv_h, 1.0, 0.0)
            } else if use_bias {
                let push = (soft.bias_rate * (separation + SLOP)).max(-MAX_PUSH_SPEED);
                (push.min(0.0), soft.mass_scale, soft.impulse_scale)
            } else {
                (0.0, 1.0, 0.0)
            };
            let speed = (va + pa * wa - vb - pb * wb).dot(c.normal);
            let impulse = -c.mass * mass_scale * (speed + bias) - impulse_scale * c.normal_impulse;
            let total = (c.normal_impulse + impulse).max(0.0);
            let delta = c.normal * (total - c.normal_impulse);
            c.normal_impulse = total;
            va += delta * inv_a;
            vb -= delta * inv_b;
            wa += c.ra.perp_dot(delta) * ia;
            wb -= c.rb.perp_dot(delta) * ib;

            let tangent = c.normal.perp();
            let slide = (va + pa * wa - vb - pb * wb).dot(tangent);
            let limit = c.friction * c.normal_impulse;
            let total = (c.tangent_impulse - c.tangent_mass * slide).clamp(-limit, limit);
            let delta = tangent * (total - c.tangent_impulse);
            c.tangent_impulse = total;
            self.vel[a] = va + delta * inv_a;
            self.vel[b] = vb - delta * inv_b;
            self.spin[a] = wa + c.ra.perp_dot(delta) * ia;
            self.spin[b] = wb - c.rb.perp_dot(delta) * ib;
        }
    }

    fn keep_out_of_walls(&mut self, anchor: usize) {
        for c in self.contacts.iter().filter(|c| c.b as usize == anchor) {
            let a = c.a as usize;
            let shift = self.moved[a] + c.ra.perp() * self.turned[a];
            let depth = -(c.separation + shift.dot(c.normal));
            if depth > MAX_WALL_DEPTH {
                self.moved[a] += c.normal * (depth - MAX_WALL_DEPTH);
                let into = self.vel[a].dot(c.normal).min(0.0);
                self.vel[a] -= c.normal * into;
            }
        }
    }

    fn update_sleep(&mut self, gravity: Vec2, dt: f32) {
        for i in 0..self.count() {
            let body = &mut self.bodies[i];
            if body.asleep || !body.can_sleep {
                continue;
            }
            let held = body.on_ground || gravity == Vec2::ZERO;
            let still =
                held && self.vel[i].length() < SLEEP_SPEED && self.spin[i].abs() < SLEEP_SPEED;
            body.still = if still { body.still + dt } else { 0.0 };
            if body.still >= SLEEP_TIME {
                body.asleep = true;
                body.previous = self.pos[i];
                self.vel[i] = Vec2::ZERO;
                self.spin[i] = 0.0;
                self.inv_mass[i] = 0.0;
                self.inv_inertia[i] = 0.0;
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
        self.contacts.sort_by_key(|c| c.b as usize == n);
        let h = dt / substeps as f32;
        let soft = Soft::new(h);
        for i in 0..n {
            self.moved[i] = Vec2::ZERO;
            self.turned[i] = 0.0;
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
                self.turned[i] += self.spin[i] * h;
            }
            self.solve(&soft, h, false);
        }

        self.keep_out_of_walls(n);
        cache.0.clear();
        for i in 0..n {
            self.pos[i] += self.moved[i];
            self.angle[i] += self.turned[i];
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
