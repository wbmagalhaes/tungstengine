use crate::AdventureTheme;
use tungstengine::color::Color;
use tungstengine::dialog::{Dialog, Voice};
use tungstengine::ecs::prelude::*;
use tungstengine::ui::prelude::*;

use super::settings::Settings;
use super::sfx::{Blip, PlayBlip};
use crate::{DisplayName, Flags};
use tungstengine::scene::Scenes;

pub fn apply_reveals(world: &mut World) {
    let ids = world
        .resource_mut::<Dialog>()
        .map(|d| d.take_reveals())
        .unwrap_or_default();
    for id in ids {
        let targets: Vec<Entity> = world
            .query::<(&DisplayName,)>()
            .filter(|(_, dn)| {
                matches!(dn, DisplayName::Hidden { key, .. } if key.as_deref() == Some(id.as_str()))
            })
            .map(|(e, _)| e)
            .collect();
        for e in targets {
            if let Some(dn) = world.get_mut::<DisplayName>(e) {
                dn.reveal_if(&id);
            }
        }
    }
}

pub fn apply_flags(world: &mut World) {
    let flags = world
        .resource_mut::<Dialog>()
        .map(|d| d.take_flags())
        .unwrap_or_default();
    if flags.is_empty() {
        return;
    }
    if let Ok(fl) = world.resource_mut::<Flags>() {
        for f in flags {
            fl.set(&f);
        }
    }
}

pub fn apply_sfx(world: &mut World) {
    let sounds = world
        .resource_mut::<Dialog>()
        .map(|d| d.take_sfx())
        .unwrap_or_default();
    for name in sounds {
        world.play_sfx(&name);
    }
}

pub fn apply_scene(world: &mut World) {
    let scene = world
        .resource_mut::<Dialog>()
        .ok()
        .and_then(|d| d.take_scene());
    if let Some(to) = scene
        && let Ok(s) = world.resource_mut::<Scenes>()
    {
        s.request(&to);
    }
}

pub fn dialog_ui(world: &mut World, ui: &mut UI, click: bool) -> bool {
    let (active, speaker, text, options, elapsed) = match world.resource::<Dialog>() {
        Ok(d) => (
            d.active,
            d.speaker.clone(),
            d.text.clone(),
            d.options.clone(),
            d.elapsed,
        ),
        _ => (false, None, String::new(), Vec::new(), 0.0),
    };

    if !active {
        return false;
    }

    let (name, voice, accent) = speaker
        .map(|a| {
            (
                a.display_name().to_string(),
                a.voice,
                Color::rgb(a.accent[0], a.accent[1], a.accent[2]),
            )
        })
        .unwrap_or((String::new(), Voice::default(), Color::TEXT_MUTED));
    let speed = world
        .resource::<Settings>()
        .map(|s| s.text_speed)
        .unwrap_or(45.0);

    let (panel, box_h, body) = world
        .resource::<AdventureTheme>()
        .map(|t| (t.panel_fill(), t.dialog_height, t.font_base))
        .unwrap_or((Color::GRAY_900.with_alpha(0.93), 130.0, 18.0));
    let r = Rect::new(40.0, ui.height - box_h - 24.0, ui.width - 80.0, box_h);

    if !options.is_empty() {
        let chosen = ui.add(DialogBox {
            rect: r,
            panel,
            accent,
            name: &name,
            text: &text,
            options: &options,
            body,
        });
        if let Some(i) = chosen
            && let Ok(d) = world.resource_mut::<Dialog>()
        {
            d.choose(i);
        }
        return true;
    }

    let dt = world.resource::<Time>().map(|t| t.dt).unwrap_or(0.0);
    let elapsed2 = elapsed + dt;
    let total = text.chars().count();
    let prev = ((elapsed * speed) as usize).min(total);
    let revealed = ((elapsed2 * speed) as usize).min(total);
    let shown: String = text.chars().take(revealed).collect();
    let full = revealed >= total;

    if revealed > prev
        && let Some(c) = text.chars().nth(revealed - 1)
        && !c.is_whitespace()
    {
        let wobble = ((c as u32 % 9) as f32 / 9.0 - 0.5) * 2.0 * voice.variation;
        let freq = 440.0 * voice.pitch * (1.0 + wobble);
        world.play_blip(Blip::sfx(freq).gain(0.18));
    }

    ui.add(DialogBox {
        rect: r,
        panel,
        accent,
        name: &name,
        text: &shown,
        options: &[],
        body,
    });

    if let Ok(d) = world.resource_mut::<Dialog>() {
        if click && full {
            d.advance();
        } else if click && !full {
            d.elapsed = total as f32 / speed;
        } else {
            d.elapsed = elapsed2;
        }
    }

    true
}

struct DialogBox<'a> {
    rect: Rect,
    panel: Color,
    accent: Color,
    name: &'a str,
    text: &'a str,
    options: &'a [String],
    body: f32,
}

impl Widget for DialogBox<'_> {
    type Out = Option<usize>;

    fn presence(&self) -> Option<Anim> {
        Some(Anim::new(0.18).fade().rise(20.0))
    }

    fn presence_key(&self) -> Option<&str> {
        Some("dialog")
    }

    fn render(self, ui: &mut UI) -> Option<usize> {
        let r = self.rect;
        ui.fill(r, self.panel);
        ui.fill(Rect::new(r.x, r.y, r.w, 3.0), self.accent);

        let has_name = !self.name.is_empty();
        if has_name {
            ui.add(
                Text::new(r.x + 18.0, r.y + 14.0, self.name)
                    .font_size(17.0)
                    .fg(self.accent),
            );
        }
        let text_y = if has_name { r.y + 46.0 } else { r.y + 18.0 };

        if self.options.is_empty() {
            ui.add(Text::new(r.x + 18.0, text_y, self.text).font_size(self.body));
            return None;
        }

        ui.text(r.x + 18.0, text_y, self.text);
        let mut chosen = None;
        for (i, opt) in self.options.iter().enumerate() {
            let br = Rect::new(
                r.x + 18.0,
                text_y + 26.0 + i as f32 * 24.0,
                r.w - 60.0,
                22.0,
            );
            if ui.button(br.with_padding(3.0), opt).clicked() {
                chosen = Some(i);
            }
        }
        chosen
    }
}
