use crate::color::Color;
use crate::ui::style::Style;

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub button_bg: Color,
    pub button_fg: Color,
    pub button_hover: Color,
    pub button_active: Color,
    pub disabled_bg: Color,
    pub disabled_fg: Color,
    pub accent: Color,
    pub field_bg: Color,
    pub mark: Color,
    pub label: Color,
    pub value: Color,
    pub track_off: Color,
    pub track_hover: Color,
    pub panel_bg: Color,
    pub panel_border: Color,
    pub chip_bg: Color,
    pub chip_hover: Color,
    pub chip_fg: Color,
    pub danger: Color,
    pub selected: Color,
    pub tooltip_bg: Color,
    pub font_size: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            button_bg: Color::GRAY_800,
            button_fg: Color::WHITE,
            button_hover: Color::GRAY_700,
            button_active: Color::ORANGE_300,
            disabled_bg: Color::GRAY_900,
            disabled_fg: Color::GRAY_500,
            accent: Color::GRAY_400,
            field_bg: Color::GRAY_950,
            mark: Color::GRAY_200,
            label: Color::GRAY_200,
            value: Color::ORANGE_300,
            track_off: Color::GRAY_950,
            track_hover: Color::GRAY_700,
            panel_bg: Color::GRAY_900,
            panel_border: Color::GRAY_950,
            chip_bg: Color::GRAY_800,
            chip_hover: Color::GRAY_700,
            chip_fg: Color::WHITE,
            danger: Color::RED_400,
            selected: Color::ORANGE_300,
            tooltip_bg: Color::GRAY_950.with_alpha(0.95),
            font_size: 16.0,
        }
    }
}

impl Theme {
    pub fn button(&self, enabled: bool) -> Style {
        let (bg, fg) = if enabled {
            (self.button_bg, self.button_fg)
        } else {
            (self.disabled_bg, self.disabled_fg)
        };
        Style {
            bg,
            fg,
            font_size: self.font_size,
            hover_color: Some(if enabled { self.button_hover } else { bg }),
            active_color: Some(self.button_active),
            ..Style::DEFAULT_BUTTON
        }
    }
}
