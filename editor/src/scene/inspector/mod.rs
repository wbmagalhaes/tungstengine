mod collider;
mod components;
mod form;
mod generic;
mod name;
mod transform;

use tungstengine::color::Color;
use tungstengine::ecs::prelude::*;
use tungstengine::scene::{GameObjDef, SpawnRegistry};
use tungstengine::ui::prelude::*;

use super::files::Modal;
use super::node::{self, NodeRef};
use super::state::EditorState;
use super::tree::{PREFAB_COLOR, def_label, placed_label};
pub use components::{InspectorFn, set_custom};
pub use form::Form;
use form::{FONT, LINE_H};
pub use generic::edit_value;

const HEADER_H: f32 = 32.0;
const HIDDEN_COMPONENTS: [&str; 2] = ["transform", "walk_grid"];
const CATEGORY_ORDER: [&str; 6] = [
    "Rendering",
    "Physics",
    "Interaction",
    "Flags & Triggers",
    "Game",
    "Other",
];

enum Action {
    AddComponent(Vec<String>),
    AddChild,
}

fn category_rank(c: &str) -> (usize, String) {
    let pos = CATEGORY_ORDER
        .iter()
        .position(|x| *x == c)
        .unwrap_or(CATEGORY_ORDER.len());
    (pos, c.to_string())
}

fn structural(world: &mut World, n: &NodeRef, action: Action) {
    match action {
        Action::AddComponent(present) => {
            let groups = world
                .resource::<SpawnRegistry>()
                .map(|r| {
                    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
                    for t in r.tags() {
                        if HIDDEN_COMPONENTS.contains(&t.as_str()) || present.contains(&t) {
                            continue;
                        }
                        let cat = r.category(&t).to_string();
                        match groups.iter_mut().find(|(c, _)| *c == cat) {
                            Some((_, list)) => list.push(t),
                            None => groups.push((cat, vec![t])),
                        }
                    }
                    groups.sort_by_key(|(c, _)| category_rank(c));
                    groups
                })
                .unwrap_or_default();
            if let Ok(s) = world.resource_mut::<EditorState>() {
                s.modal = Some(Modal::AddComponent {
                    groups,
                    scroll: 0.0,
                });
            }
        }
        Action::AddChild => {
            let Ok(s) = world.resource_mut::<EditorState>() else {
                return;
            };
            let Some(d) = node::def_mut(s, n) else {
                return;
            };
            let mut c = GameObjDef::empty("Child");
            c.set_transform(glam::Vec3::ZERO, glam::Vec3::ZERO, glam::Vec3::ONE);
            d.children.push(c);
            let mut path = n.path.clone();
            path.push(d.children.len() - 1);
            s.expanded.insert(n.key());
            node::respawn(world, n.root);
            if let Ok(s) = world.resource_mut::<EditorState>() {
                node::select(s, Some(NodeRef { root: n.root, path }));
            }
        }
    }
}

