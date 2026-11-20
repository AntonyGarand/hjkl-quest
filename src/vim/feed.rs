//! Keys in: [`Editor::feed`] hands each key to the mode it is typed in,
//! records it for the macro and the change being made, and replays keys
//! for macros and `:normal`.

use super::{Editor, Key, Mode, Outcome, Parse, format_keys, pos};

impl Editor {
    /// Takes one key.
    pub fn feed(&mut self, key: Key) -> Outcome {
        if self.depth == 0
            && let Some((_, keys)) = &mut self.macro_rec
        {
            keys.push(key);
        }
        if let Some(rec) = &mut self.recording {
            rec.push(key);
        }
        let visual_before = self.in_visual().then_some((self.anchor, self.cursor, self.mode));
        let outcome = match self.mode {
            Mode::Insert => self.insert_key(key),
            Mode::Replace => self.replace_key(key),
            Mode::Command { search } => self.command_key(key, search),
            Mode::Confirm => self.confirm_key(key),
            Mode::Normal | Mode::Visual { .. } | Mode::Block => self.normal_key(key),
        };
        if visual_before.is_some() && !self.in_visual() {
            self.last_visual = visual_before;
        }
        // A change ends when we are back in normal mode with nothing pending.
        if self.mode == Mode::Normal
            && self.pending.is_empty()
            && let Some(rec) = self.recording.take()
            && !self.replaying
        {
            self.last_change = rec;
        }
        outcome
    }

    /// Feeds keys and returns what the last one did.
    pub fn feed_all_last(&mut self, keys: &[Key]) -> Outcome {
        let mut last = Outcome::Ok;
        for &k in keys {
            last = self.feed(k);
        }
        last
    }

    /// Feeds keys, one after the other.
    pub fn feed_all(&mut self, keys: &[Key]) {
        for &k in keys {
            self.feed(k);
        }
    }

    /// Feeds replayed keys (a macro, `:normal`); stops at the first error,
    /// as vim aborts a macro. Returns whether all keys ran.
    pub(super) fn replay(&mut self, keys: &[Key]) -> bool {
        self.depth += 1;
        let mut ok = true;
        for &k in keys {
            if let Outcome::Bell(msg) = self.feed(k)
                && !msg.is_empty()
            {
                ok = false;
                break;
            }
        }
        self.depth -= 1;
        ok
    }

    /// Runs normal-mode keys from the start of `row`, as `:normal` does,
    /// and ends any mode they leave open.
    pub(super) fn run_normal_at(&mut self, row: usize, keys: &[Key]) {
        self.mode = Mode::Normal;
        self.pending.clear();
        self.cursor = pos(row, 0);
        self.want = 0;
        self.replay(keys);
        self.pending.clear();
        if self.mode != Mode::Normal {
            self.depth += 1;
            self.feed(Key::Esc);
            if self.mode != Mode::Normal {
                self.feed(Key::Esc);
            }
            self.depth -= 1;
        }
        self.mode = Mode::Normal;
        self.clamp();
    }

    /// A key in normal or visual mode: added to the pending keys, which
    /// run once they make a whole command.
    fn normal_key(&mut self, key: Key) -> Outcome {
        if key == Key::Esc {
            return self.escape_normal();
        }
        if self.pending.is_empty() && self.recording.is_none() && self.mode == Mode::Normal {
            self.recording = Some(vec![key]);
        }
        self.pending.push(key);
        let keys = self.pending.clone();
        let result = if self.in_visual() {
            self.run_visual(&keys)
        } else {
            self.run_normal(&keys)
        };
        match result {
            Err(Parse::Incomplete) => Outcome::Ok,
            Err(Parse::Invalid) => {
                self.pending.clear();
                self.recording = None;
                Outcome::Bell(format!("Not a command: {}", format_keys(&keys)))
            }
            Ok(outcome) => {
                self.pending.clear();
                if self.mode == Mode::Normal {
                    self.clamp();
                }
                outcome
            }
        }
    }

    /// Esc in normal or visual mode: drops the pending keys or the
    /// selection; with neither, it is a bell.
    fn escape_normal(&mut self) -> Outcome {
        let had = !self.pending.is_empty();
        self.pending.clear();
        self.recording = None;
        if self.in_visual() {
            self.mode = Mode::Normal;
            self.clamp();
            return Outcome::Ok;
        }
        if had {
            Outcome::Ok
        } else {
            Outcome::Bell(String::new())
        }
    }
}
