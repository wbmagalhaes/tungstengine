use crate::ui::font;
use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::style::{Style, Styled};
use crate::ui::widget::Widget;

pub struct Text<'a> {
    x: f32,
    y: f32,
    text: &'a str,
    center: Option<Rect>,
    style: Style,
    truncate: Option<f32>,
    ellipsis: &'a str,
    clip: bool,
    wrapped: Option<f32>,
}

impl<'a> Text<'a> {
    pub fn new(x: f32, y: f32, text: &'a str) -> Self {
        Text {
            x,
            y,
            text,
            center: None,
            style: Style::default(),
            truncate: None,
            ellipsis: "…",
            clip: false,
            wrapped: None,
        }
    }

    pub fn centered(rect: Rect, text: &'a str) -> Self {
        Text::new(rect.x, rect.y, text).center(rect)
    }

    pub fn center(mut self, rect: Rect) -> Self {
        self.center = Some(rect);
        self
    }

    pub fn truncate(mut self, max_w: f32) -> Self {
        self.truncate = Some(max_w);
        self
    }

    pub fn ellipsis(mut self, marker: &'a str) -> Self {
        self.ellipsis = marker;
        self.clip = false;
        self
    }

    pub fn clip(mut self) -> Self {
        self.clip = true;
        self
    }

    pub fn wrapped(mut self, max_w: f32) -> Self {
        self.wrapped = Some(max_w);
        self
    }
}

impl Styled for Text<'_> {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl Widget for Text<'_> {
    type Out = ();

    fn render(self, ui: &mut UI) {
        let s = self.style.computed(ui.theme.text());
        let o = s.opacity;
        let face = s.font;
        let fs = s.font_size;
        let fg = s.fg.with_alpha(s.fg.a * o);

        if let Some(max_w) = self.wrapped {
            let mut lines = ui.take_wrap_scratch();
            ui.wrap_into(face, fs, self.text, max_w, &mut lines);
            let lh = font::line_height(face, fs);
            let (bx, by) = match self.center {
                Some(r) => {
                    let a = r.inner();
                    let total_h = lines.len().max(1) as f32 * lh;
                    (a.x, a.y + (a.h - total_h) / 2.0)
                }
                None => (self.x, self.y),
            };
            if s.bg.a > 0.0 {
                let h = lines.len().max(1) as f32 * lh;
                ui.surface(
                    Rect::new(bx, by, max_w, h).with_z(s.z),
                    s.bg.with_alpha(s.bg.a * o),
                );
            }
            for (i, &(a, b)) in lines.iter().enumerate() {
                let line = &self.text[a..b];
                let lw = ui.text_width_font(face, fs, line);
                let lx = match self.center {
                    Some(r) => r.inner().x + (max_w - lw) / 2.0,
                    None => bx,
                };
                ui.glyphs_font(lx, by + i as f32 * lh, face, fs, line, fg, s.z);
            }
            ui.put_wrap_scratch(lines);
            return;
        }

        let full_w = ui.text_width_font(face, fs, self.text);
        let trimmed: Option<String> = match self.truncate {
            Some(max_w) if !self.clip && full_w > max_w => {
                let budget = (max_w - ui.text_width_font(face, fs, self.ellipsis)).max(0.0);
                let mut acc = 0.0;
                let mut end = 0;
                for (i, ch) in self.text.char_indices() {
                    let adv = font::advance(face, fs, ch);
                    if acc + adv > budget {
                        break;
                    }
                    acc += adv;
                    end = i + ch.len_utf8();
                }
                Some(format!("{}{}", &self.text[..end], self.ellipsis))
            }
            _ => None,
        };
        let draw_text = trimmed.as_deref().unwrap_or(self.text);
        let w = match self.truncate {
            Some(max_w) => full_w.min(max_w),
            None => full_w,
        };

        let (x, y) = match self.center {
            Some(r) => {
                let a = r.inner();
                (a.x + (a.w - w) / 2.0, a.y + (a.h - fs) / 2.0)
            }
            None => (self.x, self.y),
        };
        if s.bg.a > 0.0 {
            ui.surface(
                Rect::new(x, y, w, fs).with_z(s.z),
                s.bg.with_alpha(s.bg.a * o),
            );
        }

        let clip_overflow = self.clip && self.truncate.is_some_and(|mw| full_w > mw);
        if clip_overflow {
            let box_rect = Rect::new(x, y, w, fs).with_z(s.z);
            ui.clip(box_rect, |ui| {
                ui.glyphs_font(x, y, face, fs, draw_text, fg, s.z);
            });
        } else {
            ui.glyphs_font(x, y, face, fs, draw_text, fg, s.z);
        }
    }
}
