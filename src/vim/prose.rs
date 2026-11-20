//! Sentences and paragraphs: the `(`, `)`, `{`, `}` motions and the `is`,
//! `as`, `ip`, `ap` objects.

use super::objects::skip_blanks;
use super::{Editor, Pos, pos};

/// Whether the character at `col` ends a sentence: a `.`, `!` or `?`,
/// any closing `)]"'`, then a blank or the line's end.
fn ends_sentence(line: &[char], col: usize) -> bool {
    if !matches!(line[col], '.' | '!' | '?') {
        return false;
    }
    let mut k = col + 1;
    while k < line.len() && matches!(line[k], ')' | ']' | '"' | '\'') {
        k += 1;
    }
    k == line.len() || line[k].is_whitespace()
}

impl Editor {
    /// Where each sentence starts: after the end of one, at an empty line,
    /// and at the first character after an empty line.
    fn sentence_starts(&self) -> Vec<Pos> {
        let mut starts = Vec::new();
        let mut need = true;
        for (row, line) in self.lines.iter().enumerate() {
            if line.is_empty() {
                starts.push(pos(row, 0));
                need = true;
                continue;
            }
            for (col, &c) in line.iter().enumerate() {
                if need && !c.is_whitespace() {
                    starts.push(pos(row, col));
                    need = false;
                }
                if ends_sentence(line, col) {
                    need = true;
                }
            }
        }
        starts
    }

    /// `)` (forward) or `(`: `n` sentence starts away.
    pub(super) fn sentence_from(&self, forward: bool, n: usize) -> Pos {
        let starts = self.sentence_starts();
        let mut p = self.cursor;
        for _ in 0..n {
            p = if forward {
                starts
                    .iter()
                    .find(|&&s| s > p)
                    .copied()
                    .unwrap_or_else(|| self.last_char())
            } else {
                starts
                    .iter()
                    .rev()
                    .find(|&&s| s < p)
                    .copied()
                    .unwrap_or(pos(0, 0))
            };
        }
        p
    }

    /// The sentence object: `is` the sentence, `as` with the spaces after
    /// it (or, at a line's end, before it). It ends on its own line.
    pub(super) fn sentence_object(&self, inner: bool) -> Option<(Pos, Pos)> {
        let cur = self.cursor;
        if self.lines[cur.row].is_empty() {
            return None;
        }
        let starts = self.sentence_starts();
        let s = starts
            .iter()
            .rev()
            .find(|&&s| s <= cur)
            .copied()
            .unwrap_or(pos(0, 0));
        let mut end = match starts.iter().find(|&&p| p > cur) {
            Some(&n) if n.row == cur.row => n,
            _ => pos(cur.row, self.line_len(cur.row)),
        };
        let line = &self.lines[end.row];
        let floor = if s.row == end.row { s.col + 1 } else { 0 };
        while end.col > floor && line[end.col - 1].is_whitespace() {
            end.col -= 1;
        }
        if inner {
            return Some((s, end));
        }
        let e = skip_blanks(line, end.col);
        if e > end.col {
            return Some((s, pos(end.row, e)));
        }
        let first = &self.lines[s.row];
        let mut b = s.col;
        while b > 0 && first[b - 1].is_whitespace() {
            b -= 1;
        }
        Some((pos(s.row, b), end))
    }

    /// The row a `}` (down) or `{` from `row` stops at: past the blank
    /// lines we are on, then past the paragraph.
    fn paragraph(&self, row: usize, down: bool) -> usize {
        let mut r = row;
        let last = self.lines.len() - 1;
        if down {
            while r < last && self.lines[r].is_empty() {
                r += 1;
            }
            while r < last && !self.lines[r].is_empty() {
                r += 1;
            }
        } else {
            while r > 0 && self.lines[r].is_empty() {
                r -= 1;
            }
            while r > 0 && !self.lines[r].is_empty() {
                r -= 1;
            }
        }
        r
    }

    /// `}` (forward) or `{`, `n` times: the blank line after (or before)
    /// the paragraph, or the buffer's end (or start).
    pub(super) fn paragraph_from(&self, forward: bool, n: usize) -> Pos {
        let mut row = self.cursor.row;
        for _ in 0..n {
            row = self.paragraph(row, forward);
        }
        if forward && row == self.lines.len() - 1 && !self.lines[row].is_empty() {
            pos(row, self.last_col(row))
        } else {
            pos(row, 0)
        }
    }

    /// `ip` / `ap`: the paragraph (or run of blank lines) around the
    /// cursor; `ap` takes the blank lines after it too (or, at the end,
    /// before it).
    pub(super) fn paragraph_object(&self, inner: bool) -> (usize, usize) {
        let blank = |r: usize| self.lines[r].iter().all(|c| c.is_whitespace());
        let row = self.cursor.row;
        let on_blank = blank(row);
        let (mut a, mut b) = (row, row);
        while a > 0 && blank(a - 1) == on_blank {
            a -= 1;
        }
        while b + 1 < self.lines.len() && blank(b + 1) == on_blank {
            b += 1;
        }
        if inner {
            return (a, b);
        }
        let mut f = b;
        while f + 1 < self.lines.len() && blank(f + 1) != on_blank {
            f += 1;
            if !on_blank && !blank(f) {
                f -= 1;
                break;
            }
        }
        if f > b {
            b = f;
        } else if !on_blank {
            while a > 0 && blank(a - 1) {
                a -= 1;
            }
        }
        (a, b)
    }
}
