use glam::{Mat4, Vec2, Vec3};

use crate::ecs::prelude::*;
use crate::ecs::{Baked, render_offset};
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
            view.center = if view.pixel_snap {
                (center * zoom).round() / zoom
            } else {
                center
            };
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
    for (e, g, s) in world.query::<(&GlobalTransform, &SpriteRenderer)>() {
        if !s.visible || s.color.a <= 0.0 {
            continue;
        }
        let offset = render_offset(world, e);
        let m = Mat4::from_translation(offset) * g.matrix();
        let origin = m.transform_point3(Vec3::ZERO).truncate();
        if (origin - view.center).abs().cmpgt(half + s.size).any() {
            continue;
        }
        let lo = -s.anchor * s.size;
        let hi = lo + s.size;
        let screen = vt.transform_point3(origin.extend(0.0)).truncate();
        let snap = if view.pixel_snap {
            screen.round() - screen
        } else {
            Vec2::ZERO
        };
        let corner = |x: f32, y: f32| {
            let p = vt.transform_point3(m.transform_point3(Vec3::new(x, y, 0.0)));
            [p.x + snap.x, p.y + snap.y]
        };
        let mut quad = [
            corner(lo.x, lo.y),
            corner(hi.x, lo.y),
            corner(lo.x, hi.y),
            corner(hi.x, hi.y),
        ];
        let axis_aligned = (quad[0][1] - quad[1][1]).abs() < 1e-3;
        if view.pixel_snap && axis_aligned {
            quad = quad.map(|p| [p[0].round(), p[1].round()]);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::RenderOffset;

    fn draw(view: View2D, x: f32, offset: f32) -> DrawRect {
        let mut world = World::new();
        world.insert_resource(view);
        let e = world.spawn();
        world.insert(
            e,
            GlobalTransform(Mat4::from_translation(Vec3::new(x, 0.0, 0.0))),
        );
        world.insert(e, SpriteRenderer::default());
        world.insert(e, RenderOffset(Vec3::new(offset, 0.0, 0.0)));
        gather_world(&world).remove(0)
    }

    fn view(pixel_snap: bool) -> View2D {
        View2D {
            zoom: 10.0,
            screen: Vec2::new(100.0, 100.0),
            pixel_snap,
            ..Default::default()
        }
    }

    #[test]
    fn sprites_land_on_whole_screen_pixels() {
        let d = draw(view(true), 0.123, 0.0);
        assert_eq!(d.min[0], d.min[0].round());
        assert_ne!(draw(view(false), 0.123, 0.0).min[0], d.min[0]);
    }

    #[test]
    fn scaled_sprites_keep_whole_pixel_edges() {
        let mut world = World::new();
        world.insert_resource(view(true));
        let e = world.spawn();
        let scale = Mat4::from_scale(Vec3::new(1.0, 1.073, 1.0));
        world.insert(e, GlobalTransform(scale));
        world.insert(e, SpriteRenderer::default());
        let d = gather_world(&world).remove(0);
        assert_eq!(d.max[1], d.max[1].round());
        assert_eq!(d.min[1], d.min[1].round());
    }

    #[test]
    fn render_offset_moves_only_the_drawing() {
        let moved = draw(view(true), 0.0, 1.0);
        let base = draw(view(true), 1.0, 0.0);
        assert_eq!(moved.min, base.min);
    }
}
