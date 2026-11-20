//! The world map: the Buffer Archipelago at sea, every isle on a route
//! from the Meadow to the Archive and beyond, its chapters grouped in
//! regions by topic. Pip's ferry sails it with h and l (isle by isle), j and
//! k (chapter by chapter) and J and K (region by region); isles past the
//! next one to clear lie in fog.

mod draw;
mod info;

use std::rc::Rc;

use nsh_core::prelude::*;

use crate::levels::LEVELS;
use crate::story;
use crate::ui::colors::{CREAM, LAVENDER, TEXT, hex, hexa};
use crate::ui::motion::{Motion, Ui};
use crate::ui::state::{Screen, chapter_start, with_state};
use crate::ui::widgets::cover_canvas;

/// Map size, and the spacing of chapters and of isles within them.
const MAP_H: f32 = 600.0;
const CHAPTER_GAP: f32 = 330.0;
const ISLE_GAP: f32 = 84.0;
/// Open sea between two regions.
const REGION_GAP: f32 = 220.0;

/// Isle `i`'s chapter (its place in the campaign), its place in the
/// chapter, and how many isles the chapter has.
fn place(i: usize) -> (usize, usize, usize) {
    let start = chapter_start(i);
    let chapter = (0..=i).filter(|&k| k == chapter_start(k)).count() - 1;
    let n = (start..LEVELS.len())
        .take_while(|&k| LEVELS[k].chapter == LEVELS[i].chapter)
        .count();
    (chapter, i - start, n)
}

/// Where isle `i` lies on the map.
fn isle_pos(i: usize) -> (f32, f32) {
    let (c, k, n) = place(i);
    let region = story::region_of(LEVELS[i].chapter);
    let cx = 240.0 + c as f32 * CHAPTER_GAP + region as f32 * REGION_GAP;
    let cy = if c % 2 == 0 { 250.0 } else { 350.0 };
    let spread = k as f32 - (n as f32 - 1.0) / 2.0;
    (cx + spread * ISLE_GAP, cy + if k % 2 == 0 { -40.0 } else { 40.0 })
}

fn map_width() -> f32 {
    isle_pos(LEVELS.len() - 1).0 + 260.0
}

/// The isle selected on the map (the first, off the map).
fn selected() -> usize {
    with_state(|s| match s.screen {
        Screen::Menu { selected } => selected,
        _ => 0,
    })
}

/// Sails the ferry to the selected isle, and the camera after it.
pub(crate) fn follow(ui: Ui, motion: &Motion, animate: bool) {
    let Some(selected) = with_state(|s| match s.screen {
        Screen::Menu { selected } => Some(selected),
        _ => None,
    }) else {
        return;
    };
    let (x, y) = isle_pos(selected);
    let (vw, vh) = ui.stage.get_untracked();
    let cam_x = (vw / 2.0 - x).clamp((vw - map_width()).min(0.0), 0.0);
    let cam_y = (vh / 2.0 - y).clamp((vh - MAP_H).min(0.0), 0.0);
    // The ferry moors just off the isle's southern shore.
    let (fx, fy) = (x - 52.0, y + 14.0);
    if animate {
        let ease = Easing::Curve(Curve::Cubic, EaseMode::InOut);
        motion.hero_x.timing(fx, 520.0, ease);
        motion.hero_y.timing(fy, 520.0, ease);
        motion.cam_x.timing(cam_x, 600.0, ease);
        motion.cam_y.timing(cam_y, 600.0, ease);
    } else {
        motion.hero_x.set(fx);
        motion.hero_y.set(fy);
        motion.cam_x.set(cam_x);
        motion.cam_y.set(cam_y);
    }
}

pub(crate) fn menu_screen(ui: Ui, motion: &Rc<Motion>) -> impl IntoElement {
    view()
        .flex_1()
        .p(px(18.0))
        .gap_3()
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .items_end()
                .gap_4()
                .child(
                    text("HJKL QUEST")
                        .text_size(px(34.0))
                        .font_weight(FontWeight::BLACK)
                        .letter_spacing(px(5.0))
                        .text_color(hex(CREAM)),
                )
                .child(
                    text("The Buffer Archipelago: learn vim one isle at a time.")
                        .text_size(px(14.0))
                        .text_color(hexa(TEXT, 0.7))
                        .mb(px(5.0)),
                )
                .child(view().flex_1())
                .child(info::summary(ui)),
        )
        .child(stage(ui, motion))
        .child(info::isle_info(ui))
        .child(info::keys_legend(ui))
}

/// The sea under a moving camera, and the ferry on it.
fn stage(ui: Ui, motion: &Rc<Motion>) -> impl IntoElement {
    let ferry = motion.clone();
    let layout = motion.clone();
    view()
        .flex_1()
        .overflow_hidden()
        .rounded(px(16.0))
        .bg(hexa(0x0a2a4a, 0.55))
        .border_1()
        .border_color(hexa(LAVENDER, 0.14))
        .on_layout(move |b: &Bounds<Points>| {
            ui.stage.set((b.size.width.0, b.size.height.0));
            follow(ui, &layout, false);
        })
        .child(
            view()
                .absolute()
                .top_0()
                .left_0()
                .w(px(map_width()))
                .h(px(MAP_H))
                .bind_translate_x(&motion.cam_x)
                .bind_translate_y(&motion.cam_y)
                .child(cover_canvas(canvas(move || {
                    ui.tick.get();
                    let selected = selected();
                    let best = with_state(|s| s.best.clone());
                    draw::draw_map(&best, selected)
                })))
                .child(cover_canvas(canvas(move || draw::draw_ferry(&ferry)))),
        )
}
