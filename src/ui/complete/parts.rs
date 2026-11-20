//! The end-of-run card's sections.

use nsh_core::prelude::*;

use super::Run;
use crate::game;
use crate::levels::{LEVELS, Level};
use crate::story;
use crate::ui::colors::{CORAL, CREAM, GEM, GOLD, PRESENT, TEXT, WHITE, hex, hexa};
use crate::ui::state::with_state;
use crate::ui::widgets::{MONO, chip};
use crate::vim::{self, format_keys};

/// A section's title on the card.
fn label(title: String) -> Text {
    text(title)
        .text_size(px(11.0))
        .letter_spacing(px(1.2))
        .text_color(hexa(WHITE, 0.5))
}

/// A box across the card, tinted `color` (`fill` and `edge` its opacities).
fn tinted_box(color: u32, fill: f32, edge: f32, padding: f32) -> View {
    view()
        .w_full()
        .p(px(padding))
        .rounded(px(10.0))
        .bg(hexa(color, fill))
        .border_1()
        .border_color(hexa(color, edge))
}

/// The commands a chapter's boss leaves behind: like the chapter's own,
/// and usable from the next isle on.
pub(super) fn toolkit_box(chapter: &str) -> Option<View> {
    let kit = story::chapter(chapter)?.toolkit;
    if kit.is_empty() {
        return None;
    }
    let rows = kit.iter().map(|(keys, what)| {
        view()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                text(*keys)
                    .font_family(MONO)
                    .font_weight(FontWeight::BOLD)
                    .text_size(px(13.0))
                    .text_color(hex(GOLD))
                    .min_w(px(120.0)),
            )
            .child(
                text(*what)
                    .text_size(px(12.5))
                    .text_color(hexa(TEXT, 0.8))
                    .flex_1(),
            )
    });
    Some(
        tinted_box(PRESENT, 0.08, 0.35, 10.0)
            .gap_1()
            .child(
                text("NEW IN YOUR TOOLKIT")
                    .text_size(px(11.0))
                    .letter_spacing(px(1.4))
                    .font_weight(FontWeight::BOLD)
                    .text_color(hex(PRESENT)),
            )
            .child(
                text("Cousins of what this chapter taught. They work from the next isle on.")
                    .text_size(px(12.0))
                    .text_color(hexa(TEXT, 0.6)),
            )
            .children(rows),
    )
}

/// The ending: the archipelago restored, and the stars of the whole journey.
pub(super) fn ending() -> View {
    let (stars, cleared) = with_state(|s| {
        let stars: usize = s
            .best
            .iter()
            .enumerate()
            .filter_map(|(i, b)| b.map(|n| game::stars(n, LEVELS[i].par()) as usize))
            .sum();
        (stars, s.best.iter().filter(|b| b.is_some()).count())
    });
    tinted_box(CREAM, 0.08, 0.35, 12.0)
        .gap_2()
        .items_center()
        .child(
            text("THE END")
                .text_size(px(13.0))
                .letter_spacing(px(3.0))
                .font_weight(FontWeight::BOLD)
                .text_color(hex(CREAM)),
        )
        .child(
            text(
                "The last door of the Archive swings open. Every isle of the Buffer Archipelago answers to \
                 your keys now: no arrows, no mouse, just the home row. The wisp settles on the summit \
                 and looks out over the clouds, already counting keystrokes it could have saved.",
            )
            .text_size(px(13.5))
            .line_height(px(19.0))
            .italic()
            .text_center()
            .text_color(hexa(TEXT, 0.85)),
        )
        .child(
            text(format!(
                "{cleared} of {} isles cleared · ★ {stars} of {}",
                LEVELS.len(),
                LEVELS.len() * 3
            ))
            .font_family(MONO)
            .text_size(px(13.0))
            .text_color(hex(GOLD)),
        )
        .child(
            text("Now open a real file in vim. Every command here works there too.")
                .text_size(px(12.5))
                .text_color(hexa(TEXT, 0.6)),
        )
}

