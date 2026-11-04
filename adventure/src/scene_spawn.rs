use glam::Vec3;
use serde::{Deserialize, Serialize};

use tungstengine::animation::Animation;
use tungstengine::ecs::Transform;
use tungstengine::ecs::prelude::*;
use tungstengine::render::d3::Outline;
use tungstengine::scene::def::AnimationDef;
use tungstengine::scene::{SpawnDecision, SpawnInfo, SpawnRegistry, component_type, defaults_of};
use tungstengine::sprites::SpriteSpec;

use crate::{
    ActionMenu, ChangeSceneTrigger, Destination, DisableOnFlag, DisplayName, EnableOnFlag,
    Examinable, Flags, Hotspot, Ident, Interactable, Item, LookOnFlag, Pending, PlayOnFlag, Player,
    SfxOnFlag, SpawnOnFlag, Verb, VerbAction, VerbEntry,
};

pub fn register(reg: &mut SpawnRegistry) {
    reg.categorize(
        "Interaction",
        &[
            "interactable",
            "examinable",
            "item",
            "display_name",
            "player",
        ],
    );
    reg.categorize(
        "Flags & Triggers",
        &[
            "change_scene_trigger",
            "play_on_flag",
            "spawn_on_flag",
            "disable_on_flag",
            "enable_on_flag",
            "look_on_flag",
            "sfx_on_flag",
        ],
    );
    reg.add_with_defaults(
        "interactable",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<InteractableDef>() {
                w.insert(e, Interactable::from(d));
            }
        },
        interactable_defaults,
    );
    reg.add_with_defaults(
        "display_name",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<DisplayNameDef>() {
                w.insert(e, DisplayName::from(d));
            }
        },
        display_name_defaults,
    );
    reg.add_with_defaults(
        "player",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<PlayerDef>() {
                w.insert(e, Player::from(d));
            }
        },
        defaults_of::<PlayerDef>,
    );
    reg.add_with_defaults(
        "change_scene_trigger",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<ChangeSceneTriggerDef>() {
                w.insert(e, ChangeSceneTrigger::from(d));
            }
        },
        defaults_of::<ChangeSceneTriggerDef>,
    );
    reg.add_with_defaults(
        "play_on_flag",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<PlayOnFlagDef>() {
                w.insert(e, PlayOnFlag::from(d));
            }
        },
        defaults_of::<PlayOnFlagDef>,
    );
    reg.add_with_defaults(
        "spawn_on_flag",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<SpawnOnFlagDef>() {
                w.insert(e, SpawnOnFlag::from(d));
            }
        },
        spawn_on_flag_defaults,
    );
    reg.add_with_defaults(
        "disable_on_flag",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<DisableOnFlagDef>() {
                w.insert(e, DisableOnFlag::from(d));
            }
        },
        defaults_of::<DisableOnFlagDef>,
    );
    reg.add_with_defaults(
        "enable_on_flag",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<EnableOnFlagDef>() {
                w.insert(e, EnableOnFlag::from(d));
            }
        },
        defaults_of::<EnableOnFlagDef>,
    );
    reg.add_with_defaults(
        "look_on_flag",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<LookOnFlagDef>() {
                w.insert(e, LookOnFlag::from(d));
            }
        },
        look_on_flag_defaults,
    );
    reg.add_with_defaults(
        "sfx_on_flag",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<SfxOnFlagDef>() {
                w.insert(e, SfxOnFlag::from(d));
            }
        },
        defaults_of::<SfxOnFlagDef>,
    );
    reg.add_with_defaults(
        "item",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<ItemDef>() {
                w.insert(
                    e,
                    Item {
                        flag: d.flag,
                        take: d.take.map(Animation::from),
                    },
                );
            }
        },
        defaults_of::<ItemDef>,
    );

    reg.add_with_defaults(
        "examinable",
        |w, v, e| {
            if let Ok(d) = v.clone().try_into::<ExaminableDef>() {
                w.insert(e, Examinable::from(d));
            }
        },
        examinable_defaults,
    );

    reg.filter = Some(gate_filter);
    reg.finalize = Some(finalize);
    reg.on_change = Some(on_change);
    reg.flags = Some(|w| w.resource::<Flags>().map(|f| f.all()).unwrap_or_default());
}

