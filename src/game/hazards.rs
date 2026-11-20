//! What moves between the player's commands: crystals picked up, bugs
//! crawling and hatching, sentries marching and crystals fading.

use super::{Bug, HATCH_EVERY, MAX_BUGS, Session};
use crate::vim::{Mode, Pos};

/// Moves `bug` a tile along its line of `len` letters, turning at the ends
/// and before traps.
fn crawl_one(bug: &mut Bug, len: usize, traps: &[Pos]) {
    bug.at.col = bug.at.col.min(len - 1);
    let row = bug.at.row;
    let open = |col: isize| {
        col >= 0
            && (col as usize) < len
            && !traps.contains(&Pos {
                row,
                col: col as usize,
            })
    };
    if !open(bug.at.col as isize + bug.dir as isize) {
        bug.dir = -bug.dir;
    }
    let ahead = bug.at.col as isize + bug.dir as isize;
    if open(ahead) {
        bug.at.col = ahead as usize;
    }
}

/// Removes the bugs at `at`, returning how many there were.
fn squash(bugs: &mut Vec<Bug>, at: Pos) -> usize {
    let before = bugs.len();
    bugs.retain(|b| b.at != at);
    before - bugs.len()
}

impl Session {
    /// Whether a whole command was just done (not a half-typed one, and
    /// not in the middle of typing text or a command line).
    fn command_done(&self) -> bool {
        let typing = matches!(
            self.editor.mode,
            Mode::Command { .. } | Mode::Insert | Mode::Replace | Mode::Confirm
        );
        self.editor.pending().is_empty() && !typing
    }

    /// Picks up the crystal the cursor stands on, if any.
    pub(super) fn collect(&mut self) -> Option<Pos> {
        if !self.editor.pending().is_empty() {
            return None;
        }
        let at = self.editor.cursor;
        let gem = self.gems.iter_mut().find(|(p, got)| *p == at && !got)?;
        gem.1 = true;
        Some(at)
    }

    /// A command was completed: a bug the cursor stands on is squashed;
    /// the others crawl a tile, and every `HATCH_EVERY` commands each hatches
    /// a twin.
    pub(super) fn crawl(&mut self) -> Option<Pos> {
        // Typing a command line or text is one command, done when it ends.
        if self.bugs.is_empty() || !self.command_done() {
            return None;
        }
        let at = self.editor.cursor;
        let mut caught = squash(&mut self.bugs, at);
        for bug in &mut self.bugs {
            let len = self.editor.lines.get(bug.at.row).map_or(0, Vec::len);
            if len > 0 {
                crawl_one(bug, len, &self.traps);
            }
        }
        caught += squash(&mut self.bugs, at);
        self.crawled += 1;
        if self.crawled.is_multiple_of(HATCH_EVERY) {
            self.hatch();
        }
        self.squashed += caught;
        (caught > 0).then_some(at)
    }

    /// Each bug hatches a twin crawling the other way, up to `MAX_BUGS`.
    fn hatch(&mut self) {
        let twins: Vec<Bug> = self
            .bugs
            .iter()
            .map(|b| Bug {
                at: b.at,
                dir: -b.dir,
            })
            .collect();
        for twin in twins {
            if self.bugs.len() < MAX_BUGS {
                self.bugs.push(twin);
            }
        }
        if !self.bugs.is_empty() {
            self.message = "The bugs hatched! Catch them faster".into();
        }
    }

    /// A command was done: the sentries march a row. One that meets the
    /// cursor (or that the cursor walked into) sends it back to the start.
    pub(super) fn march(&mut self) -> Option<Pos> {
        if self.sentries.is_empty() || !self.command_done() {
            return None;
        }
        let at = self.editor.cursor;
        let walked_in = self.sentries.iter().any(|s| s.at == at);
        let lines = &self.editor.lines;
        for s in &mut self.sentries {
            let open =
                |row: isize| row >= 0 && (row as usize) < lines.len() && s.at.col < lines[row as usize].len();
            if !open(s.at.row as isize + s.dir as isize) {
                s.dir = -s.dir;
            }
            let next = s.at.row as isize + s.dir as isize;
            if open(next) {
                s.at.row = next as usize;
            }
        }
        if !walked_in && !self.sentries.iter().any(|s| s.at == at) {
            return None;
        }
        self.caught += 1;
        self.editor.set_cursor(self.level().start());
        self.message = "A sentry caught you! Back to the start: time your moves".into();
        Some(at)
    }

    /// A command was done: the fading crystals dim. One that fades out
    /// ends the isle, not cleared. Returns whether that happened.
    pub(super) fn fade(&mut self) -> bool {
        if self.fading.is_empty() || !self.command_done() || self.complete {
            return false;
        }
        let gems = &self.gems;
        self.fading
            .retain(|(p, _)| gems.iter().any(|(g, got)| g == p && !got));
        for (_, left) in &mut self.fading {
            *left = left.saturating_sub(1);
        }
        if self.fading.iter().any(|(_, left)| *left == 0) {
            self.complete = true;
            self.missed = vec!["a crystal faded: reach them sooner, with bigger jumps".into()];
            return true;
        }
        false
    }
}
