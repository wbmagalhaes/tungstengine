use std::hash::Hash;

use tungstengine::color::Color;
use tungstengine::ui::prelude::*;

use super::gizmo::Tool;
use super::state::DocKind;

pub const BAR_H: f32 = 28.0;
pub const TOOLBAR_H: f32 = 36.0;
const ITEM_H: f32 = 26.0;
const DROP_W: f32 = 200.0;
const FONT: f32 = 14.0;
const Z: f32 = 10.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileOp {
    Open,
    New,
    Save,
    SaveAs,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Cmd {
    File(DocKind, FileOp),
    SaveCurrent,
    SaveCurrentAs,
    AddObject,
    Delete,
    Focus,
    Axis(usize),
    Tool(Tool),
    TogglePaint,
}

#[derive(Clone, Copy, Default)]
pub struct MenuState {
    pub open: Option<usize>,
    pub sub: Option<usize>,
}

struct Item {
    cmd: Cmd,
    label: &'static str,
    shortcut: &'static str,
}

enum Entry {
    Item(Item),
    Sub(&'static str, &'static [Item]),
}

const fn item(cmd: Cmd, label: &'static str, shortcut: &'static str) -> Item {
    Item {
        cmd,
        label,
        shortcut,
    }
}

const fn file_menu(kind: DocKind, labels: [&'static str; 4]) -> [Item; 4] {
    [
        item(Cmd::File(kind, FileOp::Open), labels[0], ""),
        item(Cmd::File(kind, FileOp::New), labels[1], ""),
        item(Cmd::File(kind, FileOp::Save), labels[2], ""),
        item(Cmd::File(kind, FileOp::SaveAs), labels[3], ""),
    ]
}

const SCENE_ITEMS: [Item; 4] = file_menu(
    DocKind::Scene,
    ["Open scene", "New scene", "Save scene", "Save scene as"],
);
const PREFAB_ITEMS: [Item; 4] = file_menu(
    DocKind::Prefab,
    ["Open prefab", "New prefab", "Save prefab", "Save prefab as"],
);
const DIALOG_ITEMS: [Item; 4] = file_menu(
    DocKind::Dialog,
    ["Open dialog", "New dialog", "Save dialog", "Save dialog as"],
);

const MENUS: &[(&str, &[Entry])] = &[
    (
        "File",
        &[
            Entry::Sub("Scenes", &SCENE_ITEMS),
            Entry::Sub("Prefabs", &PREFAB_ITEMS),
            Entry::Sub("Dialogs", &DIALOG_ITEMS),
        ],
    ),
    (
        "Tools",
        &[
            Entry::Item(item(Cmd::Tool(Tool::Move), "Move", "W")),
            Entry::Item(item(Cmd::Tool(Tool::Scale), "Scale", "E")),
            Entry::Item(item(Cmd::Tool(Tool::Rotate), "Rotate", "R")),
            Entry::Item(item(Cmd::TogglePaint, "Paint walk grid", "P")),
        ],
    ),
];

fn headers(ui: &UI) -> Vec<Rect> {
    let mut x = 6.0;
    MENUS
        .iter()
        .map(|(label, _)| {
            let w = ui.text_width(FONT, label) + 20.0;
            let r = Rect::new(x, 0.0, w, BAR_H).with_z(Z);
            x += w;
            r
        })
        .collect()
}

fn dropdown(ui: &UI, open: usize) -> Rect {
    let h = headers(ui)[open];
    let n = MENUS[open].1.len() as f32;
    Rect::new(h.x, BAR_H, DROP_W, n * ITEM_H + 8.0).with_z(Z)
}

fn submenu(ui: &UI, m: &MenuState) -> Option<(Rect, &'static [Item])> {
    let open = m.open?;
    let sub = m.sub?;
    let Entry::Sub(_, items) = MENUS[open].1.get(sub)? else {
        return None;
    };
    let d = dropdown(ui, open);
    let r = Rect::new(
        d.x + d.w - 2.0,
        d.y + sub as f32 * ITEM_H,
        DROP_W,
        items.len() as f32 * ITEM_H + 8.0,
    )
    .with_z(Z);
    Some((r, items))
}

pub fn covers(ui: &UI, m: &MenuState, p: (f32, f32)) -> bool {
    p.1 < BAR_H
        || m.open.is_some_and(|i| dropdown(ui, i).contains(p))
        || submenu(ui, m).is_some_and(|(r, _)| r.contains(p))
}

pub fn block(ui: &mut UI, m: &MenuState) {
    if let Some(i) = m.open {
        let r = dropdown(ui, i);
        ui.block_pointer(r);
    }
    if let Some((r, _)) = submenu(ui, m) {
        ui.block_pointer(r);
    }
}

pub fn flat_button(ui: &mut UI, r: Rect, id: impl Hash, label: &str, active: bool) -> bool {
    let tw = ui.text_width(FONT, label);
    flat(ui, r, id, label, active, r.x + (r.w - tw) * 0.5)
}

pub fn list_item(ui: &mut UI, r: Rect, id: impl Hash, label: &str, active: bool) -> bool {
    flat(ui, r, id, label, active, r.x + 10.0)
}

fn flat(ui: &mut UI, r: Rect, id: impl Hash, label: &str, active: bool, text_x: f32) -> bool {
    let hit = ui.interact_id(r, id);
    let bg = if active {
        Color::SLATE_600
    } else if hit.hovered() {
        Color::SLATE_700
    } else {
        Color::CLEAR
    };
    ui.fill(r, bg);
    ui.add(
        Text::new(text_x, r.y + (r.h - FONT) * 0.5, label)
            .font_size(FONT)
            .fg(Color::GRAY_100)
            .z(r.z),
    );
    hit.clicked()
}

fn item_row(
    ui: &mut UI,
    r: Rect,
    id: impl Hash,
    label: &str,
    right: &str,
    check: bool,
    highlight: bool,
) -> (bool, bool) {
    let hit = ui.interact_id(r, id);
    if hit.hovered() || highlight {
        ui.fill(r, Color::BLUE_700);
    }
    let ty = r.y + (ITEM_H - FONT) * 0.5;
    if check {
        ui.add(
            Text::new(r.x + 6.0, ty, "*")
                .font_size(FONT)
                .fg(Color::GRAY_100)
                .z(Z),
        );
    }
    ui.add(
        Text::new(r.x + 22.0, ty, label)
            .font_size(FONT)
            .fg(Color::GRAY_100)
            .z(Z),
    );
    let sw = ui.text_width(FONT, right);
    ui.add(
        Text::new(r.x + r.w - sw - 8.0, ty, right)
            .font_size(FONT)
            .fg(Color::GRAY_400)
            .z(Z),
    );
    (hit.hovered(), hit.clicked())
}

fn panel_bg(ui: &mut UI, r: Rect) {
    ui.fill(r, Color::SLATE_800);
    ui.fill(Rect::new(r.x, r.y, r.w, 1.0).with_z(Z), Color::SLATE_600);
}

pub fn bar(
    ui: &mut UI,
    m: &mut MenuState,
    title: &str,
    checked: impl Fn(Cmd) -> bool,
) -> Option<Cmd> {
    ui.fill(Rect::new(0.0, 0.0, ui.width, BAR_H), Color::SLATE_950);
    let tw = ui.text_width(FONT, title);
    ui.add(
        Text::new((ui.width - tw) * 0.5, (BAR_H - FONT) * 0.5, title)
            .font_size(FONT)
            .fg(Color::GRAY_300),
    );

    let hs = headers(ui);
    let mut cmd = None;
    let mut toggled = false;
    for (i, r) in hs.iter().enumerate() {
        let hit = ui.interact_id(*r, ("menu", i));
        if hit.hovered() && m.open.is_some() && m.open != Some(i) {
            m.open = Some(i);
            m.sub = None;
        }
        if hit.clicked() {
            m.open = if m.open == Some(i) { None } else { Some(i) };
            m.sub = None;
            toggled = true;
        }
        let bg = if m.open == Some(i) {
            Color::SLATE_700
        } else if hit.hovered() {
            Color::SLATE_800
        } else {
            Color::CLEAR
        };
        ui.fill(*r, bg);
        ui.add(
            Text::new(r.x + 10.0, (BAR_H - FONT) * 0.5, MENUS[i].0)
                .font_size(FONT)
                .fg(Color::GRAY_200)
                .z(Z),
        );
    }

    let open = m.open?;
    let d = dropdown(ui, open);
    panel_bg(ui, d);
    for (k, entry) in MENUS[open].1.iter().enumerate() {
        let r = Rect::new(d.x + 4.0, d.y + 4.0 + k as f32 * ITEM_H, d.w - 8.0, ITEM_H).with_z(Z);
        match entry {
            Entry::Item(it) => {
                let (hovered, clicked) = item_row(
                    ui,
                    r,
                    ("menu-item", open, k),
                    it.label,
                    it.shortcut,
                    checked(it.cmd),
                    false,
                );
                if hovered {
                    m.sub = None;
                }
                if clicked {
                    cmd = Some(it.cmd);
                }
            }
            Entry::Sub(label, _) => {
                let (hovered, clicked) = item_row(
                    ui,
                    r,
                    ("menu-sub", open, k),
                    label,
                    ">",
                    false,
                    m.sub == Some(k),
                );
                if hovered || clicked {
                    m.sub = Some(k);
                }
            }
        }
    }

    let sub = submenu(ui, m);
    if let Some((s, items)) = sub {
        panel_bg(ui, s);
        for (k, it) in items.iter().enumerate() {
            let r =
                Rect::new(s.x + 4.0, s.y + 4.0 + k as f32 * ITEM_H, s.w - 8.0, ITEM_H).with_z(Z);
            let (_, clicked) = item_row(
                ui,
                r,
                ("menu-sub-item", open, k),
                it.label,
                it.shortcut,
                checked(it.cmd),
                false,
            );
            if clicked {
                cmd = Some(it.cmd);
            }
        }
    }

    let inside = d.contains(ui.cursor)
        || sub.is_some_and(|(s, _)| s.contains(ui.cursor))
        || hs.iter().any(|h| h.contains(ui.cursor));
    if cmd.is_some() || (ui.clicked && !inside && !toggled) {
        *m = MenuState::default();
    }
    cmd
}
