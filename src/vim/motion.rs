//! Motions: where a key moves the cursor, and how an operator sees the
//! text it passes over (exclusive, inclusive or whole lines).

use super::{Editor, Key, Parse, Pos, pos};

/// How an operator takes the text a motion covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MotionKind {
    Exclusive,
    Inclusive,
    Linewise,
}

/// Where a motion goes, and what it does on the way.
#[derive(Debug, Clone, Copy)]
pub(super) struct Motion {
    pub(super) to: Pos,
    pub(super) kind: MotionKind,
    /// Whether the motion sets the remembered column (all but j/k).
    keeps_want: bool,
    /// `$`: the remembered column is the end of every line.
    want_end: bool,
    /// A jump (G, %, n, marks...): sets the `` ` `` mark to where it left.
    jump: bool,
}

impl Motion {
    /// A plain motion to `to`.
    fn to(to: Pos, kind: MotionKind) -> Self {
        Motion {
            to,
            kind,
            keeps_want: true,
            want_end: false,
            jump: false,
        }
    }

    /// A jump to `to`.
    fn jump(to: Pos, kind: MotionKind) -> Self {
        Motion {
            jump: true,
            ..Motion::to(to, kind)
        }
    }
}

/// `f` and `t` take their target in; `F` and `T` stop short of it.
fn find_kind(kind: char) -> MotionKind {
    if kind == 'f' || kind == 't' {
        MotionKind::Inclusive
    } else {
        MotionKind::Exclusive
    }
}

/// The find `,` runs: the last one, the other way.
fn reverse_find(kind: char) -> char {
    match kind {
        'f' => 'F',
        'F' => 'f',
        't' => 'T',
        _ => 't',
    }
}

impl Editor {
    /// Parses a motion at `keys[i..]`; returns it and where it ends, or
    /// `Ok(None)` when the key is no motion.
    pub(super) fn parse_motion(
        &mut self,
        keys: &[Key],
        i: usize,
        count: Option<usize>,
    ) -> Result<Option<(Motion, usize)>, Parse> {
        let Some(&Key::Char(c)) = keys.get(i) else {
            return if i >= keys.len() {
                Err(Parse::Incomplete)
            } else {
                Ok(None)
            };
        };
        if !matches!(c, 'g' | 'f' | 'F' | 't' | 'T' | '`' | '\'') {
            return Ok(self.plain_motion(c, count)?.map(|m| (m, i + 1)));
        }
        let Some(&next) = keys.get(i + 1) else {
            return Err(Parse::Incomplete);
        };
        let Key::Char(arg) = next else {
            return Err(Parse::Invalid);
        };
        let motion = match c {
            'g' => self.g_motion(arg, count)?,
            '`' | '\'' => self.mark_motion(c == '`', arg)?,
            _ => {
                self.last_find = Some((c, arg));
                let to = self
                    .find(c, arg, count.unwrap_or(1), false)
                    .ok_or(Parse::Invalid)?;
                Motion::to(to, find_kind(c))
            }
        };
        Ok(Some((motion, i + 2)))
    }

    /// The motions of one key.
    fn plain_motion(&mut self, c: char, count: Option<usize>) -> Result<Option<Motion>, Parse> {
        use MotionKind::{Exclusive, Inclusive, Linewise};
        let n = count.unwrap_or(1);
        let cur = self.cursor;
        let motion = match c {
            'h' => Motion::to(pos(cur.row, cur.col.saturating_sub(n)), Exclusive),
            'l' | ' ' => Motion::to(pos(cur.row, (cur.col + n).min(self.line_len(cur.row))), Exclusive),
            'j' | 'k' => self.vertical(c == 'j', n)?,
            '0' => Motion::to(pos(cur.row, 0), Exclusive),
            '^' => Motion::to(self.line_start(cur.row), Exclusive),
            '$' => self.to_line_end(n),
            '|' => Motion::to(pos(cur.row, (n - 1).min(self.last_col(cur.row))), Exclusive),
            'w' | 'W' | 'b' | 'B' | 'e' | 'E' => self.word_motion(c, n),
            'G' | 'H' | 'M' | 'L' => Motion::jump(self.line_start(self.target_row(c, count)), Linewise),
            '_' | '+' | '-' => Motion::to(self.line_start(self.relative_row(c, n)?), Linewise),
            // {n}%: that far through the file, in percent.
            '%' if count.is_some() => Motion::jump(self.line_start(self.percent_row(n)), Linewise),
            '%' => Motion::jump(self.match_pair(cur).ok_or(Parse::Invalid)?, Inclusive),
            ';' | ',' => self.repeat_find(c == ',', n)?,
            '(' | ')' => Motion::jump(self.sentence_from(c == ')', n), Exclusive),
            '{' | '}' => Motion::jump(self.paragraph_from(c == '}', n), Exclusive),
            'n' | 'N' => Motion::jump(self.search_next(c == 'n', n)?, Exclusive),
            '*' | '#' => Motion::jump(self.search_word(c == '*', n)?, Exclusive),
            _ => return Ok(None),
        };
        Ok(Some(motion))
    }

