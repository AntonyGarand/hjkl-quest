//! HJKL Quest: learn vim by walking the Buffer Archipelago.
//!
//! The world is the text buffer: each character is a tile, and the cursor
//! is a small wisp that hops between them. `vim` is the editor, `levels`
//! the campaign, `game` a level being played; this file draws it all with
//! Native Shell.

pub mod audio;
pub mod game;
pub mod levels;
pub mod vim;

use std::cell::RefCell;
use std::f32::consts::TAU;
use std::rc::Rc;


use nsh_core::element::CanvasArg;
use nsh_core::canvas::{CanvasGradient, CanvasTextAlign, CanvasTextBaseline};
use nsh_core::storage::Storage;
use nsh_core::prelude::*;

use audio::Sfx;
use game::{Event, Session};
use levels::LEVELS;
use vim::{Key, Mode, Pos, format_keys};

/// The app's UI set (`nsh.json` `"rust"`).
#[native_module("hjkl-quest/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

// ------------------------------------------------------------------ layout

/// Tile width, top-face depth, block thickness and the skew per row that
/// gives the board its three-quarter look.
const CW: f32 = 40.0;
const CH: f32 = 30.0;
const DEPTH: f32 = 15.0;
const SKEW: f32 = 12.0;
const PAD_X: f32 = 48.0;
const PAD_Y: f32 = 64.0;
/// Rows the board makes room for (the skew is measured from the last one).
const ROWS_ROOM: f32 = 16.0;

fn tile_origin(p: Pos) -> (f32, f32) {
    (
        PAD_X + (ROWS_ROOM - p.row as f32) * SKEW + p.col as f32 * CW,
        PAD_Y + p.row as f32 * CH,
    )
}

fn tile_center(p: Pos) -> (f32, f32) {
    let (x, y) = tile_origin(p);
    (x + CW / 2.0 + SKEW / 2.0, y + CH / 2.0)
}

fn world_size(rows: usize, cols: usize) -> (f32, f32) {
    (
        PAD_X * 2.0 + ROWS_ROOM * SKEW + cols as f32 * CW + CW,
        PAD_Y + rows as f32 * CH + DEPTH + 56.0,
    )
}

// ------------------------------------------------------------------ colors

fn hex(c: u32) -> Hsla {
    rgb(c).into()
}

fn hexa(c: u32, a: f32) -> Hsla {
    hex(c).opacity(a)
}

fn shade(c: u32, k: f32) -> Hsla {
    let r = ((c >> 16) & 0xff) as f32 * k;
    let g = ((c >> 8) & 0xff) as f32 * k;
    let b = (c & 0xff) as f32 * k;
    let v = ((r.min(255.0) as u32) << 16) | ((g.min(255.0) as u32) << 8) | (b.min(255.0) as u32);
    hex(v)
}

fn noise(p: Pos) -> f32 {
    let mut h = (p.row as u32).wrapping_mul(73_856_093) ^ (p.col as u32).wrapping_mul(19_349_663);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    ((h >> 8) & 0xffff) as f32 / 65_535.0
}

const SKY_TOP: u32 = 0x0a0f24;
const SKY_MID: u32 = 0x221a46;
const SKY_LOW: u32 = 0x40285c;
const GRASS: u32 = 0x4c8f57;
const STONE: u32 = 0x5f6f93;
const AMBER: u32 = 0x9a7440;
const TEAL: u32 = 0x2f8a8f;
const WOOD: u32 = 0x86603a;
const EMBER: u32 = 0xc25a3f;
const SELECT: u32 = 0x8a63e0;
const GEM: u32 = 0x7ef3ff;
const GLYPH: u32 = 0xf6f1e3;

fn mode_color(e: &vim::Editor) -> u32 {
    match e.mode {
        Mode::Insert => 0x7dffa8,
        Mode::Visual { .. } => 0xc9a2ff,
        Mode::Command { .. } => 0x8ec5ff,
        Mode::Normal if !e.pending().is_empty() => 0xffb86b,
        Mode::Normal => 0xfff3b0,
    }
}

// ------------------------------------------------------------------- state

enum Screen {
    Menu { selected: usize },
    Play(Box<Session>),
}

struct State {
    screen: Screen,
    /// The fewest keystrokes each level was solved in.
    best: Vec<Option<usize>>,
    toast: String,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        screen: start_screen(),
        best: load_progress(),
        toast: String::new(),
    });
}

/// The menu, or with `HJKL_LEVEL=<n>` (desktop, for development) level n.
fn start_screen() -> Screen {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(n) = std::env::var("HJKL_LEVEL").ok().and_then(|v| v.parse::<usize>().ok())
        && (1..=LEVELS.len()).contains(&n) {
            return Screen::Play(Box::new(Session::new(n - 1)));
        }
    Screen::Menu { selected: 0 }
}

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

fn load_progress() -> Vec<Option<usize>> {
    let mut best = vec![None; LEVELS.len()];
    if let Some(saved) = Storage::get_item("hjkl-quest.best") {
        for entry in saved.split(',') {
            if let Some((i, n)) = entry.split_once(':')
                && let (Ok(i), Ok(n)) = (i.parse::<usize>(), n.parse::<usize>())
                    && i < best.len() {
                        best[i] = Some(n);
                    }
        }
    }
    best
}

fn save_progress(best: &[Option<usize>]) {
    let text = best
        .iter()
        .enumerate()
        .filter_map(|(i, b)| b.map(|n| format!("{i}:{n}")))
        .collect::<Vec<_>>()
        .join(",");
    let _ = Storage::set_item("hjkl-quest.best", text);
}

/// The animated values the board moves with.
struct Motion {
    hero_x: AnimatedValue,
    hero_y: AnimatedValue,
    hop: AnimatedValue,
    cam_x: AnimatedValue,
    cam_y: AnimatedValue,
    burst: AnimatedValue,
    shake: AnimatedValue,
    bob: AnimatedValue,
    float: AnimatedValue,
    twinkle_a: AnimatedValue,
    twinkle_b: AnimatedValue,
    fly_x: AnimatedValue,
    fly_y: AnimatedValue,
    drift: AnimatedValue,
    card: AnimatedValue,
}

#[derive(Clone, Copy)]
struct Ui {
    /// Bumped after every key: what the board and panels redraw from.
    tick: Signal<u64>,
    /// Bumped when the screen or the level changes.
    scene: Signal<u64>,
    stage: Signal<(f32, f32)>,
    window: Signal<(f32, f32)>,
    burst_at: Signal<(f32, f32)>,
}

// --------------------------------------------------------------------- keys

fn key_table() -> Vec<(String, Key)> {
    let mut keys = Vec::new();
    for c in 'a'..='z' {
        keys.push((c.to_string(), Key::Char(c)));
        keys.push((format!("shift+{c}"), Key::Char(c.to_ascii_uppercase())));
    }
    let shifted_digits = [')', '!', '@', '#', '$', '%', '^', '&', '*', '('];
    for d in 0..10u32 {
        let c = char::from_digit(d, 10).unwrap();
        keys.push((c.to_string(), Key::Char(c)));
        keys.push((format!("shift+{c}"), Key::Char(shifted_digits[d as usize])));
    }
    for (plain, shifted, name) in [
        ('`', '~', "`"),
        ('-', '_', "minus"),
        ('=', '+', "="),
        ('[', '{', "["),
        (']', '}', "]"),
        ('\\', '|', "\\"),
        (';', ':', ";"),
        ('\'', '"', "'"),
        (',', '<', ","),
        ('.', '>', "."),
        ('/', '?', "/"),
    ] {
        keys.push((name.to_string(), Key::Char(plain)));
        keys.push((format!("shift+{name}"), Key::Char(shifted)));
    }
    keys.push(("space".into(), Key::Char(' ')));
    keys.push(("shift+space".into(), Key::Char(' ')));
    keys.push(("escape".into(), Key::Esc));
    keys.push(("ctrl+[".into(), Key::Esc));
    keys.push(("ctrl+c".into(), Key::Esc));
    keys.push(("enter".into(), Key::Enter));
    keys.push(("backspace".into(), Key::Backspace));
    keys.push(("ctrl+r".into(), Key::CtrlR));
    keys
}

