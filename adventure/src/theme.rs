use std::path::Path;

use serde::Deserialize;

use tungstengine::color::Color;
use tungstengine::ecs::Resource;

#[derive(Resource)]
pub struct AdventureTheme {
    pub panel_bg: Color,
    pub panel_alpha: f32,
    pub title: String,
    pub title_fg: Color,
    pub muted_fg: Color,
    pub accent_fg: Color,
    pub font_lg: f32,
    pub font_base: f32,
    pub font_sm: f32,
    pub inv_slot: f32,
    pub inv_gap: f32,
    pub inv_margin: f32,
    pub dialog_height: f32,
    pub font_sans: Option<String>,
    pub font_mono: Option<String>,
}

impl Default for AdventureTheme {
    fn default() -> Self {
        AdventureTheme {
            panel_bg: Color::GRAY_900,
            panel_alpha: 0.92,
            title: "Game".into(),
            title_fg: Color::TEXT_TITLE,
            muted_fg: Color::TEXT_MUTED,
            accent_fg: Color::hex("#F2D788"),
            font_lg: 44.0,
            font_base: 18.0,
            font_sm: 16.0,
            inv_slot: 56.0,
            inv_gap: 8.0,
            inv_margin: 16.0,
            dialog_height: 130.0,
            font_sans: None,
            font_mono: None,
        }
    }
}

impl AdventureTheme {
    pub fn panel_fill(&self) -> Color {
        self.panel_bg.with_alpha(self.panel_alpha)
    }

    pub fn load(path: &Path) -> Self {
        let base = AdventureTheme::default();
        let Ok(text) = std::fs::read_to_string(path) else {
            return base;
        };
        let def: UiThemeDef = match toml::from_str(&text) {
            Ok(d) => d,
            Err(e) => {
                log::error!("ui theme `{}`: {e}", path.display());
                return base;
            }
        };
        let color = |s: Option<String>, fallback: Color| {
            s.and_then(|s| Color::parse(&s)).unwrap_or(fallback)
        };
        AdventureTheme {
            panel_bg: color(def.panel_bg, base.panel_bg),
            panel_alpha: def.panel_alpha.unwrap_or(base.panel_alpha),
            title: def.title.unwrap_or(base.title),
            title_fg: color(def.title_fg, base.title_fg),
            muted_fg: color(def.muted_fg, base.muted_fg),
            accent_fg: color(def.accent_fg, base.accent_fg),
            font_lg: def.font_lg.unwrap_or(base.font_lg),
            font_base: def.font_base.unwrap_or(base.font_base),
            font_sm: def.font_sm.unwrap_or(base.font_sm),
            inv_slot: def.inv_slot.unwrap_or(base.inv_slot),
            inv_gap: def.inv_gap.unwrap_or(base.inv_gap),
            inv_margin: def.inv_margin.unwrap_or(base.inv_margin),
            dialog_height: def.dialog_height.unwrap_or(base.dialog_height),
            font_sans: def.font_sans,
            font_mono: def.font_mono,
        }
    }
}

#[derive(Deserialize, Default)]
struct UiThemeDef {
    panel_bg: Option<String>,
    panel_alpha: Option<f32>,
    title: Option<String>,
    title_fg: Option<String>,
    muted_fg: Option<String>,
    accent_fg: Option<String>,
    font_lg: Option<f32>,
    font_base: Option<f32>,
    font_sm: Option<f32>,
    inv_slot: Option<f32>,
    inv_gap: Option<f32>,
    inv_margin: Option<f32>,
    dialog_height: Option<f32>,
    font_sans: Option<String>,
    font_mono: Option<String>,
}
