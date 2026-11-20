//! Normal mode: a count, a register, then a command, a motion or an
//! operator. The pending keys are parsed again as each key comes in, so a
//! command runs as soon as it is whole.

use super::{Editor, Key, Mode, Outcome, Parse, parse_keys, pos};

/// Two optional counts multiplied (`2d3w` deletes six words).
pub(super) fn mul_counts(a: Option<usize>, b: Option<usize>) -> Option<usize> {
    match (a, b) {
        (None, None) => None,
        (a, b) => Some((a.unwrap_or(1) * b.unwrap_or(1)).min(9999)),
    }
}

impl Editor {
    /// A count at `keys[*i..]` (a leading `0` is no count), moving `i` past it.
    pub(super) fn read_count(keys: &[Key], i: &mut usize) -> Option<usize> {
        let mut n: Option<usize> = None;
        while let Some(Key::Char(c)) = keys.get(*i) {
            if !c.is_ascii_digit() || (*c == '0' && n.is_none()) {
                break;
            }
            let digit = c.to_digit(10).unwrap() as usize;
            n = Some(n.unwrap_or(0).saturating_mul(10).saturating_add(digit).min(9999));
            *i += 1;
        }
        n
    }

    /// The count, an optional `"x` register and the count after it; returns
    /// the total count and where the command itself starts.
    pub(super) fn read_prefix(&mut self, keys: &[Key]) -> Result<(Option<usize>, usize), Parse> {
        let mut i = 0;
        let count = Self::read_count(keys, &mut i);
        if keys.get(i) != Some(&Key::Char('"')) {
            self.active_reg = None;
            return Ok((count, i));
        }
        let Some(&r) = keys.get(i + 1) else {
            return Err(Parse::Incomplete);
        };
        let Key::Char(r) = r else {
            return Err(Parse::Invalid);
        };
        if !Self::valid_register(r) {
            return Err(Parse::Invalid);
        }
        self.active_reg = Some(r);
        i += 2;
        let count2 = Self::read_count(keys, &mut i);
        Ok((mul_counts(count, count2), i))
    }

    /// The keys typed so far in normal mode, run once they are a command.
    pub(super) fn run_normal(&mut self, keys: &[Key]) -> Result<Outcome, Parse> {
        let (count, i) = self.read_prefix(keys)?;
        let Some(&key) = keys.get(i) else {
            return Err(Parse::Incomplete);
        };
        if let Some(result) = self.operator_or_g(keys, i, count) {
            return result;
        }
        if let Some((motion, end)) = self.parse_motion(keys, i, count)? {
            if end != keys.len() {
                return Err(Parse::Invalid);
            }
            self.recording = None;
            self.apply_motion(motion);
            return Ok(Outcome::Ok);
        }
        let n = count.unwrap_or(1);
        let Key::Char(c) = key else {
            return self.control_key(key, n);
        };
        if c == 'q' && self.macro_rec.is_some() {
            if keys.len() > i + 1 {
                return Err(Parse::Invalid);
            }
            self.stop_recording();
            return Ok(Outcome::Ok);
        }
        if matches!(c, 'r' | 'm' | 'q' | '@') {
            return self.char_command(keys, i, n);
        }
        if keys.len() > i + 1 {
            return Err(Parse::Invalid);
        }
        self.normal_command(keys, i, count, c)
    }

    /// Operators (`d c y > <`, `gu gU g~`) and the `g` commands that are
    /// not motions; `None` when the keys are neither.
    fn operator_or_g(
        &mut self,
        keys: &[Key],
        i: usize,
        count: Option<usize>,
    ) -> Option<Result<Outcome, Parse>> {
        match (keys[i], keys.get(i + 1)) {
            (Key::Char(op @ ('d' | 'c' | 'y' | '>' | '<')), _) => {
                Some(self.run_operator(keys, i + 1, count, op))
            }
            (Key::Char('g'), Some(&Key::Char(op @ ('u' | 'U' | '~')))) => {
                Some(self.run_operator(keys, i + 2, count, op))
            }
            (Key::Char('g'), Some(&Key::Char(g @ ('n' | 'N' | 'J' | 'i' | 'v' | 'p' | 'P' | '&')))) => {
                if keys.len() > i + 2 {
                    return Some(Err(Parse::Invalid));
                }
                Some(self.g_command(g, count.unwrap_or(1)))
            }
            _ => None,
        }
    }

    /// `gn` (select the next match), `gJ` (join, no spaces), `gi` (insert
    /// where the last insert stopped), `gv` (the last selection again), `gp`
    /// and `gP` (put, and end up just after it), `g&` (`:s` again, everywhere).
    fn g_command(&mut self, g: char, n: usize) -> Result<Outcome, Parse> {
        let outcome = match g {
            'n' | 'N' => return self.select_match(g == 'n'),
            'J' => self.join_lines(n, false),
            'i' => self.insert_where_left(),
            'v' => self.reselect(),
            '&' => self.substitute_all_again(),
            _ => self.put_and_follow(g == 'p', n),
        };
        Ok(outcome)
    }

