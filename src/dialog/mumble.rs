use crate::audio::{Vowel, VowelSound};
use crate::ui::{Delivery, RichChar};

use super::Voice;

const BASE_PITCH: f32 = 170.0;

const TUNE_START: f32 = 0.08;
const TUNE_FALL: f32 = 0.2;
const EXCLAIM_LIFT: f32 = 0.25;
const QUESTION_RISE: f32 = 0.4;
const TRAIL_FADE: f32 = 0.85;
const SHOUT_PITCH: f32 = 1.5;
const SHOUT_BRIGHTNESS: f32 = 0.5;
const SHOUT_GAIN: f32 = 1.8;
const SHOUT_HOLD: f32 = 1.5;
const WHISPER_GAIN: f32 = 0.8;

pub fn vowel_between(
    chars: &[RichChar],
    from: usize,
    to: usize,
    voice: Voice,
) -> Option<VowelSound> {
    let to = to.min(chars.len());
    let (index, vowel) = (from.min(to)..to)
        .rev()
        .find_map(|i| Some((i, sound_at(chars, i)?)))?;
    let spoken = &chars[index];
    let sound = VowelSound {
        freq: BASE_PITCH * voice.pitch * (1.0 + voice.variation * swing(spoken.ch)),
        vowel,
        tract: voice.tract,
        brightness: voice.brightness,
        gain: 1.0,
        whisper: false,
        roughness: voice.roughness,
        hold: run_length(chars, index) as f32,
    };
    let (pitch, level) = tune(chars, index);
    let sound = VowelSound {
        freq: sound.freq * pitch,
        gain: level,
        ..sound
    };
    let delivery = if hisses(chars, index) {
        Delivery::Whisper
    } else {
        spoken.delivery()
    };
    Some(match delivery {
        Delivery::Normal => sound,
        Delivery::Shout => VowelSound {
            freq: sound.freq * SHOUT_PITCH,
            brightness: sound.brightness + SHOUT_BRIGHTNESS,
            gain: sound.gain * SHOUT_GAIN,
            hold: sound.hold * SHOUT_HOLD,
            ..sound
        },
        Delivery::Whisper => VowelSound {
            gain: sound.gain * WHISPER_GAIN,
            whisper: true,
            ..sound
        },
    })
}

fn is_stop(c: char) -> bool {
    matches!(c, ',' | ';' | ':' | '.' | '?' | '!' | '…')
}

fn ends_clause(chars: &[RichChar], index: usize) -> bool {
    is_stop(chars[index].ch)
        && chars
            .get(index + 1)
            .is_none_or(|c| c.ch.is_whitespace() || is_stop(c.ch))
}

fn tune(chars: &[RichChar], index: usize) -> (f32, f32) {
    let start = (0..index)
        .rev()
        .find(|&i| ends_clause(chars, i))
        .map_or(0, |i| i + 1);
    let start = (start..index)
        .find(|&i| !chars[i].ch.is_whitespace())
        .unwrap_or(index);
    let end = (index..chars.len())
        .find(|&i| ends_clause(chars, i))
        .unwrap_or(chars.len());
    let ending: String = chars[end..]
        .iter()
        .map(|c| c.ch)
        .take_while(|&c| is_stop(c))
        .collect();
    let progress = (index - start) as f32 / (end - start).max(1) as f32;
    let mut pitch = 1.0 + TUNE_START - TUNE_FALL * progress;
    let mut level = 1.0;
    if ending.contains('!') {
        pitch += EXCLAIM_LIFT * (1.0 - progress);
    }
    if ending.contains('?') {
        let word = (start..end)
            .rev()
            .find(|&i| chars[i].ch.is_whitespace())
            .map_or(start, |i| i + 1);
        if index >= word {
            let along = (index + 1 - word) as f32 / (end - word).max(1) as f32;
            pitch += QUESTION_RISE * along;
        }
    }
    if ending.contains("...") || ending.contains('…') {
        level = 1.0 - TRAIL_FADE * progress;
    }
    (pitch, level)
}

fn swing(letter: char) -> f32 {
    let spread =
        (letter.to_lowercase().next().unwrap_or(letter) as u32).wrapping_mul(2_654_435_761);
    (spread >> 16 & 0xff) as f32 / 127.5 - 1.0
}

fn same_letter(a: char, b: char) -> bool {
    a.to_lowercase().eq(b.to_lowercase())
}

fn run_length(chars: &[RichChar], index: usize) -> usize {
    chars[index..]
        .iter()
        .enumerate()
        .take_while(|(i, c)| same_letter(c.ch, chars[index].ch) || hisses(chars, index + i))
        .count()
}

fn sound_at(chars: &[RichChar], index: usize) -> Option<Vowel> {
    let letter = chars[index].ch;
    if index > 0 && same_letter(chars[index - 1].ch, letter) {
        return None;
    }
    if let Some(vowel) = vowel_of(letter) {
        return Some(vowel);
    }
    if hisses(chars, index) {
        let continues = index > 0 && hisses(chars, index - 1);
        return (!continues).then_some(Vowel::I);
    }
    (is_hum(letter) && word_has_no_vowel(chars, index)).then_some(Vowel::U)
}

fn word_around(chars: &[RichChar], index: usize) -> impl Iterator<Item = char> {
    let letter = |c: &&RichChar| c.ch.is_alphabetic();
    let before = chars[..index].iter().rev().take_while(letter);
    let after = chars[index..].iter().take_while(letter);
    before.chain(after).map(|c| c.ch)
}

fn word_has_no_vowel(chars: &[RichChar], index: usize) -> bool {
    !word_around(chars, index).any(|c| vowel_of(c).is_some())
}

