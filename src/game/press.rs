//! One key from the player: whether the isle allows it, what it did to the
//! buffer, and the event it makes.

use super::{Event, Session, is_game_command, key_name};
use crate::vim::{Editor, Key, Mode, Outcome, Pos};

/// The editor as it was before a key, to tell what the key changed.
struct Before {
    revision: u64,
    cursor: Pos,
    mode: Mode,
    pending: String,
    /// The command line the key ends with Enter, whole.
    line: Option<String>,
    /// Whether the `:` command line was open.
    in_cmdline: bool,
}

impl Before {
    fn take(editor: &Editor, key: Key) -> Self {
        let line = (matches!(editor.mode, Mode::Command { .. }) && key == Key::Enter)
            .then(|| format!("{}{}", editor.cmd_prefix(), editor.cmdline));
        Before {
            revision: editor.revision,
            cursor: editor.cursor,
            mode: editor.mode,
            pending: editor.pending(),
            line,
            in_cmdline: editor.mode == (Mode::Command { search: false }),
        }
    }
}

/// Whether `mode` reads keys as commands (not text or a command line).
fn normalish(mode: Mode) -> bool {
    matches!(mode, Mode::Normal | Mode::Visual { .. } | Mode::Block)
}

/// Whether `op`, a pending command, ends with an operator, after which `i`
/// and `a` pick text objects.
fn after_operator(op: &str) -> bool {
    op.ends_with(['d', 'c', 'y', '>', '<']) || op.ends_with("gu") || op.ends_with("gU") || op.ends_with("g~")
}

impl Session {
    /// Takes one key from the player.
    pub fn press(&mut self, key: Key) -> Event {
        if self.complete {
            return Event::None;
        }
        if let Some(event) = self.refuse(key) {
            return event;
        }
        let before = Before::take(&self.editor, key);
        if key == Key::Char(':')
            && (self.editor.mode == Mode::Normal || self.editor.in_visual())
            && self.editor.pending().is_empty()
        {
            self.cmd_start = Some(self.keys.len());
        }
        let ex_learned = self.allowed.contains(':');
        // Before ex commands are learned, only the game's own run.
        if before.in_cmdline && key == Key::Enter && !ex_learned && !is_game_command(&self.editor.cmdline) {
            return self.refuse_ex();
        }
        self.keys.push(key);
        let snapshot = (!self.frozen.is_empty()).then(|| self.editor.clone());
        let outcome = self.editor.feed(key);
        if let Some(snapshot) = snapshot
            && !self.frozen_intact()
        {
            // The ice holds: the edit didn't happen.
            self.editor = snapshot;
            self.message = "❄ That line is frozen: change the others".into();
            return Event::Bell;
        }
        self.note_used(&before, key, &outcome);
        self.message.clear();
        self.tip.clear();
        if let Some(at) = self.trap(before.cursor) {
            return Event::Trapped(at);
        }
        self.consult_coach(&before, key, &outcome);
        let left_cmdline = before.in_cmdline && self.editor.mode != (Mode::Command { search: false });
        let event = match outcome {
            Outcome::Ok => {
                if left_cmdline && !ex_learned {
                    // An abandoned command line costs nothing either.
                    self.drop_cmdline_keys();
                }
                self.change(&before)
            }
            Outcome::Bell(msg) => {
                self.message = msg;
                Event::Bell
            }
            Outcome::Info(msg) => {
                self.message = msg;
                self.change(&before)
            }
            Outcome::Command(cmd) => {
                // The game's own commands: none of their keys count.
                self.drop_cmdline_keys();
                return Event::Command(cmd);
            }
        };
        self.settle(event)
    }

    /// A key the isle bans or the player hasn't learned: says why, and the
    /// event it makes. `None` lets the key through.
    fn refuse(&mut self, key: Key) -> Option<Event> {
        if normalish(self.editor.mode)
            && !self.editor.awaiting_char()
            && let Key::Char(c) = key
            && self.level().banned().contains(c)
            && !(c == '0' && self.editor.in_count())
        {
            self.message = format!("`{c}` is banned on this isle: find another way");
            return Some(Event::Locked(c));
        }
        let c = self.is_allowed(key).err()?;
        self.message = if c == '⏎' {
            "That key does nothing in normal mode".into()
        } else {
            format!("You haven't learned `{}` yet", key_name(c))
        };
        Some(Event::Locked(c))
    }

    /// Refuses a real ex command before ex commands are learned: the
    /// command line closes, and its keys cost nothing.
    fn refuse_ex(&mut self) -> Event {
        let cmd = self.editor.cmdline.clone();
        self.editor.feed(Key::Esc);
        self.drop_cmdline_keys();
        self.message = format!(
            ":{cmd} needs ex commands, learned on isle {}. Try :e! :q :n :h :mute",
            crate::levels::ex_isle()
        );
        Event::Locked(':')
    }

