//! The keyboard: every key vim knows is bound to `handle_key`, which hands
//! it to the screen on show, then plays the sound and motion it earned.

mod command;
mod effects;
mod menu;
mod play;
mod record;

use std::rc::Rc;

use nsh_core::keymap::{Keystroke, When};
use nsh_core::prelude::*;

use crate::game::Event;
use crate::ui::motion::{Motion, Ui};
use crate::ui::state::{Screen, State, session_value, with_state};
use crate::vim::Key;

/// What a key did besides its event, for the effects that follow.
#[derive(Default)]
struct Outcome {
    /// The screen or the isle changed: no animation from the old one.
    scene_changed: bool,
    /// A command hurt the chapter's boss.
    boss_hit: bool,
    /// A verb + noun pair new to the grammar was used.
    new_word: bool,
}

/// Plain and shifted keys, their names for the keymap, and vim's.
fn key_table() -> Vec<(String, Key)> {
    let mut keys = Vec::new();
    for c in 'a'..='z' {
        keys.push((c.to_string(), Key::Char(c)));
        keys.push((format!("shift+{c}"), Key::Char(c.to_ascii_uppercase())));
    }
    let shifted_digits = [')', '!', '@', '#', '$', '%', '^', '&', '*', '('];
    for (d, shifted) in shifted_digits.into_iter().enumerate() {
        let c = char::from_digit(d as u32, 10).unwrap();
        keys.push((c.to_string(), Key::Char(c)));
        keys.push((format!("shift+{c}"), Key::Char(shifted)));
    }
    for (plain, shifted, name) in [
        ('`', '~', "`"),
        ('-', '_', "minus"),
        ('=', '+', "="),
        ('[', '{', "["),
        (']', '}', "]"),
        ('\\', '|', "\\"),
        (';', ':', ";"),
        ('\'', '"', "'"),
        (',', '<', ","),
        ('.', '>', "."),
        ('/', '?', "/"),
    ] {
        keys.push((name.to_string(), Key::Char(plain)));
        keys.push((format!("shift+{name}"), Key::Char(shifted)));
    }
    for (name, key) in [
        ("space", Key::Char(' ')),
        ("shift+space", Key::Char(' ')),
        ("escape", Key::Esc),
        ("ctrl+[", Key::Esc),
        ("ctrl+c", Key::Esc),
        ("enter", Key::Enter),
        ("backspace", Key::Backspace),
        ("ctrl+r", Key::CtrlR),
        ("ctrl+a", Key::CtrlA),
        ("ctrl+x", Key::CtrlX),
        ("ctrl+v", Key::CtrlV),
        ("tab", Key::Tab),
        ("ctrl+i", Key::Tab),
        ("ctrl+d", Key::CtrlD),
        ("ctrl+o", Key::CtrlO),
        ("ctrl+g", Key::CtrlG),
    ] {
        keys.push((name.to_string(), key));
    }
    keys
}

/// Binds every key of the table the platform doesn't keep for itself.
pub(crate) fn bind_keys(ui: Ui, motion: &Rc<Motion>) {
    let apple = apple_keys();
    for (stroke, key) in key_table() {
        let Ok(parsed) = Keystroke::parse_with(&stroke, apple) else {
            continue;
        };
        if parsed.reserved(os()).is_some() {
            continue;
        }
        let motion = motion.clone();
        on_key_with(
            &stroke,
            KeyOptions {
                when: When::Always,
                repeat: true,
                pass_through: false,
            },
            move |_| handle_key(key, ui, &motion),
        );
    }
}

fn handle_key(key: Key, ui: Ui, motion: &Motion) {
    let mut outcome = Outcome::default();
    let cursor_before = session_value(|s| s.editor.cursor);
    let event = with_state(|state| {
        state.toast.clear();
        dispatch(state, key, &mut outcome)
    });
    let Some(event) = event else { return };
    effects::react(&event, &outcome, cursor_before, ui, motion);
    if outcome.scene_changed {
        ui.bump_scene();
    }
    ui.bump_tick();
}

/// Hands the key to the screen on show; `None` when it means nothing there.
fn dispatch(state: &mut State, key: Key, outcome: &mut Outcome) -> Option<Event> {
    match &mut state.screen {
        Screen::Menu { selected } => {
            let selected = *selected;
            menu::key(state, selected, key, outcome)
        }
        Screen::Grammar { selected } => match key {
            Key::Esc | Key::Enter | Key::Char('q' | 'v' | ' ') => {
                let selected = *selected;
                state.screen = Screen::Menu { selected };
                outcome.scene_changed = true;
                Some(Event::None)
            }
            _ => None,
        },
        Screen::Replay(r) => {
            if r.key(key) {
                leave_replay(state);
                outcome.scene_changed = true;
            }
            Some(Event::None)
        }
        // The intro takes the keyboard until it's closed.
        Screen::Play(_) if state.intro => match key {
            Key::Enter | Key::Esc | Key::Char(' ' | 'i' | 'l' | 'q') => {
                state.close_intro();
                Some(Event::None)
            }
            _ => None,
        },
        Screen::Play(_) => play::key(state, key, outcome),
    }
}

/// Back from the replay to the finished run.
fn leave_replay(state: &mut State) {
    let Screen::Replay(r) = std::mem::replace(&mut state.screen, Screen::Menu { selected: 0 }) else {
        unreachable!()
    };
    state.screen = Screen::Play(Box::new(r.session));
}