fn is_hum(letter: char) -> bool {
    matches!(letter.to_ascii_lowercase(), 'm' | 'n')
}

fn hisses(chars: &[RichChar], index: usize) -> bool {
    matches!(
        chars[index].ch.to_ascii_lowercase(),
        's' | 'h' | 'x' | 'z' | 'f'
    ) && word_has_no_vowel(chars, index)
        && !word_around(chars, index).any(is_hum)
}

fn vowel_of(letter: char) -> Option<Vowel> {
    match letter.to_lowercase().next()? {
        'a' | 'á' | 'à' => Some(Vowel::A),
        'â' | 'ã' => Some(Vowel::Uh),
        'e' | 'é' | 'ê' => Some(Vowel::E),
        'i' | 'í' | 'y' => Some(Vowel::I),
        'o' | 'ó' | 'ô' | 'õ' => Some(Vowel::O),
        'u' | 'ú' => Some(Vowel::U),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::parse_rich;

    #[test]
    fn only_vowels_make_a_sound() {
        let chars = parse_rich("b, t... r");
        assert!(vowel_between(&chars, 0, 9, Voice::default()).is_none());
    }

    #[test]
    fn a_word_without_vowels_hums_its_m_and_n() {
        let chars = parse_rich("Hmm...");
        assert!(vowel_between(&chars, 0, 1, Voice::default()).is_none());
        let hum = vowel_between(&chars, 1, 2, Voice::default()).unwrap();
        assert_eq!(hum.vowel, Vowel::U);
        let word = parse_rich("mar");
        assert!(vowel_between(&word, 0, 1, Voice::default()).is_none());
    }

    #[test]
    fn a_repeated_vowel_is_one_long_sound() {
        let chars = parse_rich("BUU!");
        let first = vowel_between(&chars, 1, 2, Voice::default()).unwrap();
        assert_eq!(first.hold, 2.0);
        assert!(vowel_between(&chars, 2, 3, Voice::default()).is_none());
    }

    #[test]
    fn a_word_without_vowels_hisses_as_one_long_breath() {
        let chars = parse_rich("Shh!");
        let hiss = vowel_between(&chars, 0, 1, Voice::default()).unwrap();
        assert!(hiss.whisper);
        assert_eq!(hiss.hold, 3.0);
        assert!(vowel_between(&chars, 1, 3, Voice::default()).is_none());
        let word = parse_rich("sol");
        assert!(vowel_between(&word, 0, 1, Voice::default()).is_none());
    }

    #[test]
    fn each_vowel_speaks_when_it_is_typed() {
        let chars = parse_rich("portão");
        assert!(vowel_between(&chars, 0, 1, Voice::default()).is_none());
        let first = vowel_between(&chars, 1, 2, Voice::default()).unwrap();
        assert_eq!(first.vowel, Vowel::O);
        assert!(vowel_between(&chars, 2, 4, Voice::default()).is_none());
        let second = vowel_between(&chars, 4, 5, Voice::default()).unwrap();
        assert_eq!(second.vowel, Vowel::Uh);
    }

    #[test]
    fn the_same_vowel_keeps_its_note_and_pitch_scales_it() {
        let chars = parse_rich("a");
        let low = Voice {
            pitch: 1.0,
            ..Voice::default()
        };
        let high = Voice {
            pitch: 2.0,
            ..Voice::default()
        };
        let a = vowel_between(&chars, 0, 1, low).unwrap();
        let b = vowel_between(&chars, 0, 1, low).unwrap();
        let c = vowel_between(&chars, 0, 1, high).unwrap();
        assert_eq!(a, b);
        assert!((c.freq / a.freq - 2.0).abs() < 0.001);
    }

    #[test]
    fn shout_and_whisper_change_the_voice() {
        let plain = vowel_between(&parse_rich("a"), 0, 1, Voice::default()).unwrap();
        let chars = parse_rich("[shout]a[/shout]");
        let shout = vowel_between(&chars, 0, 1, Voice::default()).unwrap();
        assert!(shout.freq > plain.freq && shout.gain > plain.gain);
        assert!(shout.brightness > plain.brightness);
        let chars = parse_rich("[whisper]a[/whisper]");
        let whisper = vowel_between(&chars, 0, 1, Voice::default()).unwrap();
        assert!(whisper.whisper && whisper.gain < plain.gain);
    }

    fn tuned(text: &str, index: usize) -> (f32, f32) {
        tune(&parse_rich(text), index)
    }

    #[test]
    fn pitch_falls_along_the_phrase_and_resets_at_a_comma() {
        let text = "aaaaaaaa, aaaaaaaa.";
        assert!(tuned(text, 7).0 < tuned(text, 0).0);
        assert!((tuned(text, 10).0 - tuned(text, 0).0).abs() < 0.001);
        assert!(tuned(text, 17).0 < tuned(text, 10).0);
    }

    #[test]
    fn an_exclamation_starts_higher() {
        assert!(tuned("aaaaaa!", 0).0 > tuned("aaaaaa.", 0).0);
    }

    #[test]
    fn a_question_lifts_only_the_last_word() {
        let (ask, say) = ("aaa aaa?", "aaa aaa.");
        assert_eq!(tuned(ask, 1).0, tuned(say, 1).0);
        assert!(tuned(ask, 6).0 > tuned(say, 6).0);
        assert!(tuned(ask, 6).0 > tuned(ask, 4).0);
    }

    #[test]
    fn an_ellipsis_fades_out() {
        let text = "aaaaaaaa...";
        assert_eq!(tuned(text, 0).1, 1.0);
        assert!(tuned(text, 7).1 < 0.4);
        assert_eq!(tuned("aaaaaaaa.", 7).1, 1.0);
    }
}
