use glam::Vec3;

use tungstengine::animation::Animation;
use tungstengine::dialog::Dialog;
use tungstengine::ecs::prelude::*;
use tungstengine::raycast;
use tungstengine::render::d3::prelude::*;

use super::sfx::PlayBlip;
use crate::prelude::*;
use tungstengine::nav::WalkGrid;
use tungstengine::rules::Rules;
use tungstengine::scene::{Prefabs, Scenes};

pub const REACH: f32 = 1.1;

pub type VerbHook = fn(&mut World, Entity, &Verb) -> bool;

#[derive(Resource, Default)]
pub struct Interactions {
    hooks: Vec<VerbHook>,
}

impl Interactions {
    pub fn on_verb(&mut self, hook: VerbHook) {
        self.hooks.push(hook);
    }
}

pub fn interactable_hit(world: &World, e: Entity, ray: &CursorRay) -> Option<f32> {
    let i = world.get::<Interactable>(e)?;
    let pos = world.get::<GlobalTransform>(e)?.position();
    let (has_mesh, mesh) = raycast::subtree_mesh_hit(world, e, ray);
    let sphere = || raycast::ray_sphere(ray, pos, i.pick);
    match (has_mesh, i.pick_explicit) {
        (false, _) => sphere(),
        (true, false) => mesh,
        (true, true) => raycast::closest(mesh, sphere()),
    }
}

pub fn world_pos(world: &World, e: Entity) -> Option<Vec3> {
    world
        .get::<GlobalTransform>(e)
        .map(|g| g.position())
        .or_else(|| world.get::<Transform>(e).map(|t| t.position))
}

pub fn is_close(world: &World, target: Entity) -> bool {
    let op = world_pos(world, target);
    let radius = world.get::<Interactable>(target).map(|i| i.radius);
    let pp = world
        .query::<(&Transform, &Player)>()
        .next()
        .map(|(_, t, _)| t.position);
    if let (Some(op), Some(radius), Some(pp)) = (op, radius, pp) {
        ((op.x - pp.x).powi(2) + (op.z - pp.z).powi(2)).sqrt() <= radius + REACH
    } else {
        false
    }
}

pub fn is_far_action(world: &World, target: Entity, verb: &Verb) -> bool {
    world
        .get::<Interactable>(target)
        .map(|i| i.is_far(verb))
        .unwrap_or(verb.far_by_default())
}

pub fn walk_to(world: &mut World, goal: Vec3) {
    let player_pos = world
        .query::<(&Transform, &Player)>()
        .next()
        .map(|(_, t, _)| t.position);
    let grid = world.query::<(&WalkGrid,)>().next().map(|(_, g)| g);
    let path = match (player_pos, grid) {
        (Some(pp), Some(grid)) => {
            let goal = grid.nearest_free(goal).unwrap_or(goal);
            grid.path(pp, goal)
        }
        (Some(_), None) => Some(vec![goal]),
        _ => None,
    };
    if let Some(path) = path {
        if let Ok(d) = world.resource_mut::<Destination>() {
            d.path = path;
        }
        for (_e, t, _) in world.query_mut::<(&mut Transform, &WalkMarkTag)>() {
            t.position = goal + Vec3::Y * 0.001;
        }
    }
}

pub fn display_label(world: &World, e: Entity, fallback: &str) -> String {
    match (world.get::<DisplayName>(e), world.resource::<Dialog>().ok()) {
        (Some(dn), Some(d)) => dn.label(d.actors()).to_string(),
        _ => fallback.to_string(),
    }
}

pub fn do_verb(world: &mut World, target: Entity, name: &str, verb: &Verb) {
    let hooks: Vec<VerbHook> = world
        .resource::<Interactions>()
        .map(|i| i.hooks.clone())
        .unwrap_or_default();
    for hook in hooks {
        if hook(world, target, verb) {
            return;
        }
    }

    let action = world
        .get::<Interactable>(target)
        .and_then(|i| i.action(verb).cloned())
        .unwrap_or_default();

    match action {
        VerbAction::Say { lines } => {
            let lines = if lines.is_empty() {
                vec![format!("{name}.")]
            } else {
                lines
            };
            if let Ok(d) = world.resource_mut::<Dialog>() {
                d.say(lines);
            }
        }
        VerbAction::Examine => {
            if let Some(dn) = world.get_mut::<DisplayName>(target) {
                dn.reveal();
            }
            if world.has::<Examinable>(target)
                && let Ok(ins) = world.resource_mut::<Inspect>()
            {
                ins.open(target);
            } else if let Ok(d) = world.resource_mut::<Dialog>() {
                d.say(vec![format!("{name}.")]);
            }
        }
        VerbAction::Take => take(world, target),
        VerbAction::Dialog { node } => {
            let flags = world
                .resource::<Flags>()
                .map(|f| f.all())
                .unwrap_or_default();
            if let Ok(d) = world.resource_mut::<Dialog>() {
                d.talk(&node, &flags);
            }
        }
        VerbAction::ChangeScene { scene } => {
            if let Ok(s) = world.resource_mut::<Scenes>() {
                s.request(&scene);
            }
        }
        VerbAction::None => {}
    }
}

