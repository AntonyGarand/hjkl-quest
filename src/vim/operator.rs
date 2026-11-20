//! Operators (`d c y > <`, `gu gU g~`): what text they take (a doubled
//! operator's lines, an object, `gn`, or a motion's span) and what they
//! do to it.

use super::motion::{Motion, MotionKind};
use super::normal::mul_counts;
use super::{Editor, Key, Mode, Outcome, Parse, Pos, Register, pos};

/// A charwise range [start, end) or a linewise range of rows.
#[derive(Debug, Clone, Copy)]
pub(super) enum Span {
    Chars(Pos, Pos),
    Lines(usize, usize),
}

impl Editor {
    /// An operator (`d c y > <`, or `gu gU g~` as `u U ~`) from `keys[j..]`:
    /// doubled for whole lines, then a text object, `gn`, or a motion.
    pub(super) fn run_operator(
        &mut self,
        keys: &[Key],
        j: usize,
        count: Option<usize>,
        op: char,
    ) -> Result<Outcome, Parse> {
        let mut k = j;
        let count2 = Self::read_count(keys, &mut k);
        if k >= keys.len() {
            return Err(Parse::Incomplete);
        }
        let total = mul_counts(count, count2);
        let span = if let Some(end) = Self::doubled(keys, k, op)? {
            if end != keys.len() {
                return Err(Parse::Invalid);
            }
            let last = (self.cursor.row + total.unwrap_or(1) - 1).min(self.lines.len() - 1);
            Span::Lines(self.cursor.row, last)
        } else {
            self.operator_target(keys, k, total, op)?
        };
        Ok(self.operate(op, span))
    }

    /// Where a doubled operator (`dd`, `gUU`, `gUgU`) at `keys[k..]` ends,
    /// if it is one.
    fn doubled(keys: &[Key], k: usize, op: char) -> Result<Option<usize>, Parse> {
        if keys[k] == Key::Char(op) {
            return Ok(Some(k + 1));
        }
        if !matches!(op, 'u' | 'U' | '~') || keys[k] != Key::Char('g') {
            return Ok(None);
        }
        match keys.get(k + 1) {
            None => Err(Parse::Incomplete),
            Some(&next) if next == Key::Char(op) => Ok(Some(k + 2)),
            _ => Ok(None),
        }
    }

    /// What an operator takes when not doubled: a text object, `gn`, or
    /// the text a motion passes over.
    fn operator_target(
        &mut self,
        keys: &[Key],
        k: usize,
        total: Option<usize>,
        op: char,
    ) -> Result<Span, Parse> {
        let next = keys[k];
        if matches!(next, Key::Char('i' | 'a')) {
            let Some(&obj) = keys.get(k + 1) else {
                return Err(Parse::Incomplete);
            };
            if k + 2 != keys.len() {
                return Err(Parse::Invalid);
            }
            let Key::Char(obj) = obj else {
                return Err(Parse::Invalid);
            };
            return self
                .object_span(next == Key::Char('i'), obj)
                .ok_or(Parse::Invalid);
        }
        if next == Key::Char('g') && matches!(keys.get(k + 1), Some(Key::Char('n' | 'N'))) {
            if k + 2 != keys.len() {
                return Err(Parse::Invalid);
            }
            let (s, e) = self
                .match_at_or_after(keys[k + 1] == Key::Char('n'))
                .ok_or(Parse::Invalid)?;
            return Ok(Span::Chars(s, e));
        }
        let Some((motion, end)) = self.parse_motion(keys, k, total)? else {
            return Err(Parse::Invalid);
        };
        if end != keys.len() {
            return Err(Parse::Invalid);
        }
        Ok(self.motion_span(op, motion, matches!(next, Key::Char('w' | 'W'))))
    }

    /// The text a motion from the cursor covers, for operator `op`; `word`
    /// says the motion was `w` or `W`.
    fn motion_span(&self, op: char, motion: Motion, word: bool) -> Span {
        let cur = self.cursor;
        if motion.kind == MotionKind::Linewise {
            return Span::Lines(motion.to.row.min(cur.row), motion.to.row.max(cur.row));
        }
        let (mut s, mut e) = (motion.to.min(cur), motion.to.max(cur));
        let mut inclusive = motion.kind == MotionKind::Inclusive;
        // `cw` changes to the end of the word, as `ce` would.
        if op == 'c'
            && word
            && motion.to > cur
            && let Some(end) = self.end_of_word_at(cur)
        {
            e = end;
            inclusive = true;
        }
        if !inclusive && e.row > s.row && e.col == 0 {
            // An exclusive motion ending at the start of a line stops at
            // the end of the line before (`dw` on a line's last word).
            return Span::Chars(s, pos(e.row - 1, self.line_len(e.row - 1)));
        }
        if inclusive {
            e = self.after(e);
        }
        s.col = s.col.min(self.line_len(s.row));
        Span::Chars(s, e)
    }

