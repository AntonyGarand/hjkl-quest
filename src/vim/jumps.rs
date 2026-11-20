//! Marks (`m`) and the jump list (Ctrl-o, Ctrl-i).

use super::{Editor, Outcome, Parse, Pos, pos};

const AT_START: &str = "At the start of the jump list";
const AT_END: &str = "At the end of the jump list";

impl Editor {
    /// `m{a-z}`: sets a mark at the cursor.
    pub(super) fn set_mark(&mut self, name: char) -> Result<Outcome, Parse> {
        if !name.is_ascii_alphabetic() && name != '\'' && name != '`' {
            return Err(Parse::Invalid);
        }
        self.recording = None;
        let name = if name == '`' { '\'' } else { name };
        self.marks.insert(name, self.cursor);
        Ok(Outcome::Ok)
    }

    /// A jump leaves `from` in the jump list (one entry per line).
    pub(super) fn record_jump(&mut self, from: Pos) {
        self.jumps.truncate(self.jump_at.min(self.jumps.len()));
        self.jumps.retain(|p| p.row != from.row);
        self.jumps.push(from);
        if self.jumps.len() > 50 {
            self.jumps.remove(0);
        }
        self.jump_at = self.jumps.len();
    }

    /// A jump from the cursor: sets the `` ` `` mark and the jump list.
    pub(super) fn jump_from_here(&mut self) {
        self.marks.insert('\'', self.cursor);
        self.record_jump(self.cursor);
    }

    /// Ctrl-o (back) and Ctrl-i / Tab (forward) through the jump list.
    pub(super) fn jump_through(&mut self, back: bool, n: usize) -> Outcome {
        for _ in 0..n {
            let step = if back {
                self.step_back()
            } else {
                self.step_forward()
            };
            if let Err(msg) = step {
                return Outcome::Bell(msg.into());
            }
            let to = self.jumps[self.jump_at];
            self.set_cursor(pos(to.row.min(self.lines.len() - 1), to.col));
        }
        Outcome::Ok
    }

    fn step_back(&mut self) -> Result<(), &'static str> {
        if self.jump_at == 0 {
            return Err(AT_START);
        }
        // Leaving the newest place: remember it, to come back with Ctrl-i.
        if self.jump_at == self.jumps.len() {
            let here = self.cursor;
            self.jumps.retain(|p| p.row != here.row);
            self.jumps.push(here);
            self.jump_at = self.jumps.len() - 1;
            if self.jump_at == 0 {
                return Err(AT_START);
            }
        }
        self.jump_at -= 1;
        Ok(())
    }

    fn step_forward(&mut self) -> Result<(), &'static str> {
        if self.jump_at + 1 >= self.jumps.len() {
            return Err(AT_END);
        }
        self.jump_at += 1;
        Ok(())
    }
}
