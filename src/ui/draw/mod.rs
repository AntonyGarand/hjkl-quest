//! Painters: the board and what stands on it, the minimap and the sky.
//! Each returns a `Painter` the canvases redraw when the keys move on.

mod creatures;
mod hero;
mod marks;
mod minimap;
mod sky;
mod tile;
mod world;

use std::f32::consts::TAU;

use nsh_core::canvas::{CanvasTextAlign, CanvasTextBaseline};
use nsh_core::prelude::*;

use crate::ui::colors::{BLACK, hex, hexa};
use crate::ui::layout::{CH, CW, SKEW};
use crate::vim::{Editor, Mode, Pos};

pub(crate) use creatures::{draw_bugs, draw_gems, draw_sentries};
pub(crate) use hero::{draw_burst, draw_hero, draw_trail};
pub(crate) use minimap::{draw_minimap, minimap_size};
pub(crate) use sky::draw_sky;
pub(crate) use world::draw_world;

/// A filled ellipse's path, which the canvas lacks.
pub(crate) trait EllipseAt {
    fn ellipse_at(self, x: f32, y: f32, rx: f32, ry: f32) -> Self;
}

impl EllipseAt for Painter {
    fn ellipse_at(self, x: f32, y: f32, rx: f32, ry: f32) -> Self {
        self.save()
            .translate(x, y)
            .scale(rx.max(0.01), ry.max(0.01))
            .arc(0.0, 0.0, 1.0, 0.0, TAU)
            .restore()
    }
}

/// Whether the cursor's selection covers `p`.
pub(crate) fn is_selected(e: &Editor, p: Pos) -> bool {
    match e.mode {
        Mode::Visual { line } => {
            let (s, t) = if e.anchor <= e.cursor {
                (e.anchor, e.cursor)
            } else {
                (e.cursor, e.anchor)
            };
            if line {
                p.row >= s.row && p.row <= t.row
            } else {
                p >= s && p <= t
            }
        }
        Mode::Block => {
            let (r0, r1, c0, c1, to_end) = e.block_rect();
            p.row >= r0 && p.row <= r1 && p.col >= c0 && (to_end || p.col <= c1)
        }
        _ => false,
    }
}

/// The path around a tile's top face, its corner at (`x`, `y`).
fn top_face(painter: Painter, x: f32, y: f32) -> Painter {
    painter
        .begin_path()
        .move_to(x + SKEW, y)
        .line_to(x + SKEW + CW, y)
        .line_to(x + CW, y + CH)
        .line_to(x, y + CH)
        .close_path()
}

/// A tile's letter, centred at (`cx`, `cy`), over a soft shadow.
fn letter(painter: Painter, ch: char, (cx, cy): (f32, f32), shadow: f32, color: u32) -> Painter {
    let ch = ch.to_string();
    painter
        .font("700 19px monospace")
        .text_align(CanvasTextAlign::Center)
        .text_baseline(CanvasTextBaseline::Middle)
        .fill_color(hexa(BLACK, shadow))
        .fill_text(&ch, cx + 1.0, cy + 1.5)
        .fill_color(hex(color))
        .fill_text(&ch, cx, cy + 0.5)
}
