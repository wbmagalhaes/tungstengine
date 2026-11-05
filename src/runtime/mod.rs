#[cfg(feature = "2d")]
pub mod d2;
#[cfg(feature = "3d")]
pub mod d3;

use crate::ecs::World;
use crate::input::Input;
use crate::sprites::Sprites;
use crate::ui::UI;

pub fn ui_pass(
    world: &mut World,
    ui: &mut UI,
    frame_ui: fn(&mut World, &mut UI),
    (w, h, dt): (f32, f32, f32),
) {
    let (cursor, edit) = world
        .resource::<Input>()
        .map(|i| (i.pointer(), i.edit_input()))
        .unwrap_or_default();
    if let Ok(sprites) = world.resource::<Sprites>() {
        ui.set_sprites(sprites.shared());
    }
    ui.begin(w, h, dt, cursor);
    ui.set_edit(edit);
    frame_ui(world, ui);
    ui.finish();
    if let Some(text) = ui.take_clipboard() {
        crate::clipboard::set(&text);
    }
}
