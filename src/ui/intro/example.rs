//! One example, played on a loop: the keys (those typed so far lit up),
//! the text with the cursor and any selection, and the mode or command line.

use std::rc::Rc;

use nsh_core::prelude::*;

use crate::examples::{self, Frame};
use crate::ui::colors::{BLACK, GOLD, INK, LAVENDER, NIGHT, SELECT, TEXT, WHITE, hex, hexa, mode_color};
use crate::ui::draw::is_selected;
use crate::ui::motion::Ui;
use crate::ui::widgets::MONO;
use crate::vim::{self, Editor, Key, Mode, Pos, format_keys};

/// How a character of an example is drawn.
#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Plain,
    Selected,
    Match,
    Cursor,
}

/// Beats to hold an example's first frame and its last, before it loops.
const HOLD_START: usize = 1;
const HOLD_END: usize = 3;

pub(super) fn example_card(ui: Ui, text_with_cursor: &'static str, keys: &'static str) -> impl IntoElement {
    let frames = Rc::new(examples::frames(text_with_cursor, keys));
    let all = vim::parse_keys(keys);
    view()
        .w(px(220.0))
        .p(px(10.0))
        .rounded(px(10.0))
        .bg(hexa(BLACK, 0.35))
        .border_1()
        .border_color(hexa(LAVENDER, 0.14))
        .child(dynamic(move || {
            let n = frames.len();
            // Hold the start a beat, and the end a few.
            let i = (ui.beat.get() as usize % (n + HOLD_START + HOLD_END))
                .saturating_sub(HOLD_START)
                .min(n - 1);
            Some(frame_view(&frames[i], &all))
        }))
}

fn frame_view(frame: &Frame, all: &[Key]) -> View {
    let e = &frame.editor;
    let accent = mode_color(e);
    view()
        .gap_2()
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .child(
                    text(format_keys(&all[..frame.typed]))
                        .font_family(MONO)
                        .text_size(px(14.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(hex(GOLD)),
                )
                .child(
                    text(format_keys(&all[frame.typed..]))
                        .font_family(MONO)
                        .text_size(px(14.0))
                        .text_color(hexa(WHITE, 0.3)),
                ),
        )
        .child(
            view().p(px(6.0)).rounded(px(6.0)).bg(hexa(NIGHT, 0.6)).children(
                e.lines
                    .iter()
                    .enumerate()
                    .map(|(row, line)| text_row(e, row, line)),
            ),
        )
        .child(
            text(status(frame))
                .font_family(MONO)
                .text_size(px(11.5))
                .text_color(hex(accent)),
        )
}

/// How the character at `p` is drawn.
fn cell_at(e: &Editor, lit: &[(Pos, usize)], p: Pos, in_line: bool) -> Cell {
    if p == e.cursor && !matches!(e.mode, Mode::Command { .. }) {
        Cell::Cursor
    } else if in_line && is_selected(e, p) {
        Cell::Selected
    } else if lit
        .iter()
        .any(|(m, len)| m.row == p.row && p.col >= m.col && p.col < m.col + len)
    {
        Cell::Match
    } else {
        Cell::Plain
    }
}

/// A line of the example, in runs of characters drawn alike (the cursor's
/// on its own).
fn text_row(e: &Editor, row: usize, line: &[char]) -> View {
    let lit = e.search_highlights();
    let insert = matches!(e.mode, Mode::Insert | Mode::Replace);
    let accent = mode_color(e);
    let width = if row == e.cursor.row && e.cursor.col >= line.len() {
        line.len() + 1
    } else {
        line.len().max(1)
    };
    let mut cells: Vec<(String, Cell)> = Vec::new();
    for col in 0..width {
        let cell = cell_at(e, &lit, Pos { row, col }, col < line.len());
        let c = match line.get(col).copied().unwrap_or(' ') {
            ' ' => '\u{a0}',
            c => c,
        };
        match cells.last_mut() {
            Some((t, k)) if *k == cell && cell != Cell::Cursor => t.push(c),
            _ => cells.push((c.to_string(), cell)),
        }
    }
    view()
        .flex_row()
        .children(cells.into_iter().map(move |(t, cell)| {
            let dark = cell == Cell::Cursor && !insert;
            let glyph = text(t)
                .font_family(MONO)
                .text_size(px(14.0))
                .line_height(px(19.0))
                .text_color(if dark { hex(INK) } else { hex(TEXT) });
            let base = view().child(glyph);
            match cell {
                Cell::Cursor if insert => base.border_l_1().border_color(hex(accent)),
                Cell::Cursor => base.bg(hex(accent)).rounded(px(2.0)),
                Cell::Selected => base.bg(hexa(SELECT, 0.75)),
                Cell::Match => base.bg(hexa(GOLD, 0.45)),
                Cell::Plain => base,
            }
        }))
}

/// The mode, the command line, or the example's message.
fn status(frame: &Frame) -> String {
    let e = &frame.editor;
    match e.mode {
        _ if frame.message.is_some() && !matches!(e.mode, Mode::Command { .. }) => {
            frame.message.clone().unwrap_or_default()
        }
        Mode::Command { .. } => format!("{}{}", e.cmd_prefix(), e.cmdline),
        Mode::Insert => "-- INSERT --".into(),
        Mode::Replace => "-- REPLACE --".into(),
        Mode::Visual { line: false } => "-- VISUAL --".into(),
        Mode::Visual { line: true } => "-- VISUAL LINE --".into(),
        Mode::Block => "-- VISUAL BLOCK --".into(),
        Mode::Confirm => e.confirm_prompt(),
        Mode::Normal => match e.recording_macro() {
            Some(r) => format!("recording @{r}"),
            None => "\u{a0}".into(),
        },
    }
}
