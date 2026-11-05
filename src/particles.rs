use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::ecs::prelude::*;
use crate::rng::Rng;
use crate::sprites::{SpriteArg, SpriteKey};
use crate::ui::{Sprite, SpriteDraw};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
pub struct Span(pub f32, pub f32);

impl Span {
    pub fn fixed(value: f32) -> Self {
        Span(value, value)
    }

    fn sample(self, rng: &mut Rng) -> f32 {
        self.0 + (self.1 - self.0) * rng.unit()
    }
}

impl From<f32> for Span {
    fn from(value: f32) -> Self {
        Span::fixed(value)
    }
}

impl From<(f32, f32)> for Span {
    fn from((min, max): (f32, f32)) -> Self {
        Span(min, max)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum EmitShape {
    #[default]
    Point,
    Box {
        size: [f32; 2],
    },
    Circle {
        radius: f32,
    },
}

impl EmitShape {
    fn sample(self, rng: &mut Rng) -> Vec2 {
        match self {
            EmitShape::Point => Vec2::ZERO,
            EmitShape::Box { size } => {
                Vec2::new(rng.unit() - 0.5, rng.unit() - 0.5) * Vec2::from(size)
            }
            EmitShape::Circle { radius } => {
                let angle = rng.unit() * std::f32::consts::TAU;
                Vec2::from_angle(angle) * radius * rng.unit().sqrt()
            }
        }
    }
}

fn sprite_by_name<'de, D: serde::Deserializer<'de>>(d: D) -> Result<SpriteDraw, D::Error> {
    let name = String::deserialize(d)?;
    Ok(match Sprite::from_name(&name) {
        Some(sprite) => sprite.draw(),
        None => name.draw(),
    })
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct ParticleEffect {
    pub count: u32,
    pub rate: f32,
    pub life: Span,
    pub speed: Span,
    pub angle: Span,
    pub size: Span,
    pub gravity: f32,
    pub drag: f32,
    pub end_size: f32,
    pub end_alpha: f32,
    pub stretch: f32,
    pub align: bool,
    pub colors: Vec<Color>,
    #[serde(skip_serializing, deserialize_with = "sprite_by_name")]
    pub sprite: SpriteDraw,
    pub layer: i32,
    pub shape: EmitShape,
}

impl Default for ParticleEffect {
    fn default() -> Self {
        ParticleEffect {
            count: 0,
            rate: 0.0,
            life: Span::fixed(1.0),
            speed: Span::default(),
            angle: Span(0.0, 360.0),
            size: Span::fixed(0.2),
            gravity: 0.0,
            drag: 0.0,
            end_size: 0.0,
            end_alpha: 0.0,
            stretch: 1.0,
            align: false,
            colors: vec![Color::WHITE],
            sprite: Sprite::RECT.draw(),
            layer: 0,
            shape: EmitShape::Point,
        }
    }
}

impl ParticleEffect {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }

    pub fn rate(mut self, rate: f32) -> Self {
        self.rate = rate;
        self
    }

    pub fn life(mut self, life: impl Into<Span>) -> Self {
        self.life = life.into();
        self
    }

    pub fn speed(mut self, speed: impl Into<Span>) -> Self {
        self.speed = speed.into();
        self
    }

    pub fn angle(mut self, angle: impl Into<Span>) -> Self {
        self.angle = angle.into();
        self
    }

    pub fn size(mut self, size: impl Into<Span>) -> Self {
        self.size = size.into();
        self
    }

    pub fn gravity(mut self, gravity: f32) -> Self {
        self.gravity = gravity;
        self
    }

    pub fn drag(mut self, drag: f32) -> Self {
        self.drag = drag;
        self
    }

    pub fn end_size(mut self, end_size: f32) -> Self {
        self.end_size = end_size;
        self
    }

    pub fn end_alpha(mut self, end_alpha: f32) -> Self {
        self.end_alpha = end_alpha;
        self
    }

    pub fn stretch(mut self, stretch: f32) -> Self {
        self.stretch = stretch;
        self
    }

    pub fn align(mut self) -> Self {
        self.align = true;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.colors = vec![color];
        self
    }

    pub fn colors(mut self, colors: impl IntoIterator<Item = Color>) -> Self {
        self.colors = colors.into_iter().collect();
        self
    }

    pub fn layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    pub fn shape(mut self, shape: EmitShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn sprite(mut self, sprite: impl SpriteArg) -> Self {
        self.sprite = sprite.resolve();
        self
    }

    pub fn parse(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }
}

#[derive(Component, Clone, Debug)]
pub struct ParticleEmitter {
    pub effect: ParticleEffect,
    pub active: bool,
    pub offset: Vec2,
    pub velocity: Vec2,
    pending: u32,
    once: bool,
    carry: f32,
}

impl ParticleEmitter {
    pub fn stream(effect: ParticleEffect) -> Self {
        ParticleEmitter {
            effect,
            active: true,
            offset: Vec2::ZERO,
            velocity: Vec2::ZERO,
            pending: 0,
            once: false,
            carry: 0.0,
        }
    }

    pub fn burst(effect: ParticleEffect) -> Self {
        ParticleEmitter {
            pending: effect.count,
            once: true,
            active: false,
            ..Self::stream(effect)
        }
    }

    pub fn offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    pub fn fire(&mut self) {
        self.pending += self.effect.count;
    }
}

pub fn emit_burst(world: &mut World, pos: Vec2, effect: ParticleEffect) -> Entity {
    let e = world.spawn();
    world.insert(e, Transform::from_pos(pos.extend(0.0)));
    world.insert(e, ParticleEmitter::burst(effect));
    e
}

#[derive(Clone, Copy)]
pub struct LiveParticle {
    pub pos: Vec2,
    pub velocity: Vec2,
    pub age: f32,
    pub life: f32,
    pub size: f32,
    pub end_size: f32,
    pub end_alpha: f32,
    pub stretch: f32,
    pub align: bool,
    pub gravity: f32,
    pub drag: f32,
    pub color: Color,
    pub draw: SpriteDraw,
    pub layer: i32,
}

impl LiveParticle {
    pub fn current_size(&self) -> f32 {
        let t = self.age / self.life;
        self.size * (1.0 + (self.end_size - 1.0) * t)
    }

    pub fn corners(&self) -> [Vec2; 4] {
        let size = self.current_size();
        let along = if self.align {
            self.velocity.normalize_or(Vec2::X)
        } else {
            Vec2::X
        };
        let length = along * size * self.stretch / 2.0;
        let width = along.perp() * size / 2.0;
        [
            self.pos - length - width,
            self.pos + length - width,
            self.pos - length + width,
            self.pos + length + width,
        ]
    }

    pub fn current_alpha(&self) -> f32 {
        let t = self.age / self.life;
        self.color.a * (1.0 + (self.end_alpha - 1.0) * t)
    }
}

#[derive(Resource, Default)]
#[resource(core)]
pub struct Particles {
    live: Vec<LiveParticle>,
    rng: Rng,
}

impl Particles {
    pub fn iter(&self) -> impl Iterator<Item = &LiveParticle> {
        self.live.iter()
    }

    pub fn len(&self) -> usize {
        self.live.len()
    }

    pub fn is_empty(&self) -> bool {
        self.live.is_empty()
    }

    fn spawn(&mut self, effect: &ParticleEffect, origin: Vec2, inherited: Vec2, draw: SpriteDraw) {
        let rng = &mut self.rng;
        let angle = effect.angle.sample(rng).to_radians();
        let velocity = Vec2::from_angle(angle) * effect.speed.sample(rng) + inherited;
        let pick = rng.below(effect.colors.len());
        let particle = LiveParticle {
            pos: origin + effect.shape.sample(rng),
            velocity,
            age: 0.0,
            life: effect.life.sample(rng).max(0.001),
            size: effect.size.sample(rng),
            end_size: effect.end_size,
            end_alpha: effect.end_alpha,
            stretch: effect.stretch,
            align: effect.align,
            gravity: effect.gravity,
            drag: effect.drag,
            color: effect.colors.get(pick).copied().unwrap_or(Color::WHITE),
            draw,
            layer: effect.layer,
        };
        self.live.push(particle);
    }

    fn advance(&mut self, dt: f32) {
        for p in &mut self.live {
            p.age += dt;
            p.velocity.y += p.gravity * dt;
            p.velocity *= (-p.drag * dt).exp();
            p.pos += p.velocity * dt;
        }
        self.live.retain(|p| p.age < p.life);
    }
}

impl World {
    pub fn add_particles(&mut self) {
        self.add_system_to(Stage::Late, particles_step);
    }
}

pub fn particles_step(world: &mut World) -> SysResult {
    let dt = world.dt();
    let parked = Particles {
        live: Vec::new(),
        rng: Rng(1),
    };

    let mut pool = std::mem::replace(world.resource_core_mut::<Particles>(), parked);
    let mut finished = Vec::new();
    for (e, emitter, t) in world.query_mut::<(&mut ParticleEmitter, &Transform)>() {
        let mut count = std::mem::take(&mut emitter.pending);
        if emitter.active {
            emitter.carry += emitter.effect.rate * dt;
            let whole = emitter.carry.floor();
            emitter.carry -= whole;
            count += whole as u32;
        }
        let origin = t.position.truncate() + emitter.offset;
        let draw = emitter.effect.sprite;
        for _ in 0..count {
            pool.spawn(&emitter.effect, origin, emitter.velocity, draw);
        }
        if emitter.once {
            finished.push(e);
        }
    }
    for e in finished {
        world.despawn(e);
    }
    pool.advance(dt);
    *world.resource_core_mut::<Particles>() = pool;
    Ok(())
}

pub mod prelude {
    pub use super::{
        EmitShape, ParticleEffect, ParticleEmitter, Particles, Span, emit_burst, particles_step,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Time;

    fn world(dt: f32) -> World {
        let mut w = World::new();
        w.insert_resource(Time {
            dt,
            ..Default::default()
        });
        w
    }

    fn live(w: &World) -> usize {
        w.resource_core::<Particles>().len()
    }

    #[test]
    fn burst_fires_once_and_removes_its_emitter() {
        let mut w = world(0.1);
        let effect = ParticleEffect {
            count: 5,
            ..Default::default()
        };
        let e = emit_burst(&mut w, Vec2::ZERO, effect);
        particles_step(&mut w).unwrap();
        assert_eq!(live(&w), 5);
        assert!(!w.has::<ParticleEmitter>(e));
        for _ in 0..10 {
            particles_step(&mut w).unwrap();
        }
        assert_eq!(live(&w), 0);
    }

    #[test]
    fn stream_follows_its_rate_and_stops_when_inactive() {
        let mut w = world(0.1);
        let e = w.spawn();
        w.insert(e, Transform::default());
        let effect = ParticleEffect {
            rate: 25.0,
            life: Span::fixed(100.0),
            ..Default::default()
        };
        w.insert(e, ParticleEmitter::stream(effect));
        for _ in 0..4 {
            particles_step(&mut w).unwrap();
        }
        assert_eq!(live(&w), 10);
        w.get_mut::<ParticleEmitter>(e).unwrap().active = false;
        particles_step(&mut w).unwrap();
        assert_eq!(live(&w), 10);
    }

    #[test]
    fn particles_move_and_fade() {
        let mut w = world(0.25);
        let effect = ParticleEffect {
            count: 1,
            speed: Span::fixed(4.0),
            angle: Span::fixed(0.0),
            size: Span::fixed(1.0),
            ..Default::default()
        };
        emit_burst(&mut w, Vec2::ZERO, effect);
        particles_step(&mut w).unwrap();
        let pool = w.resource_core::<Particles>();
        let p = pool.iter().next().unwrap();
        assert!((p.pos.x - 1.0).abs() < 1e-4);
        assert_eq!(p.current_size(), 0.75);
        assert_eq!(p.current_alpha(), 0.75);
    }

    #[test]
    fn aligned_particles_stretch_along_their_motion() {
        let mut w = world(0.01);
        let effect = ParticleEffect {
            count: 1,
            speed: Span::fixed(4.0),
            angle: Span::fixed(90.0),
            size: Span::fixed(1.0),
            stretch: 3.0,
            align: true,
            life: Span::fixed(100.0),
            ..Default::default()
        };
        emit_burst(&mut w, Vec2::ZERO, effect);
        particles_step(&mut w).unwrap();
        let pool = w.resource_core::<Particles>();
        let [a, b, c, _] = pool.iter().next().unwrap().corners();
        assert!(((b - a).length() - 3.0).abs() < 0.01);
        assert!((b - a).x.abs() < 1e-3);
        assert!(((c - a).length() - 1.0).abs() < 0.01);
    }

    #[test]
    fn effects_load_from_toml() {
        let effect = ParticleEffect::parse(
            r##"
            count = 18
            life = [0.4, 0.8]
            speed = [3.0, 7.0]
            colors = ["#ffffff", "#fcd34d"]
            sprite = "circle"
            shape = { kind = "box", size = [0.5, 0.0] }
            "##,
        )
        .unwrap();
        assert_eq!(effect.count, 18);
        assert_eq!(effect.life, Span(0.4, 0.8));
        assert_eq!(effect.colors[0], Color::hex("#ffffff"));
        assert_eq!(effect.shape, EmitShape::Box { size: [0.5, 0.0] });
        assert_eq!(effect.gravity, 0.0);
        assert_eq!(effect.sprite, Sprite::CIRCLE.draw());
    }

    #[test]
    fn effects_survive_a_toml_round_trip() {
        let effect = ParticleEffect {
            colors: vec![Color::hex("#fcd34d")],
            ..Default::default()
        };
        let text = toml::to_string(&effect).unwrap();
        assert_eq!(ParticleEffect::parse(&text).unwrap(), effect);
    }
}
