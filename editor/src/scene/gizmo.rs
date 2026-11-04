use glam::{Mat4, Quat, Vec3, Vec4};

use tungstengine::color::Color;
use tungstengine::ecs::prelude::*;
use tungstengine::render::d3::prelude::*;
use tungstengine::ui::prelude::*;

const HANDLE_PX: f32 = 96.0;
const PICK_PX: f32 = 8.0;
const RING_SEGMENTS: usize = 72;
const CENTER_PX: f32 = 10.0;
const VIEW_RING_PX: f32 = HANDLE_PX * 0.95;
const SCALE_RING_PX: f32 = HANDLE_PX * 1.1;
const AXES: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];
pub const AXIS_COLORS: [Color; 3] = [
    Color::rgb(255, 51, 82),
    Color::rgb(139, 220, 0),
    Color::rgb(40, 144, 255),
];
const WHITE: Color = Color::rgb(235, 235, 235);
const PLANE_SPAN: [(f32, f32); 4] = [(0.22, 0.22), (0.42, 0.22), (0.42, 0.42), (0.22, 0.42)];

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Move,
    Rotate,
    Scale,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Move => "Move",
            Tool::Rotate => "Rotate",
            Tool::Scale => "Scale",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Handle {
    Axis(usize),
    Plane(usize),
    View,
}

impl Handle {
    fn axes(self) -> [bool; 3] {
        match self {
            Handle::Axis(i) => std::array::from_fn(|k| k == i),
            Handle::Plane(i) => std::array::from_fn(|k| k != i),
            Handle::View => [false; 3],
        }
    }
}

#[derive(Clone, Copy)]
pub struct Xf {
    pub pos: Vec3,
    pub rot: Quat,
    pub scale: Vec3,
}

#[derive(Clone, Copy)]
pub struct Op {
    pub tool: Tool,
    pub axes: [bool; 3],
    pub start: Xf,
    start_ray: CursorRay,
    start_cursor: (f32, f32),
    handle: Option<Handle>,
}

impl Op {
    pub fn new(tool: Tool, axes: [bool; 3], start: Xf, ray: CursorRay, cursor: (f32, f32)) -> Self {
        Op {
            tool,
            axes,
            start,
            start_ray: ray,
            start_cursor: cursor,
            handle: None,
        }
    }

    pub fn toggle_axis(&mut self, i: usize) {
        self.axes[i] = !self.axes[i];
    }
}

pub struct View {
    view_proj: Mat4,
    cam_pos: Vec3,
    cam_fwd: Vec3,
    fov: f32,
    w: f32,
    h: f32,
}

impl View {
    pub fn of(world: &World, w: f32, h: f32) -> Option<View> {
        let (_, g, c, _) = world
            .query::<(&GlobalTransform, &Camera, &MainCam)>()
            .next()?;
        let proj = Mat4::perspective_rh(c.fov, w / h.max(1.0), c.near, c.far);
        Some(View {
            view_proj: proj * g.matrix().inverse(),
            cam_pos: g.position(),
            cam_fwd: g.rotation() * Vec3::NEG_Z,
            fov: c.fov,
            w,
            h,
        })
    }

    fn to_screen(&self, c: Vec4) -> (f32, f32) {
        let (x, y) = (c.x / c.w, c.y / c.w);
        ((x + 1.0) * 0.5 * self.w, (1.0 - y) * 0.5 * self.h)
    }

    pub fn project(&self, p: Vec3) -> Option<(f32, f32)> {
        let c = self.view_proj * p.extend(1.0);
        (c.w > 1e-4).then(|| self.to_screen(c))
    }

    pub fn project_segment(&self, a: Vec3, b: Vec3) -> Option<((f32, f32), (f32, f32))> {
        const EPS: f32 = 1e-3;
        let mut ca = self.view_proj * a.extend(1.0);
        let mut cb = self.view_proj * b.extend(1.0);
        if ca.w < EPS && cb.w < EPS {
            return None;
        }
        if ca.w < EPS {
            ca = ca.lerp(cb, (EPS - ca.w) / (cb.w - ca.w));
        } else if cb.w < EPS {
            cb = cb.lerp(ca, (EPS - cb.w) / (ca.w - cb.w));
        }
        Some((self.to_screen(ca), self.to_screen(cb)))
    }

    fn world_per_px(&self, p: Vec3) -> f32 {
        2.0 * (p - self.cam_pos).length() * (self.fov * 0.5).tan() / self.h.max(1.0)
    }
}

pub struct Pointer {
    pub ray: CursorRay,
    pub cursor: (f32, f32),
    pub pressed: bool,
    pub held: bool,
}

