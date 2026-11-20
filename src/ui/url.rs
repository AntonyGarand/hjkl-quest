//! The address: it names the isle on screen (`#slug`, `#map` on the map),
//! so a reload or a shared link opens the same isle.

use std::cell::Cell;

use nsh_core::prelude::*;

use crate::levels::LEVELS;
use crate::ui::motion::Ui;
use crate::ui::state::{DAILY_TIDE, Screen, State, with_state};
use crate::ui::theme::sync_theme;

/// An isle's name in the address: its title in lower case, words joined by
/// dashes ("Tag, You're It" is `tag-youre-it`), so links survive new isles.
fn slug(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if c == ' ' && !out.ends_with('-') {
            out.push('-');
        }
    }
    out
}

/// The isle an address names (`…#looking-back`), if any.
fn isle_in(url: &str) -> Option<usize> {
    let (_, fragment) = url.split_once('#')?;
    LEVELS.iter().position(|l| slug(l.title) == fragment)
}

/// What the address calls the screen.
fn fragment(state: &State) -> String {
    match &state.screen {
        Screen::Play(p) if p.level().chapter == DAILY_TIDE => "daily".to_string(),
        Screen::Play(p) if !p.in_campaign() => "review".to_string(),
        Screen::Play(p) => slug(LEVELS[p.index].title),
        Screen::Grammar { .. } => "grammar".to_string(),
        Screen::Menu { .. } | Screen::Replay(_) => "map".to_string(),
    }
}

thread_local! {
    /// Set once the address the app opened with has been read: before
    /// that, the screen must not overwrite it.
    static URL_READ: Cell<bool> = const { Cell::new(false) };
}

/// Keeps the current isle in the address, so reloading the page restarts
/// the isle; opens the isle an address names, at start and when the
/// browser's back or forward changes it.
pub(crate) fn follow_url(ui: Ui) {
    effect(move || {
        ui.scene.get();
        if !URL_READ.with(Cell::get) {
            return;
        }
        let here = with_state(|s| fragment(s));
        let _ = set_app_url(format!("#{here}"));
    });
    spawn(async move {
        let url = app_url().await;
        if let Some(i) = url.as_deref().and_then(isle_in) {
            with_state(|s| s.open(i));
        } else if url.as_deref().is_some_and(|u| u.ends_with("#daily")) {
            with_state(State::open_daily);
        }
        sync_theme(ui);
        URL_READ.with(|r| r.set(true));
        ui.bump_scene();
        loop {
            let change = next_app_url_change().await;
            let target = isle_in(&change.url);
            with_state(|s| go_to(s, target));
            sync_theme(ui);
            ui.bump_scene();
            ui.bump_tick();
        }
    });
}

/// Opens the isle the address now names, or the map at the isle on screen.
fn go_to(state: &mut State, target: Option<usize>) {
    if let Some(i) = target {
        state.open(i);
    } else {
        state.screen = Screen::Menu {
            selected: state.current_isle(),
        };
        state.intro = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_isle_has_its_own_address() {
        assert_eq!(slug("Tag, You're It"), "tag-youre-it");
        assert_eq!(slug("Wind-up Golem"), "windup-golem");
        let mut slugs: Vec<String> = LEVELS.iter().map(|l| slug(l.title)).collect();
        slugs.sort();
        slugs.dedup();
        assert_eq!(slugs.len(), LEVELS.len());
        for (i, level) in LEVELS.iter().enumerate() {
            let url = format!("https://antonygarand.github.io/hjkl-quest/#{}", slug(level.title));
            assert_eq!(isle_in(&url), Some(i));
        }
        assert_eq!(isle_in("https://x/hjkl-quest/#map"), None);
        assert_eq!(isle_in("https://x/hjkl-quest/"), None);
    }
}
