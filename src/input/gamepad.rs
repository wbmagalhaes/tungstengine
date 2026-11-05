use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::actions::Action;
use super::{Actions, Binding, Input};

pub const AXIS_THRESHOLD: f32 = 0.5;
const DEAD_ZONE: f32 = 0.2;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PadButton {
    South,
    East,
    West,
    North,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    LeftStick,
    RightStick,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PadAxis {
    LeftX,
    LeftY,
    RightX,
    RightY,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AxisDir {
    Positive,
    Negative,
}

const AXES: [PadAxis; 4] = [
    PadAxis::LeftX,
    PadAxis::LeftY,
    PadAxis::RightX,
    PadAxis::RightY,
];

#[derive(Default, Clone, Debug)]
pub struct PadState {
    pub connected: bool,
    down: HashSet<PadButton>,
    pressed: HashSet<PadButton>,
    released: HashSet<PadButton>,
    axes: [f32; 4],
    prev_axes: [f32; 4],
}

impl PadState {
    fn strength(axes: &[f32; 4], axis: PadAxis, dir: AxisDir) -> f32 {
        let v = axes[axis as usize];
        match dir {
            AxisDir::Positive => v.max(0.0),
            AxisDir::Negative => (-v).max(0.0),
        }
    }

    fn axis_value(&self, axis: PadAxis, dir: AxisDir) -> f32 {
        Self::strength(&self.axes, axis, dir)
    }

    fn axis_was(&self, axis: PadAxis, dir: AxisDir) -> f32 {
        Self::strength(&self.prev_axes, axis, dir)
    }

    fn end_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
        self.prev_axes = self.axes;
    }
}

#[derive(Clone, Debug)]
pub struct PlayerSlot {
    pub actions: Actions,
    pub keyboard: bool,
    pub pad: Option<usize>,
}

#[derive(Clone, Copy)]
enum Phase {
    Pressed,
    Held,
    Released,
}

pub struct PlayerInput<'a> {
    input: &'a Input,
    player: usize,
}

impl PlayerInput<'_> {
    pub fn pressed(&self, action: impl Action) -> bool {
        self.input.check(self.player, action, Phase::Pressed)
    }

    pub fn held(&self, action: impl Action) -> bool {
        self.input.check(self.player, action, Phase::Held)
    }

    pub fn released(&self, action: impl Action) -> bool {
        self.input.check(self.player, action, Phase::Released)
    }

    pub fn value(&self, action: impl Action) -> f32 {
        self.input.value(self.player, action)
    }

    pub fn pad(&self) -> Option<usize> {
        self.input.player_pad(self.player)
    }
}

