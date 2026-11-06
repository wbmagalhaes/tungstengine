use super::font;
use super::rect::Rect;

#[derive(Clone, Copy)]
pub struct DrawRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub color: [f32; 4],
    pub z: f32,
    pub tex: u32,
    pub clip: [f32; 4],
    pub quad: Option<[[f32; 2]; 4]>,
    pub uvs: Option<[[f32; 2]; 4]>,
    pub silhouette: bool,
    pub clip_round: Option<ClipRound>,
    pub round: Option<RoundShape>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipRound {
    pub center: [f32; 2],
    pub half: [f32; 2],
    pub radii: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundGradient {
    pub via: [f32; 4],
    pub to: [f32; 4],
    pub angle: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundShape {
    pub center: [f32; 2],
    pub half: [f32; 2],
    pub radii: [f32; 4],
    pub rotation: f32,
    pub sweep: Option<(f32, [f32; 4])>,
    pub blur: f32,
    pub gradient: Option<RoundGradient>,
    pub border: f32,
    pub border_color: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteDraw {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub tex: u32,
    pub px: [u16; 2],
}

impl SpriteDraw {
    pub fn image(i: u32, uv_min: [f32; 2], uv_max: [f32; 2], px: [u16; 2]) -> Self {
        SpriteDraw {
            uv_min,
            uv_max,
            tex: i + 1,
            px,
        }
    }

    pub fn flip_x(mut self) -> Self {
        std::mem::swap(&mut self.uv_min[0], &mut self.uv_max[0]);
        self
    }

    pub fn pixel_rect(&self, r: Rect) -> Rect {
        let [pw, ph] = self.px;
        if pw == 0 || ph == 0 {
            return r;
        }
        let (pw, ph) = (pw as f32, ph as f32);
        let fit = (r.w / pw).min(r.h / ph);
        let k = if fit >= 1.0 && fit.round() <= fit {
            fit.round()
        } else {
            fit
        };
        let (w, h) = (pw * k, ph * k);
        Rect::new(
            (r.x + (r.w - w) / 2.0).round(),
            (r.y + (r.h - h) / 2.0).round(),
            w,
            h,
        )
        .with_z(r.z)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Sprite(pub u32);

impl Sprite {
    pub const RECT: Sprite = Sprite(0);
    pub const CIRCLE: Sprite = Sprite(1);
    pub const TRIANGLE: Sprite = Sprite(2);
    pub const HEXAGON: Sprite = Sprite(3);
    pub const CAPSULE: Sprite = Sprite(4);
    pub const SOFT: Sprite = Sprite(5);

    pub fn from_name(name: &str) -> Option<Sprite> {
        match name {
            "rect" => Some(Sprite::RECT),
            "circle" => Some(Sprite::CIRCLE),
            "triangle" => Some(Sprite::TRIANGLE),
            "hexagon" => Some(Sprite::HEXAGON),
            "capsule" => Some(Sprite::CAPSULE),
            "soft" => Some(Sprite::SOFT),
            _ => None,
        }
    }

    pub fn builtin_name(self) -> Option<&'static str> {
        ["rect", "circle", "triangle", "hexagon", "capsule", "soft"]
            .get(self.0 as usize)
            .copied()
    }

    pub fn draw(self) -> SpriteDraw {
        let (uv_min, uv_max) = font::sprite_uv(self.0 as usize);
        SpriteDraw {
            uv_min,
            uv_max,
            tex: 0,
            px: [0, 0],
        }
    }
}
