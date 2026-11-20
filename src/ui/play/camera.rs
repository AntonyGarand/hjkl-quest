//! The camera over the board, and the wisp's place on it.

use nsh_core::prelude::*;

use crate::ui::layout::{tile_center, tile_origin, world_size};
use crate::ui::motion::{Motion, Ui};
use crate::ui::state::{Screen, with_state};

/// How long the wisp takes to hop a tile, and the camera to follow (ms).
const HOP_MS: f32 = 120.0;
const CAMERA_MS: f32 = 380.0;
/// The fewest columns the camera frames.
const MIN_COLS: usize = 8;

/// Moves the wisp and the camera to the cursor.
pub(crate) fn place_hero(ui: Ui, motion: &Motion, animate: bool) {
    let Some((cursor, rows, cols)) = with_state(|s| match &s.screen {
        Screen::Play(session) => {
            let e = &session.editor;
            let rows = e
                .lines
                .len()
                .max(session.level().goal_text().map_or(0, |g| g.lines().count()));
            let cols = e.lines.iter().map(Vec::len).max().unwrap_or(0);
            Some((e.cursor, rows, cols))
        }
        _ => None,
    }) else {
        return;
    };
    let ease = Easing::Curve(Curve::Cubic, EaseMode::Out);
    let (x, y) = tile_origin(cursor);
    if animate {
        motion.hero_x.timing(x, HOP_MS, ease);
        motion.hero_y.timing(y, HOP_MS, ease);
        motion.hop.keyframes(
            [
                Keyframe::new(0.0),
                Keyframe::at(0.4, -9.0).ease(Easing::EaseOut),
                Keyframe::new(0.0),
            ],
            KeyframeTiming::new(200.0).easing(Easing::EaseIn),
        );
    } else {
        motion.hero_x.set(x);
        motion.hero_y.set(y);
    }

    let (target_x, target_y) = camera_target(
        ui.stage.get_untracked(),
        world_size(rows, cols.max(MIN_COLS)),
        tile_center(cursor),
    );
    if animate {
        motion.cam_x.timing(target_x, CAMERA_MS, ease);
        motion.cam_y.timing(target_y, CAMERA_MS, ease);
    } else {
        motion.cam_x.set(target_x);
        motion.cam_y.set(target_y);
    }
}

/// Centres the board on an axis where it fits the view; otherwise follows
/// the cursor, keeping the board's edges in view.
fn camera_target(view: (f32, f32), world: (f32, f32), cursor: (f32, f32)) -> (f32, f32) {
    let axis = |v: f32, w: f32, c: f32| {
        if w <= v {
            (v - w) / 2.0
        } else {
            (v / 2.0 - c).clamp(v - w, 0.0)
        }
    };
    (axis(view.0, world.0, cursor.0), axis(view.1, world.1, cursor.1))
}
