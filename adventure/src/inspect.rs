use glam::{Mat4, Vec3};

use winit::keyboard::KeyCode;

use tungstengine::animation::Animator;
use tungstengine::color::Color;
use tungstengine::dialog::Dialog;
use tungstengine::ecs::prelude::*;
use tungstengine::input::Input;
use tungstengine::render::d3::prelude::*;
use tungstengine::render::d3::{CameraView, Lighting, RenderItem, Scene};
use tungstengine::ui::UI;
use tungstengine::ui::prelude::*;

use super::components::{Examinable, Flags, Inspect, Interactable};

const FOV: f32 = 0.7;
const NEAR: f32 = 0.05;
const FAR: f32 = 100.0;
const ROT_SPEED: f32 = 0.004;
const DRAG_THRESHOLD: f32 = 6.0;
const ZOOM_MIN: f32 = 0.4;
const ZOOM_MAX: f32 = 3.0;

pub fn active(world: &World) -> bool {
    world
        .resource::<Inspect>()
        .map(|i| i.active())
        .unwrap_or(false)
}

pub fn intro(world: &World) -> f32 {
    world.resource::<Inspect>().map(|i| i.fade()).unwrap_or(0.0)
}

fn entity_scale(world: &World, e: Entity) -> Vec3 {
    world
        .get::<GlobalTransform>(e)
        .map(|g| g.0.to_scale_rotation_translation().0)
        .unwrap_or(Vec3::ONE)
}

fn render_parts(world: &World, root: Entity) -> Vec<(Entity, Mat4)> {
    let inv = world
        .get::<GlobalTransform>(root)
        .map(|g| g.0.inverse())
        .unwrap_or(Mat4::IDENTITY);
    let mut parts = Vec::new();
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if let Some(c) = world.get::<Children>(n) {
            stack.extend(c.0.iter().copied());
        }
        if world.has::<Renderer>(n) {
            let g = world
                .get::<GlobalTransform>(n)
                .map(|g| g.0)
                .unwrap_or(Mat4::IDENTITY);
            parts.push((n, inv * g));
        }
    }
    parts
}

fn item_bounds(world: &World, parts: &[(Entity, Mat4)], gscale: Vec3) -> (Vec3, f32) {
    let fallback = (Vec3::ZERO, 0.5);
    let Ok(reg) = world.resource::<MeshRegistry>() else {
        return fallback;
    };
    let data = reg.data();
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for (n, rel) in parts {
        let Some(renderer) = world.get::<Renderer>(*n) else {
            continue;
        };
        for s in &renderer.slots {
            if let Some((verts, _)) = data.get(s.mesh.index()) {
                for v in verts {
                    let p = rel.transform_point3(Vec3::from_array(v.position));
                    min = min.min(p);
                    max = max.max(p);
                }
            }
        }
    }
    if !min.is_finite() || !max.is_finite() {
        return fallback;
    }
    let center = (min + max) * 0.5;
    let radius = ((max - min) * 0.5 * gscale.abs()).length();
    (center, radius.max(1e-3))
}

fn examine_view(world: &World, e: Entity, ex: &Examinable, ins: &Inspect) -> (Mat4, Mat4) {
    let gscale = entity_scale(world, e);
    let parts = render_parts(world, e);
    let (center, radius) = item_bounds(world, &parts, gscale);
    let model = Mat4::from_rotation_y(ins.yaw)
        * Mat4::from_rotation_x(ins.pitch)
        * Mat4::from_scale(gscale * ex.scale)
        * Mat4::from_translation(-(center + ex.center));
    let pop = 1.0 + (1.0 - ins.fade()) * 0.06;
    let fit = radius * ex.scale / (FOV * 0.5).sin() * 1.3 * pop;
    let cam = Mat4::from_translation(Vec3::new(0.0, 0.0, fit / ins.zoom.max(0.01)));
    (model, cam)
}

fn studio_lighting() -> Lighting {
    Lighting {
        sun_dir: Vec3::new(0.4, 0.8, 1.0),
        sun_color: Color::rgb(255, 250, 240),
        sun_intensity: 1.1,
        ambient: Color::rgb(46, 50, 62),
        sky: Color::rgb(30, 33, 42),
        background: Color::rgb(0, 0, 0),
        ..Lighting::default()
    }
}

