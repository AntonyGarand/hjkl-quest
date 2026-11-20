//! The sound and motion a key earns: bursts, shakes, the card springing
//! up, a boss flinching, dust after a long jump, a trap's throw back.

use nsh_core::prelude::*;

use crate::audio::{self, Sfx};
use crate::game::Event;
use crate::ui::colors::{BUG, GEM, LANTERN};
use crate::ui::keys::Outcome;
use crate::ui::layout::{tile_center, tile_origin};
use crate::ui::motion::{Motion, Ui};
use crate::ui::play::place_hero;
use crate::ui::state::{Screen, session_value, with_state};
use crate::ui::theme::sync_theme;
use crate::ui::worldmap;
use crate::vim::Pos;

/// How high the wisp hops when a trap throws it back.
const LIFT: f32 = 16.0;
/// How far the cursor must jump to leave a trail of dust.
const LONG_JUMP: usize = 4;
/// A trap's burst.
const TRAP_RED: u32 = 0xff6b6b;

pub(super) fn react(event: &Event, outcome: &Outcome, cursor_before: Option<Pos>, ui: Ui, motion: &Motion) {
    play_event(event, ui, motion);
    if outcome.boss_hit {
        audio::play(Sfx::Hit, 0.55);
        motion.boss.keyframes(
            [
                Keyframe::new(0.0),
                Keyframe::new(1.0),
                Keyframe::new(-0.6),
                Keyframe::new(0.3),
                Keyframe::new(0.0),
            ],
            KeyframeTiming::new(420.0),
        );
    }
    if outcome.new_word {
        audio::play(Sfx::Word, 0.4);
    }
    if !outcome.scene_changed && matches!(event, Event::Moved | Event::Gem(_) | Event::Squashed(_)) {
        dust_trail(cursor_before, ui, motion);
    }
    sync_theme(ui);
    if with_state(|s| matches!(s.screen, Screen::Menu { .. })) {
        worldmap::follow(ui, motion, !outcome.scene_changed);
    }
    place_hero(ui, motion, !outcome.scene_changed);
    if let Event::Trapped(p) = event {
        throw_back(*p, motion);
    }
}

/// The event's own sound, and its burst or shake.
fn play_event(event: &Event, ui: Ui, motion: &Motion) {
    match event {
        Event::Moved => audio::step(ui.tick.get_untracked() as usize),
        Event::Edited => audio::play(Sfx::Edit, 0.5),
        Event::Gem(p) => {
            audio::play(Sfx::Gem, 0.55);
            burst(ui, motion, tile_center(*p), GEM, 650.0);
        }
        Event::Squashed(p) => {
            audio::play(Sfx::Squash, 0.6);
            let (cx, cy) = tile_center(*p);
            burst(ui, motion, (cx, cy + 6.0), BUG, 550.0);
        }
        Event::Bell => {
            audio::play(Sfx::Bell, 0.35);
            shake(motion, &[-4.0, 4.0, -2.0], 220.0);
        }
        Event::Caught(p) => {
            audio::play(Sfx::Locked, 0.6);
            audio::play(Sfx::Hit, 0.5);
            burst(ui, motion, tile_center(*p), LANTERN, 500.0);
            shake(motion, &[-9.0, 9.0, -5.0, 3.0], 340.0);
        }
        Event::Trapped(p) => {
            audio::play(Sfx::Locked, 0.6);
            audio::play(Sfx::Bell, 0.4);
            burst(ui, motion, tile_center(*p), TRAP_RED, 500.0);
            shake(motion, &[-9.0, 9.0, -5.0, 3.0], 340.0);
        }
        Event::Locked(_) => {
            audio::play(Sfx::Locked, 0.45);
            shake(motion, &[-6.0, 6.0, -3.0], 260.0);
        }
        Event::Complete if session_value(|s| !s.cleared()).unwrap_or(false) => {
            audio::play(Sfx::Locked, 0.5);
            spring_card(motion);
        }
        Event::Complete => {
            audio::play(Sfx::Gem, 0.4);
            audio::play(Sfx::Complete, 0.6);
            spring_card(motion);
        }
        _ => {}
    }
}

/// A ring of sparks at `at`, spreading for `ms`.
fn burst(ui: Ui, motion: &Motion, at: (f32, f32), color: u32, ms: f32) {
    ui.burst_at.set(at);
    ui.burst_color.set(color);
    motion.burst.set(0.0);
    motion
        .burst
        .timing(1.0, ms, Easing::Curve(Curve::Cubic, EaseMode::Out));
}

/// Shakes the board through `swings` and back to rest.
fn shake(motion: &Motion, swings: &[f32], ms: f32) {
    let frames = std::iter::once(0.0)
        .chain(swings.iter().copied())
        .chain(std::iter::once(0.0))
        .map(Keyframe::new);
    motion.shake.keyframes(frames, KeyframeTiming::new(ms));
}

/// The end-of-run card springs up.
fn spring_card(motion: &Motion) {
    motion.card.set(0.0);
    motion.card.spring(
        1.0,
        Spring {
            stiffness: 170.0,
            damping: 14.0,
            ..Spring::default()
        },
    );
}

/// A long jump leaves a trail of dust.
fn dust_trail(cursor_before: Option<Pos>, ui: Ui, motion: &Motion) {
    let (Some(from), Some(to)) = (cursor_before, session_value(|s| s.editor.cursor)) else {
        return;
    };
    if from.row.abs_diff(to.row) + from.col.abs_diff(to.col) < LONG_JUMP {
        return;
    }
    ui.trail.set((tile_center(from), tile_center(to)));
    motion.trail.set(0.0);
    motion
        .trail
        .timing(1.0, 650.0, Easing::Curve(Curve::Cubic, EaseMode::Out));
}

/// A trap: the wisp hops onto the spikes at `trap`, and is flung back.
fn throw_back(trap: Pos, motion: &Motion) {
    let Some(at) = session_value(|s| s.editor.cursor) else {
        return;
    };
    let ((fx, fy), (tx, ty)) = (tile_origin(at), tile_origin(trap));
    let timing = || KeyframeTiming::new(460.0).easing(Easing::Linear);
    let there_and_back = |from: f32, to: f32| {
        [
            Keyframe::new(from),
            Keyframe::at(0.4, to).ease(Easing::EaseOut),
            Keyframe::at(0.55, to),
            Keyframe::new(from).ease(Easing::EaseIn),
        ]
    };
    motion.hero_x.keyframes(there_and_back(fx, tx), timing());
    motion.hero_y.keyframes(there_and_back(fy, ty), timing());
    motion.hop.keyframes(
        [
            Keyframe::new(0.0),
            Keyframe::at(0.25, -LIFT).ease(Easing::EaseOut),
            Keyframe::at(0.4, 0.0).ease(Easing::EaseIn),
            Keyframe::at(0.7, -LIFT - 8.0).ease(Easing::EaseOut),
            Keyframe::new(0.0).ease(Easing::EaseIn),
        ],
        timing(),
    );
}
