use crate::color::Color;
use crate::ui::{Rect, Sprite, UI};

pub fn vignette(ui: &mut UI, color: Color, strength: f32, band: f32, z: f32) {
    let (w, h) = (ui.width, ui.height);
    let edge = 18;
    for i in 0..edge {
        let f = 1.0 - i as f32 / edge as f32;
        let c = color.with_alpha(strength * f);
        let d = i as f32 * band;
        ui.fill(Rect::new(d, 0.0, band, h).with_z(z), c);
        ui.fill(Rect::new(w - d - band, 0.0, band, h).with_z(z), c);
        ui.fill(Rect::new(0.0, d, w, band).with_z(z), c);
        ui.fill(Rect::new(0.0, h - d - band, w, band).with_z(z), c);
    }
}

pub fn motes(ui: &mut UI, t: f32, glow: Color, z: f32) {
    let dot = Sprite::CIRCLE.draw();
    for i in 0..40 {
        let k = i as f32;
        let seed = (k * 12.9898).sin() * 43758.547;
        let fx = seed - seed.floor();
        let speed = 12.0 + 18.0 * ((k * 3.7).sin() * 0.5 + 0.5);
        let y = ui.height - ((t * speed + k * 97.0) % (ui.height + 40.0));
        let x = fx * ui.width + (t * 0.6 + k).sin() * 24.0;
        let size = 2.0 + 4.0 * ((k * 1.3).cos() * 0.5 + 0.5);
        let a = 0.15 + 0.35 * ((t * 1.5 + k).sin() * 0.5 + 0.5);
        ui.sprite(
            Rect::new(x, y, size, size).with_z(z),
            dot,
            glow.with_alpha(a),
        );
    }
}
