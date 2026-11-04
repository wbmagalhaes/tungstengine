use std::hash::Hash;

use crate::color::Color;
use crate::ui::font;
use crate::ui::immediate::{UI, key_hash};
use crate::ui::input::{delete_selection, next_word, prev_word};
use crate::ui::rect::Rect;
use crate::ui::style::{Style, StylePatch, Styled};
use crate::ui::widget::Widget;

pub struct TextInput<'a> {
    rect: Rect,
    id: u64,
    buffer: &'a mut String,
    style: Style,
    patch: StylePatch,
}

impl<'a> TextInput<'a> {
    pub fn new(rect: Rect, id: impl Hash, buffer: &'a mut String) -> Self {
        TextInput {
            rect,
            id: key_hash(id),
            buffer,
            style: Style::DEFAULT_TEXT,
            patch: StylePatch::default(),
        }
    }
}

impl Styled for TextInput<'_> {
    fn patch_mut(&mut self) -> &mut StylePatch {
        &mut self.patch
    }
}

impl Widget for TextInput<'_> {
    type Out = bool;

    fn render(self, ui: &mut UI) -> bool {
        let rect = self.rect;
        let face = self.style.with(self.patch).font;
        let px = self.style.with(self.patch).font_size;
        let buffer = self.buffer;
        let chars: Vec<char> = buffer.chars().collect();

        let over = ui.pointer_over(rect);
        if ui.clicked && over {
            ui.set_focus(Some(self.id));
            ui.reset_blink();
            let idx = caret_at_x(rect, &chars, ui.cursor.0, face, px);
            let anchor = if ui.edit_input().shift {
                ui.caret_sel().1
            } else {
                idx
            };
            ui.set_caret_sel(idx, anchor);
        }
        let focused = ui.focused_id() == Some(self.id);
        if focused {
            ui.mark_keyboard();
        }

        let bg = if focused {
            Color::GRAY_700
        } else {
            Color::GRAY_800
        };
        ui.fill(rect, bg);

        let mut submitted = false;
        if focused {
            let edit = ui.edit_input();
            if edit.left
                || edit.right
                || edit.home
                || edit.end
                || edit.backspace
                || edit.delete
                || edit.paste.is_some()
                || !edit.typed.is_empty()
            {
                ui.reset_blink();
            }
            let (caret0, anchor0) = ui.caret_sel();
            let mut chars: Vec<char> = buffer.chars().collect();
            let mut caret = caret0.min(chars.len());
            let mut anchor = anchor0.min(chars.len());
            let has_sel = caret != anchor;

            if edit.left {
                if edit.ctrl {
                    caret = prev_word(&chars, caret);
                } else if !edit.shift && has_sel {
                    caret = caret.min(anchor);
                } else {
                    caret = caret.saturating_sub(1);
                }
                if !edit.shift {
                    anchor = caret;
                }
            }
            if edit.right {
                if edit.ctrl {
                    caret = next_word(&chars, caret);
                } else if !edit.shift && has_sel {
                    caret = caret.max(anchor);
                } else {
                    caret = (caret + 1).min(chars.len());
                }
                if !edit.shift {
                    anchor = caret;
                }
            }
            if edit.home {
                caret = 0;
                if !edit.shift {
                    anchor = 0;
                }
            }
            if edit.end {
                caret = chars.len();
                if !edit.shift {
                    anchor = caret;
                }
            }
            if edit.copy || edit.cut {
                let lo = caret.min(anchor);
                let hi = caret.max(anchor);
                if lo < hi {
                    ui.set_clipboard_out(chars[lo..hi].iter().collect());
                    if edit.cut {
                        delete_selection(&mut chars, &mut caret, &mut anchor);
                    }
                }
            }
            if let Some(text) = &edit.paste {
                delete_selection(&mut chars, &mut caret, &mut anchor);
                for c in text.chars().filter(|c| !c.is_control()) {
                    chars.insert(caret, c);
                    caret += 1;
                }
                anchor = caret;
            }
            if !edit.typed.is_empty() {
                delete_selection(&mut chars, &mut caret, &mut anchor);
                for c in edit.typed.chars() {
                    chars.insert(caret, c);
                    caret += 1;
                }
                anchor = caret;
            }
            if edit.backspace {
                if !delete_selection(&mut chars, &mut caret, &mut anchor) && caret > 0 {
                    chars.remove(caret - 1);
                    caret -= 1;
                }
                anchor = caret;
            }
            if edit.delete {
                if !delete_selection(&mut chars, &mut caret, &mut anchor) && caret < chars.len() {
                    chars.remove(caret);
                }
                anchor = caret;
            }
            if edit.enter {
                submitted = true;
                ui.set_focus(None);
            }

            *buffer = chars.iter().collect();
            ui.set_caret_sel(caret, anchor);
        }

        let chars: Vec<char> = buffer.chars().collect();
        let ty = rect.y + (rect.h - px) * 0.5;

        if focused {
            let (caret, anchor) = ui.caret_sel();
            let lo = caret.min(anchor);
            let hi = caret.max(anchor);
            if lo < hi {
                let x0 = caret_x(rect, &chars, lo, face, px);
                let x1 = caret_x(rect, &chars, hi, face, px);
                ui.fill(Rect::new(x0, ty, x1 - x0, px), Color::SLATE_600);
            }
        }

        ui.glyphs_font(rect.x + 8.0, ty, face, px, buffer, Color::GRAY_200, 0.0);

        if focused && ui.caret_on() {
            let caret = ui.caret_sel().0.min(chars.len());
            let cx = caret_x(rect, &chars, caret, face, px);
            ui.fill(Rect::new(cx, ty, 2.0, px), Color::INDIGO_50);
        }

        submitted
    }
}

fn advance_width(face: font::Font, px: f32, ch: char) -> f32 {
    font::advance(face, px, ch)
}

fn caret_x(rect: Rect, chars: &[char], idx: usize, face: font::Font, px: f32) -> f32 {
    let mut x = rect.x + 8.0;
    for &ch in &chars[..idx.min(chars.len())] {
        x += advance_width(face, px, ch);
    }
    x
}

fn caret_at_x(rect: Rect, chars: &[char], x: f32, face: font::Font, px: f32) -> usize {
    let mut best = 0;
    let mut best_d = f32::INFINITY;
    let mut cx = rect.x + 8.0;
    for idx in 0..=chars.len() {
        let d = (cx - x).abs();
        if d < best_d {
            best_d = d;
            best = idx;
        }
        if let Some(&ch) = chars.get(idx) {
            cx += advance_width(face, px, ch);
        }
    }
    best
}
