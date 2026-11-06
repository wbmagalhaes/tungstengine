pub use ecs_macros::Sound;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use rodio::{OutputStream, OutputStreamHandle, Source};

use crate::ecs::prelude::*;

static QUEUE: Mutex<Vec<String>> = Mutex::new(Vec::new());

const MIN_REPEAT_GAP: f32 = 0.12;

#[derive(Resource, Default)]
pub struct Sounds {
    tones: HashMap<String, Vec<(f32, f32)>>,
    last_played: HashMap<String, f32>,
}

impl Sounds {
    pub fn declare(&mut self, sound: impl Sound, tones: &[(f32, f32)]) -> &mut Self {
        self.tones.insert(sound.name().to_string(), tones.to_vec());
        self
    }

    pub fn tones(&self, name: &str) -> &[(f32, f32)] {
        self.tones.get(name).map_or(&[], Vec::as_slice)
    }

    fn ready(&mut self, name: &str, now: f32) -> bool {
        let last = self.last_played.get(name).copied();
        if last.is_some_and(|t| now - t < MIN_REPEAT_GAP) {
            return false;
        }
        self.last_played.insert(name.to_string(), now);
        true
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

    pub fn vowel(&self, sound: VowelSound, gain: f32) {
        let src = VowelTone::new(sound, gain * VOWEL_GAIN * MASTER);
        let _ = self.handle.play_raw(src);
    }

    pub fn tone(&self, freq: f32, gain: f32, delay: Duration) {
        let src = SoftTone::new(freq, gain * MASTER).delay(delay);
        let _ = self.handle.play_raw(src);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vowel {
    A,
    E,
    I,
    O,
    U,
    Uh,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VowelSound {
    pub freq: f32,
    pub vowel: Vowel,
    pub blend: Vowel,
    pub tract: f32,
    pub brightness: f32,
    pub gain: f32,
    pub whisper: bool,
    pub roughness: f32,
    pub hold: f32,
}

const VOWEL_AH: &[(f32, f32)] = &[
    (170.0, 1.0),
    (430.0, 0.92),
    (787.0, 1.0),
    (1092.0, 0.93),
    (1297.0, 0.96),
    (1524.0, 0.7),
    (1690.0, 0.57),
    (2027.0, 0.41),
    (2367.0, 0.31),
    (2600.0, 0.3),
    (2935.0, 0.3),
    (3220.0, 0.12),
    (3410.0, 0.0),
];
const VOWEL_EH: &[(f32, f32)] = &[
    (170.0, 1.0),
    (340.0, 0.94),
    (601.0, 0.99),
    (850.0, 0.78),
    (1190.0, 0.63),
    (1584.0, 0.57),
    (2097.0, 0.7),
    (2386.0, 0.55),
    (2716.0, 0.5),
    (2833.0, 0.485),
    (3063.0, 0.31),
    (3404.0, 0.08),
    (3570.0, 0.0),
];
const VOWEL_EE: &[(f32, f32)] = &[
    (170.0, 0.98),
    (337.0, 1.0),
    (510.0, 0.75),
    (850.0, 0.49),
    (1190.0, 0.35),
    (1870.0, 0.25),
    (2380.0, 0.33),
    (2607.0, 0.44),
    (3060.0, 0.25),
    (3400.0, 0.26),
    (3573.0, 0.13),
    (3737.0, 0.0),
];
const VOWEL_OH: &[(f32, f32)] = &[
    (170.0, 0.96),
    (283.0, 0.93),
    (510.0, 1.0),
    (680.0, 0.88),
    (907.0, 0.82),
    (1023.0, 0.69),
    (1193.0, 0.5),
    (1533.0, 0.26),
    (1873.0, 0.11),
    (2213.0, 0.01),
    (2300.0, 0.0),
];
const VOWEL_OO: &[(f32, f32)] = &[
    (170.0, 0.96),
    (368.0, 1.0),
    (510.0, 0.84),
    (680.0, 0.75),
    (793.0, 0.72),
    (1020.0, 0.44),
    (1360.0, 0.17),
    (1703.0, 0.0),
];

const VOWEL_UH: &[(f32, f32)] = &[
    (170.0, 0.98),
    (384.0, 0.91),
    (677.0, 1.0),
    (847.0, 0.88),
    (1014.0, 0.8),
    (1184.0, 0.79),
    (1354.0, 0.8),
    (1524.0, 0.66),
    (1694.0, 0.53),
    (2030.0, 0.37),
    (2367.0, 0.31),
    (2707.0, 0.32),
    (2877.0, 0.2),
    (3047.0, 0.08),
    (3170.0, 0.0),
];

impl Vowel {
    fn presence(self) -> f32 {
        match self {
            Vowel::U => 1.8,
            Vowel::O => 1.5,
            Vowel::I => 1.2,
            _ => 1.0,
        }
    }

    fn envelope(self) -> &'static [(f32, f32)] {
        match self {
            Vowel::A => VOWEL_AH,
            Vowel::E => VOWEL_EH,
            Vowel::I => VOWEL_EE,
            Vowel::O => VOWEL_OH,
            Vowel::U => VOWEL_OO,
            Vowel::Uh => VOWEL_UH,
        }
    }

    fn loudness_at(self, freq: f32) -> f32 {
        let curve = self.envelope();
        let next = curve.iter().position(|&(f, _)| f >= freq);
        match next {
            None => 0.0,
            Some(0) => curve[0].1,
            Some(i) => {
                let ((f0, a0), (f1, a1)) = (curve[i - 1], curve[i]);
                a0 + (a1 - a0) * (freq - f0) / (f1 - f0)
            }
        }
    }
}

pub const VOWEL_LENGTH: f32 = 0.085;
const VOWEL_SWELL: f32 = 0.2;
const VOWEL_RELEASE: f32 = 0.25;
const WHISPER_LENGTH: f32 = 0.12;
const WHISPER_DAMPING: f32 = 0.25;
const WHISPER_LEVEL: f32 = 0.2;
const GOLDEN_RATIO: f32 = 0.618_034;
const JITTER: f32 = 0.015;
const SHIMMER: f32 = 0.14;
const BREATH: f32 = 0.04;
const WOBBLE_FOLLOW: f32 = 0.004;

static VOWELS_SPOKEN: AtomicU32 = AtomicU32::new(1);
const VOWEL_GLIDE: f32 = 0.12;
const VOWEL_GAIN: f32 = 0.5;
const HARMONICS: usize = 32;

struct VowelTone {
    freqs: [f32; HARMONICS],
    weights: [f32; HARMONICS],
    blended: [f32; HARMONICS],
    phases: [f32; HARMONICS],
    gain: f32,
    n: u32,
    total: u32,
    wobble: Wobble,
    breath: Vec<Resonator>,
}

struct Resonator {
    tuning: f32,
    gain: f32,
    low: f32,
    band: f32,
}

impl Resonator {
    fn new(freq: f32, gain: f32) -> Self {
        let angle = std::f32::consts::PI * freq / RATE as f32;
        Resonator {
            tuning: 2.0 * angle.sin(),
            gain,
            low: 0.0,
            band: 0.0,
        }
    }

    fn filter(&mut self, input: f32) -> f32 {
        self.low += self.tuning * self.band;
        let high = input - self.low - WHISPER_DAMPING * self.band;
        self.band += self.tuning * high;
        self.band * self.gain
    }
}

fn resonators(sound: VowelSound) -> Vec<Resonator> {
    sound
        .vowel
        .envelope()
        .windows(3)
        .filter(|w| w[1].1 >= w[0].1 && w[1].1 >= w[2].1)
        .map(|w| {
            let freq = w[1].0 * sound.tract;
            let tilt = (freq / 1000.0).powf(sound.brightness);
            Resonator::new(freq, w[1].1 * tilt)
        })
        .collect()
}

struct Wobble {
    seed: u32,
    amount: f32,
    period: u32,
    left: u32,
    pitch: f32,
    pitch_goal: f32,
    level: f32,
    level_goal: f32,
}

impl Wobble {
    fn new(freq: f32, amount: f32) -> Self {
        let spoken = VOWELS_SPOKEN.fetch_add(1, Ordering::Relaxed);
        let mut wobble = Wobble {
            seed: spoken.wrapping_mul(0x9e37_79b9) | 1,
            amount,
            period: (RATE as f32 / freq.max(1.0)) as u32,
            left: 0,
            pitch: 0.0,
            pitch_goal: 0.0,
            level: 0.0,
            level_goal: 0.0,
        };
        wobble.pitch = wobble.signed() * JITTER * amount;
        wobble
    }

    fn unit(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f32 / (1u32 << 24) as f32
    }

    fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    fn step(&mut self) {
        if self.left == 0 {
            self.left = self.period;
            self.pitch_goal = self.signed() * JITTER * self.amount;
            self.level_goal = self.unit() * SHIMMER * self.amount;
        }
        self.left -= 1;
        self.pitch += (self.pitch_goal - self.pitch) * WOBBLE_FOLLOW;
        self.level += (self.level_goal - self.level) * WOBBLE_FOLLOW;
    }

    fn breath(&mut self) -> f32 {
        self.signed() * BREATH * self.amount
    }
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl VowelTone {
    fn new(sound: VowelSound, gain: f32) -> Self {
        let freqs: [f32; HARMONICS] = std::array::from_fn(|i| sound.freq * (i + 1) as f32);
        let spectrum = |vowel: Vowel| {
            let mut weights = freqs
                .map(|f| vowel.loudness_at(f / sound.tract) * (f / 1000.0).powf(sound.brightness));
            let total = weights.iter().sum::<f32>().max(0.001);
            weights.iter_mut().for_each(|w| *w /= total);
            weights
        };
        let weights = spectrum(sound.vowel);
        let blended = spectrum(sound.blend);
        let length = if sound.whisper {
            WHISPER_LENGTH
        } else {
            VOWEL_LENGTH
        };
        VowelTone {
            freqs,
            weights,
            blended,
            phases: std::array::from_fn(|i| (i as f32 * GOLDEN_RATIO).fract()),
            gain: gain * sound.gain * sound.vowel.presence(),
            n: 0,
            total: (length * sound.hold * RATE as f32) as u32,
            wobble: Wobble::new(sound.freq, sound.roughness.clamp(0.0, 1.0)),
            breath: if sound.whisper {
                resonators(sound)
            } else {
                Vec::new()
            },
        }
    }
}

impl Iterator for VowelTone {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.n >= self.total {
            return None;
        }
        let progress = self.n as f32 / self.total as f32;
        self.n += 1;
        let env = smooth(progress / VOWEL_SWELL) * smooth((1.0 - progress) / VOWEL_RELEASE);
        if !self.breath.is_empty() {
            let noise = self.wobble.signed();
            let air: f32 = self.breath.iter_mut().map(|r| r.filter(noise)).sum();
            return Some((air * WHISPER_LEVEL).clamp(-1.0, 1.0) * env * self.gain);
        }
        self.wobble.step();
        let glide = (1.0 - VOWEL_GLIDE * progress) * (1.0 + self.wobble.pitch);
        let morph = smooth(progress);
        let mut wave = 0.0;
        for i in 0..HARMONICS {
            let phase = (self.phases[i] + self.freqs[i] * glide / RATE as f32).fract();
            self.phases[i] = phase;
            let weight = self.weights[i] + (self.blended[i] - self.weights[i]) * morph;
            wave += weight * (std::f32::consts::TAU * phase).sin();
        }
        let breath = self.wobble.breath();
        let body = wave * (1.0 - self.wobble.level) * (1.0 - BREATH);
        Some((body + breath) * env * self.gain)
    }
}

impl Source for VowelTone {
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
        Some(Duration::from_secs_f32(VOWEL_LENGTH))
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
    let now = world.elapsed();
    let volume = world.resource::<Volume>().copied().unwrap_or_default().sfx;
    let Ok(sounds) = world.resource_mut::<Sounds>() else {
        return Ok(());
    };
    let mut due: Vec<Vec<(f32, f32)>> = Vec::new();
    for name in &queued {
        if sounds.ready(name, now) {
            due.push(sounds.tones(name).to_vec());
        }
    }
    let Ok(audio) = world.resource::<Audio>() else {
        return Ok(());
    };
    if volume <= 0.0 {
        return Ok(());
    }
    for tones in due {
        for (i, (freq, gain)) in tones.into_iter().enumerate() {
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
    fn rapid_repeats_of_a_sound_are_merged() {
        let mut sounds = Sounds::default();
        assert!(sounds.ready("hit", 1.0));
        assert!(!sounds.ready("hit", 1.05));
        assert!(sounds.ready("other", 1.05));
        assert!(sounds.ready("hit", 1.2));
    }

    #[test]
    fn playing_queues_the_sound() {
        Test::AudioTestPing.play();
        assert!(take_queued_sounds().iter().any(|s| s == "audio_test_ping"));
    }
}

#[cfg(test)]
mod vowel_tests {
    use super::*;

    fn sound(vowel: Vowel) -> VowelSound {
        VowelSound {
            freq: 170.0,
            vowel,
            blend: vowel,
            tract: 1.0,
            brightness: 0.0,
            gain: 1.0,
            whisper: false,
            roughness: 1.0,
            hold: 1.0,
        }
    }

    fn loudest(samples: &[f32]) -> f32 {
        samples.iter().fold(0.0, |m, s| m.max(s.abs()))
    }

    #[test]
    fn a_vowel_swells_holds_and_lets_go() {
        let samples: Vec<f32> = VowelTone::new(sound(Vowel::A), 1.0).collect();
        assert_eq!(samples.len(), (VOWEL_LENGTH * RATE as f32) as usize);
        let tenth = samples.len() / 10;
        let peak = loudest(&samples);
        assert!(peak <= 1.0);
        assert!(loudest(&samples[..tenth / 4]) < peak * 0.2);
        assert!(loudest(&samples[4 * tenth..6 * tenth]) > peak * 0.7);
        assert!(samples.last().unwrap().abs() < 0.001);
    }

    #[test]
    fn every_vowel_has_its_own_spectrum() {
        let all = [Vowel::A, Vowel::E, Vowel::I, Vowel::O, Vowel::U, Vowel::Uh];
        for (n, first) in all.iter().enumerate() {
            for second in &all[n + 1..] {
                let a = VowelTone::new(sound(*first), 1.0).weights;
                let b = VowelTone::new(sound(*second), 1.0).weights;
                assert!(a.iter().zip(b).any(|(x, y)| (x - y).abs() > 0.005));
            }
        }
    }

    #[test]
    fn a_smaller_tract_moves_the_curve_up() {
        let small = VowelSound {
            tract: 1.3,
            ..sound(Vowel::U)
        };
        let high = |t: VowelTone| t.weights[10];
        assert_eq!(high(VowelTone::new(sound(Vowel::U), 1.0)), 0.0);
        assert!(high(VowelTone::new(small, 1.0)) > 0.0);
    }

    #[test]
    fn brightness_favors_high_partials() {
        let bright = VowelSound {
            brightness: 0.6,
            ..sound(Vowel::A)
        };
        let plain = VowelTone::new(sound(Vowel::A), 1.0).weights;
        let lifted = VowelTone::new(bright, 1.0).weights;
        assert!(lifted[10] > plain[10]);
        assert!(lifted[0] < plain[0]);
    }

    #[test]
    fn the_same_vowel_never_comes_out_identical() {
        let a: Vec<f32> = VowelTone::new(sound(Vowel::A), 1.0).collect();
        let b: Vec<f32> = VowelTone::new(sound(Vowel::A), 1.0).collect();
        assert_ne!(a, b);
    }

    #[test]
    fn no_roughness_is_a_clean_tone() {
        let clean = VowelSound {
            roughness: 0.0,
            ..sound(Vowel::A)
        };
        let a: Vec<f32> = VowelTone::new(clean, 1.0).collect();
        let b: Vec<f32> = VowelTone::new(clean, 1.0).collect();
        assert_eq!(a, b);
    }

    #[test]
    fn a_blend_ends_on_the_second_vowel() {
        let glide = VowelSound {
            blend: Vowel::I,
            ..sound(Vowel::O)
        };
        let tone = VowelTone::new(glide, 1.0);
        assert_eq!(tone.weights, VowelTone::new(sound(Vowel::O), 1.0).weights);
        assert_eq!(tone.blended, VowelTone::new(sound(Vowel::I), 1.0).weights);
    }

    #[test]
    fn hold_stretches_the_vowel() {
        let long = VowelSound {
            hold: 2.0,
            ..sound(Vowel::U)
        };
        let short = VowelTone::new(sound(Vowel::U), 1.0).count();
        assert!(VowelTone::new(long, 1.0).count() > short * 3 / 2);
    }

    #[test]
    fn a_whisper_is_air_at_a_lower_level_than_the_voice() {
        let whisper = VowelSound {
            whisper: true,
            ..sound(Vowel::A)
        };
        let rms = |t: VowelTone| {
            let samples: Vec<f32> = t.collect();
            (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
        };
        let air = rms(VowelTone::new(whisper, 1.0));
        let voice = rms(VowelTone::new(sound(Vowel::A), 1.0));
        assert!(air > voice * 0.2 && air < voice);
    }

    #[test]
    fn the_envelope_is_read_in_hertz() {
        assert!(Vowel::I.loudness_at(2607.0) > Vowel::U.loudness_at(2607.0));
        assert_eq!(Vowel::U.loudness_at(2000.0), 0.0);
        assert_eq!(Vowel::A.loudness_at(100.0), 1.0);
    }
}
