use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::ecs::prelude::*;

#[derive(Deserialize)]
pub struct Rule {
    pub item: String,
    pub target: String,
    #[serde(default)]
    pub consume: bool,
    #[serde(default)]
    pub say: Vec<String>,
    #[serde(default)]
    pub set: Option<String>,
    #[serde(default)]
    pub give: Option<String>,
    #[serde(default)]
    pub dialog: Option<String>,
    #[serde(default)]
    pub far: bool,
}

#[derive(Deserialize)]
pub struct Combo {
    pub a: String,
    pub b: String,
    pub result: String,
    #[serde(default)]
    pub say: Vec<String>,
}

#[derive(Resource, Default, Deserialize)]
pub struct Rules {
    #[serde(default, rename = "rule")]
    list: Vec<Rule>,
    #[serde(default, rename = "combine")]
    combos: Vec<Combo>,
}

impl Rules {
    pub fn find(&self, item: &str, target: &str) -> Option<&Rule> {
        self.list
            .iter()
            .find(|r| r.item == item && r.target == target)
    }

    pub fn find_combo(&self, x: &str, y: &str) -> Option<&Combo> {
        self.combos
            .iter()
            .find(|c| (c.a == x && c.b == y) || (c.a == y && c.b == x))
    }
}

pub fn load_rules(path: &Path) -> Rules {
    fs::read_to_string(path)
        .ok()
        .and_then(|t| toml::from_str::<Rules>(&t).ok())
        .unwrap_or_default()
}

pub mod prelude {
    pub use super::{Rules, load_rules};
}
