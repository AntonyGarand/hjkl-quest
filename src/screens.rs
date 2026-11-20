//! Smaller pieces of the screens: the grammar grid, the boss banner on a
//! chapter's last isle, and the hint card for a half-typed command.

use super::{
    Element, EllipseAt, FluentBuilder, FontWeight, IntoElement, LEVELS, MONO, Mode, Motion, PRESENT, Painter,
    ParentElement, Rc, Styled, TAU, Ui, canvas, chip, dynamic, grammar, hex, hexa, hints, is_boss, px,
    scroll, session_value, shade, story, text, view, with_state, work,
};

/// The isle where text objects are taught: hints offer them from there on.
fn objects_taught(index: usize) -> bool {
    LEVELS
        .iter()
        .position(|l| l.title == "Inside Out")
        .is_some_and(|i| index >= i)
}

/// What can follow the keys typed so far, floating over the board.
pub(crate) fn hint_card(ui: Ui) -> impl IntoElement {
    dynamic(move || {
        ui.tick.get();
        let hint = session_value(|s| {
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
        })??;
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
                            .text_color(hex(0xffd76b)),
                    ),
                )
                .child(
                    text(what)
                        .text_size(px(12.0))
                        .text_color(hexa(0xece6ff, 0.8))
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
                .bg(hexa(0x0b0820, 0.92))
                .border_1()
                .border_color(hexa(0xffb86b, 0.5))
                .child(
                    text(hint.title)
                        .font_family(MONO)
                        .text_size(px(13.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(hex(0xffb86b))
                        .mb(px(4.0)),
                )
                .child(view().flex_row().flex_wrap().gap_1().children(rows))
                .child(
                    text("Esc cancels")
                        .text_size(px(11.0))
                        .text_color(hexa(0xffffff, 0.4))
                        .mt(px(4.0)),
                ),
        )
    })
}

