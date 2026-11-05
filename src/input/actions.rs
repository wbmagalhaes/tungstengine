use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use super::Input;
use super::gamepad::{AxisDir, PadAxis, PadButton};
use crate::ecs::World;
use crate::manifest::GameFiles;
use crate::storage;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Binding {
    Key(KeyCode),
    Mouse(MouseButton),
    Pad(PadButton),
    PadAxis(PadAxis, AxisDir),
}

#[derive(Clone, Debug)]
struct ActionEntry {
    name: String,
    defaults: Vec<Binding>,
    current: Vec<Binding>,
}

pub trait Action: Copy + 'static {
    fn name(self) -> &'static str;
}

#[derive(Default, Clone, Debug)]
pub struct Actions {
    list: Vec<ActionEntry>,
    index: HashMap<String, usize>,
}

#[derive(Serialize, Deserialize, Default)]
struct SavedBindings {
    actions: HashMap<String, Vec<Binding>>,
}

impl Actions {
    pub fn declare(mut self, action: impl Action, defaults: &[Binding]) -> Self {
        let name = action.name();
        let entry = ActionEntry {
            name: name.to_string(),
            defaults: defaults.to_vec(),
            current: defaults.to_vec(),
        };
        match self.index.get(name) {
            Some(&i) => self.list[i] = entry,
            None => {
                self.index.insert(name.to_string(), self.list.len());
                self.list.push(entry);
            }
        }
        self
    }

    pub fn merge(&mut self, other: Actions) {
        for a in other.list {
            match self.index.get(&a.name) {
                Some(&i) => self.list[i] = a,
                None => {
                    self.index.insert(a.name.clone(), self.list.len());
                    self.list.push(a);
                }
            }
        }
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.list.iter().map(|a| a.name.as_str())
    }

    pub fn bindings(&self, name: &str) -> &[Binding] {
        self.get(name).map_or(&[], |a| a.current.as_slice())
    }

    pub fn defaults(&self, name: &str) -> &[Binding] {
        self.get(name).map_or(&[], |a| a.defaults.as_slice())
    }

    pub fn rebind(&mut self, name: &str, bindings: &[Binding]) -> bool {
        match self.index.get(name) {
            Some(&i) => {
                self.list[i].current = bindings.to_vec();
                true
            }
            None => false,
        }
    }

    pub fn reset(&mut self, name: &str) {
        if let Some(&i) = self.index.get(name) {
            self.list[i].current = self.list[i].defaults.clone();
        }
    }

    pub fn reset_all(&mut self) {
        for a in &mut self.list {
            a.current = a.defaults.clone();
        }
    }

    pub fn conflicts(&self, binding: Binding, except: &str) -> Vec<&str> {
        self.list
            .iter()
            .filter(|a| a.name != except && a.current.contains(&binding))
            .map(|a| a.name.as_str())
            .collect()
    }

    pub fn to_toml(&self) -> Result<String, String> {
        let saved = SavedBindings {
            actions: self
                .list
                .iter()
                .filter(|a| a.current != a.defaults)
                .map(|a| (a.name.clone(), a.current.clone()))
                .collect(),
        };
        toml::to_string_pretty(&saved).map_err(|e| e.to_string())
    }

    pub fn apply_toml(&mut self, text: &str) -> Result<(), String> {
        let saved: SavedBindings = toml::from_str(text).map_err(|e| e.to_string())?;
        for (name, bindings) in saved.actions {
            self.rebind(&name, &bindings);
        }
        Ok(())
    }

    pub fn save(&self, key: &str) -> Result<(), String> {
        storage::write(key, &self.to_toml()?)
    }

    pub fn load(&mut self, key: &str) -> Result<(), String> {
        match storage::read(key) {
            Some(text) => self.apply_toml(&text),
            None => Ok(()),
        }
    }

    fn get(&self, name: &str) -> Option<&ActionEntry> {
        self.index.get(name).map(|&i| &self.list[i])
    }
}

pub trait InputWorld {
    fn input(&self) -> &Input;
    fn input_mut(&mut self) -> &mut Input;
    fn load_actions(&mut self, actions: Actions);
}

