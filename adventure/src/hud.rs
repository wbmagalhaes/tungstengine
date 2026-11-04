use crate::AdventureTheme;
use glam::{Mat4, Vec3};

use tungstengine::color::Color;
use tungstengine::dialog::Dialog;
use tungstengine::ecs::prelude::*;
use tungstengine::input::Input;
use tungstengine::render::d3::prelude::*;
use tungstengine::scene::spawn_gameobj;
use tungstengine::sprites::Sprites;
use tungstengine::ui::prelude::*;

use super::interact::{
    apply_use, display_label, do_verb, interactable_hit, is_close, is_far_action, walk_to,
    world_pos,
};
use super::sfx::PlayBlip;
use super::systems;
use crate::prelude::*;
use crate::{tr, tr_args};
use tungstengine::input::InputWorld;
use tungstengine::localization::Localization;
use tungstengine::rules::Rules;
use tungstengine::scene::Prefabs;

fn verb_label(world: &World, verb: &Verb) -> String {
    let key = format!("verb.{}", verb.as_str());
    world
        .resource::<Localization>()
        .ok()
        .filter(|l| l.has(&key))
        .map_or_else(|| verb.as_str().to_string(), |l| l.t(&key))
}

fn item_name(world: &World, key: &str) -> String {
    world
        .resource::<Prefabs>()
        .ok()
        .and_then(|p| p.name_of(key).map(str::to_string))
        .unwrap_or_else(|| key.to_string())
}

fn ident_key(world: &World, e: Entity) -> String {
    world
        .get::<Ident>(e)
        .map(|i| i.key.clone())
        .unwrap_or_default()
}

fn accent_of(world: &World, e: Entity) -> Color {
    let actor = match world.get::<DisplayName>(e) {
        Some(DisplayName::Actor(id)) => Some(id.clone()),
        _ => None,
    };
    actor
        .and_then(|id| {
            world
                .resource::<Dialog>()
                .ok()
                .and_then(|d| d.actors().get(&id).map(|a| a.accent))
        })
        .map(|c| Color::rgb(c[0], c[1], c[2]))
        .unwrap_or(Color::TEXT_TITLE)
}

fn world_to_screen(world: &World, ui: &UI, p: Vec3) -> Option<(f32, f32)> {
    let (cam_world, fov, near, far) = world
        .query::<(&GlobalTransform, &Camera, &MainCam)>()
        .next()
        .map(|(_, g, c, _)| (g.matrix(), c.fov, c.near, c.far))?;
    let vp =
        Mat4::perspective_rh(fov, ui.width / ui.height.max(1.0), near, far) * cam_world.inverse();
    let clip = vp * p.extend(1.0);
    if clip.w <= 1e-5 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    Some((
        (ndc.x * 0.5 + 0.5) * ui.width,
        (1.0 - (ndc.y * 0.5 + 0.5)) * ui.height,
    ))
}

struct ActionPopup<'a> {
    at: (f32, f32),
    name: &'a str,
    accent: Color,
    panel: Color,
    labels: &'a [String],
}

impl Widget for ActionPopup<'_> {
    type Out = Option<usize>;

    fn presence(&self) -> Option<Anim> {
        Some(Anim::new(0.13).fade().pop(0.94))
    }

    fn presence_key(&self) -> Option<&str> {
        Some("action-menu")
    }

    fn render(self, ui: &mut UI) -> Option<usize> {
        let (bw, bh, gap, title_h) = (130.0, 30.0, 4.0, 24.0);
        let total_h = title_h + self.labels.len() as f32 * (bh + gap);
        let panel_rect = Rect::new(self.at.0 - 6.0, self.at.1 - 6.0, bw + 12.0, total_h + 6.0);

        ui.fill(panel_rect, self.panel);
        ui.add(
            Text::new(self.at.0, self.at.1, self.name)
                .font_size(18.0)
                .fg(self.accent),
        );
        let mut chosen = None;
        for (i, label) in self.labels.iter().enumerate() {
            let r = Rect::new(
                self.at.0,
                self.at.1 + title_h + i as f32 * (bh + gap),
                bw,
                bh,
            );
            if ui.button(r.with_padding(4.0), label).clicked() {
                chosen = Some(i);
            }
        }
        ui.block_pointer(panel_rect);
        chosen
    }
}