fn bind_keys(ui: Ui, motion: Rc<Motion>) {
    let apple = apple_keys();
    for (stroke, key) in key_table() {
        let Ok(parsed) = nsh_core::keymap::Keystroke::parse_with(&stroke, apple) else { continue };
        if parsed.reserved(os()).is_some() {
            continue;
        }
        let motion = motion.clone();
        on_key_with(
            &stroke,
            KeyOptions { when: nsh_core::keymap::When::Always, repeat: true, pass_through: false },
            move |_| handle_key(key, ui, &motion),
        );
    }
}

fn handle_key(key: Key, ui: Ui, motion: &Motion) {
    let mut scene_changed = false;
    let event = with_state(|state| {
        state.toast.clear();
        match &mut state.screen {
            Screen::Menu { selected } => {
                let n = LEVELS.len();
                match key {
                    Key::Char('j') => *selected = (*selected + 1).min(n - 1),
                    Key::Char('k') => *selected = selected.saturating_sub(1),
                    Key::Char('G') => *selected = n - 1,
                    Key::Char('g') => *selected = 0,
                    Key::Char('m') => {
                        audio::set_muted(!audio::muted());
                    }
                    Key::Enter | Key::Char('l') | Key::Char(' ') => {
                        let index = *selected;
                        state.screen = Screen::Play(Box::new(Session::new(index)));
                        scene_changed = true;
                        return Some(Event::Moved);
                    }
                    _ => return None,
                }
                Some(Event::Moved)
            }
            Screen::Play(session) => {
                if session.complete {
                    let index = session.index;
                    let next = match key {
                        Key::Enter | Key::Char('n') => Some((index + 1).min(LEVELS.len() - 1)),
                        Key::Char('r') => Some(index),
                        Key::Char('q') | Key::Esc => None,
                        _ => return None,
                    };
                    state.screen = match next {
                        Some(i) if !(index + 1 == LEVELS.len() && key != Key::Char('r')) => Screen::Play(Box::new(Session::new(i))),
                        _ => Screen::Menu { selected: index },
                    };
                    scene_changed = true;
                    return Some(Event::Moved);
                }
                let event = session.press(key);
                match &event {
                    Event::Command(cmd) => {
                        let cmd = cmd.trim().to_string();
                        match cmd.as_str() {
                            "q" | "q!" | "wq" | "x" | "levels" | "menu" => {
                                let index = session.index;
                                state.screen = Screen::Menu { selected: index };
                                scene_changed = true;
                            }
                            "e!" | "restart" | "r" => {
                                let index = session.index;
                                state.screen = Screen::Play(Box::new(Session::new(index)));
                                scene_changed = true;
                            }
                            "n" | "next" => {
                                let index = (session.index + 1).min(LEVELS.len() - 1);
                                state.screen = Screen::Play(Box::new(Session::new(index)));
                                scene_changed = true;
                            }
                            "mute" | "sound" => {
                                audio::set_muted(!audio::muted());
                                state.toast = if audio::muted() { "Sound off".into() } else { "Sound on".into() };
                            }
                            "help" | "h" => {
                                state.toast = ":q menu  ·  :e! restart  ·  :n skip  ·  :mute sound".into();
                            }
                            "w" => state.toast = "Saved! (progress saves by itself)".into(),
                            other => state.toast = format!("E492: Not an editor command: {other}"),
                        }
                    }
                    Event::Complete => {
                        let i = session.index;
                        let n = session.keys.len();
                        let best = &mut state.best[i];
                        if best.is_none_or(|b| n < b) {
                            *best = Some(n);
                        }
                        let best = state.best.clone();
                        save_progress(&best);
                    }
                    _ => {}
                }
                Some(event)
            }
        }
    });
    let Some(event) = event else { return };

    // Sound and motion.
    match &event {
        Event::Moved => audio::step(ui.tick.get_untracked() as usize),
        Event::Edited => audio::play(Sfx::Edit, 0.5),
        Event::Gem(p) => {
            audio::play(Sfx::Gem, 0.55);
            let (cx, cy) = tile_center(*p);
            ui.burst_at.set((cx, cy));
            motion.burst.set(0.0);
            motion.burst.timing(1.0, 650.0, Easing::Curve(Curve::Cubic, EaseMode::Out));
        }
        Event::Bell => {
            audio::play(Sfx::Bell, 0.35);
            motion.shake.keyframes(
                [Keyframe::new(0.0), Keyframe::new(-4.0), Keyframe::new(4.0), Keyframe::new(-2.0), Keyframe::new(0.0)],
                KeyframeTiming::new(220.0),
            );
        }
        Event::Locked(_) => {
            audio::play(Sfx::Locked, 0.45);
            motion.shake.keyframes(
                [Keyframe::new(0.0), Keyframe::new(-6.0), Keyframe::new(6.0), Keyframe::new(-3.0), Keyframe::new(0.0)],
                KeyframeTiming::new(260.0),
            );
        }
        Event::Complete => {
            audio::play(Sfx::Gem, 0.4);
            audio::play(Sfx::Complete, 0.6);
            motion.card.set(0.0);
            motion.card.spring(1.0, Spring { stiffness: 170.0, damping: 14.0, ..Spring::default() });
        }
        _ => {}
    }
    place_hero(ui, motion, !scene_changed);
    if scene_changed {
        ui.scene.update(|s| *s += 1);
    }
    ui.tick.update(|t| *t += 1);
}

/// Moves the wisp and the camera to the cursor.
fn place_hero(ui: Ui, motion: &Motion, animate: bool) {
    let Some((cursor, rows, cols)) = with_state(|s| match &s.screen {
        Screen::Play(session) => {
            let e = &session.editor;
            let rows = e.lines.len().max(session.level().goal_text().map_or(0, |g| g.lines().count()));
            let cols = e.lines.iter().map(|l| l.len()).max().unwrap_or(0);
            Some((e.cursor, rows, cols))
        }
        Screen::Menu { .. } => None,
    }) else {
        return;
    };
    let (x, y) = tile_origin(cursor);
    if animate {
        motion.hero_x.timing(x, 120.0, Easing::Curve(Curve::Cubic, EaseMode::Out));
        motion.hero_y.timing(y, 120.0, Easing::Curve(Curve::Cubic, EaseMode::Out));
        motion.hop.keyframes(
            [Keyframe::new(0.0), Keyframe::at(0.4, -9.0).ease(Easing::EaseOut), Keyframe::new(0.0)],
            KeyframeTiming::new(200.0).easing(Easing::EaseIn),
        );
    } else {
        motion.hero_x.set(x);
        motion.hero_y.set(y);
    }

    // Camera: centre the board when it fits; otherwise follow the cursor.
    let (vw, vh) = ui.stage.get_untracked();
    let (ww, wh) = world_size(rows, cols.max(8));
    let (cx, cy) = tile_center(cursor);
    let target_x = if ww <= vw { (vw - ww) / 2.0 } else { (vw / 2.0 - cx).clamp(vw - ww, 0.0) };
    let target_y = if wh <= vh { (vh - wh) / 2.0 } else { (vh / 2.0 - cy).clamp(vh - wh, 0.0) };
    if animate {
        motion.cam_x.timing(target_x, 380.0, Easing::Curve(Curve::Cubic, EaseMode::Out));
        motion.cam_y.timing(target_y, 380.0, Easing::Curve(Curve::Cubic, EaseMode::Out));
    } else {
        motion.cam_x.set(target_x);
        motion.cam_y.set(target_y);
    }
}

