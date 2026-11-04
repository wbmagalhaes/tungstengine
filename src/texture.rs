use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::ecs::Resource;

#[derive(Clone)]
pub struct Texture2D {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Texture2D {
    pub fn from_png(bytes: &[u8]) -> Option<Texture2D> {
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().ok()?;
        let mut buf = vec![0u8; reader.output_buffer_size()?];
        let info = reader.next_frame(&mut buf).ok()?;
        let channels = info.color_type.samples();
        let pixels = (info.width * info.height) as usize;
        let mut rgba = Vec::with_capacity(pixels * 4);
        for p in 0..pixels {
            let b = p * channels;
            let (r, g, bl, a) = match channels {
                1 => (buf[b], buf[b], buf[b], 255),
                2 => (buf[b], buf[b], buf[b], buf[b + 1]),
                3 => (buf[b], buf[b + 1], buf[b + 2], 255),
                _ => (buf[b], buf[b + 1], buf[b + 2], buf[b + 3]),
            };
            rgba.extend_from_slice(&[r, g, bl, a]);
        }
        Some(Texture2D {
            width: info.width,
            height: info.height,
            rgba,
        })
    }

    pub fn load(path: &Path) -> Option<Texture2D> {
        Self::from_png(&fs::read(path).ok()?)
    }

    pub fn from_file(path: &Path) -> Option<Texture2D> {
        let img = image::open(path).ok()?.to_rgba8();
        Some(Texture2D {
            width: img.width(),
            height: img.height(),
            rgba: img.into_raw(),
        })
    }
}

#[derive(Resource, Default)]
pub struct Textures {
    map: HashMap<String, Texture2D>,
}

impl Textures {
    pub fn get(&self, name: &str) -> Option<&Texture2D> {
        self.map.get(name)
    }

    pub fn insert(&mut self, name: &str, texture: Texture2D) {
        self.map.insert(name.to_string(), texture);
    }

    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.map.keys().cloned().collect();
        v.sort();
        v
    }
}

pub fn load_textures(dir: &Path) -> Textures {
    let mut map = HashMap::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("png") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            match Texture2D::load(&path) {
                Some(texture) => {
                    map.insert(stem.to_string(), texture);
                }
                None => log::error!("texture `{}`: failed to load", path.display()),
            }
        }
    }
    Textures { map }
}

pub mod prelude {
    pub use super::{Texture2D, Textures, load_textures};
}
