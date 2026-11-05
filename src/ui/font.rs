use std::cell::RefCell;
use std::sync::{Mutex, MutexGuard, OnceLock};

use fontdue::FontSettings;

use crate::ecs::FastMap;

pub const ATLAS_W: usize = 2048;
pub const ATLAS_H: usize = 2048;
const METRIC_PX: f32 = 32.0;
const SPRITE_PX: usize = 256;
pub const SPRITE_COUNT: usize = 6;
const CAPSULE_RADIUS: f32 = 1.0 / 3.0;
const MAX_PX: f32 = 256.0;

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Font {
    #[default]
    Sans,
    Mono,
}

#[derive(Clone, Copy)]
pub struct Glyph {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub size: [f32; 2],
    pub bearing: [f32; 2],
    pub advance: f32,
}

struct Face {
    font: fontdue::Font,
    ascent: f32,
    cap: f32,
    line: f32,
    default_advance: f32,
}

struct Atlas {
    pixels: Vec<u8>,
    pen_x: usize,
    pen_y: usize,
    row_h: usize,
    reserved_h: usize,
    glyphs: FastMap<(Font, char, u16), Option<Glyph>>,
    dirty: Option<(usize, usize)>,
}

struct FontData {
    faces: [Face; 2],
    sprites: Vec<([f32; 2], [f32; 2])>,
    atlas: Mutex<Atlas>,
}

fn shape_alpha(shape: usize, u: f32, v: f32) -> f32 {
    let p = glam::Vec2::new(u * 2.0 - 1.0, v * 2.0 - 1.0);
    let a = match shape {
        0 => 1.0,
        1 => (1.0 - p.length()) / 0.04,
        2 => {
            let t = (p.y + 0.72) / 1.34;
            let inside = p.y <= 0.62 && p.y >= -0.72 && p.x.abs() <= (1.0 - t) * 0.9;
            if inside { 1.0 } else { 0.0 }
        }
        3 => {
            let (x, y) = (p.x.abs(), p.y.abs());
            if x <= 0.86 && y <= 1.0 && y <= -1.732 * x + 1.72 {
                1.0
            } else {
                0.0
            }
        }
        4 => {
            let reach = 1.0 - CAPSULE_RADIUS;
            let spine = glam::Vec2::new(p.x.clamp(-reach, reach), 0.0);
            (CAPSULE_RADIUS - (p - spine).length()) / 0.04
        }
        5 => {
            let falloff = (1.0 - p.length()).max(0.0);
            falloff * falloff
        }
        _ => 0.0,
    };
    a.clamp(0.0, 1.0)
}

#[derive(Default)]
struct FontConfig {
    sans: Option<String>,
    mono: Option<String>,
}

static CONFIG: OnceLock<FontConfig> = OnceLock::new();
static DATA: OnceLock<FontData> = OnceLock::new();

pub fn configure_fonts(sans: Option<String>, mono: Option<String>) {
    if DATA.get().is_some() {
        log::warn!("font::configure called after the atlas was already built; ignored");
        return;
    }
    let _ = CONFIG.set(FontConfig { sans, mono });
}

const SANS_DEFAULT: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");
const MONO_DEFAULT: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");

fn face_bytes(path: Option<&String>, default: &'static [u8]) -> Vec<u8> {
    if let Some(p) = path.filter(|p| !p.is_empty()) {
        match std::fs::read(p) {
            Ok(b) => return b,
            Err(e) => log::warn!("font `{p}` failed to load ({e}); using embedded default"),
        }
    }
    default.to_vec()
}

fn data() -> &'static FontData {
    DATA.get_or_init(build)
}

fn load_face(bytes: &[u8]) -> Face {
    let font = fontdue::Font::from_bytes(bytes, FontSettings::default()).unwrap();
    let lm = font.horizontal_line_metrics(METRIC_PX).unwrap();
    let cap = ('0'..='9')
        .chain('A'..='Z')
        .chain('a'..='z')
        .map(|ch| {
            let m = font.metrics(ch, METRIC_PX);
            (m.ymin + m.height as i32) as f32
        })
        .fold(0.0f32, f32::max);
    let default_advance = font.metrics(' ', METRIC_PX).advance_width;
    Face {
        font,
        ascent: lm.ascent / METRIC_PX,
        cap: cap / METRIC_PX,
        line: lm.new_line_size / METRIC_PX,
        default_advance: default_advance / METRIC_PX,
    }
}

