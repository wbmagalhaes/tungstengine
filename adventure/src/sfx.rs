use tungstengine::audio::{Audio, Sounds, take_queued_sounds};
use tungstengine::ecs::prelude::*;

use super::settings::Settings;

#[derive(Clone, Copy)]
pub enum Channel {
    Ui,
    Sfx,
    Bgm,
}

#[derive(Clone, Copy)]
pub struct Blip {
    pub freq: f32,
    pub gain: f32,
    pub channel: Channel,
}

impl Blip {
    pub fn ui(freq: f32) -> Self {
        Blip {
            freq,
            gain: 1.0,
            channel: Channel::Ui,
        }
    }

    pub fn sfx(freq: f32) -> Self {
        Blip {
            freq,
            gain: 1.0,
            channel: Channel::Sfx,
        }
    }

    pub fn bgm(freq: f32) -> Self {
        Blip {
            freq,
            gain: 1.0,
            channel: Channel::Bgm,
        }
    }

    pub fn gain(mut self, gain: f32) -> Self {
        self.gain = gain;
        self
    }
}

pub fn drain_sounds(world: &mut World) -> SysResult {
    for name in take_queued_sounds() {
        world.play_sfx(&name);
    }
    Ok(())
}

pub trait PlayBlip {
    fn play_blip(&self, blip: Blip);

    fn play_sfx(&self, name: &str);
}

impl PlayBlip for World {
    fn play_sfx(&self, name: &str) {
        let tones = self
            .resource::<Sounds>()
            .map(|s| s.tones(name).to_vec())
            .unwrap_or_default();
        for (freq, gain) in tones {
            self.play_blip(Blip::sfx(freq).gain(gain));
        }
    }

    fn play_blip(&self, blip: Blip) {
        let vol = self
            .resource::<Settings>()
            .map(|s| match blip.channel {
                Channel::Ui => s.ui(),
                Channel::Sfx => s.sfx(),
                Channel::Bgm => s.bgm(),
            })
            .unwrap_or(1.0);
        if vol <= 0.0 {
            return;
        }
        if let Ok(audio) = self.resource::<Audio>() {
            audio.blip(blip.freq, blip.gain * vol);
        }
    }
}
