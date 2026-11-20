//! The game drawn with Native Shell: the world map, the isles, and the
//! screens around them, driven by the keyboard.

mod app;
mod boss;
mod colors;
mod complete;
mod draw;
mod grammar_screen;
mod hint;
mod intro;
mod keys;
mod layout;
mod motion;
mod play;
mod replay;
mod state;
mod storage;
mod theme;
mod url;
mod widgets;
mod worldmap;

pub use app::App;