impl Atlas {
    fn place(&mut self, w: usize, h: usize) -> Option<(usize, usize)> {
        if self.pen_x + w + 1 > ATLAS_W {
            self.pen_x = 0;
            self.pen_y += self.row_h + 1;
            self.row_h = 0;
        }
        if self.pen_y + h + 1 > ATLAS_H {
            return None;
        }
        let at = (self.pen_x, self.pen_y);
        self.pen_x += w + 1;
        self.row_h = self.row_h.max(h);
        Some(at)
    }

    fn mark(&mut self, y0: usize, y1: usize) {
        self.dirty = Some(match self.dirty {
            Some((a, b)) => (a.min(y0), b.max(y1)),
            None => (y0, y1),
        });
    }

    fn reset_glyphs(&mut self) {
        log::warn!("glyph atlas full; clearing glyph cache");
        self.glyphs.clear();
        self.pixels[self.reserved_h * ATLAS_W..].fill(0);
        self.pen_x = 0;
        self.pen_y = self.reserved_h;
        self.row_h = 0;
        self.mark(self.reserved_h, ATLAS_H);
    }
}

fn build() -> FontData {
    let cfg = CONFIG.get();
    let sans = load_face(&face_bytes(cfg.and_then(|c| c.sans.as_ref()), SANS_DEFAULT));
    let mono = load_face(&face_bytes(cfg.and_then(|c| c.mono.as_ref()), MONO_DEFAULT));

    let mut atlas = Atlas {
        pixels: vec![0u8; ATLAS_W * ATLAS_H],
        pen_x: 4,
        pen_y: 0,
        row_h: 2,
        reserved_h: 0,
        glyphs: FastMap::default(),
        dirty: None,
    };
    for y in 0..2 {
        for x in 0..2 {
            atlas.pixels[y * ATLAS_W + x] = 255;
        }
    }

    let mut sprites = Vec::with_capacity(SPRITE_COUNT);
    for shape in 0..SPRITE_COUNT {
        let (px, py) = atlas
            .place(SPRITE_PX, SPRITE_PX)
            .expect("sprite fits atlas");
        for gy in 0..SPRITE_PX {
            for gx in 0..SPRITE_PX {
                let mut acc = 0.0;
                for sy in 0..2 {
                    for sx in 0..2 {
                        let u = (gx as f32 + 0.25 + sx as f32 * 0.5) / SPRITE_PX as f32;
                        let v = (gy as f32 + 0.25 + sy as f32 * 0.5) / SPRITE_PX as f32;
                        acc += shape_alpha(shape, u, v);
                    }
                }
                atlas.pixels[(py + gy) * ATLAS_W + px + gx] = (acc / 4.0 * 255.0) as u8;
            }
        }
        sprites.push((
            [px as f32 / ATLAS_W as f32, py as f32 / ATLAS_H as f32],
            [
                (px + SPRITE_PX) as f32 / ATLAS_W as f32,
                (py + SPRITE_PX) as f32 / ATLAS_H as f32,
            ],
        ));
    }
    atlas.pen_x = 0;
    atlas.pen_y += atlas.row_h + 1;
    atlas.row_h = 0;
    atlas.reserved_h = atlas.pen_y;

    FontData {
        faces: [sans, mono],
        sprites,
        atlas: Mutex::new(atlas),
    }
}

fn face(font: Font) -> &'static Face {
    &data().faces[font as usize]
}

fn atlas() -> MutexGuard<'static, Atlas> {
    data().atlas.lock().unwrap_or_else(|e| e.into_inner())
}

fn size_key(px: f32) -> u16 {
    px.round().clamp(1.0, MAX_PX) as u16
}

fn rasterize(atlas: &mut Atlas, font: Font, ch: char, key: u16) -> Option<Glyph> {
    let f = &face(font).font;
    if !f.has_glyph(ch) {
        return None;
    }
    let (m, bitmap) = f.rasterize(ch, key as f32);
    let (px, py) = match atlas.place(m.width, m.height) {
        Some(at) => at,
        None => {
            atlas.reset_glyphs();
            atlas.place(m.width, m.height)?
        }
    };
    for gy in 0..m.height {
        let row = (py + gy) * ATLAS_W + px;
        atlas.pixels[row..row + m.width].copy_from_slice(&bitmap[gy * m.width..(gy + 1) * m.width]);
    }
    atlas.mark(py, py + m.height);
    Some(Glyph {
        uv_min: [px as f32 / ATLAS_W as f32, py as f32 / ATLAS_H as f32],
        uv_max: [
            (px + m.width) as f32 / ATLAS_W as f32,
            (py + m.height) as f32 / ATLAS_H as f32,
        ],
        size: [m.width as f32, m.height as f32],
        bearing: [m.xmin as f32, (m.ymin + m.height as i32) as f32],
        advance: m.advance_width,
    })
}