pub fn scene(world: &World) -> Option<Scene> {
    let ins = world.resource::<Inspect>().ok()?;
    let e = ins.entity?;
    let ex = world.get::<Examinable>(e)?;

    let (model, cam) = examine_view(world, e, ex, ins);
    let parts = render_parts(world, e);
    if parts.is_empty() {
        return None;
    }

    let mut items = Vec::new();
    for (n, rel) in parts {
        let Some(renderer) = world.get::<Renderer>(n) else {
            continue;
        };
        let animator = world.get::<Animator>(n);
        for s in &renderer.slots {
            let m = match &animator {
                Some(a) => model * rel * a.delta(s.bone),
                None => model * rel,
            };
            items.push(RenderItem {
                mesh: s.mesh,
                model: m,
                color: s.material.color,
                shader: s.material.shader,
                faces: s.material.faces,
                metallic: s.material.metallic,
                roughness: s.material.roughness,
                uv_offset: s.material.uv_offset,
                uv_scale: s.material.uv_scale,
                params: s.material.params,
                cast_shadows: false,
                outline_occlude: true,
            });
        }
    }

    let time = world.resource::<Time>().map(|t| t.elapsed).unwrap_or(0.0);
    Some(Scene {
        camera: CameraView {
            world: cam,
            fov: FOV,
            near: NEAR,
            far: FAR,
        },
        items,
        outlines: Vec::new(),
        outline_color: [0.0; 4],
        outline_width: 0.0,
        lighting: studio_lighting(),
        time,
        post: Vec::new(),
    })
}

fn pick_hotspot(world: &World, cursor: (f32, f32), w: f32, h: f32) -> Option<usize> {
    let ins = world.resource::<Inspect>().ok()?;
    let e = ins.entity?;
    let ex = world.get::<Examinable>(e)?;
    if ex.hotspots.is_empty() {
        return None;
    }

    let (model, cam) = examine_view(world, e, ex, ins);
    let aspect = w / h.max(1.0);
    let inv = (Mat4::perspective_rh(FOV, aspect, NEAR, FAR) * cam.inverse()).inverse();
    let ndc_x = cursor.0 / w * 2.0 - 1.0;
    let ndc_y = 1.0 - cursor.1 / h * 2.0;
    let origin = inv.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
    let far_p = inv.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));
    let dir = (far_p - origin).normalize_or_zero();

    let rscale = ex.scale * entity_scale(world, e).max_element();
    let mut best: Option<(usize, f32)> = None;
    for (i, hs) in ex.hotspots.iter().enumerate() {
        let center = model.transform_point3(hs.center);
        let radius = hs.radius * rscale;
        let oc = origin - center;
        let b = oc.dot(dir);
        let c = oc.dot(oc) - radius * radius;
        let disc = b * b - c;
        if disc < 0.0 {
            continue;
        }
        let t = -b - disc.sqrt();
        if t >= 0.0 && best.is_none_or(|(_, bd)| t < bd) {
            best = Some((i, t));
        }
    }
    best.map(|(i, _)| i)
}

pub struct InspectInput {
    pub cursor: (f32, f32),
    pub click: bool,
    pub held: bool,
    pub scroll: f32,
    pub esc: bool,
    pub blocked: bool,
}

