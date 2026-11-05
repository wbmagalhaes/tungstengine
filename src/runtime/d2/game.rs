use crate::audio::{EngineSound, Sound, Volume, drain_sounds};
use crate::color::Color;
use crate::ecs::prelude::*;
use crate::localization::Localization;
use crate::manifest::{GameAssets, Manifest};
use crate::menus::{self, GameMenu, MenuStyle};
use crate::render::ClearColor;
use crate::runtime::d2::app::SetupFn;
use crate::settings::GameSettings;
use crate::ui::{PixelPerfect, UI};

use super::{App, AppControl, FrameUi, WindowConfig};

pub fn boot(world: &mut World, assets: GameAssets) {
    let manifest = Manifest::parse(assets.manifest);
    let settings = GameSettings::load(&manifest.files.settings);
    world.insert_resource(WindowConfig {
        title: manifest.title.clone(),
        width: manifest.width,
        height: manifest.height,
        fullscreen: settings.fullscreen,
    });
    if let Some(color) = manifest.clear_color() {
        world.insert_resource(ClearColor(color));
    }
    world.insert_resource(PixelPerfect(manifest.pixel_perfect));
    world.insert_resource(settings.volume());
    world.insert_resource(crate::rng::Rng::default());

    let mut loc = Localization::new("en");
    menus::install_locale(&mut loc);
    for (locale, text) in assets.locales {
        if let Err(e) = loc.load_str(locale, text) {
            log::error!("locale {locale}: {e}");
        }
    }
    loc.set_locale(&manifest.locale);
    world.insert_resource(loc);

    world.insert_resource(GameMenu::new(
        &manifest.files.bindings,
        &manifest.files.settings,
        menu_style(&manifest),
    ));
    world.insert_resource(manifest.files.clone());
    world.add_system(drain_sounds);
}

pub fn run(setup: SetupFn, frame: FrameUi) -> Result<(), winit::error::EventLoopError> {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = env_logger::try_init();
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        let _ = console_log::init_with_level(log::Level::Warn);
    }
    let mut world = World::new();
    setup(&mut world);
    App::new(world, frame).run()
}

fn menu_style(manifest: &Manifest) -> MenuStyle {
    let base = MenuStyle::default();
    let color = |hex: &Option<String>, fallback| hex.as_deref().map_or(fallback, Color::hex);
    MenuStyle {
        card: color(&manifest.menu.card, base.card),
        border: color(&manifest.menu.border, base.border),
        title: color(&manifest.menu.title, base.title),
        z: base.z,
    }
}

pub fn pause_menu(world: &mut World, ui: &mut UI, toggle: bool, allowed: bool) {
    let Ok(menu) = world.resource_mut::<GameMenu>() else {
        return;
    };
    if !allowed {
        menu.close();
        return;
    }
    if toggle {
        menu.toggle();
    }
    let key = menu.settings_key.clone();

    let mut volume = world.resource::<Volume>().copied().unwrap_or_default();
    let mut fullscreen = world
        .resource::<WindowConfig>()
        .map(|c| c.fullscreen)
        .unwrap_or(true);
    let out = menus::draw(world, ui, &mut volume, &mut fullscreen);

    if out.settings_changed {
        world.insert_resource(volume);
    }
    if out.fullscreen_toggled {
        if let Ok(c) = world.resource_mut::<WindowConfig>() {
            c.fullscreen = fullscreen;
        }
        if let Ok(c) = world.resource_mut::<AppControl>() {
            c.fullscreen = Some(fullscreen);
        }
    }
    if out.settings_changed || out.fullscreen_toggled {
        if let Err(e) = GameSettings::new(volume, fullscreen).save(&key) {
            log::error!("settings: {e}");
        }
        EngineSound::Pick.play();
    }
    if out.quit
        && let Ok(c) = world.resource_mut::<AppControl>()
    {
        c.should_exit = true;
    }
}
