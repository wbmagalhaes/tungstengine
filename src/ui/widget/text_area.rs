use std::hash::Hash;

use crate::color::Color;
use crate::ui::font;
use crate::ui::immediate::{UI, key_hash};
use crate::ui::input::{delete_selection, next_word, prev_word};
use crate::ui::rect::Rect;
use crate::ui::style::{Style, Styled};
use crate::ui::widget::Widget;

const PAD: f32 = 6.0;

pub struct TextArea<'a> {
    rect: Rect,
    id: u64,
    buffer: &'a mut String,
    style: Style,
}

struct Row {
    start: usize,
    end: usize,
}

fn layout(chars: &[char], face: font::Font, px: f32, max_w: f32) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut start = 0;
    let mut x = 0.0;
    let mut last_space = None;
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\n' {
            rows.push(Row { start, end: i });
            start = i + 1;
            x = 0.0;
            last_space = None;
            i += 1;
            continue;
        }
        let adv = font::advance(face, px, ch);
        if x + adv > max_w && i > start {
            let cut = match last_space {
                Some(s) if s > start => s + 1,
                _ => i,
            };
            rows.push(Row { start, end: cut });
            start = cut;
            x = chars[start..i]
                .iter()
                .map(|c| font::advance(face, px, *c))
                .sum();
            last_space = None;
        }
        if ch == ' ' {
            last_space = Some(i);
        }
        x += adv;
        i += 1;
    }
    rows.push(Row {
        start,
        end: chars.len(),
    });
    rows
}

fn row_of(rows: &[Row], idx: usize) -> usize {
    rows.iter().rposition(|r| r.start <= idx).unwrap_or(0)
}

fn x_in_row(chars: &[char], row: &Row, idx: usize, face: font::Font, px: f32) -> f32 {
    chars[row.start..idx.clamp(row.start, row.end)]
        .iter()
        .map(|c| font::advance(face, px, *c))
        .sum()
}

fn idx_at_x(chars: &[char], row: &Row, x: f32, face: font::Font, px: f32) -> usize {
    let mut best = row.start;
    let mut best_d = f32::INFINITY;
    let mut cx = 0.0;
    for (i, ch) in chars[row.start..row.end]
        .iter()
        .map(Some)
        .chain(std::iter::once(None))
        .enumerate()
    {
        let d = (cx - x).abs();
        if d < best_d {
            best_d = d;
            best = row.start + i;
        }
        if let Some(c) = ch {
            cx += font::advance(face, px, *c);
        }
    }
    best
}

impl<'a> TextArea<'a> {
    pub fn new(rect: Rect, id: impl Hash, buffer: &'a mut String) -> Self {
        TextArea {
            rect,
            id: key_hash(id),
            buffer,
            style: Style::default(),
        }
    }

    pub fn height(text: &str, width: f32, px: f32, min_rows: usize) -> f32 {
        let chars: Vec<char> = text.chars().collect();
        let rows = layout(&chars, font::Font::Sans, px, width - PAD * 2.0)
            .len()
            .max(min_rows);
        rows as f32 * font::line_height(font::Font::Sans, px) + PAD * 2.0
    }
}

impl Styled for TextArea<'_> {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl Widget for TextArea<'_> {
    type Out = bool;

