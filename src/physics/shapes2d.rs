use glam::Vec2;

use super::d2::{Placed, Shape2D};

pub const MAX_POLYGON_POINTS: usize = 8;

#[derive(Clone, Copy)]
pub(super) struct Hull {
    points: [Vec2; MAX_POLYGON_POINTS],
    count: usize,
}

impl Hull {
    fn of((shape, pos, angle): Placed) -> Option<Hull> {
        let turn = Vec2::from_angle(angle);
        let mut hull = Hull {
            points: [Vec2::ZERO; MAX_POLYGON_POINTS],
            count: 0,
        };
        let mut push = |p: Vec2| {
            hull.points[hull.count] = pos + turn.rotate(p);
            hull.count += 1;
        };
        match shape {
            Shape2D::Circle(_) => return None,
            Shape2D::Box(size) => {
                let h = size / 2.0;
                push(Vec2::new(-h.x, -h.y));
                push(Vec2::new(h.x, -h.y));
                push(Vec2::new(h.x, h.y));
                push(Vec2::new(-h.x, h.y));
            }
            Shape2D::Polygon { points, count } => {
                for &p in &points[..count as usize] {
                    push(p);
                }
            }
        }
        Some(hull)
    }

    fn points(&self) -> &[Vec2] {
        &self.points[..self.count]
    }

    fn edge(&self, i: usize) -> (Vec2, Vec2) {
        (self.points[i], self.points[(i + 1) % self.count])
    }

    fn normal(&self, i: usize) -> Vec2 {
        let (a, b) = self.edge(i);
        let edge = (b - a).normalize_or_zero();
        let outward = Vec2::new(edge.y, -edge.x);
        let center = self.points().iter().sum::<Vec2>() / self.count as f32;
        if outward.dot(a - center) < 0.0 {
            -outward
        } else {
            outward
        }
    }

    fn widest_gap(&self, other: &Hull) -> (f32, usize) {
        let mut best = (f32::MIN, 0);
        for i in 0..self.count {
            let normal = self.normal(i);
            let origin = self.points[i];
            let nearest = other
                .points()
                .iter()
                .map(|p| (*p - origin).dot(normal))
                .fold(f32::MAX, f32::min);
            if nearest > best.0 {
                best = (nearest, i);
            }
        }
        best
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Manifold {
    pub normal: Vec2,
    pub points: [(Vec2, f32); 2],
    pub count: usize,
}

impl Manifold {
    pub fn single(normal: Vec2, point: Vec2, separation: f32) -> Self {
        Manifold {
            normal,
            points: [(point, separation), (Vec2::ZERO, 0.0)],
            count: 1,
        }
    }

    pub fn contacts(&self) -> &[(Vec2, f32)] {
        &self.points[..self.count]
    }

    pub fn separation(&self) -> f32 {
        self.contacts().iter().map(|c| c.1).fold(f32::MAX, f32::min)
    }
}

fn circles(pa: Vec2, ra: f32, pb: Vec2, rb: f32) -> Manifold {
    let d = pa - pb;
    let length = d.length();
    let normal = if length > 1e-6 { d / length } else { Vec2::Y };
    Manifold::single(normal, pb + normal * rb, length - ra - rb)
}

fn circle_hull(center: Vec2, radius: f32, hull: &Hull) -> Manifold {
    let mut face = (f32::MIN, 0);
    for i in 0..hull.count {
        let distance = (center - hull.points[i]).dot(hull.normal(i));
        if distance > face.0 {
            face = (distance, i);
        }
    }
    let (a, b) = hull.edge(face.1);
    if face.0 <= 0.0 {
        let normal = hull.normal(face.1);
        return Manifold::single(normal, center - normal * face.0, face.0 - radius);
    }
    let along = b - a;
    let t = ((center - a).dot(along) / along.length_squared().max(1e-9)).clamp(0.0, 1.0);
    let closest = a + along * t;
    let d = center - closest;
    let length = d.length();
    let normal = if length > 1e-6 {
        d / length
    } else {
        hull.normal(face.1)
    };
    Manifold::single(normal, closest, length - radius)
}

fn hulls(a: &Hull, b: &Hull) -> Manifold {
    let (gap_a, edge_a) = a.widest_gap(b);
    let (gap_b, edge_b) = b.widest_gap(a);
    let a_leads = gap_a >= gap_b - 1e-4;
    let (reference, incident, edge) = if a_leads {
        (a, b, edge_a)
    } else {
        (b, a, edge_b)
    };
    let normal = reference.normal(edge);
    let facing = (0..incident.count)
        .min_by(|&x, &y| {
            let (p, q) = (
                incident.normal(x).dot(normal),
                incident.normal(y).dot(normal),
            );
            p.total_cmp(&q)
        })
        .unwrap_or(0);
    let (mut start, mut end) = incident.edge(facing);
    let (r0, r1) = reference.edge(edge);
    let side = (r1 - r0).normalize_or_zero();
    for (origin, axis) in [(r0, side), (r1, -side)] {
        let (d0, d1) = ((start - origin).dot(axis), (end - origin).dot(axis));
        if d0 < 0.0 && d1 > 0.0 {
            start += (end - start) * (d0 / (d0 - d1));
        } else if d1 < 0.0 && d0 > 0.0 {
            end += (start - end) * (d1 / (d1 - d0));
        }
    }
    let toward_a = if a_leads { -normal } else { normal };
    Manifold {
        normal: toward_a,
        points: [
            (start, (start - r0).dot(normal)),
            (end, (end - r0).dot(normal)),
        ],
        count: 2,
    }
}

pub(super) fn manifold(a: Placed, b: Placed) -> Manifold {
    match (Hull::of(a), Hull::of(b)) {
        (Some(ha), Some(hb)) => hulls(&ha, &hb),
        (None, Some(hb)) => {
            let Shape2D::Circle(r) = a.0 else {
                return Manifold::default();
            };
            circle_hull(a.1, r, &hb)
        }
        (Some(ha), None) => {
            let Shape2D::Circle(r) = b.0 else {
                return Manifold::default();
            };
            let mut flipped = circle_hull(b.1, r, &ha);
            flipped.normal = -flipped.normal;
            flipped
        }
        (None, None) => match (a.0, b.0) {
            (Shape2D::Circle(ra), Shape2D::Circle(rb)) => circles(a.1, ra, b.1, rb),
            _ => Manifold::default(),
        },
    }
}

pub(super) fn hull_bounds(placed: Placed) -> Option<(Vec2, Vec2)> {
    let hull = Hull::of(placed)?;
    let lo = hull.points().iter().copied().fold(Vec2::MAX, Vec2::min);
    let hi = hull.points().iter().copied().fold(Vec2::MIN, Vec2::max);
    Some((lo, hi))
}

pub(super) fn ray_hull(placed: Placed, origin: Vec2, dir: Vec2) -> Option<f32> {
    let hull = Hull::of(placed)?;
    let (mut enter, mut leave) = (0.0f32, f32::MAX);
    for i in 0..hull.count {
        let normal = hull.normal(i);
        let distance = (hull.points[i] - origin).dot(normal);
        let speed = dir.dot(normal);
        if speed.abs() < 1e-8 {
            if distance < 0.0 {
                return None;
            }
            continue;
        }
        let t = distance / speed;
        if speed < 0.0 {
            enter = enter.max(t);
        } else {
            leave = leave.min(t);
        }
    }
    (enter <= leave).then_some(enter)
}
