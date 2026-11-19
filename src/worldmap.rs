//! The world map: the Buffer Archipelago at sea, every isle on a route
//! from the Meadow to the Archive and beyond, its chapters grouped in
//! regions by topic. Pip's ferry sails it with h and l (isle by isle), j and
//! k (chapter by chapter) and J and K (region by region); isles past the
//! next one to clear lie in fog.

use super::*;

/// Map size, and the spacing of chapters and of isles within them.
const MAP_H: f32 = 600.0;
const CHAPTER_GAP: f32 = 330.0;
const ISLE_GAP: f32 = 84.0;
/// Open sea between two regions.
const REGION_GAP: f32 = 220.0;

/// Isle `i`'s chapter (its place in the campaign), its place in the
/// chapter, and how many isles the chapter has.
fn place(i: usize) -> (usize, usize, usize) {
    let start = chapter_start(i);
    let chapter = (0..=i).filter(|&k| k == chapter_start(k)).count() - 1;
    let n = (start..LEVELS.len()).take_while(|&k| LEVELS[k].chapter == LEVELS[i].chapter).count();
    (chapter, i - start, n)
}

/// Where isle `i` lies on the map.
pub(crate) fn isle_pos(i: usize) -> (f32, f32) {
    let (c, k, n) = place(i);
    let region = story::region_of(LEVELS[i].chapter);
    let cx = 240.0 + c as f32 * CHAPTER_GAP + region as f32 * REGION_GAP;
    let cy = if c % 2 == 0 { 250.0 } else { 350.0 };
    let spread = k as f32 - (n as f32 - 1.0) / 2.0;
    (cx + spread * ISLE_GAP, cy + if k % 2 == 0 { -40.0 } else { 40.0 })
}

fn map_width() -> f32 {
    isle_pos(LEVELS.len() - 1).0 + 260.0
}

/// Sails the ferry to the selected isle, and the camera after it.
pub(crate) fn follow(ui: Ui, motion: &Motion, animate: bool) {
    let Some(selected) = with_state(|s| match s.screen {
        Screen::Menu { selected } => Some(selected),
        _ => None,
    }) else {
        return;
    };
    let (x, y) = isle_pos(selected);
    let (vw, vh) = ui.stage.get_untracked();
    let cam_x = (vw / 2.0 - x).clamp((vw - map_width()).min(0.0), 0.0);
    let cam_y = (vh / 2.0 - y).clamp((vh - MAP_H).min(0.0), 0.0);
    // The ferry moors just off the isle's southern shore.
    let (fx, fy) = (x - 52.0, y + 14.0);
    if animate {
        let ease = Easing::Curve(Curve::Cubic, EaseMode::InOut);
        motion.hero_x.timing(fx, 520.0, ease);
        motion.hero_y.timing(fy, 520.0, ease);
        motion.cam_x.timing(cam_x, 600.0, ease);
        motion.cam_y.timing(cam_y, 600.0, ease);
    } else {
        motion.hero_x.set(fx);
        motion.hero_y.set(fy);
        motion.cam_x.set(cam_x);
        motion.cam_y.set(cam_y);
    }
}

