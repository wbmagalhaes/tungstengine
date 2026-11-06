use crate::audio::{Audio, Volume};
use crate::color::Color;
use crate::ecs::prelude::*;
use crate::localization::Localization;
use crate::sprites::Sprites;
use crate::ui::{
    Button, Group, LineIcon, Rect, RichText, SpriteDraw, Stateful, Styled, UI, parse_rich,
    typed_count, typing_time,
};

use super::mumble::vowel_between;
use super::{Dialog, PortraitSide};

const SIDE: f32 = 40.0;
const BOTTOM: f32 = 24.0;
const HEIGHT: f32 = 132.0;
const PAD: f32 = 18.0;
const NAME_SIZE: f32 = 15.0;
const LINE_SIZE: f32 = 18.0;
const NAME_GAP: f32 = 6.0;
const OPTION_HEIGHT: f32 = 34.0;
const OPTION_GAP: f32 = 6.0;
const FOCUS_OUTLINE: f32 = 2.0;
const OPTION_WIDTH: f32 = 340.0;
const OPTION_PAD: f32 = 14.0;
const PORTRAIT: f32 = 96.0;
const TYPE_SPEED: f32 = 45.0;
const Z: f32 = 90.0;
const ENTER: f32 = 0.18;
const RISE: f32 = 20.0;
const HINT_SIZE: f32 = 12.0;
const HINT_BOB: f32 = 2.0;

struct Scene<'a> {
    name: &'a str,
    accent: Color,
    text: &'a str,
    typed: usize,
    options: &'a [String],
    waiting: bool,
    portrait: Option<SpriteDraw>,
    right: bool,
    focus: usize,
}

fn draw(ui: &mut UI, scene: &Scene) -> (Option<usize>, Option<usize>) {
    let t = ui.theme;
    let top = ui.height - HEIGHT - BOTTOM;
    let r = Rect::new(SIDE, top, ui.width - SIDE * 2.0, HEIGHT).with_z(Z);
    ui.fill_bordered(
        r,
        t.panel_bg,
        t.border_radius.max(10.0),
        1.0,
        t.panel_border,
    );
    let mut y = r.y + PAD;
    let z = Z + 0.01;
    let mut text_x = r.x + PAD;
    let mut text_end = r.x + r.w - PAD;
    if let Some(sprite) = scene.portrait {
        let x = if scene.right {
            text_end -= PORTRAIT + PAD;
            r.x + r.w - PAD - PORTRAIT
        } else {
            text_x += PORTRAIT + PAD;
            r.x + PAD
        };
        let spot = Rect::new(x, r.y + (r.h - PORTRAIT) / 2.0, PORTRAIT, PORTRAIT);
        ui.sprite(spot.with_z(z), sprite, Color::WHITE);
    }
    if !scene.name.is_empty() {
        ui.label(text_x, y, scene.name, scene.accent, NAME_SIZE, z);
        y += NAME_SIZE + NAME_GAP;
    }
    let width = text_end - text_x;
    ui.add(
        RichText::new(text_x, y, scene.text)
            .wrapped(width)
            .font_size(LINE_SIZE)
            .fg(t.button_fg)
            .visible(scene.typed)
            .z(z),
    );
    let mut picked = None;
    let mut hovered = None;
    let rows = scene.options.len() as f32;
    let mut row_y = r.y - rows * (OPTION_HEIGHT + OPTION_GAP);
    let option_x = r.x + (r.w - OPTION_WIDTH) / 2.0;
    for (i, option) in scene.options.iter().enumerate() {
        let area = Rect::new(option_x, row_y, OPTION_WIDTH, OPTION_HEIGHT).with_z(z);
        let label = format!("{}. {option}", i + 1);
        let mut button = Button::new(area, &label)
            .id(("conversation-option", i))
            .text_left()
            .pad_l(OPTION_PAD)
            .z(z)
            .hover(crate::ui::Style {
                scale: Some((1.0, 1.0)),
                ..Default::default()
            });
        if i == scene.focus {
            button = button.outline(t.accent, FOCUS_OUTLINE);
        }
        let resp = ui.add(button);
        if resp.hovered {
            hovered = Some(i);
        }
        if resp.clicked() {
            picked = Some(i);
        }
        row_y += OPTION_HEIGHT + OPTION_GAP;
    }
    if scene.waiting {
        let bob = (ui.elapsed() * 5.0).sin() * HINT_BOB;
        let spot = (text_end, r.y + r.h - PAD + bob);
        ui.icon_line(LineIcon::ChevronDown, spot, HINT_SIZE, scene.accent, z);
    }
    (picked, hovered)
}

