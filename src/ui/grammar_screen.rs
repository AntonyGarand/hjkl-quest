//! The grid of verbs and nouns: every operator and every motion or text
//! object, lit where the player has used the pair.

use std::collections::BTreeSet;
use std::rc::Rc;

use nsh_core::prelude::*;

use crate::grammar;
use crate::ui::colors::{CREAM, GOLD, LAVENDER, PANEL, PRESENT, TEXT, WHITE, hex, hexa};
use crate::ui::motion::Ui;
use crate::ui::state::with_state;
use crate::ui::widgets::{MONO, chip, legend};

/// A verb's column, and the noun labels' width.
const CELL_W: f32 = 64.0;
const NOUN_W: f32 = 190.0;

pub(crate) fn grammar_screen(ui: Ui) -> impl IntoElement {
    let used = Rc::new(with_state(|s| s.grammar.clone()));
    let total = grammar::VERBS.len() * grammar::NOUNS.len();
    let count = used.len();
    let rows = grammar::NOUNS
        .iter()
        .map(move |&(noun, what)| noun_row(used.clone(), noun, what));
    view()
        .flex_1()
        .items_center()
        .p(px(20.0))
        .gap_3()
        .child(
            text("YOUR VIM GRAMMAR")
                .text_size(px(28.0))
                .font_weight(FontWeight::BLACK)
                .letter_spacing(px(4.0))
                .text_color(hex(CREAM)),
        )
        .child(
            text(
                "Vim commands are sentences: a verb (an operator) and a noun (a motion or text object). \
                 Learn 8 verbs and 35 nouns and you know 280 commands. Lit cells are pairs you've used.",
            )
            .max_w(px(680.0))
            .text_center()
            .text_size(px(14.0))
            .line_height(px(20.0))
            .text_color(hexa(TEXT, 0.75)),
        )
        .child(
            text(format!("{count} of {total} used"))
                .font_family(MONO)
                .text_size(px(15.0))
                .text_color(hex(PRESENT)),
        )
        .child(
            scroll().flex_1().child(
                view()
                    .p(px(14.0))
                    .rounded(px(14.0))
                    .bg(hexa(PANEL, 0.7))
                    .border_1()
                    .border_color(hexa(LAVENDER, 0.18))
                    .child(verbs_header())
                    .children(rows),
            ),
        )
        .child(footer(ui))
}

/// The verbs across the top.
fn verbs_header() -> impl IntoElement {
    view()
        .flex_row()
        .items_end()
        .child(view().w(px(NOUN_W)))
        .children(grammar::VERBS.iter().map(|(verb, what)| {
            view()
                .w(px(CELL_W))
                .items_center()
                .child(
                    text(*verb)
                        .font_family(MONO)
                        .text_size(px(17.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(hex(CREAM)),
                )
                .child(text(*what).text_size(px(10.5)).text_color(hexa(TEXT, 0.55)))
        }))
}

/// A noun, and a cell per verb.
fn noun_row(used: Rc<BTreeSet<String>>, noun: &'static str, what: &'static str) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .py(px(1.0))
        .child(
            view()
                .w(px(NOUN_W))
                .flex_row()
                .gap_2()
                .items_center()
                .child(
                    text(noun)
                        .font_family(MONO)
                        .text_size(px(14.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(hex(GOLD))
                        .min_w(px(40.0)),
                )
                .child(text(what).text_size(px(11.5)).text_color(hexa(TEXT, 0.55))),
        )
        .children(grammar::VERBS.iter().map(move |&(verb, _)| {
            let lit = used.contains(&grammar::key(verb, noun));
            cell(word(verb, noun), lit)
        }))
}

/// The command a verb and a noun make, as typed.
fn word(verb: &str, noun: &str) -> String {
    match noun {
        "line" if verb.len() == 2 => format!("{verb}{}", &verb[1..]),
        "line" => format!("{verb}{verb}"),
        "f" | "t" | "F" | "T" => format!("{verb}{noun}x"),
        "/" => format!("{verb}/x"),
        n => format!("{verb}{n}"),
    }
}

/// A pair's cell: lit green once used.
fn cell(word: String, lit: bool) -> impl IntoElement {
    let (bg, border, color) = if lit {
        (hexa(PRESENT, 0.22), hexa(PRESENT, 0.6), hex(PRESENT))
    } else {
        (hexa(WHITE, 0.03), hexa(WHITE, 0.06), hexa(TEXT, 0.22))
    };
    view()
        .w(px(CELL_W - 4.0))
        .mx(px(2.0))
        .py(px(3.0))
        .rounded(px(5.0))
        .items_center()
        .bg(bg)
        .border_1()
        .border_color(border)
        .child(text(word).font_family(MONO).text_size(px(12.0)).text_color(color))
}

fn footer(ui: Ui) -> impl IntoElement {
    view()
        .flex_row()
        .gap_2()
        .items_center()
        .child(chip("Esc", CREAM))
        .child(legend("back to the map"))
        .child(view().w(px(10.0)))
        .child(
            text(move || {
                ui.tick.get();
                "Try a dim one on your next isle: it lights up the moment you use it.".to_string()
            })
            .text_size(px(12.5))
            .text_color(hexa(WHITE, 0.45)),
        )
}
