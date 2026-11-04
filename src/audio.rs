pub use ecs_macros::Sound;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use rodio::{OutputStream, OutputStreamHandle, Source};

use crate::ecs::prelude::*;

static QUEUE: Mutex<Vec<String>> = Mutex::new(Vec::new());

#[derive(Resource, Default)]
pub struct Sounds(HashMap<String, Vec<(f32, f32)>>);

impl Sounds {
    pub fn declare(&mut self, sound: impl Sound, tones: &[(f32, f32)]) -> &mut Self {
        self.0.insert(sound.name().to_string(), tones.to_vec());
        self
    }

    pub fn tones(&self, name: &str) -> &[(f32, f32)] {
        self.0.get(name).map_or(&[], Vec::as_slice)
    }
}

pub fn queue_sound(name: &str) {
    if let Ok(mut queue) = QUEUE.lock() {
        queue.push(name.to_string());
    }
}

pub fn take_queued_sounds() -> Vec<String> {
    QUEUE
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default()
}

#[derive(Resource, Clone, Copy)]
pub struct Volume {
    pub sfx: f32,
    pub bgm: f32,
}

impl Default for Volume {
    fn default() -> Self {
        Volume { sfx: 1.0, bgm: 1.0 }
    }
}

#[derive(Resource)]
pub struct Audio {
    _stream: OutputStream,
    handle: OutputStreamHandle,
}

impl Audio {
    pub fn new() -> Option<Audio> {
        let (stream, handle) = OutputStream::try_default().ok()?;
        Some(Audio {
            _stream: stream,
            handle,
        })
    }

    pub fn blip(&self, freq: f32, gain: f32) {
        self.tone(freq, gain, Duration::ZERO);
    }

    pub fn tone(&self, freq: f32, gain: f32, delay: Duration) {
        let src = SoftTone::new(freq, gain * MASTER).delay(delay);
        let _ = self.handle.play_raw(src);
    }
}

const MASTER: f32 = 0.35;
const RATE: u32 = 44_100;
const ATTACK: f32 = 0.012;
const LENGTH: f32 = 0.32;
const DECAY: f32 = 11.0;

struct SoftTone {
    freq: f32,
    gain: f32,
    n: u32,
    total: u32,
}

impl SoftTone {
    fn new(freq: f32, gain: f32) -> Self {
        SoftTone {
            freq,
            gain,
            n: 0,
            total: (LENGTH * RATE as f32) as u32,
        }
    }
}

impl Iterator for SoftTone {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.n >= self.total {
            return None;
        }
        let t = self.n as f32 / RATE as f32;
        self.n += 1;
        let attack = (t / ATTACK).min(1.0);
        let tail = 1.0 - (t / LENGTH).powi(4);
        let env = attack * (-DECAY * t).exp() * tail;
        let phase = std::f32::consts::TAU * self.freq * t;
        let wave = phase.sin() + 0.15 * (2.0 * phase).sin();
        Some(wave * env * self.gain)
    }
}

impl Source for SoftTone {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(LENGTH))
    }
}

pub fn drain_sounds(world: &mut World) -> SysResult {
    let queued = take_queued_sounds();
    let volume = world.resource::<Volume>().copied().unwrap_or_default().sfx;
    let (Ok(audio), Ok(sounds)) = (world.resource::<Audio>(), world.resource::<Sounds>()) else {
        return Ok(());
    };
    if volume <= 0.0 {
        return Ok(());
    }
    for name in queued {
        for (i, &(freq, gain)) in sounds.tones(&name).iter().enumerate() {
            audio.tone(freq, gain * volume, Duration::from_millis(70 * i as u64));
        }
    }
    Ok(())
}

pub trait Sound: Copy + 'static {
    fn name(self) -> &'static str;

    fn play(self) {
        queue_sound(self.name());
    }
}

#[derive(Sound, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EngineSound {
    Pick,
}

pub mod prelude {
    pub use super::{Audio, EngineSound, Sound, Sounds, Volume, drain_sounds};
}

#[cfg(test)]
mod sound_tests {
    use super::*;

    #[derive(Sound, Clone, Copy)]
    enum Test {
        AudioTestPing,
    }

    #[test]
    fn declared_sounds_are_found_by_name() {
        let mut sounds = Sounds::default();
        sounds.declare(Test::AudioTestPing, &[(440.0, 0.5)]);
        assert_eq!(sounds.tones("audio_test_ping"), &[(440.0, 0.5)]);
        assert!(sounds.tones("nope").is_empty());
    }

    #[test]
    fn playing_queues_the_sound() {
        Test::AudioTestPing.play();
        assert!(take_queued_sounds().iter().any(|s| s == "audio_test_ping"));
    }
}
