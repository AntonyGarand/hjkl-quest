//! The editor's tests, by topic: each runs keys on a small text and checks
//! the text, the cursor or what the keys returned.

mod editing;
mod ex;
mod motions;
mod visual;

use super::{Editor, Pos, parse_keys};

/// An editor on `text` with the cursor at `at`, after `keys`.
fn run(text: &str, at: Pos, keys: &str) -> Editor {
    let mut e = Editor::new(text).with_cursor(at);
    e.feed_all(&parse_keys(keys));
    e
}
