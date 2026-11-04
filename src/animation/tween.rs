use glam::{Quat, Vec3};

use crate::animation::Ease;
use crate::audio::queue_sound;
use crate::color::Color;
use crate::ecs::prelude::*;
use crate::render::d3::Renderer;

#[derive(Clone, Copy)]
enum Prop {
    Position,
    Rotation,
    Scale,
    Color,
}

#[derive(Clone, Copy)]
enum Value {
    Vec(Vec3),
    Quat(Quat),
    Color(Color),
}

#[derive(Clone, Copy)]
enum End {
    Current,
    Fixed(Value),
    Relative(Value),
}

#[derive(Clone, Copy)]
struct Track {
    prop: Prop,
    source: End,
    target: End,
    captured: Option<Value>,
    duration: f32,
    delay: f32,
    elapsed: f32,
    ease: Ease,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum OnFinish {
    Keep,
    #[default]
    Remove,
    Despawn,
}

#[derive(Component, Clone, Default)]
pub struct Animation {
    tracks: Vec<Track>,
    on_finish: OnFinish,
    looping: bool,
    ping_pong: bool,
    finish_sound: Option<String>,
}

impl Animation {
    pub fn new() -> Self {
        Animation::default()
    }

    pub fn take_item() -> Self {
        Animation::new()
            .scale(Vec3::ZERO, 0.20)
            .ease(Ease::InQuad)
            .despawn_on_finish()
    }

    pub fn pop_in() -> Self {
        Animation::new().grow_in(0.30).ease(Ease::OutBack)
    }

    pub fn pulse() -> Self {
        Animation::new()
            .scale(Vec3::splat(1.12), 0.55)
            .relative()
            .ease(Ease::InOutQuad)
            .ping_pong()
    }

    pub fn bob() -> Self {
        Animation::new()
            .position(Vec3::new(0.0, 0.25, 0.0), 0.9)
            .relative()
            .ease(Ease::InOutQuad)
            .ping_pong()
    }

    pub fn position(self, to: Vec3, duration: f32) -> Self {
        self.track(Prop::Position, Value::Vec(to), duration)
    }

    pub fn rotation(self, to: Quat, duration: f32) -> Self {
        self.track(Prop::Rotation, Value::Quat(to), duration)
    }

    pub fn scale(self, to: Vec3, duration: f32) -> Self {
        self.track(Prop::Scale, Value::Vec(to), duration)
    }

    pub fn color(self, to: Color, duration: f32) -> Self {
        self.track(Prop::Color, Value::Color(to), duration)
    }

    pub fn grow_in(mut self, duration: f32) -> Self {
        self.tracks.push(Track {
            prop: Prop::Scale,
            source: End::Fixed(Value::Vec(Vec3::ZERO)),
            target: End::Current,
            captured: None,
            duration,
            delay: 0.0,
            elapsed: 0.0,
            ease: Ease::OutQuad,
        });
        self
    }

    fn track(mut self, prop: Prop, to: Value, duration: f32) -> Self {
        self.tracks.push(Track {
            prop,
            source: End::Current,
            target: End::Fixed(to),
            captured: None,
            duration,
            delay: 0.0,
            elapsed: 0.0,
            ease: Ease::OutQuad,
        });
        self
    }

    pub fn relative(mut self) -> Self {
        if let Some(t) = self.tracks.last_mut()
            && let End::Fixed(v) = t.target
        {
            t.target = End::Relative(v);
        }
        self
    }

    pub fn ease(mut self, ease: Ease) -> Self {
        if let Some(t) = self.tracks.last_mut() {
            t.ease = ease;
        }
        self
    }

    pub fn delay(mut self, delay: f32) -> Self {
        if let Some(t) = self.tracks.last_mut() {
            t.delay = delay;
        }
        self
    }

    pub fn looping(mut self) -> Self {
        self.looping = true;
        self
    }

    pub fn ping_pong(mut self) -> Self {
        self.looping = true;
        self.ping_pong = true;
        self
    }

    pub fn on_finish(mut self, on_finish: OnFinish) -> Self {
        self.on_finish = on_finish;
        self
    }

    pub fn despawn_on_finish(self) -> Self {
        self.on_finish(OnFinish::Despawn)
    }

    pub fn finish_sound(mut self, sound: &str) -> Self {
        self.finish_sound = Some(sound.to_string());
        self
    }

    fn finished(&self) -> bool {
        !self.looping
            && self
                .tracks
                .iter()
                .all(|t| t.elapsed >= t.delay + t.duration)
    }