pub fn frame(world: &mut World, ui: &mut UI, panel: Rect, scroll: f32, enter: bool) {
    if panel.h < HEADER_H + LINE_H {
        return;
    }
    ui.fill(panel, Color::SLATE_900.with_alpha(0.94));
    ui.block_pointer(panel);
    ui.add(
        Text::new(panel.x + 10.0, panel.y + 8.0, "Inspector")
            .font_size(15.0)
            .fg(Color::GRAY_200),
    );
    let body = Rect::new(panel.x, panel.y + HEADER_H, panel.w, panel.h - HEADER_H);

    let Ok(s) = world.resource::<EditorState>() else {
        return;
    };
    let selected = node::selected(s);
    let Some((n, d, (pos0, rot0, scale0))) = selected.and_then(|n| {
        let d = node::def(s, &n)?.clone();
        let l = node::local(world, &n)?;
        Some((n, d, l))
    }) else {
        ui.add(
            Text::new(body.x + 10.0, body.y + 4.0, "nothing selected")
                .font_size(FONT)
                .fg(Color::GRAY_500),
        );
        return;
    };
    let fallback = if n.path.is_empty() {
        s.placed
            .get(n.root)
            .map(|p| placed_label(p, &s.gameobjs))
            .unwrap_or_default()
    } else {
        d.prefab_child
            .and_then(|_| node::base(world, &n))
            .map(|b| def_label(&b))
            .unwrap_or_else(|| def_label(&d))
    };
    let mut offset = s.inspector_scroll;
    let buf = s.field_buf.clone();
    let locks = s.inspector_locks.clone();
    let hidden_gizmos = s.hidden_gizmos.clone();
    let open_items = s.inspector_open.clone();
    let template = node::base(world, &n);
    let entity = node::entity(world, &n);

    if body.contains(ui.cursor) {
        offset = (offset - scroll * LINE_H * 2.0).max(0.0);
    }

    let mut name = d.name.clone();
    let (mut pos, mut rot, mut scale) = (pos0, rot0, scale0);
    let mut action = None;
    let registry_defaults = |ty: &str| {
        world
            .resource::<SpawnRegistry>()
            .ok()
            .and_then(|r| r.defaults(ty))
            .unwrap_or_default()
    };
    let views = components::views(&d, template.as_ref(), &registry_defaults);

    ui.push_clip(body);
    let mut f = Form::new(ui, body, offset, buf, locks);
    f.hidden_gizmos = hidden_gizmos;
    f.open = open_items;
    let start = f.y;

    name::draw(&mut f, &mut name, &fallback);
    if let Some(t) = &template {
        f.row_colored(
            "prefab",
            &format!("{} (overrides)", t.name.as_deref().unwrap_or(&t.key)),
            PREFAB_COLOR,
        );
    }
    if !d.key.is_empty() {
        f.row("key", &d.key);
    }
    if let Some(e) = entity {
        f.row("entity", &format!("#{}", e.id));
    }
    f.gap();

    transform::draw(&mut f, &mut pos, &mut rot, &mut scale);

    let comp_action = components::draw(&mut f, &views, template.is_some());
    if f.button("+ Component") {
        action = Some(Action::AddComponent(components::present_types(&views)));
    }
    if f.button("+ Child") {
        action = Some(Action::AddChild);
    }

    let multiline = f.multiline_focused();
    let content_h = f.y - start;
    let hidden_gizmos = std::mem::take(&mut f.hidden_gizmos);
    let open_items = std::mem::take(&mut f.open);
    let (buf, mut focused, locks) = f.finish();
    ui.pop_clip();

    if enter && focused && !multiline {
        ui.set_focus(None);
        focused = false;
    }
    let offset = offset.min((content_h - body.h + 8.0).max(0.0));

    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.inspector_scroll = offset;
        s.field_buf = buf.filter(|_| focused);
        s.inspector_locks = locks;
        s.hidden_gizmos = hidden_gizmos;
        s.inspector_open = open_items;
        if name != d.name
            && let Some(dd) = node::def_mut(s, &n)
        {
            dd.name = name;
        }
    }
    if let Some(a) = comp_action {
        components::apply(world, &n, &views, a);
    }
    if (pos, rot, scale) != (pos0, rot0, scale0) {
        node::set_local(world, &n, pos, rot, scale);
    }
    if let Some(a) = action {
        structural(world, &n, a);
    }
}

fn new_component(tag: &str) -> toml::Value {
    let mut t = toml::Table::new();
    t.insert("type".into(), toml::Value::String(tag.into()));
    if tag == "renderer" {
        t.insert("mesh".into(), toml::Value::String("cube".into()));
    }
    toml::Value::Table(t)
}

pub fn add_component(world: &mut World, tag: &str) {
    let Some(n) = world
        .resource::<EditorState>()
        .ok()
        .and_then(node::selected)
    else {
        return;
    };
    if let Ok(s) = world.resource_mut::<EditorState>()
        && let Some(d) = node::def_mut(s, &n)
    {
        d.components.push(new_component(tag));
    }
    node::respawn(world, n.root);
}

pub fn has_focus(world: &World) -> bool {
    world
        .resource::<EditorState>()
        .is_ok_and(|s| s.field_buf.is_some())
}
