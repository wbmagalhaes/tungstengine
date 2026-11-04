use std::collections::HashMap;

use glam::{EulerRot, Quat, Vec3};

use tungstengine::color::Color;
use tungstengine::ecs::prelude::*;
use tungstengine::nav::WalkGrid;
use tungstengine::physics::Collider;
use tungstengine::raycast;
use tungstengine::render::d3::prelude::*;
use tungstengine::ui::prelude::*;
use tungstengine_adventure::Interactable;
use tungstengine_adventure::interact::REACH;

use super::camera::{self, CamInput};
use super::gizmo::{self, Op, Pointer, View};
use super::menu::{BAR_H, Cmd, TOOLBAR_H};
use super::node::{self, NodeRef};
use super::state::{EditorState, FrameInput};
use super::{grid, inspector, pick, toolbar, tree};

const TREE_W: f32 = 260.0;
const REACH_COLOR: Color = Color::AMBER_400;
const PICK_COLOR: Color = Color::CYAN_300;
const PANEL_TOP: f32 = BAR_H + TOOLBAR_H + 8.0;

fn ground(ray: &CursorRay) -> Vec3 {
    if ray.dir.y.abs() > 1e-6 {
        ray.origin + ray.dir * (-ray.origin.y / ray.dir.y)
    } else {
        Vec3::ZERO
    }
}

pub fn euler_quat(rot_deg: Vec3) -> Quat {
    Quat::from_euler(
        EulerRot::YXZ,
        rot_deg.y.to_radians(),
        rot_deg.x.to_radians(),
        rot_deg.z.to_radians(),
    )
}

pub fn quat_euler(q: Quat) -> Vec3 {
    let (y, x, z) = q.to_euler(EulerRot::YXZ);
    Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees())
}

pub fn apply_transform(world: &mut World, e: Entity, pos: Vec3, rot_deg: Vec3, scale: Vec3) {
    if let Some(t) = world.get_mut::<Transform>(e) {
        t.position = pos;
        t.rotation = euler_quat(rot_deg);
        t.scale = scale;
    }
}

pub fn spawn_marker(world: &mut World, grid: &WalkGrid, cx: usize, cz: usize) -> Entity {
    let c = grid.center(cx, cz);
    let e = world.spawn();
    let mut t = Transform::from_pos(Vec3::new(c.x, 0.03, c.z));
    t.scale = Vec3::new(grid.cell * 0.92, 0.04, grid.cell * 0.92);
    world.insert(e, t);
    world.insert(
        e,
        Renderer::new(Mesh::CUBE, Material::UNLIT.color(Color::ORANGE_700)),
    );
    e
}

fn paint_cell(
    world: &mut World,
    grid: &WalkGrid,
    markers: &HashMap<(usize, usize), Entity>,
    cx: usize,
    cz: usize,
    blocked: bool,
) {
    let key = (cx, cz);
    if blocked == markers.contains_key(&key) {
        return;
    }
    if blocked {
        let e = spawn_marker(world, grid, cx, cz);
        if let Ok(s) = world.resource_mut::<EditorState>() {
            s.grid.set(cx, cz, true);
            s.markers.insert(key, e);
        }
    } else if let Some(&e) = markers.get(&key) {
        world.despawn(e);
        if let Ok(s) = world.resource_mut::<EditorState>() {
            s.grid.set(cx, cz, false);
            s.markers.remove(&key);
        }
    }
}

fn sync_selection_outline(world: &mut World, selected: Option<Entity>) {
    let prev: Vec<Entity> = world.query::<(&Hovered,)>().map(|(e, _)| e).collect();
    for e in prev {
        if Some(e) != selected {
            world.remove::<Hovered>(e);
        }
    }
    if let Some(e) = selected
        && world.is_alive(e)
        && !world.has::<Hovered>(e)
    {
        world.insert(e, Hovered);
        world.insert(
            e,
            Outline {
                color: Color::ORANGE_500,
                width: 3.0,
            },
        );
    }
}

fn draw_colliders(world: &World, ui: &mut UI, selected: Option<Entity>) {
    let Some(view) = View::of(world, ui.width, ui.height) else {
        return;
    };
    let boxes: Vec<(Entity, [Vec3; 8])> = world
        .query::<(&GlobalTransform, &Collider)>()
        .map(|(e, g, c)| (e, c.corners(g.0)))
        .collect();
    for (e, corners) in boxes {
        let (color, thick) = if Some(e) == selected {
            (Color::GREEN_400, 2.0)
        } else {
            (Color::GREEN_500.with_alpha(0.35), 1.0)
        };
        gizmo::wire_corners(ui, &view, &corners, color, thick);
    }
}