/// A chapter's boss, perched over its last isle: its name, its health
/// (what's left to do), and a flinch when a command hurts it.
pub(crate) fn boss_banner(ui: Ui, motion: Rc<Motion>) -> impl IntoElement {
    dynamic(move || {
        ui.scene.get();
        let (boss, color) = session_value(|s| {
            let chapter = story::chapter(s.level().chapter)?;
            is_boss(s).then_some((chapter.boss, chapter.theme.stone))
        })??;
        let flinch = motion.boss.mul(10.0);
        let health = move || {
            ui.tick.get();
            let left = session_value(work).unwrap_or(0);
            let full = with_state(|s| s.work_start).max(left).max(1);
            left as f32 / full as f32
        };
        Some(
            view()
                .absolute()
                .top(px(12.0))
                .left(px(12.0))
                .flex_row()
                .items_center()
                .gap_3()
                .px(px(12.0))
                .py(px(8.0))
                .rounded(px(12.0))
                .bg(hexa(0x05030f, 0.7))
                .border_1()
                .border_color(hexa(0xff9a6b, 0.45))
                .child(
                    canvas(move || draw_monster(color))
                        .w(px(54.0))
                        .h(px(46.0))
                        .bind_translate_x(&flinch),
                )
                .child(
                    view()
                        .gap_1()
                        .child(
                            text(boss)
                                .text_size(px(14.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(hex(0xff9a6b)),
                        )
                        .child(
                            view()
                                .w(px(160.0))
                                .h(px(8.0))
                                .rounded(px(4.0))
                                .bg(hexa(0xffffff, 0.12))
                                .overflow_hidden()
                                .child(
                                    view()
                                        .h_full()
                                        .rounded(px(4.0))
                                        .bg(hex(0xff6b5a))
                                        .w(move || px(160.0 * health().clamp(0.0, 1.0))),
                                ),
                        )
                        .child(
                            text(move || {
                                let h = health();
                                if h <= 0.0 {
                                    "defeated!".to_string()
                                } else {
                                    format!("{:.0}% health: every crystal, bug or fix is a hit", h * 100.0)
                                }
                            })
                            .text_size(px(11.0))
                            .text_color(hexa(0xece6ff, 0.6)),
                        ),
                ),
        )
    })
}

/// A chapter boss: a horned blob with glowing eyes, tinted like the
/// chapter's stone.
fn draw_monster(color: u32) -> Painter {
    let (cx, cy) = (27.0, 26.0);
    Painter::new()
        .fill_color(hexa(0x000000, 0.3))
        .begin_path()
        .ellipse_at(cx, 42.0, 18.0, 3.5)
        .fill()
        .fill_color(hex(0xe8dcc0))
        .begin_path()
        .move_to(cx - 14.0, cy - 8.0)
        .line_to(cx - 20.0, cy - 22.0)
        .line_to(cx - 8.0, cy - 13.0)
        .close_path()
        .move_to(cx + 14.0, cy - 8.0)
        .line_to(cx + 20.0, cy - 22.0)
        .line_to(cx + 8.0, cy - 13.0)
        .close_path()
        .fill()
        .fill_color(shade(color, 1.2))
        .begin_path()
        .ellipse_at(cx, cy, 19.0, 15.0)
        .fill()
        .fill_color(shade(color, 0.8))
        .begin_path()
        .ellipse_at(cx, cy + 7.0, 15.0, 7.0)
        .fill()
        .fill_color(hex(0xffe066))
        .begin_path()
        .arc(cx - 7.0, cy - 3.0, 3.6, 0.0, TAU)
        .arc(cx + 7.0, cy - 3.0, 3.6, 0.0, TAU)
        .fill()
        .fill_color(hex(0x1a1430))
        .begin_path()
        .arc(cx - 6.0, cy - 3.0, 1.6, 0.0, TAU)
        .arc(cx + 8.0, cy - 3.0, 1.6, 0.0, TAU)
        .fill()
        .fill_color(hex(0xffffff))
        .begin_path()
        .move_to(cx - 6.0, cy + 5.0)
        .line_to(cx - 3.0, cy + 9.0)
        .line_to(cx, cy + 5.0)
        .line_to(cx + 3.0, cy + 9.0)
        .line_to(cx + 6.0, cy + 5.0)
        .close_path()
        .fill()
}

/// The grid of verbs and nouns: every operator and every motion or text
/// object, lit where the player has used the pair.
pub(crate) fn grammar_screen(ui: Ui) -> impl IntoElement {
    let used = with_state(|s| s.grammar.clone());
    let total = grammar::VERBS.len() * grammar::NOUNS.len();
    let cell_w = 64.0;
    let header =
        view()
            .flex_row()
            .items_end()
            .child(view().w(px(190.0)))
            .children(grammar::VERBS.iter().map(move |(verb, what)| {
                view()
                    .w(px(cell_w))
                    .items_center()
                    .child(
                        text(*verb)
                            .font_family(MONO)
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(hex(0xfff3b0)),
                    )
                    .child(text(*what).text_size(px(10.5)).text_color(hexa(0xece6ff, 0.55)))
            }));
    let rows = grammar::NOUNS.iter().map(move |(noun, what)| {
        let used = used.clone();
        view()
            .flex_row()
            .items_center()
            .py(px(1.0))
            .child(
                view()
                    .w(px(190.0))
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .child(
                        text(*noun)
                            .font_family(MONO)
                            .text_size(px(14.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(hex(0xffd76b))
                            .min_w(px(40.0)),
                    )
                    .child(text(*what).text_size(px(11.5)).text_color(hexa(0xece6ff, 0.55))),
            )
            .children(grammar::VERBS.iter().map(move |(verb, _)| {
                let lit = used.contains(&grammar::key(verb, noun));
                let word = match *noun {
                    "line" if verb.len() == 2 => format!("{verb}{}", &verb[1..]),
                    "line" => format!("{verb}{verb}"),
                    "f" | "t" | "F" | "T" => format!("{verb}{noun}x"),
                    "/" => format!("{verb}/x"),
                    n => format!("{verb}{n}"),
                };
                view()
                    .w(px(cell_w - 4.0))
                    .mx(px(2.0))
                    .py(px(3.0))
                    .rounded(px(5.0))
                    .items_center()
                    .bg(if lit {
                        hexa(PRESENT, 0.22)
                    } else {
                        hexa(0xffffff, 0.03)
                    })
                    .border_1()
                    .border_color(if lit {
                        hexa(PRESENT, 0.6)
                    } else {
                        hexa(0xffffff, 0.06)
                    })
                    .child(
                        text(word)
                            .font_family(MONO)
                            .text_size(px(12.0))
                            .text_color(if lit { hex(PRESENT) } else { hexa(0xece6ff, 0.22) }),
                    )
            }))
    });
    let count = with_state(|s| s.grammar.len());
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
                .text_color(hex(0xfff3b0)),
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
            .text_color(hexa(0xece6ff, 0.75)),
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
                    .bg(hexa(0x0b0820, 0.7))
                    .border_1()
                    .border_color(hexa(0xc9b8ff, 0.18))
                    .child(header)
                    .children(rows),
            ),
        )
        .child(
            view()
                .flex_row()
                .gap_2()
                .items_center()
                .child(chip("Esc", 0xfff3b0))
                .child(text("back to the map").text_color(hexa(0xffffff, 0.6)))
                .child(view().w(px(10.0)))
                .child(
                    text(move || {
                        ui.tick.get();
                        "Try a dim one on your next isle: it lights up the moment you use it.".to_string()
                    })
                    .text_size(px(12.5))
                    .text_color(hexa(0xffffff, 0.45)),
                ),
        )
}