pub fn action_menu(world: &mut World, ui: &mut UI) {
    let (open, at, name, verbs, target): (bool, (f32, f32), String, Vec<Verb>, Option<Entity>) =
        match world.resource::<ActionMenu>() {
            Ok(m) => (m.open, m.at, m.name.clone(), m.verbs.clone(), m.target),
            Err(_) => return,
        };
    if !open {
        return;
    }

    let at = target
        .and_then(|t| world_pos(world, t))
        .and_then(|p| world_to_screen(world, ui, p + Vec3::Y))
        .unwrap_or(at);
    let (panel, title_col) = world
        .resource::<AdventureTheme>()
        .map(|t| (t.panel_fill(), t.title_fg))
        .unwrap_or((Color::GRAY_900.with_alpha(0.92), Color::TEXT_TITLE));
    let accent = target.map(|t| accent_of(world, t)).unwrap_or(title_col);
    let labels: Vec<String> = verbs.iter().map(|v| verb_label(world, v)).collect();

    let chosen = ui.add(ActionPopup {
        at,
        name: &name,
        accent,
        panel,
        labels: &labels,
    });

    if let Some(i) = chosen {
        let verb = verbs[i].clone();
        if let Some(tgt) = target {
            if is_far_action(world, tgt, &verb) || is_close(world, tgt) {
                do_verb(world, tgt, &name, &verb);
            } else {
                if let Some(op) = world_pos(world, tgt) {
                    walk_to(world, Vec3::new(op.x, 0.0, op.z));
                }
                if let Ok(p) = world.resource_mut::<Pending>() {
                    p.target = Some(tgt);
                    p.verb = Some(verb);
                    p.use_item = None;
                }
            }
        }
        if let Ok(m) = world.resource_mut::<ActionMenu>() {
            m.open = false;
        }
    }
}

pub fn inventory_bar(world: &mut World, ui: &mut UI) -> bool {
    let (items, selected) = world
        .resource::<Inventory>()
        .map(|inv| (inv.items.clone(), inv.selected))
        .unwrap_or_default();

    let (sz, gap, margin, panel, small) = world
        .resource::<AdventureTheme>()
        .map(|t| {
            (
                t.inv_slot,
                t.inv_gap,
                t.inv_margin,
                t.panel_fill(),
                t.font_sm,
            )
        })
        .unwrap_or_else(|_| {
            let d = AdventureTheme::default();
            (
                d.inv_slot,
                d.inv_gap,
                d.inv_margin,
                d.panel_fill(),
                d.font_sm,
            )
        });

    let examine_click = world.input().pressed(crate::AdventureAction::Examine);
    let cursor = world
        .resource::<Input>()
        .map(|i| i.cursor)
        .unwrap_or((0.0, 0.0));

    let mut inv_clicked: Option<usize> = None;
    let mut to_examine: Option<String> = None;
    let mut over = false;
    if !items.is_empty() {
        let y = ui.height - sz - 12.0;
        for (i, item) in items.iter().enumerate() {
            let r = Rect::new(margin + i as f32 * (sz + gap), y, sz, sz);
            if examine_click && r.contains(cursor) {
                to_examine = Some(item.clone());
            }
            let spec = world
                .resource::<Prefabs>()
                .ok()
                .and_then(|p| p.sprite_of(item));
            let sprite = match (spec, world.resource::<Sprites>().ok()) {
                (Some(spec), Some(sprites)) => sprites.resolve(&spec),
                _ => Sprite::RECT.draw(),
            };

            let resp = ui.sprite_button(r, item, sprite, Color::AMBER_300);
            over |= resp.hovered();
            if resp.clicked() {
                inv_clicked = Some(i);
            }

            if resp.hovered() {
                let label = match selected {
                    Some(s) if s != i => tr_args(
                        world,
                        "hud.combine",
                        &[
                            ("a", &item_name(world, &items[s])),
                            ("b", &item_name(world, item)),
                        ],
                    ),
                    _ => item_name(world, item),
                };
                let tw = ui.text_width(small, &label) + 12.0;
                let tr = Rect::new(r.x, r.y - 26.0, tw, 22.0);
                ui.fill(tr, panel);
                ui.add(Text::new(tr.x + 6.0, tr.y + 4.0, &label).font_size(small));
            }

            if selected == Some(i) {
                ui.fill(r, Color::RED_500);
            }
        }
    }

    if let Some(key) = to_examine {
        examine_inventory_item(world, &key);
        return true;
    }

    let Some(i) = inv_clicked else {
        return over;
    };

    let sel = world
        .resource::<Inventory>()
        .ok()
        .and_then(|inv| inv.selected);
    match sel {
        Some(s) if s == i => {
            if let Ok(inv) = world.resource_mut::<Inventory>() {
                inv.selected = None;
            }
        }
        Some(s) => combine(world, s, i),
        None => {
            if let Ok(inv) = world.resource_mut::<Inventory>() {
                inv.selected = Some(i);
            }
        }
    }
    true
}

