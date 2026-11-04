use std::f32::consts::TAU;

use glam::{Mat4, Vec3};

use tungstengine::animation::Animator;
use tungstengine::color::Color;
use tungstengine::ecs::prelude::*;
use tungstengine::input::Input;
use tungstengine::physics::Collider;
use tungstengine::render::d3::Transition;
use tungstengine::render::d3::prelude::*;
use tungstengine::scene::SceneObject;
use tungstengine::ui::prelude::*;

use super::systems::PLAYER_RADIUS;
use crate::{Debug, Destination, DisplayName, Ident, Interactable, Item, Player};
use tungstengine::nav::WalkGrid;

struct Painter<'a> {
    ui: &'a mut UI,
    vp: Mat4,
    w: f32,
    h: f32,
}

impl Painter<'_> {
    fn project(&self, p: Vec3) -> Option<(f32, f32)> {
        let clip = self.vp * p.extend(1.0);
        if clip.w <= 1e-5 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        if ndc.z < 0.0 || ndc.z > 1.0 {
            return None;
        }
        Some((
            (ndc.x * 0.5 + 0.5) * self.w,
            (1.0 - (ndc.y * 0.5 + 0.5)) * self.h,
        ))
    }

    fn dot(&mut self, p: Vec3, size: f32, color: Color) {
        if let Some((sx, sy)) = self.project(p) {
            self.ui.fill(
                Rect::new(sx - size, sy - size, size * 2.0, size * 2.0),
                color,
            );
        }
    }

    fn seg(&mut self, a: Vec3, b: Vec3, step: f32, color: Color) {
        let n = ((b - a).length() / step).ceil().max(1.0) as i32;
        for k in 0..=n {
            self.dot(a.lerp(b, k as f32 / n as f32), 1.0, color);
        }
    }

    fn rect(&mut self, center: Vec3, hx: f32, hz: f32, color: Color) {
        let c = [
            center + Vec3::new(-hx, 0.0, -hz),
            center + Vec3::new(hx, 0.0, -hz),
            center + Vec3::new(hx, 0.0, hz),
            center + Vec3::new(-hx, 0.0, hz),
        ];
        for i in 0..4 {
            self.seg(c[i], c[(i + 1) % 4], 0.12, color);
        }
    }

    fn circle(&mut self, center: Vec3, r: f32, color: Color) {
        let n = 40;
        for k in 0..n {
            let a = k as f32 / n as f32 * TAU;
            self.dot(
                center + Vec3::new(r * a.cos(), 0.0, r * a.sin()),
                1.0,
                color,
            );
        }
    }

    fn label(&mut self, p: Vec3, text: &str, color: Color) {
        if let Some((sx, sy)) = self.project(p) {
            self.ui.add(
                Text::new(sx + 4.0, sy - 6.0, text)
                    .font_size(11.0)
                    .fg(color),
            );
        }
    }
}

fn project(vp: Mat4, w: f32, h: f32, p: Vec3) -> Option<(f32, f32)> {
    let clip = vp * p.extend(1.0);
    if clip.w <= 1e-5 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    Some(((ndc.x * 0.5 + 0.5) * w, (1.0 - (ndc.y * 0.5 + 0.5)) * h))
}

fn describe(world: &World, e: Entity) -> Vec<String> {
    let mut lines = vec![format!("{e:?}")];
    if let Some(i) = world.get::<Ident>(e) {
        lines.push(format!("ident: {}", i.key));
    }
    if let Some(t) = world.get::<Transform>(e) {
        let p = t.position;
        let s = t.scale;
        lines.push(format!("pos: {:.2} {:.2} {:.2}", p.x, p.y, p.z));
        lines.push(format!("scale: {:.2} {:.2} {:.2}", s.x, s.y, s.z));
    }
    if let Some(i) = world.get::<Interactable>(e) {
        lines.push(format!("name: {}", i.name));
        lines.push(format!("verbs: {}  pick: {:.2}", i.verbs.len(), i.pick));
    }
    if let Some(d) = world.get::<DisplayName>(e) {
        match d {
            DisplayName::Fixed(l) => lines.push(format!("display: {l}")),
            DisplayName::Hidden { name, known, .. } => lines.push(format!(
                "display: {name} ({})",
                if *known { "known" } else { "hidden" }
            )),
            DisplayName::Actor(id) => lines.push(format!("actor: {id}")),
        }
    }
    if let Some(r) = world.get::<Renderer>(e) {
        lines.push(format!(
            "slots: {}  shadows: {}",
            r.slots.len(),
            r.cast_shadows
        ));
    }
    if let Some(a) = world.get::<Animator>(e) {
        lines.push(format!("clip: {}", a.current().unwrap_or("-")));
    }
    if let Some(c) = world.get::<Collider>(e) {
        lines.push(format!(
            "collider pad{:.2} nav:{} size{:.1},{:.1},{:.1}",
            c.pad, c.nav, c.size.x, c.size.y, c.size.z
        ));
    }
    if world.has::<Player>(e) {
        lines.push("player".into());
    }
    if world.has::<Item>(e) {
        lines.push("item".into());
    }
    lines
}

