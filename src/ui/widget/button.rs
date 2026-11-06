use std::hash::Hash;

use crate::animation::Ease;
use crate::color::Color;
use crate::ui::immediate::{UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::{ComputedStyle, Stateful, Style, Styled, Transition};
use crate::ui::widget::{Response, Widget};

const DEFAULT_HOVER: Style = Style {
    scale: Some((1.06, 1.06)),
    transition: Some(Transition::new(0.14, Ease::OutCubic).exit(0.24, Ease::InOutQuad)),
    ..Style::NONE
};
const DEFAULT_ACTIVE: Style = Style {
    scale: Some((0.96, 0.96)),
    transition: Some(Transition::new(0.06, Ease::OutQuad).exit(0.2, Ease::OutBack)),
    ..Style::NONE
};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Click,
    Repeat,
    Hold(f32),
}

pub struct Button<'a> {
    rect: Rect,
    label: &'a str,
    id: Option<u64>,
    style: Style,
    hover: Style,
    active: Style,
    enabled: bool,
    mode: Mode,
}

impl<'a> Button<'a> {
    pub fn new(rect: Rect, label: &'a str) -> Self {
        Button {
            rect,
            label,
            id: None,
            style: Style::default(),
            hover: Style::default(),
            active: Style::default(),
            enabled: true,
            mode: Mode::Click,
        }
    }

    pub fn id(mut self, id: impl Hash) -> Self {
        self.id = Some(key_hash(id));
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn repeat(mut self) -> Self {
        self.mode = Mode::Repeat;
        self
    }

    pub fn hold(mut self, secs: f32) -> Self {
        self.mode = Mode::Hold(secs);
        self
    }
}

impl Styled for Button<'_> {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl Stateful for Button<'_> {
    fn hover_mut(&mut self) -> &mut Style {
        &mut self.hover
    }
    fn active_mut(&mut self) -> &mut Style {
        &mut self.active
    }
}

impl Widget for Button<'_> {
    type Out = Response;

    fn render(self, ui: &mut UI) -> Response {
        let theme = ui.theme.button(self.enabled);
        let style = self.style.computed(theme.normal);
        let id = self.id.unwrap_or_else(|| key_hash(self.label));
        let mut resp = match self.id {
            Some(id) => ui.interact_hashed(self.rect, id),
            None => ui.interact(self.rect, self.label),
        };
        let own = Style {
            transition: self.style.transition,
            ..Style::NONE
        };
        let hovered = self
            .hover
            .or(own)
            .or(theme.hover)
            .or(DEFAULT_HOVER)
            .computed(style);
        let hover_target = f32::from(u8::from(resp.hovered));
        let hover_t = ui.transition((id, "hover-state"), hover_target, hovered.transition);
        let mut look = style.lerp(hovered, hover_t);
        let active = self.active.or(own).or(theme.active).or(DEFAULT_ACTIVE);
        let press = active.scale.unwrap_or((1.0, 1.0));
        let mut pressed = active.computed(look);
        pressed.scale = (look.scale.0 * press.0, look.scale.1 * press.1);
        let held = f32::from(u8::from(resp.pressed));
        let active_t = ui.transition((id, "active-state"), held, pressed.transition);
        look = look.lerp(pressed, active_t);
        let (bg, fg, fs, o) = (look.bg, look.fg, look.font_size, look.opacity);
        let (radius, size, edge) = (look.border_radius, look.border_size, look.border_color);

        let first_draw = ui.draws.len();
        let rect = self.rect.with_z(style.z);
        let outer = Rect::new(
            rect.x - size,
            rect.y - size,
            rect.w + 2.0 * size,
            rect.h + 2.0 * size,
        )
        .with_z(rect.z);
        let painted = ComputedStyle {
            opacity: o,
            border_color: edge.with_alpha(edge.a * o),
            outline_color: look.outline_color.with_alpha(look.outline_color.a * o),
            border_radius: radius,
            border_size: size,
            ..look
        };
        ui.fill_styled(outer, bg.with_alpha(bg.a * o), &painted);

        let fg = fg.with_alpha(fg.a * o);
        let area = rect.inner();
        let w = ui.text_width(fs, self.label);
        let tx = area.x + (area.w - w) / 2.0;
        let ty = area.y + (area.h - fs) / 2.0;
        ui.glyphs(tx, ty, fs, self.label, fg, rect.z);
        let pivot = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
        ui.transform_since(first_draw, pivot, look.scale, look.translate, look.rotate);

        resp.clicked = self.enabled
            && match self.mode {
                Mode::Click => resp.clicked,
                Mode::Repeat => {
                    let held = ui.hold_repeat((id, "repeat"), self.rect, 0.35, 0.09);
                    resp.clicked || held
                }
                Mode::Hold(secs) => {
                    let (progress, fired) = ui.hold_progress((id, "hold"), self.rect, secs);
                    if progress > 0.0 {
                        let r = self.rect;
                        ui.surface(
                            Rect::new(r.x, r.y, r.w * progress, r.h).with_z(r.z + 0.05),
                            ui.theme.danger.with_alpha(0.45),
                        );
                    }
                    fired
                }
            };
        resp
    }
}