pub struct GlyphRun<'a> {
    atlas: MutexGuard<'a, Atlas>,
    font: Font,
    key: u16,
}

impl GlyphRun<'_> {
    pub fn get(&mut self, ch: char) -> Option<Glyph> {
        let id = (self.font, ch, self.key);
        if let Some(g) = self.atlas.glyphs.get(&id) {
            return *g;
        }
        let g = rasterize(&mut self.atlas, self.font, ch, self.key);
        self.atlas.glyphs.insert(id, g);
        g
    }
}

pub fn glyph_run(font: Font, px: f32) -> GlyphRun<'static> {
    GlyphRun {
        atlas: atlas(),
        font,
        key: size_key(px),
    }
}

thread_local! {
    static ADVANCES: RefCell<FastMap<(Font, char, u32), f32>> = RefCell::default();
}

pub fn advance(font: Font, px: f32, ch: char) -> f32 {
    ADVANCES.with_borrow_mut(|cache| {
        *cache.entry((font, ch, px.to_bits())).or_insert_with(|| {
            let f = face(font);
            if f.font.has_glyph(ch) {
                f.font.metrics(ch, px).advance_width
            } else {
                f.default_advance * px
            }
        })
    })
}

pub fn atlas_pixels() -> Vec<u8> {
    atlas().pixels.clone()
}

pub fn atlas_size() -> (usize, usize) {
    (ATLAS_W, ATLAS_H)
}

pub fn take_dirty_rows() -> Option<(usize, usize, Vec<u8>)> {
    let mut a = atlas();
    let (y0, y1) = a.dirty.take()?;
    let y1 = y1.min(ATLAS_H);
    Some((y0, y1, a.pixels[y0 * ATLAS_W..y1 * ATLAS_W].to_vec()))
}

// Distance (in pixels at the given size) from the layout top to the baseline. Uses the
// Sans leading above the cap line for every font, so different fonts share the
// same visual top instead of drifting by their metric ascent.
pub fn top_to_baseline(font: Font, px: f32) -> f32 {
    let sans = face(Font::Sans);
    ((sans.ascent - sans.cap) + face(font).cap) * px
}

pub fn line_height(font: Font, px: f32) -> f32 {
    face(font).line * px
}

pub fn sprite_uv(i: usize) -> ([f32; 2], [f32; 2]) {
    data().sprites.get(i).copied().unwrap_or_else(white_uv)
}

pub fn white_uv() -> ([f32; 2], [f32; 2]) {
    let u = 1.0 / ATLAS_W as f32;
    let v = 1.0 / ATLAS_H as f32;
    ([u, v], [u, v])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(font: Font, ch: char, px: f32) -> Option<Glyph> {
        glyph_run(font, px).get(ch)
    }

    #[test]
    fn capsule_is_a_rounded_horizontal_bar() {
        assert_eq!(shape_alpha(4, 0.5, 0.5), 1.0);
        assert_eq!(shape_alpha(4, 0.05, 0.5), 1.0);
        assert_eq!(shape_alpha(4, 0.5, 0.05), 0.0);
        assert_eq!(shape_alpha(4, 0.01, 0.35), 0.0);
    }

    #[test]
    fn rasterizes_at_requested_size() {
        let small = glyph(Font::Sans, 'H', 14.0).unwrap();
        let big = glyph(Font::Sans, 'H', 28.0).unwrap();
        assert!(small.size[1] <= 14.0 && small.size[1] >= 8.0);
        assert!(big.size[1] > small.size[1] * 1.7);
        assert_ne!(small.uv_min, big.uv_min);
    }

    #[test]
    fn caches_glyphs() {
        let a = glyph(Font::Mono, 'q', 15.0).unwrap();
        let b = glyph(Font::Mono, 'q', 15.0).unwrap();
        assert_eq!(a.uv_min, b.uv_min);
    }

    #[test]
    fn covers_symbols_beyond_ascii() {
        assert!(glyph(Font::Sans, '▸', 14.0).is_some());
        assert!(glyph(Font::Sans, '✓', 14.0).is_some());
        assert!(glyph(Font::Sans, 'ç', 14.0).is_some());
    }

    #[test]
    fn advance_scales_with_size() {
        let a = advance(Font::Sans, 10.0, 'M');
        let b = advance(Font::Sans, 20.0, 'M');
        assert!((b - a * 2.0).abs() < 0.01);
    }
}
