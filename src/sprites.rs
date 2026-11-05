pub mod text;

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::ecs::FastMap;
use crate::ecs::prelude::*;
use crate::texture::Texture2D;
use crate::ui::{Sprite, SpriteDraw};

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum SpriteSpec {
    Named(String),
    Cell {
        sheet: String,
        cols: u32,
        rows: u32,
        index: u32,
    },
}

pub use ecs_macros::SpriteName;

const HASH_SEED: u64 = 0xcbf29ce484222325;
const HASH_PRIME: u64 = 0x100000001b3;

pub const fn sprite_hash(mut hash: u64, text: &str) -> u64 {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        hash = (hash ^ bytes[i] as u64).wrapping_mul(HASH_PRIME);
        i += 1;
    }
    hash
}

pub const fn sprite_id(name: &str) -> u64 {
    sprite_hash(HASH_SEED, name)
}

pub const fn sprite_id_with(prefix: &str, value: &str) -> u64 {
    sprite_hash(sprite_hash(sprite_id(prefix), "_"), value)
}

const FALLBACK: &str = "__missing";
const FALLBACK_ID: u64 = sprite_id(FALLBACK);
const FALLBACK_COLOR: [u8; 4] = [255, 0, 255, 255];

#[derive(Default)]
pub struct SpriteTable {
    draws: FastMap<u64, SpriteDraw>,
}

impl SpriteTable {
    pub fn draw(&self, key: &impl SpriteKey) -> SpriteDraw {
        self.draws
            .get(&key.id())
            .copied()
            .unwrap_or_else(|| self.missing(key.key().as_ref()))
    }

    fn missing(&self, name: &str) -> SpriteDraw {
        static WARNED: Mutex<Vec<String>> = Mutex::new(Vec::new());
        if let Ok(mut w) = WARNED.lock()
            && !w.iter().any(|n| n == name)
        {
            log::warn!("missing sprite `{name}`");
            w.push(name.to_string());
        }
        self.draws
            .get(&FALLBACK_ID)
            .copied()
            .unwrap_or_else(|| Sprite::RECT.draw())
    }
}

pub type SharedSprites = Rc<RefCell<SpriteTable>>;

pub trait SpriteKey {
    fn key(&self) -> Cow<'_, str>;

    fn id(&self) -> u64 {
        sprite_id(self.key().as_ref())
    }
}

impl SpriteKey for str {
    fn key(&self) -> Cow<'_, str> {
        Cow::Borrowed(self)
    }
}

impl SpriteKey for String {
    fn key(&self) -> Cow<'_, str> {
        Cow::Borrowed(self)
    }
}

impl<T: SpriteKey + ?Sized> SpriteKey for &T {
    fn key(&self) -> Cow<'_, str> {
        (**self).key()
    }

    fn id(&self) -> u64 {
        (**self).id()
    }
}

pub trait SpriteArg {
    fn name(&self) -> Option<String> {
        None
    }

    fn resolve(self, table: &SpriteTable) -> SpriteDraw;
}

impl SpriteArg for Sprite {
    fn name(&self) -> Option<String> {
        self.builtin_name().map(str::to_string)
    }

    fn resolve(self, _: &SpriteTable) -> SpriteDraw {
        self.draw()
    }
}

impl SpriteArg for SpriteDraw {
    fn name(&self) -> Option<String> {
        let builtin = (0..).map_while(|i| Sprite(i).builtin_name().map(|n| (Sprite(i), n)));
        builtin
            .into_iter()
            .find(|(sprite, _)| sprite.draw() == *self)
            .map(|(_, name)| name.to_string())
    }

    fn resolve(self, _: &SpriteTable) -> SpriteDraw {
        self
    }
}

impl<K: SpriteKey> SpriteArg for K {
    fn name(&self) -> Option<String> {
        Some(self.key().into_owned())
    }

    fn resolve(self, table: &SpriteTable) -> SpriteDraw {
        table.draw(&self)
    }
}

impl World {
    pub fn art(&self, sprite: impl SpriteArg) -> SpriteDraw {
        match self.resource::<Sprites>() {
            Ok(sprites) => sprite.resolve(&sprites.table.borrow()),
            Err(_) => sprite.resolve(&SpriteTable::default()),
        }
    }
}

#[derive(Resource, Clone)]
#[resource(core)]
pub struct Sprites {
    images: Vec<Texture2D>,
    index: HashMap<String, usize>,
    dirty: Vec<usize>,
    table: SharedSprites,
}

impl Default for Sprites {
    fn default() -> Self {
        let mut sprites = Sprites {
            images: Vec::new(),
            index: HashMap::new(),
            dirty: Vec::new(),
            table: SharedSprites::default(),
        };
        sprites.insert(
            FALLBACK,
            Texture2D {
                width: 1,
                height: 1,
                rgba: FALLBACK_COLOR.to_vec(),
            },
        );
        sprites
    }
}

impl Sprites {
    pub fn shared(&self) -> SharedSprites {
        self.table.clone()
    }

    fn lookup(&self, path: &str) -> Option<usize> {
        self.index.get(path).copied()
    }

