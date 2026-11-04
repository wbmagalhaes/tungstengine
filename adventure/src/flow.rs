use crate::AdventureAction;
use crate::settings::Settings;
use glam::Quat;
use tungstengine::audio::Volume;
use tungstengine::input::InputWorld;
use tungstengine::menus::{self, GameMenu};
use winit::event::MouseButton;

use tungstengine::dialog::Dialog;
use tungstengine::ecs::prelude::*;
use tungstengine::input::Input;
use tungstengine::runtime::d3::{AppControl, WindowConfig};
use tungstengine::scene::{self};
use tungstengine::ui::UI;

use super::menu::{MenuAction, MenuState};
use super::{
    ActionMenu, Debug, Destination, InputCapture, Inventory, Pending, Player, Screen, debug,
    dialog_ui, hud, inspect, interact, menu,
};

pub fn frame_ui(world: &mut World, ui: &mut UI) {
    match world.resource::<Screen>().copied().unwrap_or_default() {
        Screen::Menu => menu_screen(world, ui),
        Screen::Running => game_screen(world, ui),
    }
}

fn menu_screen(world: &mut World, ui: &mut UI) {
    let dt = world.resource::<Time>().map(|t| t.dt).unwrap_or(0.0);
    let intro = world
        .resource_mut::<MenuState>()
        .map(|ms| ms.intro.tick(dt))
        .unwrap_or(1.0);

    let fallback = crate::AdventureTheme::default();
    let theme = world
        .resource::<crate::AdventureTheme>()
        .unwrap_or(&fallback);
    let labels = (crate::tr(world, "menu.play"), crate::tr(world, "menu.quit"));
    match menu::build(ui, theme, intro, (&labels.0, &labels.1)) {
        MenuAction::Play => {
            if let Ok(screen) = world.resource_mut::<Screen>() {
                *screen = Screen::Running;
            }
        }
        MenuAction::Quit => {
            if let Ok(control) = world.resource_mut::<AppControl>() {
                control.should_exit = true;
            }
        }
        MenuAction::None => {}
    }
}

fn game_screen(world: &mut World, ui: &mut UI) {
    scene::change_scene(world);

    let input = world.resource::<Input>();
    let (click, held, cursor, scroll) = input
        .map(|i| {
            (
                i.mouse_released(MouseButton::Left),
                i.mouse_held(MouseButton::Left),
                i.cursor,
                i.scroll(),
            )
        })
        .unwrap_or((false, false, (0.0, 0.0), 0.0));
    let back = world.input().pressed(AdventureAction::Menu);
    let do_save = world.input().pressed(AdventureAction::Save);
    let do_load = world.input().pressed(AdventureAction::Load);
    let toggle_debug = world.input().pressed(AdventureAction::Debug);

    dialog_ui::apply_reveals(world);
    dialog_ui::apply_flags(world);
    dialog_ui::apply_sfx(world);
    dialog_ui::apply_scene(world);
    let dialog_blocked = dialog_ui::dialog_ui(world, ui, click);

    if inspect::active(world) {
        inspect::update(
            world,
            ui,
            &inspect::InspectInput {
                cursor,
                click,
                held,
                scroll,
                esc: back,
                blocked: dialog_blocked,
            },
        );
        return;
    }

    let captured = world
        .resource::<InputCapture>()
        .map(|c| c.0)
        .unwrap_or(false);

    if back
        && !captured
        && !dialog_blocked
        && let Ok(m) = world.resource_mut::<GameMenu>()
    {
        m.toggle();
    }
    if world.resource::<GameMenu>().is_ok_and(GameMenu::is_open) {
        pause_menu(world, ui);
        return;
    }
    if toggle_debug && let Ok(d) = world.resource_mut::<Debug>() {
        d.enabled = !d.enabled;
    }
    if do_save {
        let _ = tungstengine::save::save(world);
    }
    if do_load {
        tungstengine::save::load(world);
    }

    if dialog_blocked {
        return;
    }

    interact::run_pending(world);

    hud::action_menu(world, ui);
    hud::inventory_bar(world, ui);
    let blocked = ui.pointer_used() || captured;
    hud::world_click(world, click && !blocked, cursor);
    hud::world_hold(world, held && !blocked);

    if !blocked {
        hud::hover_label(world, ui, cursor);
    }
    hud::hint(world, ui);
    debug::debug_draw(world, ui);
}

fn pause_menu(world: &mut World, ui: &mut UI) {
    let settings_file = world
        .resource::<GameMenu>()
        .map_or_else(|_| String::new(), |m| m.settings_key.clone());
    let (sfx, bgm) = world
        .resource::<Settings>()
        .map_or((1.0, 1.0), |s| (s.sfx_volume, s.bgm_volume));
    let mut volume = Volume { sfx, bgm };
    let mut fullscreen = world.resource::<WindowConfig>().is_ok_and(|c| c.fullscreen);
    let out = menus::draw(world, ui, &mut volume, &mut fullscreen);
    if out.settings_changed
        && let Ok(s) = world.resource_mut::<Settings>()
    {
        s.sfx_volume = volume.sfx;
        s.bgm_volume = volume.bgm;
    }
    if out.fullscreen_toggled {
        if let Ok(c) = world.resource_mut::<WindowConfig>() {
            c.fullscreen = fullscreen;
        }
        if let Ok(c) = world.resource_mut::<AppControl>() {
            c.fullscreen = Some(fullscreen);
        }
    }
    if (out.settings_changed || out.fullscreen_toggled)
        && let Err(e) =
            tungstengine::settings::GameSettings::new(volume, fullscreen).save(&settings_file)
    {
        log::error!("settings: {e}");
    }
    if out.quit {
        if let Ok(m) = world.resource_mut::<GameMenu>() {
            m.close();
        }
        back_to_menu(world);
    }
}

fn back_to_menu(world: &mut World) {
    if let Ok(screen) = world.resource_mut::<Screen>() {
        *screen = Screen::Menu;
    }
    if let Ok(ms) = world.resource_mut::<MenuState>() {
        ms.intro.reset();
    }
    for (_e, t, p) in world.query_mut::<(&mut Transform, &Player)>() {
        t.position = p.home;
        t.rotation = Quat::IDENTITY;
    }
    if let Ok(d) = world.resource_mut::<Destination>() {
        d.path.clear();
    }
    if let Ok(m) = world.resource_mut::<ActionMenu>() {
        m.open = false;
    }
    if let Ok(inv) = world.resource_mut::<Inventory>() {
        inv.selected = None;
    }
    if let Ok(d) = world.resource_mut::<Dialog>() {
        d.active = false;
    }
    if let Ok(p) = world.resource_mut::<Pending>() {
        p.target = None;
    }
}
