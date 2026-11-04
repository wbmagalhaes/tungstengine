use glam::Vec3;

use crate::color::Color;
use crate::ecs::Component;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Mesh(pub u32);

impl Mesh {
    pub const CUBE: Mesh = Mesh(0);
    pub const SPHERE: Mesh = Mesh(1);
    pub const PYRAMID: Mesh = Mesh(2);
    pub const DISK: Mesh = Mesh(3);
    pub const CYLINDER: Mesh = Mesh(4);
    pub const CAPSULE: Mesh = Mesh(5);
    pub const PLANE: Mesh = Mesh(6);

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Shader(pub u32);

impl Shader {
    pub const UNLIT: Shader = Shader(0);
    pub const LIT: Shader = Shader(1);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Faces {
    Front,
    Back,
    Both,
}

impl Faces {
    pub const COUNT: usize = 3;

    pub fn index(self) -> usize {
        match self {
            Faces::Front => 0,
            Faces::Back => 1,
            Faces::Both => 2,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Material {
    pub color: Color,
    pub shader: Shader,
    pub faces: Faces,
    pub metallic: f32,
    pub roughness: f32,
    pub uv_offset: [f32; 2],
    pub uv_scale: [f32; 2],
    pub params: [f32; 16],
}

impl Material {
    pub const LIT: Material = Material {
        color: Color::WHITE,
        shader: Shader::LIT,
        faces: Faces::Front,
        metallic: 0.0,
        roughness: 1.0,
        uv_offset: [0.0, 0.0],
        uv_scale: [1.0, 1.0],
        params: [0.0; 16],
    };
    pub const UNLIT: Material = Material {
        color: Color::WHITE,
        shader: Shader::UNLIT,
        faces: Faces::Front,
        metallic: 0.0,
        roughness: 1.0,
        uv_offset: [0.0, 0.0],
        uv_scale: [1.0, 1.0],
        params: [0.0; 16],
    };

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn faces(mut self, faces: Faces) -> Self {
        self.faces = faces;
        self
    }
}

#[derive(Component, Clone, Copy)]
pub struct Camera {
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub offset: Vec3,
    pub smooth: f32,
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            fov: std::f32::consts::FRAC_PI_4,
            near: 0.1,
            far: 100.0,
            offset: Vec3::new(0.0, 4.5, 7.0),
            smooth: 6.0,
            zoom: 1.0,
        }
    }
}

#[derive(Component)]
pub struct MainCam;

#[derive(Clone, Copy)]
pub struct RenderSlot {
    pub mesh: Mesh,
    pub material: Material,
    pub bone: Option<usize>,
}

#[derive(Component, Clone)]
pub struct Renderer {
    pub slots: Vec<RenderSlot>,
    pub cast_shadows: bool,
    pub outline_occlude: bool,
}

impl Renderer {
    pub fn new(mesh: Mesh, material: Material) -> Self {
        Renderer {
            slots: vec![RenderSlot {
                mesh,
                material,
                bone: None,
            }],
            cast_shadows: true,
            outline_occlude: true,
        }
    }

    pub fn no_outline_occlude(mut self) -> Self {
        self.outline_occlude = false;
        self
    }
}

#[derive(Component, Clone, Copy)]
pub struct Outline {
    pub color: Color,
    pub width: f32,
}

impl Default for Outline {
    fn default() -> Self {
        Outline {
            color: Color::rgb(255, 240, 180),
            width: 3.0,
        }
    }
}

#[derive(Component)]
pub struct Hovered;
