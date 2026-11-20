//! What stands on the tiles: crystals to collect, typo bugs to squash and
//! the sentries that march the rows.

use std::f32::consts::{PI, TAU};

use nsh_core::canvas::{CanvasGradient, CanvasTextAlign, CanvasTextBaseline, LineCap};
use nsh_core::prelude::*;

use super::EllipseAt;
use crate::game::Session;
use crate::ui::colors::{BLACK, BUG, CREAM, GEM, INK, LANTERN, PANEL, WHITE, hex, hexa};
use crate::ui::layout::tile_center;

/// A bug's shell markings, head and feelers.
const BUG_DARK: u32 = 0x2d5a1a;
/// A bug's legs.
const BUG_LEGS: u32 = 0x1d3a10;
/// A sentry's sheet.
const GHOST: u32 = 0xe9e4f5;
/// A sentry's lantern flame.
const FLAME: u32 = 0xffb84c;

/// The crystals not yet collected, bobbing, each with how many commands
/// it has left when it fades.
pub(crate) fn draw_gems(session: &Session, bob: &AnimatedValue) -> Painter {
    let mut painter = Painter::new();
    for &(p, got) in &session.gems {
        if got {
            continue;
        }
        let (cx, cy) = tile_center(p);
        if let Some(n) = session.fades_in(p) {
            painter = draw_countdown(painter, (cx, cy), n);
        }
        painter = draw_gem(painter, (cx, cy), bob);
    }
    painter
}

/// A fading crystal's commands left, in a badge; it dims the crystal.
fn draw_countdown(painter: Painter, (cx, cy): (f32, f32), n: usize) -> Painter {
    let urgent = n <= 2;
    painter
        .fill_color(hexa(if urgent { 0xff6b5a } else { PANEL }, 0.85))
        .begin_path()
        .arc(cx + 14.0, cy - 18.0, 9.0, 0.0, TAU)
        .fill()
        .font("800 11px monospace")
        .text_align(CanvasTextAlign::Center)
        .text_baseline(CanvasTextBaseline::Middle)
        .fill_color(hex(CREAM))
        .fill_text(&n.to_string(), cx + 14.0, cy - 17.5)
        .global_alpha(if urgent { 0.55 } else { 0.85 })
}

fn draw_gem(painter: Painter, (cx, cy): (f32, f32), bob: &AnimatedValue) -> Painter {
    painter
        .fill_color(hexa(GEM, 0.22))
        .begin_path()
        .ellipse_at(cx, cy + 2.0, 8.0, 3.5)
        .fill()
        .save()
        .translate(cx, cy - 13.0)
        .translate(0.0, bob)
        .fill_style(
            CanvasGradient::radial(
                point(px(0.0), px(0.0)),
                px(1.0),
                point(px(0.0), px(0.0)),
                px(16.0),
            )
            .add_color_stop(percentage(0.0), hexa(GEM, 0.55))
            .add_color_stop(percentage(1.0), hexa(GEM, 0.0)),
        )
        .begin_path()
        .arc(0.0, 0.0, 16.0, 0.0, TAU)
        .fill()
        .fill_color(hex(GEM))
        .begin_path()
        .move_to(0.0, -9.0)
        .line_to(6.0, -1.0)
        .line_to(0.0, 8.0)
        .line_to(-6.0, -1.0)
        .close_path()
        .fill()
        .fill_color(hexa(WHITE, 0.75))
        .begin_path()
        .move_to(0.0, -9.0)
        .line_to(2.5, -1.0)
        .line_to(0.0, 2.0)
        .line_to(-2.5, -1.0)
        .close_path()
        .fill()
        .restore()
        .global_alpha(1.0)
}

/// The sentries: lantern-bearing ghosts, an arrow showing where each
/// marches next.
pub(crate) fn draw_sentries(session: &Session, bob: &AnimatedValue) -> Painter {
    let mut painter = Painter::new();
    for s in &session.sentries {
        let (cx, cy) = tile_center(s.at);
        let d = f32::from(s.dir);
        painter = painter
            .fill_color(hexa(BLACK, 0.3))
            .begin_path()
            .ellipse_at(cx, cy + 3.0, 10.0, 4.0)
            .fill();
        painter = draw_ghost(
            painter
                .save()
                .translate(cx, cy - 16.0)
                .translate(0.0, &bob.mul(0.6)),
        )
        .restore()
        // Where it goes next.
        .fill_color(hexa(LANTERN, 0.85))
        .begin_path()
        .move_to(cx - 5.0, cy + 9.0 * d)
        .line_to(cx + 5.0, cy + 9.0 * d)
        .line_to(cx, cy + 15.0 * d)
        .close_path()
        .fill();
    }
    painter
}