fn with_fields(mut table: toml::Table, fields: Vec<(&str, toml::Value)>) -> toml::Table {
    for (k, v) in fields {
        table.entry(k.to_string()).or_insert(v);
    }
    table
}

fn empty_list() -> toml::Value {
    toml::Value::Array(Vec::new())
}

fn floats(xs: &[f32]) -> toml::Value {
    toml::Value::Array(xs.iter().map(|x| toml::Value::Float(*x as f64)).collect())
}

fn interactable_defaults() -> toml::Table {
    let d = InteractableDef::default();
    with_fields(
        defaults_of::<InteractableDef>(),
        vec![
            ("pick", toml::Value::Float(d.radius as f64)),
            ("verb", empty_list()),
        ],
    )
}

fn display_name_defaults() -> toml::Table {
    with_fields(
        defaults_of::<DisplayNameDef>(),
        vec![("name", toml::Value::String(String::new()))],
    )
}

fn spawn_on_flag_defaults() -> toml::Table {
    with_fields(
        defaults_of::<SpawnOnFlagDef>(),
        vec![("scale", floats(&[1.0, 1.0, 1.0]))],
    )
}

fn look_on_flag_defaults() -> toml::Table {
    with_fields(defaults_of::<LookOnFlagDef>(), vec![("look", empty_list())])
}

fn examinable_defaults() -> toml::Table {
    with_fields(
        defaults_of::<ExaminableDef>(),
        vec![
            ("scale", toml::Value::Float(1.0)),
            ("distance", toml::Value::Float(3.0)),
            ("hotspots", empty_list()),
        ],
    )
}

fn gate_filter(world: &World, components: &[toml::Value]) -> SpawnDecision {
    let flags = world
        .resource::<Flags>()
        .map(|f| f.all())
        .unwrap_or_default();
    let is_set = |f: &str| flags.iter().any(|x| x == f);
    let mut hide = false;
    let mut missing = Vec::new();
    for c in components {
        match component_type(c) {
            Some("disable_on_flag") => {
                let hidden = c.get("hide").and_then(|v| v.as_bool()).unwrap_or(false);
                let flag = c.get("flag").and_then(|v| v.as_str()).unwrap_or("");
                if hidden && is_set(flag) {
                    hide = true;
                }
            }
            Some("enable_on_flag") => {
                let flag = c.get("flag").and_then(|v| v.as_str()).unwrap_or("");
                if !is_set(flag) {
                    missing.push(flag.to_string());
                }
            }
            Some("item") => {
                let flag = c.get("flag").and_then(|v| v.as_str()).unwrap_or("");
                if !flag.is_empty() && is_set(flag) {
                    hide = true;
                }
            }
            _ => {}
        }
    }
    if hide {
        return SpawnDecision::Skip;
    }
    if !missing.is_empty() {
        return SpawnDecision::Defer(missing);
    }
    SpawnDecision::Spawn
}

fn finalize(world: &mut World, e: Entity, info: &SpawnInfo) {
    if !info.key.is_empty() {
        world.insert(
            e,
            Ident {
                key: info.key.to_string(),
            },
        );
    }
    if let Some(i) = world.get_mut::<Interactable>(e) {
        i.name = info.name.to_string();
    }
    let pos = world.get::<Transform>(e).map(|t| t.position);
    if let (Some(pos), Some(p)) = (pos, world.get_mut::<Player>(e)) {
        p.home = pos;
    }
    if world.has::<Interactable>(e) && !world.has::<Outline>(e) {
        world.insert(e, Outline::default());
    }
}