    /// The control keys of normal mode.
    fn control_key(&mut self, key: Key, n: usize) -> Result<Outcome, Parse> {
        let outcome = match key {
            Key::CtrlA => return Ok(self.increment(n as i64)),
            Key::CtrlX => return Ok(self.increment(-(n as i64))),
            Key::CtrlR => {
                self.recording = None;
                self.redo_change(n)
            }
            Key::CtrlO | Key::Tab => {
                self.recording = None;
                self.jump_through(key == Key::CtrlO, n)
            }
            Key::CtrlG => {
                self.recording = None;
                Outcome::Info(self.where_am_i())
            }
            Key::CtrlV => {
                self.recording = None;
                self.mode = Mode::Block;
                self.anchor = self.cursor;
                Outcome::Ok
            }
            _ => return Err(Parse::Invalid),
        };
        Ok(outcome)
    }

    /// `r`, `m`, `q` and `@`, which take the character after them.
    fn char_command(&mut self, keys: &[Key], i: usize, n: usize) -> Result<Outcome, Parse> {
        let Some(&next) = keys.get(i + 1) else {
            return Err(Parse::Incomplete);
        };
        let Key::Char(arg) = next else {
            return Err(Parse::Invalid);
        };
        if keys.len() > i + 2 {
            return Err(Parse::Invalid);
        }
        match keys[i] {
            Key::Char('r') => self.replace_chars(arg, n),
            Key::Char('m') => self.set_mark(arg),
            Key::Char('q') => self.start_recording(arg),
            _ => self.play_macro(arg, n),
        }
    }

    /// A one-key command `c`, at `keys[i]`.
    fn normal_command(
        &mut self,
        keys: &[Key],
        i: usize,
        count: Option<usize>,
        c: char,
    ) -> Result<Outcome, Parse> {
        let n = count.unwrap_or(1);
        let outcome = match c {
            'x' | 'X' | 's' | 'D' | 'C' | 'S' | 'Y' => return self.shorthand(keys, i, c),
            'p' | 'P' => self.put_command(c == 'p', n),
            'u' => self.undo_steps(n),
            '.' => self.repeat_change(keys, i, count),
            'U' => self.restore_line(),
            'J' => self.join_lines(n, true),
            '~' => self.toggle_case_chars(n),
            '&' => self.substitute_line_again(),
            'R' => {
                self.snapshot();
                self.replaced.clear();
                self.mode = Mode::Replace;
                Outcome::Ok
            }
            'i' | 'a' | 'I' | 'A' | 'o' | 'O' => self.start_insert(c),
            'v' | 'V' => {
                self.recording = None;
                self.mode = Mode::Visual { line: c == 'V' };
                self.anchor = self.cursor;
                Outcome::Ok
            }
            ':' | '/' | '?' => self.open_command_line(c),
            _ => return Err(Parse::Invalid),
        };
        Ok(outcome)
    }

    /// `x X s D C S Y`: short for `dl dh cl d$ c$ cc yy`.
    fn shorthand(&mut self, keys: &[Key], i: usize, c: char) -> Result<Outcome, Parse> {
        let motion_keys = match c {
            'x' => "dl",
            'X' => "dh",
            's' => "cl",
            'D' => "d$",
            'C' => "c$",
            'S' => "cc",
            _ => "yy",
        };
        if (c == 'x' && self.lines[self.cursor.row].is_empty())
            || (c == 'X' && (self.lines[self.cursor.row].is_empty() || self.cursor.col == 0))
        {
            return Ok(Outcome::Bell("Nothing to delete".into()));
        }
        let mut expanded: Vec<Key> = keys[..i].to_vec();
        expanded.extend(parse_keys(motion_keys));
        self.run_normal(&expanded)
    }

    /// `i a I A o O`: into insert mode, somewhere on (or around) the line.
    fn start_insert(&mut self, c: char) -> Outcome {
        self.snapshot();
        let row = self.cursor.row;
        match c {
            'a' => self.cursor.col = (self.cursor.col + 1).min(self.line_len(row)),
            'I' => self.cursor.col = self.first_non_blank(row).min(self.line_len(row)),
            'A' => self.cursor.col = self.line_len(row),
            'o' | 'O' => {
                let at = if c == 'o' { row + 1 } else { row };
                self.lines.insert(at, Vec::new());
                self.cursor = pos(at, 0);
                self.changed();
            }
            _ => {}
        }
        self.mode = Mode::Insert;
        Outcome::Ok
    }

    /// `:`, `/` and `?`: opens the command line.
    fn open_command_line(&mut self, c: char) -> Outcome {
        self.recording = None;
        self.mode = Mode::Command { search: c != ':' };
        if c != ':' {
            self.search_back = c == '?';
        }
        self.cmdline.clear();
        Outcome::Ok
    }
}
