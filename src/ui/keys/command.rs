//! The game's own `:` commands: leave, restart, skip, the grammar, sound
//! and the lessons.

use crate::audio;
use crate::levels::LEVELS;
use crate::ui::keys::Outcome;
use crate::ui::keys::play::Isle;
use crate::ui::state::{Screen, State};

pub(super) fn run(state: &mut State, isle: Isle, cmd: &str, outcome: &mut Outcome) {
    let index = isle.index;
    match cmd {
        "e!" | "restart" | "r" if isle.free => state.open_free(index),
        "e!" | "restart" | "r" => state.retry(isle.level, index, isle.campaign),
        "n" | "next" if isle.campaign => state.open((index + 1).min(LEVELS.len() - 1)),
        // Out of the campaign, the next thing is the map.
        "q" | "q!" | "wq" | "x" | "levels" | "menu" | "n" | "next" => {
            state.screen = Screen::Menu { selected: index };
        }
        "grammar" => state.screen = Screen::Grammar { selected: index },
        "mute" | "sound" => {
            audio::set_muted(!audio::muted());
            state.toast = if audio::muted() { "Sound off" } else { "Sound on" }.into();
            return;
        }
        "help" | "h" | "intro" | "learn" => {
            state.intro = true;
            return;
        }
        "w" => {
            state.toast = "Saved! (progress saves by itself)".into();
            return;
        }
        other => {
            state.toast = format!("E492: Not an editor command: {other}");
            return;
        }
    }
    outcome.scene_changed = true;
}