pub fn update(world: &mut World, ui: &mut UI, input: &InspectInput) {
    let InspectInput {
        cursor,
        click,
        held,
        scroll,
        esc,
        blocked,
    } = *input;

    if esc {
        if let Ok(i) = world.resource_mut::<Input>() {
            i.consume_key(KeyCode::Escape);
        }
        let dialog_active = world
            .resource::<Dialog>()
            .map(|d| d.active)
            .unwrap_or(false);
        if dialog_active {
            if let Ok(d) = world.resource_mut::<Dialog>() {
                d.active = false;
            }
            return;
        }
        if let Ok(ins) = world.resource_mut::<Inspect>() {
            ins.begin_close();
        }
    }

    let dt = world.resource::<Time>().map(|t| t.dt).unwrap_or(0.0);
    let (closing, finished) = if let Ok(ins) = world.resource_mut::<Inspect>() {
        if let Some(t) = ins.intro.as_mut() {
            t.tick(dt);
        }
        let fin = ins.intro.as_ref().map(|t| t.finished()).unwrap_or(true);
        (ins.closing, fin)
    } else {
        (false, true)
    };

    if closing {
        if finished {
            let temp = world
                .resource::<Inspect>()
                .ok()
                .and_then(|i| i.temp.then_some(i.entity).flatten());
            if let Ok(ins) = world.resource_mut::<Inspect>() {
                ins.close();
            }
            if let Some(e) = temp {
                world.despawn(e);
            }
        } else {
            draw_overlay(world, ui, cursor, false);
        }
        return;
    }

    if blocked {
        if let Ok(ins) = world.resource_mut::<Inspect>() {
            ins.hover = None;
            ins.last_cursor = None;
        }
        draw_overlay(world, ui, cursor, false);
        return;
    }

    let (w, h) = (ui.width, ui.height);

    if let Ok(ins) = world.resource_mut::<Inspect>() {
        if held {
            if let Some((px, py)) = ins.last_cursor {
                let (dx, dy) = (cursor.0 - px, cursor.1 - py);
                ins.yaw += dx * ROT_SPEED;
                ins.pitch = (ins.pitch + dy * ROT_SPEED).clamp(-1.4, 1.4);
            } else {
                ins.press = Some(cursor);
            }
            ins.last_cursor = Some(cursor);
        } else {
            ins.last_cursor = None;
        }
        if scroll != 0.0 {
            ins.zoom = (ins.zoom + scroll * 0.1).clamp(ZOOM_MIN, ZOOM_MAX);
        }
    }

    let hover = pick_hotspot(world, cursor, w, h);
    if let Ok(ins) = world.resource_mut::<Inspect>() {
        ins.hover = hover;
    }

    draw_overlay(world, ui, cursor, true);

    let dragged = world
        .resource::<Inspect>()
        .ok()
        .and_then(|i| i.press)
        .map(|(px, py)| (cursor.0 - px).hypot(cursor.1 - py) > DRAG_THRESHOLD)
        .unwrap_or(false);
    if click && let Ok(ins) = world.resource_mut::<Inspect>() {
        ins.press = None;
    }

    if click && !dragged && close_rect(ui).contains(cursor) {
        if let Ok(ins) = world.resource_mut::<Inspect>() {
            ins.begin_close();
        }
        return;
    }

    if click
        && !dragged
        && let Some(idx) = hover
    {
        let (lines, flag) = world
            .resource::<Inspect>()
            .ok()
            .and_then(|i| i.entity)
            .and_then(|e| world.get::<Examinable>(e))
            .and_then(|ex| ex.hotspots.get(idx))
            .map(|hs| (hs.look.clone(), hs.flag.clone()))
            .unwrap_or_default();
        if let Some(flag) = flag
            && let Ok(f) = world.resource_mut::<Flags>()
        {
            f.set(&flag);
        }
        if !lines.is_empty()
            && let Ok(d) = world.resource_mut::<Dialog>()
        {
            d.say(lines);
        }
    }
}

fn close_rect(ui: &UI) -> Rect {
    Rect::new(ui.width - 48.0, 16.0, 32.0, 32.0)
}

fn draw_overlay(world: &World, ui: &mut UI, cursor: (f32, f32), controls: bool) {
    let Some((e, fade)) = world
        .resource::<Inspect>()
        .ok()
        .and_then(|i| i.entity.map(|e| (e, i.fade())))
    else {
        return;
    };

    ui.block_pointer(Rect::new(0.0, 0.0, ui.width, ui.height));

    let name = world
        .get::<Interactable>(e)
        .map(|i| i.name.clone())
        .unwrap_or_default();
    let label = super::interact::display_label(world, e, &name);
    if !label.is_empty() {
        ui.add(
            Text::new(24.0, 24.0, &label)
                .font_size(22.0)
                .fg(Color::TEXT_TITLE.with_alpha(fade)),
        );
    }

    if !controls {
        return;
    }

    let cb = close_rect(ui);
    ui.fill(cb, Color::GRAY_900.with_alpha(0.85 * fade));
    let xw = ui.text_width(20.0, "X");
    ui.add(
        Text::new(cb.x + (cb.w - xw) * 0.5, cb.y + 6.0, "X")
            .font_size(20.0)
            .fg(Color::TEXT_TITLE.with_alpha(fade)),
    );

    ui.add(
        Text::new(24.0, ui.height - 30.0, &crate::tr(world, "inspect.hint"))
            .fg(Color::TEXT_MUTED.with_alpha(fade)),
    );

    let hover = world.resource::<Inspect>().ok().and_then(|i| i.hover);
    if let Some(idx) = hover
        && let Some(hs) = world
            .get::<Examinable>(e)
            .and_then(|ex| ex.hotspots.get(idx).map(|h| h.name.clone()))
    {
        let tw = ui.text_width(16.0, &hs) + 12.0;
        let r = Rect::new(cursor.0 + 16.0, cursor.1 + 16.0, tw, 22.0);
        ui.fill(r, Color::GRAY_900.with_alpha(0.92));
        ui.add(Text::new(r.x + 6.0, r.y + 4.0, &hs).font_size(16.0));
    }
}