/// A sentry's ghost and its lantern's glow, around the origin.
fn draw_ghost(painter: Painter) -> Painter {
    painter
        .fill_style(
            CanvasGradient::radial(
                point(px(0.0), px(0.0)),
                px(2.0),
                point(px(0.0), px(0.0)),
                px(22.0),
            )
            .add_color_stop(percentage(0.0), hexa(LANTERN, 0.45))
            .add_color_stop(percentage(1.0), hexa(LANTERN, 0.0)),
        )
        .begin_path()
        .arc(0.0, 0.0, 22.0, 0.0, TAU)
        .fill()
        .fill_color(hex(GHOST))
        .begin_path()
        .move_to(-9.0, 10.0)
        .line_to(-9.0, -2.0)
        .arc(0.0, -2.0, 9.0, PI, 0.0)
        .line_to(9.0, 10.0)
        .line_to(6.0, 7.0)
        .line_to(3.0, 10.0)
        .line_to(0.0, 7.0)
        .line_to(-3.0, 10.0)
        .line_to(-6.0, 7.0)
        .close_path()
        .fill()
        .fill_color(hex(INK))
        .begin_path()
        .arc(-3.5, -2.0, 1.8, 0.0, TAU)
        .arc(3.5, -2.0, 1.8, 0.0, TAU)
        .fill()
        .fill_color(hex(FLAME))
        .begin_path()
        .arc(11.0, 4.0, 3.0, 0.0, TAU)
        .fill()
}

/// A typo bug: a little green beetle, facing the way it crawls.
pub(crate) fn draw_bugs(session: &Session, bob: &AnimatedValue) -> Painter {
    let mut painter = Painter::new();
    for bug in &session.bugs {
        let (cx, cy) = tile_center(bug.at);
        painter = painter
            .fill_color(hexa(BLACK, 0.3))
            .begin_path()
            .ellipse_at(cx, cy + 4.0, 11.0, 4.0)
            .fill()
            .save()
            .translate(cx, cy - 6.0)
            .translate(&bob.mul(0.4), 0.0);
        painter = draw_beetle(painter, f32::from(bug.dir)).restore();
    }
    painter
}

/// A beetle around the origin, its head towards `d` (+1 right, -1 left).
fn draw_beetle(painter: Painter, d: f32) -> Painter {
    let mut painter = painter
        .stroke_color(hex(BUG_LEGS))
        .line_width(1.6)
        .line_cap(LineCap::Round);
    // Legs, three a side.
    for k in [-5.0f32, 0.0, 5.0] {
        painter = painter
            .begin_path()
            .move_to(k, -2.0)
            .line_to(k - 2.0, -8.0)
            .move_to(k, 2.0)
            .line_to(k - 2.0, 8.0)
            .stroke();
    }
    painter
        .fill_color(hex(BUG))
        .begin_path()
        .ellipse_at(0.0, 0.0, 9.0, 6.5)
        .fill()
        .stroke_color(hex(BUG_DARK))
        .line_width(1.0)
        .begin_path()
        .move_to(-8.0, 0.0)
        .line_to(8.0, 0.0)
        .stroke()
        .fill_color(hex(BUG_DARK))
        .begin_path()
        .arc(d * 9.0, 0.0, 4.2, 0.0, TAU)
        .fill()
        .fill_color(hex(WHITE))
        .begin_path()
        .arc(d * 10.5, -1.8, 1.5, 0.0, TAU)
        .arc(d * 10.5, 1.8, 1.5, 0.0, TAU)
        .fill()
        .stroke_color(hex(BUG_DARK))
        .begin_path()
        .move_to(d * 11.0, -2.0)
        .line_to(d * 15.0, -6.0)
        .move_to(d * 11.0, 2.0)
        .line_to(d * 15.0, 6.0)
        .stroke()
}