impl InputWorld for World {
    fn input(&self) -> &Input {
        self.resource_core::<Input>()
    }

    fn input_mut(&mut self) -> &mut Input {
        self.resource_core_mut::<Input>()
    }

    fn load_actions(&mut self, declared: Actions) {
        let file = self
            .resource::<GameFiles>()
            .map_or_else(|_| GameFiles::default().bindings, |f| f.bindings.clone());
        let actions = &mut self.input_mut().actions;
        actions.merge(declared);
        if let Err(e) = actions.load(&file) {
            log::error!("bindings: {e}");
        }
    }
}

impl Input {
    pub fn pressed(&self, action: impl Action) -> bool {
        self.player(0).pressed(action)
    }

    pub fn held(&self, action: impl Action) -> bool {
        self.player(0).held(action)
    }

    pub fn released(&self, action: impl Action) -> bool {
        self.player(0).released(action)
    }

    pub fn binding_pressed(&self, binding: Binding) -> bool {
        match binding {
            Binding::Key(k) => self.keys_struck.contains(&k),
            Binding::Mouse(b) => self.mouse_pressed.contains(&b),
            _ => false,
        }
    }

    pub fn binding_held(&self, binding: Binding) -> bool {
        match binding {
            Binding::Key(k) => self.keys_down.contains(&k),
            Binding::Mouse(b) => self.mouse_down.contains(&b),
            _ => false,
        }
    }

    pub fn binding_released(&self, binding: Binding) -> bool {
        match binding {
            Binding::Key(k) => self.keys_released.contains(&k),
            Binding::Mouse(b) => self.mouse_released.contains(&b),
            _ => false,
        }
    }

    pub fn capture_binding(&self) -> Option<Binding> {
        self.keys_pressed
            .iter()
            .next()
            .map(|k| Binding::Key(*k))
            .or_else(|| self.mouse_pressed.iter().next().map(|b| Binding::Mouse(*b)))
            .or_else(|| self.capture_pad())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(ecs_macros::Action, Clone, Copy)]
    enum Act {
        Jump,
        Fire,
    }

    const JUMP: Binding = Binding::Key(KeyCode::Space);
    const ALT: Binding = Binding::Key(KeyCode::KeyW);

    fn actions() -> Actions {
        Actions::default()
            .declare(Act::Jump, &[JUMP])
            .declare(Act::Fire, &[Binding::Mouse(MouseButton::Left)])
    }

    #[test]
    fn default_and_rebound_keys_trigger_the_action() {
        let mut input = Input {
            actions: actions(),
            ..Default::default()
        };
        input.set_key(KeyCode::Space, true);
        assert!(input.pressed(Act::Jump));
        input.actions.rebind("jump", &[ALT]);
        assert!(!input.pressed(Act::Jump));
        input.set_key(KeyCode::KeyW, true);
        assert!(input.held(Act::Jump));
        assert!(!input.held(Act::Fire));
    }

    #[test]
    fn saved_bindings_round_trip_and_keep_defaults() {
        let mut a = actions();
        a.rebind("jump", &[ALT, Binding::Mouse(MouseButton::Right)]);
        let text = a.to_toml().unwrap();
        let mut b = actions();
        b.apply_toml(&text).unwrap();
        assert_eq!(b.bindings("jump"), a.bindings("jump"));
        b.reset("jump");
        assert_eq!(b.bindings("jump"), &[JUMP]);
    }

    #[test]
    fn reports_conflicts_with_other_actions() {
        let a = actions();
        assert_eq!(a.conflicts(JUMP, "fire"), vec!["jump"]);
        assert!(a.conflicts(JUMP, "jump").is_empty());
    }

    #[test]
    fn capture_returns_the_pressed_input() {
        let mut input = Input::default();
        assert_eq!(input.capture_binding(), None);
        input.set_mouse(MouseButton::Right, true);
        assert_eq!(
            input.capture_binding(),
            Some(Binding::Mouse(MouseButton::Right))
        );
    }
}
