use std::fs;
use std::path::PathBuf;

use uuid::Uuid;

use tungstengine::color::Color;
use tungstengine::ui::prelude::*;

use super::menu::{flat_button, list_item};
use super::state::DocKind;

pub const INPUT_ID: u64 = 9101;
const Z: f32 = 20.0;
const W: f32 = 420.0;
const ROW_H: f32 = 28.0;
const MAX_ROWS: usize = 14;

enum ComponentRow {
    Header(String),
    Item(String),
}

fn component_rows(groups: &[(String, Vec<String>)]) -> Vec<ComponentRow> {
    groups
        .iter()
        .flat_map(|(name, tags)| {
            std::iter::once(ComponentRow::Header(name.clone()))
                .chain(tags.iter().map(|t| ComponentRow::Item(t.clone())))
        })
        .collect()
}

pub enum Modal {
    Open(DocKind, Vec<PathBuf>),
    SaveAs(DocKind, String),
    AddObject(Vec<(Uuid, String)>),
    AddComponent {
        groups: Vec<(String, Vec<String>)>,
        scroll: f32,
    },
}

pub enum Outcome {
    Open(DocKind, PathBuf),
    SaveAs(DocKind, PathBuf),
    Add(Option<Uuid>),
    AddComponent(String),
    Cancel,
}

pub fn list(kind: DocKind) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(kind.dir())
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(kind.ext()))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

pub fn frame(
    ui: &mut UI,
    modal: &mut Modal,
    enter: bool,
    escape: bool,
    wheel: f32,
) -> Option<Outcome> {
    let screen = Rect::new(0.0, 0.0, ui.width, ui.height).with_z(Z);
    ui.block_pointer(screen);
    ui.fill(screen, Color::BLACK.with_alpha(0.5));
    if escape {
        return Some(Outcome::Cancel);
    }

    let (kind, title) = match modal {
        Modal::Open(k, _) => (*k, format!("Open {}", k.title().to_lowercase())),
        Modal::SaveAs(k, _) => (*k, format!("Save {} as", k.title().to_lowercase())),
        Modal::AddObject(_) => (DocKind::Prefab, "Add object".to_string()),
        Modal::AddComponent { .. } => (DocKind::Prefab, "Add component".to_string()),
    };
    let rows = match modal {
        Modal::Open(_, files) => files.len().clamp(1, 14) as f32,
        Modal::SaveAs(..) => 1.0,
        Modal::AddObject(objs) => (objs.len() + 1).min(14) as f32,
        Modal::AddComponent { groups, .. } => {
            component_rows(groups).len().clamp(1, MAX_ROWS) as f32
        }
    };
    let h = 96.0 + rows * ROW_H;
    let panel = Rect::new((ui.width - W) * 0.5, (ui.height - h) * 0.4, W, h).with_z(Z);
    ui.fill(panel, Color::SLATE_800);
    ui.add(
        Text::new(panel.x + 16.0, panel.y + 14.0, &title)
            .font_size(16.0)
            .fg(Color::GRAY_100)
            .z(Z),
    );
    if matches!(modal, Modal::Open(..) | Modal::SaveAs(..)) {
        ui.add(
            Text::new(panel.x + 16.0, panel.y + 36.0, kind.dir())
                .font_size(13.0)
                .fg(Color::GRAY_400)
                .z(Z),
        );
    }

    let body_y = panel.y + 58.0;
    let footer = Rect::new(panel.x + W - 96.0, panel.y + h - 34.0, 84.0, 26.0).with_z(Z);
    let mut out = None;
    match modal {
        Modal::Open(_, files) => {
            if files.is_empty() {
                ui.add(
                    Text::new(panel.x + 16.0, body_y + 6.0, "no files")
                        .font_size(14.0)
                        .fg(Color::GRAY_400)
                        .z(Z),
                );
            }
            for (i, f) in files.iter().take(14).enumerate() {
                let r = Rect::new(
                    panel.x + 8.0,
                    body_y + i as f32 * ROW_H,
                    W - 16.0,
                    ROW_H - 2.0,
                )
                .with_z(Z);
                let name = f
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if list_item(ui, r, ("open-file", i), &name, false) {
                    out = Some(Outcome::Open(kind, f.clone()));
                }
            }
        }
        Modal::AddObject(objs) => {
            let entries = std::iter::once((None, "Empty".to_string()))
                .chain(objs.iter().map(|(id, n)| (Some(*id), n.clone())));
            for (i, (id, name)) in entries.take(14).enumerate() {
                let r = Rect::new(
                    panel.x + 8.0,
                    body_y + i as f32 * ROW_H,
                    W - 16.0,
                    ROW_H - 2.0,
                )
                .with_z(Z);
                if list_item(ui, r, ("add-object", i), &name, false) {
                    out = Some(Outcome::Add(id));
                }
            }
        }
        Modal::AddComponent { groups, scroll } => {
            let rows = component_rows(groups);
            let list =
                Rect::new(panel.x, body_y, W, MAX_ROWS.min(rows.len()) as f32 * ROW_H).with_z(Z);
            let max = (rows.len() as f32 * ROW_H - list.h).max(0.0);
            if list.contains(ui.cursor) {
                *scroll = (*scroll - wheel * ROW_H).clamp(0.0, max);
            }
            ui.push_clip(list);
            for (i, row) in rows.iter().enumerate() {
                let y = body_y + i as f32 * ROW_H - *scroll;
                if y + ROW_H < list.y || y > list.y + list.h {
                    continue;
                }
                match row {
                    ComponentRow::Header(name) => {
                        ui.add(
                            Text::new(panel.x + 16.0, y + 8.0, name)
                                .font_size(12.0)
                                .fg(Color::SKY_300)
                                .z(Z),
                        );
                    }
                    ComponentRow::Item(tag) => {
                        let r = Rect::new(panel.x + 20.0, y, W - 28.0, ROW_H - 2.0).with_z(Z);
                        if list_item(ui, r, ("add-component", i), tag, false) {
                            out = Some(Outcome::AddComponent(tag.clone()));
                        }
                    }
                }
            }
            ui.pop_clip();
        }
        Modal::SaveAs(_, name) => {
            ui.focus(INPUT_ID);
            ui.add(TextInput::new(
                Rect::new(panel.x + 16.0, body_y, W - 32.0, ROW_H).with_z(Z),
                INPUT_ID,
                name,
            ));
            let ok = Rect::new(footer.x - 92.0, footer.y, 84.0, 26.0).with_z(Z);
            if (flat_button(ui, ok, "modal-save", "Save", true) || enter) && !name.trim().is_empty()
            {
                out = Some(Outcome::SaveAs(kind, kind.path_for(name)));
            }
        }
    }
    if flat_button(ui, footer, "modal-cancel", "Cancel", false) {
        out = Some(Outcome::Cancel);
    }
    out
}