    fn update(&mut self, dt: f32, tf: &mut Transform, color: &mut Color) {
        let (looping, ping_pong) = (self.looping, self.ping_pong);
        for tr in &mut self.tracks {
            let current = match tr.prop {
                Prop::Position => Value::Vec(tf.position),
                Prop::Rotation => Value::Quat(tf.rotation),
                Prop::Scale => Value::Vec(tf.scale),
                Prop::Color => Value::Color(*color),
            };
            let cap = *tr.captured.get_or_insert(current);

            tr.elapsed += dt;
            let t = progress(tr, looping, ping_pong);
            let a = resolve(tr.source, cap, tr.prop);
            let b = resolve(tr.target, cap, tr.prop);

            match (tr.prop, a, b) {
                (Prop::Position, Value::Vec(a), Value::Vec(b)) => tf.position = a.lerp(b, t),
                (Prop::Scale, Value::Vec(a), Value::Vec(b)) => tf.scale = a.lerp(b, t),
                (Prop::Rotation, Value::Quat(a), Value::Quat(b)) => tf.rotation = a.slerp(b, t),
                (Prop::Color, Value::Color(a), Value::Color(b)) => *color = a.lerp(b, t),
                _ => {}
            }
        }
    }
}

fn progress(tr: &Track, looping: bool, ping_pong: bool) -> f32 {
    let raw = (tr.elapsed - tr.delay) / tr.duration.max(1e-4);
    let t = if raw <= 0.0 {
        0.0
    } else if looping {
        if ping_pong {
            let c = raw % 2.0;
            if c > 1.0 { 2.0 - c } else { c }
        } else {
            raw % 1.0
        }
    } else {
        raw.min(1.0)
    };
    tr.ease.apply(t)
}

fn resolve(end: End, captured: Value, prop: Prop) -> Value {
    match end {
        End::Current => captured,
        End::Fixed(v) => v,
        End::Relative(v) => combine(captured, v, prop),
    }
}

fn combine(c: Value, v: Value, prop: Prop) -> Value {
    match (prop, c, v) {
        (Prop::Position, Value::Vec(c), Value::Vec(v)) => Value::Vec(c + v),
        (Prop::Scale, Value::Vec(c), Value::Vec(v)) => Value::Vec(c * v),
        (Prop::Rotation, Value::Quat(c), Value::Quat(v)) => Value::Quat(c * v),
        _ => v,
    }
}

pub fn animate(world: &mut World) -> SysResult {
    let dt = world.resource::<Time>()?.dt;
    let ents: Vec<Entity> = world.query::<(&Animation,)>().map(|(e, _)| e).collect();

    let mut despawn = Vec::new();
    let mut remove = Vec::new();

    for e in ents {
        let Some(mut tf) = world.get::<Transform>(e).copied() else {
            continue;
        };
        let mut color = world
            .get::<Renderer>(e)
            .and_then(|r| r.slots.first().map(|s| s.material.color))
            .unwrap_or(Color::WHITE);

        let (finished, on_finish, finish_sound) = {
            let Some(anim) = world.get_mut::<Animation>(e) else {
                continue;
            };
            anim.update(dt, &mut tf, &mut color);
            (anim.finished(), anim.on_finish, anim.finish_sound.clone())
        };
        if let Some(t) = world.get_mut::<Transform>(e) {
            *t = tf;
        }
        if let Some(r) = world.get_mut::<Renderer>(e) {
            for s in &mut r.slots {
                s.material.color = color;
            }
        }
        if finished {
            if let Some(name) = finish_sound {
                queue_sound(&name);
            }
            match on_finish {
                OnFinish::Despawn => despawn.push(e),
                OnFinish::Remove => remove.push(e),
                OnFinish::Keep => {}
            }
        }
    }

    for e in remove {
        world.remove::<Animation>(e);
    }
    for e in despawn {
        world.despawn(e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_shrinks_then_finishes() {
        let mut anim = Animation::new()
            .scale(Vec3::ZERO, 1.0)
            .ease(Ease::Linear)
            .despawn_on_finish();
        let mut tf = Transform::from_scale(Vec3::ONE);
        let mut col = Color::WHITE;

        anim.update(0.5, &mut tf, &mut col);
        assert!(tf.scale.x > 0.4 && tf.scale.x < 0.6);
        assert!(!anim.finished());

        anim.update(0.6, &mut tf, &mut col);
        assert_eq!(tf.scale, Vec3::ZERO);
        assert!(anim.finished());
        assert_eq!(anim.on_finish, OnFinish::Despawn);
    }

    #[test]
    fn pop_in_grows_to_captured_scale() {
        let mut anim = Animation::pop_in();
        let mut tf = Transform::from_scale(Vec3::splat(2.0));
        let mut col = Color::WHITE;

        anim.update(0.30, &mut tf, &mut col);
        assert!((tf.scale.x - 2.0).abs() < 1e-3);
        assert!(anim.finished());
    }

    #[test]
    fn pulse_is_relative_and_loops() {
        let mut anim = Animation::pulse();
        let mut tf = Transform::from_scale(Vec3::splat(2.0));
        let mut col = Color::WHITE;

        anim.update(0.275, &mut tf, &mut col);
        assert!(tf.scale.x > 2.0 && tf.scale.x < 2.24);
        assert!(!anim.finished());
    }

    #[test]
    fn color_lerps_to_target() {
        let mut anim = Animation::new().color(Color::BLACK, 1.0).ease(Ease::Linear);
        let mut tf = Transform::default();
        let mut col = Color::WHITE;

        anim.update(1.0, &mut tf, &mut col);
        assert!(col.r < 0.01 && col.g < 0.01 && col.b < 0.01);
        assert!(anim.finished());
    }
}
