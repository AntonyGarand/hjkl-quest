//! Keys on an isle: to the editor while it's being played, to the card's
//! choices once it's done.

use crate::game::{Event, Session};
use crate::grammar;
use crate::levels::{LEVELS, Level};
use crate::ui::keys::{Outcome, command, record};
use crate::ui::replay::Replay;
use crate::ui::state::{Screen, State, is_boss, now_ms, work};
use crate::vim::Key;

/// The isle on screen, as the keys after a run need it.
#[derive(Clone, Copy)]
pub(super) struct Isle {
    pub(super) index: usize,
    pub(super) level: &'static Level,
    pub(super) campaign: bool,
    pub(super) free: bool,
}

impl Isle {
    fn of(session: &Session) -> Isle {
        Isle {
            index: session.index,
            level: session.level(),
            campaign: session.in_campaign(),
            free: session.free,
        }
    }
}

pub(super) fn key(state: &mut State, key: Key, outcome: &mut Outcome) -> Option<Event> {
    let Screen::Play(session) = &state.screen else {
        return None;
    };
    let isle = Isle::of(session);
    if session.complete {
        // A missed lesson: Enter tries the isle again.
        let failed = !session.missed.is_empty();
        return after_run(state, isle, failed, key, outcome);
    }
    let event = press(state, key, outcome);
    match &event {
        Event::Command(cmd) => command::run(state, isle, cmd.trim(), outcome),
        Event::Complete => record::clear(state, isle),
        _ => {}
    }
    Some(event)
}

/// The keys on the card at the end of a run: replay, retry, next, map.
fn after_run(state: &mut State, isle: Isle, failed: bool, key: Key, outcome: &mut Outcome) -> Option<Event> {
    let Isle {
        index,
        level,
        campaign,
        free,
    } = isle;
    match key {
        Key::Char('p') => {
            let Screen::Play(done) = std::mem::replace(&mut state.screen, Screen::Menu { selected: index })
            else {
                unreachable!()
            };
            state.screen = Screen::Replay(Box::new(Replay::new(*done)));
        }
        Key::Char('r') | Key::Enter if free && (failed || key == Key::Char('r')) => state.open_free(index),
        Key::Char('r') => state.retry(level, index, campaign),
        Key::Enter if failed => state.retry(level, index, campaign),
        Key::Enter | Key::Char('n') if !state.drills.is_empty() => {
            let (next, from) = state.drills.remove(0);
            let rest = std::mem::take(&mut state.drills);
            state.open_level(next, from);
            state.drills = rest;
        }
        Key::Enter | Key::Char('n') if campaign && !free && index + 1 < LEVELS.len() => state.open(index + 1),
        Key::Enter | Key::Char('n' | 'q') | Key::Esc => {
            state.drills.clear();
            state.screen = Screen::Menu { selected: index };
        }
        _ => return None,
    }
    outcome.scene_changed = true;
    Some(Event::Moved)
}

/// Gives the key to the editor, starting the clock on the first and
/// stopping it on the clear, and notes new words and hits on a boss.
fn press(state: &mut State, key: Key, outcome: &mut Outcome) -> Event {
    let Screen::Play(session) = &mut state.screen else {
        unreachable!()
    };
    let (keys_before, used_before, work_before) = (session.keys.len(), session.used.len(), work(session));
    let event = session.press(key);
    if keys_before == 0 && !session.keys.is_empty() && state.started.is_none() {
        state.started = Some(now_ms());
    }
    if event == Event::Complete && state.took.is_none() {
        state.took = Some(state.started.map_or(0.0, |t| now_ms() - t));
    }
    let words: Vec<String> = session.used.iter().skip(used_before).cloned().collect();
    outcome.boss_hit = is_boss(session) && !session.complete && work(session) < work_before;
    for token in &words {
        outcome.new_word |= learn_word(state, token);
    }
    event
}

/// Adds a verb + noun pair to the grammar; true when it's new.
fn learn_word(state: &mut State, token: &str) -> bool {
    let Some((verb, noun)) = grammar::pair(token) else {
        return false;
    };
    if !state.grammar.insert(grammar::key(verb, noun)) {
        return false;
    }
    let total = grammar::VERBS.len() * grammar::NOUNS.len();
    state.toast = format!(
        "New word: {verb} + {noun}  ·  {} of {total} in your grammar (v on the map)",
        state.grammar.len()
    );
    state.save_grammar();
    true
}
