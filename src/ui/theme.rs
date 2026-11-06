use crate::color::Color;
use crate::ui::style::{ComputedStyle, Style, StyleStates};

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
    pub border_radius: f32,
    pub gap: f32,
    pub pad: f32,
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
            border_radius: 0.0,
            gap: 8.0,
            pad: 12.0,
        }
    }
}

impl Theme {
    pub fn base(&self) -> ComputedStyle {
        ComputedStyle {
            border_radius: [self.border_radius; 4],
            ..ComputedStyle::BASE
        }
    }

    pub fn text(&self) -> ComputedStyle {
        ComputedStyle {
            border_radius: [self.border_radius; 4],
            ..ComputedStyle::TEXT
        }
    }

    pub fn panel(&self) -> ComputedStyle {
        ComputedStyle {
            border_radius: [self.border_radius; 4],
            gap: (self.gap, self.gap),
            pad: [self.pad; 4],
            ..ComputedStyle::PANEL
        }
    }

    pub fn button(&self, enabled: bool) -> StyleStates {
        let (bg, fg) = if enabled {
            (self.button_bg, self.button_fg)
        } else {
            (self.disabled_bg, self.disabled_fg)
        };
        StyleStates {
            normal: ComputedStyle {
                bg,
                fg,
                font_size: self.font_size,
                border_radius: [self.border_radius; 4],
                ..ComputedStyle::BUTTON
            },
            hover: Style {
                bg: Some(if enabled { self.button_hover } else { bg }),
                ..Style::default()
            },
            active: Style {
                bg: Some(self.button_active),
                ..Style::default()
            },
        }
    }
}
