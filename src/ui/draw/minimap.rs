//! The whole isle in a corner: its tiles, the crystals left, the cursor
//! and the part the stage shows, which follows the camera natively.

use std::f32::consts::TAU;

use nsh_core::canvas::{CanvasTextAlign, CanvasTextBaseline};
use nsh_core::element::CanvasArg;
use nsh_core::prelude::*;

use super::is_selected;
use super::tile::TileLook;
use crate::diff::Diff;
use crate::game::Session;
use crate::ui::colors::{
    AMBER, BUG, CREAM, EMBER, GEM, GLYPH, INK, LANTERN, MISSING, SELECT, TEXT, WHITE, WOOD, hex, hexa,
    mode_color,
};
use crate::ui::layout::{CH, CW, PAD_X, PAD_Y, ROWS_ROOM, SKEW};
use crate::ui::motion::Motion;
use crate::ui::state::{goal_lines, session_diff};
use crate::vim::Pos;

/// Room around the minimap for its line and column numbers.
const MINI_GUTTER: (f32, f32) = (20.0, 12.0);
/// The most room the minimap's cells take.
const MAX_W: f32 = 280.0;
const MAX_H: f32 = 150.0;

/// The minimap's cells: how many, and how big. Big enough for letters on
/// small isles, a few pixels on large ones.
#[derive(Clone, Copy)]
struct Grid {
    rows: usize,
    cols: usize,
    cw: f32,
    ch: f32,
}

impl Grid {
    fn of(session: &Session) -> Grid {
        let e = &session.editor;
        let goal_rows = session.level().goal_text().map_or(0, |g| g.lines().count());
        let rows = e.lines.len().max(goal_rows);
        let cols = e.lines.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let rows_f = rows.max(1) as f32;
        let cw = (MAX_W / cols as f32).min(MAX_H / rows_f).clamp(2.0, 11.0);
        let ch = (cw * 1.4).max(11.0).min((MAX_H / rows_f).max(3.0));
        Grid { rows, cols, cw, ch }
    }

    /// Whether the cells are big enough to print their letters.
    fn letters(self) -> bool {
        self.cw >= 6.0 && self.ch >= 8.0
    }

    /// The centre of the cell at `p`.
    fn center(self, p: Pos) -> (f32, f32) {
        (
            p.col as f32 * self.cw + self.cw / 2.0,
            p.row as f32 * self.ch + self.ch / 2.0,
        )
    }

    fn size(self) -> (f32, f32) {
        (self.cols as f32 * self.cw, self.rows as f32 * self.ch)
    }
}

/// The minimap's size, gutter included.
pub(crate) fn minimap_size(session: &Session) -> (f32, f32) {
    let (w, h) = Grid::of(session).size();
    (w + MINI_GUTTER.0, h + MINI_GUTTER.1)
}

pub(crate) fn draw_minimap(session: &Session, motion: &Motion, stage: (f32, f32)) -> Painter {
    let e = &session.editor;
    let grid = Grid::of(session);
    let font = format!("700 {}px monospace", (grid.ch * 0.78).min(grid.cw * 1.45).floor());
    let painter = draw_minimap_rulers(Painter::new(), e.cursor, grid.rows.min(e.lines.len()), grid)
        .save()
        .translate(MINI_GUTTER.0, MINI_GUTTER.1)
        .font(&font)
        .text_align(CanvasTextAlign::Center)
        .text_baseline(CanvasTextBaseline::Middle);
    let mut painter = draw_cells(painter, session, grid);
    if let (Some(diff), Some(goal)) = (session_diff(session), goal_lines(session)) {
        painter = draw_missing(painter, session, grid, &diff, &goal);
    }
    painter = draw_dots(painter, session, grid);
    painter = draw_cursor(painter, session, grid);
    draw_view_box(painter, grid, motion, stage).restore()
}