/// The lessons a run reached the goal without.
pub(super) fn missed_box(missed: &[String]) -> View {
    tinted_box(CORAL, 0.12, 0.4, 10.0)
        .gap_2()
        .child(
            text("You reached the goal without using what this isle teaches. Use these, then it counts:")
                .text_size(px(13.5))
                .line_height(px(19.0))
                .text_color(hexa(TEXT, 0.85)),
        )
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .gap_2()
                .children(missed.iter().map(|m| chip(m.clone(), CORAL))),
        )
}

/// The run's keys over the golf solution, and the way to replay them.
pub(super) fn keys_section(count: usize, keys: &str, par: usize, level: &'static Level) -> View {
    view()
        .w_full()
        .gap_1()
        .mt(px(6.0))
        .child(label(format!("YOUR KEYS · {count}")))
        .child(
            text(keys.to_string())
                .font_family(MONO)
                .text_size(px(14.0))
                .text_color(hex(TEXT)),
        )
        .child(label(format!("GOLF SOLUTION · {par}")).mt(px(8.0)))
        .child(spoiler(format_keys(&vim::parse_keys(level.solution))))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .mt(px(6.0))
                .child(chip("p", GEM))
                .child(
                    text("replay your run beside the golf line, step by step")
                        .text_size(px(12.5))
                        .text_color(hexa(WHITE, 0.6)),
                ),
        )
}

/// The golf solution, hidden until the pointer is over it (or it is
/// pressed, on touch screens), so it doesn't spoil a retry.
fn spoiler(solution: String) -> impl IntoElement {
    let shown = signal(false);
    let masked: String = solution.chars().map(|_| '•').collect();
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .px(px(8.0))
        .py(px(4.0))
        .ml(px(-8.0))
        .rounded(px(6.0))
        .bg(move || {
            if shown.get() {
                hexa(GEM, 0.08)
            } else {
                hexa(WHITE, 0.05)
            }
        })
        .cursor_pointer()
        .on_mouse_enter(move |_: &MouseEvent| shown.set(true))
        .on_mouse_leave(move |_: &MouseEvent| shown.set(false))
        .on_press(move || shown.update(|s| *s = !*s))
        .child(
            text(move || {
                if shown.get() {
                    solution.clone()
                } else {
                    masked.clone()
                }
            })
            .font_family(MONO)
            .text_size(px(14.0))
            .text_color(move || if shown.get() { hex(GEM) } else { hexa(GEM, 0.35) }),
        )
        .child(dynamic(move || {
            (!shown.get()).then(|| {
                text("hover to reveal")
                    .text_size(px(11.0))
                    .italic()
                    .text_color(hexa(WHITE, 0.4))
            })
        }))
}

/// Habits the coach spotted, and traps that threw the wisp back.
pub(super) fn habits(habits: &[String], trapped: usize) -> View {
    view()
        .w_full()
        .gap_1()
        .mt(px(4.0))
        .child(label("HABITS TO BREAK".to_string()))
        .children(habits.iter().take(4).map(|h| {
            text(format!("• {h}"))
                .font_family(MONO)
                .text_size(px(13.0))
                .text_color(hex(GOLD))
        }))
        .child((trapped > 0).then(|| {
            text(format!(
                "• thrown back by a trap {trapped}×: jump past it in one move"
            ))
            .text_size(px(13.0))
            .text_color(hex(CORAL))
        }))
}

/// What Enter, r (or n) and q do next.
pub(super) fn footer(run: &Run) -> View {
    let failed = run.failed();
    let next = if failed {
        "try again".to_string()
    } else if run.drills_left > 0 {
        format!("next drill ({} left)", run.drills_left)
    } else if run.last() || !run.campaign || run.free {
        "back to the map".to_string()
    } else {
        "next isle".to_string()
    };
    let what = |t: String| text(t).text_color(hexa(WHITE, 0.65));
    view()
        .flex_row()
        .gap_2()
        .mt(px(10.0))
        .items_center()
        .child(chip("Enter", CREAM))
        .child(what(next).mr(px(10.0)))
        .child(chip(if failed { "n" } else { "r" }, CREAM))
        .child(what(if failed { "skip it" } else { "retry" }.to_string()).mr(px(10.0)))
        .child(chip("q", CREAM))
        .child(what("map".to_string()))
}
