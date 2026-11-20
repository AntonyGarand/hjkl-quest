//! HJKL Quest: learn vim by walking the Buffer Archipelago.
//!
//! The world is the text buffer: each character is a tile, and the cursor
//! is a small wisp that hops between them. `vim` is the editor, `levels`
//! the campaign, `game` a level being played; `ui` draws it all with
//! Native Shell.

pub mod audio;
pub mod coach;
#[cfg(test)]
mod code_size;
pub mod daily;
pub mod diff;
pub mod examples;
pub mod game;
pub mod grammar;
pub mod hints;
pub mod lessons;
pub mod levels;
pub mod regex;
pub mod review;
pub mod story;
mod ui;
pub mod vim;

use nsh_core::prelude::*;

pub use ui::App;

/// The app's UI set (`nsh.json` `"rust"`).
#[native_module("hjkl-quest/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}
