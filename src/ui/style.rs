use crate::color::Color;
use crate::ui::font::Font;

#[derive(Clone, Copy)]
pub struct Style {
    pub bg: Color,
    pub fg: Color,
    pub font: Font,
    pub font_size: f32,
    pub z: f32,
    pub opacity: f32,
    pub gap: f32,
    pub pad: f32,
    pub hover_color: Option<Color>,
    pub hover_scale: f32,
    pub active_color: Option<Color>,
    pub active_scale: f32,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            bg: Color::GRAY_800,
            fg: Color::INDIGO_50,
            font: Font::Sans,
            font_size: 16.0,
            z: 0.0,
            opacity: 1.0,
            gap: 0.0,
            pad: 0.0,
            hover_color: None,
            hover_scale: 1.0,
            active_color: None,
            active_scale: 1.0,
        }
    }
}

impl Style {
    pub const DEFAULT_BUTTON: Style = Style {
        bg: Color::GRAY_800,
        fg: Color::INDIGO_50,
        font: Font::Sans,
        font_size: 24.0,
        z: 0.0,
        opacity: 1.0,
        gap: 0.0,
        pad: 0.0,
        hover_color: Some(Color::GRAY_700),
        hover_scale: 1.06,
        active_color: Some(Color::ORANGE_300),
        active_scale: 0.96,
    };

    pub const DEFAULT_TEXT: Style = Style {
        bg: Color::CLEAR,
        fg: Color::TEXT_DEFAULT,
        font: Font::Sans,
        font_size: 16.0,
        z: 0.0,
        opacity: 1.0,
        gap: 0.0,
        pad: 0.0,
        hover_color: None,
        hover_scale: 1.0,
        active_color: None,
        active_scale: 1.0,
    };

    pub const PANEL: Style = Style {
        bg: Color::GRAY_900,
        fg: Color::TEXT_DEFAULT,
        font: Font::Sans,
        font_size: 16.0,
        z: 0.0,
        opacity: 0.92,
        gap: 8.0,
        pad: 12.0,
        hover_color: None,
        hover_scale: 1.0,
        active_color: None,
        active_scale: 1.0,
    };
}

#[derive(Clone, Copy, Default)]
pub struct StylePatch {
    pub bg: Option<Color>,
    pub fg: Option<Color>,
    pub font: Option<Font>,
    pub font_size: Option<f32>,
    pub z: Option<f32>,
    pub opacity: Option<f32>,
    pub gap: Option<f32>,
    pub pad: Option<f32>,
    pub hover_color: Option<Color>,
    pub hover_scale: Option<f32>,
    pub active_color: Option<Color>,
    pub active_scale: Option<f32>,
}

impl Style {
    pub fn with(mut self, p: StylePatch) -> Style {
        self.bg = p.bg.unwrap_or(self.bg);
        self.fg = p.fg.unwrap_or(self.fg);
        self.font = p.font.unwrap_or(self.font);
        self.font_size = p.font_size.unwrap_or(self.font_size);
        self.z = p.z.unwrap_or(self.z);
        self.opacity = p.opacity.unwrap_or(self.opacity);
        self.gap = p.gap.unwrap_or(self.gap);
        self.pad = p.pad.unwrap_or(self.pad);
        self.hover_color = p.hover_color.or(self.hover_color);
        self.hover_scale = p.hover_scale.unwrap_or(self.hover_scale);
        self.active_color = p.active_color.or(self.active_color);
        self.active_scale = p.active_scale.unwrap_or(self.active_scale);
        self
    }
}

pub trait Styled: Sized {
    fn patch_mut(&mut self) -> &mut StylePatch;

    fn style(mut self, s: Style) -> Self {
        *self.patch_mut() = StylePatch {
            bg: Some(s.bg),
            fg: Some(s.fg),
            font: Some(s.font),
            font_size: Some(s.font_size),
            z: Some(s.z),
            opacity: Some(s.opacity),
            gap: Some(s.gap),
            pad: Some(s.pad),
            hover_color: s.hover_color,
            hover_scale: Some(s.hover_scale),
            active_color: s.active_color,
            active_scale: Some(s.active_scale),
        };
        self
    }
    fn fg(mut self, color: Color) -> Self {
        self.patch_mut().fg = Some(color);
        self
    }
    fn bg(mut self, color: Color) -> Self {
        self.patch_mut().bg = Some(color);
        self
    }
    fn font(mut self, font: Font) -> Self {
        self.patch_mut().font = Some(font);
        self
    }
    fn mono(self) -> Self {
        self.font(Font::Mono)
    }
    fn font_size(mut self, px: f32) -> Self {
        self.patch_mut().font_size = Some(px);
        self
    }
    fn z(mut self, z: f32) -> Self {
        self.patch_mut().z = Some(z);
        self
    }
    fn gap(mut self, gap: f32) -> Self {
        self.patch_mut().gap = Some(gap);
        self
    }
    fn pad(mut self, pad: f32) -> Self {
        self.patch_mut().pad = Some(pad);
        self
    }
    fn opacity(mut self, opacity: f32) -> Self {
        self.patch_mut().opacity = Some(opacity);
        self
    }
    fn hover_color(mut self, color: Color) -> Self {
        self.patch_mut().hover_color = Some(color);
        self
    }
    fn hover_scale(mut self, scale: f32) -> Self {
        self.patch_mut().hover_scale = Some(scale);
        self
    }
    fn active_color(mut self, color: Color) -> Self {
        self.patch_mut().active_color = Some(color);
        self
    }
    fn active_scale(mut self, scale: f32) -> Self {
        self.patch_mut().active_scale = Some(scale);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Theme;

    #[test]
    fn patch_overrides_only_the_fields_set() {
        let theme = Theme::default();
        let patch = StylePatch {
            bg: Some(Color::RED_400),
            z: Some(3.0),
            ..Default::default()
        };
        let s = theme.button(true).with(patch);
        assert_eq!(s.bg, Color::RED_400);
        assert_eq!(s.z, 3.0);
        assert_eq!(s.fg, theme.button_fg);
        assert_eq!(s.font_size, theme.font_size);
        assert_eq!(s.hover_color, Some(theme.button_hover));
    }
}
