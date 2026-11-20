//! The card at the end of a run: stars and a verdict, the time, a boss's
//! toolkit or the ending, the keys against the golf line, habits to break,
//! and what Enter, r and q do next.

mod parts;

use std::rc::Rc;

use nsh_core::prelude::*;

use crate::game::Session;
use crate::levels::{self, Level};
use crate::story;
use crate::ui::colors::{BOSS, CARD, CORAL, CREAM, GEM, GOLD, NIGHT, TEXT, hex, hexa};
use crate::ui::motion::{Motion, Ui};
use crate::ui::state::{clock, is_boss, session_value, with_state};
use crate::ui::widgets::{MONO, cover};
use crate::vim::format_keys;

/// A finished run, as the card shows it.
struct Run {
    level: &'static Level,
    keys: String,
    count: usize,
    stars: u8,
    par: usize,
    campaign: bool,
    free: bool,
    /// The boss guarding the isle, if one does.
    boss: Option<&'static str>,
    /// The lessons the run didn't use: it doesn't count.
    missed: Vec<String>,
    habits: Vec<String>,
    trapped: usize,
    took: f64,
    best_time: Option<u32>,
    free_best: Option<u32>,
    drills_left: usize,
}

impl Run {
    fn of(s: &Session) -> Run {
        let level = s.level();
        let (campaign, free) = (s.in_campaign(), s.free);
        Run {
            level,
            keys: format_keys(&s.keys),
            count: s.keys.len(),
            stars: s.stars(),
            par: s.par(),
            campaign,
            free,
            boss: is_boss(s).then(|| story::chapter(level.chapter).map_or("", |c| c.boss)),
            missed: s.missed.clone(),
            habits: s.coach.spotted.iter().map(|h| h.label.clone()).collect(),
            trapped: s.trapped,
            took: 0.0,
            best_time: None,
            free_best: None,
            drills_left: 0,
        }
    }

    /// The run on screen, once it's done.
    fn current() -> Option<Run> {
        let (mut run, index) = session_value(|s| s.complete.then(|| (Run::of(s), s.index)))??;
        with_state(|s| {
            run.took = s.took.unwrap_or(0.0);
            run.best_time = s.times[index];
            run.free_best = s.free_best[index];
            run.drills_left = s.drills.len();
        });
        Some(run)
    }

    fn failed(&self) -> bool {
        !self.missed.is_empty()
    }

    /// The boss beaten for real: not in free play, no lesson missed.
    fn boss_beaten(&self) -> Option<&'static str> {
        self.boss.filter(|_| !self.failed() && !self.free)
    }

    /// The campaign's last isle: the ending shows.
    fn last(&self) -> bool {
        self.campaign && !self.free && self.level.title == levels::ENDING
    }

    fn verdict(&self) -> &'static str {
        if self.failed() && self.free {
            "Not this time."
        } else if self.failed() {
            "Lesson missed: the isle isn't cleared."
        } else if self.count < self.par {
            "Under par! You beat the golf solution."
        } else if self.count == self.par {
            "Exactly par. A true vim golfer."
        } else {
            "Isle cleared! Now try to reach par."
        }
    }
}

pub(crate) fn complete_card(ui: Ui, motion: &Rc<Motion>) -> impl IntoElement {
    let motion = motion.clone();
    let scale = motion.card.interpolate(&[0.0, 1.0], &[0.85, 1.0]);
    dynamic(move || {
        ui.tick.get();
        let run = Run::current()?;
        let card = view()
            .bind_scale(&scale)
            .bind_opacity(&motion.card)
            .w(px(460.0))
            .max_h(px((ui.stage.get().1 - 24.0).max(260.0)))
            .rounded(px(18.0))
            .bg(hexa(CARD, 0.96))
            .border_1()
            .border_color(hexa(CREAM, 0.35))
            .child(scroll().child(body(&run)));
        Some(
            cover(view())
                .items_center()
                .justify_center()
                .bg(hexa(NIGHT, 0.55))
                .child(card),
        )
    })
}

fn body(run: &Run) -> View {
    view()
        .p(px(24.0))
        .gap_3()
        .items_center()
        .child(stars(run))
        .child(
            text(run.verdict())
                .text_size(px(18.0))
                .font_weight(FontWeight::BOLD)
                .text_color(hex(CREAM)),
        )
        .child(run.free.then(|| free_play_line(run.free_best)))
        .child(run.boss_beaten().map(|name| {
            text(format!("⚔ {name} is defeated!"))
                .text_size(px(14.0))
                .font_weight(FontWeight::BOLD)
                .text_color(hex(BOSS))
        }))
        .child(
            run.boss_beaten()
                .and_then(|_| parts::toolkit_box(run.level.chapter)),
        )
        .child(time_line(run))
        .child((run.last() && !run.failed()).then(parts::ending))
        .child((run.failed() && !run.free).then(|| parts::missed_box(&run.missed)))
        .child(parts::keys_section(run.count, &run.keys, run.par, run.level))
        .child((!run.habits.is_empty() || run.trapped > 0).then(|| parts::habits(&run.habits, run.trapped)))
        .child(parts::footer(run))
}

/// ★★☆, or ✗ when a lesson was missed.
fn stars(run: &Run) -> Text {
    let (shown, color) = if run.failed() {
        ("✗".to_string(), CORAL)
    } else {
        let stars = run.stars as usize;
        (format!("{}{}", "★".repeat(stars), "☆".repeat(3 - stars)), GOLD)
    };
    text(shown).text_size(px(40.0)).text_color(hex(color))
}

fn free_play_line(best: Option<u32>) -> Text {
    text(match best {
        Some(b) => format!("FREE PLAY  ·  every key allowed  ·  your best {b} keys"),
        None => "FREE PLAY  ·  every key allowed".to_string(),
    })
    .font_family(MONO)
    .text_size(px(12.0))
    .letter_spacing(px(1.0))
    .text_color(hex(GEM))
}

/// The run's time, and the isle's best on a campaign clear.
fn time_line(run: &Run) -> Text {
    let best = match run.best_time {
        Some(t) if run.campaign && !run.failed() && !run.free => {
            format!("   ·   best {}", clock(f64::from(t) * 100.0))
        }
        _ => String::new(),
    };
    text(format!("⏱ {}{best}", clock(run.took)))
        .font_family(MONO)
        .text_size(px(13.0))
        .text_color(hexa(TEXT, 0.75))
}
