use std::collections::HashSet;
use std::path::Path;

use uuid::Uuid;

use tungstengine::color::Color;
use tungstengine::ecs::prelude::*;
use tungstengine::scene::GameObjDef;
use tungstengine::ui::prelude::*;

use super::menu::{Cmd, flat_button};
use super::node::{self, NodeRef};
use super::state::{DocKind, EditorState, Placed};

const ROW_H: f32 = 24.0;
const INDENT: f32 = 14.0;
const HEADER_H: f32 = 32.0;
pub const PREFAB_COLOR: Color = Color::SKY_300;

pub fn placed_label(p: &Placed, gameobjs: &[(Uuid, String)]) -> String {
    p.def
        .name
        .clone()
        .filter(|n| !n.is_empty())
        .or_else(|| {
            p.def
                .from
                .and_then(|id| gameobjs.iter().find(|(g, _)| *g == id))
                .map(|(_, n)| n.clone())
        })
        .or_else(|| {
            p.mesh.as_ref().map(|m| {
                Path::new(m)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| m.clone())
            })
        })
        .unwrap_or_else(|| "Empty".into())
}

pub fn def_label(d: &GameObjDef) -> String {
    d.name
        .clone()
        .filter(|n| !n.is_empty())
        .or_else(|| {
            d.mesh().map(|m| {
                Path::new(&m)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or(m)
            })
        })
        .unwrap_or_else(|| "Node".into())
}

struct Row {
    node: NodeRef,
    depth: usize,
    label: String,
    has_children: bool,
    editable: bool,
    prefab: bool,
    inherited: Option<(NodeRef, usize, bool)>,
}

fn push_def(
    world: &World,
    out: &mut Vec<Row>,
    n: NodeRef,
    d: &GameObjDef,
    label: String,
    expanded: &HashSet<Vec<usize>>,
) {
    let base = node::base(world, &n);
    let inherited = base
        .as_ref()
        .map(|b| b.children.clone())
        .unwrap_or_default();
    let has_own = d.children.iter().any(|c| c.prefab_child.is_none());
    let depth = n.path.len();
    out.push(Row {
        node: n.clone(),
        depth,
        label,
        has_children: !inherited.is_empty() || has_own,
        editable: true,
        prefab: base.is_some(),
        inherited: None,
    });
    if !expanded.contains(&n.key()) {
        return;
    }
    for (i, c) in inherited.iter().enumerate() {
        let removed = d.removed_children.contains(&i);
        let ov = d.children.iter().position(|o| o.prefab_child == Some(i));
        match ov.filter(|_| !removed) {
            Some(j) => {
                let mut path = n.path.clone();
                path.push(j);
                let child = NodeRef { root: n.root, path };
                let name = d.children[j].name.clone().unwrap_or_else(|| def_label(c));
                let at = out.len();
                push_def(world, out, child, &d.children[j], name, expanded);
                out[at].inherited = Some((n.clone(), i, false));
            }
            None => out.push(Row {
                node: n.clone(),
                depth: depth + 1,
                label: def_label(c),
                has_children: false,
                editable: false,
                prefab: true,
                inherited: Some((n.clone(), i, removed)),
            }),
        }
    }
    for (j, c) in d.children.iter().enumerate() {
        if c.prefab_child.is_some() {
            continue;
        }
        let mut path = n.path.clone();
        path.push(j);
        let child = NodeRef { root: n.root, path };
        push_def(world, out, child, c, def_label(c), expanded);
    }
}

fn rows(world: &World, s: &EditorState) -> Vec<Row> {
    let mut out = Vec::new();
    for (root, p) in s.placed.iter().enumerate() {
        let label = placed_label(p, &s.gameobjs);
        push_def(
            world,
            &mut out,
            NodeRef::root(root),
            &p.def,
            label,
            &s.expanded,
        );
    }
    out
}

fn caret(ui: &mut UI, cx: f32, cy: f32, open: bool) {
    let color = Color::GRAY_400;
    if open {
        ui.triangle(
            (cx - 4.0, cy - 2.0),
            (cx + 4.0, cy - 2.0),
            (cx, cy + 3.0),
            color,
        );
    } else {
        ui.triangle(
            (cx - 2.0, cy - 4.0),
            (cx + 3.0, cy),
            (cx - 2.0, cy + 4.0),
            color,
        );
    }
}

