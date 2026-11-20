//! The intro shown before an isle the first time (and on `:h`): what it
//! teaches, and each command played out on a small looping example.

mod example;

use nsh_core::prelude::*;

use crate::examples;
use crate::lessons;
use crate::levels::Level;
use crate::story;
use crate::ui::colors::{BOSS, CARD, CREAM, GEM, GOLD, INK, LAVENDER, NIGHT, TEXT, WHITE, hex, hexa};
use crate::ui::motion::Ui;
use crate::ui::state::{Screen, State, chapter_start, is_boss, with_state};
use crate::ui::widgets::{chip, cover, section_label};

/// Pip, the owl who ferries the wisp: the voice of a chapter's first isle.
const PIP: &str = "🦉 Pip";
const PIP_BLUE: u32 = 0x9fd8ff;
/// Below this width the intro's card narrows.
const NARROW: f32 = 760.0;

pub(crate) fn intro_modal(ui: Ui) -> impl IntoElement {
    dynamic(move || {
        ui.tick.get();
        let (index, level, boss) = with_state(|s| match &s.screen {
            Screen::Play(p) if s.intro => Some((p.index, p.level(), is_boss(p))),
            _ => None,
        })?;
        let (width, height) = ui.window.get_untracked();
        let card = view()
            .w_full()
            .max_w(px(if width < NARROW { 520.0 } else { 760.0 }))
            .max_h(px((height - 40.0).max(300.0)))
            .rounded(px(18.0))
            .bg(hexa(CARD, 0.97))
            .border_1()
            .border_color(hexa(CREAM, 0.35))
            .child(scroll().child(body(ui, index, level, boss)));
        Some(
            cover(view())
                .items_center()
                .justify_center()
                .bg(hexa(NIGHT, 0.74))
                .child(card),
        )
    })
}

fn body(ui: Ui, index: usize, level: &'static Level, boss: bool) -> View {
    let lessons = level.teaches.iter().map(|(keys, what)| {
        view()
            .flex_row()
            .items_start()
            .gap_3()
            .child(view().min_w(px(110.0)).items_start().child(chip(*keys, CREAM)))
            .child(
                text(*what)
                    .text_size(px(14.0))
                    .line_height(px(20.0))
                    .text_color(hexa(TEXT, 0.88))
                    .flex_1(),
            )
    });
    let cards = examples::examples(level.title)
        .iter()
        .map(move |&(text, keys)| example::example_card(ui, text, keys));
    view()
        .p(px(24.0))
        .gap_3()
        .child(
            text(level.chapter)
                .text_size(px(12.0))
                .letter_spacing(px(1.5))
                .text_color(hexa(LAVENDER, 0.75)),
        )
        .child(
            text(format!("{}. {}", index + 1, level.title))
                .text_size(px(26.0))
                .font_weight(FontWeight::BOLD)
                .text_color(hex(CREAM)),
        )
        .child(
            text(level.story)
                .text_size(px(14.0))
                .line_height(px(20.0))
                .italic()
                .text_color(hexa(TEXT, 0.75)),
        )
        .child(speech(index, level, boss).map(|(who, line)| speech_box(who, line)))
        .child(section_label("YOU WILL LEARN").mt(px(6.0)))
        .child(view().gap_2().children(lessons))
        .child(section_label("SEE IT WORK").mt(px(6.0)))
        .child(view().flex_row().flex_wrap().gap_3().children(cards))
        .child(
            text(goal(level))
                .text_size(px(13.0))
                .line_height(px(18.0))
                .text_color(hex(GEM))
                .mt(px(4.0)),
        )
        .child(
            text(requirements(level.title))
                .text_size(px(13.0))
                .line_height(px(18.0))
                .text_color(hex(GOLD)),
        )
        .child(footer(ui))
}

/// Pip speaks rowing up to a chapter; a boss guards its last isle.
fn speech(index: usize, level: &Level, boss: bool) -> Option<(&'static str, &'static str)> {
    let chapter = story::chapter(level.chapter)?;
    if index == chapter_start(index) {
        Some((PIP, chapter.pip))
    } else if boss {
        Some((chapter.boss, chapter.taunt))
    } else {
        None
    }
}

