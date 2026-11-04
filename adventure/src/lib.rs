mod components;

mod flow;

pub mod debug;
pub mod dialog_ui;
pub mod hud;
pub mod inspect;
pub mod interact;
pub mod menu;
pub mod save_defaults;
pub mod theme;

pub use theme::AdventureTheme;

use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use tungstengine::input::{Action, Binding, InputWorld, PadButton};

use tungstengine::manifest::GameFiles;

#[derive(Action, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AdventureAction {
    Menu,
    Save,
    Load,
    Debug,
    Examine,
}

pub fn boot(
    world: &mut World,
    assets: tungstengine::manifest::GameAssets,
    register: impl FnOnce(&mut World),
) {
    let manifest = tungstengine::manifest::Manifest::parse(assets.manifest);
    world.insert_resource(manifest.files.clone());
    setup(
        world,
        &manifest.title,
        &manifest.assets,
        &manifest.main_scene,
        register,
    );
    if let Some(color) = manifest.clear_color() {
        world.insert_resource(tungstengine::render::ClearColor(color));
    }
    if let Some(mask) = &manifest.transition_mask {
        let path = Path::new(&manifest.assets).join(mask);
        match tungstengine::texture::Texture2D::load(&path) {
            Some(texture) => {
                world.insert_resource(tungstengine::render::d3::TransitionMask(texture))
            }
            None => log::error!("transition mask `{}`: failed to load", path.display()),
        }
    }
    if let Ok(loc) = world.resource_mut::<tungstengine::localization::Localization>() {
        for (locale, text) in assets.locales {
            if let Err(e) = loc.load_str(locale, text) {
                log::error!("locale {locale}: {e}");
            }
        }
        loc.set_locale(&manifest.locale);
    }
}

pub fn run(
    setup: fn(&mut World),
    frame: tungstengine::runtime::d3::FrameUi,
) -> Result<(), winit::error::EventLoopError> {
    let _ = env_logger::try_init();
    let mut world = World::new();
    setup(&mut world);
    world.insert_resource(Debug {
        enabled: std::env::args().any(|a| a == "--dbg" || a == "--debug"),
        ..Default::default()
    });
    tungstengine::runtime::d3::App::new(world, frame).run()
}

fn game_files(world: &World) -> GameFiles {
    world.resource::<GameFiles>().cloned().unwrap_or_default()
}

pub fn tr(world: &World, key: &str) -> String {
    world
        .resource::<tungstengine::localization::Localization>()
        .map_or_else(|_| key.to_string(), |l| l.t(key))
}

pub fn tr_args(world: &World, key: &str, args: &[(&str, &str)]) -> String {
    world
        .resource::<tungstengine::localization::Localization>()
        .map_or_else(|_| key.to_string(), |l| l.tf(key, args))
}

pub mod scene_spawn;
pub mod settings;
pub mod sfx;
pub mod systems;

pub use components::{
    ActionMenu, ChangeSceneTrigger, CursorTag, Debug, Destination, DisableOnFlag, DisplayName,
    EnableOnFlag, Examinable, Flags, Hotspot, Ident, InputCapture, Inspect, Interactable,
    Inventory, Item, LookOnFlag, Pending, PlayOnFlag, Player, SfxOnFlag, SpawnOnFlag, Verb,
    VerbAction, VerbEntry, WalkMarkTag, input_captured,
};
pub use flow::frame_ui;
pub use scene_spawn::{VERB_ACTIONS, verb_defaults};

use std::fs;
use std::path::Path;

use glam::Vec3;

use crate::menu::MenuState;
use tungstengine::audio::Audio;
use tungstengine::color::Color;
use tungstengine::dialog::Dialog;
use tungstengine::ecs::prelude::*;
use tungstengine::render::d3::{
    GfxConfig, Material, Mesh, MeshRegistry, PostRegistry, Renderer, ShaderRegistry,
};
use tungstengine::rules;
use tungstengine::runtime::d3::WindowConfig;
use tungstengine::scene::{self, Materials, Prefabs, Scenes};
use tungstengine::sprites::load_sprites;
use tungstengine::texture::{Textures, load_textures};
use tungstengine::{animation, ui};

use self::settings::Settings;

#[derive(Resource, Default, Clone, Copy, PartialEq)]
pub enum Screen {
    #[default]
    Menu,
    Running,
}