fn take(world: &mut World, target: Entity) {
    let key = world.get::<Ident>(target).map(|i| i.key.clone());
    if let Some(key) = key
        && let Ok(inv) = world.resource_mut::<Inventory>()
    {
        inv.items.push(key);
    }

    if let Some(flag) = world.get::<Item>(target).and_then(|i| i.flag.clone())
        && let Ok(fl) = world.resource_mut::<Flags>()
    {
        fl.set(&flag);
    }

    world.play_sfx("pickup");

    let anim = world
        .get::<Item>(target)
        .and_then(|i| i.take.clone())
        .unwrap_or_else(Animation::take_item);
    world.remove::<Interactable>(target);
    world.insert(target, anim);
}

pub fn apply_use(world: &mut World, item: &str, target_key: &str) {
    let effect = world
        .resource::<Rules>()
        .ok()
        .and_then(|r| r.find(item, target_key))
        .map(|r| {
            (
                r.set.clone(),
                r.consume,
                r.give.clone(),
                r.say.clone(),
                r.dialog.clone(),
            )
        });
    match effect {
        Some((set, consume, give, say, dialog)) => {
            if let Some(f) = set
                && let Ok(fl) = world.resource_mut::<Flags>()
            {
                fl.set(&f);
            }
            if let Ok(inv) = world.resource_mut::<Inventory>() {
                if consume && let Some(pos) = inv.items.iter().position(|i| i.as_str() == item) {
                    inv.items.remove(pos);
                }
                if let Some(g) = give {
                    inv.items.push(g);
                }
            }
            if let Some(node) = dialog {
                let flags = world
                    .resource::<Flags>()
                    .map(|f| f.all())
                    .unwrap_or_default();
                if let Ok(d) = world.resource_mut::<Dialog>() {
                    d.talk(&node, &flags);
                }
            } else if !say.is_empty()
                && let Ok(d) = world.resource_mut::<Dialog>()
            {
                d.say(say);
            }
        }
        None => {
            let name_of = |key: &str| {
                world
                    .resource::<Prefabs>()
                    .ok()
                    .and_then(|p| p.name_of(key).map(str::to_string))
                    .unwrap_or_else(|| key.to_string())
            };
            let msg = if target_key.is_empty() {
                crate::tr_args(world, "hud.cant_use", &[("item", &name_of(item))])
            } else {
                crate::tr_args(
                    world,
                    "hud.cant_use_on",
                    &[("item", &name_of(item)), ("target", &name_of(target_key))],
                )
            };
            if let Ok(d) = world.resource_mut::<Dialog>() {
                d.say(vec![msg]);
            }
        }
    }
}

pub fn run_pending(world: &mut World) {
    let pending = world
        .resource::<Pending>()
        .map(|p| (p.target, p.verb.clone(), p.use_item.clone()))
        .unwrap_or((None, None, None));

    let Some(tgt) = pending.0 else {
        return;
    };

    let (verb, use_item) = (pending.1, pending.2);

    if !world.has::<Interactable>(tgt) {
        if let Ok(p) = world.resource_mut::<Pending>() {
            *p = Pending::default();
        }
        return;
    }

    if !is_close(world, tgt) {
        return;
    }

    if let Some(item) = use_item {
        let key = world
            .get::<Ident>(tgt)
            .map(|i| i.key.clone())
            .unwrap_or_default();
        apply_use(world, &item, &key);
    } else if let Some(v) = verb {
        let name = world
            .get::<Interactable>(tgt)
            .map(|i| i.name.clone())
            .unwrap_or_default();
        do_verb(world, tgt, &name, &v);
    }

    if let Ok(de) = world.resource_mut::<Destination>() {
        de.path.clear();
    }

    if let Ok(p) = world.resource_mut::<Pending>() {
        *p = Pending::default();
    }
}
