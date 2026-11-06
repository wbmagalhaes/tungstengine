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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortraitSide {
    #[default]
    Left,
    Right,
}

#[derive(Clone, Deserialize)]
pub struct Actor {
    #[serde(skip)]
    pub id: String,
    pub name: String,
    #[serde(default = "unknown_default")]
    pub unknown: String,
    pub voice: Voice,
    #[serde(default)]
    pub known: bool,
    #[serde(default = "default_accent")]
    pub accent: [u8; 3],
    #[serde(default)]
    pub portrait: Option<String>,
    #[serde(default)]
    pub portrait_side: PortraitSide,
}

impl Default for Actor {
    fn default() -> Self {
        Actor {
            id: String::new(),
            name: String::new(),
            unknown: unknown_default(),
            voice: Voice::default(),
            known: false,
            accent: default_accent(),
            portrait: None,
            portrait_side: PortraitSide::default(),
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

pub fn parse_actors(text: &str) -> Actors {
    match toml::from_str::<HashMap<String, Actor>>(text) {
        Ok(map) => with_ids(map),
        Err(e) => {
            log::error!("actors: {e}");
            Actors::default()
        }
    }
}

fn with_ids(mut map: HashMap<String, Actor>) -> Actors {
    for (id, actor) in &mut map {
        actor.id = id.clone();
    }
    Actors { map }
}

pub fn load_actors(path: &Path) -> Actors {
    match fs::read_to_string(path).map(|t| toml::from_str::<HashMap<String, Actor>>(&t)) {
        Ok(Ok(map)) => with_ids(map),
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