pub fn load_models(reg: &mut MeshRegistry, dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            load_models(reg, &path);
        } else if path.extension().and_then(|e| e.to_str()) == Some("glb")
            && let Some(s) = path.to_str()
            && reg.load(s).is_none()
        {
            log::error!("failed to load model {}", path.display());
        }
    }
}

pub fn setup(
    world: &mut World,
    title: &str,
    assets: &str,
    main_scene: &str,
    register: impl FnOnce(&mut World),
) {
    world.insert_resource(WindowConfig {
        title: title.to_string(),
        fullscreen: false,
    });
    world.init_state::<Screen>();
    let mut loc = tungstengine::localization::Localization::new("en");
    tungstengine::menus::install_locale(&mut loc);
    if let Err(e) = loc.load_str("en", include_str!("../assets/locale/en.toml")) {
        log::error!("adventure locale: {e}");
    }
    world.insert_resource(loc);

    let files = game_files(world);
    world.insert_resource(tungstengine::menus::GameMenu::new(
        &files.bindings,
        &files.settings,
        tungstengine::menus::MenuStyle::default(),
    ));
    world
        .input_mut()
        .actions
        .declare(
            AdventureAction::Menu,
            &[
                Binding::Key(KeyCode::Escape),
                Binding::Pad(PadButton::Start),
            ],
        )
        .declare(AdventureAction::Save, &[Binding::Key(KeyCode::F5)])
        .declare(AdventureAction::Load, &[Binding::Key(KeyCode::F9)])
        .declare(AdventureAction::Debug, &[Binding::Key(KeyCode::F1)])
        .declare(
            AdventureAction::Examine,
            &[Binding::Mouse(MouseButton::Right)],
        );

    load_assets(world, assets);
    register_systems(world);
    spawn_cursor(world);

    if cfg!(debug_assertions) {
        world.insert_resource(scene::SceneWatch::new(assets));
        world.add_system_if(in_state(Screen::Running), scene::hot_reload);
    }

    register(world);

    let main = world
        .resource::<Scenes>()
        .ok()
        .and_then(|s| s.path(main_scene).map(Path::to_path_buf));
    if let Some(path) = main {
        scene::load_scene(world, &path);
    }
}

fn register_systems(world: &mut World) {
    world.insert_resource(Destination::default());
    world.insert_resource(ActionMenu::default());
    world.insert_resource(Inventory::default());
    world.insert_resource(Flags::default());
    world.insert_resource(Pending::default());
    world.insert_resource(MenuState::default());
    world.insert_resource(scene::DeferredSpawns::default());
    world.insert_resource(interact::Interactions::default());
    world.insert_resource(InputCapture::default());
    world.insert_resource(Inspect::default());
    world.insert_resource(tungstengine::render::d3::OverlayHook {
        scene: inspect::scene,
        intro: inspect::intro,
    });

    let mut spawn_reg = scene::SpawnRegistry::default();
    scene::register_builtin(&mut spawn_reg);
    scene_spawn::register(&mut spawn_reg);
    world.insert_resource(spawn_reg);

    let mut saves = tungstengine::save::SaveRegistry::default();
    save_defaults::register_defaults(&mut saves);
    world.insert_resource(saves);

    let run = || in_state(Screen::Running);
    world.add_system_if(run(), systems::update_cursor);
    world.add_system_if(run(), systems::rasterize_colliders);
    world.add_system_if(run(), systems::walk);
    world.add_system_if(run(), systems::collide_slide);
    world.add_system_if(run(), systems::walkmark_visibility);
    world.add_system_if(run(), systems::hover_outline);
    world.add_system_if(run(), systems::scene_trigger);
    world.add_system_if(run(), systems::play_on_flag);
    world.add_system_if(run(), systems::spawn_on_flag);
    world.add_system_if(run(), systems::disable_on_flag);
    world.add_system_if(run(), scene::activate_deferred);
    world.add_system_if(run(), systems::look_on_flag);
    world.add_system_if(run(), systems::sfx_on_flag);
    world.add_system_if(run(), systems::player_animation);
    world.add_system_if(run(), animation::animate);
    world.add_system_if(run(), animation::animator_update);
    world.add_system_if(run(), sfx::drain_sounds);
    world.add_system_if(run(), systems::camera_follow);
}