/// Who speaks, and their line, in a box of their colour.
fn speech_box(who: &'static str, line: &str) -> impl IntoElement {
    let color = if who == PIP { PIP_BLUE } else { BOSS };
    view()
        .flex_row()
        .gap_3()
        .items_start()
        .p(px(12.0))
        .rounded(px(12.0))
        .bg(hexa(color, 0.10))
        .border_1()
        .border_color(hexa(color, 0.4))
        .child(
            text(who)
                .text_size(px(13.0))
                .font_weight(FontWeight::BOLD)
                .text_color(hex(color))
                .min_w(px(80.0)),
        )
        .child(
            text(format!("“{line}”"))
                .text_size(px(14.0))
                .line_height(px(20.0))
                .text_color(hexa(TEXT, 0.9))
                .flex_1(),
        )
}

/// What clears the isle, and the rules that make it harder.
fn goal(level: &Level) -> String {
    let goal = if !level.bugs().is_empty() {
        "Goal: squash every bug by landing on it. Each command moves them one tile; every 8 commands they hatch twins."
    } else if !level.want_files().is_empty() {
        "Goal: make the text match the goal, and write the files it asks for."
    } else if level.goal.is_some() {
        "Goal: edit the text until it matches the goal shown beside the isle."
    } else if !level.traps().is_empty() {
        "Goal: collect every crystal ◆. Spiked traps throw you back if you land on one: jump over them."
    } else {
        "Goal: collect every crystal ◆."
    };
    let mut goal = match level.budget() {
        Some(b) => format!("{goal} Only {b} keystrokes allowed: run out and the isle crumbles."),
        None => goal.to_string(),
    };
    if !level.fading().is_empty() {
        goal.push_str(" Numbered crystals fade: the number is how many commands each has left.");
    }
    if !level.sentries().is_empty() {
        goal.push_str(
            " Sentries march a row up or down with every command you finish: meet one and you're back at the start.",
        );
    }
    if !level.frozen().is_empty() {
        goal.push_str(" The icy lines are frozen: no edit may change them.");
    }
    if !level.banned().is_empty() {
        let keys: Vec<String> = level.banned().chars().map(|c| c.to_string()).collect();
        goal.push_str(" Banned here: ");
        goal.push_str(&keys.join(" "));
        goal.push('.');
    }
    goal
}

/// The commands the isle must see used, and those worth a try.
fn requirements(title: &str) -> String {
    let reqs = lessons::requirements(title);
    let pick = |optional: bool| -> Vec<&str> {
        reqs.iter()
            .filter(|(_, p)| lessons::optional(p) == optional)
            .map(|(l, _)| *l)
            .collect()
    };
    let (must, may) = (pick(false), pick(true));
    let mut line = format!(
        "To clear it, use: {}. Reaching the goal without them doesn't count.",
        must.join(" · ")
    );
    if !may.is_empty() {
        line.push_str(" Worth a try too: ");
        line.push_str(&may.join(" · "));
        line.push('.');
    }
    line
}

/// Enter (or the button) starts the isle.
fn footer(ui: Ui) -> impl IntoElement {
    let start = view()
        .px(px(18.0))
        .py(px(8.0))
        .rounded(px(10.0))
        .bg(hex(CREAM))
        .cursor_pointer()
        .on_press(move || {
            with_state(State::close_intro);
            ui.bump_tick();
        })
        .child(text("Start").font_weight(FontWeight::BOLD).text_color(hex(INK)));
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_2()
        .mt(px(6.0))
        .child(chip("Enter", CREAM))
        .child(text("start the isle").text_color(hexa(WHITE, 0.65)).mr(px(10.0)))
        .child(chip(":h", CREAM))
        .child(text("see this again").text_color(hexa(WHITE, 0.65)))
        .child(view().flex_1())
        .child(start)
}