    fn render(self, ui: &mut UI) -> bool {
        let rect = self.rect;
        let face = self.style.computed(ui.theme.text()).font;
        let px = self.style.computed(ui.theme.text()).font_size;
        let lh = font::line_height(face, px);
        let inner_w = rect.w - PAD * 2.0;
        let buffer = self.buffer;
        let mut changed = false;

        let over = ui.pointer_over(rect);
        if ui.clicked && over {
            ui.set_focus(Some(self.id));
            ui.reset_blink();
            let chars: Vec<char> = buffer.chars().collect();
            let rows = layout(&chars, face, px, inner_w);
            let r =
                (((ui.cursor.1 - rect.y - PAD) / lh).floor().max(0.0) as usize).min(rows.len() - 1);
            let idx = idx_at_x(&chars, &rows[r], ui.cursor.0 - rect.x - PAD, face, px);
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
        ui.surface(
            rect,
            if focused {
                Color::GRAY_700
            } else {
                Color::GRAY_800
            },
        );

        if focused {
            let edit = ui.edit_input();
            if edit.left
                || edit.right
                || edit.up
                || edit.down
                || edit.home
                || edit.end
                || edit.backspace
                || edit.delete
                || edit.enter
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
            let before = chars.clone();

            if edit.left {
                if edit.ctrl {
                    caret = prev_word(&chars, caret);
                } else if !edit.shift && has_sel {
                    caret = caret.min(anchor);
                } else {
                    caret = caret.saturating_sub(1);
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
            }
            if edit.up || edit.down || edit.home || edit.end {
                let rows = layout(&chars, face, px, inner_w);
                let r = row_of(&rows, caret);
                let x = x_in_row(&chars, &rows[r], caret, face, px);
                if edit.up && r > 0 {
                    caret = idx_at_x(&chars, &rows[r - 1], x, face, px);
                }
                if edit.down && r + 1 < rows.len() {
                    caret = idx_at_x(&chars, &rows[r + 1], x, face, px);
                }
                if edit.home {
                    caret = rows[r].start;
                }
                if edit.end {
                    caret = rows[r].end;
                }
            }
            if (edit.left || edit.right || edit.up || edit.down || edit.home || edit.end)
                && !edit.shift
            {
                anchor = caret;
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
            let insert = |chars: &mut Vec<char>, caret: &mut usize, anchor: &mut usize, s: &str| {
                delete_selection(chars, caret, anchor);
                for c in s.chars().filter(|c| *c == '\n' || !c.is_control()) {
                    chars.insert(*caret, c);
                    *caret += 1;
                }
                *anchor = *caret;
            };
            if let Some(text) = &edit.paste {
                insert(&mut chars, &mut caret, &mut anchor, text);
            }
            if !edit.typed.is_empty() {
                insert(&mut chars, &mut caret, &mut anchor, &edit.typed);
            }
            if edit.enter {
                insert(&mut chars, &mut caret, &mut anchor, "\n");
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
            changed = chars != before;
            *buffer = chars.iter().collect();
            ui.set_caret_sel(caret, anchor);
        }

        let chars: Vec<char> = buffer.chars().collect();
        let rows = layout(&chars, face, px, inner_w);
        let (caret, anchor) = if focused {
            let (c, a) = ui.caret_sel();
            (c.min(chars.len()), a.min(chars.len()))
        } else {
            (0, 0)
        };
        let (lo, hi) = (caret.min(anchor), caret.max(anchor));
        for (r, row) in rows.iter().enumerate() {
            let y = rect.y + PAD + r as f32 * lh;
            if focused && lo < hi && lo <= row.end && hi >= row.start {
                let x0 = x_in_row(&chars, row, lo.max(row.start), face, px);
                let x1 = x_in_row(&chars, row, hi.min(row.end), face, px);
                ui.surface(
                    Rect::new(rect.x + PAD + x0, y, (x1 - x0).max(2.0), lh),
                    Color::SLATE_600,
                );
            }
            let text: String = chars[row.start..row.end].iter().collect();
            ui.glyphs_font(rect.x + PAD, y, face, px, &text, Color::GRAY_200, 0.0);
        }
        if focused && ui.caret_on() {
            let r = row_of(&rows, caret);
            let x = x_in_row(&chars, &rows[r], caret, face, px);
            let y = rect.y + PAD + r as f32 * lh;
            ui.surface(Rect::new(rect.x + PAD + x, y, 2.0, lh), Color::INDIGO_50);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn newlines_split_rows() {
        let c = text("ab\ncd\n");
        let rows = layout(&c, font::Font::Sans, 14.0, 1000.0);
        assert_eq!(rows.len(), 3);
        assert_eq!((rows[1].start, rows[1].end), (3, 5));
        assert_eq!((rows[2].start, rows[2].end), (6, 6));
    }

    #[test]
    fn wraps_at_word_boundary() {
        let c = text("hello world again");
        let w = font::advance(font::Font::Sans, 14.0, 'h') * 8.0;
        let rows = layout(&c, font::Font::Sans, 14.0, w);
        assert!(rows.len() >= 2);
        assert_eq!(rows[1].start, 6);
    }

    #[test]
    fn caret_round_trips_through_x() {
        let c = text("abc def");
        let rows = layout(&c, font::Font::Sans, 14.0, 1000.0);
        let x = x_in_row(&c, &rows[0], 4, font::Font::Sans, 14.0);
        assert_eq!(idx_at_x(&c, &rows[0], x, font::Font::Sans, 14.0), 4);
    }
}