pub fn button(ui: &mut UI, r: Rect, label: &str, enabled: bool, id: &str) -> bool {
    button_outlined(ui, r, label, enabled, id, None)
}

pub fn button_outlined(
    ui: &mut UI,
    r: Rect,
    label: &str,
    enabled: bool,
    id: &str,
    outline: Option<Color>,
) -> bool {
    let mut b = Button::new(r, label)
        .id((id, label))
        .enabled(enabled)
        .z(r.z);
    if let Some(c) = outline {
        b = b.outline(c, 3.0);
    }
    ui.add(b).clicked()
}

pub fn repeat_button(ui: &mut UI, r: Rect, label: &str, enabled: bool, id: &str) -> bool {
    ui.add(
        Button::new(r, label)
            .id((id, label))
            .enabled(enabled)
            .repeat()
            .z(r.z),
    )
    .clicked()
}

pub fn hold_button(
    ui: &mut UI,
    r: Rect,
    label: &str,
    enabled: bool,
    id: &str,
    outline: Option<Color>,
    secs: f32,
) -> bool {
    let mut b = Button::new(r, label)
        .id((id, label))
        .enabled(enabled)
        .hold(secs)
        .z(r.z);
    if let Some(c) = outline {
        b = b.outline(c, 3.0);
    }
    ui.add(b).clicked()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;
    use crate::ui::widget::Tabs;

    fn frame(ui: &mut UI, at: (f32, f32), held: bool, released: bool) {
        ui.begin(
            800.0,
            600.0,
            0.1,
            Pointer {
                pos: at,
                held,
                released,
                ..Default::default()
            },
        );
    }

    #[test]
    fn disabled_button_never_clicks() {
        let mut ui = UI::new();
        let r = Rect::new(0.0, 0.0, 100.0, 40.0);
        frame(&mut ui, (10.0, 10.0), false, true);
        assert!(ui.add(Button::new(r, "ok").id("a")).clicked());
        frame(&mut ui, (10.0, 10.0), false, true);
        assert!(
            !ui.add(Button::new(r, "ok").id("a").enabled(false))
                .clicked()
        );
    }

    #[test]
    fn hold_button_fires_only_after_holding() {
        let mut ui = UI::new();
        let r = Rect::new(0.0, 0.0, 100.0, 40.0);
        let mut fired = 0;
        for _ in 0..20 {
            frame(&mut ui, (10.0, 10.0), true, false);
            if ui.add(Button::new(r, "fugir").id("h").hold(1.0)).clicked() {
                fired += 1;
            }
        }
        assert_eq!(fired, 1);
        frame(&mut ui, (10.0, 10.0), false, true);
        assert!(!ui.add(Button::new(r, "fugir").id("h").hold(1.0)).clicked());
    }

    #[test]
    fn tabs_report_new_selection_only() {
        let mut ui = UI::new();
        let bar = Rect::new(0.0, 0.0, 400.0, 40.0);
        let labels = ["A", "B"];
        frame(&mut ui, (200.0, 10.0), false, true);
        let tabs = Tabs::new(bar, &labels).width(150.0).selected(0).id("t");
        assert_eq!(ui.add(tabs), Some(1));
        frame(&mut ui, (20.0, 10.0), false, true);
        let tabs = Tabs::new(bar, &labels).width(150.0).selected(0).id("t");
        assert_eq!(ui.add(tabs), None);
    }
}

#[cfg(test)]
mod transform_tests {
    use super::*;
    use crate::input::Pointer;
    use crate::ui::Theme;

    #[test]
    fn translate_and_rotate_move_the_whole_button() {
        let mut ui = UI::new();
        ui.begin(800.0, 600.0, 0.016, Pointer::default());
        let rect = Rect::new(100.0, 100.0, 40.0, 20.0);
        ui.add(
            Button::new(rect, "Ok")
                .border_radius(4.0)
                .translate_x(10.0)
                .rotate(std::f32::consts::FRAC_PI_2),
        );
        let box_draw = ui.draws.iter().find(|d| d.round.is_some()).unwrap();
        let round = box_draw.round.unwrap();
        assert_eq!(round.center, [130.0, 110.0]);
        assert_eq!(round.rotation, std::f32::consts::FRAC_PI_2);
        assert!(box_draw.max[1] - box_draw.min[1] > box_draw.max[0] - box_draw.min[0]);
    }

