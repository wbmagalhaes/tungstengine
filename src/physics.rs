use glam::{EulerRot, Mat4, Quat, Vec2, Vec3};

use crate::ecs::Component;

#[derive(Component, Clone, Copy)]
pub struct Collider {
    pub pad: f32,
    pub nav: bool,
    pub size: Vec3,
    pub offset: Vec3,
    pub rotation: Vec3,
}

impl Default for Collider {
    fn default() -> Self {
        Collider {
            pad: 0.0,
            nav: true,
            size: Vec3::ONE,
            offset: Vec3::ZERO,
            rotation: Vec3::ZERO,
        }
    }
}

impl Collider {
    pub fn local_matrix(&self) -> Mat4 {
        let r = self.rotation;
        let rot = Quat::from_euler(
            EulerRot::YXZ,
            r.y.to_radians(),
            r.x.to_radians(),
            r.z.to_radians(),
        );
        Mat4::from_scale_rotation_translation(self.size, rot, self.offset)
    }

    pub fn world_matrix(&self, global: Mat4) -> Mat4 {
        global * self.local_matrix()
    }

    pub fn corners(&self, global: Mat4) -> [Vec3; 8] {
        let m = self.world_matrix(global);
        std::array::from_fn(|i| {
            let s = |bit: usize| if i & bit == 0 { -0.5 } else { 0.5 };
            m.transform_point3(Vec3::new(s(1), s(2), s(4)))
        })
    }

    pub fn footprint(&self, global: Mat4) -> Footprint {
        Footprint::hull(self.corners(global).map(|c| Vec2::new(c.x, c.z)).to_vec())
    }
}

pub struct Footprint {
    pub points: Vec<Vec2>,
}

fn cross(o: Vec2, a: Vec2, b: Vec2) -> f32 {
    (a - o).perp_dot(b - o)
}

fn seg_closest(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 < 1e-12 {
        return a;
    }
    a + ab * ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
}

fn edges(pts: &[Vec2]) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
    (0..pts.len()).map(move |i| (pts[i], pts[(i + 1) % pts.len()]))
}

fn separated(a: &[Vec2], b: &[Vec2]) -> bool {
    for (p, q) in edges(a).chain(edges(b)) {
        let axis = (q - p).perp();
        if axis.length_squared() < 1e-12 {
            continue;
        }
        let proj = |pts: &[Vec2]| {
            pts.iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), v| {
                    let d = v.dot(axis);
                    (lo.min(d), hi.max(d))
                })
        };
        let (a0, a1) = proj(a);
        let (b0, b1) = proj(b);
        if a1 < b0 || b1 < a0 {
            return true;
        }
    }
    false
}

impl Footprint {
    pub fn hull(mut pts: Vec<Vec2>) -> Self {
        pts.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        pts.dedup_by(|a, b| a.distance_squared(*b) < 1e-10);
        if pts.len() < 3 {
            return Footprint { points: pts };
        }
        let mut lower: Vec<Vec2> = Vec::new();
        for &p in &pts {
            while lower.len() >= 2
                && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0
            {
                lower.pop();
            }
            lower.push(p);
        }
        let mut upper: Vec<Vec2> = Vec::new();
        for &p in pts.iter().rev() {
            while upper.len() >= 2
                && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0
            {
                upper.pop();
            }
            upper.push(p);
        }
        lower.pop();
        upper.pop();
        lower.extend(upper);
        Footprint { points: lower }
    }

    pub fn contains(&self, p: Vec2) -> bool {
        self.points.len() >= 3 && edges(&self.points).all(|(a, b)| cross(a, b, p) >= 0.0)
    }

    pub fn closest_on_boundary(&self, p: Vec2) -> Vec2 {
        if self.points.len() == 1 {
            return self.points[0];
        }
        edges(&self.points)
            .map(|(a, b)| seg_closest(p, a, b))
            .min_by(|x, y| x.distance_squared(p).total_cmp(&y.distance_squared(p)))
            .unwrap_or(p)
    }

    pub fn distance(&self, p: Vec2) -> f32 {
        if self.points.is_empty() || self.contains(p) {
            return 0.0;
        }
        self.closest_on_boundary(p).distance(p)
    }

    pub fn distance_to(&self, other: &[Vec2]) -> f32 {
        if self.points.is_empty() || other.is_empty() {
            return f32::INFINITY;
        }
        if self.points.len() >= 3 && other.len() >= 3 && !separated(&self.points, other) {
            return 0.0;
        }
        let a_to_b = self.points.iter().map(|p| {
            edges(other)
                .map(|(a, b)| seg_closest(*p, a, b).distance(*p))
                .fold(f32::INFINITY, f32::min)
        });
        let b_to_a = other.iter().map(|p| {
            edges(&self.points)
                .map(|(a, b)| seg_closest(*p, a, b).distance(*p))
                .fold(f32::INFINITY, f32::min)
        });
        a_to_b.chain(b_to_a).fold(f32::INFINITY, f32::min)
    }

    pub fn bounds(&self) -> (Vec2, Vec2) {
        self.points.iter().fold(
            (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
            |(lo, hi), p| (lo.min(*p), hi.max(*p)),
        )
    }

    pub fn centroid(&self) -> Vec2 {
        if self.points.is_empty() {
            return Vec2::ZERO;
        }
        self.points.iter().copied().sum::<Vec2>() / self.points.len() as f32
    }
}

pub mod prelude {
    pub use super::Collider;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_box_footprint_is_square() {
        let c = Collider::default();
        let fp = c.footprint(Mat4::IDENTITY);
        assert_eq!(fp.points.len(), 4);
        assert!(fp.contains(Vec2::ZERO));
        assert!(!fp.contains(Vec2::new(0.6, 0.0)));
        assert!((fp.distance(Vec2::new(1.0, 0.0)) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn rotated_box_covers_diagonal() {
        let c = Collider {
            size: Vec3::new(4.0, 1.0, 0.2),
            rotation: Vec3::new(0.0, 45.0, 0.0),
            ..Default::default()
        };
        let fp = c.footprint(Mat4::IDENTITY);
        assert!(fp.contains(Vec2::new(1.0, -1.0)));
        assert!(!fp.contains(Vec2::new(1.0, 1.0)));
    }

    #[test]
    fn offset_and_entity_scale_compose() {
        let c = Collider {
            size: Vec3::new(1.0, 3.0, 6.8),
            offset: Vec3::new(0.0, 1.5, 0.0),
            ..Default::default()
        };
        let corners = c.corners(Mat4::from_translation(Vec3::new(3.35, 0.0, 0.0)));
        let min_y = corners.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_z = corners
            .iter()
            .map(|p| p.z)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(min_y.abs() < 1e-5);
        assert!((max_z - 3.4).abs() < 1e-5);
    }
}
