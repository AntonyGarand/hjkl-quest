//! The map's painters: the sea and its regions, the route, the isles, and
//! Pip's ferry.

use std::f32::consts::TAU;

use nsh_core::canvas::{CanvasGradient, CanvasTextAlign, CanvasTextBaseline, LineCap};
use nsh_core::prelude::*;

use super::{ISLE_GAP, MAP_H, isle_pos, map_width, place};
use crate::game;
use crate::levels::LEVELS;
use crate::story;
use crate::ui::colors::{
    BLACK, CREAM, GLYPH, GOLD, INK, LAVENDER, TEXT, WHITE, WOOD, hex, hexa, noise, shade,
};
use crate::ui::draw::EllipseAt;
use crate::ui::motion::Motion;
use crate::ui::state::{chapter_start, frontier, is_boss_isle};
use crate::vim::Pos;

/// The sea, the routes between the isles, and the isles themselves.
pub(super) fn draw_map(best: &[Option<usize>], selected: usize) -> Painter {
    let fog_from = frontier(best) + 2;
    let mut painter = draw_waves(Painter::new());
    painter = draw_regions(painter, fog_from);
    painter = draw_route(painter, best);
    painter = draw_chapter_names(painter, fog_from);
    for (i, &isle_best) in best.iter().enumerate().take(LEVELS.len()) {
        let foggy = i >= fog_from && isle_best.is_none();
        painter = draw_isle(painter, i, isle_best, i == selected, foggy);
    }
    painter
}

fn draw_waves(mut painter: Painter) -> Painter {
    let w = map_width();
    for row in 0..14 {
        let y = 20.0 + row as f32 * 44.0;
        let mut x = (row % 3) as f32 * 40.0;
        painter = painter
            .stroke_color(hexa(0xbfd8ff, 0.08))
            .line_width(1.5)
            .line_cap(LineCap::Round);
        while x < w {
            painter = painter
                .begin_path()
                .move_to(x, y)
                .quadratic_curve_to(point(px(x + 9.0), px(y - 5.0)), point(px(x + 18.0), px(y)))
                .stroke();
            x += 70.0 + noise(Pos { row, col: x as usize }) * 60.0;
        }
    }
    painter
}

/// Each region's waters, tinted, with its name and topic above.
fn draw_regions(mut painter: Painter, fog_from: usize) -> Painter {
    for (r, region) in story::REGIONS.iter().enumerate() {
        let isles: Vec<usize> = (0..LEVELS.len())
            .filter(|&i| story::region_of(LEVELS[i].chapter) == r)
            .collect();
        let (Some(&first), Some(&last)) = (isles.first(), isles.last()) else {
            continue;
        };
        let (x0, x1) = (isle_pos(first).0 - 110.0, isle_pos(last).0 + 110.0);
        let foggy = first >= fog_from;
        let fade = |clear: f32, fogged: f32| if foggy { fogged } else { clear };
        painter = painter
            .fill_color(hexa(region.tint, fade(0.11, 0.05)))
            .begin_path()
            .round_rect(
                Bounds::new(point(px(x0), px(18.0)), size(px(x1 - x0), px(MAP_H - 36.0))),
                px(28.0),
            )
            .fill()
            .stroke_color(hexa(region.tint, fade(0.35, 0.15)))
            .line_width(1.5)
            .stroke()
            .font("800 15px sans-serif")
            .text_align(CanvasTextAlign::Center)
            .text_baseline(CanvasTextBaseline::Middle)
            .fill_color(hexa(CREAM, fade(0.85, 0.35)))
            .fill_text(&region.name.to_uppercase(), f32::midpoint(x0, x1), 46.0)
            .font("italic 12px sans-serif")
            .fill_color(hexa(TEXT, fade(0.6, 0.3)))
            .fill_text(region.topic, f32::midpoint(x0, x1), 66.0);
    }
    painter
}

/// The route, golden where it's been sailed.
fn draw_route(mut painter: Painter, best: &[Option<usize>]) -> Painter {
    for i in 1..LEVELS.len() {
        let ((x0, y0), (x1, y1)) = (isle_pos(i - 1), isle_pos(i));
        let sailed = best[i - 1].is_some();
        painter = painter
            .stroke_color(if sailed {
                hexa(GOLD, 0.6)
            } else {
                hexa(WHITE, 0.18)
            })
            .line_width(2.0)
            .line_dash(&[px(5.0), px(7.0)])
            .begin_path()
            .move_to(x0, y0 + 10.0)
            .quadratic_curve_to(
                point(px(f32::midpoint(x0, x1)), px(f32::midpoint(y0, y1) + 40.0)),
                point(px(x1), px(y1 + 10.0)),
            )
            .stroke()
            .line_dash(&[]);
    }
    painter
}

/// Chapter names over their isles.
fn draw_chapter_names(mut painter: Painter, fog_from: usize) -> Painter {
    for (i, level) in LEVELS.iter().enumerate() {
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
            .fill_color(hexa(LAVENDER, if foggy { 0.3 } else { 0.75 }))
            .fill_text(&level.chapter.to_uppercase(), cx, y);
    }
    painter
}

