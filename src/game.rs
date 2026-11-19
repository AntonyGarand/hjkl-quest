//! A level being played: the editor, the crystals, the keystroke count and
//! which keys the player has learned. No UI here either.

use crate::levels::{LEVELS, Level, unlocked};
use crate::vim::{Editor, Key, Mode, Outcome, Pos};

/// What a key did, for the UI to react with sound and motion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Moved,
    Edited,
    Gem(Pos),
    Bell,
    Locked(char),
    Complete,
    Command(String),
    None,
}

pub struct Session {
    pub index: usize,
    pub editor: Editor,
    pub gems: Vec<(Pos, bool)>,
    pub keys: Vec<Key>,
    pub complete: bool,
    pub message: String,
    allowed: String,
}

pub fn stars(keys: usize, par: usize) -> u8 {
    if keys <= par {
        3
    } else if keys <= par + par.div_ceil(2) {
        2
    } else {
        1
    }
}

impl Session {
    pub fn new(index: usize) -> Self {
        let level = &LEVELS[index];
        let editor = Editor::new(level.text()).with_cursor(level.start());
        let mut session = Session {
            index,
            editor,
            gems: level.gems().into_iter().map(|p| (p, false)).collect(),
            keys: Vec::new(),
            complete: false,
            message: String::new(),
            allowed: unlocked(index),
        };
        session.collect();
        session
    }

    pub fn level(&self) -> &'static Level {
        &LEVELS[self.index]
    }

    pub fn par(&self) -> usize {
        self.level().par()
    }

    pub fn stars(&self) -> u8 {
        stars(self.keys.len(), self.par())
    }

    pub fn gems_left(&self) -> usize {
        self.gems.iter().filter(|(_, got)| !got).count()
    }

    /// Whether the buffer still differs from the goal.
    pub fn goal_met(&self) -> bool {
        self.level().goal_text().is_none_or(|g| self.editor.text() == g)
    }

    fn is_allowed(&self, key: Key) -> Result<(), char> {
        let normalish = matches!(self.editor.mode, Mode::Normal | Mode::Visual { .. });
        if !normalish || self.editor.awaiting_char() {
            return Ok(());
        }
        match key {
            Key::Esc => Ok(()),
            Key::CtrlR => {
                if self.allowed.contains('u') {
                    Ok(())
                } else {
                    Err('r')
                }
            }
            Key::Enter | Key::Backspace => Err('⏎'),
            Key::Char(c) => {
                let pending = self.editor.pending();
                if c == '0' && self.editor.in_count() {
                    return Ok(());
                }
                // After an operator, i and a pick text objects, the
                // operator's own key repeats it (dd) and g starts gg.
                let after_operator = pending.ends_with(['d', 'c', 'y'])
                    || pending.chars().last().is_some_and(|l| l.is_ascii_digit()) && pending.contains(['d', 'c', 'y']);
                if after_operator && (c == 'i' || c == 'a') {
                    return Ok(());
                }
                if pending.ends_with(['i', 'a']) && pending.contains(['d', 'c', 'y']) {
                    return Ok(());
                }
                if matches!(self.editor.mode, Mode::Visual { .. }) && matches!(c, 'i' | 'a' | 'o' | 'd' | 'x' | 'y' | 'c' | 's' | 'u' | 'U' | '~' | 'J') {
                    return Ok(());
                }
                if self.allowed.contains(c) { Ok(()) } else { Err(c) }
            }
        }
    }

    /// Takes one key from the player.
    pub fn press(&mut self, key: Key) -> Event {
        if self.complete {
            return Event::None;
        }
        if let Err(c) = self.is_allowed(key) {
            self.message = if c == '⏎' {
                "That key does nothing in normal mode".into()
            } else {
                format!("You haven't learned `{c}` yet")
            };
            return Event::Locked(c);
        }
        self.keys.push(key);
        let revision = self.editor.revision;
        let cursor = self.editor.cursor;
        let outcome = self.editor.feed(key);
        self.message.clear();
        let mut event = match outcome {
            Outcome::Ok => {
                if self.editor.revision != revision {
                    Event::Edited
                } else if self.editor.cursor != cursor {
                    Event::Moved
                } else {
                    Event::None
                }
            }
            Outcome::Bell(msg) => {
                self.message = msg;
                Event::Bell
            }
            Outcome::Command(cmd) => {
                // Not a key that counts: the game's own commands.
                self.keys.pop();
                return Event::Command(cmd);
            }
        };
        if let Some(gem) = self.collect() {
            event = Event::Gem(gem);
        }
        if self.check_complete() {
            event = Event::Complete;
        }
        event
    }

    fn collect(&mut self) -> Option<Pos> {
        if !self.editor.pending().is_empty() {
            return None;
        }
        let at = self.editor.cursor;
        let gem = self.gems.iter_mut().find(|(p, got)| *p == at && !got)?;
        gem.1 = true;
        Some(at)
    }

    fn check_complete(&mut self) -> bool {
        if self.gems_left() == 0
            && self.goal_met()
            && self.editor.mode == Mode::Normal
            && self.editor.pending().is_empty()
        {
            self.complete = true;
        }
        self.complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vim::{format_keys, parse_keys};

    #[test]
    fn every_solution_solves_its_level_at_par() {
        for (i, level) in LEVELS.iter().enumerate() {
            let mut s = Session::new(i);
            assert!(!s.complete, "level {i} {} starts complete", level.title);
            let keys = parse_keys(level.solution);
            for (k, &key) in keys.iter().enumerate() {
                assert!(!s.complete, "level {i} {} completes early after {}", level.title, format_keys(&keys[..k]));
                let event = s.press(key);
                assert!(
                    !matches!(event, Event::Locked(_) | Event::Bell),
                    "level {i} {}: key {} ({k}) gave {event:?}: {}\nbuffer:\n{}",
                    level.title,
                    key,
                    s.message,
                    s.editor.text()
                );
            }
            assert!(
                s.complete,
                "level {i} {} not solved by {}: {} gems left, cursor {:?}, buffer:\n{}",
                level.title,
                level.solution,
                s.gems_left(),
                s.editor.cursor,
                s.editor.text()
            );
            assert_eq!(s.keys.len(), level.par());
            assert_eq!(s.stars(), 3);
        }
    }

    #[test]
    fn marks_lie_on_the_map() {
        for level in LEVELS {
            let e = Editor::new(level.text());
            for p in level.gems().into_iter().chain([level.start()]) {
                assert!(
                    p.row < e.lines.len() && (p.col < e.line_len(p.row) || p.col == 0),
                    "{}: mark {p:?} is off the map",
                    level.title
                );
            }
        }
    }

    #[test]
    fn locked_keys_do_not_count() {
        let mut s = Session::new(0);
        assert_eq!(s.press(Key::Char('w')), Event::Locked('w'));
        assert!(s.keys.is_empty());
        assert_eq!(s.press(Key::Char('l')), Event::Moved);
        assert_eq!(s.keys.len(), 1);
    }

    #[test]
    fn star_thresholds() {
        assert_eq!(stars(10, 10), 3);
        assert_eq!(stars(15, 10), 2);
        assert_eq!(stars(16, 10), 1);
    }
}
