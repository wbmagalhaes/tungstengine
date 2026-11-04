use glam::{Mat4, Vec3};

use crate::ecs::prelude::*;
use crate::physics::Collider;
use crate::render::d3::{CursorRay, MeshRegistry, Renderer};

pub fn ray_triangle(ray: &CursorRay, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let e1 = b - a;
    let e2 = c - a;
    let p = ray.dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-8 {
        return None;
    }
    let inv = 1.0 / det;
    let s = ray.origin - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = ray.dir.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 1e-4).then_some(t)
}

pub fn ray_aabb(ray: &CursorRay, min: Vec3, max: Vec3) -> Option<f32> {
    let inv = ray.dir.recip();
    let t1 = (min - ray.origin) * inv;
    let t2 = (max - ray.origin) * inv;
    let near = t1.min(t2).max_element();
    let far = t1.max(t2).min_element();
    (far >= near.max(0.0)).then_some(near.max(0.0))
}

pub fn ray_sphere(ray: &CursorRay, center: Vec3, radius: f32) -> Option<f32> {
    let oc = ray.origin - center;
    let b = oc.dot(ray.dir);
    let disc = b * b - (oc.dot(oc) - radius * radius);
    if disc < 0.0 {
        return None;
    }
    let s = disc.sqrt();
    let t = -b - s;
    if t >= 0.0 {
        Some(t)
    } else {
        let t2 = -b + s;
        (t2 >= 0.0).then_some(t2)
    }
}

pub fn closest(a: Option<f32>, b: Option<f32>) -> Option<f32> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (x, y) => x.or(y),
    }
}

pub fn mesh_hit(world: &World, meshes: &MeshRegistry, e: Entity, ray: &CursorRay) -> Option<f32> {
    let renderer = world.get::<Renderer>(e)?;
    let model = world
        .get::<GlobalTransform>(e)
        .map(|g| g.0)
        .unwrap_or(Mat4::IDENTITY);
    let data = meshes.data();
    let mut best = None;
    for slot in &renderer.slots {
        let Some((verts, idx)) = data.get(slot.mesh.index()) else {
            continue;
        };
        let world_verts: Vec<Vec3> = verts
            .iter()
            .map(|v| model.transform_point3(Vec3::from_array(v.position)))
            .collect();
        for tri in idx.as_chunks::<3>().0 {
            let [a, b, c] = tri.map(|i| world_verts[i as usize]);
            best = closest(best, ray_triangle(ray, a, b, c));
        }
    }
    best
}

pub fn collider_hit(world: &World, e: Entity, ray: &CursorRay) -> Option<f32> {
    let c = world.get::<Collider>(e)?;
    let inv = c.world_matrix(world.get::<GlobalTransform>(e)?.0).inverse();
    let local = CursorRay {
        origin: inv.transform_point3(ray.origin),
        dir: inv.transform_vector3(ray.dir),
    };
    ray_aabb(&local, Vec3::splat(-0.5), Vec3::splat(0.5))
}

pub fn subtree(world: &World, root: Entity) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        if let Some(c) = world.get::<Children>(e) {
            stack.extend(c.0.iter().copied());
        }
        out.push(e);
    }
    out
}

pub fn subtree_mesh_hit(world: &World, root: Entity, ray: &CursorRay) -> (bool, Option<f32>) {
    let Ok(meshes) = world.resource::<MeshRegistry>() else {
        return (false, None);
    };
    let mut has_mesh = false;
    let mut best = None;
    for e in subtree(world, root) {
        if world.has::<Renderer>(e) {
            has_mesh = true;
            best = closest(best, mesh_hit(world, meshes, e, ray));
        }
    }
    (has_mesh, best)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ray(o: Vec3, d: Vec3) -> CursorRay {
        CursorRay {
            origin: o,
            dir: d.normalize(),
        }
    }

    #[test]
    fn triangle_hit_and_miss() {
        let r = ray(Vec3::new(0.2, 0.2, 5.0), Vec3::NEG_Z);
        let (a, b, c) = (Vec3::ZERO, Vec3::X, Vec3::Y);
        assert!((ray_triangle(&r, a, b, c).unwrap() - 5.0).abs() < 1e-4);
        let miss = ray(Vec3::new(2.0, 2.0, 5.0), Vec3::NEG_Z);
        assert!(ray_triangle(&miss, a, b, c).is_none());
    }

    #[test]
    fn aabb_entry_distance() {
        let r = ray(Vec3::new(0.0, 0.0, 5.0), Vec3::NEG_Z);
        let d = ray_aabb(&r, Vec3::splat(-0.5), Vec3::splat(0.5)).unwrap();
        assert!((d - 4.5).abs() < 1e-4);
    }

    #[test]
    fn sphere_behind_ray_misses() {
        let r = ray(Vec3::new(0.0, 0.0, 5.0), Vec3::Z);
        assert!(ray_sphere(&r, Vec3::ZERO, 1.0).is_none());
    }

    #[test]
    fn closest_prefers_nearer() {
        assert_eq!(closest(Some(2.0), Some(1.0)), Some(1.0));
        assert_eq!(closest(None, Some(3.0)), Some(3.0));
    }
}