fn draw_isle(painter: Painter, i: usize, best: Option<usize>, selected: bool, foggy: bool) -> Painter {
    let (x, y) = isle_pos(i);
    let theme = story::theme(LEVELS[i].chapter);
    let boss = is_boss_isle(i);
    let r = if boss { 30.0 } else { 24.0 };
    let mut painter = painter;
    if selected {
        painter = painter
            .fill_style(
                CanvasGradient::radial(point(px(x), px(y)), px(4.0), point(px(x), px(y)), px(r * 2.6))
                    .add_color_stop(percentage(0.0), hexa(CREAM, 0.35))
                    .add_color_stop(percentage(1.0), hexa(CREAM, 0.0)),
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
        painter = draw_lair(painter, (x, y));
    }
    if best.is_some() {
        painter = draw_flag(painter, (x, y));
    }
    if foggy {
        painter = draw_fog(painter, (x, y));
    }
    draw_isle_name(painter, i, best, selected, foggy)
}

/// A boss's lair: a dark spire with a red eye.
fn draw_lair(painter: Painter, (x, y): (f32, f32)) -> Painter {
    painter
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
        .fill()
}

/// A flag on the cleared.
fn draw_flag(painter: Painter, (x, y): (f32, f32)) -> Painter {
    painter
        .stroke_color(hex(GLYPH))
        .line_width(1.5)
        .begin_path()
        .move_to(x + 8.0, y - 2.0)
        .line_to(x + 8.0, y - 26.0)
        .stroke()
        .fill_color(hex(GOLD))
        .begin_path()
        .move_to(x + 8.0, y - 26.0)
        .line_to(x + 22.0, y - 21.0)
        .line_to(x + 8.0, y - 16.0)
        .close_path()
        .fill()
}

/// Fog over an isle not yet in reach.
fn draw_fog(mut painter: Painter, (x, y): (f32, f32)) -> Painter {
    painter = painter.fill_color(hexa(0xd8d4e8, 0.55));
    for (dx, dy, rr) in [
        (-16.0, -6.0, 15.0),
        (0.0, -12.0, 18.0),
        (16.0, -4.0, 14.0),
        (-4.0, 4.0, 16.0),
        (10.0, 6.0, 12.0),
    ] {
        painter = painter.begin_path().arc(x + dx, y + dy, rr, 0.0, TAU).fill();
    }
    painter
}

/// The isle's name below it, and its stars once cleared.
fn draw_isle_name(painter: Painter, i: usize, best: Option<usize>, selected: bool, foggy: bool) -> Painter {
    let (x, y) = isle_pos(i);
    let level = &LEVELS[i];
    let r = if is_boss_isle(i) { 30.0 } else { 24.0 };
    let name = if foggy {
        "???".to_string()
    } else {
        format!("{}. {}", i + 1, level.title)
    };
    let painter = painter
        .font(if selected {
            "700 13px sans-serif"
        } else {
            "600 12px sans-serif"
        })
        .text_align(CanvasTextAlign::Center)
        .text_baseline(CanvasTextBaseline::Middle)
        .fill_color(hexa(BLACK, 0.5))
        .fill_text(&name, x + 1.0, y + r * 0.62 + 21.0)
        .fill_color(if selected {
            hex(CREAM)
        } else {
            hexa(TEXT, if foggy { 0.4 } else { 0.85 })
        })
        .fill_text(&name, x, y + r * 0.62 + 20.0);
    let Some(n) = best else {
        return painter;
    };
    let stars = game::stars(n, level.par()) as usize;
    painter.font("12px sans-serif").fill_color(hex(GOLD)).fill_text(
        &format!("{}{}", "★".repeat(stars), "☆".repeat(3 - stars)),
        x,
        y + r * 0.62 + 36.0,
    )
}

/// Pip's ferry: a little boat with a sail, and Pip the owl at the stern.
pub(super) fn draw_ferry(motion: &Motion) -> Painter {
    Painter::new()
        .save()
        .translate(&motion.hero_x, &motion.hero_y)
        .translate(0.0, &motion.bob)
        .fill_color(hexa(BLACK, 0.25))
        .begin_path()
        .ellipse_at(0.0, 9.0, 22.0, 4.0)
        .fill()
        .fill_color(hex(WOOD))
        .begin_path()
        .move_to(-20.0, 0.0)
        .line_to(20.0, 0.0)
        .line_to(13.0, 9.0)
        .line_to(-13.0, 9.0)
        .close_path()
        .fill()
        .stroke_color(hex(GLYPH))
        .line_width(1.5)
        .begin_path()
        .move_to(0.0, 0.0)
        .line_to(0.0, -30.0)
        .stroke()
        .fill_color(hex(GLYPH))
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
        .fill_color(hex(WHITE))
        .begin_path()
        .arc(-13.5, -8.0, 2.3, 0.0, TAU)
        .arc(-8.5, -8.0, 2.3, 0.0, TAU)
        .fill()
        .fill_color(hex(INK))
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
