use crate::color::Color;
use crate::ui::immediate::UI;
use crate::ui::rect::Rect;
use crate::ui::style::Styled;
use crate::ui::widget::Text;

pub const DENY_SECS: f32 = 2.4;

pub fn deny_shake(t_now: f32, age: f32) -> f32 {
    if age >= 0.4 {
        return 0.0;
    }
    (t_now * 60.0).sin() * 7.0 * (1.0 - age / 0.4)
}

pub fn deny_bubble(ui: &mut UI, text: &str, cx: f32, bottom: f32, age: f32, z: f32) {
    let a = (DENY_SECS - age).clamp(0.0, 1.0);
    let w = ui.text_width(18.0, text) + 20.0;
    let r = Rect::new(cx - w / 2.0, bottom - 28.0 - age * 10.0, w, 28.0).with_z(z);
    ui.fill(r, Color::BLACK.with_alpha(0.7 * a));
    ui.add(
        Text::centered(r, text)
            .fg(ui.theme.danger.with_alpha(a))
            .font_size(18.0)
            .z(z + 0.01),
    );
}