/// The sea, the routes between the isles, and the isles themselves.
fn draw_map(best: &[Option<usize>], selected: usize) -> Painter {
    let w = map_width();
    let fog_from = frontier(best) + 2;
    let mut painter = Painter::new();
    // Waves.
    for row in 0..14 {
        let y = 20.0 + row as f32 * 44.0;
        let mut x = (row % 3) as f32 * 40.0;
        painter = painter.stroke_color(hexa(0xbfd8ff, 0.08)).line_width(1.5).line_cap(LineCap::Round);
        while x < w {
            painter = painter.begin_path().move_to(x, y).quadratic_curve_to(point(px(x + 9.0), px(y - 5.0)), point(px(x + 18.0), px(y))).stroke();
            x += 70.0 + noise(Pos { row, col: x as usize }) * 60.0;
        }
    }
    // Each region's waters, tinted, with its name and topic above.
    for (r, region) in story::REGIONS.iter().enumerate() {
        let isles: Vec<usize> = (0..LEVELS.len()).filter(|&i| story::region_of(LEVELS[i].chapter) == r).collect();
        let (Some(&first), Some(&last)) = (isles.first(), isles.last()) else { continue };
        let (x0, x1) = (isle_pos(first).0 - 110.0, isle_pos(last).0 + 110.0);
        let foggy = first >= fog_from;
        painter = painter
            .fill_color(hexa(region.tint, if foggy { 0.05 } else { 0.11 }))
            .begin_path()
            .round_rect(Bounds::new(point(px(x0), px(18.0)), size(px(x1 - x0), px(MAP_H - 36.0))), px(28.0))
            .fill()
            .stroke_color(hexa(region.tint, if foggy { 0.15 } else { 0.35 }))
            .line_width(1.5)
            .stroke()
            .font("800 15px sans-serif")
            .text_align(CanvasTextAlign::Center)
            .text_baseline(CanvasTextBaseline::Middle)
            .fill_color(hexa(0xfff3b0, if foggy { 0.35 } else { 0.85 }))
            .fill_text(&region.name.to_uppercase(), (x0 + x1) / 2.0, 46.0)
            .font("italic 12px sans-serif")
            .fill_color(hexa(0xece6ff, if foggy { 0.3 } else { 0.6 }))
            .fill_text(region.topic, (x0 + x1) / 2.0, 66.0);
    }
    // The route, golden where it's been sailed.
    for i in 1..LEVELS.len() {
        let ((x0, y0), (x1, y1)) = (isle_pos(i - 1), isle_pos(i));
        let sailed = best[i - 1].is_some();
        painter = painter
            .stroke_color(if sailed { hexa(0xffd76b, 0.6) } else { hexa(0xffffff, 0.18) })
            .line_width(2.0)
            .line_dash(&[px(5.0), px(7.0)])
            .begin_path()
            .move_to(x0, y0 + 10.0)
            .quadratic_curve_to(point(px((x0 + x1) / 2.0), px((y0 + y1) / 2.0 + 40.0)), point(px(x1), px(y1 + 10.0)))
            .stroke()
            .line_dash(&[]);
    }
    // Chapter names over their isles.
    for i in 0..LEVELS.len() {
        if i != chapter_start(i) {
            continue;
        }
        let (c, _, n) = place(i);
        let (x, _) = isle_pos(i);
        let cx = x + (n as f32 - 1.0) / 2.0 * ISLE_GAP;
        let y = if c % 2 == 0 { 140.0 } else { 240.0 };
        let foggy = i >= fog_from;
        painter = painter
            .font("700 12px sans-serif")
            .text_align(CanvasTextAlign::Center)
            .text_baseline(CanvasTextBaseline::Middle)
            .fill_color(hexa(0xc9b8ff, if foggy { 0.3 } else { 0.75 }))
            .fill_text(&LEVELS[i].chapter.to_uppercase(), cx, y);
    }
    for i in 0..LEVELS.len() {
        painter = draw_isle(painter, i, best[i], i == selected, i >= fog_from && best[i].is_none());
    }
    painter
}

