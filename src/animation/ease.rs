use std::f32::consts::PI;

use serde::{Deserialize, Serialize};

use crate::color::Color;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ease {
    Linear,
    InQuad,
    OutQuad,
    InOutQuad,
    OutCubic,
    InOutCubic,
    OutBack,
    OutElastic,
    OutBounce,
}

fn out_bounce(t: f32) -> f32 {
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

impl Ease {
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Ease::Linear => t,
            Ease::InQuad => t * t,
            Ease::OutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Ease::InOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Ease::OutCubic => 1.0 - (1.0 - t).powi(3),
            Ease::InOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Ease::OutBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
            }
            Ease::OutElastic => {
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    let c4 = (2.0 * PI) / 3.0;
                    2f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
                }
            }
            Ease::OutBounce => out_bounce(t),
        }
    }
}

pub trait Lerp: Copy {
    fn lerp(self, other: Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(self, other: f32, t: f32) -> f32 {
        self + (other - self) * t
    }
}

impl Lerp for [f32; 2] {
    fn lerp(self, other: [f32; 2], t: f32) -> [f32; 2] {
        [
            self[0] + (other[0] - self[0]) * t,
            self[1] + (other[1] - self[1]) * t,
        ]
    }
}

impl Lerp for Color {
    fn lerp(self, other: Color, t: f32) -> Color {
        Color::lerp(self, other, t)
    }
}

#[derive(Clone, Copy)]
pub struct Tween<T> {
    from: T,
    to: T,
    duration: f32,
    elapsed: f32,
    delay: f32,
    ease: Ease,
    loops: bool,
    ping_pong: bool,
}

impl<T: Lerp> Tween<T> {
    pub fn new(from: T, to: T, duration: f32) -> Self {
        Tween {
            from,
            to,
            duration,
            elapsed: 0.0,
            delay: 0.0,
            ease: Ease::OutQuad,
            loops: false,
            ping_pong: false,
        }
    }

    pub fn ease(mut self, ease: Ease) -> Self {
        self.ease = ease;
        self
    }

    pub fn delay(mut self, delay: f32) -> Self {
        self.delay = delay;
        self
    }

    pub fn looping(mut self) -> Self {
        self.loops = true;
        self
    }

    pub fn ping_pong(mut self) -> Self {
        self.loops = true;
        self.ping_pong = true;
        self
    }

    pub fn progress(&self) -> f32 {
        let mut t = (self.elapsed - self.delay) / self.duration;
        if t <= 0.0 {
            return 0.0;
        }
        if self.loops {
            if self.ping_pong {
                let cycle = t % 2.0;
                t = if cycle > 1.0 { 2.0 - cycle } else { cycle };
            } else {
                t %= 1.0;
            }
        } else {
            t = t.min(1.0);
        }
        self.ease.apply(t)
    }

    pub fn value(&self) -> T {
        self.from.lerp(self.to, self.progress())
    }

    pub fn tick(&mut self, dt: f32) -> T {
        self.elapsed += dt;
        self.value()
    }

    pub fn finished(&self) -> bool {
        !self.loops && self.elapsed >= self.delay + self.duration
    }

    pub fn reset(&mut self) {
        self.elapsed = 0.0;
    }
}
