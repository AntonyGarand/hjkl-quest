//! Sound effects and the ambient pad, synthesized at start-up into WAV
//! bytes: the game ships no audio files.

use std::cell::RefCell;
use std::f32::consts::TAU;

use nsh_core::prelude::*;

const RATE: u32 = 22_050;

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
}

const ALL: [Sfx; 8] = [Sfx::Step, Sfx::Edit, Sfx::Gem, Sfx::Bell, Sfx::Locked, Sfx::Complete, Sfx::Pad1, Sfx::Pad2];

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
        let w = (TAU * freq * t).sin() + 0.3 * (TAU * 2.0 * freq * t).sin() + 0.1 * (TAU * 3.0 * freq * t).sin();
        *slot += w * env * gain;
    }
}

fn buffer(seconds: f32) -> Vec<f32> {
    vec![0.0; (seconds * RATE as f32) as usize]
}

fn synth(sfx: Sfx) -> Vec<u8> {
    let samples = match sfx {
        Sfx::Step => {
            let mut b = buffer(0.09);
            tone(&mut b, 0.0, 740.0, 0.09, 0.25, 45.0);
            b
        }
        Sfx::Edit => {
            // A soft, filtered click of noise: chiselling stone.
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
        Sfx::Gem => {
            let mut b = buffer(0.7);
            for (k, f) in [1046.5, 1318.5, 1568.0, 2093.0].iter().enumerate() {
                tone(&mut b, k as f32 * 0.06, *f, 0.5, 0.18, 7.0);
            }
            b
        }
        Sfx::Bell => {
            let mut b = buffer(0.15);
            tone(&mut b, 0.0, 196.0, 0.15, 0.3, 25.0);
            b
        }
        Sfx::Locked => {
            let mut b = buffer(0.22);
            tone(&mut b, 0.0, 233.0, 0.1, 0.25, 30.0);
            tone(&mut b, 0.09, 175.0, 0.13, 0.25, 25.0);
            b
        }
        Sfx::Complete => {
            let mut b = buffer(1.6);
            for (k, f) in [523.25, 659.25, 783.99, 1046.5, 1318.5].iter().enumerate() {
                tone(&mut b, k as f32 * 0.11, *f, 1.2, 0.17, 3.5);
            }
            tone(&mut b, 0.55, 261.63, 1.0, 0.15, 2.5);
            b
        }
        Sfx::Pad1 | Sfx::Pad2 => {
            // A slow chord that swells in and out: the night air.
            let chord: &[f32] = if sfx == Sfx::Pad1 { &[220.0, 261.63, 329.63, 440.0] } else { &[196.0, 246.94, 293.66, 392.0] };
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

pub fn play(sfx: Sfx, volume: f32) {
    if MUTED.with(|m| m.get()) {
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
    if MUTED.with(|m| m.get()) {
        return;
    }
    let rates = [1.0, 1.12, 0.94, 1.06, 0.89];
    SOUNDS.with(|s| {
        if let Some((_, sound)) = s.borrow().iter().find(|(k, _)| *k == Sfx::Step) {
            sound.play(0.35, rates[variant % rates.len()]);
        }
    });
}

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

pub fn muted() -> bool {
    MUTED.with(|m| m.get())
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
