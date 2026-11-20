//! Sound effects and the ambient pad, synthesized at start-up into WAV
//! bytes: the game ships no audio files.

use std::cell::RefCell;
use std::f32::consts::TAU;

use nsh_core::prelude::*;

/// Samples per second.
const RATE: u32 = 22_050;

/// A sound effect (or a chord of the ambient pad).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sfx {
    Step,
    Edit,
    Gem,
    Bell,
    Locked,
    Complete,
    Pad1,
    Pad2,
    /// A bug caught: a squelch and a pop.
    Squash,
    /// A boss struck: a low thump.
    Hit,
    /// A new verb + noun pair: a bright two-note chirp.
    Word,
}

const ALL: [Sfx; 11] = [
    Sfx::Step,
    Sfx::Edit,
    Sfx::Gem,
    Sfx::Bell,
    Sfx::Locked,
    Sfx::Complete,
    Sfx::Pad1,
    Sfx::Pad2,
    Sfx::Squash,
    Sfx::Hit,
    Sfx::Word,
];

thread_local! {
    static SOUNDS: RefCell<Vec<(Sfx, Sound)>> = const { RefCell::new(Vec::new()) };
    static MUTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32_000.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// A note: a sine with a few soft harmonics and an exponential decay.
fn tone(out: &mut [f32], start: f32, freq: f32, len: f32, gain: f32, decay: f32) {
    let s0 = (start * RATE as f32) as usize;
    let n = (len * RATE as f32) as usize;
    for i in 0..n {
        let Some(slot) = out.get_mut(s0 + i) else { break };
        let t = i as f32 / RATE as f32;
        let attack = (t / 0.004).min(1.0);
        let env = attack * (-t * decay).exp();
        let w =
            (TAU * freq * t).sin() + 0.3 * (TAU * 2.0 * freq * t).sin() + 0.1 * (TAU * 3.0 * freq * t).sin();
        *slot += w * env * gain;
    }
}

fn buffer(seconds: f32) -> Vec<f32> {
    vec![0.0; (seconds * RATE as f32) as usize]
}

/// A note, as `tone` plays it: start, frequency, length, gain and decay.
type Note = (f32, f32, f32, f32, f32);

/// `seconds` of sound holding `notes`.
fn notes(seconds: f32, notes: &[Note]) -> Vec<f32> {
    let mut b = buffer(seconds);
    for &(start, freq, len, gain, decay) in notes {
        tone(&mut b, start, freq, len, gain, decay);
    }
    b
}

/// `seconds` of sound with `freqs` rising one after another, `gap` apart;
/// each note is (length, gain, decay).
fn arpeggio(seconds: f32, freqs: &[f32], gap: f32, (len, gain, decay): (f32, f32, f32)) -> Vec<f32> {
    let mut b = buffer(seconds);
    for (k, f) in freqs.iter().enumerate() {
        tone(&mut b, k as f32 * gap, *f, len, gain, decay);
    }
    b
}

/// A pitch falling from `high + low` towards `low` at rate `fall`, fading
/// at rate `fade` from `gain`.
fn sweep(b: &mut [f32], (high, fall, low): (f32, f32, f32), fade: f32, gain: f32) {
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / RATE as f32;
        let f = high * (-t * fall).exp() + low;
        *s = (TAU * f * t).sin() * (-t * fade).exp() * gain;
    }
}

/// A soft, filtered click of noise: chiselling stone.
fn chisel() -> Vec<f32> {
    let mut b = buffer(0.08);
    let mut seed = 12345u32;
    let mut last = 0.0;
    for (i, s) in b.iter_mut().enumerate() {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let noise = ((seed >> 16) as f32 / 32_768.0) - 1.0;
        last = last * 0.75 + noise * 0.25;
        let t = i as f32 / RATE as f32;
        *s = last * (-t * 50.0).exp() * 0.9;
    }
    tone(&mut b, 0.0, 330.0, 0.08, 0.2, 40.0);
    b
}

/// A slow chord that swells in and out: the night air.
fn pad(chord: &[f32]) -> Vec<f32> {
    let len = 4.8;
    let mut b = buffer(len);
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / RATE as f32;
        let env = (t / 1.6).min(1.0) * ((len - t) / 1.8).clamp(0.0, 1.0);
        let mut v = 0.0;
        for (k, f) in chord.iter().enumerate() {
            let detune = 1.0 + 0.002 * (k as f32 - 1.5);
            v += (TAU * f * detune * t).sin() + 0.15 * (TAU * 2.0 * f * t + k as f32).sin();
        }
        *s = v * env * 0.07;
    }
    b
}

