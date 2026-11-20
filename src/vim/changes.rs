//! The small changes of normal mode: joining lines, case, replacing
//! characters, Ctrl-a, putting a register, and shifting lines.

use super::{Editor, Outcome, Parse, Register, SHIFT, pos};

/// The other case of `c` (itself when it has none).
fn toggle_case(c: char) -> char {
    if c.is_lowercase() {
        c.to_uppercase().next().unwrap_or(c)
    } else if c.is_uppercase() {
        c.to_lowercase().next().unwrap_or(c)
    } else {
        c
    }
}

/// `>`: a shiftwidth of spaces in front of a line (not an empty one).
fn indent(line: &mut Vec<char>) {
    if !line.is_empty() {
        line.splice(0..0, std::iter::repeat_n(' ', SHIFT));
    }
}

/// `<`: up to a shiftwidth of leading spaces off a line, or one tab.
fn outdent(line: &mut Vec<char>) {
    let n = line.iter().take(SHIFT).take_while(|c| **c == ' ').count();
    let n = if n == 0 && line.first() == Some(&'\t') {
        1
    } else {
        n
    };
    line.drain(..n);
}

impl Editor {
    /// `J` (one space between the lines) or `gJ` (none): joins `n` lines,
    /// at least two.
    pub(super) fn join_lines(&mut self, n: usize, spaces: bool) -> Outcome {
        if self.cursor.row + 1 >= self.lines.len() {
            return Outcome::Bell("Nothing below to join".into());
        }
        self.snapshot();
        for _ in 0..n.saturating_sub(1).max(1) {
            let row = self.cursor.row;
            if row + 1 >= self.lines.len() {
                break;
            }
            let next = self.lines.remove(row + 1);
            if spaces {
                self.join_spaced(row, next);
            } else {
                self.cursor.col = self.lines[row].len();
                self.lines[row].extend(next);
            }
        }
        self.changed();
        self.clamp();
        Outcome::Ok
    }

    /// Joins `next` onto `row` as `J` does: its leading blanks go, and one
    /// space goes between (none before a `)`).
    fn join_spaced(&mut self, row: usize, next: Vec<char>) {
        let next: Vec<char> = next.into_iter().skip_while(|c| c.is_whitespace()).collect();
        while self.lines[row].last() == Some(&' ') {
            self.lines[row].pop();
        }
        let col = self.lines[row].len();
        if !next.is_empty() && !self.lines[row].is_empty() && next[0] != ')' {
            self.lines[row].push(' ');
        }
        self.lines[row].extend(next);
        self.cursor.col = col;
    }

    /// `~`: toggles the case of `n` characters and moves past them.
    pub(super) fn toggle_case_chars(&mut self, n: usize) -> Outcome {
        let row = self.cursor.row;
        if self.lines[row].is_empty() {
            return Outcome::Bell(String::new());
        }
        self.snapshot();
        let end = (self.cursor.col + n).min(self.line_len(row));
        self.recase(&[(row, self.cursor.col, end)], '~');
        self.cursor.col = end.min(self.last_col(row));
        self.changed();
        Outcome::Ok
    }

    /// Changes the case of the cells `(row, from, to)`: `u` lower, `U`
    /// upper, else toggled.
    pub(super) fn recase(&mut self, cells: &[(usize, usize, usize)], how: char) {
        for &(row, a, b) in cells {
            let b = b.min(self.line_len(row));
            let Some(chars) = self.lines[row].get_mut(a..b) else {
                continue;
            };
            for ch in chars {
                *ch = match how {
                    'u' => ch.to_lowercase().next().unwrap_or(*ch),
                    'U' => ch.to_uppercase().next().unwrap_or(*ch),
                    _ => toggle_case(*ch),
                };
            }
        }
    }

    /// `r{char}`: replaces `n` characters with `with`.
    pub(super) fn replace_chars(&mut self, with: char, n: usize) -> Result<Outcome, Parse> {
        let (row, col) = (self.cursor.row, self.cursor.col);
        if col + n > self.line_len(row) {
            return Err(Parse::Invalid);
        }
        self.snapshot();
        self.lines[row][col..col + n].fill(with);
        self.cursor.col += n - 1;
        self.changed();
        Ok(Outcome::Ok)
    }