/// The buffer's characters, coloured as their tiles.
fn draw_cells(mut painter: Painter, session: &Session, grid: Grid) -> Painter {
    let e = &session.editor;
    let Grid { cw, ch, .. } = grid;
    let editing = session.level().goal_text().is_some();
    let diff = session_diff(session);
    for (row, line) in e.lines.iter().enumerate() {
        let y = row as f32 * ch;
        if line.is_empty() {
            painter = painter
                .fill_color(hexa(WOOD, 0.9))
                .fill_rect(0.0, y + 1.0, cw - 0.5, ch - 2.0);
        }
        for (col, &c) in line.iter().enumerate() {
            let p = Pos { row, col };
            let color = if is_selected(e, p) {
                SELECT
            } else if diff.as_ref().is_some_and(|d| d.wrong[row][col]) {
                EMBER
            } else if editing && c == '.' {
                AMBER
            } else {
                TileLook::of(c).color()
            };
            painter = painter
                .fill_color(hex(color))
                .fill_rect(col as f32 * cw, y, cw - 0.5, ch - 0.5);
            if grid.letters() && !c.is_whitespace() && (editing || c != '.') {
                let (x, y) = grid.center(p);
                painter = painter
                    .fill_color(hexa(GLYPH, 0.95))
                    .fill_text(&c.to_string(), x, y + 0.5);
            }
        }
    }
    painter
}

/// What's missing: ghost rows at the end, slots where text goes.
fn draw_missing(
    mut painter: Painter,
    session: &Session,
    grid: Grid,
    diff: &Diff,
    goal: &[Vec<char>],
) -> Painter {
    let lines = session.editor.lines.len();
    let Grid { cols, cw, ch, .. } = grid;
    let mut extra = 0;
    for &(before, g) in &diff.missing_lines {
        if before < lines {
            painter =
                painter
                    .fill_color(hex(MISSING))
                    .fill_rect(0.0, before as f32 * ch - 1.0, grid.size().0, 2.0);
            continue;
        }
        let y = (lines + extra) as f32 * ch;
        extra += 1;
        for (col, &c) in goal[g].iter().enumerate().take(cols) {
            let x = col as f32 * cw;
            painter = painter
                .stroke_color(hexa(MISSING, 0.6))
                .line_width(1.0)
                .begin_path()
                .move_to(x + 0.5, y + 0.5)
                .line_to(x + cw - 1.0, y + 0.5)
                .line_to(x + cw - 1.0, y + ch - 1.0)
                .line_to(x + 0.5, y + ch - 1.0)
                .close_path()
                .stroke();
            if grid.letters() && !c.is_whitespace() {
                painter = painter.fill_color(hexa(MISSING, 0.8)).fill_text(
                    &c.to_string(),
                    x + cw / 2.0,
                    y + ch / 2.0 + 0.5,
                );
            }
        }
    }
    for (at, _) in &diff.inserts {
        painter = painter.fill_color(hex(MISSING)).fill_rect(
            at.col as f32 * cw - 1.0,
            at.row as f32 * ch - 1.0,
            2.5,
            ch + 1.0,
        );
    }
    painter
}

/// Traps as red triangles; crystals, bugs and sentries as dots.
fn draw_dots(mut painter: Painter, session: &Session, grid: Grid) -> Painter {
    let Grid { cw, ch, .. } = grid;
    for &t in &session.traps {
        let (x, y) = (t.col as f32 * cw, t.row as f32 * ch);
        painter = painter
            .fill_color(hex(0xff4d4d))
            .begin_path()
            .move_to(x + cw / 2.0, y + 1.0)
            .line_to(x + cw - 0.5, y + ch - 1.0)
            .line_to(x + 0.5, y + ch - 1.0)
            .close_path()
            .fill();
    }
    let dot = |painter: Painter, p: Pos, color: u32, r: f32| {
        let (x, y) = grid.center(p);
        painter
            .fill_color(hex(color))
            .begin_path()
            .arc(x, y, r, 0.0, TAU)
            .fill()
    };
    for &(p, got) in &session.gems {
        if !got {
            painter = dot(painter, p, GEM, (cw * 0.55).max(2.0));
        }
    }
    for bug in &session.bugs {
        painter = dot(painter, bug.at, BUG, (cw * 0.6).max(2.5));
    }
    for s in &session.sentries {
        painter = dot(painter, s.at, LANTERN, (cw * 0.6).max(2.5));
    }
    painter
}

