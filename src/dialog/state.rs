use std::path::Path;

use crate::ecs::prelude::*;

use super::actor::{Actor, Actors, load_actors};
use super::runner::{Presentation, YarnRunner};

#[derive(Resource, Default)]
pub struct Dialog {
    runner: Option<YarnRunner>,
    actors: Actors,
    using_runner: bool,
    queue: Vec<String>,
    pending_reveals: Vec<String>,
    pending_flags: Vec<String>,
    pending_sfx: Vec<String>,
    pending_scene: Option<String>,
    pub active: bool,
    pub speaker: Option<Actor>,
    pub text: String,
    pub options: Vec<String>,
    pub elapsed: f32,
    pub auto: bool,
}

impl Dialog {
    pub fn load(dialogs_dir: &Path, actors_path: &Path) -> Self {
        Dialog {
            runner: YarnRunner::from_dir(dialogs_dir),
            actors: load_actors(actors_path),
            ..Default::default()
        }
    }

    pub fn actors(&self) -> &Actors {
        &self.actors
    }

    pub fn take_reveals(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_reveals)
    }

    pub fn take_flags(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_flags)
    }

    pub fn take_sfx(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_sfx)
    }

    pub fn take_scene(&mut self) -> Option<String> {
        self.pending_scene.take()
    }

    pub fn talk(&mut self, node: &str, flags: &[String]) {
        self.queue.clear();
        self.using_runner = true;
        if let Some(r) = self.runner.as_mut() {
            r.set_flags(flags);
        }
        let started = self.runner.as_mut().map(|r| r.start(node)).unwrap_or(false);
        if started {
            self.active = true;
            self.pull();
        } else {
            log::warn!("dialog node `{node}` not found");
            self.active = false;
        }
    }

    pub fn say(&mut self, lines: Vec<String>) {
        self.using_runner = false;
        self.speaker = None;
        self.options.clear();
        self.queue = lines;
        self.active = !self.queue.is_empty();
        self.pop();
    }

    fn pop(&mut self) {
        if self.queue.is_empty() {
            self.active = false;
        } else {
            self.text = self.queue.remove(0);
            self.elapsed = 0.0;
        }
    }

    fn pull(&mut self) {
        let (pres, commands) = {
            let Some(runner) = self.runner.as_mut() else {
                self.active = false;
                return;
            };
            (runner.advance(), runner.drain_commands())
        };
        for cmd in commands {
            if let Some(flag) = cmd.strip_prefix("flag ").map(str::trim) {
                self.pending_flags.push(flag.to_string());
                continue;
            }
            if let Some(sound) = cmd.strip_prefix("sfx ").map(str::trim) {
                self.pending_sfx.push(sound.to_string());
                continue;
            }
            if let Some(scene) = cmd.strip_prefix("change_scene ").map(str::trim) {
                self.pending_scene = Some(scene.to_string());
                continue;
            }
            let Some(target) = cmd.strip_prefix("reveal ").map(str::trim) else {
                continue;
            };
            if let Some(id) = target.strip_prefix("act:") {
                self.actors.reveal(id.trim());
            } else if let Some(id) = target.strip_prefix("obj:") {
                self.pending_reveals.push(id.trim().to_string());
            } else {
                log::warn!("reveal: expected `act:` or `obj:` prefix, got `{target}`");
            }
        }
        match pres {
            Presentation::Line { actor, text } => {
                self.speaker = actor.and_then(|id| self.actors.get(&id).cloned());
                self.text = text;
                self.options.clear();
                self.elapsed = 0.0;
            }
            Presentation::Options(opts) => self.options = opts,
            Presentation::Done => {
                self.active = false;
                self.options.clear();
            }
        }
    }

    pub fn advance(&mut self) {
        if !self.options.is_empty() {
            return;
        }
        if self.using_runner {
            self.pull();
        } else {
            self.pop();
        }
    }

    pub fn choose(&mut self, index: usize) {
        if let Some(runner) = self.runner.as_mut() {
            runner.choose(index);
        }
        self.options.clear();
        self.pull();
    }

    pub fn close(&mut self) {
        self.active = false;
        self.options.clear();
        self.queue.clear();
    }
}
