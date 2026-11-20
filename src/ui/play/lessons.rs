//! What the isle teaches: its commands, and the ones it must see used,
//! ticked off as they are. On narrow screens, a strip over the stage.

use nsh_core::prelude::*;

use crate::lessons;
use crate::levels::Level;
use crate::ui::colors::{BLACK, CREAM, PRESENT, TEXT, hex, hexa};
use crate::ui::motion::Ui;
use crate::ui::play::goal::goal_view;
use crate::ui::state::session_value;
use crate::ui::widgets::{MONO, chip, section_label};

/// The lesson panel folded into a strip of its commands, its checklist
/// and its goal.
pub(super) fn lesson_strip(ui: Ui, level: &'static Level) -> impl IntoElement {
    let chips = view()
        .flex_row()
        .flex_wrap()
        .gap_2()
        .items_center()
        .children(level.teaches.iter().map(|(keys, what)| {
            view()
                .flex_row()
                .gap_1()
                .items_center()
                .child(chip(*keys, CREAM))
                .child(
                    text(*what)
                        .text_size(px(12.0))
                        .text_color(hexa(TEXT, 0.7))
                        .mr(px(6.0)),
                )
        }));
    let goal = level.goal_text().map(|_| {
        view()
            .flex_row()
            .gap_3()
            .items_start()
            .px(px(10.0))
            .py(px(6.0))
            .rounded(px(8.0))
            .bg(hexa(BLACK, 0.35))
            .child(section_label("GOAL").mt(px(3.0)))
            .child(view().flex_1().child(goal_view(ui, 12.0)))
    });
    view()
        .gap_2()
        .child(chips)
        .child(lesson_checklist(ui, level.title))
        .child(goal)
}

/// The commands the isle must see used, ticked off as they are.
pub(super) fn lesson_checklist(ui: Ui, title: &'static str) -> impl IntoElement {
    if lessons::requirements(title).is_empty() {
        return view().into_element();
    }
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(section_label("USE ON THIS ISLE").mr(px(4.0)))
        .child(dynamic(move || {
            ui.tick.get();
            let status = session_value(|s| lessons::status(title, &s.used)).unwrap_or_default();
            Some(
                view().flex_row().flex_wrap().gap_2().children(
                    status
                        .into_iter()
                        .map(|(label, optional, done)| lesson(label, optional, done)),
                ),
            )
        }))
        .into_element()
}

/// One command to use: ✓ once used, ○ while required, ◇ if optional.
fn lesson(label: &str, optional: bool, done: bool) -> impl IntoElement {
    let (mark, color) = match (done, optional) {
        (true, _) => ("✓", hex(PRESENT)),
        (false, false) => ("○", hexa(TEXT, 0.6)),
        (false, true) => ("◇", hexa(TEXT, 0.35)),
    };
    let shown = if optional {
        format!("{mark} {label} (optional)")
    } else {
        format!("{mark} {label}")
    };
    text(shown)
        .font_family(MONO)
        .text_size(px(12.5))
        .text_color(color)
}
