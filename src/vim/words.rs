//! Words: the character classes vim splits them by, and the `w`, `b`,
//! `e`, `ge` steps between them.

use super::{Editor, Pos, pos};

/// A character's class: 0 blank, 1 punctuation, 2 word characters. With
/// `big` (WORDs), everything but blanks is one class.
pub(super) fn class(c: char, big: bool) -> u8 {
    if c.is_whitespace() {
        0
    } else if big || c.is_alphanumeric() || c == '_' {
        2
    } else {
        1
    }
}

impl Editor {
    /// The class of the character at `p` (0 past the end of a line).
    fn cls_at(&self, p: Pos, big: bool) -> u8 {
        self.char_at(p).map_or(0, |c| class(c, big))
    }

    fn empty_line(&self, row: usize) -> bool {
        self.lines[row].is_empty()
    }

    /// `w`: the start of the next word (an empty line counts as one).
    pub(super) fn word_forward(&self, start: Pos, big: bool) -> Pos {
        let mut p = start;
        let cls = self.cls_at(p, big);
        if cls != 0 {
            // Off the word the cursor is on.
            loop {
                let Some((q, nl)) = self.next(p) else {
                    return self.past_end();
                };
                p = q;
                if nl || self.cls_at(q, big) != cls {
                    break;
                }
            }
        }
        // Over blanks, to the next word or empty line.
        loop {
            if p != start && (self.empty_line(p.row) || self.cls_at(p, big) != 0) {
                return p;
            }
            let Some((q, _)) = self.next(p) else {
                return self.past_end();
            };
            p = q;
        }
    }

    /// `e`: the end of the word (the next one, when already at an end).
    pub(super) fn word_end(&self, start: Pos, big: bool) -> Pos {
        let Some((mut p, _)) = self.next(start) else {
            return start;
        };
        while self.cls_at(p, big) == 0 {
            let Some((q, _)) = self.next(p) else {
                return p;
            };
            p = q;
        }
        let cls = self.cls_at(p, big);
        while let Some((q, nl)) = self.next(p) {
            if nl || self.cls_at(q, big) != cls {
                break;
            }
            p = q;
        }
        p
    }

    /// `b`: the start of the word (the one before, when already at a start).
    pub(super) fn word_backward(&self, start: Pos, big: bool) -> Pos {
        let Some((mut p, _)) = self.prev(start) else {
            return start;
        };
        while self.cls_at(p, big) == 0 {
            if self.empty_line(p.row) {
                return pos(p.row, 0);
            }
            let Some((q, _)) = self.prev(p) else {
                return p;
            };
            p = q;
        }
        let cls = self.cls_at(p, big);
        while let Some((q, nl)) = self.prev(p) {
            if nl || self.cls_at(q, big) != cls {
                break;
            }
            p = q;
        }
        p
    }

    /// `ge`: the end of the word before.
    pub(super) fn word_end_backward(&self, start: Pos, big: bool) -> Pos {
        let mut p = start;
        let cls = self.cls_at(p, big);
        if cls != 0 {
            // Off the word the cursor is on.
            loop {
                let Some((q, nl)) = self.prev(p) else {
                    return pos(0, 0);
                };
                p = q;
                if nl || self.cls_at(q, big) != cls {
                    break;
                }
            }
        }
        while self.cls_at(p, big) == 0 && !self.empty_line(p.row) {
            let Some((q, _)) = self.prev(p) else {
                return p;
            };
            p = q;
        }
        p
    }

    /// The last character of the word at `p` on its line, as `cw` sees it
    /// (`None` on a blank).
    pub(super) fn end_of_word_at(&self, p: Pos) -> Option<Pos> {
        let cls = self
            .char_at(p)
            .filter(|c| !c.is_whitespace())
            .map(|c| class(c, false))?;
        let mut end = p;
        while self.cls_at(pos(end.row, end.col + 1), false) == cls {
            end.col += 1;
        }
        Some(end)
    }

    /// The keyword under or after `p` on its line, for `*` and `#`.
    pub(super) fn word_under(&self, p: Pos) -> Option<String> {
        let line = &self.lines[p.row];
        let is_word = |c: &char| class(*c, false) == 2;
        let col = p.col + line.get(p.col..)?.iter().position(is_word)?;
        let start = line[..col].iter().rposition(|c| !is_word(c)).map_or(0, |k| k + 1);
        let end = col
            + line[col..]
                .iter()
                .position(|c| !is_word(c))
                .unwrap_or(line.len() - col);
        Some(line[start..end].iter().collect())
    }
}
