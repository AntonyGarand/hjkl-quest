//! Insert mode and replace mode: typing into the line.

use super::{Editor, Key, Mode, Outcome, SHIFT, pos};

impl Editor {
    /// A key in insert mode.
    pub(super) fn insert_key(&mut self, key: Key) -> Outcome {
        if std::mem::take(&mut self.insert_reg) {
            return self.insert_register(key);
        }
        match key {
            Key::CtrlR => self.insert_reg = true,
            Key::Esc => self.leave_insert(),
            Key::Enter => {
                self.split_line();
                self.changed();
            }
            Key::Backspace => self.insert_backspace(),
            Key::Char(c) => {
                self.insert_char(c);
                self.changed();
            }
            Key::Tab => self.insert_tab(),
            _ => return Outcome::Bell("That key does nothing in insert mode here".into()),
        }
        Outcome::Ok
    }

    /// Ctrl-r {register}: types the register's text (a line break after
    /// it when it holds whole lines).
    fn insert_register(&mut self, key: Key) -> Outcome {
        let Key::Char(name) = key else { return Outcome::Ok };
        let Some(text) = self.register_text(name).filter(|t| !t.is_empty()) else {
            return Outcome::Bell(format!("Register {name} is empty"));
        };
        let linewise = if name == '"' {
            self.register.linewise
        } else {
            self.registers
                .get(&name.to_ascii_lowercase())
                .is_some_and(|r| r.linewise)
        };
        let text = if linewise { format!("{text}\n") } else { text };
        for c in text.chars() {
            if c == '\n' {
                self.split_line();
            } else {
                self.insert_char(c);
            }
        }
        self.changed();
        Outcome::Ok
    }

    /// Types `c` at the cursor.
    fn insert_char(&mut self, c: char) {
        self.lines[self.cursor.row].insert(self.cursor.col, c);
        self.cursor.col += 1;
    }

    /// Tab: spaces up to the next multiple of the shiftwidth.
    fn insert_tab(&mut self) {
        for _ in 0..SHIFT - self.cursor.col % SHIFT {
            self.insert_char(' ');
        }
        self.changed();
    }

    /// Backspace: deletes the character before the cursor, or joins the
    /// line to the one above at its start.
    fn insert_backspace(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
            self.lines[self.cursor.row].remove(self.cursor.col);
            self.changed();
        } else if self.cursor.row > 0 {
            let line = self.lines.remove(self.cursor.row);
            self.cursor.row -= 1;
            self.cursor.col = self.line_len(self.cursor.row);
            self.lines[self.cursor.row].extend(line);
            self.changed();
        }
    }

    /// Esc: back to normal mode (copying a block insert to its other lines).
    fn leave_insert(&mut self) {
        self.marks.insert('^', self.cursor);
        if self.finish_block_insert() {
            return;
        }
        self.back_to_normal();
    }

    /// Leaves insert or replace mode: the cursor steps back onto the text.
    fn back_to_normal(&mut self) {
        self.mode = Mode::Normal;
        self.set_cursor(pos(self.cursor.row, self.cursor.col.saturating_sub(1)));
    }

    /// `gi`: insert where the last insert stopped.
    pub(super) fn insert_where_left(&mut self) -> Outcome {
        self.snapshot();
        let at = self.marks.get(&'^').copied().unwrap_or(self.cursor);
        let row = at.row.min(self.lines.len() - 1);
        self.cursor = pos(row, at.col.min(self.line_len(row)));
        self.mode = Mode::Insert;
        Outcome::Ok
    }

    /// A key in replace mode.
    pub(super) fn replace_key(&mut self, key: Key) -> Outcome {
        match key {
            Key::Esc => self.back_to_normal(),
            Key::Backspace => self.replace_backspace(),
            Key::Enter => {
                self.replaced.clear();
                self.split_line();
                self.changed();
            }
            Key::Char(c) => self.replace_char(c),
            _ => return Outcome::Bell("That key does nothing in replace mode here".into()),
        }
        Outcome::Ok
    }

    /// Overwrites the character at the cursor (or adds one at the line's end).
    fn replace_char(&mut self, c: char) {
        let (row, col) = (self.cursor.row, self.cursor.col);
        if col < self.line_len(row) {
            self.replaced.push(Some(self.lines[row][col]));
            self.lines[row][col] = c;
        } else {
            self.replaced.push(None);
            self.lines[row].push(c);
        }
        self.cursor.col += 1;
        self.changed();
    }

    /// Backspace in replace mode: puts back what was overwritten.
    fn replace_backspace(&mut self) {
        if self.cursor.col == 0 {
            return;
        }
        self.cursor.col -= 1;
        let (row, col) = (self.cursor.row, self.cursor.col);
        match self.replaced.pop() {
            Some(Some(c)) => self.lines[row][col] = c,
            Some(None) => {
                self.lines[row].remove(col);
            }
            None => {}
        }
        self.changed();
    }
}
