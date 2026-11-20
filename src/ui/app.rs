//! The root component: the sky, and over it the screen on show.

use std::rc::Rc;

use nsh_core::prelude::*;

use crate::audio;
use crate::story;
use crate::ui::colors::{SKY_TOP, TEXT, hex};
use crate::ui::draw::draw_sky;
use crate::ui::grammar_screen::grammar_screen;
use crate::ui::keys::bind_keys;
use crate::ui::motion::{Motion, Ui};
use crate::ui::play::play_screen;
use crate::ui::replay::replay_screen;
use crate::ui::state::{Screen, State, with_state};
use crate::ui::theme::sync_theme;
use crate::ui::url::follow_url;
use crate::ui::widgets::cover_canvas;
use crate::ui::worldmap::menu_screen;

/// The application's root component.
#[component]
pub fn App() -> impl IntoElement {
    let ui = Ui::new();
    let motion = Rc::new(Motion::new());
    motion.start_ambience();
    audio::load_all();
    audio::start_ambience();
    bind_keys(ui, &motion);
    follow_url(ui);
    with_state(State::new_run);
    sync_theme(ui);

    let sky = motion.clone();
    view()
        .flex_1()
        .bg(hex(SKY_TOP))
        .text_color(hex(TEXT))
        .on_layout(move |b: &Bounds<Points>| ui.window.set((b.size.width.0, b.size.height.0)))
        .child(cover_canvas(canvas(move || {
            let chapter = ui.chapter.get();
            draw_sky(
                ui.window.get(),
                &sky,
                story::CHAPTERS[chapter.min(story::CHAPTERS.len() - 1)].theme,
            )
        })))
        .child(view().flex_1().p_safe_area().child(dynamic(move || {
            ui.scene.get();
            screen(ui, &motion)
        })))
}

/// The screen on show.
fn screen(ui: Ui, motion: &Rc<Motion>) -> AnyElement {
    let which = with_state(|s| match s.screen {
        Screen::Play(_) => 0,
        Screen::Menu { .. } => 1,
        Screen::Grammar { .. } => 2,
        Screen::Replay(_) => 3,
    });
    // Built outside the state's borrow: the screens read it themselves.
    match which {
        0 => play_screen(ui, motion).into_element(),
        1 => menu_screen(ui, motion).into_element(),
        2 => grammar_screen(ui).into_element(),
        _ => replay_screen(ui).into_element(),
    }
}
