use super::immediate::UI;
use super::rect::Rect;
use super::style::Style;

#[derive(Clone, Copy)]
pub enum Dir {
    Vertical,
    Horizontal,
}

pub struct Layout {
    dir: Dir,
    area: Rect,
    cursor: f32,
    gap: f32,
}

impl Layout {
    pub fn new(dir: Dir, area: Rect, gap: f32) -> Self {
        let cursor = match dir {
            Dir::Vertical => area.y,
            Dir::Horizontal => area.x,
        };
        Layout {
            dir,
            area,
            cursor,
            gap,
        }
    }

    pub fn next(&mut self, size: f32) -> Rect {
        let r = match self.dir {
            Dir::Vertical => Rect::new(self.area.x, self.cursor, self.area.w, size),
            Dir::Horizontal => Rect::new(self.cursor, self.area.y, size, self.area.h),
        };
        self.cursor += size + self.gap;
        Rect {
            z: self.area.z,
            ..r
        }
    }

    pub fn remaining(&self) -> f32 {
        let end = match self.dir {
            Dir::Vertical => self.area.y + self.area.h,
            Dir::Horizontal => self.area.x + self.area.w,
        };
        (end - self.cursor).max(0.0)
    }

    pub fn fill(&mut self) -> Rect {
        self.next(self.remaining())
    }

    pub fn skip(&mut self, amount: f32) {
        self.cursor += amount;
    }

    pub fn text(&mut self, ui: &UI, style: &Style, text: &str) -> Rect {
        let max_w = match self.dir {
            Dir::Vertical => self.area.w,
            Dir::Horizontal => self.remaining(),
        };
        let (_, h) = ui.measure(style.font, style.font_size, text, Some(max_w));
        self.next(h)
    }

    pub fn split<const N: usize>(&mut self, total: f32, weights: [f32; N]) -> [Rect; N] {
        let sum: f32 = weights.iter().sum();
        let inner = (total - self.gap * (N as f32 - 1.0)).max(0.0);
        weights.map(|w| self.next(inner * w / sum))
    }
}
