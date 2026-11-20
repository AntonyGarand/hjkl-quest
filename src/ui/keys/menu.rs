//! Keys on the world map: sailing between isles, chapters and regions,
//! and opening an isle, free play, the daily tide or review drills.

use crate::audio;
use crate::game::Event;
use crate::levels::LEVELS;
use crate::story;
use crate::ui::keys::Outcome;
use crate::ui::state::{Screen, State, chapter_start};
use crate::vim::Key;

pub(super) fn key(state: &mut State, selected: usize, key: Key, outcome: &mut Outcome) -> Option<Event> {
    if let Some(to) = sail(selected, key) {
        state.screen = Screen::Menu { selected: to };
        return Some(Event::Moved);
    }
    match key {
        Key::Char('m') => audio::set_muted(!audio::muted()),
        Key::Char('v') => {
            state.screen = Screen::Grammar { selected };
            outcome.scene_changed = true;
            return Some(Event::None);
        }
        Key::Char('d') => {
            state.open_daily();
            outcome.scene_changed = true;
        }
        Key::Char('f') if state.best[selected].is_some() => {
            state.open_free(selected);
            outcome.scene_changed = true;
        }
        Key::Char('f') => {
            state.toast =
                "Free play opens once an isle is cleared: then f replays it with every key you know.".into();
        }
        Key::Char('r') if state.open_review() => outcome.scene_changed = true,
        Key::Char('r') => {
            state.toast = "Review drills come from isles you've cleared: clear a few first.".into();
        }
        Key::Enter | Key::Char(' ' | 'i') => {
            state.open(selected);
            outcome.scene_changed = true;
        }
        _ => return None,
    }
    Some(Event::Moved)
}

/// Where a movement key sails the ferry from isle `from`: h and l isle by
/// isle, j and k chapter by chapter, J and K region by region, g and G to
/// the ends. `None` for other keys.
fn sail(from: usize, key: Key) -> Option<usize> {
    let n = LEVELS.len();
    let to = match key {
        Key::Char('l' | 'w') => (from + 1).min(n - 1),
        Key::Char('h' | 'b') => from.saturating_sub(1),
        Key::Char('j') => next_chapter(from).unwrap_or(from),
        Key::Char('k') => previous_chapter(from),
        Key::Char('J') => next_region(from),
        Key::Char('K') => previous_region(from),
        Key::Char('G') => n - 1,
        Key::Char('g') => 0,
        _ => return None,
    };
    Some(to)
}

/// The first isle of the next chapter, if there is one.
fn next_chapter(from: usize) -> Option<usize> {
    let here = LEVELS[from].chapter;
    (from..LEVELS.len()).find(|&k| LEVELS[k].chapter != here)
}

/// The start of this chapter, or of the one before when already there.
fn previous_chapter(from: usize) -> usize {
    let start = chapter_start(from);
    if start < from {
        start
    } else {
        chapter_start(start.saturating_sub(1))
    }
}

fn region(isle: usize) -> usize {
    story::region_of(LEVELS[isle].chapter)
}

/// The first isle of the next region, or `from` in the last one.
fn next_region(from: usize) -> usize {
    let here = region(from);
    (from..LEVELS.len()).find(|&k| region(k) != here).unwrap_or(from)
}

/// The start of this region, or of the one before when already there.
fn previous_region(from: usize) -> usize {
    let here = region(from);
    let start = (0..=from)
        .rev()
        .take_while(|&k| region(k) == here)
        .last()
        .unwrap_or(0);
    if start < from {
        return start;
    }
    let before = region(start.saturating_sub(1));
    (0..start)
        .rev()
        .take_while(|&k| region(k) == before)
        .last()
        .unwrap_or(0)
}