    /// Whether `key` may be pressed now; the locked key if not.
    fn is_allowed(&self, key: Key) -> Result<(), char> {
        let mode = self.editor.mode;
        if !normalish(mode) || self.editor.awaiting_char() {
            return Ok(());
        }
        let pending = self.editor.pending();
        match key {
            // Undo and redo are always there, so a wrong edit never strands
            // the player before the isle that teaches them properly.
            Key::Esc | Key::CtrlR => Ok(()),
            Key::Char('u') if mode == Mode::Normal && pending.is_empty() => Ok(()),
            Key::Enter | Key::Backspace => Err('⏎'),
            // The command line always opens, for the game's own commands;
            // real ex commands wait until `:` is learned.
            Key::Char(':') if mode == Mode::Normal => Ok(()),
            // A macro being recorded can always be stopped.
            Key::Char('q') if self.editor.recording_macro().is_some() && pending.is_empty() => Ok(()),
            Key::Char(c) if self.always_allowed(c, &pending) => Ok(()),
            other if self.allowed.contains(other.raw()) => Ok(()),
            other => Err(other.raw()),
        }
    }

    /// Keys that only finish a command already begun: a count's `0`, a
    /// text object after an operator, a visual-mode action.
    fn always_allowed(&self, c: char, pending: &str) -> bool {
        if c == '0' && self.editor.in_count() {
            return true;
        }
        // After an operator, i and a pick text objects, the operator's own
        // key repeats it (dd) and g starts gg.
        let op = pending.trim_end_matches(|c: char| c.is_ascii_digit());
        if after_operator(op) && (c == 'i' || c == 'a') {
            return true;
        }
        self.editor.in_visual()
            && matches!(
                c,
                'i' | 'a' | 'o' | 'd' | 'x' | 'y' | 'c' | 's' | 'u' | 'U' | '~' | 'J'
            )
    }

    /// Remembers a finished command, as typed, to check the isle's lesson.
    fn note_used(&mut self, before: &Before, key: Key, outcome: &Outcome) {
        if let Some(line) = &before.line {
            self.used.push(line.clone());
        } else if normalish(before.mode)
            && self.editor.pending().is_empty()
            && !matches!(outcome, Outcome::Bell(_))
        {
            self.used.push(format!("{}{key}", before.pending));
        }
    }

    /// Landing on a trap throws the wisp back to `from`; the keys still
    /// count. Returns the trap, if one was hit.
    fn trap(&mut self, from: Pos) -> Option<Pos> {
        let at = self.editor.cursor;
        let hit = !matches!(
            self.editor.mode,
            Mode::Command { .. } | Mode::Insert | Mode::Replace
        ) && self.editor.pending().is_empty()
            && at != from
            && self.traps.contains(&at);
        if !hit {
            return None;
        }
        self.editor.set_cursor(from);
        self.trapped += 1;
        self.message = "Ouch, a trap! It threw you back: jump over it in one move".into();
        Some(at)
    }

    /// A finished normal-mode command goes to the coach.
    fn consult_coach(&mut self, before: &Before, key: Key, outcome: &Outcome) {
        if before.mode == Mode::Normal && self.editor.pending().is_empty() && matches!(outcome, Outcome::Ok) {
            let token = format!("{}{key}", before.pending);
            if let Some(tip) = self.coach.push(token, &self.allowed) {
                self.tip = tip;
            }
        }
    }

    /// What the key changed: the text, the cursor, or nothing.
    fn change(&self, before: &Before) -> Event {
        if self.editor.revision != before.revision {
            Event::Edited
        } else if self.editor.cursor != before.cursor {
            Event::Moved
        } else {
            Event::None
        }
    }

    /// After a command: crystals, bugs, sentries and fading take their
    /// turn, then the isle may end. Returns the event that matters most.
    fn settle(&mut self, mut event: Event) -> Event {
        if let Some(gem) = self.collect() {
            event = Event::Gem(gem);
        }
        if let Some(at) = self.crawl() {
            event = Event::Squashed(at);
        }
        if let Some(at) = self.march() {
            event = Event::Caught(at);
        }
        if self.fade() {
            return Event::Complete;
        }
        if self.check_complete() {
            event = Event::Complete;
        } else if let Some(budget) = self.level().budget()
            && !self.free
            && self.keys.len() >= budget
        {
            // Out of keystrokes: the isle ends, not cleared.
            self.complete = true;
            self.missed = vec![format!("finish within {budget} keys")];
            event = Event::Complete;
        }
        event
    }

    fn drop_cmdline_keys(&mut self) {
        if let Some(start) = self.cmd_start.take() {
            self.keys.truncate(start);
        }
    }
}
