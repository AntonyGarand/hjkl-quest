//! What the game reads and sets from outside: the cursor, the command
//! being typed, and the bits of state the UI shows.

use super::{Editor, Key, Mode, Pos, format_keys};

impl Editor {
    /// The same editor with the cursor at `at`.
    #[must_use]
    pub fn with_cursor(mut self, at: Pos) -> Self {
        self.set_cursor(at);
        self
    }

    /// Drops a half-typed command (its count, operator...).
    pub fn cancel_pending(&mut self) {
        self.pending.clear();
        self.recording = None;
    }

    /// Puts the cursor back somewhere (kept on the text), and makes its
    /// column the one `j` and `k` remember.
    pub fn set_cursor(&mut self, at: Pos) {
        self.cursor = at;
        self.clamp();
        self.want = self.cursor.col;
    }

    /// The keys of the command being typed (vim's `showcmd`).
    pub fn pending(&self) -> String {
        format_keys(&self.pending)
    }

    /// Whether the next key is a character argument (after `f`, `t`, `r`, ...).
    pub fn awaiting_char(&self) -> bool {
        let keys = &self.pending;
        let visual = self.in_visual();
        let mut i = 0;
        while i < keys.len() {
            let before_is_count = keys[..i].iter().all(|k| matches!(k, Key::Char('0'..='9')));
            let takes_arg = matches!(keys[i], Key::Char('f' | 'F' | 't' | 'T' | 'r' | 'm' | '`' | '\'' | '"' | 'q' | '@'))
                // i and a pick a text object after an operator, or in visual mode.
                || matches!(keys[i], Key::Char('i' | 'a')) && (visual || !before_is_count);
            if takes_arg && i + 1 == keys.len() {
                return true;
            }
            i += if takes_arg { 2 } else { 1 };
        }
        false
    }

    /// Whether a visual mode (`v`, `V` or Ctrl-v) is on.
    pub fn in_visual(&self) -> bool {
        matches!(self.mode, Mode::Visual { .. } | Mode::Block)
    }

    /// The register a macro is being recorded into.
    pub fn recording_macro(&self) -> Option<char> {
        self.macro_rec.as_ref().map(|(r, _)| *r)
    }

    /// Whether the pending keys end with a count digit.
    pub fn in_count(&self) -> bool {
        let pending = &self.pending;
        match pending.last() {
            Some(Key::Char('1'..='9')) => true,
            Some(Key::Char('0')) => {
                pending.len() > 1 && matches!(pending[pending.len() - 2], Key::Char('0'..='9'))
            }
            _ => false,
        }
    }

    /// Ctrl-g: where the cursor is (vimtutor 4.1).
    pub fn where_am_i(&self) -> String {
        let n = self.lines.len();
        let row = self.cursor.row + 1;
        format!(
            "\"isle\" {n} lines --{}%-- line {row} of {n}, col {}",
            row * 100 / n,
            self.cursor.col + 1
        )
    }

    /// The prompt in front of the command line: `:`, `/` or `?`.
    pub fn cmd_prefix(&self) -> char {
        match self.mode {
            Mode::Command { search: true } if self.search_back => '?',
            Mode::Command { search: true } => '/',
            _ => ':',
        }
    }
}