fn examine_inventory_item(world: &mut World, key: &str) {
    let Some(id) = world.resource::<Prefabs>().ok().and_then(|p| p.id_of(key)) else {
        return;
    };
    let Some(e) = spawn_gameobj(world, id, Vec3::new(0.0, -1000.0, 0.0)) else {
        return;
    };
    if world.has::<Examinable>(e) && world.has::<Renderer>(e) {
        if let Ok(ins) = world.resource_mut::<Inspect>() {
            ins.open_temp(e);
        }
    } else {
        world.despawn(e);
    }
}

fn combine(world: &mut World, a_idx: usize, b_idx: usize) {
    let names = world
        .resource::<Inventory>()
        .ok()
        .map(|inv| (inv.items.get(a_idx).cloned(), inv.items.get(b_idx).cloned()));
    let Some((Some(a), Some(b))) = names else {
        return;
    };
    let combo = world
        .resource::<Rules>()
        .ok()
        .and_then(|r| r.find_combo(&a, &b))
        .map(|c| (c.result.clone(), c.say.clone()));
    match combo {
        Some((result, say)) => {
            if let Ok(inv) = world.resource_mut::<Inventory>() {
                let (hi, lo) = (a_idx.max(b_idx), a_idx.min(b_idx));
                if hi < inv.items.len() {
                    inv.items.remove(hi);
                }
                if lo < inv.items.len() {
                    inv.items.remove(lo);
                }
                inv.items.push(result);
                inv.selected = None;
            }

            world.play_sfx("combine");

            if !say.is_empty()
                && let Ok(d) = world.resource_mut::<Dialog>()
            {
                d.say(say);
            }
        }
        None => {
            let msg = tr_args(
                world,
                "hud.no_combine",
                &[("a", &item_name(world, &a)), ("b", &item_name(world, &b))],
            );
            if let Ok(d) = world.resource_mut::<Dialog>() {
                d.say(vec![msg]);
            }
            if let Ok(inv) = world.resource_mut::<Inventory>() {
                inv.selected = None;
            }
        }
    }
}

fn picked(world: &World, ray: &CursorRay) -> Option<Entity> {
    let mut hit: Option<(Entity, f32)> = None;
    let ents: Vec<Entity> = world.query::<(&Interactable,)>().map(|(e, _)| e).collect();
    for e in ents {
        if let Some(d) = interactable_hit(world, e, ray)
            && hit.as_ref().is_none_or(|(_, hd)| d < *hd)
        {
            hit = Some((e, d));
        }
    }
    hit.map(|(e, _)| e)
}

pub fn world_hold(world: &mut World, held: bool) {
    if !held {
        return;
    }
    let menu_open = world
        .resource::<ActionMenu>()
        .map(|m| m.open)
        .unwrap_or(false);
    let item_selected = world
        .resource::<Inventory>()
        .map(|inv| inv.selected.is_some())
        .unwrap_or(false);
    if menu_open || item_selected {
        return;
    }
    let ray = world.resource::<CursorRay>().copied().unwrap_or_default();
    if picked(world, &ray).is_some() {
        return;
    }
    walk_to(world, systems::ground_point(&ray));
}

