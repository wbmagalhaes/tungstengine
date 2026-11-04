use tungstengine::ecs::prelude::*;
use tungstengine::ui::{Rect, UI, button};

pub type ModuleFrame = fn(&mut World, &mut UI);

const TAB_W: f32 = 120.0;
const TAB_H: f32 = 26.0;
const TAB_Z: f32 = 100.0;

#[derive(Clone, Copy)]
pub struct EditorModule {
    pub name: &'static str,
    pub frame: ModuleFrame,
}

#[derive(Resource, Default)]
pub struct EditorShell {
    modules: Vec<EditorModule>,
    active: usize,
}

impl EditorShell {
    pub fn add(&mut self, name: &'static str, frame: ModuleFrame) -> &mut Self {
        self.modules.push(EditorModule { name, frame });
        self
    }

    pub fn active(&self) -> Option<EditorModule> {
        self.modules.get(self.active).copied()
    }

    pub fn select(&mut self, name: &str) -> bool {
        match self.modules.iter().position(|m| m.name == name) {
            Some(i) => {
                self.active = i;
                true
            }
            None => false,
        }
    }
}

#[cfg(feature = "2d")]
pub struct Editor2d {
    pub title: &'static str,
    pub width: f32,
    pub height: f32,
    pub modules: &'static [EditorModule],
    pub setup: fn(&mut World),
}

#[cfg(feature = "2d")]
pub fn run_2d(editor: Editor2d) -> Result<(), winit::error::EventLoopError> {
    use tungstengine::runtime::d2::{App, WindowConfig};
    let mut world = World::new();
    world.insert_resource(WindowConfig {
        title: editor.title.into(),
        width: editor.width,
        height: editor.height,
        fullscreen: false,
    });
    world.insert_resource(Time::default());
    (editor.setup)(&mut world);
    let mut shell = EditorShell::default();
    for m in editor.modules {
        shell.add(m.name, m.frame);
    }
    world.insert_resource(shell);
    App::new(world, frame).run()
}

pub fn frame(world: &mut World, ui: &mut UI) {
    let Ok(shell) = world.resource::<EditorShell>() else {
        return;
    };
    let Some(module) = shell.active() else {
        return;
    };
    let tabs: Vec<&'static str> = shell.modules.iter().map(|m| m.name).collect();
    (module.frame)(world, ui);
    if tabs.len() < 2 {
        return;
    }
    let right = ui.width - tabs.len() as f32 * (TAB_W + 4.0);
    for (i, name) in tabs.into_iter().enumerate() {
        let r = Rect::new(right + i as f32 * (TAB_W + 4.0), 4.0, TAB_W, TAB_H).with_z(TAB_Z);
        if button(ui, r, name, true, &format!("editor-tab-{i}"))
            && let Ok(s) = world.resource_mut::<EditorShell>()
        {
            s.active = i;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(_: &mut World, _: &mut UI) {}

    #[test]
    fn selects_modules_by_name() {
        let mut s = EditorShell::default();
        s.add("Scene", a).add("Data", a);
        assert_eq!(s.active().map(|m| m.name), Some("Scene"));
        assert!(s.select("Data"));
        assert_eq!(s.active().map(|m| m.name), Some("Data"));
        assert!(!s.select("Nope"));
    }
}