// ------------------------------------------------------------------ drawing

/// The longest common subsequence of `a` and `b`, as pairs of indices.
fn lcs<T: PartialEq>(a: &[T], b: &[T]) -> Vec<(usize, usize)> {
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![vec![0u16; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] { dp[i + 1][j + 1] + 1 } else { dp[i + 1][j].max(dp[i][j + 1]) };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < n && j < m {
        if a[i] == b[j] {
            out.push((i, j));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// Which characters of the buffer are not where the goal wants them: lines
/// are matched first, then the characters of the lines that differ.
fn wrong_marks(buf: &[Vec<char>], goal: &[Vec<char>]) -> Vec<Vec<bool>> {
    let mut wrong: Vec<Vec<bool>> = buf.iter().map(|l| vec![true; l.len()]).collect();
    let same = lcs(buf, goal);
    // Between matched lines, pair the leftover lines in order.
    let mut anchors = same.clone();
    anchors.push((buf.len(), goal.len()));
    let (mut bi, mut gi) = (0, 0);
    for (ba, ga) in anchors {
        let (bs, gs) = (bi..ba, gi..ga);
        for (b, g) in bs.clone().zip(gs.clone()) {
            for (i, _) in lcs(&buf[b], &goal[g]) {
                wrong[b][i] = false;
            }
        }
        if ba < buf.len() {
            wrong[ba].iter_mut().for_each(|w| *w = false);
        }
        bi = ba + 1;
        gi = ga + 1;
    }
    wrong
}

fn draw_world(session: &Session) -> Painter {
    let e = &session.editor;
    let goal: Option<Vec<Vec<char>>> = session.level().goal_text().map(|g| g.split('\n').map(|l| l.chars().collect()).collect());
    let selection = match e.mode {
        Mode::Visual { line } => {
            let (s, t) = if e.anchor <= e.cursor { (e.anchor, e.cursor) } else { (e.cursor, e.anchor) };
            Some((s, t, line))
        }
        _ => None,
    };
    let selected = |p: Pos| match selection {
        Some((s, t, true)) => p.row >= s.row && p.row <= t.row,
        Some((s, t, false)) => p >= s && p <= t,
        None => false,
    };

    let wrong = goal.as_ref().map(|g| wrong_marks(&e.lines, g));
    let mut painter = Painter::new();
    let rows = e.lines.len().max(goal.as_ref().map_or(0, |g| g.len()));

    // Soft shadow of the island on the clouds below.
    for row in 0..e.lines.len() {
        let len = e.lines[row].len().max(1);
        let (x, y) = tile_origin(Pos { row, col: 0 });
        painter = painter
            .fill_color(hexa(0x000000, 0.18))
            .begin_path()
            .ellipse_at(x + len as f32 * CW / 2.0 + 10.0, y + CH + DEPTH + 22.0, len as f32 * CW / 2.0 + 8.0, 7.0)
            .fill();
    }

    for row in 0..rows {
        let line: &[char] = e.lines.get(row).map_or(&[], |l| l.as_slice());
        let goal_line: Option<&[char]> = goal.as_ref().map(|g| g.get(row).map_or(&[][..], |l| l.as_slice()));
        let width = line.len().max(goal_line.map_or(0, |g| g.len()));
        if line.is_empty() && row < e.lines.len() {
            painter = draw_tile(painter, Pos { row, col: 0 }, None, TileLook::Plank);
        }
        for col in 0..width {
            let p = Pos { row, col };
            let c = line.get(col).copied();
            let want = goal_line.and_then(|g| g.get(col).copied());
            let is_wrong = wrong.as_ref().is_some_and(|w| w.get(row).and_then(|l| l.get(col)).copied().unwrap_or(false));
            let look = match c {
                None => {
                    // Ghosts only where the goal has lines the buffer lacks.
                    if let (Some(w), true) = (want, row >= e.lines.len()) {
                        painter = draw_ghost(painter, p, w);
                    }
                    continue;
                }
                Some(_) if selected(p) => TileLook::Selected,
                Some(_) if is_wrong => TileLook::Wrong,
                Some(ch) => TileLook::of(ch),
            };
            painter = draw_tile(painter, p, c, look);
        }
    }
    painter
}

#[derive(Clone, Copy, PartialEq)]
enum TileLook {
    Grass,
    Flower,
    Stone,
    Amber,
    Teal,
    Plank,
    Wrong,
    Selected,
}

impl TileLook {
    fn of(c: char) -> TileLook {
        match c {
            ' ' => TileLook::Grass,
            '.' => TileLook::Flower,
            c if c.is_ascii_digit() => TileLook::Teal,
            c if c.is_alphanumeric() || c == '_' => TileLook::Stone,
            _ => TileLook::Amber,
        }
    }

    fn color(self) -> u32 {
        match self {
            TileLook::Grass | TileLook::Flower => GRASS,
            TileLook::Stone => STONE,
            TileLook::Amber => AMBER,
            TileLook::Teal => TEAL,
            TileLook::Plank => WOOD,
            TileLook::Wrong => EMBER,
            TileLook::Selected => SELECT,
        }
    }
}

fn draw_tile(painter: Painter, p: Pos, c: Option<char>, look: TileLook) -> Painter {
    let (x, y) = tile_origin(p);
    let base = look.color();
    let k = 0.92 + noise(p) * 0.16;
    let top = shade(base, k);
    let front = shade(base, k * 0.62);
    let side = shade(base, k * 0.48);
    let mut painter = painter
        // Left side, seen where the block has no neighbour.
        .fill_color(side)
        .begin_path()
        .move_to(x, y + CH)
        .line_to(x + SKEW, y)
        .line_to(x + SKEW, y + DEPTH)
        .line_to(x, y + CH + DEPTH)
        .close_path()
        .fill()
        // Front.
        .fill_color(front)
        .fill_rect(x, y + CH, CW, DEPTH)
        // Top.
        .fill_color(top)
        .begin_path()
        .move_to(x + SKEW, y)
        .line_to(x + SKEW + CW, y)
        .line_to(x + CW, y + CH)
        .line_to(x, y + CH)
        .close_path()
        .fill()
        // A lighter rim on the back edge.
        .stroke_color(hexa(0xffffff, 0.10))
        .line_width(1.0)
        .begin_path()
        .move_to(x + SKEW + 1.0, y + 0.5)
        .line_to(x + SKEW + CW - 1.0, y + 0.5)
        .stroke()
        .stroke_color(hexa(0x000000, 0.22))
        .begin_path()
        .move_to(x + CW, y + CH)
        .line_to(x + CW, y + CH + DEPTH)
        .stroke();
    let (cx, cy) = (x + CW / 2.0 + SKEW / 2.0, y + CH / 2.0);
    match look {
        TileLook::Flower => {
            let hue = [0xffd6e7, 0xfff1a8, 0xd6e4ff][(noise(p) * 3.0) as usize % 3];
            painter = painter
                .fill_color(hex(hue))
                .begin_path()
                .arc(cx - 3.0 + noise(Pos { row: p.col, col: p.row }) * 6.0, cy, 1.8, 0.0, TAU)
                .fill();
        }
        TileLook::Grass => {
            painter = painter
                .stroke_color(hexa(0xb8f0a0, 0.35))
                .line_width(1.0)
                .begin_path()
                .move_to(cx - 4.0, cy + 3.0)
                .line_to(cx - 3.0, cy - 1.0)
                .move_to(cx + 3.0, cy + 2.0)
                .line_to(cx + 4.0, cy - 2.0)
                .stroke();
        }
        TileLook::Plank => {
            painter = painter
                .stroke_color(hexa(0x3a2410, 0.6))
                .line_width(1.0)
                .begin_path()
                .move_to(x + SKEW * 0.66 + 1.0, y + CH / 3.0)
                .line_to(x + SKEW * 0.66 + CW - 1.0, y + CH / 3.0)
                .move_to(x + SKEW * 0.33 + 1.0, y + CH * 2.0 / 3.0)
                .line_to(x + SKEW * 0.33 + CW - 1.0, y + CH * 2.0 / 3.0)
                .stroke();
        }
        _ => {}
    }
    if let Some(ch) = c.filter(|c| !matches!(c, ' ' | '.')) {
        let glyph = if look == TileLook::Wrong { 0xfff4e6 } else { GLYPH };
        painter = painter
            .font("700 19px monospace")
            .text_align(CanvasTextAlign::Center)
            .text_baseline(CanvasTextBaseline::Middle)
            .fill_color(hexa(0x000000, 0.35))
            .fill_text(&ch.to_string(), cx + 1.0, cy + 1.5)
            .fill_color(hex(glyph))
            .fill_text(&ch.to_string(), cx, cy + 0.5);
    }
    painter
}

/// Where the goal has a character the buffer lacks: a faint outline.
fn draw_ghost(painter: Painter, p: Pos, want: char) -> Painter {
    let (x, y) = tile_origin(p);
    let mut painter = painter
        .stroke_color(hexa(0xffd2a8, 0.45))
        .line_width(1.0)
        .line_dash(&[px(3.0), px(3.0)])
        .begin_path()
        .move_to(x + SKEW, y)
        .line_to(x + SKEW + CW, y)
        .line_to(x + CW, y + CH)
        .line_to(x, y + CH)
        .close_path()
        .stroke()
        .line_dash(&[]);
    if want != ' ' {
        painter = painter
            .font("700 18px monospace")
            .text_align(CanvasTextAlign::Center)
            .text_baseline(CanvasTextBaseline::Middle)
            .fill_color(hexa(0xffd2a8, 0.5))
            .fill_text(&want.to_string(), x + CW / 2.0 + SKEW / 2.0, y + CH / 2.0 + 0.5);
    }
    painter
}

fn draw_gems(session: &Session, bob: &AnimatedValue) -> Painter {
    let mut painter = Painter::new();
    for &(p, got) in &session.gems {
        if got {
            continue;
        }
        let (cx, cy) = tile_center(p);
        painter = painter
            .fill_color(hexa(GEM, 0.22))
            .begin_path()
            .ellipse_at(cx, cy + 2.0, 8.0, 3.5)
            .fill()
            .save()
            .translate(cx, cy - 13.0)
            .translate(0.0, bob)
            .fill_style(
                CanvasGradient::radial(point(px(0.0), px(0.0)), px(1.0), point(px(0.0), px(0.0)), px(16.0))
                    .add_color_stop(percentage(0.0), hexa(GEM, 0.55))
                    .add_color_stop(percentage(1.0), hexa(GEM, 0.0)),
            )
            .begin_path()
            .arc(0.0, 0.0, 16.0, 0.0, TAU)
            .fill()
            .fill_color(hex(GEM))
            .begin_path()
            .move_to(0.0, -9.0)
            .line_to(6.0, -1.0)
            .line_to(0.0, 8.0)
            .line_to(-6.0, -1.0)
            .close_path()
            .fill()
            .fill_color(hexa(0xffffff, 0.75))
            .begin_path()
            .move_to(0.0, -9.0)
            .line_to(2.5, -1.0)
            .line_to(0.0, 2.0)
            .line_to(-2.5, -1.0)
            .close_path()
            .fill()
            .restore();
    }
    painter
}

fn draw_hero(session: &Session, motion: &Motion) -> Painter {
    let e = &session.editor;
    let color = mode_color(e);
    let insert = e.mode == Mode::Insert;
    let mut painter = Painter::new().save().translate(&motion.hero_x, &motion.hero_y);
    // The cursor itself: a glowing outline on the tile (a bar in insert mode).
    if insert {
        painter = painter
            .fill_color(hex(color))
            .begin_path()
            .move_to(SKEW - 1.0, 0.0)
            .line_to(SKEW + 2.0, 0.0)
            .line_to(3.0, CH)
            .line_to(0.0, CH)
            .close_path()
            .fill();
    } else {
        painter = painter
            .fill_color(hexa(color, 0.88))
            .stroke_color(hex(0xffffff))
            .line_width(1.5)
            .begin_path()
            .move_to(SKEW, 0.0)
            .line_to(SKEW + CW, 0.0)
            .line_to(CW, CH)
            .line_to(0.0, CH)
            .close_path()
            .fill()
            .stroke();
        // As vim's block cursor: the character under it, inverted.
        if let Some(ch) = e.char_at(e.cursor).filter(|c| !c.is_whitespace()) {
            painter = painter
                .font("700 19px monospace")
                .text_align(CanvasTextAlign::Center)
                .text_baseline(CanvasTextBaseline::Middle)
                .fill_color(hex(0x1a1430))
                .fill_text(&ch.to_string(), CW / 2.0 + SKEW / 2.0, CH / 2.0 + 0.5);
        }
    }
    // Its shadow on the tile.
    painter = painter
        .fill_color(hexa(0x000000, 0.25))
        .begin_path()
        .ellipse_at(CW + SKEW * 0.4 - 3.0, 6.0, 6.0, 2.0)
        .fill();
    // The wisp, floating above the tile's back edge.
    let (wx, wy) = (CW + SKEW * 0.4, -3.0);
    painter = painter
        .translate(0.0, &motion.hop)
        .fill_style(
            CanvasGradient::radial(point(px(wx), px(wy)), px(1.0), point(px(wx), px(wy)), px(18.0))
                .add_color_stop(percentage(0.0), hexa(color, 0.7))
                .add_color_stop(percentage(1.0), hexa(color, 0.0)),
        )
        .begin_path()
        .arc(wx, wy, 18.0, 0.0, TAU)
        .fill()
        .fill_color(hex(0xfffdf5))
        .begin_path()
        .arc(wx, wy, 5.5, 0.0, TAU)
        .fill()
        .fill_color(hex(0x1a1430))
        .begin_path()
        .arc(wx - 1.9, wy - 0.5, 1.0, 0.0, TAU)
        .arc(wx + 1.9, wy - 0.5, 1.0, 0.0, TAU)
        .fill()
        .restore();
    painter
}

/// The minimap's cell size for a buffer of `rows` × `cols`.
/// The minimap's cell width and height for a buffer of `rows` × `cols`.
fn minimap_cell(rows: usize, cols: usize) -> (f32, f32) {
    let rows = rows.max(1) as f32;
    let w = (240.0 / cols.max(1) as f32).min(120.0 / rows).clamp(2.0, 9.0);
    let h = (w * 1.3).max(11.0).min((120.0 / rows).max(3.0));
    (w, h)
}

fn minimap_size(session: &Session) -> (f32, f32) {
    let e = &session.editor;
    let goal_rows = session.level().goal_text().map_or(0, |g| g.lines().count());
    let rows = e.lines.len().max(goal_rows);
    let cols = e.lines.iter().map(|l| l.len()).max().unwrap_or(1).max(1);
    let (cw, ch) = minimap_cell(rows, cols);
    (cols as f32 * cw, rows as f32 * ch)
}

/// The whole isle in a corner: its tiles, the crystals left, the cursor
/// and the part the stage shows, which follows the camera natively.
fn draw_minimap(session: &Session, motion: &Motion, stage: (f32, f32)) -> Painter {
    let e = &session.editor;
    let goal_rows = session.level().goal_text().map_or(0, |g| g.lines().count());
    let rows = e.lines.len().max(goal_rows);
    let cols = e.lines.iter().map(|l| l.len()).max().unwrap_or(1).max(1);
    let (cw, ch) = minimap_cell(rows, cols);
    let (mw, mh) = (cols as f32 * cw, rows as f32 * ch);
    let goal: Option<Vec<Vec<char>>> = session.level().goal_text().map(|g| g.split('\n').map(|l| l.chars().collect()).collect());
    let wrong = goal.as_ref().map(|g| wrong_marks(&e.lines, g));
    let mut painter = Painter::new();
    for (row, line) in e.lines.iter().enumerate() {
        let y = row as f32 * ch;
        if line.is_empty() {
            painter = painter.fill_color(hexa(WOOD, 0.9)).fill_rect(0.0, y + 1.0, cw - 0.5, ch - 2.0);
        }
        for (col, &c) in line.iter().enumerate() {
            let is_wrong = wrong.as_ref().is_some_and(|w| w[row].get(col).copied().unwrap_or(false));
            let color = if is_wrong { EMBER } else { TileLook::of(c).color() };
            painter = painter.fill_color(hex(color)).fill_rect(col as f32 * cw, y, cw - 0.5, ch - 0.5);
        }
    }
    for &(p, got) in &session.gems {
        if !got {
            let (x, y) = (p.col as f32 * cw + cw / 2.0, p.row as f32 * ch + ch / 2.0);
            painter = painter.fill_color(hex(GEM)).begin_path().arc(x, y, (cw * 0.55).max(2.0), 0.0, TAU).fill();
        }
    }
    let cur = e.cursor;
    painter = painter
        .fill_color(hex(mode_color(e)))
        .stroke_color(hex(0xffffff))
        .line_width(1.0)
        .fill_rect(cur.col as f32 * cw - 1.0, cur.row as f32 * ch - 1.0, cw + 1.5, ch + 1.5);

    // The stage's window onto the board, mapped back to rows and columns.
    let (vw, vh) = stage;
    let x0 = PAD_X + (ROWS_ROOM - rows as f32 / 2.0) * SKEW;
    let vx = motion.cam_x.mul(-cw / CW).add(-x0 * cw / CW);
    let vy = motion.cam_y.mul(-ch / CH).add(-PAD_Y * ch / CH);
    let (w, h) = (vw / CW * cw, vh / CH * ch);
    // On an axis where the whole isle fits, the box spans the minimap.
    let (fit_x, fit_y) = (w >= mw, h >= mh);
    if !(fit_x && fit_y) {
        let x: CanvasArg = if fit_x { 0.0.into() } else { (&vx).into() };
        let y: CanvasArg = if fit_y { 0.0.into() } else { (&vy).into() };
        let (w, h) = (if fit_x { mw } else { w }, if fit_y { mh } else { h });
        painter = painter
            .save()
            .begin_path()
            .move_to(-2.0, -2.0)
            .line_to(mw + 2.0, -2.0)
            .line_to(mw + 2.0, mh + 2.0)
            .line_to(-2.0, mh + 2.0)
            .close_path()
            .clip()
            .translate(x, y)
            .fill_color(hexa(0xfff3b0, 0.12))
            .stroke_color(hexa(0xfff3b0, 0.9))
            .line_width(1.5)
            .begin_path()
            .move_to(0.0, 0.0)
            .line_to(w, 0.0)
            .line_to(w, h)
            .line_to(0.0, h)
            .close_path()
            .fill()
            .stroke()
            .restore();
    }
    painter
}

fn draw_burst(at: (f32, f32), burst: &AnimatedValue) -> Painter {
    let (cx, cy) = at;
    let radius = burst.mul(30.0).add(4.0);
    let fade = burst.interpolate(&[0.0, 1.0], &[1.0, 0.0]);
    let mut painter = Painter::new()
        .global_alpha(&fade)
        .stroke_color(hex(GEM))
        .line_width(2.0)
        .begin_path()
        .arc(cx, cy - 10.0, &radius, 0.0, TAU)
        .stroke();
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU;
        let r = burst.mul(26.0).add(6.0);
        painter = painter
            .save()
            .translate(cx, cy - 10.0)
            .rotate(a)
            .translate(&r, 0.0)
            .fill_color(hex(0xffffff))
            .fill_rect(-1.5, -1.5, 3.0, 3.0)
            .restore();
    }
    painter
}

fn draw_sky(size: (f32, f32), motion: &Motion) -> Painter {
    let (w, h) = size;
    let sky = CanvasGradient::linear(point(px(0.0), px(0.0)), point(px(0.0), px(h)))
        .add_color_stop(percentage(0.0), hex(SKY_TOP))
        .add_color_stop(percentage(0.6), hex(SKY_MID))
        .add_color_stop(percentage(1.0), hex(SKY_LOW));
    let mut painter = Painter::new().fill_style(sky).fill_rect(0.0, 0.0, w, h);
    // The moon.
    let (mx, my) = (w * 0.82, h * 0.16);
    painter = painter
        .fill_style(
            CanvasGradient::radial(point(px(mx), px(my)), px(10.0), point(px(mx), px(my)), px(90.0))
                .add_color_stop(percentage(0.0), hexa(0xfff6d8, 0.35))
                .add_color_stop(percentage(1.0), hexa(0xfff6d8, 0.0)),
        )
        .begin_path()
        .arc(mx, my, 90.0, 0.0, TAU)
        .fill()
        .fill_color(hex(0xfff3cf))
        .begin_path()
        .arc(mx, my, 22.0, 0.0, TAU)
        .fill()
        .fill_color(hexa(SKY_MID, 0.9))
        .begin_path()
        .arc(mx + 9.0, my - 5.0, 19.0, 0.0, TAU)
        .fill();
    // Stars, in two groups that twinkle out of step.
    for (group, alpha) in [(0u32, &motion.twinkle_a), (1u32, &motion.twinkle_b)] {
        painter = painter.save().global_alpha(alpha).fill_color(hex(0xfdf8ff));
        for i in 0..70u32 {
            let seed = i * 2 + group;
            let fx = noise(Pos { row: seed as usize, col: 7 });
            let fy = noise(Pos { row: 3, col: seed as usize + 11 });
            let r = 0.5 + noise(Pos { row: seed as usize, col: seed as usize }) * 1.3;
            painter = painter.begin_path().arc(fx * w, fy * h * 0.75, r, 0.0, TAU).fill();
        }
        painter = painter.restore();
    }
    // Drifting clouds low on the horizon.
    for (k, (y, len, alpha)) in [(0.78, 0.5, 0.10), (0.88, 0.7, 0.08), (0.95, 0.45, 0.12)].iter().enumerate() {
        let speed = [1.0, 0.6, 1.4][k];
        let offset = motion.drift.mul(w * speed);
        painter = painter.save().translate(&offset, 0.0).fill_color(hexa(0xd9c8ff, *alpha));
        for copy in [-1.0f32, 0.0] {
            let x0 = copy * w * speed + k as f32 * 140.0;
            for b in 0..5 {
                let bx = x0 + b as f32 * w * len / 5.0;
                painter = painter.begin_path().ellipse_at(bx, h * y, w * len / 4.0, 18.0 + (b % 2) as f32 * 8.0).fill();
            }
        }
        painter = painter.restore();
    }
    // Fireflies.
    painter = painter.save().translate(&motion.fly_x, &motion.fly_y);
    for i in 0..14u32 {
        let fx = noise(Pos { row: i as usize + 40, col: 3 }) * w;
        let fy = h * 0.35 + noise(Pos { row: 5, col: i as usize + 60 }) * h * 0.6;
        painter = painter
            .fill_style(
                CanvasGradient::radial(point(px(fx), px(fy)), px(0.5), point(px(fx), px(fy)), px(7.0))
                    .add_color_stop(percentage(0.0), hexa(0xfff7a0, 0.9))
                    .add_color_stop(percentage(1.0), hexa(0xfff7a0, 0.0)),
            )
            .begin_path()
            .arc(fx, fy, 7.0, 0.0, TAU)
            .fill();
    }
    painter.restore()
}

trait EllipseAt {
    fn ellipse_at(self, x: f32, y: f32, rx: f32, ry: f32) -> Self;
}

impl EllipseAt for Painter {
    fn ellipse_at(self, x: f32, y: f32, rx: f32, ry: f32) -> Self {
        self.save().translate(x, y).scale(rx.max(0.01), ry.max(0.01)).arc(0.0, 0.0, 1.0, 0.0, TAU).restore()
    }
}

// ----------------------------------------------------------------- the app

const MONO: &str = "monospace";

fn chip(label: impl Into<String>, color: u32) -> impl IntoElement {
    view()
        .px(px(7.0))
        .py(px(2.0))
        .rounded(px(5.0))
        .bg(hexa(color, 0.16))
        .border_1()
        .border_color(hexa(color, 0.45))
        .child(text(label.into()).font_family(MONO).text_size(px(13.0)).text_color(hex(color)).font_weight(FontWeight::BOLD))
}

/// The application's root component.
#[component]
pub fn App() -> impl IntoElement {
    let ui = Ui {
        tick: signal(0_u64),
        scene: signal(0_u64),
        stage: signal((900.0_f32, 520.0_f32)),
        window: signal((1200.0_f32, 800.0_f32)),
        burst_at: signal((0.0_f32, 0.0_f32)),
    };
    let motion = Rc::new(Motion {
        hero_x: animated_value(0.0),
        hero_y: animated_value(0.0),
        hop: animated_value(0.0),
        cam_x: animated_value(0.0),
        cam_y: animated_value(0.0),
        burst: animated_value(1.0),
        shake: animated_value(0.0),
        bob: animated_value(0.0),
        float: animated_value(0.0),
        twinkle_a: animated_value(0.9),
        twinkle_b: animated_value(0.3),
        fly_x: animated_value(0.0),
        fly_y: animated_value(0.0),
        drift: animated_value(0.0),
        card: animated_value(0.0),
    });
    // Ambient loops: all run natively, with no code per frame.
    let breathe = |v: &AnimatedValue, a: f32, b: f32, ms: f32| {
        v.keyframes(
            [Keyframe::new(a), Keyframe::new(b)],
            KeyframeTiming::new(ms).easing(Easing::EaseInOut).infinite().direction(PlaybackDirection::Alternate).frame_rate(30.0),
        );
    };
    breathe(&motion.bob, -2.5, 2.5, 1100.0);
    breathe(&motion.float, -3.0, 3.0, 3200.0);
    breathe(&motion.twinkle_a, 0.95, 0.25, 1900.0);
    breathe(&motion.twinkle_b, 0.25, 0.95, 2600.0);
    breathe(&motion.fly_x, -30.0, 30.0, 7000.0);
    breathe(&motion.fly_y, 12.0, -18.0, 4300.0);
    motion.drift.keyframes(
        [Keyframe::new(0.0), Keyframe::new(1.0)],
        KeyframeTiming::new(90_000.0).easing(Easing::Linear).infinite().frame_rate(20.0),
    );

    audio::load_all();
    audio::start_ambience();
    bind_keys(ui, motion.clone());

    let sky_motion = motion.clone();
    let screen_motion = motion.clone();
    view()
        .flex_1()
        .bg(hex(SKY_TOP))
        .text_color(hex(0xece6ff))
        .on_layout(move |b: &Bounds<Points>| ui.window.set((b.size.width.0, b.size.height.0)))
        .child(
            canvas(move || draw_sky(ui.window.get(), &sky_motion))
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0(),
        )
        .child(
            view().flex_1().p_safe_area().child(dynamic(move || {
                ui.scene.get();
                let playing = with_state(|s| matches!(s.screen, Screen::Play(_)));
                if playing {
                    play_screen(ui, screen_motion.clone()).into_element()
                } else {
                    menu_screen(ui).into_element()
                }
            })),
        )
}

fn menu_screen(ui: Ui) -> impl IntoElement {
    let rows = LEVELS.iter().enumerate().map(move |(i, level)| {
        let new_chapter = i == 0 || LEVELS[i - 1].chapter != level.chapter;
        view()
            .child(new_chapter.then(|| {
                text(level.chapter)
                    .text_size(px(12.0))
                    .font_weight(FontWeight::BOLD)
                    .letter_spacing(px(1.5))
                    .text_color(hexa(0xc9b8ff, 0.7))
                    .mt(px(if i == 0 { 0.0 } else { 10.0 }))
                    .mb(px(4.0))
            }))
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .px(px(12.0))
                    .py(px(7.0))
                    .rounded(px(8.0))
                    .bg(move || {
                        ui.tick.get();
                        let sel = with_state(|s| matches!(s.screen, Screen::Menu { selected } if selected == i));
                        if sel { hexa(0xfff3b0, 0.14) } else { hexa(0xffffff, 0.0) }
                    })
                    .border_1()
                    .border_color(move || {
                        ui.tick.get();
                        let sel = with_state(|s| matches!(s.screen, Screen::Menu { selected } if selected == i));
                        if sel { hexa(0xfff3b0, 0.5) } else { hexa(0xffffff, 0.0) }
                    })
                    .cursor_pointer()
                    .on_press(move || {
                        with_state(|s| s.screen = Screen::Play(Box::new(Session::new(i))));
                        ui.scene.update(|s| *s += 1);
                    })
                    .child(text(format!("{:>2}", i + 1)).font_family(MONO).text_color(hexa(0xffffff, 0.45)).w(px(22.0)))
                    .child(text(level.title).text_size(px(15.0)).flex_1())
                    .child(
                        view().flex_row().gap_1().children(level.unlock.chars().take(6).map(|c| {
                            text(c.to_string()).font_family(MONO).text_size(px(12.0)).text_color(hexa(0xfff3b0, 0.75))
                        })),
                    )
                    .child(
                        text(move || {
                            ui.tick.get();
                            let best = with_state(|s| s.best[i]);
                            match best {
                                Some(n) => {
                                    let stars = game::stars(n, LEVELS[i].par());
                                    format!("{}{}  {n}/{}", "★".repeat(stars as usize), "☆".repeat(3 - stars as usize), LEVELS[i].par())
                                }
                                None => format!("☆☆☆  par {}", LEVELS[i].par()),
                            }
                        })
                        .font_family(MONO)
                        .text_size(px(13.0))
                        .text_color(hex(0xffd76b))
                        .w(px(130.0))
                        .text_right(),
                    ),
            )
    });
    view()
        .flex_1()
        .items_center()
        .child(
            view()
                .w_full()
                .max_w(px(760.0))
                .flex_1()
                .px(px(24.0))
                .pt(px(36.0))
                .gap_4()
                .child(
                    view()
                        .items_center()
                        .gap_2()
                        .child(text("HJKL QUEST").text_size(px(44.0)).font_weight(FontWeight::BLACK).letter_spacing(px(6.0)).text_color(hex(0xfff3b0)))
                        .child(
                            text("A journey through the Buffer Archipelago: learn vim one isle at a time.")
                                .text_size(px(15.0))
                                .text_color(hexa(0xece6ff, 0.75)),
                        ),
                )
                .child(
                    view()
                        .flex_row()
                        .justify_center()
                        .gap_2()
                        .flex_wrap()
                        .child(chip("j k", 0xfff3b0))
                        .child(text("choose").text_color(hexa(0xffffff, 0.6)).mr(px(8.0)))
                        .child(chip("Enter", 0xfff3b0))
                        .child(text("play").text_color(hexa(0xffffff, 0.6)).mr(px(8.0)))
                        .child(chip(":q", 0xfff3b0))
                        .child(text("back here").text_color(hexa(0xffffff, 0.6)).mr(px(8.0)))
                        .child(chip("m", 0xfff3b0))
                        .child(text(move || {
                            ui.tick.get();
                            if audio::muted() { "sound off" } else { "sound on" }
                        })
                        .text_color(hexa(0xffffff, 0.6))),
                )
                .child(
                    scroll().flex_1().child(
                        view()
                            .p(px(14.0))
                            .rounded(px(14.0))
                            .bg(hexa(0x0b0820, 0.55))
                            .border_1()
                            .border_color(hexa(0xc9b8ff, 0.18))
                            .children(rows),
                    ),
                )
                .child(
                    text("Every isle has a par: the keystrokes of a short vim-golf solution. Can you match it?")
                        .text_size(px(13.0))
                        .text_color(hexa(0xece6ff, 0.5))
                        .text_center()
                        .pb(px(16.0)),
                ),
        )
}

fn session_value<R>(f: impl FnOnce(&Session) -> R) -> Option<R> {
    with_state(|s| match &s.screen {
        Screen::Play(session) => Some(f(session)),
        Screen::Menu { .. } => None,
    })
}

fn play_screen(ui: Ui, motion: Rc<Motion>) -> impl IntoElement {
    let Some(index) = session_value(|s| s.index) else { return view().into_element() };
    let level = &LEVELS[index];
    // Place the wisp at once when the level opens.
    place_hero(ui, &motion, false);

    let gems = motion.clone();
    let hero = motion.clone();
    let burst = motion.clone();
    let card = motion.clone();
    let minimap = motion.clone();
    let stage = view()
        .flex_1()
        .overflow_hidden()
        .rounded(px(16.0))
        .bg(hexa(0x05030f, 0.25))
        .border_1()
        .border_color(hexa(0xc9b8ff, 0.14))
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
                .child(
                    view()
                        .bind_translate_y(&motion.float)
                        .bind_translate_x(&motion.shake)
                        .w(move || {
                            ui.tick.get();
                            px(session_value(|s| world_size(s.editor.lines.len() + 2, s.editor.lines.iter().map(|l| l.len()).max().unwrap_or(0).max(8) + 12).0).unwrap_or(400.0))
                        })
                        .h(move || {
                            ui.tick.get();
                            px(session_value(|s| world_size(s.editor.lines.len() + 4, 8).1).unwrap_or(300.0))
                        })
                        .child(
                            canvas(move || {
                                ui.tick.get();
                                session_value(draw_world).unwrap_or_default()
                            })
                            .absolute()
                            .top_0()
                            .left_0()
                            .right_0()
                            .bottom_0(),
                        )
                        .child(
                            canvas(move || {
                                ui.tick.get();
                                session_value(|s| draw_gems(s, &gems.bob)).unwrap_or_default()
                            })
                            .absolute()
                            .top_0()
                            .left_0()
                            .right_0()
                            .bottom_0(),
                        )
                        .child(
                            canvas(move || {
                                ui.tick.get();
                                session_value(|s| draw_hero(s, &hero)).unwrap_or_default()
                            })
                            .absolute()
                            .top_0()
                            .left_0()
                            .right_0()
                            .bottom_0(),
                        )
                        .child(
                            canvas(move || draw_burst(ui.burst_at.get(), &burst.burst))
                                .absolute()
                                .top_0()
                                .left_0()
                                .right_0()
                                .bottom_0(),
                        ),
                ),
        )
        .child(dynamic(move || {
            ui.tick.get();
            let (mw, mh) = session_value(minimap_size)?;
            let map = minimap.clone();
            Some(
                view()
                    .absolute()
                    .top(px(12.0))
                    .right(px(12.0))
                    .p(px(8.0))
                    .rounded(px(10.0))
                    .bg(hexa(0x05030f, 0.62))
                    .border_1()
                    .border_color(hexa(0xc9b8ff, 0.22))
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
        }))
        .child(status_line(ui))
        .child(complete_card(ui, card));

    let top = view()
        .flex_row()
        .items_center()
        .gap_4()
        .child(
            view()
                .flex_1()
                .gap_1()
                .child(text(level.chapter).text_size(px(12.0)).letter_spacing(px(1.5)).text_color(hexa(0xc9b8ff, 0.75)))
                .child(
                    text(format!("{}. {}", index + 1, level.title))
                        .text_size(px(24.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(hex(0xfff3b0)),
                ),
        )
        .child(stat("KEYS", move || {
            ui.tick.get();
            session_value(|s| s.keys.len().to_string()).unwrap_or_default()
        }))
        .child(stat("PAR", move || level.par().to_string()))
        .child(stat("CRYSTALS", move || {
            ui.tick.get();
            session_value(|s| {
                let total = s.gems.len();
                if total == 0 { "–".to_string() } else { format!("{}/{}", total - s.gems_left(), total) }
            })
            .unwrap_or_default()
        }))
        .child(stat("BEST", move || {
            ui.tick.get();
            with_state(|s| s.best[index].map_or("–".to_string(), |n| n.to_string()))
        }));

    let trail = view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(text("typed").text_size(px(12.0)).text_color(hexa(0xffffff, 0.45)))
        .child(
            text(move || {
                ui.tick.get();
                session_value(|s| {
                    let keys = format_keys(&s.keys);
                    let chars: Vec<char> = keys.chars().collect();
                    if chars.len() > 70 { format!("…{}", chars[chars.len() - 70..].iter().collect::<String>()) } else { keys }
                })
                .unwrap_or_default()
            })
            .font_family(MONO)
            .text_size(px(14.0))
            .text_color(hex(0xfff3b0))
            .number_of_lines(1),
        );

    // On narrow screens the lesson panel folds into a strip of its commands.
    let strip = dynamic(move || {
        (ui.window.get().0 < 980.0).then(|| {
            view().flex_row().flex_wrap().gap_2().items_center().children(level.teaches.iter().map(|(keys, what)| {
                view()
                    .flex_row()
                    .gap_1()
                    .items_center()
                    .child(chip(*keys, 0xfff3b0))
                    .child(text(*what).text_size(px(12.0)).text_color(hexa(0xece6ff, 0.7)).mr(px(6.0)))
            }))
        })
    });
    view()
        .flex_1()
        .p(px(18.0))
        .gap_3()
        .child(top)
        .child(strip)
        .child(
            view()
                .flex_1()
                .flex_row()
                .gap_4()
                .child(view().flex_1().gap_2().child(stage).child(trail))
                .child(dynamic(move || (ui.window.get().0 >= 980.0).then(|| side_panel(index)))),
        )
        .into_element()
}

fn stat(label: &'static str, value: impl Fn() -> String + 'static) -> impl IntoElement {
    view()
        .items_end()
        .min_w(px(64.0))
        .child(text(label).text_size(px(11.0)).letter_spacing(px(1.2)).text_color(hexa(0xffffff, 0.45)))
        .child(text(value).font_family(MONO).text_size(px(22.0)).font_weight(FontWeight::BOLD).text_color(hex(0xece6ff)))
}

fn status_line(ui: Ui) -> impl IntoElement {
    view()
        .absolute()
        .left_0()
        .right_0()
        .bottom_0()
        .flex_row()
        .items_center()
        .gap_3()
        .px(px(14.0))
        .py(px(8.0))
        .bg(hexa(0x05030f, 0.72))
        .child(
            text(move || {
                ui.tick.get();
                session_value(|s| match s.editor.mode {
                    Mode::Normal => "NORMAL".to_string(),
                    Mode::Insert => "-- INSERT --".to_string(),
                    Mode::Visual { line: false } => "-- VISUAL --".to_string(),
                    Mode::Visual { line: true } => "-- VISUAL LINE --".to_string(),
                    Mode::Command { .. } => "COMMAND".to_string(),
                })
                .unwrap_or_default()
            })
            .font_family(MONO)
            .font_weight(FontWeight::BOLD)
            .text_size(px(13.0))
            .text_color(move || {
                ui.tick.get();
                hex(session_value(|s| mode_color(&s.editor)).unwrap_or(0xffffff))
            })
            .w(px(150.0)),
        )
        .child(
            text(move || {
                ui.tick.get();
                session_value(|s| match s.editor.mode {
                    Mode::Command { search } => format!("{}{}▏", if search { '/' } else { ':' }, s.editor.cmdline),
                    _ => {
                        let msg = with_state_toast();
                        if !msg.is_empty() { msg } else { s.message.clone() }
                    }
                })
                .unwrap_or_default()
            })
            .font_family(MONO)
            .text_size(px(14.0))
            .flex_1()
            .number_of_lines(1),
        )
        .child(
            text(move || {
                ui.tick.get();
                session_value(|s| s.editor.pending()).unwrap_or_default()
            })
            .font_family(MONO)
            .text_size(px(14.0))
            .text_color(hex(0xffb86b)),
        )
        .child(
            text(move || {
                ui.tick.get();
                session_value(|s| format!("{}:{}", s.editor.cursor.row + 1, s.editor.cursor.col + 1)).unwrap_or_default()
            })
            .font_family(MONO)
            .text_size(px(13.0))
            .text_color(hexa(0xffffff, 0.5)),
        )
}

fn with_state_toast() -> String {
    STATE.with(|s| s.try_borrow().map(|s| s.toast.clone()).unwrap_or_default())
}

fn side_panel(index: usize) -> impl IntoElement {
    let level = &LEVELS[index];
    let teaches = level.teaches.iter().map(|(keys, what)| {
        view()
            .flex_row()
            .items_start()
            .gap_2()
            .child(view().min_w(px(64.0)).items_start().child(chip(*keys, 0xfff3b0)))
            .child(text(*what).text_size(px(13.5)).text_color(hexa(0xece6ff, 0.85)).flex_1())
    });
    view()
        .w(px(320.0))
        .gap_3()
        .p(px(16.0))
        .rounded(px(16.0))
        .bg(hexa(0x0b0820, 0.6))
        .border_1()
        .border_color(hexa(0xc9b8ff, 0.16))
        .child(text(level.story).text_size(px(14.0)).line_height(px(20.0)).text_color(hexa(0xece6ff, 0.8)).italic())
        .child(text("NEW COMMANDS").text_size(px(11.0)).letter_spacing(px(1.5)).text_color(hexa(0xc9b8ff, 0.7)).mt(px(4.0)))
        .child(view().gap_2().children(teaches))
        .child(level.goal_text().map(|goal| {
            view()
                .gap_2()
                .mt(px(4.0))
                .child(text("MAKE THE TEXT READ").text_size(px(11.0)).letter_spacing(px(1.5)).text_color(hexa(0xc9b8ff, 0.7)))
                .child(
                    view()
                        .p(px(10.0))
                        .rounded(px(8.0))
                        .bg(hexa(0x000000, 0.35))
                        .child(text(goal.to_string()).font_family(MONO).text_size(px(13.0)).text_color(hex(0xffd2a8))),
                )
        }))
        .child((level.goal.is_none()).then(|| {
            text("Collect every crystal ◆ to clear the isle.").text_size(px(13.0)).text_color(hex(GEM)).mt(px(4.0))
        }))
        .child(view().flex_1())
        .child(text(format!("Tip: {}", level.tip)).text_size(px(12.5)).line_height(px(18.0)).text_color(hexa(0xece6ff, 0.55)))
        .child(
            text(":e! restart · :q menu · :n skip · :mute")
                .font_family(MONO)
                .text_size(px(11.5))
                .text_color(hexa(0xece6ff, 0.4)),
        )
}

fn complete_card(ui: Ui, motion: Rc<Motion>) -> impl IntoElement {
    let scale = motion.card.interpolate(&[0.0, 1.0], &[0.85, 1.0]);
    dynamic(move || {
        ui.tick.get();
        let done = session_value(|s| s.complete).unwrap_or(false);
        if !done {
            return None;
        }
        let (index, keys, stars, par) = session_value(|s| (s.index, format_keys(&s.keys), s.stars(), s.par())).unwrap();
        let level = &LEVELS[index];
        let count = session_value(|s| s.keys.len()).unwrap();
        let last = index + 1 == LEVELS.len();
        let verdict = if count < par {
            "Under par! You beat the golf solution."
        } else if count == par {
            "Exactly par. A true vim golfer."
        } else {
            "Isle cleared! Now try to reach par."
        };
        Some(
            view()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .items_center()
                .justify_center()
                .bg(hexa(0x05030f, 0.55))
                .child(
                    view()
                        .bind_scale(&scale)
                        .bind_opacity(&motion.card)
                        .w(px(460.0))
                        .p(px(24.0))
                        .gap_3()
                        .rounded(px(18.0))
                        .bg(hexa(0x171032, 0.96))
                        .border_1()
                        .border_color(hexa(0xfff3b0, 0.35))
                        .items_center()
                        .child(
                            text(format!("{}{}", "★".repeat(stars as usize), "☆".repeat(3 - stars as usize)))
                                .text_size(px(40.0))
                                .text_color(hex(0xffd76b)),
                        )
                        .child(text(verdict).text_size(px(18.0)).font_weight(FontWeight::BOLD).text_color(hex(0xfff3b0)))
                        .child(
                            view()
                                .w_full()
                                .gap_1()
                                .mt(px(6.0))
                                .child(text(format!("YOUR KEYS · {count}")).text_size(px(11.0)).letter_spacing(px(1.2)).text_color(hexa(0xffffff, 0.5)))
                                .child(text(keys).font_family(MONO).text_size(px(14.0)).text_color(hex(0xece6ff)))
                                .child(text(format!("GOLF SOLUTION · {par}")).text_size(px(11.0)).letter_spacing(px(1.2)).text_color(hexa(0xffffff, 0.5)).mt(px(8.0)))
                                .child(text(format_keys(&vim::parse_keys(level.solution))).font_family(MONO).text_size(px(14.0)).text_color(hex(GEM))),
                        )
                        .child(
                            view()
                                .flex_row()
                                .gap_2()
                                .mt(px(10.0))
                                .items_center()
                                .child(chip("Enter", 0xfff3b0))
                                .child(text(if last { "back to the map" } else { "next isle" }).text_color(hexa(0xffffff, 0.65)).mr(px(10.0)))
                                .child(chip("r", 0xfff3b0))
                                .child(text("retry").text_color(hexa(0xffffff, 0.65)).mr(px(10.0)))
                                .child(chip("q", 0xfff3b0))
                                .child(text("map").text_color(hexa(0xffffff, 0.65))),
                        ),
                ),
        )
    })
}
