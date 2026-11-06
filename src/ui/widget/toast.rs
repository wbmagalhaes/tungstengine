use crate::animation::Ease;
use crate::ecs::prelude::*;
use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::style::Transition;

const LIFE: f32 = 3.0;
const MARGIN: f32 = 16.0;
const HEIGHT: f32 = 40.0;
const GAP: f32 = 8.0;
const PAD: f32 = 14.0;
const SLIDE: f32 = 36.0;
const STACK_RATE: f32 = 14.0;
const Z: f32 = 300.0;
const CURVE: Transition = Transition::new(0.22, Ease::OutBack).exit(0.18, Ease::InQuad);
const EXIT_TIME: f32 = 0.18;

struct Item {
    id: u64,
    text: String,
    age: f32,
    life: f32,
    rise: f32,
}

#[derive(Resource, Default)]
pub struct Toasts {
    items: Vec<Item>,
    next: u64,
}

impl Toasts {
    pub fn push(&mut self, text: impl Into<String>) {
        self.push_for(text, LIFE);
    }

    pub fn push_for(&mut self, text: impl Into<String>, seconds: f32) {
        self.next += 1;
        self.items.push(Item {
            id: self.next,
            text: text.into(),
            age: 0.0,
            life: seconds,
            rise: 0.0,
        });
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn draw(&mut self, ui: &mut UI) {
        let t = ui.theme;
        let dt = ui.dt();
        self.items.retain(|item| item.age < item.life + EXIT_TIME);
        let ease = 1.0 - (-STACK_RATE * dt).exp();
        let count = self.items.len();
        for (n, item) in self.items.iter_mut().enumerate() {
            item.age += dt;
            let target = (count - 1 - n) as f32 * (HEIGHT + GAP);
            item.rise += (target - item.rise) * ease;
            let goal = f32::from(u8::from(item.age < item.life));
            if item.age <= dt {
                ui.transition(("toast", item.id), 0.0, CURVE);
            }
            let shown = ui.transition(("toast", item.id), goal, CURVE);
            let alpha = shown.clamp(0.0, 1.0);
            let width = ui.text_width(t.font_size, &item.text) + PAD * 2.0;
            let x = ui.width - MARGIN - width + (1.0 - shown) * SLIDE;
            let y = ui.height - MARGIN - HEIGHT - item.rise;
            let card = Rect::new(x, y, width, HEIGHT).with_z(Z);
            let edge = t.accent.with_alpha(0.6 * alpha);
            let bg = t.panel_bg.with_alpha(0.95 * alpha);
            ui.fill_bordered(card, bg, t.border_radius, 1.0, edge);
            let ink = t.label.with_alpha(alpha);
            let text_y = y + (HEIGHT - t.font_size) / 2.0;
            ui.label(x + PAD, text_y, &item.text, ink, t.font_size, Z + 0.01);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    fn frame(ui: &mut UI, toasts: &mut Toasts, dt: f32) {
        ui.begin(800.0, 600.0, dt, Pointer::default());
        toasts.draw(ui);
    }

    #[test]
    fn toasts_leave_after_their_life() {
        let mut ui = UI::new();
        let mut toasts = Toasts::default();
        toasts.push_for("Saved", 0.2);
        frame(&mut ui, &mut toasts, 0.1);
        assert_eq!(toasts.len(), 1);
        for _ in 0..6 {
            frame(&mut ui, &mut toasts, 0.1);
        }
        assert!(toasts.is_empty());
    }

    #[test]
    fn newest_sits_at_the_bottom_of_the_stack() {
        let mut ui = UI::new();
        let mut toasts = Toasts::default();
        toasts.push("Old");
        toasts.push("New");
        for _ in 0..40 {
            frame(&mut ui, &mut toasts, 0.03);
        }
        assert!(toasts.items[0].rise > toasts.items[1].rise + HEIGHT);
    }
}
