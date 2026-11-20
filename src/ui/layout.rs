//! Where the board's tiles go: each character is a block seen from a
//! three-quarter view, every row skewed a little to the right of the next.

use crate::vim::Pos;

/// Tile width, top-face depth, block thickness and the skew per row that
/// gives the board its three-quarter look.
pub(crate) const CW: f32 = 40.0;
pub(crate) const CH: f32 = 30.0;
pub(crate) const DEPTH: f32 = 15.0;
pub(crate) const SKEW: f32 = 12.0;
pub(crate) const PAD_X: f32 = 48.0;
pub(crate) const PAD_Y: f32 = 64.0;
/// Rows the board makes room for (the skew is measured from the last one).
pub(crate) const ROWS_ROOM: f32 = 16.0;
/// Room below the last row, for the isle's shadow on the clouds.
const BOTTOM_ROOM: f32 = 56.0;

/// The top-left corner of the tile at `p`.
pub(crate) fn tile_origin(p: Pos) -> (f32, f32) {
    (
        PAD_X + (ROWS_ROOM - p.row as f32) * SKEW + p.col as f32 * CW,
        PAD_Y + p.row as f32 * CH,
    )
}

/// The middle of the tile's top face.
pub(crate) fn tile_center(p: Pos) -> (f32, f32) {
    let (x, y) = tile_origin(p);
    (x + CW / 2.0 + SKEW / 2.0, y + CH / 2.0)
}

/// The board's size for `rows` × `cols` tiles.
pub(crate) fn world_size(rows: usize, cols: usize) -> (f32, f32) {
    (
        PAD_X * 2.0 + ROWS_ROOM * SKEW + cols as f32 * CW + CW,
        PAD_Y + rows as f32 * CH + DEPTH + BOTTOM_ROOM,
    )
}
