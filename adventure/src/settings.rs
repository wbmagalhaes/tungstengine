use std::fs;
use std::path::Path;

use serde::Deserialize;

use tungstengine::ecs::prelude::*;

#[derive(Clone, Copy, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ShadowQuality {
    Off,
    Low,
    #[default]
    Medium,
    High,
}

impl ShadowQuality {
    pub fn resolution(self) -> u32 {
        match self {
            ShadowQuality::Off => 0,
            ShadowQuality::Low => 1024,
            ShadowQuality::Medium => 2048,
            ShadowQuality::High => 4096,
        }
    }
}

#[derive(Clone, Copy, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Msaa {
    #[default]
    Off,
    X2,
    X4,
    X8,
}

impl Msaa {
    pub fn samples(self) -> u32 {
        match self {
            Msaa::Off => 1,
            Msaa::X2 => 2,
            Msaa::X4 => 4,
            Msaa::X8 => 8,
        }
    }
}

#[derive(Resource, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub text_speed: f32,
    pub ui_volume: f32,
    pub sfx_volume: f32,
    pub bgm_volume: f32,
    pub muted: bool,
    pub shadow_quality: ShadowQuality,
    pub msaa: Msaa,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            text_speed: 45.0,
            ui_volume: 0.7,
            sfx_volume: 0.7,
            bgm_volume: 0.5,
            muted: false,
            shadow_quality: ShadowQuality::default(),
            msaa: Msaa::default(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        match fs::read_to_string(path) {
            Ok(text) => match toml::from_str(&text) {
                Ok(settings) => settings,
                Err(e) => {
                    log::error!("settings `{}`: {e}", path.display());
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        }
    }
}

fn cycle_vol(v: f32) -> f32 {
    if v < 0.5 {
        0.7
    } else if v < 0.9 {
        1.0
    } else {
        0.3
    }
}

fn vol_pct(v: f32) -> u32 {
    (v * 100.0).round() as u32
}

impl Settings {
    pub fn speed_key(&self) -> &'static str {
        if self.text_speed <= 30.0 {
            "settings.speed_slow"
        } else if self.text_speed >= 70.0 {
            "settings.speed_fast"
        } else {
            "settings.speed_normal"
        }
    }

    pub fn cycle_speed(&mut self) {
        self.text_speed = if self.text_speed <= 30.0 {
            45.0
        } else if self.text_speed < 70.0 {
            80.0
        } else {
            25.0
        };
    }

    // effective per-channel gains (0 when muted)
    pub fn ui(&self) -> f32 {
        if self.muted { 0.0 } else { self.ui_volume }
    }
    pub fn sfx(&self) -> f32 {
        if self.muted { 0.0 } else { self.sfx_volume }
    }
    pub fn bgm(&self) -> f32 {
        if self.muted { 0.0 } else { self.bgm_volume }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }

    pub fn cycle_ui(&mut self) {
        self.ui_volume = cycle_vol(self.ui_volume);
    }
    pub fn cycle_sfx(&mut self) {
        self.sfx_volume = cycle_vol(self.sfx_volume);
    }
    pub fn cycle_bgm(&mut self) {
        self.bgm_volume = cycle_vol(self.bgm_volume);
    }

    pub fn ui_name(&self) -> String {
        format!("UI: {}", vol_pct(self.ui_volume))
    }
    pub fn sfx_name(&self) -> String {
        format!("SFX: {}", vol_pct(self.sfx_volume))
    }
    pub fn bgm_name(&self) -> String {
        format!("BGM: {}", vol_pct(self.bgm_volume))
    }
}
