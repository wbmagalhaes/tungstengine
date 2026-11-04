use serde::{Deserialize, Serialize};

use crate::animation::Ease;
use crate::ecs::Resource;
use crate::texture::Texture2D;

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(default)]
pub struct TransitionDef {
    pub dur: f32,
    pub hold: f32,
    pub color: [f32; 4],
    pub soft: f32,
    pub ease: Ease,
}

impl Default for TransitionDef {
    fn default() -> Self {
        TransitionDef {
            dur: 0.60,
            hold: 1.0,
            color: [0.02, 0.02, 0.03, 1.0],
            soft: 0.02,
            ease: Ease::InOutCubic,
        }
    }
}

const MASK_SIZE: u32 = 128;

#[derive(Resource, Clone)]
pub struct TransitionMask(pub Texture2D);

impl Default for TransitionMask {
    fn default() -> Self {
        let half = MASK_SIZE as f32 / 2.0;
        let corner = half * std::f32::consts::SQRT_2;
        let mut rgba = Vec::with_capacity((MASK_SIZE * MASK_SIZE * 4) as usize);
        for y in 0..MASK_SIZE {
            for x in 0..MASK_SIZE {
                let (dx, dy) = (x as f32 + 0.5 - half, y as f32 + 0.5 - half);
                let v = ((dx * dx + dy * dy).sqrt() / corner * 255.0).min(255.0) as u8;
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
        TransitionMask(Texture2D {
            width: MASK_SIZE,
            height: MASK_SIZE,
            rgba,
        })
    }
}

#[derive(Resource)]
pub struct Transition {
    pub def: TransitionDef,
    pub t: f32,
    pub held: f32,
    pub active: bool,
}

impl Default for Transition {
    fn default() -> Self {
        Transition::new(TransitionDef::default())
    }
}

impl Transition {
    pub fn new(def: TransitionDef) -> Self {
        Transition {
            def,
            t: 0.0,
            held: 0.0,
            active: false,
        }
    }

    pub fn start(&mut self) {
        self.active = true;
        self.t = 0.0;
        self.held = 0.0;
    }

    pub fn advance(&mut self, dt: f32) {
        if !self.active {
            return;
        }
        if self.t >= 1.0 && self.held < self.def.hold {
            self.t = 1.0;
            self.held += dt;
            return;
        }
        self.t += dt / self.def.dur.max(1e-4);
        if self.t >= 2.0 {
            self.t = 0.0;
            self.held = 0.0;
            self.active = false;
        }
    }

    pub fn covered(&self) -> bool {
        self.active && self.t >= 1.0
    }

    pub fn cutoff(&self) -> f32 {
        if !self.active {
            return 0.0;
        }
        let x = if self.t <= 1.0 { self.t } else { 2.0 - self.t };
        self.def.ease.apply(x)
    }

    pub fn view(&self) -> TransitionView {
        TransitionView {
            cutoff: self.cutoff(),
            soft: self.def.soft,
            color: self.def.color,
        }
    }
}

#[derive(Default)]
pub struct TransitionView {
    pub cutoff: f32,
    pub soft: f32,
    pub color: [f32; 4],
}
