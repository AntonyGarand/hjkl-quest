//! A chapter's boss, perched over its last isle: its name, its health
//! (what's left to do), and a flinch when a command hurts it.

use std::f32::consts::TAU;
use std::rc::Rc;

use nsh_core::prelude::*;

use crate::story;
use crate::ui::colors::{BLACK, BOSS, INK, NIGHT, TEXT, WHITE, hex, hexa, shade};
use crate::ui::draw::EllipseAt;
use crate::ui::motion::{Motion, Ui};
use crate::ui::state::{is_boss, session_value, with_state, work};

/// The health bar's width.
const BAR_W: f32 = 160.0;

pub(crate) fn boss_banner(ui: Ui, motion: Rc<Motion>) -> impl IntoElement {
    dynamic(move || {
        ui.scene.get();
        let (boss, color) = session_value(|s| {
            let chapter = story::chapter(s.level().chapter)?;
            is_boss(s).then_some((chapter.boss, chapter.theme.stone))
        })??;
        let flinch = motion.boss.mul(10.0);
        Some(
            view()
                .absolute()
                .top(px(12.0))
                .left(px(12.0))
                .flex_row()
                .items_center()
                .gap_3()
                .px(px(12.0))
                .py(px(8.0))
                .rounded(px(12.0))
                .bg(hexa(NIGHT, 0.7))
                .border_1()
                .border_color(hexa(BOSS, 0.45))
                .child(
                    canvas(move || draw_monster(color))
                        .w(px(54.0))
                        .h(px(46.0))
                        .bind_translate_x(&flinch),
                )
                .child(health_view(ui, boss)),
        )
    })
}

/// How much is left to do, out of what the isle opened with: 0 to 1.
fn health(ui: Ui) -> f32 {
    ui.tick.get();
    let left = session_value(work).unwrap_or(0);
    let full = with_state(|s| s.work_start).max(left).max(1);
    left as f32 / full as f32
}

/// The boss's name over its health bar.
fn health_view(ui: Ui, boss: &'static str) -> impl IntoElement {
    view()
        .gap_1()
        .child(
            text(boss)
                .text_size(px(14.0))
                .font_weight(FontWeight::BOLD)
                .text_color(hex(BOSS)),
        )
        .child(
            view()
                .w(px(BAR_W))
                .h(px(8.0))
                .rounded(px(4.0))
                .bg(hexa(WHITE, 0.12))
                .overflow_hidden()
                .child(
                    view()
                        .h_full()
                        .rounded(px(4.0))
                        .bg(hex(0xff6b5a))
                        .w(move || px(BAR_W * health(ui).clamp(0.0, 1.0))),
                ),
        )
        .child(
            text(move || {
                let h = health(ui);
                if h <= 0.0 {
                    "defeated!".to_string()
                } else {
                    format!("{:.0}% health: every crystal, bug or fix is a hit", h * 100.0)
                }
            })
            .text_size(px(11.0))
            .text_color(hexa(TEXT, 0.6)),
        )
}

/// A chapter boss: a horned blob with glowing eyes, tinted like the
/// chapter's stone.
fn draw_monster(color: u32) -> Painter {
    let (cx, cy) = (27.0, 26.0);
    Painter::new()
        .fill_color(hexa(BLACK, 0.3))
        .begin_path()
        .ellipse_at(cx, 42.0, 18.0, 3.5)
        .fill()
        // Horns.
        .fill_color(hex(0xe8dcc0))
        .begin_path()
        .move_to(cx - 14.0, cy - 8.0)
        .line_to(cx - 20.0, cy - 22.0)
        .line_to(cx - 8.0, cy - 13.0)
        .close_path()
        .move_to(cx + 14.0, cy - 8.0)
        .line_to(cx + 20.0, cy - 22.0)
        .line_to(cx + 8.0, cy - 13.0)
        .close_path()
        .fill()
        // Body and belly.
        .fill_color(shade(color, 1.2))
        .begin_path()
        .ellipse_at(cx, cy, 19.0, 15.0)
        .fill()
        .fill_color(shade(color, 0.8))
        .begin_path()
        .ellipse_at(cx, cy + 7.0, 15.0, 7.0)
        .fill()
        // Eyes.
        .fill_color(hex(0xffe066))
        .begin_path()
        .arc(cx - 7.0, cy - 3.0, 3.6, 0.0, TAU)
        .arc(cx + 7.0, cy - 3.0, 3.6, 0.0, TAU)
        .fill()
        .fill_color(hex(INK))
        .begin_path()
        .arc(cx - 6.0, cy - 3.0, 1.6, 0.0, TAU)
        .arc(cx + 8.0, cy - 3.0, 1.6, 0.0, TAU)
        .fill()
        // Teeth.
        .fill_color(hex(WHITE))
        .begin_path()
        .move_to(cx - 6.0, cy + 5.0)
        .line_to(cx - 3.0, cy + 9.0)
        .line_to(cx, cy + 5.0)
        .line_to(cx + 3.0, cy + 9.0)
        .line_to(cx + 6.0, cy + 5.0)
        .close_path()
        .fill()
}
