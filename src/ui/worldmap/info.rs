//! Around the map: the journey so far, the selected isle's card, and the
//! map's keys.

use nsh_core::prelude::*;

use crate::audio;
use crate::daily;
use crate::game;
use crate::grammar;
use crate::levels::{LEVELS, Level};
use crate::story;
use crate::ui::colors::{BOSS, CREAM, GEM, GOLD, LAVENDER, ORANGE, PANEL, TEXT, WHITE, hex, hexa};
use crate::ui::motion::Ui;
use crate::ui::state::{clock, frontier, is_boss_isle, with_state};
use crate::ui::widgets::{MONO, chip, legend};
use crate::ui::worldmap::selected;

/// Isles cleared, stars won, the grammar and today's tide.
pub(super) fn summary(ui: Ui) -> impl IntoElement {
    text(move || {
        ui.tick.get();
        with_state(|s| {
            let cleared = s.best.iter().filter(|b| b.is_some()).count();
            let stars: usize = s
                .best
                .iter()
                .enumerate()
                .filter_map(|(i, b)| b.map(|n| game::stars(n, LEVELS[i].par()) as usize))
                .sum();
            let daily = match s.daily_best {
                Some((d, n)) if d == daily::today() => format!("  ·  today's tide: {n} keys"),
                _ => String::new(),
            };
            format!(
                "{cleared} of {} isles  ·  ★ {stars}  ·  grammar {} of {}{daily}",
                LEVELS.len(),
                s.grammar.len(),
                grammar::VERBS.len() * grammar::NOUNS.len()
            )
        })
    })
    .font_family(MONO)
    .text_size(px(13.0))
    .text_color(hexa(TEXT, 0.7))
}

/// The selected isle, as the card under the map shows it.
struct Isle {
    index: usize,
    level: &'static Level,
    best: Option<usize>,
    time: Option<u32>,
    free_best: Option<u32>,
    foggy: bool,
    toast: String,
}

impl Isle {
    fn selected() -> Isle {
        let index = selected();
        with_state(|s| Isle {
            index,
            level: &LEVELS[index],
            best: s.best[index],
            time: s.times[index],
            free_best: s.free_best[index],
            foggy: index >= frontier(&s.best) + 2 && s.best[index].is_none(),
            toast: s.toast.clone(),
        })
    }

    /// ★★☆ and the keys against par.
    fn score(&self) -> String {
        let par = self.level.par();
        match self.best {
            Some(n) => {
                let stars = game::stars(n, par) as usize;
                format!(
                    "{}{}  {n} keys, par {par}",
                    "★".repeat(stars),
                    "☆".repeat(3 - stars)
                )
            }
            None => format!("☆☆☆  par {par}"),
        }
    }
}

/// The selected isle's card: its region and chapter, name, story, boss,
/// keys, scores and free play.
pub(super) fn isle_info(ui: Ui) -> impl IntoElement {
    dynamic(move || {
        ui.tick.get();
        let isle = Isle::selected();
        Some(
            view()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap_4()
                .p(px(14.0))
                .rounded(px(14.0))
                .bg(hexa(PANEL, 0.7))
                .border_1()
                .border_color(hexa(LAVENDER, 0.18))
                .child(about(&isle))
                .child(scores(&isle)),
        )
    })
}

/// Where the isle is, what it's called, its story and its boss.
fn about(isle: &Isle) -> impl IntoElement {
    let level = isle.level;
    let n = isle.index + 1;
    let (title, story_line) = if isle.foggy {
        (
            format!("{n}. Uncharted waters"),
            "Clear the isles before it to lift the fog. You can still sail there.".to_string(),
        )
    } else {
        (format!("{n}. {}", level.title), level.story.to_string())
    };
    let toast = isle.toast.clone();
    view()
        .flex_1()
        .min_w(px(260.0))
        .gap_1()
        .child(
            text(format!(
                "{}  ·  {}",
                story::REGIONS[story::region_of(level.chapter)].name,
                level.chapter
            ))
            .text_size(px(12.0))
            .letter_spacing(px(1.5))
            .text_color(hexa(LAVENDER, 0.75)),
        )
        .child(
            text(title)
                .text_size(px(22.0))
                .font_weight(FontWeight::BOLD)
                .text_color(hex(CREAM)),
        )
        .child(
            text(story_line)
                .text_size(px(13.0))
                .line_height(px(18.0))
                .italic()
                .text_color(hexa(TEXT, 0.7))
                .number_of_lines(2),
        )
        .child(is_boss_isle(isle.index).then(|| {
            let name = story::chapter(level.chapter).map_or("", |c| c.boss);
            text(format!("⚔ Boss: {name}"))
                .text_size(px(13.0))
                .font_weight(FontWeight::BOLD)
                .text_color(hex(BOSS))
        }))
        .child((!toast.is_empty()).then(|| text(toast).text_size(px(13.0)).text_color(hex(ORANGE))))
}

/// The keys the isle unlocks, its best score and times, and free play.
fn scores(isle: &Isle) -> impl IntoElement {
    view()
        .gap_1()
        .items_end()
        .child(
            view().flex_row().gap_1().children(
                isle.level
                    .unlock
                    .chars()
                    .take(8)
                    .map(|c| chip(game::key_name(c), CREAM)),
            ),
        )
        .child(
            text(isle.score())
                .font_family(MONO)
                .text_size(px(14.0))
                .text_color(hex(GOLD)),
        )
        .child(isle.time.map(|t| {
            text(format!("⏱ best {}", clock(f64::from(t) * 100.0)))
                .font_family(MONO)
                .text_size(px(12.5))
                .text_color(hexa(TEXT, 0.6))
        }))
        .child(isle.free_best.map(|b| {
            text(format!("free play best {b} keys"))
                .font_family(MONO)
                .text_size(px(12.5))
                .text_color(hexa(GEM, 0.8))
        }))
        .child(isle.best.is_some().then(|| {
            view()
                .flex_row()
                .gap_1()
                .items_center()
                .child(chip("f", GEM))
                .child(
                    text("free play: every key")
                        .text_size(px(12.0))
                        .text_color(hexa(TEXT, 0.6)),
                )
        }))
}

/// The map's keys.
pub(super) fn keys_legend(ui: Ui) -> impl IntoElement {
    let mut legend_row = view()
        .flex_row()
        .flex_wrap()
        .justify_center()
        .items_center()
        .gap_2();
    for (keys, color, what) in [
        ("h l", CREAM, "isle"),
        ("j k", CREAM, "chapter"),
        ("J K", CREAM, "region"),
        ("Enter", CREAM, "sail in"),
        ("d", GEM, "daily tide"),
        ("r", GEM, "review drills"),
        ("v", GEM, "your grammar"),
    ] {
        legend_row = legend_row
            .child(chip(keys, color))
            .child(legend(what).mr(px(6.0)));
    }
    legend_row.child(chip("m", CREAM)).child(
        text(move || {
            ui.tick.get();
            if audio::muted() { "sound off" } else { "sound on" }
        })
        .text_color(hexa(WHITE, 0.6)),
    )
}
