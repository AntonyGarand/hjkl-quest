//! The sky behind every screen: the chapter's colours, a sun or a moon and
//! stars, drifting clouds, and its weather.

use std::f32::consts::TAU;

use nsh_core::canvas::CanvasGradient;
use nsh_core::prelude::*;

use super::EllipseAt;
use crate::story::{Theme, Weather};
use crate::ui::colors::{WHITE, hex, hexa, noise};
use crate::ui::motion::Motion;
use crate::vim::Pos;

pub(crate) fn draw_sky(size: (f32, f32), motion: &Motion, theme: Theme) -> Painter {
    let (w, h) = size;
    let [top, mid, low] = theme.sky;
    let sky = CanvasGradient::linear(point(px(0.0), px(0.0)), point(px(0.0), px(h)))
        .add_color_stop(percentage(0.0), hex(top))
        .add_color_stop(percentage(0.6), hex(mid))
        .add_color_stop(percentage(1.0), hex(low));
    let painter = Painter::new().fill_style(sky).fill_rect(0.0, 0.0, w, h);
    let at = (w * 0.82, h * 0.16);
    let painter = if theme.day {
        draw_sun(painter, at)
    } else {
        draw_stars(draw_moon(painter, at, mid), size, motion)
    };
    draw_weather(draw_clouds(painter, size, motion), size, motion, theme.weather)
}

/// A soft glow of `color` around (`x`, `y`).
fn draw_glow(painter: Painter, (x, y): (f32, f32), r: f32, color: u32, alpha: f32) -> Painter {
    painter
        .fill_style(
            CanvasGradient::radial(point(px(x), px(y)), px(10.0), point(px(x), px(y)), px(r))
                .add_color_stop(percentage(0.0), hexa(color, alpha))
                .add_color_stop(percentage(1.0), hexa(color, 0.0)),
        )
        .begin_path()
        .arc(x, y, r, 0.0, TAU)
        .fill()
}

/// A low, warm sun.
fn draw_sun(painter: Painter, (x, y): (f32, f32)) -> Painter {
    draw_glow(painter, (x, y), 120.0, 0xffe6a8, 0.45)
        .fill_color(hex(0xfff0c4))
        .begin_path()
        .arc(x, y, 26.0, 0.0, TAU)
        .fill()
}

/// A crescent moon: a disc with the sky's `mid` colour biting into it.
fn draw_moon(painter: Painter, (x, y): (f32, f32), mid: u32) -> Painter {
    draw_glow(painter, (x, y), 90.0, 0xfff6d8, 0.35)
        .fill_color(hex(0xfff3cf))
        .begin_path()
        .arc(x, y, 22.0, 0.0, TAU)
        .fill()
        .fill_color(hexa(mid, 0.9))
        .begin_path()
        .arc(x + 9.0, y - 5.0, 19.0, 0.0, TAU)
        .fill()
}

/// Stars, in two groups that twinkle out of step.
fn draw_stars(mut painter: Painter, (w, h): (f32, f32), motion: &Motion) -> Painter {
    for (group, alpha) in [(0usize, &motion.twinkle_a), (1usize, &motion.twinkle_b)] {
        painter = painter.save().global_alpha(alpha).fill_color(hex(0xfdf8ff));
        for i in 0..70 {
            let seed = i * 2 + group;
            let fx = noise(Pos { row: seed, col: 7 });
            let fy = noise(Pos {
                row: 3,
                col: seed + 11,
            });
            let r = 0.5 + noise(Pos { row: seed, col: seed }) * 1.3;
            painter = painter
                .begin_path()
                .arc(fx * w, fy * h * 0.75, r, 0.0, TAU)
                .fill();
        }
        painter = painter.restore();
    }
    painter
}