fn draw_isle(mut painter: Painter, i: usize, best: Option<usize>, selected: bool, foggy: bool) -> Painter {
    let (x, y) = isle_pos(i);
    let level = &LEVELS[i];
    let theme = story::theme(level.chapter);
    let boss = i + 1 == LEVELS.len() || LEVELS[i + 1].chapter != level.chapter;
    let r = if boss { 30.0 } else { 24.0 };
    if selected {
        painter = painter
            .fill_style(
                CanvasGradient::radial(point(px(x), px(y)), px(4.0), point(px(x), px(y)), px(r * 2.6))
                    .add_color_stop(percentage(0.0), hexa(0xfff3b0, 0.35))
                    .add_color_stop(percentage(1.0), hexa(0xfff3b0, 0.0)),
            )
            .begin_path()
            .arc(x, y, r * 2.6, 0.0, TAU)
            .fill();
    }
    painter = painter
        // Shallows, beach, grass.
        .fill_color(hexa(0x7fd6d0, 0.18))
        .begin_path()
        .ellipse_at(x, y + 6.0, r + 12.0, r * 0.62 + 8.0)
        .fill()
        .fill_color(hex(0xd9c48f))
        .begin_path()
        .ellipse_at(x, y + 4.0, r + 3.0, r * 0.62 + 2.0)
        .fill()
        .fill_color(shade(theme.grass, if foggy { 0.5 } else { 1.0 }))
        .begin_path()
        .ellipse_at(x, y, r, r * 0.58)
        .fill()
        .fill_color(shade(theme.stone, 0.9))
        .begin_path()
        .ellipse_at(x - r * 0.25, y - 4.0, r * 0.35, r * 0.22)
        .fill();
    if boss && best.is_none() && !foggy {
        // A boss's lair: a dark spire with a red eye.
        painter = painter
            .fill_color(hex(0x2a1a2a))
            .begin_path()
            .move_to(x + 4.0, y - 2.0)
            .line_to(x + 11.0, y - 30.0)
            .line_to(x + 18.0, y - 2.0)
            .close_path()
            .fill()
            .fill_color(hex(0xff4d4d))
            .begin_path()
            .arc(x + 11.0, y - 16.0, 2.2, 0.0, TAU)
            .fill();
    }
    if best.is_some() {
        // A flag on the cleared.
        painter = painter
            .stroke_color(hex(0xf6f1e3))
            .line_width(1.5)
            .begin_path()
            .move_to(x + 8.0, y - 2.0)
            .line_to(x + 8.0, y - 26.0)
            .stroke()
            .fill_color(hex(0xffd76b))
            .begin_path()
            .move_to(x + 8.0, y - 26.0)
            .line_to(x + 22.0, y - 21.0)
            .line_to(x + 8.0, y - 16.0)
            .close_path()
            .fill();
    }
    if foggy {
        painter = painter.fill_color(hexa(0xd8d4e8, 0.55));
        for (dx, dy, rr) in [(-16.0, -6.0, 15.0), (0.0, -12.0, 18.0), (16.0, -4.0, 14.0), (-4.0, 4.0, 16.0), (10.0, 6.0, 12.0)] {
            painter = painter.begin_path().arc(x + dx, y + dy, rr, 0.0, TAU).fill();
        }
    }
    // Name and stars below.
    let name = if foggy { "???".to_string() } else { format!("{}. {}", i + 1, level.title) };
    painter = painter
        .font(if selected { "700 13px sans-serif" } else { "600 12px sans-serif" })
        .text_align(CanvasTextAlign::Center)
        .text_baseline(CanvasTextBaseline::Middle)
        .fill_color(hexa(0x000000, 0.5))
        .fill_text(&name, x + 1.0, y + r * 0.62 + 21.0)
        .fill_color(if selected { hex(0xfff3b0) } else { hexa(0xece6ff, if foggy { 0.4 } else { 0.85 }) })
        .fill_text(&name, x, y + r * 0.62 + 20.0);
    if let Some(n) = best {
        let stars = game::stars(n, level.par()) as usize;
        painter = painter
            .font("12px sans-serif")
            .fill_color(hex(0xffd76b))
            .fill_text(&format!("{}{}", "★".repeat(stars), "☆".repeat(3 - stars)), x, y + r * 0.62 + 36.0);
    }
    painter
}