pub struct Outcome {
    pub captured: bool,
    pub xf: Option<Xf>,
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn seg_dist(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let len2 = abx * abx + aby * aby;
    let t = if len2 > 1e-6 {
        (((p.0 - a.0) * abx + (p.1 - a.1) * aby) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    dist(p, (a.0 + abx * t, a.1 + aby * t))
}

fn in_quad(p: (f32, f32), q: &[(f32, f32); 4]) -> bool {
    let mut sign = 0.0f32;
    for i in 0..4 {
        let (a, b) = (q[i], q[(i + 1) % 4]);
        let c = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
        if c != 0.0 {
            if sign != 0.0 && c.signum() != sign {
                return false;
            }
            sign = c.signum();
        }
    }
    true
}

fn axis_param(ray: &CursorRay, origin: Vec3, dir: Vec3) -> Option<f32> {
    let w0 = origin - ray.origin;
    let b = dir.dot(ray.dir);
    let denom = 1.0 - b * b;
    if denom < 1e-5 {
        return None;
    }
    Some((b * ray.dir.dot(w0) - dir.dot(w0)) / denom)
}

fn plane_hit(ray: &CursorRay, origin: Vec3, normal: Vec3) -> Option<Vec3> {
    let denom = normal.dot(ray.dir);
    if denom.abs() < 1e-5 {
        return None;
    }
    let s = normal.dot(origin - ray.origin) / denom;
    (s >= 0.0).then(|| ray.origin + ray.dir * s)
}

fn screen_angle(c: (f32, f32), p: (f32, f32)) -> f32 {
    (p.1 - c.1).atan2(p.0 - c.0)
}

fn dirs(tool: Tool, xf: &Xf) -> [Vec3; 3] {
    match tool {
        Tool::Scale => AXES.map(|a| xf.rot * a),
        _ => AXES,
    }
}

pub fn apply(op: &Op, view: &View, ray: &CursorRay, cursor: (f32, f32)) -> Option<Xf> {
    let s = op.start;
    let mut out = s;
    let n = op.axes.iter().filter(|a| **a).count();
    let center = view.project(s.pos)?;
    match op.tool {
        Tool::Move => {
            if n == 1 {
                let i = op.axes.iter().position(|a| *a)?;
                let t0 = axis_param(&op.start_ray, s.pos, AXES[i])?;
                let t1 = axis_param(ray, s.pos, AXES[i])?;
                out.pos = s.pos + AXES[i] * (t1 - t0);
            } else {
                let normal = match n {
                    2 => AXES[op.axes.iter().position(|a| !*a)?],
                    _ => view.cam_fwd,
                };
                let p0 = plane_hit(&op.start_ray, s.pos, normal)?;
                let p1 = plane_hit(ray, s.pos, normal)?;
                out.pos = s.pos + (p1 - p0);
            }
        }
        Tool::Scale => {
            let f = dist(cursor, center) / dist(op.start_cursor, center).max(1.0);
            for i in 0..3 {
                if n == 0 || op.axes[i] {
                    out.scale[i] = (s.scale[i] * f).max(0.001);
                }
            }
        }
        Tool::Rotate => {
            let d = screen_angle(center, cursor) - screen_angle(center, op.start_cursor);
            let axis = match n {
                1 => AXES[op.axes.iter().position(|a| *a)?],
                _ => view.cam_fwd,
            };
            let sign = if axis.dot(view.cam_fwd) >= 0.0 {
                1.0
            } else {
                -1.0
            };
            out.rot = Quat::from_axis_angle(axis, d * sign) * s.rot;
        }
    }
    Some(out)
}

fn circle(ui: &mut UI, c: (f32, f32), r: f32, color: Color, thick: f32) {
    let n = 48;
    for i in 0..n {
        let a0 = i as f32 / n as f32 * std::f32::consts::TAU;
        let a1 = (i + 1) as f32 / n as f32 * std::f32::consts::TAU;
        ui.line(
            (c.0 + a0.cos() * r, c.1 + a0.sin() * r),
            (c.0 + a1.cos() * r, c.1 + a1.sin() * r),
            color,
            thick,
        );
    }
}

struct RingSeg {
    a: (f32, f32),
    b: (f32, f32),
    front: bool,
}

fn ring(view: &View, center: Vec3, normal: Vec3, radius: f32) -> Vec<RingSeg> {
    let u = normal.any_orthonormal_vector();
    let w = normal.cross(u);
    let to_cam = (view.cam_pos - center).normalize_or_zero();
    let point = |i: usize| {
        let a = i as f32 / RING_SEGMENTS as f32 * std::f32::consts::TAU;
        center + (u * a.cos() + w * a.sin()) * radius
    };
    (0..RING_SEGMENTS)
        .filter_map(|i| {
            let (p0, p1) = (point(i), point(i + 1));
            let (a, b) = view.project_segment(p0, p1)?;
            let mid = (p0 + p1) * 0.5 - center;
            Some(RingSeg {
                a,
                b,
                front: mid.dot(to_cam) >= -radius * 0.05,
            })
        })
        .collect()
}

fn plane_quad(view: &View, xf: &Xf, d: &[Vec3; 3], i: usize, size: f32) -> Option<[(f32, f32); 4]> {
    let (j, k) = ((i + 1) % 3, (i + 2) % 3);
    let mut out = [(0.0, 0.0); 4];
    for (n, (a, b)) in PLANE_SPAN.iter().enumerate() {
        out[n] = view.project(xf.pos + (d[j] * *a + d[k] * *b) * size)?;
    }
    Some(out)
}

fn axis_visible(view: &View, d: Vec3) -> bool {
    d.dot(view.cam_fwd).abs() < 0.97
}

fn plane_visible(view: &View, normal: Vec3) -> bool {
    normal.dot(view.cam_fwd).abs() > 0.2
}

fn hover(view: &View, tool: Tool, xf: &Xf, cursor: (f32, f32)) -> Option<Handle> {
    let c = view.project(xf.pos)?;
    let size = HANDLE_PX * view.world_per_px(xf.pos);
    let d = dirs(tool, xf);
    let r = dist(cursor, c);

    if tool == Tool::Rotate {
        let mut best: Option<(Handle, f32)> = None;
        for (i, axis) in AXES.into_iter().enumerate() {
            let dd = ring(view, xf.pos, axis, size * 0.8)
                .iter()
                .filter(|s| s.front)
                .map(|s| seg_dist(cursor, s.a, s.b))
                .fold(f32::INFINITY, f32::min);
            if dd < PICK_PX && best.is_none_or(|(_, bd)| dd < bd) {
                best = Some((Handle::Axis(i), dd));
            }
        }
        if best.is_none() && (r - VIEW_RING_PX).abs() < PICK_PX {
            return Some(Handle::View);
        }
        return best.map(|(h, _)| h);
    }

    match tool {
        Tool::Move if r < CENTER_PX + 3.0 => return Some(Handle::View),
        Tool::Scale if (r - SCALE_RING_PX).abs() < PICK_PX => return Some(Handle::View),
        _ => {}
    }
    for i in 0..3 {
        if plane_visible(view, d[i])
            && let Some(q) = plane_quad(view, xf, &d, i, size)
            && in_quad(cursor, &q)
        {
            return Some(Handle::Plane(i));
        }
    }
    let mut best: Option<(Handle, f32)> = None;
    for (i, dir) in d.iter().enumerate() {
        if !axis_visible(view, *dir) {
            continue;
        }
        let Some(tip) = view.project(xf.pos + *dir * size) else {
            continue;
        };
        let dd = seg_dist(cursor, c, tip);
        if dd < PICK_PX && best.is_none_or(|(_, bd)| dd < bd) {
            best = Some((Handle::Axis(i), dd));
        }
    }
    best.map(|(h, _)| h)
}

fn tint(base: Color, hot: bool) -> Color {
    if hot {
        base.lerp(Color::WHITE, 0.55)
    } else {
        base
    }
}

fn draw(ui: &mut UI, view: &View, tool: Tool, xf: &Xf, active: Option<Handle>) {
    let Some(c) = view.project(xf.pos) else {
        return;
    };
    let size = HANDLE_PX * view.world_per_px(xf.pos);
    let d = dirs(tool, xf);
    let hot = |h: Handle| active == Some(h);

    if tool == Tool::Rotate {
        circle(
            ui,
            c,
            VIEW_RING_PX,
            tint(WHITE.with_alpha(0.6), hot(Handle::View)),
            1.5,
        );
        for (i, axis) in AXES.into_iter().enumerate() {
            let col = tint(AXIS_COLORS[i], hot(Handle::Axis(i)));
            for s in ring(view, xf.pos, axis, size * 0.8) {
                if s.front {
                    ui.line(s.a, s.b, col, 2.5);
                } else {
                    ui.line(s.a, s.b, col.with_alpha(0.25), 1.5);
                }
            }
        }
        return;
    }

    for i in 0..3 {
        if !plane_visible(view, d[i]) {
            continue;
        }
        if let Some(q) = plane_quad(view, xf, &d, i, size) {
            let col = tint(AXIS_COLORS[i], hot(Handle::Plane(i)));
            ui.quad(
                q,
                col.with_alpha(if hot(Handle::Plane(i)) { 0.8 } else { 0.45 }),
            );
        }
    }

    for (i, dir) in d.iter().enumerate() {
        if !axis_visible(view, *dir) {
            continue;
        }
        let Some(tip) = view.project(xf.pos + *dir * size) else {
            continue;
        };
        let col = tint(AXIS_COLORS[i], hot(Handle::Axis(i)));
        let (dx, dy) = (tip.0 - c.0, tip.1 - c.1);
        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (ux, uy) = (dx / len, dy / len);
        match tool {
            Tool::Move => {
                let base = (tip.0 - ux * 16.0, tip.1 - uy * 16.0);
                ui.line((c.0 + ux * CENTER_PX, c.1 + uy * CENTER_PX), base, col, 2.0);
                ui.triangle(
                    (tip.0 + ux * 2.0, tip.1 + uy * 2.0),
                    (base.0 - uy * 6.0, base.1 + ux * 6.0),
                    (base.0 + uy * 6.0, base.1 - ux * 6.0),
                    col,
                );
            }
            _ => {
                ui.line(c, tip, col, 2.0);
                ui.fill(Rect::new(tip.0 - 5.0, tip.1 - 5.0, 10.0, 10.0), col);
            }
        }
    }

    match tool {
        Tool::Move => circle(ui, c, CENTER_PX, tint(WHITE, hot(Handle::View)), 1.5),
        _ => circle(
            ui,
            c,
            SCALE_RING_PX,
            tint(WHITE.with_alpha(0.6), hot(Handle::View)),
            1.5,
        ),
    }
}

pub fn draw_constraint(ui: &mut UI, view: &View, op: &Op) {
    let d = dirs(op.tool, &op.start);
    for (i, on) in op.axes.iter().enumerate() {
        if *on
            && let Some((a, b)) =
                view.project_segment(op.start.pos - d[i] * 500.0, op.start.pos + d[i] * 500.0)
        {
            ui.line(a, b, AXIS_COLORS[i].with_alpha(0.8), 1.5);
        }
    }
}

pub fn modal_line(ui: &mut UI, view: &View, op: &Op, cursor: (f32, f32)) {
    if op.tool == Tool::Move {
        return;
    }
    if let Some(c) = view.project(op.start.pos) {
        let n = (dist(c, cursor) / 6.0) as usize;
        for k in (0..n).step_by(2) {
            let t0 = k as f32 / n as f32;
            let t1 = (k + 1) as f32 / n as f32;
            let p = |t: f32| (c.0 + (cursor.0 - c.0) * t, c.1 + (cursor.1 - c.1) * t);
            ui.line(p(t0), p(t1), WHITE.with_alpha(0.7), 1.0);
        }
    }
}

pub fn frame(
    ui: &mut UI,
    view: &View,
    tool: Tool,
    drag: &mut Option<Op>,
    xf: Xf,
    input: &Pointer,
) -> Outcome {
    if let Some(op) = drag.as_ref() {
        if !input.held {
            *drag = None;
        } else {
            let next = apply(op, view, &input.ray, input.cursor);
            draw(ui, view, tool, &next.unwrap_or(xf), op.handle);
            return Outcome {
                captured: true,
                xf: next,
            };
        }
    }

    let hovered = hover(view, tool, &xf, input.cursor);
    draw(ui, view, tool, &xf, hovered);
    let started = match hovered {
        Some(h) if input.pressed => {
            let mut op = Op::new(tool, h.axes(), xf, input.ray, input.cursor);
            op.handle = Some(h);
            Some(op)
        }
        _ => None,
    };
    let captured = started.is_some();
    *drag = started;
    Outcome { captured, xf: None }
}

pub fn world_circle(
    ui: &mut UI,
    view: &View,
    center: Vec3,
    normal: Vec3,
    radius: f32,
    color: Color,
    thick: f32,
) {
    let u = normal.any_orthonormal_vector();
    let w = normal.cross(u);
    let point = |i: usize| {
        let a = i as f32 / RING_SEGMENTS as f32 * std::f32::consts::TAU;
        center + (u * a.cos() + w * a.sin()) * radius
    };
    for i in 0..RING_SEGMENTS {
        if let Some((a, b)) = view.project_segment(point(i), point(i + 1)) {
            ui.line(a, b, color, thick);
        }
    }
}

pub fn wire_corners(ui: &mut UI, view: &View, corners: &[Vec3; 8], color: Color, thick: f32) {
    for a in 0..8 {
        for bit in [1, 2, 4] {
            let b = a | bit;
            if b != a
                && let Some((pa, pb)) = view.project_segment(corners[a], corners[b])
            {
                ui.line(pa, pb, color, thick);
            }
        }
    }
}
