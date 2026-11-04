use std::collections::HashSet;

use glam::Vec3;

use tungstengine::animation::{Animation, Ease, Tween};
use tungstengine::dialog::Actors;
use tungstengine::ecs::prelude::*;

#[derive(Component, Clone)]
pub enum DisplayName {
    Fixed(String),
    Hidden {
        key: Option<String>,
        unknown: String,
        name: String,
        known: bool,
    },
    Actor(String),
}

impl DisplayName {
    pub fn reveal(&mut self) {
        if let DisplayName::Hidden { known, .. } = self {
            *known = true;
        }
    }

    pub fn reveal_if(&mut self, id: &str) {
        if let DisplayName::Hidden { key, known, .. } = self
            && key.as_deref() == Some(id)
        {
            *known = true;
        }
    }

    pub fn label<'a>(&'a self, actors: &'a Actors) -> &'a str {
        match self {
            DisplayName::Fixed(s) => s,
            DisplayName::Hidden {
                unknown,
                name,
                known,
                ..
            } => {
                if *known {
                    name
                } else {
                    unknown
                }
            }
            DisplayName::Actor(id) => actors.get(id).map(|a| a.display_name()).unwrap_or(id),
        }
    }
}

#[derive(Component)]
pub struct CursorTag;

#[derive(Component)]
pub struct WalkMarkTag;

#[derive(Component)]
pub struct Player {
    pub speed: f32,
    pub home: Vec3,
    pub vel: f32,
}

#[derive(Resource, Default)]
pub struct Destination {
    pub path: Vec<Vec3>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Verb(pub String);

impl Verb {
    pub fn new(s: impl Into<String>) -> Self {
        Verb(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is(&self, s: &str) -> bool {
        self.0 == s
    }

    pub fn far_by_default(&self) -> bool {
        self.0 == "look"
    }
}

#[derive(Component)]
pub struct Ident {
    pub key: String,
}

#[derive(Component)]
pub struct Item {
    pub flag: Option<String>,
    pub take: Option<Animation>,
}

#[derive(Clone, Debug, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", remote = "Self")]
pub enum VerbAction {
    Say {
        #[serde(default)]
        lines: Vec<String>,
    },
    Examine,
    Take,
    Dialog {
        node: String,
    },
    ChangeScene {
        scene: String,
    },
    #[default]
    None,
}

impl serde::Serialize for VerbAction {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        VerbAction::serialize(self, s)
    }
}

impl<'de> serde::Deserialize<'de> for VerbAction {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let mut table = <toml::Table as serde::Deserialize>::deserialize(d)?;
        table
            .entry("action")
            .or_insert_with(|| toml::Value::String("none".into()));
        VerbAction::deserialize(toml::Value::Table(table)).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone)]
pub struct VerbEntry {
    pub verb: Verb,
    pub far: bool,
    pub action: VerbAction,
}

#[derive(Component)]
pub struct Interactable {
    pub name: String,
    pub radius: f32,
    pub pick: f32,
    pub pick_explicit: bool,
    pub verbs: Vec<VerbEntry>,
}

impl Interactable {
    pub fn entry(&self, verb: &Verb) -> Option<&VerbEntry> {
        self.verbs.iter().find(|v| &v.verb == verb)
    }

    pub fn action(&self, verb: &Verb) -> Option<&VerbAction> {
        self.entry(verb).map(|v| &v.action)
    }

    pub fn verb_list(&self) -> Vec<Verb> {
        self.verbs.iter().map(|v| v.verb.clone()).collect()
    }

    pub fn is_far(&self, verb: &Verb) -> bool {
        self.entry(verb)
            .map(|v| v.far)
            .unwrap_or_else(|| verb.far_by_default())
    }

    pub fn set_look(&mut self, lines: Vec<String>) {
        let found = self
            .verbs
            .iter_mut()
            .find(|v| v.verb.is("look") && matches!(v.action, VerbAction::Say { .. }));
        match found {
            Some(v) => v.action = VerbAction::Say { lines },
            None => self.verbs.push(VerbEntry {
                verb: Verb::new("look"),
                far: true,
                action: VerbAction::Say { lines },
            }),
        }
    }
}

#[derive(Resource, Default)]
pub struct Flags {
    set: HashSet<String>,
}

impl Flags {
    pub fn set(&mut self, name: &str) {
        self.set.insert(name.to_string());
    }

    pub fn clear(&mut self, name: &str) {
        self.set.remove(name);
    }

    pub fn has(&self, name: &str) -> bool {
        self.set.contains(name)
    }

