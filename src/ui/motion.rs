//! The signals the screens redraw from, and the animated values the board
//! moves with: they run natively, with no code per frame.

use std::time::Duration;

use nsh_core::prelude::*;

use crate::ui::colors::GEM;

/// The animated values the board moves with.
pub(crate) struct Motion {
    pub(crate) hero_x: AnimatedValue,
    pub(crate) hero_y: AnimatedValue,
    pub(crate) hop: AnimatedValue,
    pub(crate) cam_x: AnimatedValue,
    pub(crate) cam_y: AnimatedValue,
    pub(crate) burst: AnimatedValue,
    pub(crate) shake: AnimatedValue,
    pub(crate) bob: AnimatedValue,
    pub(crate) float: AnimatedValue,
    pub(crate) twinkle_a: AnimatedValue,
    pub(crate) twinkle_b: AnimatedValue,
    pub(crate) fly_x: AnimatedValue,
    pub(crate) fly_y: AnimatedValue,
    pub(crate) drift: AnimatedValue,
    pub(crate) card: AnimatedValue,
    /// Rain, snow and leaves falling: 0 to 1, looping.
    pub(crate) fall: AnimatedValue,
    /// A long jump's dust trail fading: 0 to 1.
    pub(crate) trail: AnimatedValue,
    /// A boss reeling from a hit.
    pub(crate) boss: AnimatedValue,
}

impl Motion {
    pub(crate) fn new() -> Motion {
        Motion {
            hero_x: animated_value(0.0),
            hero_y: animated_value(0.0),
            hop: animated_value(0.0),
            cam_x: animated_value(0.0),
            cam_y: animated_value(0.0),
            burst: animated_value(1.0),
            shake: animated_value(0.0),
            bob: animated_value(0.0),
            float: animated_value(0.0),
            twinkle_a: animated_value(0.9),
            twinkle_b: animated_value(0.3),
            fly_x: animated_value(0.0),
            fly_y: animated_value(0.0),
            drift: animated_value(0.0),
            card: animated_value(0.0),
            fall: animated_value(0.0),
            trail: animated_value(1.0),
            boss: animated_value(0.0),
        }
    }

    /// Starts the ambient loops: bobbing crystals, twinkling stars,
    /// fireflies, drifting clouds and falling weather.
    pub(crate) fn start_ambience(&self) {
        breathe(&self.bob, -2.5, 2.5, 1100.0);
        breathe(&self.float, -3.0, 3.0, 3200.0);
        breathe(&self.twinkle_a, 0.95, 0.25, 1900.0);
        breathe(&self.twinkle_b, 0.25, 0.95, 2600.0);
        breathe(&self.fly_x, -30.0, 30.0, 7000.0);
        breathe(&self.fly_y, 12.0, -18.0, 4300.0);
        sweep(&self.drift, 90_000.0, 20.0);
        sweep(&self.fall, 9_000.0, 30.0);
    }
}

/// Swings `v` between `a` and `b` and back, forever.
fn breathe(v: &AnimatedValue, a: f32, b: f32, ms: f32) {
    v.keyframes(
        [Keyframe::new(a), Keyframe::new(b)],
        KeyframeTiming::new(ms)
            .easing(Easing::EaseInOut)
            .infinite()
            .direction(PlaybackDirection::Alternate)
            .frame_rate(30.0),
    );
}

/// Runs `v` from 0 to 1 at an even pace, over and over.
fn sweep(v: &AnimatedValue, ms: f32, frame_rate: f32) {
    v.keyframes(
        [Keyframe::new(0.0), Keyframe::new(1.0)],
        KeyframeTiming::new(ms)
            .easing(Easing::Linear)
            .infinite()
            .frame_rate(frame_rate),
    );
}

#[derive(Clone, Copy)]
pub(crate) struct Ui {
    /// Bumped after every key: what the board and panels redraw from.
    pub(crate) tick: Signal<u64>,
    /// Bumped when the screen or the level changes.
    pub(crate) scene: Signal<u64>,
    pub(crate) stage: Signal<(f32, f32)>,
    pub(crate) window: Signal<(f32, f32)>,
    pub(crate) burst_at: Signal<(f32, f32)>,
    /// Steps the intro's examples along.
    pub(crate) beat: Signal<u64>,
    /// The burst's colour: a crystal's, or a squashed bug's.
    pub(crate) burst_color: Signal<u32>,
    /// A long jump: where it started and ended, for the dust trail.
    pub(crate) trail: Signal<((f32, f32), (f32, f32))>,
    /// The chapter on screen, for the sky.
    pub(crate) chapter: Signal<usize>,
}

/// How often the beat steps the intro's examples and the replay along.
const BEAT: Duration = Duration::from_millis(520);

impl Ui {
    /// The signals, with the beat started.
    pub(crate) fn new() -> Ui {
        let ui = Ui {
            tick: signal(0_u64),
            scene: signal(0_u64),
            stage: signal((900.0_f32, 520.0_f32)),
            window: signal((1200.0_f32, 800.0_f32)),
            burst_at: signal((0.0_f32, 0.0_f32)),
            beat: signal(0_u64),
            burst_color: signal(GEM),
            trail: signal(((0.0_f32, 0.0_f32), (0.0_f32, 0.0_f32))),
            chapter: signal(0_usize),
        };
        set_interval(BEAT, move || {
            ui.beat.update(|b| *b += 1);
        });
        ui
    }

    /// Redraws what follows the keys.
    pub(crate) fn bump_tick(self) {
        self.tick.update(|t| *t += 1);
    }

    /// Rebuilds the screen.
    pub(crate) fn bump_scene(self) {
        self.scene.update(|s| *s += 1);
    }
}