fn inspect_target(
    world: &World,
    vp: Mat4,
    w: f32,
    h: f32,
    cursor: (f32, f32),
) -> Option<(Vec3, Vec<String>)> {
    let mut best: Option<(Entity, Vec3, f32)> = None;
    for (e, g, _) in world.query::<(&GlobalTransform, &SceneObject)>() {
        let pos = g.position();
        let Some((sx, sy)) = project(vp, w, h, pos) else {
            continue;
        };
        let d = ((sx - cursor.0).powi(2) + (sy - cursor.1).powi(2)).sqrt();
        if d < 60.0 && best.as_ref().is_none_or(|(_, _, bd)| d < *bd) {
            best = Some((e, pos, d));
        }
    }
    let (e, pos, _) = best?;
    Some((pos, describe(world, e)))
}

pub fn debug_draw(world: &mut World, ui: &mut UI) {
    if !world
        .resource::<Debug>()
        .map(|d| d.enabled)
        .unwrap_or(false)
    {
        return;
    }

    let dt = world.resource::<Time>().map(|t| t.dt).unwrap_or(0.0);
    let fps = world
        .resource_mut::<Debug>()
        .map(|d| {
            let inst = if dt > 1e-6 { 1.0 / dt } else { 0.0 };
            d.fps = if d.fps > 0.0 {
                d.fps * 0.9 + inst * 0.1
            } else {
                inst
            };
            d.fps
        })
        .unwrap_or(0.0);

    let (w, h) = (ui.width, ui.height);
    let Some((cam_world, fov, near, far)) = world
        .query::<(&GlobalTransform, &Camera, &MainCam)>()
        .next()
        .map(|(_, g, c, _)| (g.matrix(), c.fov, c.near, c.far))
    else {
        return;
    };

    let vp = Mat4::perspective_rh(fov, w / h.max(1.0), near, far) * cam_world.inverse();

    let grid = world.query::<(&WalkGrid,)>().next().map(|(_, g)| g.clone());
    let colliders: Vec<(Vec<Vec3>, Vec3, f32, bool)> = world
        .query::<(&GlobalTransform, &Collider)>()
        .map(|(_, g, c)| {
            let fp = c.footprint(g.0);
            let (scale, _, _) = c.world_matrix(g.0).to_scale_rotation_translation();
            (
                fp.points.iter().map(|p| Vec3::new(p.x, 0.0, p.y)).collect(),
                scale,
                c.pad,
                c.nav,
            )
        })
        .collect();
    let player = world
        .query::<(&Transform, &Player)>()
        .next()
        .map(|(_, t, _)| t.position);
    let path: Vec<Vec3> = world
        .resource::<Destination>()
        .map(|d| d.path.clone())
        .unwrap_or_default();

    let cursor = world
        .resource::<Input>()
        .map(|i| i.cursor)
        .unwrap_or((0.0, 0.0));
    let inspect = inspect_target(world, vp, w, h, cursor);

    let mut p = Painter { ui, vp, w, h };

    if let Some(grid) = grid {
        let (ox, oz) = (grid.origin[0], grid.origin[1]);
        let wx = grid.cols as f32 * grid.cell;
        let wz = grid.rows as f32 * grid.cell;
        let line = Color::SLATE_400.with_alpha(0.18);
        for i in 0..=grid.cols {
            let x = ox + i as f32 * grid.cell;
            p.seg(
                Vec3::new(x, 0.0, oz),
                Vec3::new(x, 0.0, oz + wz),
                grid.cell,
                line,
            );
        }
        for j in 0..=grid.rows {
            let z = oz + j as f32 * grid.cell;
            p.seg(
                Vec3::new(ox, 0.0, z),
                Vec3::new(ox + wx, 0.0, z),
                grid.cell,
                line,
            );
        }
        let half = grid.cell * 0.46;
        for cz in 0..grid.rows {
            for cx in 0..grid.cols {
                if grid.is_blocked_cell(cx, cz) {
                    p.rect(
                        grid.center(cx, cz),
                        half,
                        half,
                        Color::RED_500.with_alpha(0.7),
                    );
                }
            }
        }
    }

    for (poly, size, pad, nav) in colliders {
        let color = if nav {
            Color::AMBER_400
        } else {
            Color::CYAN_400
        };
        for i in 0..poly.len() {
            p.seg(poly[i], poly[(i + 1) % poly.len()], 0.05, color);
        }
        let center = poly.iter().copied().sum::<Vec3>() / poly.len().max(1) as f32;
        p.label(
            center,
            &format!("{:.1}x{:.1} pad{:.2}", size.x, size.z, pad),
            color,
        );
    }

    if let Some(pp) = player {
        p.circle(pp, PLAYER_RADIUS, Color::EMERALD_400);
        let mut prev = pp;
        for wp in &path {
            p.seg(prev, *wp, 0.2, Color::TEAL_300.with_alpha(0.7));
            prev = *wp;
        }
    }
    for wp in &path {
        p.dot(*wp, 4.0, Color::TEAL_300.with_alpha(0.9));
    }

    if let Some((pos, lines)) = inspect {
        p.circle(pos, 0.35, Color::AMBER_400);
        let lh = 16.0;
        let pw = 200.0;
        let ph = lines.len() as f32 * lh + 12.0;
        let panel = Rect::new(12.0, h - ph - 12.0, pw, ph);
        p.ui.fill(panel, Color::GRAY_900.with_alpha(0.9));
        for (i, line) in lines.iter().enumerate() {
            let fg = if i == 0 {
                Color::AMBER_300
            } else {
                Color::TEXT_MUTED
            };
            p.ui.add(
                Text::new(panel.x + 8.0, panel.y + 6.0 + i as f32 * lh, line)
                    .font_size(13.0)
                    .fg(fg),
            );
        }
    }

    let stats = world.resource::<RenderStats>().copied().unwrap_or_default();
    let (ram, cpu) = sys_stats();
    let lines = [
        format!("{fps:.0} FPS"),
        format!("{} draws", stats.draw_calls),
        format!("{} inst", stats.instances),
        format!("{} tris", stats.triangles),
        format!("VRAM ~{}", mib(stats.vram_bytes)),
        format!("RAM {}", mib(ram)),
        format!("CPU {cpu:.0}%"),
    ];
    p.ui.add(Text::new(w - 132.0, 16.0, "DEBUG").fg(Color::RED_400));
    for (i, line) in lines.iter().enumerate() {
        p.ui.add(Text::new(w - 132.0, 34.0 + i as f32 * 16.0, line).fg(Color::TEXT_MUTED));
    }

    let btn = Rect::new(
        w - 132.0,
        34.0 + lines.len() as f32 * 16.0 + 8.0,
        120.0,
        26.0,
    );
    let triggered = p.ui.button(btn, "TRANSITION").clicked();
    if triggered
        && let Ok(t) = world.resource_mut::<Transition>()
        && !t.active
    {
        t.start();
    }
}