    pub fn all(&self) -> Vec<String> {
        self.set.iter().cloned().collect()
    }

    pub fn load_from(&mut self, list: Vec<String>) {
        self.set = list.into_iter().collect();
    }
}

#[derive(Resource, Default)]
pub struct ActionMenu {
    pub open: bool,
    pub target: Option<Entity>,
    pub name: String,
    pub verbs: Vec<Verb>,
    pub at: (f32, f32),
}

#[derive(Resource, Default)]
pub struct Inventory {
    pub items: Vec<String>,
    pub selected: Option<usize>,
}

#[derive(Resource, Default)]
pub struct Debug {
    pub enabled: bool,
    pub fps: f32,
}

#[derive(Resource, Default)]
pub struct InputCapture(pub bool);

/// Single source of truth for "world input is captured by a modal".
/// World systems gate on this instead of each re-deriving the condition.
pub fn input_captured(world: &World) -> bool {
    world
        .resource::<InputCapture>()
        .map(|c| c.0)
        .unwrap_or(false)
        || world
            .resource::<Inspect>()
            .map(|i| i.active())
            .unwrap_or(false)
}

#[derive(Clone)]
pub struct Hotspot {
    pub name: String,
    pub center: Vec3,
    pub radius: f32,
    pub look: Vec<String>,
    pub flag: Option<String>,
}

#[derive(Component, Clone)]
pub struct Examinable {
    pub center: Vec3,
    pub scale: f32,
    pub distance: f32,
    pub hotspots: Vec<Hotspot>,
}

impl Default for Examinable {
    fn default() -> Self {
        Examinable {
            center: Vec3::ZERO,
            scale: 1.0,
            distance: 3.0,
            hotspots: Vec::new(),
        }
    }
}

const INTRO_DUR: f32 = 0.16;

#[derive(Resource, Default)]
pub struct Inspect {
    pub entity: Option<Entity>,
    pub temp: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub hover: Option<usize>,
    pub last_cursor: Option<(f32, f32)>,
    pub press: Option<(f32, f32)>,
    pub closing: bool,
    pub intro: Option<Tween<f32>>,
}

impl Inspect {
    pub fn active(&self) -> bool {
        self.entity.is_some()
    }

    pub fn open(&mut self, e: Entity) {
        self.open_inner(e, false);
    }

    pub fn open_temp(&mut self, e: Entity) {
        self.open_inner(e, true);
    }

    fn open_inner(&mut self, e: Entity, temp: bool) {
        self.entity = Some(e);
        self.temp = temp;
        self.yaw = 0.0;
        self.pitch = 0.0;
        self.zoom = 1.0;
        self.hover = None;
        self.last_cursor = None;
        self.press = None;
        self.closing = false;
        self.intro = Some(Tween::new(0.0, 1.0, INTRO_DUR).ease(Ease::OutCubic));
    }

    pub fn fade(&self) -> f32 {
        self.intro.as_ref().map(|t| t.value()).unwrap_or(0.0)
    }

    pub fn begin_close(&mut self) {
        if self.closing {
            return;
        }
        let from = self.fade();
        self.closing = true;
        self.intro = Some(Tween::new(from, 0.0, INTRO_DUR).ease(Ease::InQuad));
    }

    pub fn close(&mut self) {
        self.entity = None;
        self.temp = false;
        self.hover = None;
        self.last_cursor = None;
        self.closing = false;
        self.intro = None;
    }
}

#[derive(Resource, Default)]
pub struct Pending {
    pub target: Option<Entity>,
    pub verb: Option<Verb>,
    pub use_item: Option<String>,
}

#[derive(Component)]
pub struct ChangeSceneTrigger {
    pub to: String,
    pub flag: Option<String>,
    pub radius: f32,
}

#[derive(Component)]
pub struct PlayOnFlag {
    pub flag: String,
    pub anim: Animation,
}

#[derive(Component)]
pub struct SpawnOnFlag {
    pub flag: String,
    pub prefab: String,
    pub offset: Vec3,
    pub scale: Vec3,
}

#[derive(Component)]
pub struct DisableOnFlag {
    pub flag: String,
    pub hide: bool,
}

#[derive(Component)]
pub struct EnableOnFlag {
    pub flag: String,
}

#[derive(Component)]
pub struct LookOnFlag {
    pub flag: String,
    pub look: Vec<String>,
}

#[derive(Component)]
pub struct SfxOnFlag {
    pub flag: String,
    pub sound: String,
}