impl Input {
    pub fn player(&self, player: usize) -> PlayerInput<'_> {
        PlayerInput {
            input: self,
            player,
        }
    }

    pub fn player_count(&self) -> usize {
        1 + self.extra_players.len()
    }

    pub fn add_player(&mut self, keyboard: bool) -> usize {
        let pad = self.free_pad();
        self.extra_players.push(PlayerSlot {
            actions: self.actions.clone(),
            keyboard,
            pad,
        });
        self.extra_players.len()
    }

    pub fn player_actions_mut(&mut self, player: usize) -> Option<&mut Actions> {
        match player {
            0 => Some(&mut self.actions),
            n => self.extra_players.get_mut(n - 1).map(|p| &mut p.actions),
        }
    }

    pub fn assign_pad(&mut self, player: usize, pad: Option<usize>) {
        if player == 0 {
            self.main_pad = pad;
        } else if let Some(p) = self.extra_players.get_mut(player - 1) {
            p.pad = pad;
        }
    }

    pub fn pad_connected(&mut self, pad: usize) {
        if self.pads.len() <= pad {
            self.pads.resize_with(pad + 1, PadState::default);
        }
        self.pads[pad].connected = true;
        if self.pad_owner(pad).is_some() {
            return;
        }
        if self.main_pad.is_none() {
            self.main_pad = Some(pad);
        } else if let Some(p) = self.extra_players.iter_mut().find(|p| p.pad.is_none()) {
            p.pad = Some(pad);
        }
    }

    pub fn pad_disconnected(&mut self, pad: usize) {
        if let Some(state) = self.pads.get_mut(pad) {
            *state = PadState::default();
        }
        if let Some(owner) = self.pad_owner(pad) {
            self.assign_pad(owner, None);
        }
    }

    pub fn set_pad_button(&mut self, pad: usize, button: PadButton, down: bool) {
        let Some(state) = self.pads.get_mut(pad) else {
            return;
        };
        if down {
            if state.down.insert(button) {
                state.pressed.insert(button);
            }
        } else if state.down.remove(&button) {
            state.released.insert(button);
        }
    }

    pub fn set_pad_axis(&mut self, pad: usize, axis: PadAxis, value: f32) {
        if let Some(state) = self.pads.get_mut(pad) {
            state.axes[axis as usize] = if value.abs() < DEAD_ZONE { 0.0 } else { value };
        }
    }

    pub(super) fn pads_end_frame(&mut self) {
        self.pads.iter_mut().for_each(PadState::end_frame);
    }

    pub(super) fn capture_pad(&self) -> Option<Binding> {
        for pad in &self.pads {
            if let Some(b) = pad.pressed.iter().next() {
                return Some(Binding::Pad(*b));
            }
            for axis in AXES {
                for dir in [AxisDir::Positive, AxisDir::Negative] {
                    if pad.axis_value(axis, dir) >= AXIS_THRESHOLD
                        && pad.axis_was(axis, dir) < AXIS_THRESHOLD
                    {
                        return Some(Binding::PadAxis(axis, dir));
                    }
                }
            }
        }
        None
    }

    fn free_pad(&self) -> Option<usize> {
        (0..self.pads.len()).find(|&i| self.pads[i].connected && self.pad_owner(i).is_none())
    }

    fn pad_owner(&self, pad: usize) -> Option<usize> {
        if self.main_pad == Some(pad) {
            return Some(0);
        }
        self.extra_players
            .iter()
            .position(|p| p.pad == Some(pad))
            .map(|i| i + 1)
    }

    fn player_pad(&self, player: usize) -> Option<usize> {
        match player {
            0 => self.main_pad,
            n => self.extra_players.get(n - 1).and_then(|p| p.pad),
        }
    }

    fn player_slot(&self, player: usize) -> Option<(&Actions, bool)> {
        match player {
            0 => Some((&self.actions, true)),
            n => self
                .extra_players
                .get(n - 1)
                .map(|p| (&p.actions, p.keyboard)),
        }
    }

    fn check(&self, player: usize, action: impl Action, phase: Phase) -> bool {
        let Some((actions, keyboard)) = self.player_slot(player) else {
            return false;
        };
        let pad = self
            .player_pad(player)
            .and_then(|i| self.pads.get(i))
            .filter(|p| p.connected);
        actions.bindings(action.name()).iter().any(|b| match *b {
            Binding::Key(_) | Binding::Mouse(_) => {
                keyboard
                    && match phase {
                        Phase::Pressed => self.binding_pressed(*b),
                        Phase::Held => self.binding_held(*b),
                        Phase::Released => self.binding_released(*b),
                    }
            }
            Binding::Pad(button) => pad.is_some_and(|p| match phase {
                Phase::Pressed => p.pressed.contains(&button),
                Phase::Held => p.down.contains(&button),
                Phase::Released => p.released.contains(&button),
            }),
            Binding::PadAxis(axis, dir) => pad.is_some_and(|p| {
                let (now, was) = (p.axis_value(axis, dir), p.axis_was(axis, dir));
                match phase {
                    Phase::Pressed => now >= AXIS_THRESHOLD && was < AXIS_THRESHOLD,
                    Phase::Held => now >= AXIS_THRESHOLD,
                    Phase::Released => now < AXIS_THRESHOLD && was >= AXIS_THRESHOLD,
                }
            }),
        })
    }

    fn value(&self, player: usize, action: impl Action) -> f32 {
        let Some((actions, _)) = self.player_slot(player) else {
            return 0.0;
        };
        let pad = self.player_pad(player).and_then(|i| self.pads.get(i));
        let analog = actions
            .bindings(action.name())
            .iter()
            .filter_map(|b| match (*b, pad) {
                (Binding::PadAxis(axis, dir), Some(p)) => Some(p.axis_value(axis, dir)),
                _ => None,
            })
            .fold(0.0, f32::max);
        match (analog > 0.0, self.check(player, action, Phase::Held)) {
            (true, _) => analog,
            (false, true) => 1.0,
            (false, false) => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::KeyCode;

    #[derive(ecs_macros::Action, Clone, Copy)]
    enum Act {
        Jump,
        Right,
    }

    fn input() -> Input {
        Input {
            actions: Actions::default()
                .declare(
                    Act::Jump,
                    &[Binding::Key(KeyCode::Space), Binding::Pad(PadButton::South)],
                )
                .declare(
                    Act::Right,
                    &[Binding::PadAxis(PadAxis::LeftX, AxisDir::Positive)],
                ),
            ..Default::default()
        }
    }

    #[test]
    fn each_player_reads_only_their_pad() {
        let mut i = input();
        let p2 = i.add_player(false);
        i.pad_connected(0);
        i.pad_connected(1);
        i.set_pad_button(1, PadButton::South, true);
        assert!(!i.player(0).pressed(Act::Jump));
        assert!(i.player(p2).pressed(Act::Jump));
    }

    #[test]
    fn keyboard_only_reaches_keyboard_players() {
        let mut i = input();
        let p2 = i.add_player(false);
        i.set_key(KeyCode::Space, true);
        assert!(i.pressed(Act::Jump));
        assert!(!i.player(p2).pressed(Act::Jump));
    }

    #[test]
    fn axis_acts_as_button_and_keeps_analog_value() {
        let mut i = input();
        i.pad_connected(0);
        i.set_pad_axis(0, PadAxis::LeftX, 0.8);
        assert!(i.pressed(Act::Right));
        assert!((i.player(0).value(Act::Right) - 0.8).abs() < 1e-6);
        i.end_frame();
        assert!(!i.pressed(Act::Right) && i.held(Act::Right));
        i.set_pad_axis(0, PadAxis::LeftX, 0.1);
        assert!(i.released(Act::Right));
        assert_eq!(i.player(0).value(Act::Right), 0.0);
    }

    #[test]
    fn disconnect_frees_the_pad_for_reassignment() {
        let mut i = input();
        i.pad_connected(0);
        assert_eq!(i.player(0).pad(), Some(0));
        i.pad_disconnected(0);
        assert_eq!(i.player(0).pad(), None);
        i.pad_connected(0);
        assert_eq!(i.player(0).pad(), Some(0));
    }

    #[test]
    fn rebinding_captures_pad_input() {
        let mut i = input();
        i.pad_connected(0);
        i.set_pad_button(0, PadButton::North, true);
        assert_eq!(i.capture_binding(), Some(Binding::Pad(PadButton::North)));
    }
}
