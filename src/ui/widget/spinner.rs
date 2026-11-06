use std::f32::consts::TAU;

use crate::color::Color;
use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::widget::Widget;

const DOTS: usize = 10;
const DOT_SHARE: f32 = 0.16;
const TURNS_PER_SECOND: f32 = 1.0;
const FAINT: f32 = 0.15;

pub struct Spinner {
    rect: Rect,
    color: Option<Color>,
    speed: f32,
}

impl Spinner {
    pub fn new(rect: Rect) -> Self {
        Spinner {
            rect,
            color: None,
            speed: TURNS_PER_SECOND,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn speed(mut self, turns_per_second: f32) -> Self {
        self.speed = turns_per_second;
        self
    }
}

impl Widget for Spinner {
    type Out = ();

    fn render(self, ui: &mut UI) {
        let color = self.color.unwrap_or(ui.theme.accent);
        let r = self.rect;
        let side = r.w.min(r.h);
        let dot = side * DOT_SHARE;
        let reach = (side - dot) / 2.0;
        let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
        let head = (ui.elapsed() * self.speed).fract();
        for i in 0..DOTS {
            let share = i as f32 / DOTS as f32;
            let turn = share * TAU;
            let behind = (head - share).rem_euclid(1.0);
            let alpha = FAINT + (1.0 - FAINT) * (1.0 - behind).powi(2);
            let x = cx + turn.cos() * reach - dot / 2.0;
            let y = cy + turn.sin() * reach - dot / 2.0;
            let spot = Rect::new(x, y, dot, dot).with_z(r.z);
            ui.fill_round(spot, color.with_alpha(color.a * alpha), dot / 2.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Pointer;

    fn alphas(ui: &mut UI) -> Vec<f32> {
        ui.begin(800.0, 600.0, 0.1, Pointer::default());
        ui.add(Spinner::new(Rect::new(0.0, 0.0, 40.0, 40.0).with_z(2.0)));
        ui.draws.iter().map(|d| d.color[3]).collect()
    }

    #[test]
    fn dots_keep_their_place_and_the_light_moves() {
        let mut ui = UI::new();
        let first = alphas(&mut ui);
        let second = alphas(&mut ui);
        assert_eq!(first.len(), DOTS);
        assert_ne!(first, second);
        assert!(ui.draws.iter().all(|d| d.z == 2.0));
    }
}
