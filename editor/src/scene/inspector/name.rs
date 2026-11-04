use tungstengine::ui::prelude::*;

use super::form::{EDIT_H, Form};

const SLOT: u64 = 0;

pub fn draw(f: &mut Form, name: &mut Option<String>, fallback: &str) {
    let h = EDIT_H + 4.0;
    if f.visible(h) {
        let shown = name.clone().unwrap_or_else(|| fallback.to_string());
        let r = Rect::new(f.x, f.y + 2.0, f.w, EDIT_H);
        if let Some(t) = f.input(SLOT, r, shown) {
            let t = t.trim();
            *name = (!t.is_empty()).then(|| t.to_string());
        }
    }
    f.y += h;
}
