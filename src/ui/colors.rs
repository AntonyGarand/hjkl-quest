//! The palette, and small helpers to turn `0xrrggbb` into colours.

use nsh_core::prelude::*;

use crate::vim::{Editor, Mode, Pos};

/// A colour from `0xrrggbb`.
pub(crate) fn hex(c: u32) -> Hsla {
    rgb(c).into()
}

/// A colour from `0xrrggbb`, at opacity `a`.
pub(crate) fn hexa(c: u32, a: f32) -> Hsla {
    hex(c).opacity(a)
}

/// `c` lit (`k` > 1) or darkened (`k` < 1), channel by channel.
pub(crate) fn shade(c: u32, k: f32) -> Hsla {
    let r = ((c >> 16) & 0xff) as f32 * k;
    let g = ((c >> 8) & 0xff) as f32 * k;
    let b = (c & 0xff) as f32 * k;
    let v = ((r.min(255.0) as u32) << 16) | ((g.min(255.0) as u32) << 8) | (b.min(255.0) as u32);
    hex(v)
}

/// A steady pseudo-random number in 0..1 for a position: what varies the
/// tiles' shades and scatters stars, waves and raindrops.
pub(crate) fn noise(p: Pos) -> f32 {
    let mut h = (p.row as u32).wrapping_mul(73_856_093) ^ (p.col as u32).wrapping_mul(19_349_663);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    ((h >> 8) & 0xffff) as f32 / 65_535.0
}

// The board.
pub(crate) const SKY_TOP: u32 = 0x0a0f24;
pub(crate) const AMBER: u32 = 0x9a7440;
pub(crate) const TEAL: u32 = 0x2f8a8f;
pub(crate) const WOOD: u32 = 0x86603a;
pub(crate) const EMBER: u32 = 0xc25a3f;
pub(crate) const SELECT: u32 = 0x8a63e0;
pub(crate) const GEM: u32 = 0x7ef3ff;
pub(crate) const BUG: u32 = 0x8be04e;
pub(crate) const GLYPH: u32 = 0xf6f1e3;
pub(crate) const TRAP: u32 = 0x7a3442;
pub(crate) const ICE: u32 = 0x8fc3e0;
/// A sentry's lantern.
pub(crate) const LANTERN: u32 = 0xff9a4c;
/// Where missing text shows, in amber.
pub(crate) const MISSING: u32 = 0xffb44c;
/// Goal characters already in the buffer.
pub(crate) const PRESENT: u32 = 0x8fe3a0;

// Text and panels.
pub(crate) const WHITE: u32 = 0xffffff;
pub(crate) const BLACK: u32 = 0x000000;
/// Body text.
pub(crate) const TEXT: u32 = 0xece6ff;
/// Titles, key chips and the normal-mode cursor.
pub(crate) const CREAM: u32 = 0xfff3b0;
/// Small capitals over a section.
pub(crate) const LAVENDER: u32 = 0xc9b8ff;
/// Keys to type, stars, habits.
pub(crate) const GOLD: u32 = 0xffd76b;
/// Text on a light background: the block cursor's letter.
pub(crate) const INK: u32 = 0x1a1430;
/// The dark behind overlays and the status line.
pub(crate) const NIGHT: u32 = 0x05030f;
pub(crate) const PANEL: u32 = 0x0b0820;
/// A modal card.
pub(crate) const CARD: u32 = 0x171032;
/// Half-typed commands and messages.
pub(crate) const ORANGE: u32 = 0xffb86b;
/// A boss's name.
pub(crate) const BOSS: u32 = 0xff9a6b;
/// A missed lesson, text to remove.
pub(crate) const CORAL: u32 = 0xff8a70;

/// The cursor's colour, by mode (amber while a command is half typed).
pub(crate) fn mode_color(e: &Editor) -> u32 {
    match e.mode {
        Mode::Insert => 0x7dffa8,
        Mode::Visual { .. } | Mode::Block => 0xc9a2ff,
        Mode::Replace => 0xff8f8f,
        Mode::Command { .. } | Mode::Confirm => 0x8ec5ff,
        Mode::Normal if !e.pending().is_empty() => ORANGE,
        Mode::Normal => CREAM,
    }
}
