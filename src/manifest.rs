use serde::Deserialize;

use crate::color::Color;
use crate::ecs::Resource;

pub use ecs_macros::embed_game;

pub const MANIFEST_FILE: &str = "manifest.toml";
pub const SETTINGS_FILE: &str = "settings.toml";
pub const BINDINGS_FILE: &str = "bindings.toml";

#[derive(Resource, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct GameFiles {
    pub settings: String,
    pub bindings: String,
}

impl Default for GameFiles {
    fn default() -> Self {
        GameFiles {
            settings: SETTINGS_FILE.into(),
            bindings: BINDINGS_FILE.into(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct GameAssets {
    pub manifest: &'static str,
    pub locales: &'static [(&'static str, &'static str)],
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Manifest {
    pub title: String,
    pub width: f32,
    pub height: f32,
    pub locale: String,
    pub clear_color: Option<String>,
    pub assets: String,
    pub main_scene: String,
    pub transition_mask: Option<String>,
    pub files: GameFiles,
    pub menu: MenuColors,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct MenuColors {
    pub card: Option<String>,
    pub border: Option<String>,
    pub title: Option<String>,
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest {
            title: "Game".into(),
            width: 1280.0,
            height: 720.0,
            locale: "en".into(),
            clear_color: None,
            assets: "assets".into(),
            main_scene: "main".into(),
            transition_mask: None,
            files: GameFiles::default(),
            menu: MenuColors::default(),
        }
    }
}

impl Manifest {
    pub fn parse(text: &str) -> Manifest {
        toml::from_str(text).unwrap_or_else(|e| panic!("invalid manifest.toml: {e}"))
    }

    pub fn clear_color(&self) -> Option<Color> {
        self.clear_color.as_deref().map(Color::hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_can_be_remapped_one_by_one() {
        let m = Manifest::parse("[files]\nbindings = \"keys.toml\"\n");
        assert_eq!(m.files.bindings, "keys.toml");
        assert_eq!(m.files.settings, SETTINGS_FILE);
    }

    #[test]
    fn fills_defaults() {
        let m = Manifest::parse("title = \"Cat\"\n[menu]\ncard = \"#112233\"\n");
        assert_eq!(m.width, 1280.0);
        assert_eq!(m.files.settings, SETTINGS_FILE);
        assert_eq!(m.menu.card.as_deref(), Some("#112233"));
    }
}
