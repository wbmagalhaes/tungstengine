#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub padding: f32,
    pub z: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect {
            x,
            y,
            w,
            h,
            padding: 0.0,
            z: 0.0,
        }
    }

    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    pub fn with_z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }

    pub fn inner(self) -> Rect {
        Rect {
            x: self.x + self.padding,
            y: self.y + self.padding,
            w: (self.w - 2.0 * self.padding).max(0.0),
            h: (self.h - 2.0 * self.padding).max(0.0),
            padding: 0.0,
            z: self.z,
        }
    }

    pub fn scaled(self, s: f32) -> Rect {
        let cx = self.x + self.w / 2.0;
        let cy = self.y + self.h / 2.0;
        let w = self.w * s;
        let h = self.h * s;
        Rect {
            x: cx - w / 2.0,
            y: cy - h / 2.0,
            w,
            h,
            padding: self.padding * s,
            z: self.z,
        }
    }

    pub fn center_x(mut self, width: f32) -> Self {
        self.x = (width - self.w) / 2.0;
        self
    }

    pub fn center_y(mut self, height: f32) -> Self {
        self.y = (height - self.h) / 2.0;
        self
    }

    pub fn center(self, width: f32, height: f32) -> Self {
        self.center_x(width).center_y(height)
    }

    pub fn center_in(self, outer: Rect) -> Self {
        Rect {
            x: outer.x + (outer.w - self.w) / 2.0,
            y: outer.y + (outer.h - self.h) / 2.0,
            ..self
        }
    }

    pub fn intersect(self, other: Rect) -> Rect {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.w).min(other.x + other.w);
        let y1 = (self.y + self.h).min(other.y + other.h);
        Rect {
            z: other.z,
            ..Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
        }
    }

    pub fn overlaps(self, o: Rect) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }

    pub fn contains(self, p: (f32, f32)) -> bool {
        p.0 >= self.x && p.0 <= self.x + self.w && p.1 >= self.y && p.1 <= self.y + self.h
    }

    pub fn min(self) -> [f32; 2] {
        [self.x, self.y]
    }

    pub fn max(self) -> [f32; 2] {
        [self.x + self.w, self.y + self.h]
    }

    pub fn split_v<const N: usize>(self, gap: f32) -> [Rect; N] {
        let h = (self.h - gap * (N as f32 - 1.0)) / N as f32;
        std::array::from_fn(|i| Rect {
            y: self.y + (h + gap) * i as f32,
            h,
            ..self
        })
    }

    pub fn split_h<const N: usize>(self, gap: f32) -> [Rect; N] {
        let w = (self.w - gap * (N as f32 - 1.0)) / N as f32;
        std::array::from_fn(|i| Rect {
            x: self.x + (w + gap) * i as f32,
            w,
            ..self
        })
    }
}