/// Drifting clouds low on the horizon.
fn draw_clouds(mut painter: Painter, size: (f32, f32), motion: &Motion) -> Painter {
    let (w, h) = size;
    let bands = [
        (0.78, 0.5, 0.10, 1.0),
        (0.88, 0.7, 0.08, 0.6),
        (0.95, 0.45, 0.12, 1.4),
    ];
    for (k, (y, len, alpha, speed)) in bands.into_iter().enumerate() {
        let offset = motion.drift.mul(w * speed);
        painter = painter
            .save()
            .translate(&offset, 0.0)
            .fill_color(hexa(0xd9c8ff, alpha));
        for copy in [-1.0f32, 0.0] {
            let x0 = copy * w * speed + k as f32 * 140.0;
            for b in 0..5 {
                let bx = x0 + b as f32 * w * len / 5.0;
                painter = painter
                    .begin_path()
                    .ellipse_at(bx, h * y, w * len / 4.0, 18.0 + (b % 2) as f32 * 8.0)
                    .fill();
            }
        }
        painter = painter.restore();
    }
    painter
}

/// The chapter's weather: fireflies and motes drift; snow, rain, leaves
/// and embers fall (or rise) on a loop.
fn draw_weather(painter: Painter, size: (f32, f32), motion: &Motion, weather: Weather) -> Painter {
    match weather {
        Weather::Fireflies => draw_drifters(painter, size, motion, 0xfff7a0),
        Weather::Motes => draw_drifters(painter, size, motion, 0xcfe0ff),
        _ => draw_falling(painter, size, motion, weather),
    }
}

/// Two copies a screen apart, moving one screen per loop: seamless.
fn draw_falling(mut painter: Painter, (w, h): (f32, f32), motion: &Motion, weather: Weather) -> Painter {
    let rising = weather == Weather::Embers;
    let shift = if rising {
        motion.fall.mul(-h)
    } else {
        motion.fall.mul(h)
    };
    painter = painter.save().translate(0.0, &shift);
    for copy in [0.0f32, if rising { 1.0 } else { -1.0 }] {
        for i in 0..60 {
            let x = noise(Pos { row: i + 90, col: 13 }) * w;
            let y = noise(Pos {
                row: 17,
                col: i + 120,
            }) * h
                + copy * h;
            let s = 0.6 + noise(Pos { row: i, col: 99 });
            painter = draw_drop(painter, weather, (x, y), s, i % 2 == 0);
        }
    }
    painter.restore()
}

/// One snowflake, raindrop, leaf or ember, of size `s`; `even` picks a
/// leaf's colour.
fn draw_drop(painter: Painter, weather: Weather, (x, y): (f32, f32), s: f32, even: bool) -> Painter {
    match weather {
        Weather::Snow => painter
            .fill_color(hexa(WHITE, 0.75))
            .begin_path()
            .arc(x, y, 1.4 * s + 0.6, 0.0, TAU)
            .fill(),
        Weather::Rain => painter
            .stroke_color(hexa(0xb8d0ff, 0.35))
            .line_width(1.0)
            .begin_path()
            .move_to(x, y)
            .line_to(x - 3.0, y + 12.0 * s)
            .stroke(),
        Weather::Leaves => painter
            .fill_color(hexa(if even { 0xd98a3a } else { 0xc9b04a }, 0.7))
            .begin_path()
            .ellipse_at(x, y, 3.5 * s, 1.8 * s)
            .fill(),
        _ => painter
            .fill_color(hexa(0xffa04a, 0.7))
            .begin_path()
            .arc(x, y, 1.2 * s, 0.0, TAU)
            .fill(),
    }
}

/// Fireflies or motes, glowing `glow`, wandering together.
fn draw_drifters(mut painter: Painter, (w, h): (f32, f32), motion: &Motion, glow: u32) -> Painter {
    painter = painter.save().translate(&motion.fly_x, &motion.fly_y);
    for i in 0..14 {
        let fx = noise(Pos { row: i + 40, col: 3 }) * w;
        let fy = h * 0.35 + noise(Pos { row: 5, col: i + 60 }) * h * 0.6;
        painter = painter
            .fill_style(
                CanvasGradient::radial(point(px(fx), px(fy)), px(0.5), point(px(fx), px(fy)), px(7.0))
                    .add_color_stop(percentage(0.0), hexa(glow, 0.9))
                    .add_color_stop(percentage(1.0), hexa(glow, 0.0)),
            )
            .begin_path()
            .arc(fx, fy, 7.0, 0.0, TAU)
            .fill();
    }
    painter.restore()
}