fn draw_interactable(world: &World, ui: &mut UI, selected: Option<Entity>) {
    let Some(e) = selected else {
        return;
    };
    let Some(view) = View::of(world, ui.width, ui.height) else {
        return;
    };
    let mut stack = vec![e];
    while let Some(n) = stack.pop() {
        if let Some(c) = world.get::<Children>(n) {
            stack.extend(c.0.iter().copied());
        }
        let (Some(i), Some(g)) = (
            world.get::<Interactable>(n),
            world.get::<GlobalTransform>(n),
        ) else {
            continue;
        };
        let pos = g.position();
        let ground = Vec3::new(pos.x, 0.02, pos.z);
        gizmo::world_circle(
            ui,
            &view,
            ground,
            Vec3::Y,
            i.radius + REACH,
            REACH_COLOR,
            1.5,
        );
        let has_mesh = raycast::subtree(world, n)
            .iter()
            .any(|x| world.has::<Renderer>(*x));
        if !has_mesh || i.pick_explicit {
            for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                gizmo::world_circle(ui, &view, pos, axis, i.pick, PICK_COLOR, 1.0);
            }
        }
    }
}

fn paint(world: &mut World, inp: &FrameInput, over_ui: bool) {
    let Ok(s) = world.resource::<EditorState>() else {
        return;
    };
    let (grid, markers, value, painting, last_cell) = (
        s.grid.clone(),
        s.markers.clone(),
        s.paint_value,
        s.painting,
        s.last_cell,
    );
    if !over_ui && (inp.pressed || (inp.held && painting)) {
        let ray = world.resource::<CursorRay>().copied().unwrap_or_default();
        let g = ground(&ray);
        if let Some(cell) = grid.cell_of(g.x, g.z) {
            if inp.pressed {
                let value = !grid.is_blocked_cell(cell.0, cell.1);
                paint_cell(world, &grid, &markers, cell.0, cell.1, value);
                if let Ok(s) = world.resource_mut::<EditorState>() {
                    s.paint_value = value;
                    s.painting = true;
                    s.last_cell = Some(cell);
                }
            } else if Some(cell) != last_cell {
                paint_cell(world, &grid, &markers, cell.0, cell.1, value);
                if let Ok(s) = world.resource_mut::<EditorState>() {
                    s.last_cell = Some(cell);
                }
            }
        }
    }
    if inp.released
        && let Ok(s) = world.resource_mut::<EditorState>()
    {
        s.painting = false;
    }
}

fn selected_node(world: &World) -> Option<NodeRef> {
    world
        .resource::<EditorState>()
        .ok()
        .and_then(node::selected)
}

pub fn constrain(world: &mut World, axis: usize) {
    let ray = world.resource::<CursorRay>().copied().unwrap_or_default();
    let cursor = world
        .resource::<tungstengine::input::Input>()
        .map(|i| i.cursor)
        .unwrap_or_default();
    let xf = selected_node(world).and_then(|n| node::world_xf(world, &n));
    let Ok(s) = world.resource_mut::<EditorState>() else {
        return;
    };
    if let Some(op) = s.modal_op.as_mut() {
        op.toggle_axis(axis);
        return;
    }
    let Some(xf) = xf.filter(|_| !s.paint_mode) else {
        return;
    };
    let axes = std::array::from_fn(|k| k == axis);
    s.modal_op = Some(Op::new(s.tool, axes, xf, ray, cursor));
    s.gizmo = None;
}

fn modal(world: &mut World, ui: &mut UI, inp: &FrameInput, op: Op) {
    let Some(n) = selected_node(world) else {
        if let Ok(s) = world.resource_mut::<EditorState>() {
            s.modal_op = None;
        }
        return;
    };
    let Some(view) = View::of(world, ui.width, ui.height) else {
        return;
    };
    let ray = world.resource::<CursorRay>().copied().unwrap_or_default();
    if inp.escape || inp.rmb_pressed {
        node::set_world(world, &n, op.start, None);
        if let Ok(s) = world.resource_mut::<EditorState>() {
            s.modal_op = None;
        }
        return;
    }
    if let Some(xf) = gizmo::apply(&op, &view, &ray, inp.cursor) {
        node::set_world(world, &n, xf, Some(op.tool));
    }
    gizmo::draw_constraint(ui, &view, &op);
    gizmo::modal_line(ui, &view, &op, inp.cursor);
    let axes: String = op
        .axes
        .iter()
        .zip(["X", "Y", "Z"])
        .filter(|(on, _)| **on)
        .map(|(_, n)| n)
        .collect();
    let msg = format!(
        "{} {} | LMB/Enter: confirm | Esc/RMB: cancel",
        op.tool.label(),
        if axes.is_empty() {
            "livre".into()
        } else {
            axes
        }
    );
    ui.add(
        Text::new(16.0, BAR_H + TOOLBAR_H + 10.0, &msg)
            .font_size(14.0)
            .fg(Color::GRAY_100),
    );
    if (inp.pressed || inp.enter)
        && let Ok(s) = world.resource_mut::<EditorState>()
    {
        s.modal_op = None;
    }
}

