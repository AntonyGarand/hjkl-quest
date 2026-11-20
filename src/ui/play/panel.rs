//! The lesson panel beside the stage: the isle's story, its new commands,
//! what to use, the text to make, the files to write, and a tip.

use nsh_core::prelude::*;

use crate::levels::Level;
use crate::ui::colors::{BLACK, BUG, CREAM, GEM, LAVENDER, PANEL, PRESENT, TEXT, hex, hexa};
use crate::ui::motion::Ui;
use crate::ui::play::goal::goal_view;
use crate::ui::play::lessons::lesson_checklist;
use crate::ui::state::session_value;
use crate::ui::widgets::{MONO, chip, section_label};

pub(super) fn side_panel(level: &'static Level, ui: Ui) -> impl IntoElement {
    let teaches = level.teaches.iter().map(|(keys, what)| {
        view()
            .flex_row()
            .items_start()
            .gap_2()
            .child(view().min_w(px(64.0)).items_start().child(chip(*keys, CREAM)))
            .child(
                text(*what)
                    .text_size(px(13.5))
                    .text_color(hexa(TEXT, 0.85))
                    .flex_1(),
            )
    });
    view()
        .w(px(320.0))
        .gap_3()
        .p(px(16.0))
        .rounded(px(16.0))
        .bg(hexa(PANEL, 0.6))
        .border_1()
        .border_color(hexa(LAVENDER, 0.16))
        .child(
            text(level.story)
                .text_size(px(14.0))
                .line_height(px(20.0))
                .text_color(hexa(TEXT, 0.8))
                .italic(),
        )
        .child(section_label("NEW COMMANDS").mt(px(4.0)))
        .child(view().gap_2().children(teaches))
        .child(lesson_checklist(ui, level.title))
        .child(level.goal_text().map(|_| goal_section(ui)))
        .child((!level.files().is_empty() || !level.want_files().is_empty()).then(|| shelf(level, ui)))
        .child(crystals_note(level))
        .child(bugs_note(level))
        .child(view().flex_1())
        .child(
            text(format!("Tip: {}", level.tip))
                .text_size(px(12.5))
                .line_height(px(18.0))
                .text_color(hexa(TEXT, 0.55)),
        )
        .child(
            text(":h lessons · :e! restart · :q menu · :n skip · :mute")
                .font_family(MONO)
                .text_size(px(11.5))
                .text_color(hexa(TEXT, 0.4)),
        )
}

/// The text to make, coloured by what's in place.
fn goal_section(ui: Ui) -> impl IntoElement {
    view()
        .gap_2()
        .mt(px(4.0))
        .child(section_label("MAKE THE TEXT READ"))
        .child(
            view()
                .p(px(10.0))
                .rounded(px(8.0))
                .bg(hexa(BLACK, 0.35))
                .child(goal_view(ui, 13.0)),
        )
        .child(
            text("Green is in place, amber still missing; red tiles on the isle are extra.")
                .text_size(px(11.5))
                .line_height(px(16.0))
                .text_color(hexa(TEXT, 0.5)),
        )
}

/// The files the isle has on its shelf, and the ones it wants written.
fn shelf(level: &'static Level, ui: Ui) -> impl IntoElement {
    let names = level
        .files()
        .iter()
        .map(|(n, _)| *n)
        .collect::<Vec<_>>()
        .join("  ");
    view()
        .gap_1()
        .mt(px(4.0))
        .child(section_label("ON THE SHELF"))
        .child(
            text(names)
                .font_family(MONO)
                .text_size(px(12.5))
                .text_color(hexa(TEXT, 0.8)),
        )
        .children(
            level
                .want_files()
                .iter()
                .map(move |&(name, want)| file_to_write(ui, name, want)),
        )
}

/// A file to write: ticked once it holds `want`.
fn file_to_write(ui: Ui, name: &'static str, want: &'static str) -> impl IntoElement {
    view()
        .gap_1()
        .mt(px(4.0))
        .child(
            text(move || {
                ui.tick.get();
                let done =
                    session_value(|s| s.editor.files.get(name).is_some_and(|f| f == want)).unwrap_or(false);
                format!("{} AND WRITE {name}", if done { "✓" } else { "…" })
            })
            .text_size(px(11.0))
            .letter_spacing(px(1.2))
            .text_color(hex(PRESENT)),
        )
        .child(
            text(want.to_string())
                .font_family(MONO)
                .text_size(px(12.5))
                .text_color(hexa(TEXT, 0.75)),
        )
}

/// What clears the isle, when it's crystals.
fn crystals_note(level: &'static Level) -> Option<Text> {
    (level.goal.is_none() && !level.gems().is_empty()).then(|| {
        text("Collect every crystal ◆ to clear the isle.")
            .text_size(px(13.0))
            .text_color(hex(GEM))
            .mt(px(4.0))
    })
}

/// What clears the isle, when it's bugs.
fn bugs_note(level: &'static Level) -> Option<Text> {
    (!level.bugs().is_empty()).then(|| {
        text("Squash every bug: land on it. Each command moves them a tile, and they hatch twins every 8.")
            .text_size(px(13.0))
            .line_height(px(18.0))
            .text_color(hex(BUG))
            .mt(px(4.0))
    })
}
