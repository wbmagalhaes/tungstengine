use crate::audio::Volume;
use crate::color::Color;
use crate::ecs::prelude::*;
use crate::input::{Binding, Input, InputWorld};
use crate::localization::Localization;
use crate::ui::{Checkbox, Rect, Stepper, Styled, Text, UI, Z_TOOLTIP, button};

const BUTTON_W: f32 = 300.0;
const BUTTON_H: f32 = 50.0;
const ROW_H: f32 = 40.0;

#[derive(Clone, Copy, PartialEq, Default)]
enum Page {
    #[default]
    Main,
    Settings,
    Controls,
}

#[derive(Clone, Debug)]
pub struct MenuStyle {
    pub z: f32,
    pub card: Color,
    pub border: Color,
    pub title: Color,
}

impl Default for MenuStyle {
    fn default() -> Self {
        MenuStyle {
            z: Z_TOOLTIP + 1.0,
            card: Color::GRAY_900,
            border: Color::GRAY_500,
            title: Color::AMBER_200,
        }
    }
}

#[derive(Resource, Default)]
pub struct GameMenu {
    pub style: MenuStyle,
    pub bindings_key: String,
    pub settings_key: String,
    open: bool,
    page: Page,
    capturing: Option<String>,
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct MenuOutcome {
    pub settings_changed: bool,
    pub fullscreen_toggled: bool,
    pub quit: bool,
}

impl GameMenu {
    pub fn new(bindings_key: &str, settings_key: &str, style: MenuStyle) -> Self {
        GameMenu {
            style,
            bindings_key: bindings_key.to_string(),
            settings_key: settings_key.to_string(),
            ..Default::default()
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn close(&mut self) {
        self.open = false;
        self.capturing = None;
    }

    pub fn toggle(&mut self) {
        if self.capturing.take().is_some() {
            return;
        }
        match (self.open, self.page) {
            (false, _) => {
                self.open = true;
                self.page = Page::Main;
            }
            (true, Page::Main) => self.open = false,
            (true, _) => self.page = Page::Main,
        }
    }
}

pub fn install_locale(loc: &mut Localization) {
    if let Err(e) = loc.load_str("en", include_str!("../assets/locale/en.toml")) {
        log::error!("menus locale: {e}");
    }
}

pub fn binding_label(binding: Binding) -> String {
    match binding {
        Binding::Key(k) => {
            let name = format!("{k:?}");
            name.strip_prefix("Key").unwrap_or(&name).to_string()
        }
        Binding::Mouse(b) => format!("Mouse {b:?}"),
        Binding::Pad(b) => format!("Pad {b:?}"),
        Binding::PadAxis(a, d) => format!("Pad {a:?} {d:?}"),
    }
}

fn tr(world: &World, key: &str) -> String {
    world
        .resource::<Localization>()
        .map_or_else(|_| key.to_string(), |l| l.t(key))
}

fn action_label(world: &World, name: &str) -> String {
    let key = format!("action.{name}");
    world
        .resource::<Localization>()
        .ok()
        .filter(|l| l.has(&key))
        .map_or_else(|| name.to_string(), |l| l.t(&key))
}

pub fn draw(
    world: &mut World,
    ui: &mut UI,
    volume: &mut Volume,
    fullscreen: &mut bool,
) -> MenuOutcome {
    let mut out = MenuOutcome::default();
    let Ok(menu) = world.resource::<GameMenu>() else {
        return out;
    };
    if !menu.open {
        return out;
    }
    let (page, style) = (menu.page, menu.style.clone());
    let z = style.z;
    let names: Vec<String> = world.input().actions.names().map(str::to_string).collect();
    let (w, h) = match page {
        Page::Main => (460.0, 396.0),
        Page::Settings => (460.0, 410.0),
        Page::Controls => (520.0, 230.0 + names.len() as f32 * ROW_H),
    };

    let full = Rect::new(0.0, 0.0, ui.width, ui.height).with_z(z);
    ui.block_pointer(full);
    ui.fill(full, Color::BLACK.with_alpha(0.6));
    let card = Rect::new((ui.width - w) / 2.0, (ui.height - h) / 2.0, w, h).with_z(z + 0.01);
    ui.fill(
        card.with_padding(-3.0).inner().with_z(z + 0.009),
        style.border,
    );
    ui.fill(card, style.card);
    let title = tr(
        world,
        match page {
            Page::Main => "menus.pause",
            Page::Settings => "menus.settings",
            Page::Controls => "menus.controls",
        },
    );
    ui.add(
        Text::centered(Rect::new(card.x, card.y + 22.0, card.w, 32.0), &title)
            .fg(style.title)
            .font_size(28.0)
            .z(z + 0.02),
    );

    let bx = card.x + (card.w - BUTTON_W) / 2.0;
    let slot = |y: f32| Rect::new(bx, y, BUTTON_W, BUTTON_H).with_z(z + 0.02);
    let mut next = None;
    let mut close = false;
    match page {
        Page::Main => {
            let mut y = card.y + 84.0;
            if button(ui, slot(y), &tr(world, "menus.resume"), true, "menu-resume") {
                close = true;
            }
            y += 66.0;
            if button(
                ui,
                slot(y),
                &tr(world, "menus.settings"),
                true,
                "menu-settings",
            ) {
                next = Some(Page::Settings);
            }
            y += 66.0;
            if button(
                ui,
                slot(y),
                &tr(world, "menus.controls"),
                true,
                "menu-controls",
            ) {
                next = Some(Page::Controls);
            }
            y += 66.0;
            if cfg!(not(target_arch = "wasm32"))
                && button(ui, slot(y), &tr(world, "menus.quit"), true, "menu-quit")
            {
                out.quit = true;
            }
        }
        Page::Settings => {
            let t = ui.theme;
            let labels = [tr(world, "menus.sfx"), tr(world, "menus.bgm")];
            for (k, name) in labels.iter().enumerate() {
                let y = card.y + 90.0 + k as f32 * 90.0;
                let value = if k == 0 {
                    &mut volume.sfx
                } else {
                    &mut volume.bgm
                };
                ui.label(card.x + 40.0, y, name, t.label, 18.0, z + 0.02);
                let pct = format!("{:.0}%", *value * 100.0);
                let tw = ui.text_width(18.0, &pct);
                ui.label(
                    card.x + card.w - 40.0 - tw,
                    y,
                    &pct,
                    t.value,
                    18.0,
                    z + 0.02,
                );
                let row = Rect::new(card.x + 40.0, y + 32.0, card.w - 80.0, 40.0).with_z(z + 0.02);
                out.settings_changed |= ui
                    .add(Stepper::new(value, 10, row).id(("menu-volume", k)))
                    .changed();
            }
            out.fullscreen_toggled = ui
                .add(
                    Checkbox::new(fullscreen, &tr(world, "menus.fullscreen"))
                        .at(card.x + 40.0, card.y + 272.0)
                        .z(z + 0.02),
                )
                .changed();
            if button(
                ui,
                slot(card.y + card.h - 74.0),
                &tr(world, "menus.back"),
                true,
                "menu-back",
            ) {
                next = Some(Page::Main);
            }
        }
        Page::Controls => controls(world, ui, card, &names, z, &mut next),
    }

    if let Ok(m) = world.resource_mut::<GameMenu>() {
        if let Some(p) = next {
            m.page = p;
            m.capturing = None;
        }
        if close {
            m.close();
        }
    }
    out
}

fn controls(
    world: &mut World,
    ui: &mut UI,
    card: Rect,
    names: &[String],
    z: f32,
    next: &mut Option<Page>,
) {
    capture(world);
    let t = ui.theme;
    let capturing = world
        .resource::<GameMenu>()
        .ok()
        .and_then(|m| m.capturing.clone());
    let waiting = tr(world, "menus.press_key");
    let mut clicked = None;
    for (i, name) in names.iter().enumerate() {
        let y = card.y + 76.0 + i as f32 * ROW_H;
        let label = action_label(world, name);
        ui.label(card.x + 40.0, y + 8.0, &label, t.label, 18.0, z + 0.02);
        let bound = world
            .input()
            .actions
            .bindings(name)
            .iter()
            .map(|b| binding_label(*b))
            .collect::<Vec<_>>()
            .join(" / ");
        let text = if capturing.as_deref() == Some(name) {
            &waiting
        } else {
            &bound
        };
        let r = Rect::new(card.x + card.w - 260.0, y, 220.0, ROW_H - 6.0).with_z(z + 0.02);
        if button(ui, r, text, true, &format!("menu-bind-{i}")) {
            clicked = Some(name.clone());
        }
    }
    let foot = card.y + card.h - 136.0;
    let bx = card.x + (card.w - BUTTON_W) / 2.0;
    if button(
        ui,
        Rect::new(bx, foot, BUTTON_W, BUTTON_H).with_z(z + 0.02),
        &tr(world, "menus.reset"),
        true,
        "menu-reset",
    ) {
        world.input_mut().actions.reset_all();
        persist(world);
    }
    if button(
        ui,
        Rect::new(bx, foot + 66.0, BUTTON_W, BUTTON_H).with_z(z + 0.02),
        &tr(world, "menus.back"),
        true,
        "menu-back",
    ) {
        *next = Some(Page::Main);
    }
    if let (Some(name), Ok(m)) = (clicked, world.resource_mut::<GameMenu>()) {
        m.capturing = Some(name);
    }
}

fn capture(world: &mut World) {
    let Some(name) = world
        .resource::<GameMenu>()
        .ok()
        .and_then(|m| m.capturing.clone())
    else {
        return;
    };
    let Some(binding) = world
        .resource::<Input>()
        .ok()
        .and_then(Input::capture_binding)
    else {
        return;
    };
    world.input_mut().actions.rebind(&name, &[binding]);
    if let Ok(m) = world.resource_mut::<GameMenu>() {
        m.capturing = None;
    }
    persist(world);
}

fn persist(world: &World) {
    let Ok(m) = world.resource::<GameMenu>() else {
        return;
    };
    if !m.bindings_key.is_empty()
        && let Err(e) = world.input().actions.save(&m.bindings_key)
    {
        log::error!("bindings: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::KeyCode;

    #[test]
    fn toggle_opens_then_backs_out_of_subpages() {
        let mut m = GameMenu::default();
        m.toggle();
        assert!(m.is_open());
        m.page = Page::Controls;
        m.toggle();
        assert!(m.is_open() && m.page == Page::Main);
        m.toggle();
        assert!(!m.is_open());
    }

    #[test]
    fn toggle_while_capturing_only_cancels_capture() {
        let mut m = GameMenu::default();
        m.toggle();
        m.capturing = Some("jump".into());
        m.toggle();
        assert!(m.is_open() && m.capturing.is_none());
    }

    #[test]
    fn binding_labels_are_readable() {
        assert_eq!(binding_label(Binding::Key(KeyCode::KeyW)), "W");
        assert_eq!(binding_label(Binding::Key(KeyCode::Space)), "Space");
    }
}