fn edit_objects(world: &mut World, ui: &mut UI, inp: &FrameInput, over_ui: bool) {
    let Ok(s) = world.resource::<EditorState>() else {
        return;
    };
    if let Some(op) = s.modal_op {
        return modal(world, ui, inp, op);
    }
    let (tool, mut drag) = (s.tool, s.gizmo);
    let placed = s.placed.clone();
    let ray = world.resource::<CursorRay>().copied().unwrap_or_default();

    let mut captured = false;
    let transform_gizmo = world
        .resource::<EditorState>()
        .is_ok_and(|s| !s.hidden_gizmos.contains("transform"));
    if transform_gizmo
        && let Some(n) = selected_node(world)
        && let Some(xf) = node::world_xf(world, &n)
        && let Some(view) = View::of(world, ui.width, ui.height)
    {
        let pointer = Pointer {
            ray,
            cursor: inp.cursor,
            pressed: inp.pressed && !over_ui,
            held: inp.held,
        };
        let out = gizmo::frame(ui, &view, tool, &mut drag, xf, &pointer);
        captured = out.captured;
        if let Some(xf) = out.xf {
            node::set_world(world, &n, xf, Some(tool));
        }
    }
    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.gizmo = drag;
    }

    if inp.pressed && !over_ui && !captured {
        let hit = pick::pick(world, &placed, &ray).map(|(root, e)| NodeRef {
            root,
            path: node::path_of(world, root, e),
        });
        if let Ok(s) = world.resource_mut::<EditorState>() {
            node::select(s, hit);
        }
    }
}

pub fn frame(world: &mut World, ui: &mut UI, inp: &FrameInput) -> Option<Cmd> {
    let side_h = ui.height - PANEL_TOP - 56.0;
    let tree_h = (side_h * 0.45).max(120.0);
    let tree_panel = Rect::new(ui.width - TREE_W - 12.0, PANEL_TOP, TREE_W, tree_h);
    let inspector_panel = Rect::new(
        tree_panel.x,
        PANEL_TOP + tree_h + 6.0,
        TREE_W,
        side_h - tree_h - 6.0,
    );

    let over_ui = inp.modal
        || inp.menu_covers
        || inp.cursor.1 < BAR_H + TOOLBAR_H
        || tree_panel.contains(inp.cursor)
        || inspector_panel.contains(inp.cursor);
    let modal_active = world
        .resource::<EditorState>()
        .is_ok_and(|s| s.modal_op.is_some());
    let cam_busy = camera::frame(
        world,
        CamInput {
            over_ui,
            enabled: !inp.modal && !modal_active,
        },
    );
    let over_ui = over_ui || cam_busy;
    grid::frame(world);
    let paint_mode = world
        .resource::<EditorState>()
        .map(|s| s.paint_mode)
        .unwrap_or(false);

    let selected_entity =
        |world: &World| selected_node(world).and_then(|n| node::entity(world, &n));
    let before = selected_entity(world);
    let hidden = world
        .resource::<EditorState>()
        .map(|s| s.hidden_gizmos.clone())
        .unwrap_or_default();
    if !hidden.contains("collider") {
        draw_colliders(world, ui, before);
    }
    if !hidden.contains("interactable") {
        draw_interactable(world, ui, before.filter(|_| !paint_mode));
    }
    if paint_mode {
        paint(world, inp, over_ui);
    } else {
        edit_objects(world, ui, inp, over_ui);
    }

    let mut cmd = toolbar::frame(world, ui);
    cmd = tree::frame(world, ui, tree_panel, inp.scroll).or(cmd);
    inspector::frame(world, ui, inspector_panel, inp.scroll, inp.enter);

    let after = selected_entity(world);
    if after != before
        && let Ok(s) = world.resource_mut::<EditorState>()
    {
        s.inspector_scroll = 0.0;
    }
    let selected = after.filter(|_| !paint_mode && !hidden.contains("renderer"));
    sync_selection_outline(world, selected);

    let status = world
        .resource::<EditorState>()
        .map(|s| s.status.clone())
        .unwrap_or_default();
    ui.add(Text::new(16.0, ui.height - 46.0, &status).font_size(15.0));
    let hint = if paint_mode {
        "Paint: click/drag on the floor toggles blocked | Save writes the grid"
    } else {
        "click: select | W/E/R: move/scale/rotate | X/Y/Z: lock axis | Del: delete | Ctrl+S: save"
    };
    ui.add(
        Text::new(16.0, ui.height - 26.0, hint)
            .font_size(14.0)
            .fg(Color::GRAY_400),
    );
    cmd
}