fn load_assets(world: &mut World, assets: &str) {
    let p = |rel: &str| format!("{assets}/{rel}");

    world.insert_resource(Dialog::load(
        Path::new(&p("dialogs")),
        Path::new(&p("config/actors.toml")),
    ));
    let settings = Settings::load(Path::new(&p("config/settings.toml")));
    world.insert_resource(GfxConfig {
        shadow_size: settings.shadow_quality.resolution(),
        msaa: settings.msaa.samples(),
    });
    let mut settings = settings;
    let settings_file = game_files(world).settings;
    if tungstengine::storage::read(&settings_file).is_some() {
        let saved = tungstengine::settings::GameSettings::load(&settings_file);
        settings.sfx_volume = saved.sfx;
        settings.bgm_volume = saved.bgm;
        if let Ok(c) = world.resource_mut::<WindowConfig>() {
            c.fullscreen = saved.fullscreen;
        }
    }
    world.insert_resource(settings);
    let theme = crate::AdventureTheme::load(Path::new(&p("config/ui.toml")));
    let resolve_font = |rel: &str| {
        if Path::new(rel).is_absolute() {
            rel.to_string()
        } else {
            p(rel)
        }
    };
    ui::configure_fonts(
        theme.font_sans.as_deref().map(resolve_font),
        theme.font_mono.as_deref().map(resolve_font),
    );
    world.insert_resource(theme);

    let mut meshes = MeshRegistry::new();
    load_models(&mut meshes, Path::new(&p("models")));
    world.insert_resource(meshes);

    world.insert_resource(ShaderRegistry::new());
    world.insert_resource(PostRegistry::default());
    world.insert_resource(scene::load_post_profiles(Path::new(&p("post"))));
    world.insert_resource(scene::load_scenes(Path::new(&p("config/scenes.toml"))));
    world.insert_resource(rules::load_rules(Path::new(&p("config/rules.toml"))));
    if let Some(audio) = Audio::new() {
        world.insert_resource(audio);
    }

    world.insert_resource(load_textures(Path::new(&p("textures"))));
    world.insert_resource(load_sprites(Path::new(&p("sprites"))));
    world.insert_resource(scene::load_materials(Path::new(&p("materials"))));
    world.insert_resource(scene::load_prefabs(Path::new(&p("prefabs"))));

    if let Ok(materials) = world.resource::<Materials>() {
        log::debug!(
            "materials ({}): {}",
            materials.names().len(),
            materials.names().join(", ")
        );
    }
    if let Ok(scenes) = world.resource::<Scenes>() {
        log::debug!("scenes: {}", scenes.names().join(", "));
    }
    if let Ok(prefabs) = world.resource::<Prefabs>() {
        let names: Vec<String> = prefabs.list().into_iter().map(|(_, n)| n).collect();
        log::debug!("prefabs ({}): {}", names.len(), names.join(", "));
    }
    if let Ok(textures) = world.resource::<Textures>() {
        log::debug!(
            "textures ({}): {}",
            textures.names().len(),
            textures.names().join(", ")
        );
    }
}

fn spawn_cursor(world: &mut World) {
    let cursor = world.spawn();
    world.insert(cursor, Transform::from_scale(Vec3::new(0.6, 0.12, 0.6)));
    world.insert(
        cursor,
        Renderer::new(Mesh::DISK, Material::UNLIT.color(Color::TEXT_TITLE)).no_outline_occlude(),
    );
    world.insert(cursor, CursorTag);

    let mark = world.spawn();
    world.insert(mark, Transform::from_scale(Vec3::ZERO));
    world.insert(
        mark,
        Renderer::new(Mesh::DISK, Material::UNLIT.color(Color::EMERALD_300)).no_outline_occlude(),
    );
    world.insert(mark, WalkMarkTag);
}

pub mod prelude {
    pub use super::components::{
        ActionMenu, ChangeSceneTrigger, CursorTag, Debug, Destination, DisableOnFlag, DisplayName,
        EnableOnFlag, Examinable, Flags, Hotspot, Ident, Inspect, Interactable, Inventory, Item,
        LookOnFlag, Pending, PlayOnFlag, Player, SfxOnFlag, SpawnOnFlag, Verb, VerbAction,
        VerbEntry, WalkMarkTag,
    };
}

#[macro_export]
macro_rules! boot {
    ($world:expr, $register:expr) => {
        $crate::boot($world, ::tungstengine::manifest::embed_game!(), $register)
    };
    ($world:expr, $manifest:literal, $register:expr) => {
        $crate::boot(
            $world,
            ::tungstengine::manifest::embed_game!($manifest),
            $register,
        )
    };
}
