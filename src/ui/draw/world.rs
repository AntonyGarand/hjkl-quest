//! The isle: the buffer's characters as tiles, with its shadow, rulers,
//! traps, search matches, and on editing isles what's still missing.

use nsh_core::prelude::*;

use super::EllipseAt;
use super::is_selected;
use super::marks::{draw_insert, draw_rulers, draw_seam};
use super::tile::{TileLook, draw_ghost, draw_match, draw_tile, draw_trap};
use crate::diff::Diff;
use crate::game::Session;
use crate::ui::colors::{BLACK, hexa};
use crate::ui::layout::{CH, CW, DEPTH, tile_origin};
use crate::ui::state::goal_lines;
use crate::vim::Pos;

pub(crate) fn draw_world(session: &Session) -> Painter {
    let e = &session.editor;
    let goal = goal_lines(session);
    let diff = goal.as_ref().map(|g| crate::diff::diff(&e.lines, g));
    let painter = draw_shadow(Painter::new(), session);
    let mut painter = draw_rulers(painter, session);
    painter = draw_tiles(painter, session, goal.is_some(), diff.as_ref());
    for &t in &session.traps {
        painter = draw_trap(painter, t, e.char_at(t));
    }
    // Search matches: 'hlsearch', 'incsearch', and the :s///c question.
    for (at, len) in e.search_highlights() {
        for col in at.col..at.col + len {
            painter = draw_match(painter, Pos { row: at.row, col });
        }
    }
    match (diff, goal) {
        (Some(diff), Some(goal)) => draw_missing(painter, session, &diff, &goal),
        _ => painter,
    }
}

/// Soft shadow of the island on the clouds below.
fn draw_shadow(mut painter: Painter, session: &Session) -> Painter {
    for (row, line) in session.editor.lines.iter().enumerate() {
        let len = line.len().max(1);
        let (x, y) = tile_origin(Pos { row, col: 0 });
        painter = painter
            .fill_color(hexa(BLACK, 0.18))
            .begin_path()
            .ellipse_at(
                x + len as f32 * CW / 2.0 + 10.0,
                y + CH + DEPTH + 22.0,
                len as f32 * CW / 2.0 + 8.0,
                7.0,
            )
            .fill();
    }
    painter
}

fn draw_tiles(mut painter: Painter, session: &Session, editing: bool, diff: Option<&Diff>) -> Painter {
    let e = &session.editor;
    for (row, line) in e.lines.iter().enumerate() {
        if line.is_empty() {
            let look = if is_selected(e, Pos { row, col: 0 }) {
                TileLook::Selected
            } else {
                TileLook::Plank
            };
            painter = draw_tile(painter, Pos { row, col: 0 }, None, look);
        }
        for (col, &ch) in line.iter().enumerate() {
            let p = Pos { row, col };
            let wrong = diff.is_some_and(|d| d.wrong[row][col]);
            painter = draw_tile(painter, p, Some(ch), look_of(session, p, ch, wrong, editing));
        }
    }
    painter
}

/// How the tile at `p`, holding `ch`, looks.
fn look_of(session: &Session, p: Pos, ch: char, wrong: bool, editing: bool) -> TileLook {
    if is_selected(&session.editor, p) {
        TileLook::Selected
    } else if session.traps.contains(&p) {
        TileLook::Trap
    } else if session.is_frozen(p.row) {
        TileLook::Ice
    } else if wrong {
        TileLook::Wrong
    } else if editing && ch == '.' {
        // In text, a period is a character to see, not a flower.
        TileLook::Amber
    } else {
        TileLook::of(ch)
    }
}

/// Lines missing at the end show as ghost rows below the isle; lines
/// missing in between as a seam where they go; text missing inside a line
/// as a slot.
fn draw_missing(mut painter: Painter, session: &Session, diff: &Diff, goal: &[Vec<char>]) -> Painter {
    let e = &session.editor;
    let mut extra = 0;
    let mut seams: Vec<(usize, Vec<String>)> = Vec::new();
    for &(before, g) in &diff.missing_lines {
        if before < e.lines.len() {
            let text: String = goal[g].iter().collect();
            match seams.last_mut() {
                Some((b, texts)) if *b == before => texts.push(text),
                _ => seams.push((before, vec![text])),
            }
            continue;
        }
        let row = e.lines.len() + extra;
        extra += 1;
        if goal[g].is_empty() {
            painter = draw_ghost(painter, Pos { row, col: 0 }, ' ');
        }
        for (col, &want) in goal[g].iter().enumerate() {
            painter = draw_ghost(painter, Pos { row, col }, want);
        }
    }
    for (before, texts) in seams {
        let shown: Vec<&str> = texts
            .iter()
            .map(|t| if t.is_empty() { "(empty line)" } else { t.as_str() })
            .collect();
        painter = draw_seam(painter, before, &e.lines, &format!("+ {}", shown.join("  ⏎ ")));
    }
    // Full text on the cursor's row; elsewhere how much, to keep busy
    // isles readable.
    for (at, text) in &diff.inserts {
        painter = draw_insert(painter, *at, text, at.row == e.cursor.row);
    }
    painter
}
