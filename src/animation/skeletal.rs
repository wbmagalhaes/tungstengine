use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};

use crate::ecs::prelude::*;
use crate::render::d3::Renderer;

pub struct Joint {
    pub parent: Option<usize>,
    pub base: Mat4,
    pub bind_local: Mat4,
    pub inv_bind_world: Mat4,
}

#[derive(Default)]
pub struct JointAnim {
    pub trans: Vec<(f32, Vec3)>,
    pub rot: Vec<(f32, Quat)>,
    pub scale: Vec<(f32, Vec3)>,
}

pub struct Clip {
    pub name: String,
    pub duration: f32,
    pub joints: Vec<JointAnim>,
}

pub struct Skeleton {
    pub joints: Vec<Joint>,
    pub clips: Vec<Clip>,
}

impl Skeleton {
    pub fn clip_index(&self, name: &str) -> Option<usize> {
        self.clips.iter().position(|c| c.name == name)
    }
}

#[derive(Component, Clone)]
pub struct Animator {
    skeleton: Arc<Skeleton>,
    clip: usize,
    pub time: f32,
    pub speed: f32,
    pub looping: bool,
    pub playing: bool,
    pub fade_dur: f32,
    prev: Option<usize>,
    prev_time: f32,
    fade: f32,
    deltas: Vec<Mat4>,
}

impl Animator {
    pub fn new(skeleton: Arc<Skeleton>) -> Self {
        let count = skeleton.joints.len();
        Animator {
            skeleton,
            clip: 0,
            time: 0.0,
            speed: 1.0,
            looping: true,
            playing: true,
            fade_dur: 0.15,
            prev: None,
            prev_time: 0.0,
            fade: 1.0,
            deltas: vec![Mat4::IDENTITY; count],
        }
    }

    pub fn clips(&self) -> impl Iterator<Item = &str> {
        self.skeleton.clips.iter().map(|c| c.name.as_str())
    }

    pub fn current(&self) -> Option<&str> {
        self.skeleton.clips.get(self.clip).map(|c| c.name.as_str())
    }

    pub fn play(&mut self, name: &str) {
        if let Some(i) = self.skeleton.clip_index(name)
            && i != self.clip
        {
            self.prev = Some(self.clip);
            self.prev_time = self.time;
            self.fade = 0.0;
            self.clip = i;
            self.time = 0.0;
            self.playing = true;
        }
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn resume(&mut self) {
        self.playing = true;
    }

    pub fn delta(&self, bone: Option<usize>) -> Mat4 {
        match bone {
            Some(i) => self.deltas.get(i).copied().unwrap_or(Mat4::IDENTITY),
            None => Mat4::IDENTITY,
        }
    }

    fn advance(&mut self, dt: f32) {
        if self.playing {
            self.time += dt * self.speed;
            if self.prev.is_some() {
                self.prev_time += dt * self.speed;
            }
        }
        if self.fade < 1.0 {
            self.fade += dt / self.fade_dur.max(1e-4);
            if self.fade >= 1.0 {
                self.prev = None;
            }
        }

        let Some(clip) = self.skeleton.clips.get(self.clip) else {
            return;
        };
        let t = loop_time(self.time, clip.duration, self.looping);
        let prev = self.prev.and_then(|i| self.skeleton.clips.get(i));
        let prev_t = prev.map(|c| loop_time(self.prev_time, c.duration, self.looping));
        let fade = self.fade.clamp(0.0, 1.0);

        let joints = &self.skeleton.joints;
        let mut world: Vec<Mat4> = vec![Mat4::IDENTITY; joints.len()];
        for (i, joint) in joints.iter().enumerate() {
            let (mut tr, mut ro, mut sc) = sample_trs(&clip.joints[i], joint.bind_local, t);
            if let (Some(p), Some(pt)) = (prev, prev_t) {
                let (ptr, pro, psc) = sample_trs(&p.joints[i], joint.bind_local, pt);
                tr = ptr.lerp(tr, fade);
                ro = pro.slerp(ro, fade);
                sc = psc.lerp(sc, fade);
            }
            let local = Mat4::from_scale_rotation_translation(sc, ro, tr);
            world[i] = match joint.parent {
                Some(p) => world[p] * local,
                None => joint.base,
            };
        }
        if self.deltas.len() != joints.len() {
            self.deltas.resize(joints.len(), Mat4::IDENTITY);
        }
        for i in 0..joints.len() {
            self.deltas[i] = world[i] * joints[i].inv_bind_world;
        }
    }
}

fn loop_time(time: f32, duration: f32, looping: bool) -> f32 {
    if looping {
        if duration > 0.0 {
            time.rem_euclid(duration)
        } else {
            0.0
        }
    } else {
        time.clamp(0.0, duration)
    }
}

fn sample_trs(anim: &JointAnim, bind: Mat4, t: f32) -> (Vec3, Quat, Vec3) {
    let (bt, br, bs) = decompose(bind);
    (
        sample_vec(&anim.trans, t).unwrap_or(bt),
        sample_quat(&anim.rot, t).unwrap_or(br),
        sample_vec(&anim.scale, t).unwrap_or(bs),
    )
}

fn decompose(m: Mat4) -> (Vec3, Quat, Vec3) {
    let (s, r, t) = m.to_scale_rotation_translation();
    (t, r, s)
}

fn frame(times: &[(f32, impl Copy)], t: f32) -> Option<(usize, usize, f32)> {
    if times.is_empty() {
        return None;
    }
    if t <= times[0].0 {
        return Some((0, 0, 0.0));
    }
    let last = times.len() - 1;
    if t >= times[last].0 {
        return Some((last, last, 0.0));
    }
    let mut i = 0;
    while i + 1 < times.len() && times[i + 1].0 < t {
        i += 1;
    }
    let (t0, _) = times[i];
    let (t1, _) = times[i + 1];
    let span = (t1 - t0).max(1e-6);
    Some((i, i + 1, (t - t0) / span))
}

fn sample_vec(keys: &[(f32, Vec3)], t: f32) -> Option<Vec3> {
    let (a, b, f) = frame(keys, t)?;
    Some(keys[a].1.lerp(keys[b].1, f))
}

fn sample_quat(keys: &[(f32, Quat)], t: f32) -> Option<Quat> {
    let (a, b, f) = frame(keys, t)?;
    Some(keys[a].1.slerp(keys[b].1, f))
}

pub fn animator_update(world: &mut World) -> SysResult {
    let dt = world.resource::<Time>()?.dt;
    let ents: Vec<Entity> = world
        .query::<(&Animator, &Renderer)>()
        .map(|(e, _, _)| e)
        .collect();
    for e in ents {
        if let Some(a) = world.get_mut::<Animator>(e) {
            a.advance(dt);
        }
    }
    Ok(())
}
