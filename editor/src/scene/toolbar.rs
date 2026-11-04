use tungstengine::color::Color;
use tungstengine::ecs::prelude::*;
use tungstengine::ui::prelude::*;

use super::gizmo::Tool;
use super::menu::{BAR_H, Cmd, TOOLBAR_H, flat_button};
use super::state::EditorState;

pub fn frame(world: &mut World, ui: &mut UI) -> Option<Cmd> {
    let (tool, paint_mode) = world
        .resource::<EditorState>()
        .map(|s| (s.tool, s.paint_mode))
        .ok()?;

    ui.fill(Rect::new(0.0, BAR_H, ui.width, TOOLBAR_H), Color::SLATE_900);
    let by = BAR_H + 4.0;
    let bh = TOOLBAR_H - 8.0;
    let mut x = 8.0;
    let mut cmd = None;
    for (t, label) in [
        (Tool::Move, "Move"),
        (Tool::Scale, "Scale"),
        (Tool::Rotate, "Rotate"),
    ] {
        let r = Rect::new(x, by, 64.0, bh);
        if flat_button(ui, r, ("tool", label), label, !paint_mode && tool == t) {
            cmd = Some(Cmd::Tool(t));
        }
        x += 68.0;
    }
    cmd
}