fn on_change(world: &mut World) {
    if let Ok(d) = world.resource_mut::<Destination>() {
        d.path.clear();
    }
    if let Ok(m) = world.resource_mut::<ActionMenu>() {
        m.open = false;
    }
    if let Ok(p) = world.resource_mut::<Pending>() {
        *p = Pending::default();
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VerbDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub far: Option<bool>,
    #[serde(flatten)]
    pub action: VerbAction,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InteractableDef {
    pub radius: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pick: Option<f32>,
    #[serde(default, rename = "verb")]
    pub verbs: Vec<VerbDef>,
}

impl Default for InteractableDef {
    fn default() -> Self {
        InteractableDef {
            radius: 1.0,
            pick: None,
            verbs: Vec::new(),
        }
    }
}

pub const VERB_ACTIONS: [&str; 6] = ["say", "examine", "take", "dialog", "change_scene", "none"];

pub fn verb_defaults(action: &str) -> toml::Table {
    let mut t = toml::Table::new();
    match action {
        "say" => {
            t.insert("lines".into(), empty_list());
        }
        "dialog" => {
            t.insert("node".into(), toml::Value::String(String::new()));
        }
        "change_scene" => {
            t.insert("scene".into(), toml::Value::String(String::new()));
        }
        _ => {}
    }
    t
}

impl From<InteractableDef> for Interactable {
    fn from(def: InteractableDef) -> Self {
        let verbs = def
            .verbs
            .into_iter()
            .map(|v| {
                let verb = Verb::new(v.name);
                VerbEntry {
                    far: v.far.unwrap_or_else(|| verb.far_by_default()),
                    verb,
                    action: v.action,
                }
            })
            .collect();
        Interactable {
            name: String::new(),
            radius: def.radius,
            pick: def.pick.unwrap_or(def.radius),
            pick_explicit: def.pick.is_some(),
            verbs,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HotspotDef {
    name: String,
    #[serde(default)]
    center: [f32; 3],
    radius: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    look: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    flag: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ExaminableDef {
    #[serde(default)]
    center: [f32; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scale: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    distance: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hotspots: Vec<HotspotDef>,
}

impl From<ExaminableDef> for Examinable {
    fn from(def: ExaminableDef) -> Self {
        let hotspots = def
            .hotspots
            .into_iter()
            .map(|h| Hotspot {
                name: h.name,
                center: Vec3::from_array(h.center),
                radius: h.radius,
                look: h.look,
                flag: h.flag,
            })
            .collect();
        Examinable {
            center: Vec3::from_array(def.center),
            scale: def.scale.unwrap_or(1.0),
            distance: def.distance.unwrap_or(3.0),
            hotspots,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DisplayNameDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unknown: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

impl From<DisplayNameDef> for DisplayName {
    fn from(def: DisplayNameDef) -> Self {
        let id = def.id.as_deref();
        if let Some(actor) = id.and_then(|s| s.strip_prefix("act:")) {
            DisplayName::Actor(actor.to_string())
        } else if let Some(key) = id.and_then(|s| s.strip_prefix("obj:")) {
            DisplayName::Hidden {
                key: Some(key.to_string()),
                unknown: def
                    .unknown
                    .unwrap_or_else(|| tungstengine::dialog::UNKNOWN_LABEL.to_string()),
                name: def.name.unwrap_or_default(),
                known: false,
            }
        } else {
            DisplayName::Fixed(def.name.unwrap_or_default())
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerDef {
    speed: f32,
}

impl Default for PlayerDef {
    fn default() -> Self {
        PlayerDef { speed: 6.0 }
    }
}

impl From<PlayerDef> for Player {
    fn from(def: PlayerDef) -> Self {
        Player {
            speed: def.speed,
            home: Vec3::ZERO,
            vel: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChangeSceneTriggerDef {
    to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    flag: Option<String>,
    #[serde(default = "trigger_radius")]
    radius: f32,
}

fn trigger_radius() -> f32 {
    1.0
}

impl Default for ChangeSceneTriggerDef {
    fn default() -> Self {
        ChangeSceneTriggerDef {
            to: String::new(),
            flag: None,
            radius: trigger_radius(),
        }
    }
}

impl From<ChangeSceneTriggerDef> for ChangeSceneTrigger {
    fn from(def: ChangeSceneTriggerDef) -> Self {
        ChangeSceneTrigger {
            to: def.to,
            flag: def.flag,
            radius: def.radius,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PlayOnFlagDef {
    flag: String,
    animation: AnimationDef,
}

impl From<PlayOnFlagDef> for PlayOnFlag {
    fn from(def: PlayOnFlagDef) -> Self {
        PlayOnFlag {
            flag: def.flag,
            anim: Animation::from(def.animation),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SpawnOnFlagDef {
    flag: String,
    prefab: String,
    #[serde(default)]
    offset: [f32; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scale: Option<[f32; 3]>,
}

impl From<SpawnOnFlagDef> for SpawnOnFlag {
    fn from(def: SpawnOnFlagDef) -> Self {
        SpawnOnFlag {
            flag: def.flag,
            prefab: def.prefab,
            offset: Vec3::from_array(def.offset),
            scale: def.scale.map(Vec3::from_array).unwrap_or(Vec3::ONE),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DisableOnFlagDef {
    flag: String,
    #[serde(default)]
    hide: bool,
}

impl From<DisableOnFlagDef> for DisableOnFlag {
    fn from(def: DisableOnFlagDef) -> Self {
        DisableOnFlag {
            flag: def.flag,
            hide: def.hide,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct EnableOnFlagDef {
    flag: String,
}

impl From<EnableOnFlagDef> for EnableOnFlag {
    fn from(def: EnableOnFlagDef) -> Self {
        EnableOnFlag { flag: def.flag }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct LookOnFlagDef {
    flag: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    look: Vec<String>,
}

impl From<LookOnFlagDef> for LookOnFlag {
    fn from(def: LookOnFlagDef) -> Self {
        LookOnFlag {
            flag: def.flag,
            look: def.look,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SfxOnFlagDef {
    flag: String,
    sound: String,
}

impl From<SfxOnFlagDef> for SfxOnFlag {
    fn from(def: SfxOnFlagDef) -> Self {
        SfxOnFlag {
            flag: def.flag,
            sound: def.sound,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ItemDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite: Option<SpriteSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub take: Option<AnimationDef>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
type = "interactable"
radius = 2.6
pick = 1.0

[[verb]]
name = "look"
far = true
action = "say"
lines = ["a", "b"]

[[verb]]
name = "subir"
action = "change_scene"
scene = "nox_apartment_2f"

[[verb]]
name = "push"
action = "dialog"
node = "puff_push"

[[verb]]
name = "examine"
action = "examine"

[[verb]]
name = "take"
action = "take"

[[verb]]
name = "use"
"#;

    #[test]
    fn interactable_round_trip() {
        let def: InteractableDef = toml::from_str(SAMPLE).unwrap();
        assert_eq!(def.radius, 2.6);
        assert_eq!(def.pick, Some(1.0));
        let actions: Vec<_> = def.verbs.iter().map(|v| v.action.clone()).collect();
        assert_eq!(
            actions,
            vec![
                VerbAction::Say {
                    lines: vec!["a".into(), "b".into()]
                },
                VerbAction::ChangeScene {
                    scene: "nox_apartment_2f".into()
                },
                VerbAction::Dialog {
                    node: "puff_push".into()
                },
                VerbAction::Examine,
                VerbAction::Take,
                VerbAction::None,
            ]
        );
        assert_eq!(def.verbs[0].far, Some(true));
        assert_eq!(def.verbs[1].far, None);
        let text = toml::to_string(&def).unwrap();
        let back: InteractableDef = toml::from_str(&text).unwrap();
        assert_eq!(def, back);

        let i = Interactable::from(def);
        assert!(i.is_far(&Verb::new("look")));
        assert!(!i.is_far(&Verb::new("subir")));
        assert_eq!(i.pick, 1.0);
    }

    #[test]
    fn interactable_defaults_deserialize() {
        let t = interactable_defaults();
        assert_eq!(t["pick"], t["radius"]);
        let def: InteractableDef = toml::Value::Table(t).try_into().unwrap();
        assert!(def.verbs.is_empty());
        for a in VERB_ACTIONS {
            let mut v = verb_defaults(a);
            v.insert("name".into(), toml::Value::String("x".into()));
            v.insert("action".into(), toml::Value::String(a.into()));
            let vd: VerbDef = toml::Value::Table(v.clone()).try_into().unwrap();
            let back = toml::Table::try_from(&vd).unwrap();
            assert_eq!(back, v, "{a}");
        }
    }
}
