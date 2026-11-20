//! The buffer as lines of characters: their text and lengths, the
//! positions next to a position, and keeping the cursor on the text.

use super::{Editor, Mode, Pos, pos};

/// Splits text into lines of characters (there is always one).
pub(super) fn to_lines(text: &str) -> Vec<Vec<char>> {
    text.split('\n').map(|l| l.chars().collect()).collect()
}

impl Editor {
    /// The whole buffer, lines joined with newlines.
    pub fn text(&self) -> String {
        self.lines_text(0, self.lines.len() - 1)
    }

    /// Rows `a..=b`, joined with newlines.
    pub(super) fn lines_text(&self, a: usize, b: usize) -> String {
        self.lines[a..=b]
            .iter()
            .map(|l| l.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// How many characters `row` has (0 past the end of the buffer).
    pub fn line_len(&self, row: usize) -> usize {
        self.lines.get(row).map_or(0, Vec::len)
    }

    /// The character at `p`, if there is one.
    pub fn char_at(&self, p: Pos) -> Option<char> {
        self.lines.get(p.row).and_then(|l| l.get(p.col)).copied()
    }

    /// The column of the last character of `row` (0 on an empty line).
    pub(super) fn last_col(&self, row: usize) -> usize {
        self.line_len(row).saturating_sub(1)
    }

    /// The first non-blank column of `row`.
    pub(super) fn first_non_blank(&self, row: usize) -> usize {
        let line = &self.lines[row];
        line.iter()
            .position(|c| !c.is_whitespace())
            .unwrap_or(line.len().saturating_sub(1))
    }

    /// The first non-blank character of `row`.
    pub(super) fn line_start(&self, row: usize) -> Pos {
        pos(row, self.first_non_blank(row))
    }

    /// Keeps the cursor on the text: on a character in normal mode, or
    /// just past the line's end in insert and replace mode.
    pub(super) fn clamp(&mut self) {
        let last_row = self.lines.len() - 1;
        self.cursor.row = self.cursor.row.min(last_row);
        let max = match self.mode {
            Mode::Insert | Mode::Replace => self.line_len(self.cursor.row),
            _ => self.last_col(self.cursor.row),
        };
        self.cursor.col = self.cursor.col.min(max);
    }

    /// The position after `p` in the buffer (the end of its line when `p` is
    /// the last character).
    pub(super) fn after(&self, p: Pos) -> Pos {
        pos(p.row, (p.col + 1).min(self.line_len(p.row)))
    }

    /// The position before `p`: the previous character, or the last one
    /// of the line above.
    pub(super) fn before(&self, p: Pos) -> Pos {
        if p.col > 0 {
            pos(p.row, p.col - 1)
        } else if p.row > 0 {
            pos(p.row - 1, self.last_col(p.row - 1))
        } else {
            p
        }
    }

    /// The next position, and whether it crossed a line.
    pub(super) fn next(&self, p: Pos) -> Option<(Pos, bool)> {
        if p.col + 1 < self.line_len(p.row) {
            Some((pos(p.row, p.col + 1), false))
        } else if p.row + 1 < self.lines.len() {
            Some((pos(p.row + 1, 0), true))
        } else {
            None
        }
    }

    /// The previous position, and whether it crossed a line.
    pub(super) fn prev(&self, p: Pos) -> Option<(Pos, bool)> {
        if p.col > 0 {
            Some((pos(p.row, p.col - 1), false))
        } else if p.row > 0 {
            Some((pos(p.row - 1, self.last_col(p.row - 1)), true))
        } else {
            None
        }
    }

    /// Just past the buffer's last character: where `w` stops when no word
    /// follows (the cursor clamps back onto the last character).
    pub(super) fn past_end(&self) -> Pos {
        let row = self.lines.len() - 1;
        pos(row, self.line_len(row))
    }

    /// The buffer's last character.
    pub(super) fn last_char(&self) -> Pos {
        let row = self.lines.len() - 1;
        pos(row, self.last_col(row))
    }

    /// Breaks the line at the cursor; the cursor goes to the new line.
    pub(super) fn split_line(&mut self) {
        let row = self.cursor.row;
        let rest = self.lines[row].split_off(self.cursor.col);
        self.lines.insert(row + 1, rest);
        self.cursor = pos(row + 1, 0);
    }

    /// Counts a change to the buffer, for the UI to redraw.
    pub(super) fn changed(&mut self) {
        self.revision += 1;
    }
}
