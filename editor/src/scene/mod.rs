mod camera;
mod dialog;
mod document;
mod files;
mod gizmo;
mod grid;
mod inspector;
mod menu;
mod node;
mod pick;
mod state;
mod toolbar;
mod tree;
mod viewport;

use std::path::{Path, PathBuf};

use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use tungstengine::ecs::prelude::*;
use tungstengine::input::Input;
use tungstengine::render::d3::prelude::*;
use tungstengine::scene;
use tungstengine::ui::prelude::*;

use files::{Modal, Outcome};
use gizmo::Tool;
use menu::{Cmd, FileOp};
use state::{DocKind, EditorState, FrameInput, PREFABS_DIR, file_name, file_stem};

pub struct EditorConfig {
    pub register: fn(&mut scene::SpawnRegistry),
    pub start_scene: &'static str,
}

pub fn run(config: EditorConfig) -> Result<(), winit::error::EventLoopError> {
    let mut world = World::new();
    setup(&mut world, config);
    let mut shell = crate::EditorShell::default();
    shell.add("Scene", frame_ui);
    world.insert_resource(shell);
    tungstengine::runtime::d3::App::new(world, crate::shell::frame).run()
}

pub fn setup(world: &mut World, config: EditorConfig) {
    let mut spawn_reg = scene::SpawnRegistry::default();
    scene::register_builtin(&mut spawn_reg);
    tungstengine_adventure::scene_spawn::register(&mut spawn_reg);
    (config.register)(&mut spawn_reg);
    world.insert_resource(spawn_reg);
    world.insert_resource(scene::DeferredSpawns::default());

    grid::setup(world);

    let mut meshes = MeshRegistry::new();
    tungstengine_adventure::load_models(&mut meshes, Path::new("assets/models"));
    world.insert_resource(meshes);
    world.insert_resource(tungstengine::texture::load_textures(Path::new(
        "assets/textures",
    )));
    world.insert_resource(scene::load_materials(Path::new("assets/materials")));

    let registry = scene::load_prefabs(Path::new(PREFABS_DIR));
    let gameobjs = registry.list();
    world.insert_resource(registry);
    scene::spawn_object(world, &scene::camera([0.0, 16.0, 11.0], [0.0, 0.0, 0.0]));
    world.insert_resource(EditorState::new(gameobjs));
    camera::setup(world);

    let start = Path::new(config.start_scene);
    if start.exists() {
        document::open_scene(world, start);
    } else {
        document::new_scene(world);
    }
}

fn set_status(world: &mut World, msg: String) {
    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.status = msg;
    }
}

fn set_modal(world: &mut World, modal: Modal) {
    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.modal = Some(modal);
    }
}

fn show_dialog(world: &mut World, on: bool) {
    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.dialog_mode = on;
    }
}

fn current_kind(world: &World) -> DocKind {
    match world.resource::<EditorState>() {
        Ok(s) if s.dialog_mode => DocKind::Dialog,
        Ok(s) => s.doc,
        Err(_) => DocKind::Scene,
    }
}

fn is_open(world: &World, kind: DocKind) -> bool {
    match kind {
        DocKind::Dialog => dialog::is_loaded(world),
        _ => world.resource::<EditorState>().is_ok_and(|s| s.doc == kind),
    }
}

fn current_path(world: &World, kind: DocKind) -> Option<PathBuf> {
    match kind {
        DocKind::Dialog => dialog::path(world),
        _ => world
            .resource::<EditorState>()
            .ok()
            .filter(|s| s.doc == kind)
            .and_then(|s| s.path.clone()),
    }
}

fn save(world: &mut World, kind: DocKind, path: PathBuf, save_as: bool) {
    match kind {
        DocKind::Scene => document::save_scene(world, path),
        DocKind::Prefab => document::save_prefab(world, path, save_as),
        DocKind::Dialog => dialog::save(world, path),
    }
}