    pub fn resolve(&self, spec: &SpriteSpec) -> SpriteDraw {
        match spec {
            SpriteSpec::Named(name) => {
                if let Some(sprite) = Sprite::from_name(name) {
                    return sprite.draw();
                }
                match self.lookup(name) {
                    Some(i) => SpriteDraw::image(i as u32, [0.0, 0.0], [1.0, 1.0], [0, 0]),
                    None => Sprite::RECT.draw(),
                }
            }
            SpriteSpec::Cell {
                sheet,
                cols,
                rows,
                index,
            } => match self.lookup(sheet) {
                Some(i) => {
                    let cols = (*cols).max(1);
                    let rows = (*rows).max(1);
                    let cx = (index % cols) as f32;
                    let cy = (index / cols) as f32;
                    let (cw, ch) = (1.0 / cols as f32, 1.0 / rows as f32);
                    SpriteDraw::image(
                        i as u32,
                        [cx * cw, cy * ch],
                        [(cx + 1.0) * cw, (cy + 1.0) * ch],
                        [0, 0],
                    )
                }
                None => Sprite::RECT.draw(),
            },
        }
    }

    pub fn images(&self) -> &[Texture2D] {
        &self.images
    }

    pub fn insert(&mut self, name: &str, texture: Texture2D) -> SpriteDraw {
        let i = self.images.len();
        self.images.push(texture);
        self.index.insert(name.to_string(), i);
        let draw = self.draw(i);
        self.table.borrow_mut().draws.insert(sprite_id(name), draw);
        draw
    }

    pub fn replace(&mut self, name: &str, texture: Texture2D) -> bool {
        let Some(&i) = self.index.get(name) else {
            return false;
        };
        let old = &self.images[i];
        if old.width != texture.width || old.height != texture.height {
            return false;
        }
        self.images[i] = texture;
        if !self.dirty.contains(&i) {
            self.dirty.push(i);
        }
        true
    }

    pub fn take_dirty(&mut self) -> Vec<usize> {
        std::mem::take(&mut self.dirty)
    }

    pub fn find(&self, key: impl SpriteKey) -> Option<SpriteDraw> {
        self.index.get(key.key().as_ref()).map(|&i| self.draw(i))
    }

    pub fn has(&self, key: impl SpriteKey) -> bool {
        self.index.contains_key(key.key().as_ref())
    }

    pub fn get(&self, key: impl SpriteKey) -> SpriteDraw {
        let name = key.key();
        let name = name.as_ref();
        self.find(name)
            .unwrap_or_else(|| self.table.borrow().missing(name))
    }

    fn draw(&self, i: usize) -> SpriteDraw {
        let t = &self.images[i];
        SpriteDraw::image(
            i as u32,
            [0.0, 0.0],
            [1.0, 1.0],
            [t.width as u16, t.height as u16],
        )
    }
}

pub fn load_sprites(dir: &Path) -> Sprites {
    let mut sprites = Sprites::default();
    let Ok(entries) = fs::read_dir(dir) else {
        return sprites;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "png" | "jpg" | "jpeg") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        match Texture2D::from_file(&path) {
            Some(tex) => {
                sprites.insert(stem, tex);
            }
            None => log::error!("sprite `{}`: failed to load", path.display()),
        }
    }
    sprites
}

pub mod prelude {
    pub use super::{SpriteArg, SpriteKey, SpriteName, Sprites, load_sprites};
}

#[cfg(test)]
mod key_tests {
    use super::*;
    use crate::ui::UI;

    #[derive(SpriteName, Clone, Copy)]
    enum Art<'a> {
        Chest,
        Enemy(&'a str),
    }

    fn sprites() -> Sprites {
        let tex = || Texture2D {
            width: 2,
            height: 2,
            rgba: vec![255; 16],
        };
        let mut s = Sprites::default();
        s.insert("chest", tex());
        s.insert("enemy_slime", tex());
        s
    }

    fn world(sprites: Sprites) -> World {
        let mut w = World::new();
        w.insert_resource(sprites);
        w
    }

    #[test]
    fn each_world_resolves_from_its_own_sprites() {
        let filled = world(sprites());
        let empty = world(Sprites::default());
        assert_ne!(filled.art(Art::Chest).px, [0, 0]);
        assert_eq!(empty.art(Art::Chest).px, [1, 1]);
        assert_ne!(filled.art(Art::Chest).px, [0, 0]);
    }

    #[test]
    fn missing_keys_fall_back_to_the_magenta_tile() {
        let s = sprites();
        assert_eq!(s.images()[0].rgba, FALLBACK_COLOR);
        let fallback = s.get(FALLBACK);
        let w = world(s);
        assert_eq!(w.art(Art::Enemy("nope")).tex, fallback.tex);
    }

    #[test]
    fn streamed_ids_match_the_full_name() {
        assert_eq!(sprite_id_with("enemy", "slime"), sprite_id("enemy_slime"));
        assert_eq!(Art::Enemy("slime").id(), "enemy_slime".id());
        assert_eq!(Art::Chest.id(), sprite_id("chest"));
    }

    #[test]
    fn enum_keys_with_data_resolve_by_name() {
        let s = sprites();
        assert!(s.has(Art::Chest));
        assert!(s.has(Art::Enemy("slime")));
        assert!(!s.has(Art::Enemy("bat")));
    }

    #[test]
    fn ui_draws_from_a_key() {
        let mut ui = UI::new();
        ui.set_sprites(sprites().shared());
        ui.sprite(
            crate::ui::Rect::new(0.0, 0.0, 4.0, 4.0),
            Art::Chest,
            crate::color::Color::WHITE,
        );
        assert_eq!(ui.draws.len(), 1);
    }
}
