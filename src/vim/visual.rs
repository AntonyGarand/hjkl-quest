//! Visual mode (`v`, `V`, and the keys block mode shares with them):
//! moving the selection, picking objects, and the commands on it.

use super::operator::Span;
use super::{Editor, Key, Mode, Outcome, Parse, Pos, parse_keys, pos};

impl Editor {
    /// The keys typed so far in a visual mode, run once they are a command.
    pub(super) fn run_visual(&mut self, keys: &[Key]) -> Result<Outcome, Parse> {
        let (count, i) = self.read_prefix(keys)?;
        let Some(&key) = keys.get(i) else {
            return Err(Parse::Incomplete);
        };
        let (line, block) = match self.mode {
            Mode::Visual { line } => (line, false),
            Mode::Block => (false, true),
            _ => return Err(Parse::Invalid),
        };
        if key == Key::CtrlV {
            self.mode = if block { Mode::Normal } else { Mode::Block };
            return Ok(Outcome::Ok);
        }
        if matches!(key, Key::Char('i' | 'a')) {
            return self.select_object(keys, i);
        }
        if key == Key::Char('g') && matches!(keys.get(i + 1), Some(Key::Char('n' | 'N'))) {
            let (_, e) = self
                .match_at_or_after(keys[i + 1] == Key::Char('n'))
                .ok_or(Parse::Invalid)?;
            self.cursor = self.before(e);
            return Ok(Outcome::Ok);
        }
        if let Some((motion, end)) = self.parse_motion(keys, i, count)? {
            if end != keys.len() {
                return Err(Parse::Invalid);
            }
            self.apply_motion(motion);
            return Ok(Outcome::Ok);
        }
        let Key::Char(c) = key else {
            return Err(Parse::Invalid);
        };
        if c == 'r' {
            return self.visual_replace(keys, i);
        }
        if keys.len() > i + 1 {
            return Err(Parse::Invalid);
        }
        if let Some(outcome) = self.visual_shared(c) {
            return Ok(outcome);
        }
        if block {
            return self.block_command(c);
        }
        self.visual_command(c, line)
    }