fn synth(sfx: Sfx) -> Vec<u8> {
    let samples = match sfx {
        Sfx::Step => notes(0.09, &[(0.0, 740.0, 0.09, 0.25, 45.0)]),
        Sfx::Edit => chisel(),
        Sfx::Gem => arpeggio(0.7, &[1046.5, 1318.5, 1568.0, 2093.0], 0.06, (0.5, 0.18, 7.0)),
        Sfx::Bell => notes(0.15, &[(0.0, 196.0, 0.15, 0.3, 25.0)]),
        Sfx::Locked => notes(
            0.22,
            &[(0.0, 233.0, 0.1, 0.25, 30.0), (0.09, 175.0, 0.13, 0.25, 25.0)],
        ),
        Sfx::Complete => {
            let mut b = arpeggio(
                1.6,
                &[523.25, 659.25, 783.99, 1046.5, 1318.5],
                0.11,
                (1.2, 0.17, 3.5),
            );
            tone(&mut b, 0.55, 261.63, 1.0, 0.15, 2.5);
            b
        }
        Sfx::Squash => {
            // A falling chirp, then a pop.
            let mut b = buffer(0.22);
            sweep(&mut b, (900.0, 9.0, 120.0), 18.0, 0.35);
            tone(&mut b, 0.07, 1400.0, 0.08, 0.15, 60.0);
            b
        }
        Sfx::Hit => {
            let mut b = buffer(0.35);
            sweep(&mut b, (110.0, 4.0, 45.0), 9.0, 0.6);
            tone(&mut b, 0.0, 220.0, 0.12, 0.12, 30.0);
            b
        }
        Sfx::Word => notes(
            0.4,
            &[(0.0, 1174.7, 0.3, 0.15, 12.0), (0.08, 1568.0, 0.3, 0.15, 10.0)],
        ),
        Sfx::Pad1 => pad(&[220.0, 261.63, 329.63, 440.0]),
        Sfx::Pad2 => pad(&[196.0, 246.94, 293.66, 392.0]),
    };
    wav(&samples)
}

/// Decodes every sound; call once from the root component.
pub fn load_all() {
    for sfx in ALL {
        spawn(async move {
            if let Ok(sound) = load_sound(synth(sfx)).await {
                SOUNDS.with(|s| s.borrow_mut().push((sfx, sound)));
            }
        });
    }
}

/// Plays `sfx` at `volume` (0 to 1), unless muted.
pub fn play(sfx: Sfx, volume: f32) {
    if MUTED.with(std::cell::Cell::get) {
        return;
    }
    SOUNDS.with(|s| {
        if let Some((_, sound)) = s.borrow().iter().find(|(k, _)| *k == sfx) {
            sound.play(volume.clamp(0.0, 1.0), 1.0);
        }
    });
}

/// A step's pitch changes a little with where it lands, so walking sounds alive.
pub fn step(variant: usize) {
    if MUTED.with(std::cell::Cell::get) {
        return;
    }
    let rates = [1.0, 1.12, 0.94, 1.06, 0.89];
    SOUNDS.with(|s| {
        if let Some((_, sound)) = s.borrow().iter().find(|(k, _)| *k == Sfx::Step) {
            sound.play(0.35, rates[variant % rates.len()]);
        }
    });
}

/// Mutes or unmutes every sound; muting stops the pad.
pub fn set_muted(muted: bool) {
    MUTED.with(|m| m.set(muted));
    if muted {
        SOUNDS.with(|s| {
            for (k, sound) in s.borrow().iter() {
                if matches!(k, Sfx::Pad1 | Sfx::Pad2) {
                    sound.stop();
                }
            }
        });
    }
}

/// Whether sound is off.
pub fn muted() -> bool {
    MUTED.with(std::cell::Cell::get)
}

/// Plays the pad every four seconds, alternating chords.
pub fn start_ambience() {
    let turn = std::cell::Cell::new(0usize);
    set_interval(std::time::Duration::from_millis(4200), move || {
        let n = turn.get();
        turn.set(n + 1);
        play(if n.is_multiple_of(2) { Sfx::Pad1 } else { Sfx::Pad2 }, 0.55);
    });
}
