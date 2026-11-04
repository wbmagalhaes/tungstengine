use glam::{Mat4, Quat, Vec3};

use crate::ecs::{Component, Entity, World};

#[derive(Component, Clone, Copy)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Transform {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn from_pos(pos: Vec3) -> Self {
        Transform {
            position: pos,
            ..Default::default()
        }
    }

    pub fn from_pos_rot(pos: Vec3, rot: Quat) -> Self {
        Transform {
            position: pos,
            rotation: rot,
            ..Default::default()
        }
    }

    pub fn from_scale(scale: Vec3) -> Self {
        Transform {
            scale,
            ..Default::default()
        }
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

#[derive(Component, Clone, Copy)]
pub struct GlobalTransform(pub Mat4);

impl Default for GlobalTransform {
    fn default() -> Self {
        GlobalTransform(Mat4::IDENTITY)
    }
}

impl GlobalTransform {
    pub const IDENTITY: GlobalTransform = GlobalTransform(Mat4::IDENTITY);

    pub fn matrix(&self) -> Mat4 {
        self.0
    }

    pub fn position(&self) -> Vec3 {
        self.0.to_scale_rotation_translation().2
    }

    pub fn rotation(&self) -> Quat {
        self.0.to_scale_rotation_translation().1
    }

    pub fn scale(&self) -> Vec3 {
        self.0.to_scale_rotation_translation().0
    }
}

#[derive(Component, Clone, Copy)]
pub struct Parent(pub Entity);

#[derive(Component, Clone, Default)]
pub struct Children(pub Vec<Entity>);

#[derive(Component, Clone, Copy, Default)]
pub struct Static;

#[derive(Component, Clone, Copy, Default)]
pub struct Baked;

#[derive(Component, Clone, Copy, Default, Debug, PartialEq)]
pub struct RenderOffset(pub Vec3);

pub fn render_offset(world: &World, e: Entity) -> Vec3 {
    let mut total = Vec3::ZERO;
    let mut node = Some(e);
    while let Some(n) = node {
        total += world.get::<RenderOffset>(n).map_or(Vec3::ZERO, |o| o.0);
        node = world.get::<Parent>(n).map(|p| p.0);
    }
    total
}
