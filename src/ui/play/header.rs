//! Above the stage, the isle's chapter, name and scores; below it, the
//! keys typed so far.

use nsh_core::prelude::*;

use crate::levels::Level;
use crate::ui::colors::{CREAM, LAVENDER, TEXT, WHITE, hex, hexa};
use crate::ui::motion::Ui;
use crate::ui::state::{clock, now_ms, session_value, with_state};
use crate::ui::widgets::MONO;
use crate::vim::format_keys;

/// The most keys the typed line shows: older ones scroll off.
const TYPED_SHOWN: usize = 70;
/// What a score shows when there is none.
const NONE: &str = "–";

/// The chapter over the isle's name (numbered in the campaign).
fn title(index: usize, level: &'static Level, campaign: bool) -> impl IntoElement {
    let title = if campaign {
        format!("{}. {}", index + 1, level.title)
    } else {
        level.title.to_string()
    };
    view()
        .flex_1()
        .min_w(px(220.0))
        .gap_1()
        .child(
            text(level.chapter)
                .text_size(px(12.0))
                .letter_spacing(px(1.5))
                .text_color(hexa(LAVENDER, 0.75)),
        )
        .child(
            text(title)
                .text_size(px(24.0))
                .font_weight(FontWeight::BOLD)
                .text_color(hex(CREAM)),
        )
}

pub(super) fn top_bar(ui: Ui, index: usize, level: &'static Level, campaign: bool) -> impl IntoElement {
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_4()
        .child(title(index, level, campaign))
        .child(stat("KEYS", move || {
            ui.tick.get();
            session_value(|s| match s.level().budget() {
                Some(b) => format!("{}/{b}", s.keys.len()),
                None => s.keys.len().to_string(),
            })
            .unwrap_or_default()
        }))
        .child(stat("TIME", move || {
            ui.tick.get();
            ui.beat.get();
            with_state(|s| match (s.took, s.started) {
                (Some(t), _) => clock(t),
                (None, Some(t)) => clock(now_ms() - t),
                (None, None) => clock(0.0),
            })
        }))
        .child(stat("PAR", move || level.par().to_string()))
        .child(stat("CRYSTALS", move || {
            ui.tick.get();
            session_value(|s| match s.gems.len() {
                0 => NONE.to_string(),
                total => format!("{}/{}", total - s.gems_left(), total),
            })
            .unwrap_or_default()
        }))
        .child(stat("BEST", move || {
            ui.tick.get();
            if !campaign {
                return NONE.to_string();
            }
            with_state(|s| s.best[index].map_or(NONE.to_string(), |n| n.to_string()))
        }))
}

/// A score: its label over its value.
fn stat(label: &'static str, value: impl Fn() -> String + 'static) -> impl IntoElement {
    view()
        .items_end()
        .min_w(px(64.0))
        .child(
            text(label)
                .text_size(px(11.0))
                .letter_spacing(px(1.2))
                .text_color(hexa(WHITE, 0.45)),
        )
        .child(
            text(value)
                .font_family(MONO)
                .text_size(px(22.0))
                .font_weight(FontWeight::BOLD)
                .text_color(hex(TEXT)),
        )
}

/// The keys typed so far, the latest last.
pub(super) fn typed(ui: Ui) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(text("typed").text_size(px(12.0)).text_color(hexa(WHITE, 0.45)))
        .child(
            text(move || {
                ui.tick.get();
                session_value(|s| {
                    let keys = format_keys(&s.keys);
                    let chars: Vec<char> = keys.chars().collect();
                    if chars.len() > TYPED_SHOWN {
                        format!(
                            "…{}",
                            chars[chars.len() - TYPED_SHOWN..].iter().collect::<String>()
                        )
                    } else {
                        keys
                    }
                })
                .unwrap_or_default()
            })
            .font_family(MONO)
            .text_size(px(14.0))
            .text_color(hex(CREAM))
            .number_of_lines(1),
        )
}
