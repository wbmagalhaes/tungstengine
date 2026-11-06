use super::immediate::UI;
use super::layout::{Dir, Layout};
use super::rect::Rect;
use super::style::{ComputedStyle, Style, Styled};
use super::widget::{Button, Response, Text, Widget};

pub struct Nest<'u> {
    ui: &'u mut UI,
    layout: Layout,
}

impl<'u> Nest<'u> {
    pub fn ui(&mut self) -> &mut UI {
        self.ui
    }

    pub fn next(&mut self, size: f32) -> Rect {
        self.layout.next(size)
    }

    pub fn fill(&mut self) -> Rect {
        self.layout.fill()
    }

    pub fn skip(&mut self, amount: f32) {
        self.layout.skip(amount);
    }

    pub fn split<const N: usize>(&mut self, total: f32, weights: [f32; N]) -> [Rect; N] {
        self.layout.split(total, weights)
    }

    pub fn sized<W: Widget>(&mut self, size: f32, make: impl FnOnce(Rect) -> W) -> W::Out {
        let r = self.layout.next(size);
        self.ui.add(make(r))
    }

    pub fn button(&mut self, label: &str) -> Response {
        self.button_styled(ComputedStyle::BUTTON.into(), label)
    }

    pub fn button_styled(&mut self, style: Style, label: &str) -> Response {
        let look = style.computed(self.ui.theme.button(true).normal);
        let (_, th) = self.ui.measure(look.font, look.font_size, label, None);
        let r = self.layout.next(th + 16.0);
        self.ui.add(Button::new(r, label).style(style))
    }

    pub fn text(&mut self, text: &str) {
        self.text_styled(Style::default(), text);
    }

    pub fn text_styled(&mut self, style: Style, text: &str) {
        let look = style.computed(self.ui.theme.text());
        let r = self.layout.text(self.ui, &look, text);
        self.ui
            .add(Text::new(r.x, r.y, text).style(style).wrapped(r.w));
    }

    pub fn panel<R>(&mut self, rect: Rect, body: impl FnOnce(&mut Nest) -> R) -> R {
        self.ui.panel(rect, body)
    }

    pub fn panel_styled<R>(
        &mut self,
        rect: Rect,
        style: Style,
        body: impl FnOnce(&mut Nest) -> R,
    ) -> R {
        self.ui.panel_styled(rect, style, body)
    }
}

impl UI {
    pub fn column<R>(&mut self, area: Rect, gap: f32, body: impl FnOnce(&mut Nest) -> R) -> R {
        let layout = Layout::new(Dir::Vertical, area, gap);
        body(&mut Nest { ui: self, layout })
    }

    pub fn row<R>(&mut self, area: Rect, gap: f32, body: impl FnOnce(&mut Nest) -> R) -> R {
        let layout = Layout::new(Dir::Horizontal, area, gap);
        body(&mut Nest { ui: self, layout })
    }

    pub fn panel<R>(&mut self, rect: Rect, body: impl FnOnce(&mut Nest) -> R) -> R {
        self.panel_styled(rect, Style::default(), body)
    }

    pub fn panel_styled<R>(
        &mut self,
        rect: Rect,
        style: Style,
        body: impl FnOnce(&mut Nest) -> R,
    ) -> R {
        let look = style.computed(self.theme.panel());
        self.fill_styled(rect, look.bg.with_alpha(look.bg.a * look.opacity), &look);
        self.push_clip_corners(rect, look.border_radius);
        let [top, right, bottom, left] = look.pad;
        let inner = Rect::new(
            rect.x + left,
            rect.y + top,
            (rect.w - left - right).max(0.0),
            (rect.h - top - bottom).max(0.0),
        )
        .with_z(rect.z);
        let out = self.column(inner, look.gap.1, body);
        self.pop_clip();
        out
    }
}
