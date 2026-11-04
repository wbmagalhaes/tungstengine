use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use winit::event::MouseButton;

use tungstengine::color::Color;
use tungstengine::ecs::prelude::*;
use tungstengine::input::Input;
use tungstengine::ui::prelude::*;

use super::menu::{BAR_H, flat_button};

const DIALOG_PATH: &str = "assets/dialogs/test.dialog";
const NW: f32 = 168.0;
const NH: f32 = 52.0;
const TOP_H: f32 = 36.0;

#[derive(Serialize, Deserialize, Clone)]
struct OptionDef {
    text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    target: Option<usize>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Node {
    id: usize,
    kind: String,
    #[serde(default)]
    text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    set: Option<String>,
    pos: [f32; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    next: Option<usize>,
    #[serde(default, rename = "option", skip_serializing_if = "Vec::is_empty")]
    options: Vec<OptionDef>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct Doc {
    #[serde(default)]
    speaker: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    start: Option<usize>,
    #[serde(default, rename = "node")]
    nodes: Vec<Node>,
}

#[derive(Resource)]
pub struct DialogEditor {
    doc: Doc,
    selected: Option<usize>,
    dragging: Option<usize>,
    drag_off: (f32, f32),
    connect: Option<(usize, Option<usize>)>,
    next_id: usize,
    status: String,
    path: Option<PathBuf>,
}

impl Default for DialogEditor {
    fn default() -> Self {
        let path = Path::new(DIALOG_PATH);
        let doc = read_doc(path).unwrap_or_default();
        DialogEditor::with_doc(doc, path.exists().then(|| path.to_path_buf()))
    }
}

impl DialogEditor {
    fn with_doc(doc: Doc, path: Option<PathBuf>) -> Self {
        let next_id = doc.nodes.iter().map(|n| n.id + 1).max().unwrap_or(0);
        DialogEditor {
            doc,
            selected: None,
            dragging: None,
            drag_off: (0.0, 0.0),
            connect: None,
            next_id,
            status: String::new(),
            path,
        }
    }
}

fn read_doc(path: &Path) -> Result<Doc, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    toml::from_str(&text).map_err(|e| e.to_string())
}

pub fn is_loaded(world: &World) -> bool {
    world.resource::<DialogEditor>().is_ok()
}

pub fn path(world: &World) -> Option<PathBuf> {
    world
        .resource::<DialogEditor>()
        .ok()
        .and_then(|e| e.path.clone())
}

pub fn open(world: &mut World, path: &Path) -> Result<(), String> {
    let doc = read_doc(path)?;
    world.insert_resource(DialogEditor::with_doc(doc, Some(path.to_path_buf())));
    Ok(())
}

pub fn new_doc(world: &mut World) {
    world.insert_resource(DialogEditor::with_doc(Doc::default(), None));
}

pub fn save(world: &mut World, path: PathBuf) {
    let Ok(ed) = world.resource_mut::<DialogEditor>() else {
        return;
    };
    let text = toml::to_string_pretty(&ed.doc).unwrap_or_default();
    ed.status = match fs::write(&path, text) {
        Ok(_) => format!("saved: {} nodes", ed.doc.nodes.len()),
        Err(e) => format!("error: {e}"),
    };
    ed.path = Some(path);
}

fn node_rect(n: &Node) -> Rect {
    Rect::new(n.pos[0], n.pos[1], NW, NH)
}

fn node_color(kind: &str) -> Color {
    match kind {
        "choice" => Color::SLATE_700,
        "end" => Color::GRAY_700,
        _ => Color::GRAY_700,
    }
}

pub fn ensure(world: &mut World) {
    if world.resource::<DialogEditor>().is_err() {
        world.insert_resource(DialogEditor::default());
    }
}

pub fn frame(world: &mut World, ui: &mut UI) -> bool {
    let (click, pressed, held, released, cursor) = match world.resource::<Input>() {
        Ok(i) => (
            i.mouse_released(MouseButton::Left),
            i.mouse_pressed(MouseButton::Left),
            i.mouse_held(MouseButton::Left),
            i.mouse_released(MouseButton::Left),
            i.cursor,
        ),
        Err(_) => return false,
    };
    let _ = click;

    let mut ed = match world.resource::<DialogEditor>() {
        Ok(e) => DialogEditor {
            doc: e.doc.clone(),
            selected: e.selected,
            dragging: e.dragging,
            drag_off: e.drag_off,
            connect: e.connect,
            next_id: e.next_id,
            status: e.status.clone(),
            path: e.path.clone(),
        },
        Err(_) => return false,
    };

    // top bar
    let mut back = false;
    ui.fill(Rect::new(0.0, BAR_H, ui.width, TOP_H), Color::SLATE_900);
    let top = |x: f32, w: f32, label: &str, ui: &mut UI| {
        flat_button(
            ui,
            Rect::new(x, BAR_H + 4.0, w, TOP_H - 8.0),
            ("dlg", label),
            label,
            false,
        )
    };

    if top(8.0, 70.0, "< Scene", ui) {
        back = true;
    }

    let add = |kind: &str, ed: &mut DialogEditor| {
        let id = ed.next_id;
        ed.next_id += 1;
        let n = Node {
            id,
            kind: kind.to_string(),
            text: if kind == "end" {
                String::new()
            } else {
                "...".to_string()
            },
            set: None,
            pos: [
                240.0 + (id as f32 % 5.0) * 30.0,
                120.0 + (id as f32 % 7.0) * 24.0,
            ],
            next: None,
            options: Vec::new(),
        };
        ed.doc.nodes.push(n);
        ed.selected = Some(id);
    };

    if top(92.0, 76.0, "+ Line", ui) {
        add("line", &mut ed);
    }

    if top(172.0, 86.0, "+ Choice", ui) {
        add("choice", &mut ed);
    }

    if top(262.0, 60.0, "+ End", ui) {
        add("end", &mut ed);
    }

    if top(326.0, 70.0, "Delete", ui)
        && let Some(sel) = ed.selected
    {
        ed.doc.nodes.retain(|n| n.id != sel);
        for n in &mut ed.doc.nodes {
            if n.next == Some(sel) {
                n.next = None;
            }
            for o in &mut n.options {
                if o.target == Some(sel) {
                    o.target = None;
                }
            }
        }
        ed.selected = None;
    }
    ui.add(Text::new(412.0, BAR_H + 10.0, &ed.status).font_size(14.0));

    // connections (drawn under nodes)
    let centers: Vec<(usize, f32, f32)> = ed
        .doc
        .nodes
        .iter()
        .map(|n| (n.id, n.pos[0] + NW / 2.0, n.pos[1] + NH / 2.0))
        .collect();
    let center_of = |id: usize| {
        centers
            .iter()
            .find(|(i, _, _)| *i == id)
            .map(|(_, x, y)| (*x, *y))
    };

    for n in &ed.doc.nodes {
        let from = (n.pos[0] + NW / 2.0, n.pos[1] + NH);
        if let Some(t) = n.next
            && let Some((tx, ty)) = center_of(t)
        {
            draw_line(ui, from, (tx, ty), Color::GRAY_400);
        }
        for o in &n.options {
            if let Some(t) = o.target
                && let Some((tx, ty)) = center_of(t)
            {
                draw_line(ui, from, (tx, ty), Color::SLATE_400);
            }
        }
    }

    // nodes
    for n in &ed.doc.nodes {
        let r = node_rect(n);
        let sel = ed.selected == Some(n.id);
        ui.fill(
            Rect::new(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0),
            if sel {
                Color::AMBER_200
            } else {
                Color::GRAY_900
            },
        );
        ui.fill(r, node_color(&n.kind));
        let label = match n.kind.as_str() {
            "choice" => "[escolha]".to_string(),
            "end" => "[fim]".to_string(),
            _ => clip(&n.text, 22),
        };
        ui.add(
            Text::new(r.x + 8.0, r.y + 8.0, &format!("#{}", n.id))
                .font_size(14.0)
                .fg(Color::GRAY_400),
        );
        ui.add(Text::new(r.x + 8.0, r.y + 26.0, &label).font_size(15.0));
    }

    // panel area on the left blocks node interaction
    let over_panel = cursor.0 < 250.0 || cursor.1 < BAR_H + TOP_H;

    // node interaction (drag / select / connect target)
    if pressed && !over_panel {
        let hit = ed
            .doc
            .nodes
            .iter()
            .rev()
            .find(|n| node_rect(n).contains(cursor))
            .map(|n| n.id);
        if let Some((src, opt)) = ed.connect.take() {
            if let Some(t) = hit
                && let Some(n) = ed.doc.nodes.iter_mut().find(|n| n.id == src)
            {
                match opt {
                    Some(oi) => {
                        if let Some(o) = n.options.get_mut(oi) {
                            o.target = Some(t);
                        }
                    }
                    None => n.next = Some(t),
                }
            }
        } else if let Some(id) = hit {
            ed.selected = Some(id);
            ed.dragging = Some(id);
            if let Some(n) = ed.doc.nodes.iter().find(|n| n.id == id) {
                ed.drag_off = (cursor.0 - n.pos[0], cursor.1 - n.pos[1]);
            }
        } else {
            ed.selected = None;
        }
    }

    if held
        && let Some(id) = ed.dragging
        && let Some(n) = ed.doc.nodes.iter_mut().find(|n| n.id == id)
    {
        n.pos = [cursor.0 - ed.drag_off.0, cursor.1 - ed.drag_off.1];
    }

    if released {
        ed.dragging = None;
    }

    // selected node panel
    if let Some(sel) = ed.selected
        && let Some(idx) = ed.doc.nodes.iter().position(|n| n.id == sel)
    {
        panel(ui, &mut ed, idx);
    }

    world.insert_resource(ed);
    back
}

fn panel(ui: &mut UI, ed: &mut DialogEditor, idx: usize) {
    let px = 16.0;
    let pw = 220.0;
    ui.fill(
        Rect::new(0.0, BAR_H + TOP_H, 250.0, ui.height - BAR_H - TOP_H),
        Color::GRAY_900.with_alpha(0.94),
    );
    let kind = ed.doc.nodes[idx].kind.clone();
    ui.add(
        Text::new(
            px,
            BAR_H + TOP_H + 14.0,
            &format!("Node #{} ({kind})", ed.doc.nodes[idx].id),
        )
        .fg(Color::TEXT_TITLE),
    );

    let mut y = BAR_H + TOP_H + 42.0;
    if kind != "end" {
        ui.add(Text::new(px, y, "texto:").font_size(14.0));
        ui.add(TextInput::new(
            Rect::new(px, y + 18.0, pw, 28.0),
            7001u64,
            &mut ed.doc.nodes[idx].text,
        ));
        y += 56.0;
    }

    if kind == "line" {
        // set flag
        let mut setflag = ed.doc.nodes[idx].set.clone().unwrap_or_default();
        ui.add(Text::new(px, y, "set flag:").font_size(14.0));
        ui.add(TextInput::new(
            Rect::new(px, y + 18.0, pw, 28.0),
            7002u64,
            &mut setflag,
        ));
        ed.doc.nodes[idx].set = (!setflag.trim().is_empty()).then_some(setflag);
        y += 56.0;
        // next
        let next_label = match ed.doc.nodes[idx].next {
            Some(t) => format!("next: #{t}"),
            None => "next: (none)".to_string(),
        };
        ui.add(Text::new(px, y, &next_label).font_size(14.0));
        if ui
            .button(
                Rect::new(px, y + 18.0, 130.0, 26.0).with_padding(3.0),
                "connect >",
            )
            .clicked()
        {
            ed.connect = Some((ed.doc.nodes[idx].id, None));
        }
        y += 52.0;
    }

    if kind == "choice" {
        let id = ed.doc.nodes[idx].id;
        let count = ed.doc.nodes[idx].options.len();
        for oi in 0..count {
            ui.add(TextInput::new(
                Rect::new(px, y, 150.0, 26.0),
                7100u64 + oi as u64,
                &mut ed.doc.nodes[idx].options[oi].text,
            ));
            let tgt = ed.doc.nodes[idx].options[oi].target;
            let lbl = match tgt {
                Some(t) => format!(">#{t}"),
                None => ">?".to_string(),
            };
            if ui
                .button(Rect::new(px + 156.0, y, 56.0, 26.0).with_padding(3.0), &lbl)
                .clicked()
            {
                ed.connect = Some((id, Some(oi)));
            }
            y += 30.0;
        }
        if ui
            .button(Rect::new(px, y, 130.0, 26.0).with_padding(3.0), "+ option")
            .clicked()
        {
            ed.doc.nodes[idx].options.push(OptionDef {
                text: "opcao".to_string(),
                target: None,
            });
        }
        y += 34.0;
    }

    if let Some((_, _)) = ed.connect {
        ui.add(
            Text::new(px, y, "click a node to connect")
                .font_size(14.0)
                .fg(Color::TEXT_TITLE),
        );
    }
}

fn clip(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        let t: String = s.chars().take(n - 1).collect();
        format!("{t}…")
    } else {
        s.to_string()
    }
}

fn draw_line(ui: &mut UI, a: (f32, f32), b: (f32, f32), color: Color) {
    let steps = 24;
    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let x = a.0 + (b.0 - a.0) * t;
        let y = a.1 + (b.1 - a.1) * t;
        ui.fill(Rect::new(x - 1.5, y - 1.5, 3.0, 3.0), color);
    }
}
