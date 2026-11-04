use tungstengine::ecs::prelude::*;
use tungstengine::physics::Collider;
use tungstengine::raycast::{closest, collider_hit, mesh_hit, ray_sphere, subtree};
use tungstengine::render::d3::prelude::*;

use super::state::Placed;

const EMPTY_RADIUS: f32 = 0.35;

fn object_hit(
    world: &World,
    meshes: &MeshRegistry,
    p: &Placed,
    ray: &CursorRay,
) -> Option<(f32, Entity)> {
    let mut best: Option<(f32, Entity)> = None;
    let mut solid = false;
    for e in subtree(world, p.entity) {
        solid |= world.has::<Renderer>(e) || world.has::<Collider>(e);
        let d = closest(mesh_hit(world, meshes, e, ray), collider_hit(world, e, ray));
        if let Some(d) = d
            && best.is_none_or(|(bd, _)| d < bd)
        {
            best = Some((d, e));
        }
    }
    if solid {
        best
    } else {
        ray_sphere(ray, p.pos, EMPTY_RADIUS).map(|d| (d, p.entity))
    }
}

pub fn pick(world: &World, placed: &[Placed], ray: &CursorRay) -> Option<(usize, Entity)> {
    let meshes = world.resource::<MeshRegistry>().ok()?;
    placed
        .iter()
        .enumerate()
        .filter_map(|(i, p)| object_hit(world, meshes, p, ray).map(|(d, e)| (i, d, e)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _, e)| (i, e))
}
