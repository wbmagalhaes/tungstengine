use glam::{Mat4, Vec2, Vec3};

use crate::ecs::Baked;
use crate::ecs::prelude::*;
use crate::ui::DrawRect;

use super::components::{Camera2D, MainCam, SpriteRenderer, View2D};

pub fn propagate_transforms(world: &mut World) {
    let roots: Vec<(Entity, Mat4)> = world
        .query::<(&Transform,)>()
        .filter(|(e, _)| {
            !world.has::<Parent>(*e) && !(world.has::<Static>(*e) && world.has::<Baked>(*e))
        })
        .map(|(e, t)| (e, t.matrix()))
        .collect();
    let mut stack = Vec::new();
    for (e, m) in roots {
        if let Some(g) = world.get_mut::<GlobalTransform>(e) {
            g.0 = m;
        } else {
            world.insert(e, GlobalTransform(m));
        }
        if world.has::<Static>(e) {
            world.insert(e, Baked);
        }
        if let Some(children) = world.get::<Children>(e) {
            stack.extend(children.0.iter().map(|&c| (c, m)));
        }
        while let Some((c, parent)) = stack.pop() {
            let Some(local) = world.get::<Transform>(c).map(|t| t.matrix()) else {
                continue;
            };
            let m = parent * local;
            world.insert(c, GlobalTransform(m));
            if let Some(children) = world.get::<Children>(c) {
                stack.extend(children.0.iter().map(|&k| (k, m)));
            }
        }
    }
}

pub fn update_view(world: &mut World, screen: Vec2) {
    let cam = world
        .query::<(&GlobalTransform, &Camera2D, &MainCam)>()
        .next()
        .map(|(_, g, c, _)| (g.position().truncate(), c.zoom));
    if let Ok(view) = world.resource_mut::<View2D>() {
        if let Some((center, zoom)) = cam {
            view.center = center;
            view.zoom = zoom;
        }
        view.screen = screen;
    }
}

pub fn gather_world(world: &World) -> Vec<DrawRect> {
    let view = world.resource::<View2D>().copied().unwrap_or_default();
    let vt = view.transform();
    let clip = [0.0, 0.0, view.screen.x, view.screen.y];
    let half = view.visible_half() + Vec2::splat(4.0);

    let mut items: Vec<((i32, f32), DrawRect)> = Vec::new();
    for (_, g, s) in world.query::<(&GlobalTransform, &SpriteRenderer)>() {
        if !s.visible || s.color.a <= 0.0 {
            continue;
        }
        let m = g.matrix();
        let origin = m.transform_point3(Vec3::ZERO).truncate();
        if (origin - view.center).abs().cmpgt(half + s.size).any() {
            continue;
        }
        let lo = -s.anchor * s.size;
        let hi = lo + s.size;
        let corner = |x: f32, y: f32| {
            let p = vt.transform_point3(m.transform_point3(Vec3::new(x, y, 0.0)));
            [p.x, p.y]
        };
        let quad = [
            corner(lo.x, lo.y),
            corner(hi.x, lo.y),
            corner(lo.x, hi.y),
            corner(hi.x, hi.y),
        ];
        let xs = quad.map(|p| p[0]);
        let ys = quad.map(|p| p[1]);
        let min = [
            xs.into_iter().fold(f32::INFINITY, f32::min),
            ys.into_iter().fold(f32::INFINITY, f32::min),
        ];
        let max = [
            xs.into_iter().fold(f32::NEG_INFINITY, f32::max),
            ys.into_iter().fold(f32::NEG_INFINITY, f32::max),
        ];
        let (mut uv_min, mut uv_max) = (s.sprite.uv_min, s.sprite.uv_max);
        if s.flip_x {
            std::mem::swap(&mut uv_min[0], &mut uv_max[0]);
        }
        items.push((
            (s.layer, origin.y + s.sort_offset),
            DrawRect {
                min,
                max,
                uv_min,
                uv_max,
                color: s.color.to_array(),
                z: 0.0,
                tex: s.sprite.tex,
                clip,
                quad: Some(quad),
                uvs: None,
            },
        ));
    }
    items.sort_by(|a, b| {
        a.0.0.cmp(&b.0.0).then(
            a.0.1
                .partial_cmp(&b.0.1)
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });
    items.into_iter().map(|(_, d)| d).collect()
}
