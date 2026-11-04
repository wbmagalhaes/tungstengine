use crate::AdventureTheme;
use tungstengine::animation::{Ease, Tween};
use tungstengine::ecs::Resource;
use tungstengine::ui::prelude::*;

#[derive(Resource)]
pub struct MenuState {
    pub intro: Tween<f32>,
}

impl Default for MenuState {
    fn default() -> Self {
        MenuState {
            intro: Tween::new(0.0, 1.0, 0.6).ease(Ease::OutBack),
        }
    }
}

pub enum MenuAction {
    None,
    Play,
    Quit,
}

pub fn build(ui: &mut UI, theme: &AdventureTheme, intro: f32, labels: (&str, &str)) -> MenuAction {
    let alpha = intro.clamp(0.0, 1.0);
    let slide = (1.0 - intro) * 30.0;

    let title_px = theme.font_lg;
    let title = theme.title.clone();
    let title_box = ui.center_x(Rect::new(
        0.0,
        ui.height * 0.20 - slide,
        ui.text_width(title_px, &title),
        title_px,
    ));

    ui.add(
        Text::new(title_box.x, title_box.y, &title)
            .font_size(title_px)
            .fg(theme.accent_fg.with_alpha(alpha)),
    );

    let area = ui.center_x(Rect::new(0.0, ui.height * 0.48, 300.0, 130.0));
    let [play, quit] = area.split_v::<2>(18.0);

    let mut action = MenuAction::None;
    if ui.button(play.with_padding(8.0), labels.0).clicked() {
        action = MenuAction::Play;
    }
    if ui.button(quit.with_padding(8.0), labels.1).clicked() {
        action = MenuAction::Quit;
    }

    action
}