/// Pip's ferry: a little boat with a sail, and Pip the owl at the stern.
fn draw_ferry(motion: &Motion) -> Painter {
    Painter::new()
        .save()
        .translate(&motion.hero_x, &motion.hero_y)
        .translate(0.0, &motion.bob)
        .fill_color(hexa(0x000000, 0.25))
        .begin_path()
        .ellipse_at(0.0, 9.0, 22.0, 4.0)
        .fill()
        .fill_color(hex(0x86603a))
        .begin_path()
        .move_to(-20.0, 0.0)
        .line_to(20.0, 0.0)
        .line_to(13.0, 9.0)
        .line_to(-13.0, 9.0)
        .close_path()
        .fill()
        .stroke_color(hex(0xf6f1e3))
        .line_width(1.5)
        .begin_path()
        .move_to(0.0, 0.0)
        .line_to(0.0, -30.0)
        .stroke()
        .fill_color(hex(0xf6f1e3))
        .begin_path()
        .move_to(2.0, -28.0)
        .line_to(17.0, -6.0)
        .line_to(2.0, -6.0)
        .close_path()
        .fill()
        // Pip.
        .fill_color(hex(0x9a7a5a))
        .begin_path()
        .ellipse_at(-11.0, -6.0, 6.0, 7.0)
        .fill()
        .fill_color(hex(0xffffff))
        .begin_path()
        .arc(-13.5, -8.0, 2.3, 0.0, TAU)
        .arc(-8.5, -8.0, 2.3, 0.0, TAU)
        .fill()
        .fill_color(hex(0x1a1430))
        .begin_path()
        .arc(-13.5, -8.0, 1.1, 0.0, TAU)
        .arc(-8.5, -8.0, 1.1, 0.0, TAU)
        .fill()
        .fill_color(hex(0xffb84c))
        .begin_path()
        .move_to(-12.0, -5.5)
        .line_to(-10.0, -5.5)
        .line_to(-11.0, -3.5)
        .close_path()
        .fill()
        // The wisp, riding along.
        .fill_color(hex(0xfffdf5))
        .begin_path()
        .arc(8.0, -3.0, 3.5, 0.0, TAU)
        .fill()
        .restore()
}

