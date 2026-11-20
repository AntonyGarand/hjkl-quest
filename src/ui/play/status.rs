//! Vim's status line along the bottom of the stage: the mode, the command
//! line or the latest message, the keys pending, and the cursor's place.

use nsh_core::prelude::*;

use crate::game::Session;
use crate::ui::colors::{NIGHT, ORANGE, WHITE, hex, hexa, mode_color};
use crate::ui::motion::Ui;
use crate::ui::state::{current_toast, session_value};
use crate::ui::widgets::MONO;
use crate::vim::Mode;

pub(super) fn status_line(ui: Ui) -> impl IntoElement {
    view()
        .absolute()
        .left_0()
        .right_0()
        .bottom_0()
        .flex_row()
        .items_center()
        .gap_3()
        .px(px(14.0))
        .py(px(8.0))
        .bg(hexa(NIGHT, 0.72))
        .child(
            text(move || {
                ui.tick.get();
                session_value(|s| mode_name(s.editor.mode).to_string()).unwrap_or_default()
            })
            .font_family(MONO)
            .font_weight(FontWeight::BOLD)
            .text_size(px(13.0))
            .text_color(move || {
                ui.tick.get();
                hex(session_value(|s| mode_color(&s.editor)).unwrap_or(WHITE))
            })
            .w(px(150.0)),
        )
        .child(
            text(move || {
                ui.tick.get();
                // Read before the session: both live in the state.
                let toast = current_toast();
                session_value(|s| message(s, toast)).unwrap_or_default()
            })
            .font_family(MONO)
            .text_size(px(14.0))
            .flex_1()
            .number_of_lines(1),
        )
        .child(
            text(move || {
                ui.tick.get();
                session_value(|s| match s.editor.recording_macro() {
                    Some(r) => format!("recording @{r}  {}", s.editor.pending()),
                    None => s.editor.pending(),
                })
                .unwrap_or_default()
            })
            .font_family(MONO)
            .text_size(px(14.0))
            .text_color(hex(ORANGE)),
        )
        .child(
            text(move || {
                ui.tick.get();
                session_value(|s| format!("{}:{}", s.editor.cursor.row + 1, s.editor.cursor.col + 1))
                    .unwrap_or_default()
            })
            .font_family(MONO)
            .text_size(px(13.0))
            .text_color(hexa(WHITE, 0.5)),
        )
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Normal => "NORMAL",
        Mode::Insert => "-- INSERT --",
        Mode::Visual { line: false } => "-- VISUAL --",
        Mode::Visual { line: true } => "-- VISUAL LINE --",
        Mode::Block => "-- VISUAL BLOCK --",
        Mode::Replace => "-- REPLACE --",
        Mode::Command { .. } => "COMMAND",
        Mode::Confirm => "CONFIRM",
    }
}

/// The command line being typed, a question, or the latest word: the
/// game's toast first, then the editor's message, then a tip.
fn message(s: &Session, toast: String) -> String {
    match s.editor.mode {
        Mode::Command { .. } => format!("{}{}▏", s.editor.cmd_prefix(), s.editor.cmdline),
        Mode::Confirm => s.editor.confirm_prompt(),
        _ if !toast.is_empty() => toast,
        _ if !s.message.is_empty() => s.message.clone(),
        _ if !s.tip.is_empty() => format!("💡 {}", s.tip),
        _ => String::new(),
    }
}
