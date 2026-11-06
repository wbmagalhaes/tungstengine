use crate::animation::Ease;
use crate::color::Color;
use crate::ui::font::Font;

#[derive(Clone, Copy, Debug)]
pub struct Transition {
    pub enter: (f32, Ease),
    pub exit: (f32, Ease),
}

impl Transition {
    pub const INSTANT: Transition = Transition::new(0.0, Ease::Linear);

    pub const fn new(seconds: f32, ease: Ease) -> Self {
        Transition {
            enter: (seconds, ease),
            exit: (seconds, ease),
        }
    }

    pub const fn exit(mut self, seconds: f32, ease: Ease) -> Self {
        self.exit = (seconds, ease);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearGradient {
    pub angle: f32,
    pub from: Color,
    pub via: Option<Color>,
    pub to: Color,
}

impl LinearGradient {
    pub fn middle(&self) -> Color {
        self.via.unwrap_or(self.from.lerp(self.to, 0.5))
    }
}

#[derive(Clone, Copy, Default)]
pub struct Style {
    pub bg: Option<Color>,
    pub fg: Option<Color>,
    pub font: Option<Font>,
    pub font_size: Option<f32>,
    pub opacity: Option<f32>,
    pub gap: Option<(f32, f32)>,
    pub pad: Option<[f32; 4]>,
    pub border_radius: Option<[f32; 4]>,
    pub border_size: Option<f32>,
    pub border_color: Option<Color>,
    pub outline_size: Option<f32>,
    pub outline_color: Option<Color>,
    pub outline_offset: Option<f32>,
    pub backdrop_blur: Option<f32>,
    pub outline_sweep: Option<Color>,
    pub outline_sweep_speed: Option<f32>,
    pub z: Option<f32>,
    pub translate: Option<(f32, f32)>,
    pub rotate: Option<f32>,
    pub scale: Option<(f32, f32)>,
    pub gradient: Option<LinearGradient>,
    pub transition: Option<Transition>,
}

#[derive(Clone, Copy)]
pub struct ComputedStyle {
    pub bg: Color,
    pub fg: Color,
    pub font: Font,
    pub font_size: f32,
    pub opacity: f32,
    pub gap: (f32, f32),
    pub pad: [f32; 4],
    pub border_radius: [f32; 4],
    pub border_size: f32,
    pub border_color: Color,
    pub outline_size: f32,
    pub outline_color: Color,
    pub outline_offset: f32,
    pub backdrop_blur: f32,
    pub outline_sweep: Color,
    pub outline_sweep_speed: f32,
    pub z: f32,
    pub translate: (f32, f32),
    pub rotate: f32,
    pub scale: (f32, f32),
    pub gradient: Option<LinearGradient>,
    pub transition: Transition,
}

#[derive(Clone, Copy)]
pub struct StyleStates {
    pub normal: ComputedStyle,
    pub hover: Style,
    pub active: Style,
}

pub fn fit_corners(radii: [f32; 4], width: f32, height: f32) -> [f32; 4] {
    let [tl, tr, br, bl] = radii.map(|r| r.max(0.0));
    let share = |side: f32, sum: f32| if sum > 0.0 { side / sum } else { 1.0 };
    let scale = share(width, tl + tr)
        .min(share(width, bl + br))
        .min(share(height, tl + bl))
        .min(share(height, tr + br))
        .min(1.0);
    [tl, tr, br, bl].map(|r| r * scale)
}

impl ComputedStyle {
    pub const TEXT: ComputedStyle = ComputedStyle {
        bg: Color::CLEAR,
        fg: Color::TEXT_DEFAULT,
        font: Font::Sans,
        font_size: 16.0,
        z: 0.0,
        opacity: 1.0,
        gap: (0.0, 0.0),
        pad: [0.0; 4],
        border_radius: [0.0; 4],
        border_size: 0.0,
        border_color: Color::CLEAR,
        outline_size: 0.0,
        outline_color: Color::CLEAR,
        outline_offset: 0.0,
        backdrop_blur: 0.0,
        outline_sweep: Color::CLEAR,
        outline_sweep_speed: 0.3,
        scale: (1.0, 1.0),
        translate: (0.0, 0.0),
        rotate: 0.0,
        gradient: None,
        transition: Transition::INSTANT,
    };

    pub const BASE: ComputedStyle = ComputedStyle {
        bg: Color::GRAY_800,
        fg: Color::INDIGO_50,
        ..ComputedStyle::TEXT
    };

    pub const BUTTON: ComputedStyle = ComputedStyle {
        font_size: 24.0,
        ..ComputedStyle::BASE
    };

    pub const PANEL: ComputedStyle = ComputedStyle {
        bg: Color::GRAY_900,
        opacity: 0.92,
        ..ComputedStyle::TEXT
    };

    pub fn lerp(self, to: ComputedStyle, t: f32) -> ComputedStyle {
        let mix = |from: f32, to: f32| from + (to - from) * t;
        ComputedStyle {
            bg: self.bg.lerp(to.bg, t),
            fg: self.fg.lerp(to.fg, t),
            font_size: mix(self.font_size, to.font_size),
            opacity: mix(self.opacity, to.opacity),
            border_radius: [0, 1, 2, 3].map(|i| mix(self.border_radius[i], to.border_radius[i])),
            border_size: mix(self.border_size, to.border_size),
            border_color: self.border_color.lerp(to.border_color, t),
            outline_size: mix(self.outline_size, to.outline_size),
            outline_color: self.outline_color.lerp(to.outline_color, t),
            outline_offset: mix(self.outline_offset, to.outline_offset),
            backdrop_blur: mix(self.backdrop_blur, to.backdrop_blur),
            outline_sweep: self.outline_sweep.lerp(to.outline_sweep, t),
            outline_sweep_speed: mix(self.outline_sweep_speed, to.outline_sweep_speed),
            scale: (mix(self.scale.0, to.scale.0), mix(self.scale.1, to.scale.1)),
            translate: (
                mix(self.translate.0, to.translate.0),
                mix(self.translate.1, to.translate.1),
            ),
            rotate: mix(self.rotate, to.rotate),
            gradient: match (self.gradient, to.gradient) {
                (Some(a), Some(b)) => Some(LinearGradient {
                    angle: mix(a.angle, b.angle),
                    from: a.from.lerp(b.from, t),
                    via: Some(a.middle().lerp(b.middle(), t)),
                    to: a.to.lerp(b.to, t),
                }),
                (a, b) => {
                    if t < 0.5 {
                        a
                    } else {
                        b
                    }
                }
            },
            ..self
        }
    }
}

impl Style {
    pub const NONE: Style = Style {
        bg: None,
        fg: None,
        font: None,
        font_size: None,
        z: None,
        opacity: None,
        gap: None,
        pad: None,
        border_radius: None,
        border_size: None,
        border_color: None,
        outline_size: None,
        outline_color: None,
        outline_offset: None,
        backdrop_blur: None,
        outline_sweep: None,
        outline_sweep_speed: None,
        scale: None,
        translate: None,
        rotate: None,
        gradient: None,
        transition: None,
    };

    pub fn or(self, base: Style) -> Style {
        Style {
            bg: self.bg.or(base.bg),
            fg: self.fg.or(base.fg),
            font: self.font.or(base.font),
            font_size: self.font_size.or(base.font_size),
            z: self.z.or(base.z),
            opacity: self.opacity.or(base.opacity),
            gap: self.gap.or(base.gap),
            pad: self.pad.or(base.pad),
            border_radius: self.border_radius.or(base.border_radius),
            border_size: self.border_size.or(base.border_size),
            border_color: self.border_color.or(base.border_color),
            outline_size: self.outline_size.or(base.outline_size),
            outline_color: self.outline_color.or(base.outline_color),
            outline_offset: self.outline_offset.or(base.outline_offset),
            backdrop_blur: self.backdrop_blur.or(base.backdrop_blur),
            outline_sweep: self.outline_sweep.or(base.outline_sweep),
            outline_sweep_speed: self.outline_sweep_speed.or(base.outline_sweep_speed),
            scale: self.scale.or(base.scale),
            translate: self.translate.or(base.translate),
            rotate: self.rotate.or(base.rotate),
            gradient: self.gradient.or(base.gradient),
            transition: self.transition.or(base.transition),
        }
    }

    pub fn computed(self, base: ComputedStyle) -> ComputedStyle {
        ComputedStyle {
            bg: self.bg.unwrap_or(base.bg),
            fg: self.fg.unwrap_or(base.fg),
            font: self.font.unwrap_or(base.font),
            font_size: self.font_size.unwrap_or(base.font_size),
            z: self.z.unwrap_or(base.z),
            opacity: self.opacity.unwrap_or(base.opacity),
            gap: self.gap.unwrap_or(base.gap),
            pad: self.pad.unwrap_or(base.pad),
            border_radius: self.border_radius.unwrap_or(base.border_radius),
            border_size: self.border_size.unwrap_or(base.border_size),
            border_color: self.border_color.unwrap_or(base.border_color),
            outline_size: self.outline_size.unwrap_or(base.outline_size),
            outline_color: self.outline_color.unwrap_or(base.outline_color),
            outline_offset: self.outline_offset.unwrap_or(base.outline_offset),
            backdrop_blur: self.backdrop_blur.unwrap_or(base.backdrop_blur),
            outline_sweep: self.outline_sweep.unwrap_or(base.outline_sweep),
            outline_sweep_speed: self.outline_sweep_speed.unwrap_or(base.outline_sweep_speed),
            scale: self.scale.unwrap_or(base.scale),
            translate: self.translate.unwrap_or(base.translate),
            rotate: self.rotate.unwrap_or(base.rotate),
            gradient: self.gradient.or(base.gradient),
            transition: self.transition.unwrap_or(base.transition),
        }
    }
}

impl From<ComputedStyle> for Style {
    fn from(s: ComputedStyle) -> Self {
        Style {
            bg: Some(s.bg),
            fg: Some(s.fg),
            font: Some(s.font),
            font_size: Some(s.font_size),
            z: Some(s.z),
            opacity: Some(s.opacity),
            gap: Some(s.gap),
            pad: Some(s.pad),
            border_radius: Some(s.border_radius),
            border_size: Some(s.border_size),
            border_color: Some(s.border_color),
            outline_size: Some(s.outline_size),
            outline_color: Some(s.outline_color),
            outline_offset: Some(s.outline_offset),
            backdrop_blur: Some(s.backdrop_blur),
            outline_sweep: Some(s.outline_sweep),
            outline_sweep_speed: Some(s.outline_sweep_speed),
            scale: Some(s.scale),
            translate: Some(s.translate),
            rotate: Some(s.rotate),
            gradient: s.gradient,
            transition: Some(s.transition),
        }
    }
}

pub trait Styled: Sized {
    fn style_mut(&mut self) -> &mut Style;

    fn style(mut self, style: Style) -> Self {
        *self.style_mut() = style;
        self
    }
    fn fg(mut self, color: Color) -> Self {
        self.style_mut().fg = Some(color);
        self
    }
    fn bg(mut self, color: Color) -> Self {
        self.style_mut().bg = Some(color);
        self
    }
    fn font(mut self, font: Font) -> Self {
        self.style_mut().font = Some(font);
        self
    }
    fn sans(self) -> Self {
        self.font(Font::Sans)
    }
    fn mono(self) -> Self {
        self.font(Font::Mono)
    }
    fn font_size(mut self, px: f32) -> Self {
        self.style_mut().font_size = Some(px);
        self
    }
    fn z(mut self, z: f32) -> Self {
        self.style_mut().z = Some(z);
        self
    }
    fn gap(self, gap: f32) -> Self {
        self.gap_xy(gap, gap)
    }
    fn gap_xy(mut self, x: f32, y: f32) -> Self {
        self.style_mut().gap = Some((x, y));
        self
    }
    fn gap_x(mut self, x: f32) -> Self {
        let y = self.style_mut().gap.map_or(0.0, |g| g.1);
        self.gap_xy(x, y)
    }
    fn gap_y(mut self, y: f32) -> Self {
        let x = self.style_mut().gap.map_or(0.0, |g| g.0);
        self.gap_xy(x, y)
    }
    fn pad(self, pad: f32) -> Self {
        self.pad_sides(pad, pad, pad, pad)
    }
    fn pad_lr(mut self, x: f32) -> Self {
        let [top, _, bottom, _] = self.style_mut().pad.unwrap_or_default();
        self.pad_sides(top, x, bottom, x)
    }
    fn pad_tb(mut self, y: f32) -> Self {
        let [_, right, _, left] = self.style_mut().pad.unwrap_or_default();
        self.pad_sides(y, right, y, left)
    }
    fn pad_t(mut self, top: f32) -> Self {
        let [_, right, bottom, left] = self.style_mut().pad.unwrap_or_default();
        self.pad_sides(top, right, bottom, left)
    }
    fn pad_r(mut self, right: f32) -> Self {
        let [top, _, bottom, left] = self.style_mut().pad.unwrap_or_default();
        self.pad_sides(top, right, bottom, left)
    }
    fn pad_b(mut self, bottom: f32) -> Self {
        let [top, right, _, left] = self.style_mut().pad.unwrap_or_default();
        self.pad_sides(top, right, bottom, left)
    }
    fn pad_l(mut self, left: f32) -> Self {
        let [top, right, bottom, _] = self.style_mut().pad.unwrap_or_default();
        self.pad_sides(top, right, bottom, left)
    }
    fn pad_sides(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.style_mut().pad = Some([top, right, bottom, left]);
        self
    }
    fn opacity(mut self, opacity: f32) -> Self {
        self.style_mut().opacity = Some(opacity);
        self
    }
    fn border_radius(self, px: f32) -> Self {
        self.border_radius_corners(px, px, px, px)
    }
    fn border_radius_corners(mut self, tl: f32, tr: f32, br: f32, bl: f32) -> Self {
        self.style_mut().border_radius = Some([tl, tr, br, bl]);
        self
    }
    fn border_radius_t(mut self, px: f32) -> Self {
        let [_, _, br, bl] = self.style_mut().border_radius.unwrap_or_default();
        self.border_radius_corners(px, px, br, bl)
    }
    fn border_radius_b(mut self, px: f32) -> Self {
        let [tl, tr, _, _] = self.style_mut().border_radius.unwrap_or_default();
        self.border_radius_corners(tl, tr, px, px)
    }
    fn border_radius_l(mut self, px: f32) -> Self {
        let [_, tr, br, _] = self.style_mut().border_radius.unwrap_or_default();
        self.border_radius_corners(px, tr, br, px)
    }
    fn border_radius_r(mut self, px: f32) -> Self {
        let [tl, _, _, bl] = self.style_mut().border_radius.unwrap_or_default();
        self.border_radius_corners(tl, px, px, bl)
    }
    fn border_radius_tl(mut self, px: f32) -> Self {
        let [_, tr, br, bl] = self.style_mut().border_radius.unwrap_or_default();
        self.border_radius_corners(px, tr, br, bl)
    }
    fn border_radius_tr(mut self, px: f32) -> Self {
        let [tl, _, br, bl] = self.style_mut().border_radius.unwrap_or_default();
        self.border_radius_corners(tl, px, br, bl)
    }
    fn border_radius_br(mut self, px: f32) -> Self {
        let [tl, tr, _, bl] = self.style_mut().border_radius.unwrap_or_default();
        self.border_radius_corners(tl, tr, px, bl)
    }
    fn border_radius_bl(mut self, px: f32) -> Self {
        let [tl, tr, br, _] = self.style_mut().border_radius.unwrap_or_default();
        self.border_radius_corners(tl, tr, br, px)
    }
    fn border_size(mut self, px: f32) -> Self {
        self.style_mut().border_size = Some(px);
        self
    }
    fn border_color(mut self, color: Color) -> Self {
        self.style_mut().border_color = Some(color);
        self
    }
    fn outline(self, color: Color, px: f32) -> Self {
        self.outline_color(color).outline_size(px)
    }
    fn outline_size(mut self, px: f32) -> Self {
        self.style_mut().outline_size = Some(px);
        self
    }
    fn outline_color(mut self, color: Color) -> Self {
        self.style_mut().outline_color = Some(color);
        self
    }
    fn outline_offset(mut self, px: f32) -> Self {
        self.style_mut().outline_offset = Some(px);
        self
    }
    fn backdrop_blur(mut self, px: f32) -> Self {
        self.style_mut().backdrop_blur = Some(px);
        self
    }
    fn outline_sweep(mut self, color: Color) -> Self {
        self.style_mut().outline_sweep = Some(color);
        self
    }
    fn outline_sweep_speed(mut self, turns_per_second: f32) -> Self {
        self.style_mut().outline_sweep_speed = Some(turns_per_second);
        self
    }
    fn scale(self, scale: f32) -> Self {
        self.scale_xy(scale, scale)
    }
    fn scale_xy(mut self, x: f32, y: f32) -> Self {
        self.style_mut().scale = Some((x, y));
        self
    }
    fn scale_x(mut self, x: f32) -> Self {
        let y = self.style_mut().scale.map_or(1.0, |s| s.1);
        self.scale_xy(x, y)
    }
    fn scale_y(mut self, y: f32) -> Self {
        let x = self.style_mut().scale.map_or(1.0, |s| s.0);
        self.scale_xy(x, y)
    }
    fn translate(self, offset: f32) -> Self {
        self.translate_xy(offset, offset)
    }
    fn translate_xy(mut self, x: f32, y: f32) -> Self {
        self.style_mut().translate = Some((x, y));
        self
    }
    fn translate_x(mut self, x: f32) -> Self {
        let y = self.style_mut().translate.map_or(0.0, |t| t.1);
        self.translate_xy(x, y)
    }
    fn translate_y(mut self, y: f32) -> Self {
        let x = self.style_mut().translate.map_or(0.0, |t| t.0);
        self.translate_xy(x, y)
    }
    fn rotate(mut self, radians: f32) -> Self {
        self.style_mut().rotate = Some(radians);
        self
    }
    fn gradient(mut self, angle: f32, from: Color, to: Color) -> Self {
        self.style_mut().gradient = Some(LinearGradient {
            angle,
            from,
            via: None,
            to,
        });
        self
    }
    fn gradient_via(mut self, angle: f32, from: Color, via: Color, to: Color) -> Self {
        self.style_mut().gradient = Some(LinearGradient {
            angle,
            from,
            via: Some(via),
            to,
        });
        self
    }
    fn transition(mut self, transition: Transition) -> Self {
        self.style_mut().transition = Some(transition);
        self
    }
}

pub trait Stateful: Sized {
    fn hover_mut(&mut self) -> &mut Style;
    fn active_mut(&mut self) -> &mut Style;

    fn hover(mut self, style: Style) -> Self {
        *self.hover_mut() = style;
        self
    }
    fn active(mut self, style: Style) -> Self {
        *self.active_mut() = style;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Theme;

    fn rounded() -> Theme {
        Theme {
            border_radius: 6.0,
            ..Theme::default()
        }
    }

    #[test]
    fn unset_fields_inherit_from_the_theme() {
        let theme = rounded();
        let style = Style {
            bg: Some(Color::RED_400),
            z: Some(3.0),
            ..Default::default()
        };
        let s = style.computed(theme.button(true).normal);
        assert_eq!(s.bg, Color::RED_400);
        assert_eq!(s.z, 3.0);
        assert_eq!(s.fg, theme.button_fg);
        assert_eq!(s.font_size, theme.font_size);
        assert_eq!(s.border_radius, [6.0; 4]);
    }

    #[test]
    fn zero_radius_beats_a_rounded_theme() {
        let theme = rounded();
        let style = Style {
            border_radius: Some([0.0; 4]),
            ..Default::default()
        };
        assert_eq!(style.computed(theme.panel()).border_radius, [0.0; 4]);
        assert_eq!(Style::default().computed(theme.panel()).pad, [theme.pad; 4]);
    }

    #[test]
    fn a_state_keeps_the_theme_fields_it_does_not_set() {
        let theme = Theme::default();
        let mine = Style {
            border_size: Some(2.0),
            ..Default::default()
        };
        let hover = mine.or(theme.button(true).hover);
        assert_eq!(hover.border_size, Some(2.0));
        assert_eq!(hover.bg, Some(theme.button_hover));
    }

    struct Probe(Style);

    impl Styled for Probe {
        fn style_mut(&mut self) -> &mut Style {
            &mut self.0
        }
    }

    #[test]
    fn axis_builders_keep_the_other_axis() {
        let probe = || Probe(Style::default());
        let radius = probe()
            .border_radius(4.0)
            .border_radius_tl(9.0)
            .border_radius_b(1.0);
        assert_eq!(radius.0.border_radius, Some([9.0, 4.0, 1.0, 1.0]));
        assert_eq!(
            probe().translate_y(2.0).translate_x(1.0).0.translate,
            Some((1.0, 2.0))
        );
        assert_eq!(probe().gap(4.0).0.gap, Some((4.0, 4.0)));
        assert_eq!(probe().gap(4.0).gap_x(9.0).0.gap, Some((9.0, 4.0)));
        assert_eq!(probe().gap_y(2.0).0.gap, Some((0.0, 2.0)));
        assert_eq!(probe().scale(2.0).scale_y(3.0).0.scale, Some((2.0, 3.0)));
        assert_eq!(
            probe().pad(3.0).pad_lr(1.0).0.pad,
            Some([3.0, 1.0, 3.0, 1.0])
        );
        assert_eq!(probe().pad_tb(2.0).0.pad, Some([2.0, 0.0, 2.0, 0.0]));
        assert_eq!(
            probe().pad(3.0).pad_l(9.0).0.pad,
            Some([3.0, 3.0, 3.0, 9.0])
        );
    }

    #[test]
    fn oversized_corners_shrink_together_like_css() {
        assert_eq!(fit_corners([100.0; 4], 40.0, 20.0), [10.0; 4]);
        assert_eq!(
            fit_corners([30.0, 10.0, 0.0, 0.0], 20.0, 100.0),
            [15.0, 5.0, 0.0, 0.0]
        );
        assert_eq!(fit_corners([4.0; 4], 40.0, 20.0), [4.0; 4]);
    }

    #[test]
    fn lerp_blends_every_visual_field() {
        let from = ComputedStyle::BASE;
        let to = Style {
            border_radius: Some([10.0, 0.0, 0.0, 0.0]),
            scale: Some((2.0, 3.0)),
            ..Default::default()
        }
        .computed(from);
        let half = from.lerp(to, 0.5);
        assert_eq!(half.border_radius, [5.0, 0.0, 0.0, 0.0]);
        assert_eq!(half.scale, (1.5, 2.0));
    }
}
