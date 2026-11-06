use std::hash::Hash;

use crate::ui::immediate::{UI, key_hash};
use crate::ui::rect::Rect;
use crate::ui::style::{Style, Styled};
use crate::ui::widget::{Button, Widget};

pub struct Tabs<'a> {
    rect: Rect,
    labels: &'a [&'a str],
    width: f32,
    gap: f32,
    selected: usize,
    selected_style: Option<Style>,
    id: u64,
}

impl<'a> Tabs<'a> {
    pub fn new(rect: Rect, labels: &'a [&'a str]) -> Self {
        Tabs {
            rect,
            labels,
            width: 160.0,
            gap: 10.0,
            selected: 0,
            selected_style: None,
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

    pub fn selected_style(mut self, style: Style) -> Self {
        self.selected_style = Some(style);
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
        let at = self.rect;
        let selected_color = ui.theme.selected;
        let mut pick = None;
        for (k, label) in self.labels.iter().enumerate() {
            let r = Rect::new(
                at.x + k as f32 * (self.width + self.gap),
                at.y,
                self.width,
                at.h,
            )
            .with_z(at.z);
            let mut b = Button::new(r, label).id((self.id, k)).z(at.z);
            if k == self.selected {
                b = match self.selected_style {
                    Some(style) => b.style(style).z(at.z),
                    None => b.outline(selected_color, 3.0),
                };
            }
            if ui.add(b).clicked() && k != self.selected {
                pick = Some(k);
            }
        }
        pick
    }
}