/// The cursor's cell, in its mode's colour, with its letter inverted.
fn draw_cursor(painter: Painter, session: &Session, grid: Grid) -> Painter {
    let e = &session.editor;
    let cur = e.cursor;
    let Grid { cw, ch, .. } = grid;
    let painter = painter
        .fill_color(hex(mode_color(e)))
        .stroke_color(hex(WHITE))
        .line_width(1.0)
        .fill_rect(
            cur.col as f32 * cw - 1.0,
            cur.row as f32 * ch - 1.0,
            cw + 1.5,
            ch + 1.5,
        );
    match e.char_at(cur).filter(|c| grid.letters() && !c.is_whitespace()) {
        Some(c) => {
            let (x, y) = grid.center(cur);
            painter.fill_color(hex(INK)).fill_text(&c.to_string(), x, y + 0.5)
        }
        None => painter,
    }
}

/// The stage's window onto the board, mapped back to rows and columns.
fn draw_view_box(painter: Painter, grid: Grid, motion: &Motion, stage: (f32, f32)) -> Painter {
    let Grid { rows, cw, ch, .. } = grid;
    let (mw, mh) = grid.size();
    let (vw, vh) = stage;
    let x0 = PAD_X + (ROWS_ROOM - rows as f32 / 2.0) * SKEW;
    let vx = motion.cam_x.mul(-cw / CW).add(-x0 * cw / CW);
    let vy = motion.cam_y.mul(-ch / CH).add(-PAD_Y * ch / CH);
    let (w, h) = (vw / CW * cw, vh / CH * ch);
    // On an axis where the whole isle fits, the box spans the minimap.
    let (fit_x, fit_y) = (w >= mw, h >= mh);
    if fit_x && fit_y {
        return painter;
    }
    let x: CanvasArg = if fit_x { 0.0.into() } else { (&vx).into() };
    let y: CanvasArg = if fit_y { 0.0.into() } else { (&vy).into() };
    let (w, h) = (if fit_x { mw } else { w }, if fit_y { mh } else { h });
    painter
        .save()
        .begin_path()
        .move_to(-2.0, -2.0)
        .line_to(mw + 2.0, -2.0)
        .line_to(mw + 2.0, mh + 2.0)
        .line_to(-2.0, mh + 2.0)
        .close_path()
        .clip()
        .translate(x, y)
        .fill_color(hexa(CREAM, 0.12))
        .stroke_color(hexa(CREAM, 0.9))
        .line_width(1.5)
        .begin_path()
        .move_to(0.0, 0.0)
        .line_to(w, 0.0)
        .line_to(w, h)
        .line_to(0.0, h)
        .close_path()
        .fill()
        .stroke()
        .restore()
}

/// The minimap's line numbers down the left and column numbers along the
/// top: every 5th (or 10th, when cells are small), and the cursor's own.
fn draw_minimap_rulers(painter: Painter, cur: Pos, rows: usize, grid: Grid) -> Painter {
    let Grid { cols, cw, ch, .. } = grid;
    let (gx, gy) = MINI_GUTTER;
    let color = |here: bool| if here { hex(CREAM) } else { hexa(TEXT, 0.5) };
    let mut painter = painter
        .font("600 9px monospace")
        .text_baseline(CanvasTextBaseline::Middle)
        .text_align(CanvasTextAlign::Right);
    let row_step = if ch >= 9.0 { 5 } else { 10 };
    for row in 0..rows {
        let n = row + 1;
        let here = row == cur.row;
        // Numbers closer than a label's height to the cursor's would overlap it.
        let near = row.abs_diff(cur.row) as f32 * ch < 9.0 && !here;
        if (here || n == 1 || n % row_step == 0) && !near {
            painter = painter.fill_color(color(here)).fill_text(
                &n.to_string(),
                gx - 4.0,
                gy + row as f32 * ch + ch / 2.0,
            );
        }
    }
    let col_step = if cw >= 7.0 { 5 } else { 10 };
    painter = painter.text_align(CanvasTextAlign::Center);
    for col in 0..cols {
        let n = col + 1;
        let here = col == cur.col;
        let width = n.to_string().len() as f32 * 6.0;
        let near = (col.abs_diff(cur.col) as f32 * cw) < width && !here;
        if (here || n == 1 || n % col_step == 0) && !near {
            painter = painter.fill_color(color(here)).fill_text(
                &n.to_string(),
                gx + col as f32 * cw + cw / 2.0,
                gy / 2.0,
            );
        }
    }
    painter
}
