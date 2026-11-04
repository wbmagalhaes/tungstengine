use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::ecs::prelude::*;

#[derive(Clone, Copy, Deserialize)]
pub struct Voice {
    pub pitch: f32,
    #[serde(default = "default_variation")]
    pub variation: f32,
}

impl Default for Voice {
    fn default() -> Self {
        Voice {
            pitch: 1.0,
            variation: 0.2,
        }
    }
}

fn default_variation() -> f32 {
    0.2
}

#[derive(Clone, Deserialize)]
pub struct Actor {
    pub name: String,
    #[serde(default = "unknown_default")]
    pub unknown: String,
    pub voice: Voice,
    #[serde(default)]
    pub known: bool,
    #[serde(default = "default_accent")]
    pub accent: [u8; 3],
}

impl Default for Actor {
    fn default() -> Self {
        Actor {
            name: String::new(),
            unknown: unknown_default(),
            voice: Voice::default(),
            known: false,
            accent: default_accent(),
        }
    }
}

fn unknown_default() -> String {
    super::UNKNOWN_LABEL.into()
}

fn default_accent() -> [u8; 3] {
    [144, 161, 185]
}

impl Actor {
    pub fn display_name(&self) -> &str {
        if self.known {
            &self.name
        } else {
            &self.unknown
        }
    }
}

#[derive(Resource, Default)]
pub struct Actors {
    map: HashMap<String, Actor>,
}

impl Actors {
    pub fn get(&self, id: &str) -> Option<&Actor> {
        self.map.get(id)
    }

    pub fn reveal(&mut self, id: &str) {
        if let Some(actor) = self.map.get_mut(id) {
            actor.known = true;
        }
    }
}

pub fn load_actors(path: &Path) -> Actors {
    match fs::read_to_string(path).map(|t| toml::from_str::<HashMap<String, Actor>>(&t)) {
        Ok(Ok(map)) => Actors { map },
        Ok(Err(e)) => {
            log::error!("actors `{}`: {e}", path.display());
            Actors::default()
        }
        Err(e) => {
            log::error!("actors `{}`: {e}", path.display());
            Actors::default()
        }
    }
}