fn file_op(world: &mut World, kind: DocKind, op: FileOp) {
    let open = is_open(world, kind);
    match op {
        FileOp::Open => set_modal(world, Modal::Open(kind, files::list(kind))),
        FileOp::New => match kind {
            DocKind::Scene => document::new_scene(world),
            DocKind::Prefab => document::new_prefab(world),
            DocKind::Dialog => {
                dialog::new_doc(world);
                show_dialog(world, true);
            }
        },
        FileOp::Save | FileOp::SaveAs if !open => {
            set_status(world, format!("no {} open", kind.title().to_lowercase()))
        }
        FileOp::Save => match current_path(world, kind) {
            Some(p) => save(world, kind, p, false),
            None => file_op(world, kind, FileOp::SaveAs),
        },
        FileOp::SaveAs => {
            let stem = file_stem(current_path(world, kind).as_deref());
            set_modal(world, Modal::SaveAs(kind, stem));
        }
    }
}

fn run_cmd(world: &mut World, cmd: Cmd) {
    match cmd {
        Cmd::File(kind, op) => file_op(world, kind, op),
        Cmd::SaveCurrent => file_op(world, current_kind(world), FileOp::Save),
        Cmd::SaveCurrentAs => file_op(world, current_kind(world), FileOp::SaveAs),
        Cmd::AddObject => {
            let objs = world
                .resource::<EditorState>()
                .map(|s| s.gameobjs.clone())
                .unwrap_or_default();
            set_modal(world, Modal::AddObject(objs));
        }
        Cmd::Delete => document::delete_selected(world),
        Cmd::Focus => {
            let target = world
                .resource::<EditorState>()
                .ok()
                .and_then(node::selected)
                .and_then(|n| node::world_xf(world, &n))
                .map(|xf| xf.pos);
            if let Some(t) = target {
                camera::focus(world, t);
            }
        }
        Cmd::Tool(t) => {
            if let Ok(s) = world.resource_mut::<EditorState>()
                && s.gizmo.is_none()
            {
                s.tool = t;
                s.paint_mode = false;
                if let Some(op) = s.modal_op.as_mut() {
                    op.tool = t;
                }
            }
        }
        Cmd::Axis(i) => viewport::constrain(world, i),
        Cmd::TogglePaint => {
            if let Ok(s) = world.resource_mut::<EditorState>()
                && s.doc == DocKind::Scene
            {
                s.paint_mode = !s.paint_mode;
                s.gizmo = None;
            }
        }
    }
}

fn apply_outcome(world: &mut World, outcome: Outcome) {
    match outcome {
        Outcome::Open(DocKind::Scene, p) => document::open_scene(world, &p),
        Outcome::Open(DocKind::Prefab, p) => document::open_prefab(world, &p),
        Outcome::Open(DocKind::Dialog, p) => match dialog::open(world, &p) {
            Ok(_) => show_dialog(world, true),
            Err(e) => set_status(world, format!("error: {e}")),
        },
        Outcome::SaveAs(kind, p) => save(world, kind, p, true),
        Outcome::Add(id) => document::add_object(world, id),
        Outcome::AddComponent(tag) => inspector::add_component(world, &tag),
        Outcome::Cancel => {}
    }
}

