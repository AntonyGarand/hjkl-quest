//! Marks around the tiles: line and column numbers, and the slots and
//! seams where missing text goes.

use std::f32::consts::TAU;

use nsh_core::canvas::{CanvasTextAlign, CanvasTextBaseline, LineCap};
use nsh_core::prelude::*;

use crate::game::Session;
use crate::ui::colors::{CREAM, MISSING, TEXT, hex, hexa, mode_color};
use crate::ui::layout::{CH, CW, SKEW, tile_origin};
use crate::vim::Pos;

/// A pill's width per character of its label, and its padding.
const PILL_CHAR: f32 = 7.4;
const PILL_PAD: f32 = 12.0;

fn pill_width(label: &str) -> f32 {
    label.chars().count() as f32 * PILL_CHAR + PILL_PAD
}

/// A small label in a rounded pill, its left edge at `x`, centred on `y`.
fn draw_pill(painter: Painter, x: f32, y: f32, label: &str, color: u32) -> Painter {
    painter
        .fill_color(hexa(0x1a0f05, 0.88))
        .stroke_color(hexa(color, 0.95))
        .line_width(1.2)
        .begin_path()
        .round_rect(
            Bounds {
                origin: point(px(x), px(y - 9.0)),
                size: size(px(pill_width(label)), px(18.0)),
            },
            px(9.0),
        )
        .fill()
        .stroke()
        .font("700 12px monospace")
        .text_align(CanvasTextAlign::Left)
        .text_baseline(CanvasTextBaseline::Middle)
        .fill_color(hex(color))
        .fill_text(label, x + 6.0, y + 0.5)
}

/// Missing text inside a line: a glowing slot where it goes, and what.
pub(super) fn draw_insert(painter: Painter, at: Pos, text: &str, full: bool) -> Painter {
    let (x, y) = tile_origin(at);
    let label = if full {
        format!("+{}", text.replace(' ', "␣"))
    } else {
        format!("+{}", text.chars().count())
    };
    let painter = painter
        .stroke_color(hexa(MISSING, 0.35))
        .line_width(7.0)
        .line_cap(LineCap::Round)
        .begin_path()
        .move_to(x + SKEW, y + 2.0)
        .line_to(x + 1.0, y + CH - 2.0)
        .stroke()
        .stroke_color(hex(MISSING))
        .line_width(2.5)
        .begin_path()
        .move_to(x + SKEW, y + 2.0)
        .line_to(x + 1.0, y + CH - 2.0)
        .stroke();
    let w = pill_width(&label);
    draw_pill(painter, x + SKEW - w / 2.0, y - 11.0, &label, MISSING)
}

/// Whole lines missing before buffer row `before`: a dashed seam between
/// the rows, and their text.
pub(super) fn draw_seam(painter: Painter, before: usize, lines: &[Vec<char>], label: &str) -> Painter {
    let width = lines[before.saturating_sub(1)..=before.min(lines.len() - 1)]
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(1)
        .max(1);
    let (x, y) = tile_origin(Pos { row: before, col: 0 });
    let painter = painter
        .stroke_color(hex(MISSING))
        .line_width(2.5)
        .line_dash(&[px(6.0), px(4.0)])
        .begin_path()
        .move_to(x + SKEW, y)
        .line_to(x + SKEW + width as f32 * CW, y)
        .stroke()
        .line_dash(&[]);
    draw_pill(painter, x + SKEW + width as f32 * CW + 8.0, y, label, MISSING)
}

/// Line numbers down the left (absolute, and how far each line is from
/// the cursor, for counts like 5j) and column numbers along the top.
pub(super) fn draw_rulers(painter: Painter, session: &Session) -> Painter {
    let painter = draw_line_numbers(painter.text_baseline(CanvasTextBaseline::Middle), session);
    draw_column_numbers(painter, session)
}

fn draw_line_numbers(mut painter: Painter, session: &Session) -> Painter {
    let e = &session.editor;
    let cur = e.cursor;
    let accent = mode_color(e);
    for row in 0..e.lines.len() {
        let (x, y) = tile_origin(Pos { row, col: 0 });
        let (gx, gy) = (x + SKEW / 2.0 - 8.0, y + CH / 2.0 + 0.5);
        let here = row == cur.row;
        painter = painter
            .font(if here {
                "700 13px monospace"
            } else {
                "500 12px monospace"
            })
            .text_align(CanvasTextAlign::Right)
            .fill_color(if here { hex(accent) } else { hexa(TEXT, 0.38) })
            .fill_text(&(row + 1).to_string(), gx - 22.0, gy);
        if !here {
            let arrow = if row < cur.row { '↑' } else { '↓' };
            painter = painter
                .font("700 11px monospace")
                .fill_color(hexa(CREAM, 0.7))
                .fill_text(&format!("{arrow}{}", row.abs_diff(cur.row)), gx, gy);
        }
    }
    painter
}

/// Columns along the top edge of the first row: a dot each, a number
/// every 5th and on the cursor's.
fn draw_column_numbers(mut painter: Painter, session: &Session) -> Painter {
    let e = &session.editor;
    let cur = e.cursor;
    let accent = mode_color(e);
    let cols = e.lines.iter().map(Vec::len).max().unwrap_or(0);
    let (_, top) = tile_origin(Pos { row: 0, col: 0 });
    let y = top - 30.0;
    for col in 0..cols {
        let (x, _) = tile_origin(Pos { row: 0, col });
        let cx = x + SKEW + CW / 2.0;
        let n = col + 1;
        let here = col == cur.col;
        let major = n == 1 || n % 5 == 0;
        let tenth = n % 10 == 0;
        painter = painter
            .fill_color(hexa(TEXT, if tenth { 0.5 } else { 0.22 }))
            .begin_path()
            .arc(cx, y + 11.0, if major { 1.8 } else { 1.1 }, 0.0, TAU)
            .fill();
        if major || here {
            painter = painter
                .font(if here {
                    "700 12px monospace"
                } else {
                    "500 11px monospace"
                })
                .text_align(CanvasTextAlign::Center)
                .fill_color(if here {
                    hex(accent)
                } else {
                    hexa(TEXT, if tenth { 0.6 } else { 0.38 })
                })
                .fill_text(&n.to_string(), cx, y);
        }
    }
    painter
}