fn portrait(world: &World) -> Option<SpriteDraw> {
    let dialog = world.resource::<Dialog>().ok()?;
    let speaker = dialog.speaker.as_ref()?;
    let base = speaker
        .portrait
        .clone()
        .unwrap_or_else(|| format!("actor-{}", speaker.id));
    let sprites = world.resource::<Sprites>().ok()?;
    let mood = dialog.mood.as_ref().map(|m| format!("{base}-{m}"));
    mood.and_then(|name| sprites.find(name.as_str()))
        .or_else(|| sprites.find(base.as_str()))
}

fn spoken_name(world: &World) -> Option<String> {
    let speaker = world.resource::<Dialog>().ok()?.speaker.as_ref()?;
    let state = if speaker.known { "name" } else { "unknown" };
    let key = format!("actor.{}.{state}", speaker.id);
    let translated = world
        .resource::<Localization>()
        .ok()
        .filter(|loc| loc.has(&key))
        .map(|loc| loc.t(&key));
    Some(translated.unwrap_or_else(|| speaker.display_name().to_string()))
}

pub fn conversation(world: &mut World, ui: &mut UI) -> bool {
    let dt = world.dt();
    let portrait = portrait(world);
    let name = spoken_name(world);
    let Ok(dialog) = world.resource_mut::<Dialog>() else {
        return false;
    };
    if !dialog.active {
        return false;
    }
    let full_screen = Rect::new(0.0, 0.0, ui.width, ui.height).with_z(Z - 0.5);
    let edit = ui.edit_input();
    let settled = dialog.elapsed > 0.0;
    let confirm = settled && (edit.enter || edit.typed.contains(' '));
    let click = settled && ui.clicked || confirm;
    ui.block_pointer(full_screen);

    let chars = parse_rich(&dialog.text);
    let typed = typed_count(&chars, dialog.elapsed, TYPE_SPEED);
    let done = typed >= chars.len();
    let sound = dialog
        .speaker
        .as_ref()
        .and_then(|s| vowel_between(&chars, dialog.spoken, typed, s.voice));
    dialog.spoken = dialog.spoken.max(typed);
    let full_text = dialog.text.clone();
    let name = name.unwrap_or_default();
    let accent = dialog.speaker.as_ref().map_or(ui.theme.accent, |s| {
        Color::rgb(s.accent[0], s.accent[1], s.accent[2])
    });
    let options = if done {
        dialog.options.clone()
    } else {
        Vec::new()
    };
    let scene = Scene {
        name: &name,
        accent,
        text: &full_text,
        typed,
        options: &options,
        waiting: done && options.is_empty(),
        portrait,
        focus: dialog.focus,
        right: dialog
            .side
            .or(dialog.speaker.as_ref().map(|s| s.portrait_side))
            .is_some_and(|side| side == PortraitSide::Right),
    };
    let (mut picked, hovered) = ui.add(
        Group::new(|ui: &mut UI| draw(ui, &scene))
            .anim(crate::ui::Anim::new(ENTER).fade().rise(RISE))
            .key("conversation"),
    );
    let count = options.len();
    if count > 0 {
        if let Some(index) = hovered {
            dialog.focus = index;
        }
        if edit.up {
            dialog.focus = (dialog.focus + count - 1) % count;
        }
        if edit.down {
            dialog.focus = (dialog.focus + 1) % count;
        }
        if confirm {
            picked = picked.or(Some(dialog.focus));
        }
        let number = edit.typed.chars().find_map(|c| c.to_digit(10));
        if let Some(digit) = number.filter(|d| (1..=count as u32).contains(d)) {
            picked = Some(digit as usize - 1);
        }
    }
    if let Some(index) = picked {
        dialog.choose(index);
    } else if click && done && options.is_empty() {
        dialog.advance();
    } else if click && !done {
        dialog.elapsed = typing_time(&chars, TYPE_SPEED);
        dialog.spoken = chars.len();
    } else {
        dialog.elapsed += dt;
    }
    if let Some(sound) = sound {
        let gain = world.resource::<Volume>().copied().unwrap_or_default().sfx;
        if let Ok(audio) = world.resource::<Audio>() {
            audio.vowel(sound, gain);
        }
    }
    true
}