fn shortcuts(world: &World, dialog_mode: bool) -> Option<Cmd> {
    let i = world.resource::<Input>().ok()?;
    let ctrl = i.key_held(KeyCode::ControlLeft) || i.key_held(KeyCode::ControlRight);
    let shift = i.key_held(KeyCode::ShiftLeft) || i.key_held(KeyCode::ShiftRight);
    if ctrl {
        return if i.key_pressed(KeyCode::KeyS) && shift {
            Some(Cmd::SaveCurrentAs)
        } else if i.key_pressed(KeyCode::KeyS) {
            Some(Cmd::SaveCurrent)
        } else if i.key_pressed(KeyCode::KeyO) {
            Some(Cmd::File(current_kind(world), FileOp::Open))
        } else {
            None
        };
    }
    if dialog_mode || i.mouse_held(MouseButton::Right) {
        return None;
    }
    [
        (KeyCode::Delete, Cmd::Delete),
        (KeyCode::KeyF, Cmd::Focus),
        (KeyCode::KeyW, Cmd::Tool(Tool::Move)),
        (KeyCode::KeyE, Cmd::Tool(Tool::Scale)),
        (KeyCode::KeyR, Cmd::Tool(Tool::Rotate)),
        (KeyCode::KeyX, Cmd::Axis(0)),
        (KeyCode::KeyY, Cmd::Axis(1)),
        (KeyCode::KeyZ, Cmd::Axis(2)),
        (KeyCode::KeyP, Cmd::TogglePaint),
    ]
    .into_iter()
    .find(|(k, _)| i.key_pressed(*k))
    .map(|(_, c)| c)
}

fn doc_title(world: &World) -> String {
    let Ok(s) = world.resource::<EditorState>() else {
        return String::new();
    };
    if s.dialog_mode {
        let p = dialog::path(world);
        return format!("Dialog - {}", file_name(p.as_deref()));
    }
    format!("{} - {}", s.doc.title(), file_name(s.path.as_deref()))
}

pub fn frame_ui(world: &mut World, ui: &mut UI) {
    let Ok(s) = world.resource::<EditorState>() else {
        return;
    };
    let (menu_state, dialog_mode, modal_open) = (s.menu, s.dialog_mode, s.modal.is_some());
    let Ok(i) = world.resource::<Input>() else {
        return;
    };
    let cursor = i.cursor;
    let (enter, escape) = (
        i.key_pressed(KeyCode::Enter),
        i.key_pressed(KeyCode::Escape),
    );
    let wheel = i.scroll();
    let inp = FrameInput {
        pressed: i.mouse_pressed(MouseButton::Left),
        held: i.mouse_held(MouseButton::Left),
        released: i.mouse_released(MouseButton::Left),
        cursor,
        scroll: i.scroll(),
        menu_covers: menu_state.open.is_some() || menu::covers(ui, &menu_state, cursor),
        modal: modal_open,
        enter,
        escape,
        rmb_pressed: i.mouse_pressed(MouseButton::Right),
    };

    let key_cmd = if modal_open {
        None
    } else {
        shortcuts(world, dialog_mode || inspector::has_focus(world))
    };
    if !modal_open {
        menu::block(ui, &menu_state);
    }

    let mut cmd = None;
    if dialog_mode {
        dialog::ensure(world);
        if dialog::frame(world, ui) {
            show_dialog(world, false);
        }
    } else {
        cmd = viewport::frame(world, ui, &inp);
    }

    let title = doc_title(world);
    let (tool, paint_mode) = world
        .resource::<EditorState>()
        .map(|s| (s.tool, s.paint_mode))
        .unwrap_or_default();
    let checked = |c: Cmd| match c {
        Cmd::Tool(t) => !paint_mode && tool == t,
        Cmd::TogglePaint => paint_mode,
        _ => false,
    };
    let mut menu_state = menu_state;
    let menu_cmd = menu::bar(ui, &mut menu_state, &title, checked).filter(|_| !modal_open);

    let mut modal = world
        .resource_mut::<EditorState>()
        .ok()
        .and_then(|s| s.modal.take());
    let outcome = modal
        .as_mut()
        .and_then(|m| files::frame(ui, m, enter, escape, wheel));
    if let Ok(s) = world.resource_mut::<EditorState>() {
        s.menu = menu_state;
        if outcome.is_none() {
            s.modal = modal;
        }
    }
    if let Some(o) = outcome {
        apply_outcome(world, o);
    }
    if let Some(c) = menu_cmd.or(cmd).or(key_cmd) {
        run_cmd(world, c);
    }
}
