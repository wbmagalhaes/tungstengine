use super::immediate::UI;
use super::layout::{Dir, Layout};
use super::rect::Rect;
use super::style::{Style, Styled};
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
        self.button_styled(Style::DEFAULT_BUTTON, label)
    }

    pub fn button_styled(&mut self, style: Style, label: &str) -> Response {
        let (_, th) = self.ui.measure(style.font, style.font_size, label, None);
        let r = self.layout.next(th + 16.0);
        self.ui.add(Button::new(r, label).style(style))
    }

    pub fn text(&mut self, text: &str) {
        self.text_styled(Style::DEFAULT_TEXT, text);
    }

    pub fn text_styled(&mut self, style: Style, text: &str) {
        let r = self.layout.text(self.ui, &style, text);
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
        self.panel_styled(rect, Style::PANEL, body)
    }

    pub fn panel_styled<R>(
        &mut self,
        rect: Rect,
        style: Style,
        body: impl FnOnce(&mut Nest) -> R,
    ) -> R {
        self.fill(rect, style.bg.with_alpha(style.bg.a * style.opacity));
        self.push_clip(rect);
        let inner = rect.with_padding(style.pad).inner();
        let out = self.column(inner, style.gap, body);
        self.pop_clip();
        out
    }
}