pub(crate) fn menu_screen(ui: Ui, motion: Rc<Motion>) -> impl IntoElement {
    let ferry = motion.clone();
    let layout = motion.clone();
    let stage = view()
        .flex_1()
        .overflow_hidden()
        .rounded(px(16.0))
        .bg(hexa(0x0a2a4a, 0.55))
        .border_1()
        .border_color(hexa(0xc9b8ff, 0.14))
        .on_layout(move |b: &Bounds<Points>| {
            ui.stage.set((b.size.width.0, b.size.height.0));
            follow(ui, &layout, false);
        })
        .child(
            view()
                .absolute()
                .top_0()
                .left_0()
                .w(px(map_width()))
                .h(px(MAP_H))
                .bind_translate_x(&motion.cam_x)
                .bind_translate_y(&motion.cam_y)
                .child(
                    canvas(move || {
                        ui.tick.get();
                        let (best, selected) = with_state(|s| {
                            let selected = match s.screen {
                                Screen::Menu { selected } => selected,
                                _ => 0,
                            };
                            (s.best.clone(), selected)
                        });
                        draw_map(&best, selected)
                    })
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .bottom_0(),
                )
                .child(canvas(move || draw_ferry(&ferry)).absolute().top_0().left_0().right_0().bottom_0()),
        );

    let info = dynamic(move || {
        ui.tick.get();
        let (selected, best, time, fog_from, toast, free_best) = with_state(|s| {
            let selected = match s.screen {
                Screen::Menu { selected } => selected,
                _ => 0,
            };
            (selected, s.best[selected], s.times[selected], frontier(&s.best) + 2, s.toast.clone(), s.free_best[selected])
        });
        let level = &LEVELS[selected];
        let foggy = selected >= fog_from && best.is_none();
        let boss = selected + 1 == LEVELS.len() || LEVELS[selected + 1].chapter != level.chapter;
        let score = match best {
            Some(n) => {
                let stars = game::stars(n, level.par()) as usize;
                format!("{}{}  {n} keys, par {}", "★".repeat(stars), "☆".repeat(3 - stars), level.par())
            }
            None => format!("☆☆☆  par {}", level.par()),
        };
        Some(
            view()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap_4()
                .p(px(14.0))
                .rounded(px(14.0))
                .bg(hexa(0x0b0820, 0.7))
                .border_1()
                .border_color(hexa(0xc9b8ff, 0.18))
                .child(
                    view()
                        .flex_1()
                        .min_w(px(260.0))
                        .gap_1()
                        .child(
                            text(format!("{}  ·  {}", story::REGIONS[story::region_of(level.chapter)].name, level.chapter))
                                .text_size(px(12.0))
                                .letter_spacing(px(1.5))
                                .text_color(hexa(0xc9b8ff, 0.75)),
                        )
                        .child(
                            text(if foggy { format!("{}. Uncharted waters", selected + 1) } else { format!("{}. {}", selected + 1, level.title) })
                                .text_size(px(22.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(hex(0xfff3b0)),
                        )
                        .child(
                            text(if foggy {
                                "Clear the isles before it to lift the fog. You can still sail there.".to_string()
                            } else {
                                level.story.to_string()
                            })
                            .text_size(px(13.0))
                            .line_height(px(18.0))
                            .italic()
                            .text_color(hexa(0xece6ff, 0.7))
                            .number_of_lines(2),
                        )
                        .child(boss.then(|| {
                            let name = story::chapter(level.chapter).map_or("", |c| c.boss);
                            text(format!("⚔ Boss: {name}")).text_size(px(13.0)).font_weight(FontWeight::BOLD).text_color(hex(0xff9a6b))
                        }))
                        .child((!toast.is_empty()).then(|| text(toast).text_size(px(13.0)).text_color(hex(0xffb86b)))),
                )
                .child(
                    view()
                        .gap_1()
                        .items_end()
                        .child(view().flex_row().gap_1().children(level.unlock.chars().take(8).map(|c| chip(game::key_name(c), 0xfff3b0))))
                        .child(text(score).font_family(MONO).text_size(px(14.0)).text_color(hex(0xffd76b)))
                        .child(time.map(|t| text(format!("⏱ best {}", clock(t as f64 * 100.0))).font_family(MONO).text_size(px(12.5)).text_color(hexa(0xece6ff, 0.6))))
                        .child(free_best.map(|b| text(format!("free play best {b} keys")).font_family(MONO).text_size(px(12.5)).text_color(hexa(GEM, 0.8))))
                        .child((best.is_some()).then(|| {
                            view().flex_row().gap_1().items_center().child(chip("f", GEM)).child(text("free play: every key").text_size(px(12.0)).text_color(hexa(0xece6ff, 0.6)))
                        })),
                ),
        )
    });

    let summary = text(move || {
        ui.tick.get();
        with_state(|s| {
            let cleared = s.best.iter().filter(|b| b.is_some()).count();
            let stars: usize = s.best.iter().enumerate().filter_map(|(i, b)| b.map(|n| game::stars(n, LEVELS[i].par()) as usize)).sum();
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
    .text_color(hexa(0xece6ff, 0.7));

    let keys = view()
        .flex_row()
        .flex_wrap()
        .justify_center()
        .items_center()
        .gap_2()
        .child(chip("h l", 0xfff3b0))
        .child(text("isle").text_color(hexa(0xffffff, 0.6)).mr(px(6.0)))
        .child(chip("j k", 0xfff3b0))
        .child(text("chapter").text_color(hexa(0xffffff, 0.6)).mr(px(6.0)))
        .child(chip("J K", 0xfff3b0))
        .child(text("region").text_color(hexa(0xffffff, 0.6)).mr(px(6.0)))
        .child(chip("Enter", 0xfff3b0))
        .child(text("sail in").text_color(hexa(0xffffff, 0.6)).mr(px(6.0)))
        .child(chip("d", GEM))
        .child(text("daily tide").text_color(hexa(0xffffff, 0.6)).mr(px(6.0)))
        .child(chip("r", GEM))
        .child(text("review drills").text_color(hexa(0xffffff, 0.6)).mr(px(6.0)))
        .child(chip("v", GEM))
        .child(text("your grammar").text_color(hexa(0xffffff, 0.6)).mr(px(6.0)))
        .child(chip("m", 0xfff3b0))
        .child(
            text(move || {
                ui.tick.get();
                if audio::muted() { "sound off" } else { "sound on" }
            })
            .text_color(hexa(0xffffff, 0.6)),
        );

    view()
        .flex_1()
        .p(px(18.0))
        .gap_3()
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .items_end()
                .gap_4()
                .child(text("HJKL QUEST").text_size(px(34.0)).font_weight(FontWeight::BLACK).letter_spacing(px(5.0)).text_color(hex(0xfff3b0)))
                .child(text("The Buffer Archipelago: learn vim one isle at a time.").text_size(px(14.0)).text_color(hexa(0xece6ff, 0.7)).mb(px(5.0)))
                .child(view().flex_1())
                .child(summary),
        )
        .child(stage)
        .child(info)
        .child(keys)
}
