use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use glam::Vec3;
use uuid::Uuid;

use tungstengine::ecs::prelude::*;
use tungstengine::nav::WalkGrid;
use tungstengine::scene::{GameObjDef, SceneHeader};

use super::files::Modal;
use super::gizmo::{Op, Tool};
use super::menu::MenuState;

pub const SCENES_DIR: &str = "assets/scenes";
pub const PREFABS_DIR: &str = "assets/prefabs";
pub const DIALOGS_DIR: &str = "assets/dialogs";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    Scene,
    Prefab,
    Dialog,
}

impl DocKind {
    pub fn dir(self) -> &'static str {
        match self {
            DocKind::Scene => SCENES_DIR,
            DocKind::Prefab => PREFABS_DIR,
            DocKind::Dialog => DIALOGS_DIR,
        }
    }

    pub fn ext(self) -> &'static str {
        match self {
            DocKind::Dialog => "dialog",
            _ => "toml",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            DocKind::Scene => "Scene",
            DocKind::Prefab => "Prefab",
            DocKind::Dialog => "Dialog",
        }
    }

    pub fn path_for(self, name: &str) -> PathBuf {
        let ext = format!(".{}", self.ext());
        let stem = name.trim().trim_end_matches(ext.as_str());
        Path::new(self.dir()).join(format!("{stem}{ext}"))
    }
}

pub fn file_name(path: Option<&Path>) -> String {
    path.and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed".into())
}

pub fn file_stem(path: Option<&Path>) -> String {
    path.and_then(|p| p.file_stem())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[derive(Clone)]
pub struct Placed {
    pub entity: Entity,
    pub def: GameObjDef,
    pub mesh: Option<String>,
    pub pos: Vec3,
    pub rot: Vec3,
    pub scale: Vec3,
}

pub struct FrameInput {
    pub pressed: bool,
    pub held: bool,
    pub released: bool,
    pub cursor: (f32, f32),
    pub scroll: f32,
    pub menu_covers: bool,
    pub modal: bool,
    pub enter: bool,
    pub escape: bool,
    pub rmb_pressed: bool,
}

#[derive(Resource)]
pub struct EditorState {
    pub gameobjs: Vec<(Uuid, String)>,
    pub doc: DocKind,
    pub path: Option<PathBuf>,
    pub header: SceneHeader,
    pub placed: Vec<Placed>,
    pub reserved: Vec<GameObjDef>,
    pub selected: Option<usize>,
    pub sel_path: Vec<usize>,
    pub status: String,
    pub dialog_mode: bool,
    pub grid: WalkGrid,
    pub paint_mode: bool,
    pub paint_value: bool,
    pub painting: bool,
    pub last_cell: Option<(usize, usize)>,
    pub markers: HashMap<(usize, usize), Entity>,
    pub tool: Tool,
    pub gizmo: Option<Op>,
    pub modal_op: Option<Op>,
    pub tree_scroll: f32,
    pub inspector_scroll: f32,
    pub field_buf: Option<(u64, String)>,
    pub inspector_locks: HashSet<u64>,
    pub hidden_gizmos: HashSet<String>,
    pub inspector_open: HashSet<u64>,
    pub expanded: HashSet<Vec<usize>>,
    pub menu: MenuState,
    pub modal: Option<Modal>,
}

pub fn default_grid() -> WalkGrid {
    WalkGrid::new(1.0, 24, 24, [-12.0, -12.0])
}

impl EditorState {
    pub fn new(gameobjs: Vec<(Uuid, String)>) -> Self {
        EditorState {
            gameobjs,
            doc: DocKind::Scene,
            path: None,
            header: SceneHeader::default(),
            placed: Vec::new(),
            reserved: Vec::new(),
            selected: None,
            sel_path: Vec::new(),
            status: String::new(),
            dialog_mode: false,
            grid: default_grid(),
            paint_mode: false,
            paint_value: false,
            painting: false,
            last_cell: None,
            markers: HashMap::new(),
            tool: Tool::default(),
            gizmo: None,
            modal_op: None,
            tree_scroll: 0.0,
            inspector_scroll: 0.0,
            field_buf: None,
            inspector_locks: HashSet::new(),
            hidden_gizmos: HashSet::new(),
            inspector_open: HashSet::new(),
            expanded: HashSet::new(),
            menu: MenuState::default(),
            modal: None,
        }
    }
}