    #[test]
    fn gradient_reaches_a_square_button_too() {
        let mut ui = UI::new();
        ui.begin(800.0, 600.0, 0.016, Pointer::default());
        let rect = Rect::new(0.0, 0.0, 40.0, 20.0);
        ui.add(Button::new(rect, "Ok").gradient(1.0, Color::RED, Color::BLUE));
        let round = ui.draws.iter().find_map(|d| d.round).unwrap();
        let gradient = round.gradient.unwrap();
        assert_eq!(gradient.to, Color::BLUE.to_array());
        assert_eq!(gradient.angle, 1.0);
        assert_eq!(gradient.via, Color::RED.lerp(Color::BLUE, 0.5).to_array());

        ui.begin(800.0, 600.0, 0.016, Pointer::default());
        ui.add(Button::new(rect, "Ok").gradient_via(1.0, Color::RED, Color::WHITE, Color::BLUE));
        let round = ui.draws.iter().find_map(|d| d.round).unwrap();
        assert_eq!(round.gradient.unwrap().via, Color::WHITE.to_array());
    }

    #[test]
    fn exit_runs_on_its_own_time_and_retargets_without_a_jump() {
        let curve = Transition::new(1.0, Ease::Linear).exit(0.5, Ease::Linear);
        let mut ui = UI::new();
        ui.begin(800.0, 600.0, 0.1, Pointer::default());
        assert_eq!(ui.transition("t", 0.0, curve), 0.0);
        let rising = ui.transition("t", 1.0, curve);
        assert!((rising - 0.1).abs() < 1e-4);
        let up = (0..4).fold(0.0, |_, _| ui.transition("t", 1.0, curve));
        assert!((up - 0.5).abs() < 1e-4);
        let down = ui.transition("t", 0.0, curve);
        assert!((down - 0.4).abs() < 1e-4);
    }

    #[test]
    fn scale_grows_the_shape_and_the_text_together() {
        let mut ui = UI::new();
        ui.begin(800.0, 600.0, 0.016, Pointer::default());
        let rect = Rect::new(100.0, 100.0, 40.0, 20.0);
        ui.add(Button::new(rect, "Ok").border_radius(4.0).scale(2.0));
        let round = ui.draws.iter().find_map(|d| d.round).unwrap();
        assert_eq!(round.half, [40.0, 20.0]);
        assert_eq!(round.radii, [8.0; 4]);
        let plain = |ui: &mut UI| {
            let glyph = ui.draws.iter().find(|d| d.round.is_none()).unwrap();
            glyph.max[1] - glyph.min[1]
        };
        let big = plain(&mut ui);
        ui.begin(800.0, 600.0, 0.016, Pointer::default());
        ui.add(Button::new(rect, "Ok").border_radius(4.0));
        assert!((big / plain(&mut ui) - 2.0).abs() < 0.01);
    }

    #[test]
    fn button_keeps_its_scale_behavior_without_the_theme() {
        assert!(Theme::default().button(true).hover.scale.is_none());
        assert_eq!(DEFAULT_HOVER.scale, Some((1.06, 1.06)));
    }

    #[test]
    fn outline_sits_outside_the_border_with_its_offset() {
        let mut ui = UI::new();
        ui.begin(800.0, 600.0, 0.016, Pointer::default());
        let rect = Rect::new(100.0, 100.0, 40.0, 20.0);
        ui.add(
            Button::new(rect, "Ok")
                .border_radius(4.0)
                .border_size(1.0)
                .outline(Color::RED, 2.0)
                .outline_offset(3.0),
        );
        let shapes: Vec<_> = ui.draws.iter().filter_map(|d| d.round).collect();
        let (ring, body) = (shapes[0], shapes[1]);
        assert_eq!(ring.half, [body.half[0] + 5.0, body.half[1] + 5.0]);
        assert_eq!(ring.radii, [9.0; 4]);
        assert_eq!(ring.border, 2.0);
        assert_eq!(body.border, 1.0);
    }

    #[test]
    fn outline_sweep_advances_with_time_on_the_ring_only() {
        let mut ui = UI::new();
        let rect = Rect::new(0.0, 0.0, 40.0, 20.0);
        let frame = |ui: &mut UI| {
            ui.begin(800.0, 600.0, 0.25, Pointer::default());
            ui.add(
                Button::new(rect, "Ok")
                    .outline(Color::RED, 2.0)
                    .outline_sweep(Color::WHITE)
                    .outline_sweep_speed(1.0),
            );
            let shapes: Vec<_> = ui.draws.iter().filter_map(|d| d.round).collect();
            shapes[0].sweep
        };
        let first = frame(&mut ui).unwrap().0;
        let second = frame(&mut ui).unwrap().0;
        assert!(((second - first).rem_euclid(1.0) - 0.25).abs() < 1e-4);
    }

    #[test]
    fn backdrop_blur_works_on_a_square_rect_with_its_radius() {
        let mut ui = UI::new();
        ui.begin(800.0, 600.0, 0.016, Pointer::default());
        let rect = Rect::new(0.0, 0.0, 40.0, 20.0);
        ui.add(Button::new(rect, "Ok").backdrop_blur(12.0));
        let round = ui.draws.iter().find_map(|d| d.round).unwrap();
        assert_eq!(round.blur, 12.0);
    }
}