    /// The columns a span covers on each row: `(row, from, to)`.
    fn span_cells(&self, span: Span) -> Vec<(usize, usize, usize)> {
        match span {
            Span::Lines(a, b) => (a..=b).map(|r| (r, 0, self.line_len(r))).collect(),
            Span::Chars(s, e) => (s.row..=e.row)
                .map(|r| {
                    let len = self.line_len(r);
                    let from = if r == s.row { s.col.min(len) } else { 0 };
                    let to = if r == e.row { e.col.min(len) } else { len };
                    (r, from, to.max(from))
                })
                .collect(),
        }
    }

    /// The text of the cells `(row, from, to)`, rows joined with newlines.
    pub(super) fn cells_text(&self, cells: &[(usize, usize, usize)]) -> String {
        cells
            .iter()
            .map(|&(r, a, b)| self.lines[r][a..b].iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The text of a span, as a register holds it.
    pub(super) fn span_text(&self, span: Span) -> Register {
        match span {
            Span::Lines(a, b) => Register {
                text: self.lines_text(a, b),
                linewise: true,
            },
            Span::Chars(..) => Register {
                text: self.cells_text(&self.span_cells(span)),
                linewise: false,
            },
        }
    }

    /// Takes a span out of the buffer; the cursor goes to where it was.
    pub(super) fn delete_span(&mut self, span: Span) {
        match span {
            Span::Lines(a, b) => {
                self.lines.drain(a..=b);
                if self.lines.is_empty() {
                    self.lines.push(Vec::new());
                }
                let row = a.min(self.lines.len() - 1);
                self.cursor = self.line_start(row);
            }
            Span::Chars(s, e) => {
                if s.row == e.row {
                    let len = self.line_len(s.row);
                    self.lines[s.row].drain(s.col.min(len)..e.col.min(len));
                } else {
                    let tail: Vec<char> = self.lines[e.row][e.col.min(self.line_len(e.row))..].to_vec();
                    self.lines[s.row].truncate(s.col);
                    self.lines[s.row].extend(tail);
                    self.lines.drain(s.row + 1..=e.row);
                }
                self.cursor = s;
            }
        }
    }

    /// Runs operator `op` on a span.
    pub(super) fn operate(&mut self, op: char, span: Span) -> Outcome {
        if let Span::Chars(s, e) = span
            && e <= s
            && op != 'c'
        {
            return Outcome::Bell(String::new());
        }
        match op {
            '>' | '<' => self.shift_span(span, op == '>'),
            'u' | 'U' | '~' => self.recase_span(span, op),
            'y' => self.yank_span(span),
            'd' => self.delete_text(span),
            _ => self.change_span(span),
        }
        Outcome::Ok
    }

    /// `>` / `<`: shifts the lines the span touches.
    fn shift_span(&mut self, span: Span, right: bool) {
        let (a, b) = match span {
            Span::Lines(a, b) => (a, b),
            // A span ending at the start of a line leaves that line alone.
            Span::Chars(s, e) if e.col == 0 && e.row > s.row => (s.row, e.row - 1),
            Span::Chars(s, e) => (s.row, e.row),
        };
        self.snapshot();
        self.shift_lines(a, b, right);
        self.cursor = self.line_start(a);
        self.want = self.cursor.col;
        self.changed();
    }

    /// `gu`, `gU`, `g~`: changes the case of the span.
    fn recase_span(&mut self, span: Span, how: char) {
        self.snapshot();
        let cells = self.span_cells(span);
        self.recase(&cells, how);
        let at = match span {
            Span::Lines(a, _) => pos(a, self.cursor.col),
            Span::Chars(s, _) => s,
        };
        self.set_cursor(at);
        self.changed();
    }

    /// `y`: copies the span; the cursor goes to its start.
    fn yank_span(&mut self, span: Span) {
        let text = self.span_text(span);
        self.store(text, true);
        self.cursor = match span {
            Span::Lines(a, _) => pos(a, self.cursor.col.min(self.last_col(a))).min(self.cursor),
            Span::Chars(s, _) => s,
        };
        if let Span::Lines(a, _) = span
            && self.cursor.row != a
        {
            self.cursor = pos(a, 0);
        }
        self.recording = None;
    }

    /// `d`: deletes the span into a register.
    fn delete_text(&mut self, span: Span) {
        self.snapshot();
        let text = self.span_text(span);
        self.store(text, false);
        self.delete_span(span);
        self.changed();
        self.mode = Mode::Normal;
        self.clamp();
        self.want = self.cursor.col;
    }

    /// `c`: deletes the span (lines become one empty line) and inserts.
    fn change_span(&mut self, span: Span) {
        self.snapshot();
        let text = self.span_text(span);
        self.store(text, false);
        match span {
            Span::Lines(a, b) => {
                self.lines.drain(a + 1..=b);
                self.lines[a].clear();
                self.cursor = pos(a, 0);
            }
            Span::Chars(..) => self.delete_span(span),
        }
        self.changed();
        self.mode = Mode::Insert;
        self.clamp();
    }
}
