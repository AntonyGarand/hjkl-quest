//! The intro shown before an isle the first time (and on `:h`): what it
//! teaches, and each command played out on a small looping example.

use super::*;

pub(crate) fn intro_modal(ui: Ui) -> impl IntoElement {
    dynamic(move || {
        ui.tick.get();
        let (index, level, boss) = with_state(|s| match &s.screen {
            Screen::Play(p) if s.intro => Some((p.index, p.level(), is_boss(p))),
            _ => None,
        })?;
        // Pip speaks rowing up to a chapter; a boss guards its last isle.
        let chapter = story::chapter(level.chapter);
        let first = index == chapter_start(index);
        let speech = chapter.and_then(|c| {
            if first {
                Some(("🦉 Pip", c.pip))
            } else if boss {
                Some((c.boss, c.taunt))
            } else {
                None
            }
        });
        let narrow = ui.window.get_untracked().0 < 760.0;
        let max_h = (ui.window.get_untracked().1 - 40.0).max(300.0);
        let lessons = level.teaches.iter().map(|(keys, what)| {
            view()
                .flex_row()
                .items_start()
                .gap_3()
                .child(view().min_w(px(110.0)).items_start().child(chip(*keys, 0xfff3b0)))
                .child(text(*what).text_size(px(14.0)).line_height(px(20.0)).text_color(hexa(0xece6ff, 0.88)).flex_1())
        });
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
            goal.push_str(" Sentries march a row up or down with every command you finish: meet one and you're back at the start.");
        }
        if !level.frozen().is_empty() {
            goal.push_str(" The icy lines are frozen: no edit may change them.");
        }
        if !level.banned().is_empty() {
            let keys: Vec<String> = level.banned().chars().map(|c| c.to_string()).collect();
            goal.push_str(&format!(" Banned here: {}.", keys.join(" ")));
        }
        let cards = examples::examples(level.title).iter().map(move |&(text, keys)| example_card(ui, text, keys));
        let heading = |label: &'static str| text(label).text_size(px(11.0)).letter_spacing(px(1.5)).text_color(hexa(0xc9b8ff, 0.7)).mt(px(6.0));
        let start = view()
            .px(px(18.0))
            .py(px(8.0))
            .rounded(px(10.0))
            .bg(hex(0xfff3b0))
            .cursor_pointer()
            .on_press(move || {
                with_state(|s| s.close_intro());
                ui.tick.update(|t| *t += 1);
            })
            .child(text("Start").font_weight(FontWeight::BOLD).text_color(hex(0x1a1430)));
        let footer = view()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap_2()
            .mt(px(6.0))
            .child(chip("Enter", 0xfff3b0))
            .child(text("start the isle").text_color(hexa(0xffffff, 0.65)).mr(px(10.0)))
            .child(chip(":h", 0xfff3b0))
            .child(text("see this again").text_color(hexa(0xffffff, 0.65)))
            .child(view().flex_1())
            .child(start);
        let body = view()
            .p(px(24.0))
            .gap_3()
            .child(text(level.chapter).text_size(px(12.0)).letter_spacing(px(1.5)).text_color(hexa(0xc9b8ff, 0.75)))
            .child(text(format!("{}. {}", index + 1, level.title)).text_size(px(26.0)).font_weight(FontWeight::BOLD).text_color(hex(0xfff3b0)))
            .child(text(level.story).text_size(px(14.0)).line_height(px(20.0)).italic().text_color(hexa(0xece6ff, 0.75)))
            .child(speech.map(|(who, line)| {
                let boss_speaking = !who.starts_with('🦉');
                let color = if boss_speaking { 0xff9a6b } else { 0x9fd8ff };
                view()
                    .flex_row()
                    .gap_3()
                    .items_start()
                    .p(px(12.0))
                    .rounded(px(12.0))
                    .bg(hexa(color, 0.10))
                    .border_1()
                    .border_color(hexa(color, 0.4))
                    .child(text(who).text_size(px(13.0)).font_weight(FontWeight::BOLD).text_color(hex(color)).min_w(px(80.0)))
                    .child(text(format!("“{line}”")).text_size(px(14.0)).line_height(px(20.0)).text_color(hexa(0xece6ff, 0.9)).flex_1())
            }))
            .child(heading("YOU WILL LEARN"))
            .child(view().gap_2().children(lessons))
            .child(heading("SEE IT WORK"))
            .child(view().flex_row().flex_wrap().gap_3().children(cards))
            .child(text(goal).text_size(px(13.0)).line_height(px(18.0)).text_color(hex(GEM)).mt(px(4.0)))
            .child(
                text({
                    let reqs = lessons::requirements(level.title);
                    let must: Vec<&str> = reqs.iter().filter(|(_, p)| !lessons::optional(p)).map(|(l, _)| *l).collect();
                    let may: Vec<&str> = reqs.iter().filter(|(_, p)| lessons::optional(p)).map(|(l, _)| *l).collect();
                    let mut line = format!("To clear it, use: {}. Reaching the goal without them doesn't count.", must.join(" · "));
                    if !may.is_empty() {
                        line.push_str(&format!(" Worth a try too: {}.", may.join(" · ")));
                    }
                    line
                })
                .text_size(px(13.0))
                .line_height(px(18.0))
                .text_color(hex(0xffd76b)),
            )
            .child(footer);
        Some(
            view()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .items_center()
                .justify_center()
                .bg(hexa(0x05030f, 0.74))
                .child(
                    view()
                        .w_full()
                        .max_w(px(if narrow { 520.0 } else { 760.0 }))
                        .max_h(px(max_h))
                        .rounded(px(18.0))
                        .bg(hexa(0x171032, 0.97))
                        .border_1()
                        .border_color(hexa(0xfff3b0, 0.35))
                        .child(scroll().child(body)),
                ),
        )
    })
}

