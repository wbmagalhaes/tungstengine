use std::f32::consts::FRAC_PI_2;

use crate::animation::Ease;
use std::hash::Hash;

use crate::ui::immediate::{UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::{ComputedStyle, LinearGradient, Transition};
use crate::ui::widget::Widget;

const INSET: f32 = 3.0;
const ICON_W: f32 = 20.0;
const ICON_GAP: f32 = 7.0;
const ICON_INSET: f32 = 6.0;
const FILL_START_ALPHA: f32 = 0.4;
const FILL_END_ALPHA: f32 = 0.9;
const HOVER_TINT: f32 = 0.08;
const SLIDE: Transition = Transition::new(0.2, Ease::OutCubic);
const HOVER: Transition = Transition::new(0.08, Ease::OutCubic).exit(0.18, Ease::InOutQuad);

pub type Leading<'a> = &'a dyn Fn(&mut UI, Rect, usize);

pub struct Tabs<'a> {
    rect: Rect,
    labels: &'a [&'a str],
    width: f32,
    selected: usize,
    leading: Option<Leading<'a>>,
    id: u64,
}

impl<'a> Tabs<'a> {
    pub fn new(rect: Rect, labels: &'a [&'a str]) -> Self {
        Tabs {
            rect,
            labels,
            width: 160.0,
            selected: 0,
            leading: None,
            id: 0,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    pub fn leading(mut self, f: Leading<'a>) -> Self {
        self.leading = Some(f);
        self
    }

    pub fn id(mut self, id: impl Hash) -> Self {
        self.id = key_hash(id);
        self
    }
}

impl Widget for Tabs<'_> {
    type Out = Option<usize>;

    fn render(self, ui: &mut UI) -> Option<usize> {
        let t = ui.theme;
        let at = self.rect;
        let count = self.labels.len() as f32;
        let total = self.width * count;
        let pill_w = (total - INSET * (count + 1.0)) / count;
        let stride = pill_w + INSET;
        let track = Rect::new(at.x, at.y, total, at.h).with_z(at.z);
        ui.field_box(track, false);
        let target = self.selected as f32;
        let spot = ui.transition((self.id, "tabs-slide"), target, SLIDE);
        let radius = (t.border_radius - INSET).max(0.0);
        if self.selected < self.labels.len() {
            let pill = Rect::new(
                at.x + INSET + spot * stride,
                at.y + INSET,
                pill_w,
                at.h - INSET * 2.0,
            )
            .with_z(at.z + 0.001);
            let look = ComputedStyle {
                border_radius: [radius; 4],
                gradient: Some(LinearGradient {
                    angle: FRAC_PI_2,
                    from: t.accent.with_alpha(FILL_START_ALPHA),
                    via: None,
                    to: t.accent.with_alpha(FILL_END_ALPHA),
                }),
                ..ComputedStyle::BASE
            };
            ui.fill_styled(pill, t.accent, &look);
        }
        let size = t.font_size;
        let mut pick = None;
        for (k, label) in self.labels.iter().enumerate() {
            let cell = Rect::new(
                at.x + INSET + k as f32 * stride,
                at.y + INSET,
                pill_w,
                at.h - INSET * 2.0,
            )
            .with_z(at.z);
            let resp = ui.interact_curved(cell, (self.id, k), HOVER);
            let chosen = k == self.selected;
            if !chosen && resp.hover() > 0.0 {
                let glow = cell.with_z(at.z + 0.001);
                ui.fill_round(
                    glow,
                    t.button_fg.with_alpha(HOVER_TINT * resp.hover()),
                    radius,
                );
            }
            let ink = if chosen { t.button_fg } else { t.label };
            let icon = if self.leading.is_some() {
                ICON_W + ICON_GAP
            } else {
                0.0
            };
            let x = cell.x + (cell.w - ui.text_width(size, label) - icon) / 2.0;
            if let Some(draw) = self.leading {
                let spot = Rect::new(x, cell.y + ICON_INSET, ICON_W, cell.h - ICON_INSET * 2.0);
                draw(ui, spot.with_z(at.z + 0.002), k);
            }
            let x = x + icon;
            ui.label(
                x,
                cell.y + (cell.h - size) / 2.0,
                label,
                ink,
                size,
                at.z + 0.002,
            );
            if resp.clicked && !chosen {
                pick = Some(k);
            }
        }
        pick
    }
}