pub fn frame(world: &mut World, ui: &mut UI, panel: Rect, scroll: f32) -> Option<Cmd> {
    let Ok(s) = world.resource::<EditorState>() else {
        return None;
    };
    let rows = rows(world, s);
    let expanded = s.expanded.clone();
    let selected = node::selected(s);
    let can_add = s.doc == DocKind::Scene;
    let mut offset = s.tree_scroll;

    ui.fill(panel, Color::SLATE_900.with_alpha(0.94));
    ui.block_pointer(panel);
    ui.add(
        Text::new(panel.x + 10.0, panel.y + 8.0, "Hierarchy")
            .font_size(15.0)
            .fg(Color::GRAY_200),
    );
    let mut cmd = None;
    if can_add
        && flat_button(
            ui,
            Rect::new(panel.x + panel.w - 32.0, panel.y + 4.0, 26.0, 24.0),
            "tree-add",
            "+",
            false,
        )
    {
        cmd = Some(Cmd::AddObject);
    }

    let list = Rect::new(panel.x, panel.y + HEADER_H, panel.w, panel.h - HEADER_H);
    let max_offset = (rows.len() as f32 * ROW_H - list.h).max(0.0);
    if list.contains(ui.cursor) {
        offset -= scroll * ROW_H;
    }
    offset = offset.clamp(0.0, max_offset);

    let mut select = None;
    let mut toggle = None;
    let mut toggle_child = None;
    let mut create = None;
    ui.push_clip(list);
    for (i, row) in rows.iter().enumerate() {
        let y = list.y + i as f32 * ROW_H - offset;
        if y + ROW_H < list.y || y > list.y + list.h {
            continue;
        }
        let r = Rect::new(list.x, y, list.w, ROW_H);
        let x = list.x + 8.0 + row.depth as f32 * INDENT;
        let hit = ui.interact_id(r, ("tree", i, row.node.key()));
        let is_selected = row.editable && selected.as_ref() == Some(&row.node);
        if is_selected {
            ui.fill(r, Color::BLUE_700);
        } else if hit.hovered() {
            ui.fill(r, Color::SLATE_800);
        }
        if row.has_children {
            caret(
                ui,
                x + 5.0,
                y + ROW_H * 0.5,
                expanded.contains(&row.node.key()),
            );
        }
        let fg = match (row.editable, row.prefab, row.depth) {
            (false, _, _) => Color::GRAY_500,
            (_, true, _) => PREFAB_COLOR,
            (_, _, 0) => Color::GRAY_100,
            _ => Color::GRAY_300,
        };
        ui.add(
            Text::new(x + 14.0, y + 4.0, &row.label)
                .font_size(14.0)
                .fg(fg)
                .truncate(list.x + list.w - x - 22.0),
        );
        let on_button = row.inherited.is_some() && ui.cursor.0 > list.x + list.w - 28.0;
        if let Some((parent, ci, removed)) = &row.inherited {
            let (ci, removed) = (*ci, *removed);
            if removed {
                let lw = ui.text_width(14.0, &row.label);
                ui.fill(
                    Rect::new(x + 14.0, y + ROW_H * 0.5, lw.min(list.w - x - 22.0), 1.0),
                    Color::GRAY_500,
                );
            }
            if hit.hovered() || removed {
                let b = Rect::new(list.x + list.w - 26.0, y + 2.0, 20.0, ROW_H - 4.0);
                if flat_button(
                    ui,
                    b,
                    ("tree-inherited", i),
                    if removed { "↺" } else { "×" },
                    false,
                ) {
                    toggle_child = Some((parent.clone(), ci));
                }
            }
        }
        if hit.clicked() && !on_button {
            if row.has_children && ui.cursor.0 < x + 14.0 {
                toggle = Some(row.node.key());
            } else if row.editable {
                select = Some(row.node.clone());
            } else if let Some((parent, ci, false)) = &row.inherited {
                create = Some((parent.clone(), *ci));
            }
        }
    }
    ui.pop_clip();

    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.tree_scroll = offset;
        if let Some(k) = toggle
            && !s.expanded.remove(&k)
        {
            s.expanded.insert(k);
        }
        if let Some(n) = select {
            node::select(s, Some(n));
        }
    }
    if let Some((parent, ci)) = create
        && let Some(n) = node::override_child(world, &parent, ci)
        && let Ok(s) = world.resource_mut::<EditorState>()
    {
        s.expanded.insert(parent.key());
        node::select(s, Some(n));
    }
    if let Some((n, ci)) = toggle_child {
        if let Ok(s) = world.resource_mut::<EditorState>()
            && let Some(d) = node::def_mut(s, &n)
        {
            if d.removed_children.contains(&ci) {
                d.removed_children.retain(|c| *c != ci);
            } else {
                d.removed_children.push(ci);
                d.removed_children.sort_unstable();
            }
        }
        node::respawn(world, n.root);
    }
    cmd
}
