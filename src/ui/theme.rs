//! The look of the chapter on screen: its tiles' colours and its sky.

use std::cell::Cell;

use crate::levels::LEVELS;
use crate::story::{self, Theme};
use crate::ui::motion::Ui;
use crate::ui::state::{Screen, with_state};

thread_local! {
    /// The look of the chapter on screen, for the tiles and the sky.
    static THEME: Cell<Theme> = Cell::new(story::CHAPTERS[0].theme);
}

pub(crate) fn theme() -> Theme {
    THEME.with(Cell::get)
}

/// Takes the look of the chapter on screen (the map's selected isle, or
/// the isle being played).
pub(crate) fn sync_theme(ui: Ui) {
    let label = with_state(|s| match &s.screen {
        Screen::Play(p) => p.level().chapter,
        Screen::Replay(r) => r.session.level().chapter,
        Screen::Menu { selected } | Screen::Grammar { selected } => LEVELS[*selected].chapter,
    });
    THEME.with(|t| t.set(story::theme(label)));
    let index = story::CHAPTERS
        .iter()
        .position(|c| story::chapter(label).is_some_and(|k| std::ptr::eq(k, c)))
        .unwrap_or(0);
    if ui.chapter.get_untracked() != index {
        ui.chapter.set(index);
    }
}