    /// Ctrl-a / Ctrl-x: adds `by` to the number at or after the cursor.
    pub(super) fn increment(&mut self, by: i64) -> Outcome {
        let row = self.cursor.row;
        let line = &self.lines[row];
        let mut col = self.cursor.col;
        if line.get(col).is_some_and(char::is_ascii_digit) {
            while col > 0 && line[col - 1].is_ascii_digit() {
                col -= 1;
            }
        } else {
            match (col..line.len()).find(|&k| line[k].is_ascii_digit()) {
                Some(k) => col = k,
                None => return Outcome::Bell("No number under or after the cursor".into()),
            }
        }
        let mut end = col;
        while end < line.len() && line[end].is_ascii_digit() {
            end += 1;
        }
        let negative = col > 0 && line[col - 1] == '-';
        let start = if negative { col - 1 } else { col };
        let digits: String = line[col..end].iter().collect();
        let value = digits.parse::<i64>().unwrap_or(0) * if negative { -1 } else { 1 };
        let new: Vec<char> = (value + by).to_string().chars().collect();
        self.snapshot();
        let len = new.len();
        self.lines[row].splice(start..end, new);
        self.cursor.col = start + len - 1;
        self.want = self.cursor.col;
        self.changed();
        Outcome::Ok
    }

    /// `p` / `P`: puts the register `n` times, after or before the cursor.
    pub(super) fn put_command(&mut self, after: bool, n: usize) -> Outcome {
        let reg = self.read_reg();
        if reg.text.is_empty() && !reg.linewise {
            return Outcome::Bell("Nothing in that register yet".into());
        }
        self.snapshot();
        self.put(after, n);
        self.changed();
        Outcome::Ok
    }

    /// `gp` / `gP`: puts, and leaves the cursor just after the new text.
    pub(super) fn put_and_follow(&mut self, after: bool, n: usize) -> Outcome {
        let linewise = self.read_reg().linewise;
        let rows = self.lines.len();
        let outcome = self.put_command(after, n);
        if outcome != Outcome::Ok {
            return outcome;
        }
        if linewise {
            let row = self.cursor.row + (self.lines.len() - rows);
            self.cursor = pos(row.min(self.lines.len() - 1), 0);
        } else {
            self.cursor.col += 1;
        }
        self.want = self.cursor.col;
        Outcome::Ok
    }

    /// Puts the register `n` times: whole lines below (or above) the
    /// cursor's, or text after (or at) the cursor.
    fn put(&mut self, after: bool, n: usize) {
        let reg = self.read_reg();
        if reg.linewise {
            self.put_lines(&reg, after, n);
        } else {
            self.put_text(&reg.text.repeat(n), after);
        }
        self.clamp();
    }

    fn put_lines(&mut self, reg: &Register, after: bool, n: usize) {
        let new: Vec<Vec<char>> = (0..n)
            .flat_map(|_| reg.text.split('\n').map(|l| l.chars().collect::<Vec<char>>()))
            .collect();
        let at = if after {
            self.cursor.row + 1
        } else {
            self.cursor.row
        };
        self.lines.splice(at..at, new);
        self.cursor = pos(at, self.first_non_blank(at));
    }

    fn put_text(&mut self, text: &str, after: bool) {
        let row = self.cursor.row;
        let col = if after && !self.lines[row].is_empty() {
            self.cursor.col + 1
        } else {
            self.cursor.col
        };
        let col = col.min(self.line_len(row));
        let tail: Vec<char> = self.lines[row].split_off(col);
        let parts: Vec<&str> = text.split('\n').collect();
        let mut end = pos(row, col);
        for (k, part) in parts.iter().enumerate() {
            if k > 0 {
                self.lines.insert(end.row + 1, Vec::new());
                end = pos(end.row + 1, 0);
            }
            self.lines[end.row].extend(part.chars());
            end.col = self.line_len(end.row);
        }
        self.lines[end.row].extend(tail);
        self.cursor = if parts.len() > 1 {
            pos(row, col)
        } else {
            pos(end.row, end.col.saturating_sub(1))
        };
    }

    /// Shifts rows `a..=b` a shiftwidth right or left.
    pub(super) fn shift_lines(&mut self, a: usize, b: usize, right: bool) {
        for line in &mut self.lines[a..=b] {
            if right {
                indent(line);
            } else {
                outdent(line);
            }
        }
    }
}
