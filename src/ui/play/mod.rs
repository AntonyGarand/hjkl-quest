//! The play screen: the isle's name and scores over the stage, the keys
//! typed below it, and the lessons beside it (or above, when narrow).

mod camera;
mod goal;
mod header;
mod lessons;
mod panel;
mod stage;
mod status;

use std::rc::Rc;

use nsh_core::prelude::*;

use crate::ui::intro::intro_modal;
use crate::ui::motion::{Motion, Ui};
use crate::ui::state::session_value;

pub(crate) use camera::place_hero;

/// Below this width the lesson panel folds into a strip above the stage.
const NARROW: f32 = 980.0;

pub(crate) fn play_screen(ui: Ui, motion: &Rc<Motion>) -> impl IntoElement {
    let Some((index, level, campaign)) = session_value(|s| (s.index, s.level(), s.in_campaign())) else {
        return view().into_element();
    };
    // Place the wisp at once when the level opens.
    place_hero(ui, motion, false);
    let strip = dynamic(move || (ui.window.get().0 < NARROW).then(|| lessons::lesson_strip(ui, level)));
    let panel = dynamic(move || (ui.window.get().0 >= NARROW).then(|| panel::side_panel(level, ui)));
    view()
        .flex_1()
        .p(px(18.0))
        .gap_3()
        .child(header::top_bar(ui, index, level, campaign))
        .child(strip)
        .child(
            view()
                .flex_1()
                .flex_row()
                .gap_4()
                .child(
                    view()
                        .flex_1()
                        .gap_2()
                        .child(stage::stage(ui, motion))
                        .child(header::typed(ui)),
                )
                .child(panel),
        )
        .child(intro_modal(ui))
        .into_element()
}