pub fn world_click(world: &mut World, click: bool, cursor: (f32, f32)) {
    if !click {
        return;
    }
    let ray = world.resource::<CursorRay>().copied().unwrap_or_default();
    let selected_item = world
        .resource::<Inventory>()
        .ok()
        .and_then(|inv| inv.selected.and_then(|s| inv.items.get(s).cloned()));

    let candidates: Vec<(Entity, String, Vec<Verb>)> = world
        .query::<(&Interactable,)>()
        .map(|(e, obj)| (e, obj.name.clone(), obj.verb_list()))
        .collect();
    let mut hit: Option<(Entity, String, Vec<Verb>, f32)> = None;
    for (e, name, verbs) in candidates {
        if let Some(d) = interactable_hit(world, e, &ray)
            && hit.as_ref().is_none_or(|(_, _, _, hd)| d < *hd)
        {
            hit = Some((e, name, verbs, d));
        }
    }

    if let Some(item) = selected_item {
        if let Some((he, _, _, _)) = &hit {
            let he = *he;
            let key = ident_key(world, he);
            let far = world
                .resource::<Rules>()
                .ok()
                .and_then(|r| r.find(&item, &key))
                .map(|r| r.far)
                .unwrap_or(false);
            if far || is_close(world, he) {
                apply_use(world, &item, &key);
            } else {
                if let Some(op) = world_pos(world, he) {
                    walk_to(world, Vec3::new(op.x, 0.0, op.z));
                }
                if let Ok(p) = world.resource_mut::<Pending>() {
                    p.target = Some(he);
                    p.verb = None;
                    p.use_item = Some(item);
                }
            }
        } else {
            apply_use(world, &item, "");
        }
        if let Ok(inv) = world.resource_mut::<Inventory>() {
            inv.selected = None;
        }
    } else if let Some((he, hname, mut hverbs, _)) = hit {
        if world.has::<Item>(he) && !hverbs.iter().any(|v| v.is("take")) {
            hverbs.push(Verb::new("take"));
        }
        let label = display_label(world, he, &hname);
        if let Ok(m) = world.resource_mut::<ActionMenu>() {
            m.open = true;
            m.target = Some(he);
            m.name = label;
            m.verbs = hverbs;
            m.at = cursor;
        }
        if let Ok(p) = world.resource_mut::<Pending>() {
            *p = Pending::default();
        }
    } else {
        walk_to(world, systems::ground_point(&ray));
        if let Ok(p) = world.resource_mut::<Pending>() {
            *p = Pending::default();
        }
        if let Ok(m) = world.resource_mut::<ActionMenu>() {
            m.open = false;
        }
    }
}

pub fn hover_label(world: &World, ui: &mut UI, cursor: (f32, f32)) {
    let menu_open = world
        .resource::<ActionMenu>()
        .map(|m| m.open)
        .unwrap_or(false);
    if menu_open {
        return;
    }
    let ray = world.resource::<CursorRay>().copied().unwrap_or_default();
    let Some(e) = picked(world, &ray) else {
        return;
    };
    let name = world
        .get::<Interactable>(e)
        .map(|i| i.name.clone())
        .unwrap_or_default();
    let mut label = display_label(world, e, &name);
    if label.is_empty() {
        return;
    }
    if let Some(item) = world
        .resource::<Inventory>()
        .ok()
        .and_then(|inv| inv.selected.and_then(|s| inv.items.get(s).cloned()))
    {
        label = tr_args(
            world,
            "hud.use_on",
            &[
                ("verb", &verb_label(world, &Verb::new("use"))),
                ("item", &item_name(world, &item)),
                ("target", &label),
            ],
        );
    }

    let (panel, small) = world
        .resource::<AdventureTheme>()
        .map(|t| (t.panel_fill(), t.font_sm))
        .unwrap_or((Color::GRAY_900.with_alpha(0.92), 16.0));
    let tw = ui.text_width(small, &label) + 12.0;
    let r = Rect::new(cursor.0 + 16.0, cursor.1 + 16.0, tw, 22.0);
    ui.add(
        Group::new(move |ui| {
            ui.fill(r, panel);
            ui.add(Text::new(r.x + 6.0, r.y + 4.0, &label).font_size(small));
        })
        .key("hover-label")
        .anim(Anim::new(0.05).pop(0.92)),
    );
}

pub fn hint(world: &World, ui: &mut UI) {
    ui.add(Text::new(16.0, 16.0, &tr(world, "hud.hint")).fg(Color::TEXT_MUTED));
}
