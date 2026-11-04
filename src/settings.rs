use serde::{Deserialize, Serialize};

use crate::audio::Volume;
use crate::storage;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct GameSettings {
    pub sfx: f32,
    pub bgm: f32,
    pub fullscreen: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        GameSettings {
            sfx: 1.0,
            bgm: 1.0,
            fullscreen: true,
        }
    }
}

impl GameSettings {
    pub fn new(volume: Volume, fullscreen: bool) -> Self {
        GameSettings {
            sfx: volume.sfx,
            bgm: volume.bgm,
            fullscreen,
        }
    }

    pub fn volume(&self) -> Volume {
        Volume {
            sfx: self.sfx,
            bgm: self.bgm,
        }
    }

    pub fn parse(text: &str) -> GameSettings {
        let mut s: GameSettings = toml::from_str(text).unwrap_or_default();
        s.sfx = s.sfx.clamp(0.0, 1.0);
        s.bgm = s.bgm.clamp(0.0, 1.0);
        s
    }

    pub fn load(key: &str) -> GameSettings {
        storage::read(key).map_or_else(GameSettings::default, |t| GameSettings::parse(&t))
    }

    pub fn save(&self, key: &str) -> Result<(), String> {
        let text = toml::to_string(self).map_err(|e| e.to_string())?;
        storage::write(key, &text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_partial_files_and_clamps() {
        let s = GameSettings::parse("sfx = 2.0\nfullscreen = false\n");
        assert_eq!(s.sfx, 1.0);
        assert_eq!(s.bgm, 1.0);
        assert!(!s.fullscreen);
    }

    #[test]
    fn reads_integer_volumes_from_older_files() {
        let s = GameSettings::parse("sfx = 0\nbgm = 0\nfullscreen = false\n");
        assert_eq!((s.sfx, s.bgm, s.fullscreen), (0.0, 0.0, false));
    }

    #[test]
    fn broken_file_falls_back_to_defaults() {
        assert_eq!(GameSettings::parse("not toml ="), GameSettings::default());
    }
}
