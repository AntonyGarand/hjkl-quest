//! The wisp and its cursor, and the bursts and dust it leaves behind.

use std::f32::consts::TAU;

use nsh_core::canvas::{CanvasGradient, CanvasTextAlign, CanvasTextBaseline};
use nsh_core::prelude::*;

use super::EllipseAt;
use crate::game::Session;
use crate::ui::colors::{BLACK, CREAM, INK, WHITE, hex, hexa, mode_color};
use crate::ui::layout::{CH, CW, SKEW};
use crate::ui::motion::Motion;
use crate::vim::Mode;

/// The wisp's body.
const WISP: u32 = 0xfffdf5;

pub(crate) fn draw_hero(session: &Session, motion: &Motion) -> Painter {
    let e = &session.editor;
    let color = mode_color(e);
    let painter = Painter::new().save().translate(&motion.hero_x, &motion.hero_y);
    // The cursor itself: a glowing outline on the tile (a bar in insert mode).
    let painter = if e.mode == Mode::Insert {
        painter
            .fill_color(hex(color))
            .begin_path()
            .move_to(SKEW - 1.0, 0.0)
            .line_to(SKEW + 2.0, 0.0)
            .line_to(3.0, CH)
            .line_to(0.0, CH)
            .close_path()
            .fill()
    } else {
        draw_block_cursor(painter, session, color)
    };
    // Its shadow on the tile.
    let painter = painter
        .fill_color(hexa(BLACK, 0.25))
        .begin_path()
        .ellipse_at(CW + SKEW * 0.4 - 3.0, 6.0, 6.0, 2.0)
        .fill();
    draw_wisp(painter, motion, color).restore()
}

/// As vim's block cursor: the tile lit, the character under it inverted.
fn draw_block_cursor(painter: Painter, session: &Session, color: u32) -> Painter {
    let e = &session.editor;
    let painter = painter
        .fill_color(hexa(color, 0.88))
        .stroke_color(hex(WHITE))
        .line_width(1.5)
        .begin_path()
        .move_to(SKEW, 0.0)
        .line_to(SKEW + CW, 0.0)
        .line_to(CW, CH)
        .line_to(0.0, CH)
        .close_path()
        .fill()
        .stroke();
    match e.char_at(e.cursor).filter(|c| !c.is_whitespace()) {
        Some(ch) => painter
            .font("700 19px monospace")
            .text_align(CanvasTextAlign::Center)
            .text_baseline(CanvasTextBaseline::Middle)
            .fill_color(hex(INK))
            .fill_text(&ch.to_string(), CW / 2.0 + SKEW / 2.0, CH / 2.0 + 0.5),
        None => painter,
    }
}

/// The wisp, floating above the tile's back edge, hopping as it moves.
fn draw_wisp(painter: Painter, motion: &Motion, color: u32) -> Painter {
    let (wx, wy) = (CW + SKEW * 0.4, -3.0);
    painter
        .translate(0.0, &motion.hop)
        .fill_style(
            CanvasGradient::radial(point(px(wx), px(wy)), px(1.0), point(px(wx), px(wy)), px(18.0))
                .add_color_stop(percentage(0.0), hexa(color, 0.7))
                .add_color_stop(percentage(1.0), hexa(color, 0.0)),
        )
        .begin_path()
        .arc(wx, wy, 18.0, 0.0, TAU)
        .fill()
        .fill_color(hex(WISP))
        .begin_path()
        .arc(wx, wy, 5.5, 0.0, TAU)
        .fill()
        .fill_color(hex(INK))
        .begin_path()
        .arc(wx - 1.9, wy - 0.5, 1.0, 0.0, TAU)
        .arc(wx + 1.9, wy - 0.5, 1.0, 0.0, TAU)
        .fill()
}

/// A ring and sparks spreading from `at` as `burst` runs 0 to 1.
pub(crate) fn draw_burst(at: (f32, f32), color: u32, burst: &AnimatedValue) -> Painter {
    let (cx, cy) = at;
    let radius = burst.mul(30.0).add(4.0);
    let fade = burst.interpolate(&[0.0, 1.0], &[1.0, 0.0]);
    let mut painter = Painter::new()
        .global_alpha(&fade)
        .stroke_color(hex(color))
        .line_width(2.0)
        .begin_path()
        .arc(cx, cy - 10.0, &radius, 0.0, TAU)
        .stroke();
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU;
        let r = burst.mul(26.0).add(6.0);
        painter = painter
            .save()
            .translate(cx, cy - 10.0)
            .rotate(a)
            .translate(&r, 0.0)
            .fill_color(if k % 2 == 0 { hex(WHITE) } else { hex(color) })
            .fill_rect(-1.5, -1.5, 3.0, 3.0)
            .restore();
    }
    painter
}

/// Dust puffs along a long jump, fading as they spread.
pub(crate) fn draw_trail(path: ((f32, f32), (f32, f32)), trail: &AnimatedValue) -> Painter {
    let ((x0, y0), (x1, y1)) = path;
    let fade = trail.interpolate(&[0.0, 1.0], &[0.7, 0.0]);
    let grow = trail.mul(5.0).add(2.0);
    let n = ((x1 - x0).hypot(y1 - y0) / 14.0).clamp(3.0, 40.0) as usize;
    let mut painter = Painter::new().global_alpha(&fade).fill_color(hex(CREAM));
    for k in 0..n {
        let t = k as f32 / n as f32;
        let (x, y) = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t - 4.0);
        painter = painter
            .save()
            .translate(x, y + (k % 2) as f32 * 3.0)
            .begin_path()
            .arc(0.0, 0.0, &grow, 0.0, TAU)
            .fill()
            .restore();
    }
    painter
}
