//! The hint card: what can follow the keys typed so far, floating over
//! the board while a command is half typed.

use nsh_core::prelude::*;

use crate::game::Session;
use crate::hints::{self, Hint};
use crate::levels::LEVELS;
use crate::ui::colors::{GOLD, ORANGE, PANEL, TEXT, WHITE, hex, hexa};
use crate::ui::motion::Ui;
use crate::ui::state::session_value;
use crate::ui::widgets::MONO;
use crate::vim::Mode;

/// The isle where text objects are taught: hints offer them from there on.
fn objects_taught(index: usize) -> bool {
    LEVELS
        .iter()
        .position(|l| l.title == "Inside Out")
        .is_some_and(|i| index >= i)
}

/// The hint for the keys pending, if a command is half typed.
fn pending_hint(s: &Session) -> Option<Hint> {
    if s.complete || !matches!(s.editor.mode, Mode::Normal | Mode::Visual { .. } | Mode::Block) {
        return None;
    }
    let pending = s.editor.pending();
    if pending.is_empty() {
        return None;
    }
    hints::hint(
        &pending,
        s.editor.in_visual(),
        s.allowed(),
        objects_taught(s.index),
    )
}

pub(crate) fn hint_card(ui: Ui) -> impl IntoElement {
    dynamic(move || {
        ui.tick.get();
        let hint = session_value(pending_hint)??;
        let rows = hint.next.into_iter().map(|(keys, what)| {
            view()
                .w(px(266.0))
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    view().w(px(80.0)).child(
                        text(keys)
                            .font_family(MONO)
                            .text_size(px(13.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(hex(GOLD)),
                    ),
                )
                .child(
                    text(what)
                        .text_size(px(12.0))
                        .text_color(hexa(TEXT, 0.8))
                        .flex_1(),
                )
        });
        Some(
            view()
                .absolute()
                .left(px(12.0))
                .bottom(px(46.0))
                .max_w(px(560.0))
                .p(px(12.0))
                .gap_1()
                .rounded(px(12.0))
                .bg(hexa(PANEL, 0.92))
                .border_1()
                .border_color(hexa(ORANGE, 0.5))
                .child(
                    text(hint.title)
                        .font_family(MONO)
                        .text_size(px(13.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(hex(ORANGE))
                        .mb(px(4.0)),
                )
                .child(view().flex_row().flex_wrap().gap_1().children(rows))
                .child(
                    text("Esc cancels")
                        .text_size(px(11.0))
                        .text_color(hexa(WHITE, 0.4))
                        .mt(px(4.0)),
                ),
        )
    })
}
