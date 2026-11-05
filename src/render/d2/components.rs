use glam::{Mat4, Vec2};

use crate::color::Color;
use crate::ecs::prelude::*;
use crate::ui::{Sprite, SpriteDraw};

pub const PIXELS_PER_UNIT: f32 = 64.0;

#[derive(Component, Clone, Copy)]
pub struct SpriteRenderer {
    pub sprite: SpriteDraw,
    pub size: Vec2,
    pub anchor: Vec2,
    pub color: Color,
    pub layer: i32,
    pub flip_x: bool,
    pub visible: bool,
    pub sort_offset: f32,
}

impl Default for SpriteRenderer {
    fn default() -> Self {
        SpriteRenderer {
            sprite: Sprite::RECT.draw(),
            size: Vec2::ONE,
            anchor: Vec2::splat(0.5),
            color: Color::WHITE,
            layer: 0,
            flip_x: false,
            visible: true,
            sort_offset: 0.0,
        }
    }
}

impl SpriteRenderer {
    pub fn new(sprite: SpriteDraw, size: Vec2) -> Self {
        SpriteRenderer {
            sprite,
            size,
            ..Default::default()
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    pub fn anchor(mut self, anchor: Vec2) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn sort_offset(mut self, offset: f32) -> Self {
        self.sort_offset = offset;
        self
    }

    pub fn feet(self) -> Self {
        self.anchor(Vec2::new(0.5, 1.0))
    }
}

#[derive(Component, Clone, Copy)]
pub struct Camera2D {
    pub zoom: f32,
}

impl Default for Camera2D {
    fn default() -> Self {
        Camera2D { zoom: 1.0 }
    }
}

#[derive(Component, Clone, Copy, Default)]
pub struct MainCam;

#[derive(Resource, Clone, Copy)]
pub struct View2D {
    pub pixel_snap: bool,
    pub center: Vec2,
    pub zoom: f32,
    pub screen: Vec2,
}

impl Default for View2D {
    fn default() -> Self {
        View2D {
            pixel_snap: true,
            center: Vec2::ZERO,
            zoom: PIXELS_PER_UNIT,
            screen: Vec2::new(1280.0, 720.0),
        }
    }
}

impl View2D {
    pub fn world_to_screen(&self, p: Vec2) -> Vec2 {
        (p - self.center) * self.zoom + self.screen * 0.5
    }

    pub fn screen_to_world(&self, p: Vec2) -> Vec2 {
        (p - self.screen * 0.5) / self.zoom + self.center
    }

    pub fn visible_half(&self) -> Vec2 {
        self.screen * 0.5 / self.zoom
    }

    pub fn transform(&self) -> Mat4 {
        Mat4::from_translation((self.screen * 0.5).extend(0.0))
            * Mat4::from_scale(glam::Vec3::new(self.zoom, self.zoom, 1.0))
            * Mat4::from_translation((-self.center).extend(0.0))
    }
}

#[derive(Resource, Clone, Copy, Default)]
pub struct CursorWorld(pub Vec2);
