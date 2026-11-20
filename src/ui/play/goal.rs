//! The goal text of an editing isle, each character green when the buffer
//! already has it and amber while it's missing, and a count of what's
//! left to do.

use nsh_core::prelude::*;

use crate::diff::Diff;
use crate::ui::colors::{CORAL, MISSING, PRESENT, hex};
use crate::ui::motion::Ui;
use crate::ui::state::{goal_lines, session_diff, session_value};
use crate::ui::widgets::MONO;

pub(super) fn goal_view(ui: Ui, size: f32) -> impl IntoElement {
    dynamic(move || {
        ui.tick.get();
        let (goal, d) = session_value(|s| (goal_lines(s), session_diff(s)))?;
        let (goal, d) = (goal?, d?);
        let summary = summary(&d);
        let rows: Vec<View> = goal
            .iter()
            .enumerate()
            .map(|(g, line)| goal_line(runs(&d, g, line), size))
            .collect();
        Some(view().child(view().children(rows)).child(summary))
    })
}

/// Goal row `g` in runs of characters in place (true) or missing.
fn runs(d: &Diff, g: usize, line: &[char]) -> Vec<(String, bool)> {
    let mut runs: Vec<(String, bool)> = Vec::new();
    for (j, c) in line.iter().enumerate() {
        let ok = d.present[g][j];
        // Spaces still to type show as a dot, so they can be seen.
        let shown = match (*c, ok) {
            (' ', true) => '\u{a0}',
            (' ', false) => '␣',
            (c, _) => c,
        };
        match runs.last_mut() {
            Some((t, o)) if *o == ok => t.push(shown),
            _ => runs.push((shown.to_string(), ok)),
        }
    }
    if line.is_empty() {
        let missing = d.missing_lines.iter().any(|&(_, mg)| mg == g);
        let shown = if missing { "⏎ empty line" } else { "\u{a0}" };
        runs.push((shown.into(), !missing));
    }
    runs
}

fn goal_line(runs: Vec<(String, bool)>, size: f32) -> View {
    view()
        .flex_row()
        .flex_wrap()
        .children(runs.into_iter().map(move |(t, ok)| {
            text(t)
                .font_family(MONO)
                .text_size(px(size))
                .line_height(px(size * 1.45))
                .font_weight(if ok { FontWeight::NORMAL } else { FontWeight::BOLD })
                .text_color(hex(if ok { PRESENT } else { MISSING }))
        }))
}

/// How many characters are in place, to add and to remove.
fn summary(d: &Diff) -> View {
    let count = |label: String, color: u32| text(label).text_size(px(12.0)).text_color(hex(color));
    view()
        .flex_row()
        .flex_wrap()
        .gap_2()
        .mt(px(6.0))
        .child(count(format!("✓ {} in place", d.present_count()), PRESENT))
        .child(count(format!("+ {} to add", d.missing_count()), MISSING))
        .child(count(format!("✕ {} to remove", d.wrong_count()), CORAL))
}