/// How a character of an example is drawn.
#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Plain,
    Selected,
    Match,
    Cursor,
}

/// One example, played on a loop: the keys (those typed so far lit up),
/// the text with the cursor and any selection, and the mode or command line.
fn example_card(ui: Ui, text_with_cursor: &'static str, keys: &'static str) -> impl IntoElement {
    let frames = Rc::new(examples::frames(text_with_cursor, keys));
    let all = vim::parse_keys(keys);
    view()
        .w(px(220.0))
        .p(px(10.0))
        .rounded(px(10.0))
        .bg(hexa(0x000000, 0.35))
        .border_1()
        .border_color(hexa(0xc9b8ff, 0.14))
        .child(dynamic(move || {
            let n = frames.len();
            // Hold the start a beat, and the end a few.
            let i = (ui.beat.get() as usize % (n + 4)).saturating_sub(1).min(n - 1);
            let frame = &frames[i];
            let e = &frame.editor;
            let insert = matches!(e.mode, Mode::Insert | Mode::Replace);
            let accent = mode_color(e);
            let lit = e.search_highlights();
            let rows = e.lines.iter().enumerate().map(|(row, line)| {
                let mut cells: Vec<(String, Cell)> = Vec::new();
                let width = if row == e.cursor.row && e.cursor.col >= line.len() { line.len() + 1 } else { line.len().max(1) };
                for col in 0..width {
                    let p = Pos { row, col };
                    let c = line.get(col).copied().unwrap_or(' ');
                    let cell = if p == e.cursor && !matches!(e.mode, Mode::Command { .. }) {
                        Cell::Cursor
                    } else if col < line.len() && is_selected(e, p) {
                        Cell::Selected
                    } else if lit.iter().any(|(m, len)| m.row == row && col >= m.col && col < m.col + len) {
                        Cell::Match
                    } else {
                        Cell::Plain
                    };
                    let c = if c == ' ' { '\u{a0}' } else { c };
                    match cells.last_mut() {
                        Some((t, k)) if *k == cell && cell != Cell::Cursor => t.push(c),
                        _ => cells.push((c.to_string(), cell)),
                    }
                }
                view().flex_row().children(cells.into_iter().map(move |(t, cell)| {
                    let dark = cell == Cell::Cursor && !insert;
                    let glyph = text(t)
                        .font_family(MONO)
                        .text_size(px(14.0))
                        .line_height(px(19.0))
                        .text_color(if dark { hex(0x1a1430) } else { hex(0xece6ff) });
                    let base = view().child(glyph);
                    match cell {
                        Cell::Cursor if insert => base.border_l_1().border_color(hex(accent)),
                        Cell::Cursor => base.bg(hex(accent)).rounded(px(2.0)),
                        Cell::Selected => base.bg(hexa(SELECT, 0.75)),
                        Cell::Match => base.bg(hexa(0xffd76b, 0.45)),
                        Cell::Plain => base,
                    }
                }))
            });
            let status = match e.mode {
                _ if frame.message.is_some() && !matches!(e.mode, Mode::Command { .. }) => frame.message.clone().unwrap_or_default(),
                Mode::Command { .. } => format!("{}{}", e.cmd_prefix(), e.cmdline),
                Mode::Insert => "-- INSERT --".into(),
                Mode::Replace => "-- REPLACE --".into(),
                Mode::Visual { line: false } => "-- VISUAL --".into(),
                Mode::Visual { line: true } => "-- VISUAL LINE --".into(),
                Mode::Block => "-- VISUAL BLOCK --".into(),
                Mode::Confirm => e.confirm_prompt(),
                Mode::Normal => match e.recording_macro() {
                    Some(r) => format!("recording @{r}"),
                    None => "\u{a0}".into(),
                },
            };
            Some(
                view()
                    .gap_2()
                    .child(
                        view()
                            .flex_row()
                            .flex_wrap()
                            .child(text(format_keys(&all[..frame.typed])).font_family(MONO).text_size(px(14.0)).font_weight(FontWeight::BOLD).text_color(hex(0xffd76b)))
                            .child(text(format_keys(&all[frame.typed..])).font_family(MONO).text_size(px(14.0)).text_color(hexa(0xffffff, 0.3))),
                    )
                    .child(view().p(px(6.0)).rounded(px(6.0)).bg(hexa(0x05030f, 0.6)).children(rows))
                    .child(text(status).font_family(MONO).text_size(px(11.5)).text_color(hex(accent))),
            )
        }))
}
