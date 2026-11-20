//! One tile of the board: a block of grass, stone, wood or amber with its
//! letter on top; and what can lie on one: a trap, a ghost of missing
//! text, a search match.

use std::f32::consts::TAU;

use nsh_core::canvas::{CanvasTextAlign, CanvasTextBaseline};
use nsh_core::prelude::*;

use super::{letter, top_face};
use crate::ui::colors::{
    AMBER, BLACK, EMBER, GLYPH, GOLD, ICE, SELECT, TEAL, TRAP, WHITE, WOOD, hex, hexa, noise, shade,
};
use crate::ui::layout::{CH, CW, DEPTH, SKEW, tile_origin};
use crate::ui::theme::theme;
use crate::vim::Pos;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum TileLook {
    Grass,
    Flower,
    Stone,
    Amber,
    Teal,
    Plank,
    Wrong,
    Selected,
    Trap,
    Ice,
}

impl TileLook {
    /// The look a character gives its tile.
    pub(super) fn of(c: char) -> TileLook {
        match c {
            ' ' => TileLook::Grass,
            '.' => TileLook::Flower,
            c if c.is_ascii_digit() => TileLook::Teal,
            c if c.is_alphanumeric() || c == '_' => TileLook::Stone,
            _ => TileLook::Amber,
        }
    }

    pub(super) fn color(self) -> u32 {
        match self {
            TileLook::Grass | TileLook::Flower => theme().grass,
            TileLook::Stone => theme().stone,
            TileLook::Amber => AMBER,
            TileLook::Teal => TEAL,
            TileLook::Plank => WOOD,
            TileLook::Wrong => EMBER,
            TileLook::Selected => SELECT,
            TileLook::Trap => TRAP,
            TileLook::Ice => ICE,
        }
    }
}

/// The colours of flowers on a period's tile.
const FLOWERS: [u32; 3] = [0xffd6e7, 0xfff1a8, 0xd6e4ff];

pub(super) fn draw_tile(painter: Painter, p: Pos, c: Option<char>, look: TileLook) -> Painter {
    let (x, y) = tile_origin(p);
    let painter = draw_block(painter, p, look.color());
    let (cx, cy) = (x + CW / 2.0 + SKEW / 2.0, y + CH / 2.0);
    let painter = match look {
        TileLook::Flower => {
            let hue = FLOWERS[(noise(p) * 3.0) as usize % 3];
            let flip = Pos {
                row: p.col,
                col: p.row,
            };
            painter
                .fill_color(hex(hue))
                .begin_path()
                .arc(cx - 3.0 + noise(flip) * 6.0, cy, 1.8, 0.0, TAU)
                .fill()
        }
        TileLook::Grass => painter
            .stroke_color(hexa(0xb8f0a0, 0.35))
            .line_width(1.0)
            .begin_path()
            .move_to(cx - 4.0, cy + 3.0)
            .line_to(cx - 3.0, cy - 1.0)
            .move_to(cx + 3.0, cy + 2.0)
            .line_to(cx + 4.0, cy - 2.0)
            .stroke(),
        TileLook::Plank => painter
            .stroke_color(hexa(0x3a2410, 0.6))
            .line_width(1.0)
            .begin_path()
            .move_to(x + SKEW * 0.66 + 1.0, y + CH / 3.0)
            .line_to(x + SKEW * 0.66 + CW - 1.0, y + CH / 3.0)
            .move_to(x + SKEW * 0.33 + 1.0, y + CH * 2.0 / 3.0)
            .line_to(x + SKEW * 0.33 + CW - 1.0, y + CH * 2.0 / 3.0)
            .stroke(),
        _ => painter,
    };
    match c.filter(|c| !matches!(c, ' ' | '.')) {
        Some(ch) => {
            let glyph = if look == TileLook::Wrong { 0xfff4e6 } else { GLYPH };
            letter(painter, ch, (cx, cy), 0.35, glyph)
        }
        None => painter,
    }
}

