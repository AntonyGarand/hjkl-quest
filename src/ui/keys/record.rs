//! Records kept when a run ends: the best keys and time on a campaign
//! isle, in free play or on the daily tide, and when each isle was last
//! practised.

use crate::daily;
use crate::ui::keys::play::Isle;
use crate::ui::state::{DAILY_TIDE, Screen, State};

/// Keeps what a run that reached the goal earned (nothing if a lesson
/// was missed).
pub(super) fn clear(state: &mut State, isle: Isle) {
    let Screen::Play(session) = &state.screen else {
        return;
    };
    if !session.cleared() {
        return;
    }
    let keys = session.keys.len();
    if isle.free {
        free_play(state, isle.index, keys as u32);
    } else if !isle.campaign && isle.level.chapter == DAILY_TIDE {
        daily_tide(state, keys);
    } else if !isle.campaign {
        // A drill keeps its isle fresh.
        state.practice(isle.index);
    } else {
        campaign(state, isle.index, keys);
    }
}

fn free_play(state: &mut State, index: usize, keys: u32) {
    if state.free_best[index].is_none_or(|b| keys < b) {
        state.free_best[index] = Some(keys);
        state.save_free_best();
    }
}

fn daily_tide(state: &mut State, keys: usize) {
    let today = daily::today();
    if state.daily_best.is_none_or(|(d, b)| d != today || keys < b) {
        state.daily_best = Some((today, keys));
        state.save_daily();
    }
}

fn campaign(state: &mut State, i: usize, keys: usize) {
    let best = &mut state.best[i];
    if best.is_none_or(|b| keys < b) {
        *best = Some(keys);
    }
    state.save_progress();
    let tenths = (state.took.unwrap_or(0.0) / 100.0) as u32;
    if state.times[i].is_none_or(|t| tenths < t) {
        state.times[i] = Some(tenths);
        state.save_times();
    }
    state.practice(i);
}
