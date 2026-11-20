//! History: undo and redo (`u`, Ctrl-r), the line `U` restores, and the
//! change `.` repeats.

use super::{Editor, Key, Mode, Outcome, Pos, pos};

/// The buffer and cursor at one point in the undo history.
#[derive(Debug, Clone)]
pub(super) struct Snapshot {
    lines: Vec<Vec<char>>,
    cursor: Pos,
}

impl Editor {
    /// The buffer and cursor as they are now.
    pub(super) fn checkpoint(&self) -> Snapshot {
        Snapshot {
            lines: self.lines.clone(),
            cursor: self.cursor,
        }
    }

    /// Saves an undo step before a change (none while replaying `.` or
    /// inside an ex command), and remembers the line for `U`.
    pub(super) fn snapshot(&mut self) {
        if self.replaying || self.quiet > 0 {
            return;
        }
        let row = self.cursor.row;
        if self.line_undo.as_ref().map(|(r, _)| *r) != Some(row) {
            self.line_undo = Some((row, self.lines[row].clone()));
        }
        self.undo.push(self.checkpoint());
        self.redo.clear();
    }

    /// Saves an undo step even while replaying (`.` and `U` are one step).
    pub(super) fn snapshot_force(&mut self) {
        if self.quiet > 0 {
            return;
        }
        self.undo.push(self.checkpoint());
        self.redo.clear();
    }

    /// Ends an ex command's change: one undo step back to `before`.
    pub(super) fn commit(&mut self, before: Snapshot) {
        if !self.replaying && self.quiet == 0 {
            self.undo.push(before);
            self.redo.clear();
        }
        self.changed();
    }

    /// [`Editor::commit`], when the command changed the lines at all.
    pub(super) fn commit_if_changed(&mut self, before: Snapshot) {
        if self.lines != before.lines {
            self.commit(before);
        }
    }

    /// Steps `n` times through the history, back (undo) or forward (redo);
    /// returns how many steps there were.
    fn travel(&mut self, back: bool, n: usize) -> usize {
        let mut done = 0;
        for _ in 0..n {
            let (from, to) = if back {
                (&mut self.undo, &mut self.redo)
            } else {
                (&mut self.redo, &mut self.undo)
            };
            let Some(s) = from.pop() else { continue };
            to.push(Snapshot {
                lines: std::mem::replace(&mut self.lines, s.lines),
                cursor: self.cursor,
            });
            self.cursor = s.cursor;
            done += 1;
        }
        if done > 0 {
            self.changed();
            self.clamp();
        }
        done
    }

    /// `u`: undoes `n` changes.
    pub(super) fn undo_steps(&mut self, n: usize) -> Outcome {
        self.recording = None;
        if self.travel(true, n) == 0 {
            return Outcome::Bell("Already at oldest change".into());
        }
        Outcome::Ok
    }

    /// Ctrl-r: redoes `n` changes.
    pub(super) fn redo_change(&mut self, n: usize) -> Outcome {
        if self.travel(false, n) == 0 {
            return Outcome::Bell("Already at newest change".into());
        }
        Outcome::Ok
    }

    /// `U`: puts back the last changed line as it was (itself one change,
    /// so `U` again undoes the `U`).
    pub(super) fn restore_line(&mut self) -> Outcome {
        self.recording = None;
        let Some((row, text)) = self.line_undo.take() else {
            return Outcome::Bell("No line to restore".into());
        };
        if row >= self.lines.len() {
            return Outcome::Bell("No line to restore".into());
        }
        self.snapshot_force();
        let now = std::mem::replace(&mut self.lines[row], text);
        self.line_undo = Some((row, now));
        self.cursor = pos(row, 0);
        self.want = 0;
        self.changed();
        Outcome::Ok
    }

    /// `.`: replays the last change; a count replaces the one it had.
    pub(super) fn repeat_change(&mut self, keys: &[Key], i: usize, count: Option<usize>) -> Outcome {
        self.recording = None;
        if self.last_change.is_empty() {
            return Outcome::Bell("No change to repeat".into());
        }
        let mut change = self.last_change.clone();
        if count.is_some() {
            let mut j = 0;
            Self::read_count(&change, &mut j);
            let mut with: Vec<Key> = keys[..i].to_vec();
            with.extend_from_slice(&change[j..]);
            change = with;
        }
        self.pending.clear();
        self.replaying = true;
        self.depth += 1;
        self.snapshot_force();
        for &k in &change {
            self.feed(k);
        }
        if matches!(self.mode, Mode::Insert | Mode::Replace) {
            self.feed(Key::Esc);
        }
        self.depth -= 1;
        self.replaying = false;
        self.last_change = change;
        Outcome::Ok
    }
}
