//! The stage: the board in layers of canvases under a moving camera, the
//! minimap, and what floats over them.

use std::rc::Rc;

use nsh_core::prelude::*;

use crate::game::Session;
use crate::ui::boss::boss_banner;
use crate::ui::colors::{LAVENDER, NIGHT, hexa};
use crate::ui::complete::complete_card;
use crate::ui::draw::{
    draw_bugs, draw_burst, draw_gems, draw_hero, draw_minimap, draw_sentries, draw_trail, draw_world,
    minimap_size,
};
use crate::ui::hint::hint_card;
use crate::ui::layout::world_size;
use crate::ui::motion::{Motion, Ui};
use crate::ui::play::status::status_line;
use crate::ui::state::session_value;
use crate::ui::widgets::cover_canvas;

/// Room the board keeps beyond the buffer, so the camera can frame it.
const SPARE_ROWS: usize = 2;
const SPARE_COLS: usize = 12;

pub(super) fn stage(ui: Ui, motion: &Rc<Motion>) -> impl IntoElement {
    view()
        .flex_1()
        .overflow_hidden()
        .rounded(px(16.0))
        .bg(hexa(NIGHT, 0.25))
        .border_1()
        .border_color(hexa(LAVENDER, 0.14))
        .on_layout(move |b: &Bounds<Points>| {
            ui.stage.set((b.size.width.0, b.size.height.0));
        })
        .child(
            view()
                .absolute()
                .top_0()
                .left_0()
                .bind_translate_x(&motion.cam_x)
                .bind_translate_y(&motion.cam_y)
                .child(board(ui, motion)),
        )
        .child(minimap(ui, motion.clone()))
        .child(boss_banner(ui, motion.clone()))
        .child(hint_card(ui))
        .child(status_line(ui))
        .child(complete_card(ui, motion))
}

/// The board, sized to the buffer, floating and shaking, its layers bottom
/// to top: tiles, crystals, dust, sentries, bugs, the wisp, bursts.
fn board(ui: Ui, motion: &Rc<Motion>) -> impl IntoElement {
    let trail = motion.clone();
    let burst = motion.clone();
    view()
        .bind_translate_y(&motion.float)
        .bind_translate_x(&motion.shake)
        .w(move || {
            ui.tick.get();
            px(session_value(|s| {
                let cols = s.editor.lines.iter().map(Vec::len).max().unwrap_or(0).max(8);
                world_size(s.editor.lines.len() + SPARE_ROWS, cols + SPARE_COLS).0
            })
            .unwrap_or(400.0))
        })
        .h(move || {
            ui.tick.get();
            px(session_value(|s| world_size(s.editor.lines.len() + 4, 8).1).unwrap_or(300.0))
        })
        .child(layer(ui, motion, |s, _| draw_world(s)))
        .child(layer(ui, motion, |s, m| draw_gems(s, &m.bob)))
        .child(cover_canvas(canvas(move || {
            draw_trail(ui.trail.get(), &trail.trail)
        })))
        .child(layer(ui, motion, |s, m| draw_sentries(s, &m.bob)))
        .child(layer(ui, motion, |s, m| draw_bugs(s, &m.bob)))
        .child(layer(ui, motion, draw_hero))
        .child(cover_canvas(canvas(move || {
            draw_burst(ui.burst_at.get(), ui.burst_color.get(), &burst.burst)
        })))
}

/// A canvas over the board, redrawn from the session after every key.
fn layer(
    ui: Ui,
    motion: &Rc<Motion>,
    draw: impl Fn(&Session, &Motion) -> Painter + 'static,
) -> impl IntoElement {
    let motion = motion.clone();
    cover_canvas(canvas(move || {
        ui.tick.get();
        session_value(|s| draw(s, &motion)).unwrap_or_default()
    }))
}

/// The minimap, in the stage's top right corner.
fn minimap(ui: Ui, motion: Rc<Motion>) -> impl IntoElement {
    dynamic(move || {
        ui.tick.get();
        let (mw, mh) = session_value(minimap_size)?;
        let map = motion.clone();
        Some(
            view()
                .absolute()
                .top(px(12.0))
                .right(px(12.0))
                .p(px(8.0))
                .rounded(px(10.0))
                .bg(hexa(NIGHT, 0.62))
                .border_1()
                .border_color(hexa(LAVENDER, 0.22))
                .child(
                    canvas(move || {
                        ui.tick.get();
                        let stage = ui.stage.get();
                        session_value(|s| draw_minimap(s, &map, stage)).unwrap_or_default()
                    })
                    .w(px(mw))
                    .h(px(mh)),
                ),
        )
    })
}
