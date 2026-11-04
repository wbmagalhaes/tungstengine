use super::sprite::DrawRect;
use crate::animation::{Ease, Tween};

#[derive(Clone, Copy)]
struct Step {
    dur: f32,
    ease: Ease,
    offset: [f32; 2],
    scale: f32,
    alpha: f32,
}

impl Step {
    fn frame(&self, t: f32) -> AnimFrame {
        AnimFrame {
            offset: [self.offset[0] * (1.0 - t), self.offset[1] * (1.0 - t)],
            scale: self.scale + (1.0 - self.scale) * t,
            alpha: self.alpha + (1.0 - self.alpha) * t,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Anim {
    enter: Step,
    exit: Option<Step>,
}

impl Anim {
    pub fn new(dur: f32) -> Self {
        Anim {
            enter: Step {
                dur,
                ease: Ease::OutCubic,
                offset: [0.0, 0.0],
                scale: 1.0,
                alpha: 1.0,
            },
            exit: None,
        }
    }

    pub fn ease(mut self, ease: Ease) -> Self {
        self.enter.ease = ease;
        self
    }

    pub fn fade(mut self) -> Self {
        self.enter.alpha = 0.0;
        self
    }

    pub fn slide(mut self, x: f32, y: f32) -> Self {
        self.enter.offset = [x, y];
        self
    }

    pub fn rise(self, dy: f32) -> Self {
        self.slide(0.0, dy)
    }

    pub fn pop(mut self, scale: f32) -> Self {
        self.enter.scale = scale;
        self
    }

    pub fn exit(mut self, exit: Anim) -> Self {
        self.exit = Some(exit.enter);
        self
    }

    fn exit_step(&self) -> Step {
        self.exit.unwrap_or(Step {
            ease: Ease::InQuad,
            ..self.enter
        })
    }
}

#[derive(Clone, Copy)]
pub struct AnimFrame {
    offset: [f32; 2],
    scale: f32,
    alpha: f32,
}

pub fn bbox_center(draws: &[DrawRect]) -> [f32; 2] {
    if draws.is_empty() {
        return [0.0, 0.0];
    }
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    for d in draws.iter() {
        min_x = min_x.min(d.min[0]);
        min_y = min_y.min(d.min[1]);
        max_x = max_x.max(d.max[0]);
        max_y = max_y.max(d.max[1]);
    }
    [(min_x + max_x) * 0.5, (min_y + max_y) * 0.5]
}

pub fn apply_anim(draws: &mut [DrawRect], frame: AnimFrame, center: [f32; 2], ancestor: [f32; 4]) {
    let [cx, cy] = center;
    let [ox, oy] = frame.offset;
    let s = frame.scale;
    for d in draws.iter_mut() {
        d.min[0] = cx + (d.min[0] - cx) * s + ox;
        d.max[0] = cx + (d.max[0] - cx) * s + ox;
        d.min[1] = cy + (d.min[1] - cy) * s + oy;
        d.max[1] = cy + (d.max[1] - cy) * s + oy;
        if let Some(q) = d.quad.as_mut() {
            for p in q.iter_mut() {
                p[0] = cx + (p[0] - cx) * s + ox;
                p[1] = cy + (p[1] - cy) * s + oy;
            }
        }
        d.color[3] *= frame.alpha;
        if d.clip != ancestor && d.clip[2].is_finite() && d.clip[3].is_finite() {
            d.clip[0] = cx + (d.clip[0] - cx) * s + ox;
            d.clip[1] = cy + (d.clip[1] - cy) * s + oy;
            d.clip[2] *= s;
            d.clip[3] *= s;
        }
    }
}

pub struct Presence {
    pub tween: Tween<f32>,
    pub target: f32,
    pub anim: Anim,
    pub exitable: bool,
    pub present: bool,
    pub draws: Vec<DrawRect>,
    pub ancestor: [f32; 4],
}

impl Presence {
    pub fn new(anim: Anim, exitable: bool) -> Self {
        Presence {
            tween: Tween::new(0.0, 1.0, anim.enter.dur).ease(anim.enter.ease),
            target: 1.0,
            anim,
            exitable,
            present: true,
            draws: Vec::new(),
            ancestor: [0.0, 0.0, f32::INFINITY, f32::INFINITY],
        }
    }

    pub fn snapshot_center(&self) -> [f32; 2] {
        bbox_center(&self.draws)
    }

    pub fn retarget(&mut self, to: f32) {
        if self.target != to {
            let from = self.tween.value();
            let step = if to > 0.0 {
                self.anim.enter
            } else {
                self.anim.exit_step()
            };
            self.tween = Tween::new(from, to, step.dur).ease(step.ease);
            self.target = to;
        }
    }

    pub fn frame(&self) -> AnimFrame {
        let step = if self.target > 0.0 {
            self.anim.enter
        } else {
            self.anim.exit_step()
        };
        step.frame(self.tween.value())
    }
}