    /// The selection's start and end, in buffer order.
    fn selection(&self) -> (Pos, Pos) {
        if self.anchor <= self.cursor {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }

    /// The selected columns of each row: `(row, from, to)`.
    fn visual_cells(&self) -> Vec<(usize, usize, usize)> {
        let line = match self.mode {
            Mode::Block => return self.block_cells(),
            Mode::Visual { line } => line,
            _ => return Vec::new(),
        };
        let (s, e) = self.selection();
        (s.row..=e.row)
            .map(|r| {
                let len = self.line_len(r);
                let from = if line || r != s.row { 0 } else { s.col.min(len) };
                let to = if line || r != e.row {
                    len
                } else {
                    (e.col + 1).min(len)
                };
                (r, from, to.max(from))
            })
            .collect()
    }

    /// `i{obj}` / `a{obj}`: selects a text object.
    fn select_object(&mut self, keys: &[Key], i: usize) -> Result<Outcome, Parse> {
        let Some(&obj) = keys.get(i + 1) else {
            return Err(Parse::Incomplete);
        };
        let Key::Char(obj) = obj else {
            return Err(Parse::Invalid);
        };
        match self
            .object_span(keys[i] == Key::Char('i'), obj)
            .ok_or(Parse::Invalid)?
        {
            Span::Lines(a, b) => {
                self.mode = Mode::Visual { line: true };
                self.anchor = pos(a, 0);
                self.cursor = pos(b, 0);
            }
            Span::Chars(s, e) => {
                if e <= s {
                    return Ok(Outcome::Bell(String::new()));
                }
                self.anchor = s;
                self.cursor = self.before(e);
            }
        }
        Ok(Outcome::Ok)
    }

    /// `r{char}`: every selected character becomes `char`.
    fn visual_replace(&mut self, keys: &[Key], i: usize) -> Result<Outcome, Parse> {
        let Some(&next) = keys.get(i + 1) else {
            return Err(Parse::Incomplete);
        };
        let Key::Char(with) = next else {
            return Err(Parse::Invalid);
        };
        if keys.len() > i + 2 {
            return Err(Parse::Invalid);
        }
        let (s, _) = self.selection();
        let cells = self.visual_cells();
        self.snapshot();
        for &(row, a, b) in &cells {
            self.lines[row][a..b].fill(with);
        }
        self.mode = Mode::Normal;
        self.cursor = cells.first().map_or(s, |&(row, a, _)| pos(row, a));
        self.changed();
        self.clamp();
        Ok(Outcome::Ok)
    }

    /// The commands all visual modes share: `o`, case, shifts and `:`.
    /// `None` when `c` is not one of them.
    fn visual_shared(&mut self, c: char) -> Option<Outcome> {
        let (s, e) = self.selection();
        match c {
            'o' => std::mem::swap(&mut self.anchor, &mut self.cursor),
            '~' | 'u' | 'U' => {
                let cells = self.visual_cells();
                self.mode = Mode::Normal;
                self.snapshot();
                self.recase(&cells, c);
                self.cursor = cells.first().map_or(s, |&(row, a, _)| pos(row, a));
                self.changed();
                self.clamp();
            }
            '>' | '<' => {
                self.mode = Mode::Normal;
                return Some(self.operate(c, Span::Lines(s.row, e.row)));
            }
            ':' => {
                self.marks.insert('<', pos(s.row, 0));
                self.marks.insert('>', pos(e.row, 0));
                self.recording = None;
                self.mode = Mode::Command { search: false };
                self.cmdline = "'<,'>".into();
            }
            _ => return None,
        }
        Some(Outcome::Ok)
    }

    /// The commands of `v` and `V`: switching between them, operators, `J`.
    fn visual_command(&mut self, c: char, line: bool) -> Result<Outcome, Parse> {
        let (s, e) = self.selection();
        let span = if line {
            Span::Lines(s.row, e.row)
        } else {
            Span::Chars(s, self.after(e))
        };
        match c {
            'v' | 'V' => {
                let want_line = c == 'V';
                self.mode = if want_line == line {
                    Mode::Normal
                } else {
                    Mode::Visual { line: want_line }
                };
                Ok(Outcome::Ok)
            }
            'd' | 'x' | 'y' | 'c' | 's' => {
                self.mode = Mode::Normal;
                let op = match c {
                    'x' => 'd',
                    's' => 'c',
                    o => o,
                };
                Ok(self.operate(op, span))
            }
            'J' => {
                self.mode = Mode::Normal;
                let (a, b) = (s.row, e.row.max(s.row + 1));
                self.cursor = pos(a, self.cursor.col);
                let keys = parse_keys(&format!("{}J", b - a + 1));
                self.run_normal(&keys)
            }
            _ => Err(Parse::Invalid),
        }
    }

    /// `gn` in normal mode: selects the match under or after the cursor.
    pub(super) fn select_match(&mut self, forward: bool) -> Result<Outcome, Parse> {
        let (s, e) = self.match_at_or_after(forward).ok_or(Parse::Invalid)?;
        self.recording = None;
        self.mode = Mode::Visual { line: false };
        self.anchor = s;
        self.cursor = if e > s { self.before(e) } else { s };
        Ok(Outcome::Ok)
    }

    /// `gv`: the last selection again.
    pub(super) fn reselect(&mut self) -> Outcome {
        let Some((anchor, cursor, mode)) = self.last_visual else {
            return Outcome::Bell("No previous selection".into());
        };
        self.recording = None;
        let fit = |e: &Self, p: Pos| {
            let row = p.row.min(e.lines.len() - 1);
            pos(row, p.col.min(e.last_col(row)))
        };
        self.anchor = fit(self, anchor);
        self.cursor = fit(self, cursor);
        self.mode = mode;
        Outcome::Ok
    }
}
