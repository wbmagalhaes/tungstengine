use std::hash::Hash;

use crate::color::Color;
use crate::ui::immediate::{UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::{StylePatch, Styled};
use crate::ui::widget::{Response, Widget};

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
    patch: StylePatch,
    outline: Option<(Color, f32)>,
    enabled: bool,
    mode: Mode,
}

impl<'a> Button<'a> {
    pub fn new(rect: Rect, label: &'a str) -> Self {
        Button {
            rect,
            label,
            id: None,
            patch: StylePatch::default(),
            outline: None,
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

    pub fn outline(mut self, color: Color, width: f32) -> Self {
        self.outline = Some((color, width));
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
    fn patch_mut(&mut self) -> &mut StylePatch {
        &mut self.patch
    }
}

impl Widget for Button<'_> {
    type Out = Response;

    fn render(self, ui: &mut UI) -> Response {
        let style = ui.theme.button(self.enabled).with(self.patch);
        let id = self.id.unwrap_or_else(|| key_hash(self.label));
        let mut resp = match self.id {
            Some(id) => ui.interact_hashed(self.rect, id),
            None => ui.interact(self.rect, self.label),
        };
        let t = resp.hover;

        let mut scale = 1.0 + (style.hover_scale - 1.0) * t;
        let mut bg = style.bg.lerp(style.hover_color.unwrap_or(style.bg), t);
        if resp.pressed {
            scale *= style.active_scale;
            if let Some(ac) = style.active_color {
                bg = ac;
            }
        }

        let o = style.opacity;
        let rect = self.rect.scaled(scale).with_z(style.z);
        if let Some((color, w)) = self.outline {
            let r = Rect::new(rect.x - w, rect.y - w, rect.w + 2.0 * w, rect.h + 2.0 * w)
                .with_z(rect.z - 0.001);
            ui.fill(r, color.with_alpha(color.a * o));
        }
        ui.fill(rect, bg.with_alpha(bg.a * o));

        let fs = style.font_size;
        let fg = style.fg.with_alpha(style.fg.a * o);
        let area = rect.inner();
        let w = ui.text_width(fs, self.label);
        let tx = area.x + (area.w - w) / 2.0;
        let ty = area.y + (area.h - fs) / 2.0;
        ui.glyphs(tx, ty, fs, self.label, fg, rect.z);

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
                        ui.fill(
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