fn mib(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

struct SysMon {
    sys: sysinfo::System,
    pid: sysinfo::Pid,
    last: std::time::Instant,
    ram: u64,
    cpu: f32,
    primed: bool,
}

static SYSMON: std::sync::LazyLock<std::sync::Mutex<Option<SysMon>>> =
    std::sync::LazyLock::new(|| {
        std::sync::Mutex::new(sysinfo::get_current_pid().ok().map(|pid| SysMon {
            sys: sysinfo::System::new(),
            pid,
            last: std::time::Instant::now(),
            ram: 0,
            cpu: 0.0,
            primed: false,
        }))
    });

fn sys_stats() -> (u64, f32) {
    let Ok(mut guard) = SYSMON.lock() else {
        return (0, 0.0);
    };
    let Some(m) = guard.as_mut() else {
        return (0, 0.0);
    };
    if !m.primed || m.last.elapsed().as_millis() >= 400 {
        let pid = m.pid;
        m.sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[pid]),
            true,
            sysinfo::ProcessRefreshKind::nothing()
                .with_memory()
                .with_cpu(),
        );
        if let Some(p) = m.sys.process(pid) {
            m.ram = p.memory();
            m.cpu = p.cpu_usage();
        }
        m.last = std::time::Instant::now();
        m.primed = true;
    }
    (m.ram, m.cpu)
}
