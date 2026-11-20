//! Visual block mode (Ctrl-v): the rectangle, and what `d`, `c`, `I`, `A`
//! and friends do to it.

use std::ops::RangeInclusive;

use super::{Editor, Mode, Outcome, Parse, Register, pos};

/// After a block `I`, `A` or `c`, the text typed on the first line goes to
/// the others on `<Esc>`.
#[derive(Debug, Clone)]
pub(super) struct BlockInsert {
    /// Where typing started, on the first line.
    row: usize,
    start: usize,
    /// The other lines, and the column the text goes in on them.
    rows: RangeInclusive<usize>,
    col: usize,
    /// `$A`: at the end of every line.
    to_end: bool,
    /// `A`: short lines are padded out to the column; else skipped.
    pad: bool,
}

impl Editor {
    /// The selected rectangle in visual block mode: rows, columns, and
    /// whether it runs to the end of every line (`$`).
    pub fn block_rect(&self) -> (usize, usize, usize, usize, bool) {
        let (r0, r1) = (
            self.anchor.row.min(self.cursor.row),
            self.anchor.row.max(self.cursor.row),
        );
        let (c0, c1) = (
            self.anchor.col.min(self.cursor.col),
            self.anchor.col.max(self.cursor.col),
        );
        (r0, r1, c0, c1, self.want == usize::MAX)
    }

    /// The block's columns on each row: `(row, from, to)`.
    pub(super) fn block_cells(&self) -> Vec<(usize, usize, usize)> {
        let (r0, r1, c0, c1, to_end) = self.block_rect();
        (r0..=r1)
            .map(|r| {
                let len = self.line_len(r);
                (r, c0.min(len), if to_end { len } else { (c1 + 1).min(len) })
            })
            .collect()
    }

    /// A command on the block (the ones only block mode has).
    pub(super) fn block_command(&mut self, c: char) -> Result<Outcome, Parse> {
        match c {
            'O' => std::mem::swap(&mut self.anchor.col, &mut self.cursor.col),
            'v' | 'V' => self.mode = Mode::Visual { line: c == 'V' },
            'd' | 'x' | 'y' | 'c' | 's' => self.block_cut(c),
            'I' | 'A' => self.block_append(c == 'A'),
            _ => return Err(Parse::Invalid),
        }
        Ok(Outcome::Ok)
    }

    /// `d`, `x`, `y`, `c` and `s` on the block.
    fn block_cut(&mut self, c: char) {
        let (r0, r1, c0, _, _) = self.block_rect();
        let cells = self.block_cells();
        let text = self.cells_text(&cells);
        self.store(
            Register {
                text,
                linewise: false,
            },
            c == 'y',
        );
        self.mode = Mode::Normal;
        if c != 'y' {
            self.snapshot();
            for &(r, a, b) in &cells {
                self.lines[r].drain(a..b);
            }
            self.changed();
        }
        if c == 'c' || c == 's' {
            let start = c0.min(self.line_len(r0));
            self.start_block_insert(BlockInsert {
                row: r0,
                start,
                rows: r0 + 1..=r1,
                col: c0,
                to_end: false,
                pad: false,
            });
        } else {
            self.set_cursor(pos(r0, c0));
        }
    }

    /// `I` (before the block) or `A` (after it; `$A` at every line's end).
    fn block_append(&mut self, append: bool) {
        let (r0, r1, c0, c1, to_end) = self.block_rect();
        self.snapshot();
        let to_end = append && to_end;
        let col = if append { c1 + 1 } else { c0 };
        let start = if to_end {
            self.line_len(r0)
        } else {
            if append && self.line_len(r0) < col {
                self.lines[r0].resize(col, ' ');
            }
            col.min(self.line_len(r0))
        };
        self.start_block_insert(BlockInsert {
            row: r0,
            start,
            rows: r0 + 1..=r1,
            col,
            to_end,
            pad: append,
        });
    }

    fn start_block_insert(&mut self, insert: BlockInsert) {
        self.mode = Mode::Insert;
        self.cursor = pos(insert.row, insert.start);
        self.block_insert = Some(insert);
    }

    /// On `<Esc>` after a block insert: copies what was typed on the first
    /// line to the others. Returns whether there was one to finish.
    pub(super) fn finish_block_insert(&mut self) -> bool {
        let Some(b) = self.block_insert.take() else {
            return false;
        };
        if self.cursor.row != b.row || self.cursor.col < b.start {
            return false;
        }
        let typed: Vec<char> = self.lines[b.row][b.start..self.cursor.col].to_vec();
        for row in b.rows {
            let len = self.line_len(row);
            let col = if b.to_end { len } else { b.col };
            if len < col {
                if !b.pad {
                    continue;
                }
                self.lines[row].resize(col, ' ');
            }
            self.lines[row].splice(col..col, typed.iter().copied());
        }
        self.changed();
        self.mode = Mode::Normal;
        self.set_cursor(pos(b.row, b.start));
        true
    }
}
