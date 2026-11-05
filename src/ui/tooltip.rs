use crate::color::Color;
use crate::ui::{Rect, SpriteDraw, Styled, Text, UI, Widget};

pub const Z_TOOLTIP: f32 = 12.0;

#[derive(Clone, Copy)]
pub enum TipAt {
    Cursor,
    Above(f32, f32),
    Right(f32, f32),
}

pub type TipPart = (Option<SpriteDraw>, String);

pub struct Tip<'a> {
    pub lines: Vec<(&'a str, Color)>,
    pub accent: Option<Color>,
    pub icon: Option<SpriteDraw>,
    pub struck: Vec<usize>,
    pub rich: Vec<(usize, Vec<TipPart>)>,
    pub at: TipAt,
}

impl<'a> Tip<'a> {
    pub fn new(text: &'a str, color: Color) -> Self {
        Tip {
            lines: vec![(text, color)],
            accent: None,
            icon: None,
            struck: Vec::new(),
            rich: Vec::new(),
            at: TipAt::Cursor,
        }
    }

    pub fn at(mut self, at: TipAt) -> Self {
        self.at = at;
        self
    }

    pub fn struck_line(mut self, text: &'a str, color: Color) -> Self {
        self.struck.push(self.lines.len());
        self.lines.push((text, color));
        self
    }

    pub fn line(mut self, text: &'a str, color: Color) -> Self {
        self.lines.push((text, color));
        self
    }

    pub fn rich_line(mut self, parts: Vec<TipPart>, color: Color) -> Self {
        self.rich.push((self.lines.len(), parts));
        self.lines.push(("", color));
        self
    }

    pub fn accent(mut self, color: Color) -> Self {
        self.accent = Some(color);
        self
    }

    pub fn icon(mut self, sprite: SpriteDraw) -> Self {
        self.icon = Some(sprite);
        self
    }
}

impl Widget for Tip<'_> {
    type Out = ();

    fn render(self, ui: &mut UI) {
        let tip = self;
        let at = tip.at;
        let size = |i: usize| if i == 0 { 14.0 } else { 13.0 };
        let line_h = 20.0;
        let icon_w = if tip.icon.is_some() { 28.0 } else { 0.0 };
        let icon_size = 16.0;
        let part_w = |ui: &UI, i: usize, parts: &[TipPart]| {
            parts
                .iter()
                .map(|(icon, t)| {
                    icon.map_or(0.0, |_| icon_size + 3.0) + ui.text_width(size(i), t) + 8.0
                })
                .sum::<f32>()
        };
        let text_w = tip
            .lines
            .iter()
            .enumerate()
            .map(|(i, (t, _))| match tip.rich.iter().find(|r| r.0 == i) {
                Some((_, parts)) => part_w(ui, i, parts),
                None => ui.text_width(size(i), t),
            })
            .fold(0.0, f32::max);
        let w = text_w + icon_w + 20.0;
        let h = tip.lines.len() as f32 * line_h + 10.0;
        let (mut x, mut y) = match at {
            TipAt::Cursor => (ui.cursor.0 + 16.0, ui.cursor.1 + 14.0),
            TipAt::Above(cx, bottom) => (cx - w / 2.0, bottom - h),
            TipAt::Right(left, cy) => (left, cy - h / 2.0),
        };
        if x + w > ui.width - 4.0 {
            x = match at {
                TipAt::Cursor => ui.cursor.0 - w - 12.0,
                _ => ui.width - 4.0 - w,
            };
        }
        if y + h > ui.height - 4.0 {
            y = match at {
                TipAt::Cursor => ui.cursor.1 - h - 8.0,
                _ => ui.height - 4.0 - h,
            };
        }
        let r = Rect::new(x.max(4.0), y.max(4.0), w, h).with_z(Z_TOOLTIP);
        ui.fill(r, ui.theme.tooltip_bg);
        if let Some(a) = tip.accent {
            ui.fill(Rect::new(r.x, r.y, r.w, 2.0).with_z(r.z + 0.001), a);
        }
        if let Some(sprite) = tip.icon {
            ui.sprite(
                Rect::new(r.x + 8.0, r.y + (h - 22.0) / 2.0, 22.0, 22.0).with_z(r.z + 0.001),
                sprite,
                Color::WHITE,
            );
        }
        for (i, (text, color)) in tip.lines.iter().enumerate() {
            let (tx, ty) = (r.x + 10.0 + icon_w, r.y + 6.0 + i as f32 * line_h);
            if let Some((_, parts)) = tip.rich.iter().find(|r| r.0 == i) {
                let mut x = tx;
                for (icon, t) in parts {
                    if let Some(sprite) = icon {
                        ui.sprite(
                            Rect::new(x, ty, icon_size, icon_size).with_z(r.z + 0.002),
                            *sprite,
                            Color::WHITE,
                        );
                        x += icon_size + 3.0;
                    }
                    ui.add(
                        Text::new(x, ty, t)
                            .fg(*color)
                            .font_size(size(i))
                            .z(r.z + 0.002),
                    );
                    x += ui.text_width(size(i), t) + 8.0;
                }
                continue;
            }
            ui.add(
                Text::new(tx, ty, text)
                    .fg(*color)
                    .font_size(size(i))
                    .z(r.z + 0.002),
            );
            if tip.struck.contains(&i) {
                let tw = ui.text_width(size(i), text);
                ui.fill(
                    Rect::new(tx, ty + size(i) * 0.55, tw, 1.5).with_z(r.z + 0.003),
                    color.with_alpha(0.9),
                );
            }
        }
    }
}