/// The block itself: its left side, front and top, lit a little
/// differently tile to tile.
fn draw_block(painter: Painter, p: Pos, base: u32) -> Painter {
    let (x, y) = tile_origin(p);
    let k = 0.92 + noise(p) * 0.16;
    let painter = painter
        // Left side, seen where the block has no neighbour.
        .fill_color(shade(base, k * 0.48))
        .begin_path()
        .move_to(x, y + CH)
        .line_to(x + SKEW, y)
        .line_to(x + SKEW, y + DEPTH)
        .line_to(x, y + CH + DEPTH)
        .close_path()
        .fill()
        // Front.
        .fill_color(shade(base, k * 0.62))
        .fill_rect(x, y + CH, CW, DEPTH)
        // Top.
        .fill_color(shade(base, k));
    top_face(painter, x, y)
        .fill()
        // A lighter rim on the back edge.
        .stroke_color(hexa(WHITE, 0.10))
        .line_width(1.0)
        .begin_path()
        .move_to(x + SKEW + 1.0, y + 0.5)
        .line_to(x + SKEW + CW - 1.0, y + 0.5)
        .stroke()
        .stroke_color(hexa(BLACK, 0.22))
        .begin_path()
        .move_to(x + CW, y + CH)
        .line_to(x + CW, y + CH + DEPTH)
        .stroke()
}

/// A trap: a flat plate of dark red stone, level with the tiles so it
/// hides nothing behind it, with a row of small spikes along its front
/// edge. The cursor can never stand on it. Its letter stays readable, for
/// f and t.
pub(super) fn draw_trap(painter: Painter, p: Pos, c: Option<char>) -> Painter {
    let (x, y) = tile_origin(p);
    let mut painter = top_face(painter.fill_color(shade(TRAP, 1.15)), x, y)
        .fill()
        .stroke_color(hexa(0xff6b6b, 0.7))
        .line_width(1.4)
        .stroke();
    // Small spikes along the front edge, inside the tile.
    for k in 0..5 {
        let bx = x + 6.0 + k as f32 * 7.0;
        let by = y + CH - 2.0;
        painter = painter
            .fill_color(hex(0xc3cbdd))
            .begin_path()
            .move_to(bx - 2.5, by)
            .line_to(bx, by - 6.0)
            .line_to(bx + 2.5, by)
            .close_path()
            .fill()
            .fill_color(hex(0x737d97))
            .begin_path()
            .move_to(bx, by)
            .line_to(bx, by - 6.0)
            .line_to(bx + 2.5, by)
            .close_path()
            .fill();
    }
    match c.filter(|c| !matches!(c, ' ' | '.')) {
        Some(ch) => letter(
            painter,
            ch,
            (x + CW / 2.0 + SKEW / 2.0, y + CH / 2.0 - 2.0),
            0.4,
            0xffe4e0,
        ),
        None => painter,
    }
}

/// A search match on a tile: a warm glow on its top face.
pub(super) fn draw_match(painter: Painter, p: Pos) -> Painter {
    let (x, y) = tile_origin(p);
    painter
        .fill_color(hexa(GOLD, 0.38))
        .stroke_color(hexa(GOLD, 0.95))
        .line_width(1.5)
        .begin_path()
        .move_to(x + SKEW + 1.0, y + 1.0)
        .line_to(x + SKEW + CW - 1.0, y + 1.0)
        .line_to(x + CW - 1.0, y + CH - 1.0)
        .line_to(x + 1.0, y + CH - 1.0)
        .close_path()
        .fill()
        .stroke()
}

/// Where the goal has a character the buffer lacks: a faint outline.
pub(super) fn draw_ghost(painter: Painter, p: Pos, want: char) -> Painter {
    let (x, y) = tile_origin(p);
    let painter = painter
        .stroke_color(hexa(0xffd2a8, 0.45))
        .line_width(1.0)
        .line_dash(&[px(3.0), px(3.0)]);
    let painter = top_face(painter, x, y).stroke().line_dash(&[]);
    if want == ' ' {
        return painter;
    }
    painter
        .font("700 18px monospace")
        .text_align(CanvasTextAlign::Center)
        .text_baseline(CanvasTextBaseline::Middle)
        .fill_color(hexa(0xffd2a8, 0.5))
        .fill_text(&want.to_string(), x + CW / 2.0 + SKEW / 2.0, y + CH / 2.0 + 0.5)
}
