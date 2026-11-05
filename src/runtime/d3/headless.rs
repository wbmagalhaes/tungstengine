use winit::keyboard::KeyCode;

use crate::ecs::prelude::*;
use crate::input::Input;
use crate::ui::{TextRec, UI};

use super::app::{FrameUi, install_core, tick};

pub struct Headless {
    pub world: World,
    ui: UI,
    frame_ui: FrameUi,
    pub width: f32,
    pub height: f32,
    pub dt: f32,
}

impl Headless {
    pub fn new(mut world: World, frame_ui: FrameUi, width: f32, height: f32) -> Self {
        install_core(&mut world);
        let mut ui = UI::new();
        ui.record_texts = true;
        let mut h = Headless {
            world,
            ui,
            frame_ui,
            width,
            height,
            dt: 1.0 / 60.0,
        };
        h.frame();
        h
    }

    pub fn frame(&mut self) {
        tick(
            &mut self.world,
            &mut self.ui,
            self.frame_ui,
            self.width,
            self.height,
            self.dt,
        );
    }

    pub fn frames(&mut self, n: u64) {
        for _ in 0..n {
            self.frame();
        }
    }

    pub fn wait(&mut self, secs: f32) {
        self.frames((secs / self.dt).ceil().max(1.0) as u64);
    }

    pub fn key(&mut self, key: KeyCode, down: bool) {
        if let Ok(input) = self.world.resource_mut::<Input>() {
            input.set_key(key, down);
        }
    }

    pub fn find_text(&self, label: &str) -> Option<&TextRec> {
        self.ui.texts.iter().find(|t| t.text.contains(label))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::d3::{Body3D, Collider3D, Physics3D};
    use glam::Vec3;

    fn hud(world: &mut World, ui: &mut UI) {
        let y = world
            .query::<(&Body3D, &Transform)>()
            .next()
            .map_or(0.0, |(_, _, t)| t.position.y);
        ui.text(0.0, 0.0, &format!("height {y:.1}"));
    }

    #[test]
    fn a_full_3d_frame_runs_systems_physics_transforms_and_ui() {
        let mut world = World::new();
        world.insert_resource(Physics3D::with_gravity(Vec3::new(0.0, -10.0, 0.0)));
        world.add_physics3d();
        let ground = world.spawn();
        world.insert(ground, Transform::from_pos(Vec3::new(0.0, -0.5, 0.0)));
        world.insert(ground, Collider3D::cuboid(Vec3::new(20.0, 1.0, 20.0)));
        let ball = world.spawn();
        world.insert(ball, Transform::from_pos(Vec3::new(0.0, 3.0, 0.0)));
        world.insert(ball, Collider3D::sphere(0.5));
        world.insert(ball, Body3D::default());

        let mut h = Headless::new(world, hud, 1280.0, 720.0);
        h.wait(2.0);
        let g = h.world.get::<GlobalTransform>(ball).unwrap();
        assert!((g.position().y - 0.5).abs() < 0.02);
        assert!(h.world.get::<Body3D>(ball).unwrap().on_ground);
        assert!(h.find_text("height 0.5").is_some());
    }
}