    /// The motions after `g`: `gg`, `ge`, `gE`, `g_`, `g*` and `g#`.
    fn g_motion(&mut self, g: char, count: Option<usize>) -> Result<Motion, Parse> {
        let n = count.unwrap_or(1);
        let cur = self.cursor;
        let motion = match g {
            'g' => Motion::jump(self.line_start(self.target_row('g', count)), MotionKind::Linewise),
            'e' | 'E' => {
                let to = (0..n).fold(cur, |p, _| self.word_end_backward(p, g == 'E'));
                Motion::to(to, MotionKind::Inclusive)
            }
            '_' => {
                let row = (cur.row + n - 1).min(self.lines.len() - 1);
                let col = self.lines[row]
                    .iter()
                    .rposition(|c| !c.is_whitespace())
                    .unwrap_or(0);
                Motion::to(pos(row, col), MotionKind::Inclusive)
            }
            // g* and g#: the word under the cursor, also inside longer words.
            '*' | '#' => Motion::jump(self.search_partial_word(g == '*', n)?, MotionKind::Exclusive),
            _ => return Err(Parse::Invalid),
        };
        Ok(motion)
    }

    /// `` `x `` (the mark itself) or `'x` (its line).
    fn mark_motion(&self, exact: bool, name: char) -> Result<Motion, Parse> {
        let name = if name == '`' { '\'' } else { name };
        let at = *self.marks.get(&name).ok_or(Parse::Invalid)?;
        let row = at.row.min(self.lines.len() - 1);
        Ok(if exact {
            Motion::jump(pos(row, at.col.min(self.last_col(row))), MotionKind::Exclusive)
        } else {
            Motion::jump(self.line_start(row), MotionKind::Linewise)
        })
    }

    /// `j` and `k`: down or up, back to the remembered column.
    fn vertical(&self, down: bool, n: usize) -> Result<Motion, Parse> {
        let cur = self.cursor;
        let row = if down {
            if cur.row + 1 >= self.lines.len() {
                return Err(Parse::Invalid);
            }
            (cur.row + n).min(self.lines.len() - 1)
        } else {
            if cur.row == 0 {
                return Err(Parse::Invalid);
            }
            cur.row.saturating_sub(n)
        };
        let col = self.want.min(self.last_col(row));
        Ok(Motion {
            keeps_want: false,
            ..Motion::to(pos(row, col), MotionKind::Linewise)
        })
    }

    /// `$`: the end of the line (`n - 1` lines down), and of every line
    /// `j` and `k` go to after it.
    fn to_line_end(&self, n: usize) -> Motion {
        let row = (self.cursor.row + n - 1).min(self.lines.len() - 1);
        Motion {
            want_end: true,
            ..Motion::to(pos(row, self.last_col(row)), MotionKind::Inclusive)
        }
    }

    /// `w W b B e E`, `n` times.
    fn word_motion(&self, c: char, n: usize) -> Motion {
        let big = c.is_ascii_uppercase();
        let step: fn(&Self, Pos, bool) -> Pos = match c.to_ascii_lowercase() {
            'w' => Self::word_forward,
            'b' => Self::word_backward,
            _ => Self::word_end,
        };
        let to = (0..n).fold(self.cursor, |p, _| step(self, p, big));
        let kind = if matches!(c, 'e' | 'E') {
            MotionKind::Inclusive
        } else {
            MotionKind::Exclusive
        };
        Motion::to(to, kind)
    }

    /// The row of `G` (the last, or line `count`), `gg` (the first, or line
    /// `count`), and `H`, `M`, `L` (top, middle, bottom).
    fn target_row(&self, c: char, count: Option<usize>) -> usize {
        let len = self.lines.len();
        let n = count.unwrap_or(1);
        match c {
            'G' => count.map_or(len - 1, |n| n.max(1).min(len) - 1),
            'g' => count.map_or(0, |n| n.max(1).min(len) - 1),
            'H' => (n - 1).min(len - 1),
            'L' => (len - 1).saturating_sub(n - 1),
            _ => (len - 1) / 2,
        }
    }

    /// The row of `_` (`n - 1` down), `+` (`n` down) and `-` (`n` up).
    fn relative_row(&self, c: char, n: usize) -> Result<usize, Parse> {
        let row = self.cursor.row;
        let row = match c {
            '_' => row + n - 1,
            '+' => row + n,
            _ => row.checked_sub(n).ok_or(Parse::Invalid)?,
        };
        if row >= self.lines.len() {
            return Err(Parse::Invalid);
        }
        Ok(row)
    }

    /// The row `n` percent of the way through the buffer.
    fn percent_row(&self, n: usize) -> usize {
        let n = n.min(100);
        ((n * self.lines.len()).div_ceil(100)).max(1) - 1
    }

    /// `;` (the last find again) or `,` (the other way).
    fn repeat_find(&self, reverse: bool, n: usize) -> Result<Motion, Parse> {
        let (kind, target) = self.last_find.ok_or(Parse::Invalid)?;
        let kind = if reverse { reverse_find(kind) } else { kind };
        let to = self.find(kind, target, n, true).ok_or(Parse::Invalid)?;
        Ok(Motion::to(to, find_kind(kind)))
    }

    /// Moves the cursor as `motion` says.
    pub(super) fn apply_motion(&mut self, motion: Motion) {
        if motion.jump {
            self.marks.insert('\'', self.cursor);
            if motion.to != self.cursor {
                self.record_jump(self.cursor);
            }
        }
        self.cursor = motion.to;
        if motion.keeps_want {
            self.want = if motion.want_end {
                usize::MAX
            } else {
                motion.to.col
            };
        }
        self.clamp();
    }
}
