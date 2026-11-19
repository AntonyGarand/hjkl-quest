//! HJKL Quest: learn vim by walking the Buffer Archipelago.
//!
//! The world is the text buffer: each character is a tile, and the cursor
//! is a small wisp that hops between them. `vim` is the editor, `levels`
//! the campaign, `game` a level being played; this file draws it all with
//! Native Shell.

pub mod audio;
pub mod coach;
pub mod daily;
pub mod diff;
pub mod examples;
mod intro;
pub mod game;
pub mod grammar;
pub mod hints;
pub mod lessons;
pub mod levels;
mod replay;
pub mod review;
mod screens;
pub mod story;
pub mod vim;
mod worldmap;

use std::cell::RefCell;
use std::f32::consts::TAU;
use std::rc::Rc;


use nsh_core::element::CanvasArg;
use nsh_core::canvas::{CanvasGradient, CanvasTextAlign, CanvasTextBaseline, LineCap};
use nsh_core::storage::Storage;
use nsh_core::prelude::*;

use audio::Sfx;
use game::{Event, Session};
use levels::{LEVELS, Level};
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
const AMBER: u32 = 0x9a7440;
const TEAL: u32 = 0x2f8a8f;
const WOOD: u32 = 0x86603a;
const EMBER: u32 = 0xc25a3f;
const SELECT: u32 = 0x8a63e0;
const GEM: u32 = 0x7ef3ff;
const BUG: u32 = 0x8be04e;
const GLYPH: u32 = 0xf6f1e3;

fn mode_color(e: &vim::Editor) -> u32 {
    match e.mode {
        Mode::Insert => 0x7dffa8,
        Mode::Visual { .. } | Mode::Block => 0xc9a2ff,
        Mode::Replace => 0xff8f8f,
        Mode::Command { .. } | Mode::Confirm => 0x8ec5ff,
        Mode::Normal if !e.pending().is_empty() => 0xffb86b,
        Mode::Normal => 0xfff3b0,
    }
}

// ------------------------------------------------------------------- state

enum Screen {
    Menu { selected: usize },
    Play(Box<Session>),
    /// The verb × noun grid, from the map.
    Grammar { selected: usize },
    /// A finished run played back beside the golf solution.
    Replay(Box<replay::Replay>),
}

struct State {
    screen: Screen,
    /// The fewest keystrokes each level was solved in.
    best: Vec<Option<usize>>,
    toast: String,
    /// The isle's intro is showing (its lessons, before playing).
    intro: bool,
    /// Isles whose intro has been seen.
    seen: Vec<bool>,
    /// Verb + noun pairs the player has used (`grammar::key`).
    grammar: std::collections::BTreeSet<String>,
    /// The fastest clear of each isle, in tenths of a second.
    times: Vec<Option<u32>>,
    /// When each isle was last cleared, counted in clears: what review
    /// drills pick from.
    practiced: Vec<u32>,
    clears: u32,
    /// The current run: when its first key came, and how long it took (ms).
    started: Option<f64>,
    took: Option<f64>,
    /// How much was left to do when the isle opened: a boss's full health.
    work_start: usize,
    /// Drills to play after this one: (drill, the isle it's from).
    drills: Vec<(&'static Level, usize)>,
    /// The best clear of a daily puzzle: (day, keys).
    daily_best: Option<(u64, usize)>,
}

impl State {
    /// Opens isle `i`, with its intro the first time.
    fn open(&mut self, i: usize) {
        self.screen = Screen::Play(Box::new(Session::new(i)));
        self.intro = !self.seen[i] && self.best[i].is_none();
        self.drills.clear();
        self.new_run();
    }

    /// Opens a daily puzzle or a drill, with the keys of isle `index`.
    fn open_level(&mut self, level: &'static Level, index: usize) {
        self.screen = Screen::Play(Box::new(Session::custom(level, index)));
        self.intro = false;
        self.new_run();
    }

    /// Plays the same isle, puzzle or drill again.
    fn retry(&mut self, level: &'static Level, index: usize, campaign: bool) {
        if campaign {
            let drills = std::mem::take(&mut self.drills);
            self.open(index);
            self.intro = false;
            self.drills = drills;
        } else {
            self.open_level(level, index);
        }
    }

    fn new_run(&mut self) {
        self.started = None;
        self.took = None;
        self.work_start = match &self.screen {
            Screen::Play(s) => work(s),
            _ => 0,
        };
    }

    fn close_intro(&mut self) {
        self.intro = false;
        if let Screen::Play(s) = &self.screen {
            self.seen[s.index] = true;
            save_entries("hjkl-quest.seen-isles", self.seen.iter().enumerate().filter(|(_, s)| **s).map(|(i, _)| (i, "1".to_string())));
        }
    }
}

fn load_seen() -> Vec<bool> {
    let mut seen = vec![false; LEVELS.len()];
    for (i, _) in load_entries("hjkl-quest.seen-isles", "hjkl-quest.seen", ',') {
        seen[i] = true;
    }
    seen
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        screen: start_screen(),
        best: load_progress(),
        toast: String::new(),
        intro: false,
        seen: load_seen(),
        grammar: Storage::get_item("hjkl-quest.grammar").map(|g| g.split('|').filter(|p| !p.is_empty()).map(String::from).collect()).unwrap_or_default(),
        times: load_numbers("hjkl-quest.times").into_iter().map(|t| t.map(|t| t as u32)).collect(),
        practiced: load_numbers("hjkl-quest.practiced").into_iter().map(|t| t.unwrap_or(0) as u32).collect(),
        clears: Storage::get_item("hjkl-quest.clears").and_then(|c| c.parse().ok()).unwrap_or(0),
        started: None,
        took: None,
        work_start: 0,
        drills: Vec::new(),
        daily_best: Storage::get_item("hjkl-quest.daily").and_then(|d| {
            let (day, keys) = d.split_once('=')?;
            Some((day.parse().ok()?, keys.parse().ok()?))
        }),
    });
}

/// A number saved per isle (`title=value|…`).
fn load_numbers(key: &str) -> Vec<Option<usize>> {
    let mut out = vec![None; LEVELS.len()];
    if let Some(saved) = Storage::get_item(key) {
        for (t, v) in saved.split('|').filter_map(|e| e.split_once('=')) {
            if let Some(i) = index_of(t) {
                out[i] = v.parse().ok();
            }
        }
    }
    out
}

fn save_numbers(key: &str, values: impl Iterator<Item = Option<u32>>) {
    save_entries(key, values.enumerate().filter_map(|(i, v)| v.map(|v| (i, v.to_string()))));
}

/// What's left to do on an isle: crystals, bugs, characters to fix.
fn work(s: &Session) -> usize {
    s.gems_left() + s.bugs.len() + session_diff(s).map_or(0, |d| d.wrong_count() + d.missing_count())
}

/// Whether the isle is its chapter's last: a boss guards it.
fn is_boss(s: &Session) -> bool {
    s.in_campaign() && (s.index + 1 == LEVELS.len() || LEVELS[s.index + 1].chapter != s.level().chapter)
}

/// The first isle not yet cleared: the ones after the next are in fog.
fn frontier(best: &[Option<usize>]) -> usize {
    best.iter().position(|b| b.is_none()).unwrap_or(best.len())
}

/// "1:05.3" from milliseconds.
fn clock(ms: f64) -> String {
    let tenths = (ms / 100.0) as u64;
    format!("{}:{:02}.{}", tenths / 600, tenths / 10 % 60, tenths % 10)
}

fn now_ms() -> f64 {
    nsh_core::time::unix_ms()
}

thread_local! {
    /// The look of the chapter on screen, for the tiles and the sky.
    static THEME: std::cell::Cell<story::Theme> = std::cell::Cell::new(story::CHAPTERS[0].theme);
}

fn theme() -> story::Theme {
    THEME.with(|t| t.get())
}

/// Takes the look of the chapter on screen (the map's selected isle, or
/// the isle being played).
fn sync_theme(ui: Ui) {
    let label = with_state(|s| match &s.screen {
        Screen::Play(p) => p.level().chapter,
        Screen::Replay(r) => r.session.level().chapter,
        Screen::Menu { selected } | Screen::Grammar { selected } => LEVELS[*selected].chapter,
    });
    THEME.with(|t| t.set(story::theme(label)));
    let index = story::CHAPTERS.iter().position(|c| story::chapter(label).is_some_and(|k| std::ptr::eq(k, c))).unwrap_or(0);
    if ui.chapter.get_untracked() != index {
        ui.chapter.set(index);
    }
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

/// The isles in the order the first saves used, when progress was kept by
/// position: isles added since then would shift it, so it is now kept by title.
const OLD_ORDER: &[&str] = &[
    "First Steps", "Counting Sheep", "Word Hopping", "Punctuation Pass", "Edge of the World", "Long Jumps",
    "Seek and Find", "Paragraph Isles", "Mirror Matching", "Search Party", "Chomp", "Demolition",
    "Change of Heart", "Insert Coin", "Copycat", "Time Travel", "Inside Out", "Highlights", "Ex Marks the Spot",
    "The Refactor", "Named Vaults", "Waymarks", "Tally Marks", "Wind-up Golem", "Assembly Line", "Terraces",
    "Shouting Match", "Overwrite", "Ice Columns", "Next Match", "Tag, You're It", "Global Decree", "Moving Day",
    "Normal Order", "The Grand Refactor",
];

fn index_of(title: &str) -> Option<usize> {
    LEVELS.iter().position(|l| l.title == title)
}

/// Saved `title=value` pairs, or (from older saves) `index:value` pairs.
fn load_entries(key: &str, old_key: &str, old_sep: char) -> Vec<(usize, String)> {
    if let Some(saved) = Storage::get_item(key) {
        return saved.split('|').filter_map(|e| e.split_once('=')).filter_map(|(t, v)| Some((index_of(t)?, v.to_string()))).collect();
    }
    let Some(saved) = Storage::get_item(old_key) else { return Vec::new() };
    saved
        .split(',')
        .filter_map(|e| {
            let (i, v) = if old_sep == ':' { e.split_once(':')? } else { (e, "1") };
            let title = OLD_ORDER.get(i.parse::<usize>().ok()?)?;
            Some((index_of(title)?, v.to_string()))
        })
        .collect()
}

fn save_entries(key: &str, entries: impl Iterator<Item = (usize, String)>) {
    let text = entries.map(|(i, v)| format!("{}={v}", LEVELS[i].title)).collect::<Vec<_>>().join("|");
    let _ = Storage::set_item(key, text);
}

fn load_progress() -> Vec<Option<usize>> {
    let mut best = vec![None; LEVELS.len()];
    for (i, v) in load_entries("hjkl-quest.progress", "hjkl-quest.best", ':') {
        best[i] = v.parse().ok();
    }
    best
}

fn save_progress(best: &[Option<usize>]) {
    save_entries("hjkl-quest.progress", best.iter().enumerate().filter_map(|(i, b)| b.map(|n| (i, n.to_string()))));
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
    /// Rain, snow and leaves falling: 0 to 1, looping.
    fall: AnimatedValue,
    /// A long jump's dust trail fading: 0 to 1.
    trail: AnimatedValue,
    /// A boss reeling from a hit.
    boss: AnimatedValue,
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
    /// Steps the intro's examples along.
    beat: Signal<u64>,
    /// The burst's colour: a crystal's, or a squashed bug's.
    burst_color: Signal<u32>,
    /// A long jump: where it started and ended, for the dust trail.
    trail: Signal<((f32, f32), (f32, f32))>,
    /// The chapter on screen, for the sky.
    chapter: Signal<usize>,
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
    keys.push(("ctrl+a".into(), Key::CtrlA));
    keys.push(("ctrl+x".into(), Key::CtrlX));
    keys.push(("ctrl+v".into(), Key::CtrlV));
    keys.push(("tab".into(), Key::Tab));
    keys.push(("ctrl+i".into(), Key::Tab));
    keys.push(("ctrl+d".into(), Key::CtrlD));
    keys.push(("ctrl+o".into(), Key::CtrlO));
    keys.push(("ctrl+g".into(), Key::CtrlG));
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

/// The first isle of the chapter isle `i` is in.
fn chapter_start(i: usize) -> usize {
    (0..=i).rev().find(|&k| k == 0 || LEVELS[k - 1].chapter != LEVELS[i].chapter).unwrap_or(0)
}

/// Opens today's tide.
fn open_daily(state: &mut State) {
    let level: &'static Level = Box::leak(Box::new(daily::daily(daily::today())));
    state.open_level(level, LEVELS.len() - 1);
}

/// Opens a round of review drills from the isles cleared longest ago.
fn open_review(state: &mut State) -> bool {
    let mut cleared: Vec<usize> = (0..LEVELS.len()).filter(|&i| state.best[i].is_some()).collect();
    cleared.sort_by_key(|&i| state.practiced[i]);
    let titles: Vec<&str> = cleared.iter().map(|&i| LEVELS[i].title).collect();
    let mut drills: Vec<(&'static Level, usize)> = review::pick(&titles, 5, state.clears as u64)
        .into_iter()
        .map(|l| {
            let from = l.title.strip_prefix("Review · ").and_then(index_of).unwrap_or(0);
            (&*Box::leak(Box::new(l)), from)
        })
        .collect();
    if drills.is_empty() {
        return false;
    }
    let (first, from) = drills.remove(0);
    state.open_level(first, from);
    state.drills = drills;
    true
}

fn handle_key(key: Key, ui: Ui, motion: &Motion) {
    let mut scene_changed = false;
    let mut boss_hit = false;
    let mut new_word = false;
    let cursor_before = session_value(|s| s.editor.cursor);
    let event = with_state(|state| {
        state.toast.clear();
        match &mut state.screen {
            Screen::Menu { selected } => {
                let n = LEVELS.len();
                match key {
                    Key::Char('l' | 'w') => *selected = (*selected + 1).min(n - 1),
                    Key::Char('h' | 'b') => *selected = selected.saturating_sub(1),
                    Key::Char('j') => {
                        let here = LEVELS[*selected].chapter;
                        if let Some(k) = (*selected..n).find(|&k| LEVELS[k].chapter != here) {
                            *selected = k;
                        }
                    }
                    Key::Char('k') => {
                        let start = chapter_start(*selected);
                        *selected = if start < *selected { start } else { chapter_start(start.saturating_sub(1)) };
                    }
                    Key::Char('G') => *selected = n - 1,
                    Key::Char('g') => *selected = 0,
                    Key::Char('m') => {
                        audio::set_muted(!audio::muted());
                    }
                    Key::Char('v') => {
                        let selected = *selected;
                        state.screen = Screen::Grammar { selected };
                        scene_changed = true;
                        return Some(Event::None);
                    }
                    Key::Char('d') => {
                        open_daily(state);
                        scene_changed = true;
                        return Some(Event::Moved);
                    }
                    Key::Char('r') => {
                        if open_review(state) {
                            scene_changed = true;
                        } else {
                            state.toast = "Review drills come from isles you've cleared: clear a few first.".into();
                        }
                        return Some(Event::Moved);
                    }
                    Key::Enter | Key::Char(' ' | 'i') => {
                        let index = *selected;
                        state.open(index);
                        scene_changed = true;
                        return Some(Event::Moved);
                    }
                    _ => return None,
                }
                Some(Event::Moved)
            }
            Screen::Grammar { selected } => match key {
                Key::Esc | Key::Enter | Key::Char('q' | 'v' | ' ') => {
                    let selected = *selected;
                    state.screen = Screen::Menu { selected };
                    scene_changed = true;
                    Some(Event::None)
                }
                _ => None,
            },
            Screen::Replay(r) => {
                if r.key(key) {
                    let Screen::Replay(r) = std::mem::replace(&mut state.screen, Screen::Menu { selected: 0 }) else { unreachable!() };
                    state.screen = Screen::Play(Box::new(r.session));
                    scene_changed = true;
                }
                Some(Event::None)
            }
            Screen::Play(_) if state.intro => {
                // The intro takes the keyboard until it's closed.
                match key {
                    Key::Enter | Key::Esc | Key::Char(' ' | 'i' | 'l' | 'q') => {
                        state.close_intro();
                        Some(Event::None)
                    }
                    _ => None,
                }
            }
            Screen::Play(session) => {
                let (index, level, campaign) = (session.index, session.level(), session.in_campaign());
                if session.complete {
                    // A missed lesson: Enter tries the isle again.
                    let failed = !session.missed.is_empty();
                    match key {
                        Key::Char('p') => {
                            let Screen::Play(done) = std::mem::replace(&mut state.screen, Screen::Menu { selected: index }) else { unreachable!() };
                            state.screen = Screen::Replay(Box::new(replay::Replay::new(*done)));
                        }
                        Key::Char('r') => state.retry(level, index, campaign),
                        Key::Enter if failed => state.retry(level, index, campaign),
                        Key::Enter | Key::Char('n') if !state.drills.is_empty() => {
                            let (next, from) = state.drills.remove(0);
                            let rest = std::mem::take(&mut state.drills);
                            state.open_level(next, from);
                            state.drills = rest;
                        }
                        Key::Enter | Key::Char('n') if campaign && index + 1 < LEVELS.len() => state.open(index + 1),
                        Key::Enter | Key::Char('n' | 'q') | Key::Esc => {
                            state.drills.clear();
                            state.screen = Screen::Menu { selected: index };
                        }
                        _ => return None,
                    }
                    scene_changed = true;
                    return Some(Event::Moved);
                }
                let (keys_before, used_before, work_before) = (session.keys.len(), session.used.len(), work(session));
                let event = session.press(key);
                if keys_before == 0 && !session.keys.is_empty() && state.started.is_none() {
                    state.started = Some(now_ms());
                }
                if event == Event::Complete && state.took.is_none() {
                    state.took = Some(state.started.map_or(0.0, |t| now_ms() - t));
                }
                // New words for the grammar.
                for token in session.used.iter().skip(used_before) {
                    if let Some((verb, noun)) = grammar::pair(token)
                        && state.grammar.insert(grammar::key(verb, noun))
                    {
                        let total = grammar::VERBS.len() * grammar::NOUNS.len();
                        state.toast = format!("New word: {verb} + {noun}  ·  {} of {total} in your grammar (v on the map)", state.grammar.len());
                        let _ = Storage::set_item("hjkl-quest.grammar", state.grammar.iter().cloned().collect::<Vec<_>>().join("|"));
                        new_word = true;
                    }
                }
                boss_hit = is_boss(session) && !session.complete && work(session) < work_before;
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
                                state.retry(level, index, campaign);
                                scene_changed = true;
                            }
                            "n" | "next" if campaign => {
                                let index = (session.index + 1).min(LEVELS.len() - 1);
                                state.open(index);
                                scene_changed = true;
                            }
                            "n" | "next" => {
                                state.screen = Screen::Menu { selected: index };
                                scene_changed = true;
                            }
                            "grammar" => {
                                state.screen = Screen::Grammar { selected: index };
                                scene_changed = true;
                            }
                            "mute" | "sound" => {
                                audio::set_muted(!audio::muted());
                                state.toast = if audio::muted() { "Sound off".into() } else { "Sound on".into() };
                            }
                            "help" | "h" | "intro" | "learn" => state.intro = true,
                            "w" => state.toast = "Saved! (progress saves by itself)".into(),
                            other => state.toast = format!("E492: Not an editor command: {other}"),
                        }
                    }
                    Event::Complete if !session.cleared() => {}
                    Event::Complete if !campaign => {
                        let n = session.keys.len();
                        if level.chapter == "Daily Tide" {
                            let today = daily::today();
                            if state.daily_best.is_none_or(|(d, b)| d != today || n < b) {
                                state.daily_best = Some((today, n));
                                let _ = Storage::set_item("hjkl-quest.daily", format!("{today}={n}"));
                            }
                        } else {
                            // A drill keeps its isle fresh.
                            state.clears += 1;
                            state.practiced[index] = state.clears;
                            save_numbers("hjkl-quest.practiced", state.practiced.iter().map(|&p| Some(p)));
                            let _ = Storage::set_item("hjkl-quest.clears", state.clears.to_string());
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
                        let tenths = (state.took.unwrap_or(0.0) / 100.0) as u32;
                        if state.times[i].is_none_or(|t| tenths < t) {
                            state.times[i] = Some(tenths);
                            save_numbers("hjkl-quest.times", state.times.iter().copied());
                        }
                        state.clears += 1;
                        state.practiced[i] = state.clears;
                        save_numbers("hjkl-quest.practiced", state.practiced.iter().map(|&p| Some(p)));
                        let _ = Storage::set_item("hjkl-quest.clears", state.clears.to_string());
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
            ui.burst_color.set(GEM);
            motion.burst.set(0.0);
            motion.burst.timing(1.0, 650.0, Easing::Curve(Curve::Cubic, EaseMode::Out));
        }
        Event::Squashed(p) => {
            audio::play(Sfx::Squash, 0.6);
            let (cx, cy) = tile_center(*p);
            ui.burst_at.set((cx, cy + 6.0));
            ui.burst_color.set(BUG);
            motion.burst.set(0.0);
            motion.burst.timing(1.0, 550.0, Easing::Curve(Curve::Cubic, EaseMode::Out));
        }
        Event::Bell => {
            audio::play(Sfx::Bell, 0.35);
            motion.shake.keyframes(
                [Keyframe::new(0.0), Keyframe::new(-4.0), Keyframe::new(4.0), Keyframe::new(-2.0), Keyframe::new(0.0)],
                KeyframeTiming::new(220.0),
            );
        }
        Event::Trapped(p) => {
            audio::play(Sfx::Locked, 0.6);
            audio::play(Sfx::Bell, 0.4);
            let (cx, cy) = tile_center(*p);
            ui.burst_at.set((cx, cy));
            ui.burst_color.set(0xff6b6b);
            motion.burst.set(0.0);
            motion.burst.timing(1.0, 500.0, Easing::Curve(Curve::Cubic, EaseMode::Out));
            motion.shake.keyframes(
                [Keyframe::new(0.0), Keyframe::new(-9.0), Keyframe::new(9.0), Keyframe::new(-5.0), Keyframe::new(3.0), Keyframe::new(0.0)],
                KeyframeTiming::new(340.0),
            );
        }
        Event::Locked(_) => {
            audio::play(Sfx::Locked, 0.45);
            motion.shake.keyframes(
                [Keyframe::new(0.0), Keyframe::new(-6.0), Keyframe::new(6.0), Keyframe::new(-3.0), Keyframe::new(0.0)],
                KeyframeTiming::new(260.0),
            );
        }
        Event::Complete if session_value(|s| !s.cleared()).unwrap_or(false) => {
            audio::play(Sfx::Locked, 0.5);
            motion.card.set(0.0);
            motion.card.spring(1.0, Spring { stiffness: 170.0, damping: 14.0, ..Spring::default() });
        }
        Event::Complete => {
            audio::play(Sfx::Gem, 0.4);
            audio::play(Sfx::Complete, 0.6);
            motion.card.set(0.0);
            motion.card.spring(1.0, Spring { stiffness: 170.0, damping: 14.0, ..Spring::default() });
        }
        _ => {}
    }
    if boss_hit {
        audio::play(Sfx::Hit, 0.55);
        motion.boss.keyframes(
            [Keyframe::new(0.0), Keyframe::new(1.0), Keyframe::new(-0.6), Keyframe::new(0.3), Keyframe::new(0.0)],
            KeyframeTiming::new(420.0),
        );
    }
    if new_word {
        audio::play(Sfx::Word, 0.4);
    }
    // A long jump leaves a trail of dust.
    if !scene_changed
        && let (Some(from), Some(to)) = (cursor_before, session_value(|s| s.editor.cursor))
        && from.row.abs_diff(to.row) + from.col.abs_diff(to.col) >= 4
        && matches!(event, Event::Moved | Event::Gem(_) | Event::Squashed(_))
    {
        ui.trail.set((tile_center(from), tile_center(to)));
        motion.trail.set(0.0);
        motion.trail.timing(1.0, 650.0, Easing::Curve(Curve::Cubic, EaseMode::Out));
    }
    sync_theme(ui);
    if with_state(|s| matches!(s.screen, Screen::Menu { .. })) {
        worldmap::follow(ui, motion, !scene_changed);
    }
    place_hero(ui, motion, !scene_changed);
    // A trap: the wisp hops up onto the block, and is flung back.
    if let Event::Trapped(p) = &event
        && let Some(at) = session_value(|s| s.editor.cursor)
    {
        let ((fx, fy), (tx, ty)) = (tile_origin(at), tile_origin(*p));
        let timing = || KeyframeTiming::new(460.0).easing(Easing::Linear);
        motion.hero_x.keyframes([Keyframe::new(fx), Keyframe::at(0.4, tx).ease(Easing::EaseOut), Keyframe::at(0.55, tx), Keyframe::new(fx).ease(Easing::EaseIn)], timing());
        motion.hero_y.keyframes([Keyframe::new(fy), Keyframe::at(0.4, ty).ease(Easing::EaseOut), Keyframe::at(0.55, ty), Keyframe::new(fy).ease(Easing::EaseIn)], timing());
        motion.hop.keyframes(
            [
                Keyframe::new(0.0),
                Keyframe::at(0.25, -LIFT - 10.0).ease(Easing::EaseOut),
                Keyframe::at(0.4, -LIFT).ease(Easing::EaseIn),
                Keyframe::at(0.7, -LIFT - 18.0).ease(Easing::EaseOut),
                Keyframe::new(0.0).ease(Easing::EaseIn),
            ],
            timing(),
        );
    }
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
        _ => None,
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

/// The goal of an editing level, as lines of characters.
fn goal_lines(session: &Session) -> Option<Vec<Vec<char>>> {
    session.level().goal_text().map(|g| g.split('\n').map(|l| l.chars().collect()).collect())
}

fn session_diff(session: &Session) -> Option<diff::Diff> {
    goal_lines(session).map(|g| diff::diff(&session.editor.lines, &g))
}

/// Whether the cursor's selection covers `p`.
fn is_selected(e: &vim::Editor, p: Pos) -> bool {
    match e.mode {
        Mode::Visual { line } => {
            let (s, t) = if e.anchor <= e.cursor { (e.anchor, e.cursor) } else { (e.cursor, e.anchor) };
            if line { p.row >= s.row && p.row <= t.row } else { p >= s && p <= t }
        }
        Mode::Block => {
            let (r0, r1, c0, c1, to_end) = e.block_rect();
            p.row >= r0 && p.row <= r1 && p.col >= c0 && (to_end || p.col <= c1)
        }
        _ => false,
    }
}

/// Where missing text shows, in amber.
const MISSING: u32 = 0xffb44c;

fn draw_world(session: &Session) -> Painter {
    let e = &session.editor;
    let goal = goal_lines(session);
    let diff = goal.as_ref().map(|g| diff::diff(&e.lines, g));
    let mut painter = Painter::new();

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
    painter = draw_rulers(painter, session);

    for (row, line) in e.lines.iter().enumerate() {
        if line.is_empty() {
            let look = if is_selected(e, Pos { row, col: 0 }) { TileLook::Selected } else { TileLook::Plank };
            painter = draw_tile(painter, Pos { row, col: 0 }, None, look);
        }
        for (col, &ch) in line.iter().enumerate() {
            let p = Pos { row, col };
            let wrong = diff.as_ref().is_some_and(|d| d.wrong[row][col]);
            let look = if is_selected(e, p) {
                TileLook::Selected
            } else if session.traps.contains(&p) {
                TileLook::Trap
            } else if wrong {
                TileLook::Wrong
            } else if goal.is_some() && ch == '.' {
                // In text, a period is a character to see, not a flower.
                TileLook::Amber
            } else {
                TileLook::of(ch)
            };
            painter = draw_tile(painter, p, Some(ch), look);
        }
    }

    for &t in &session.traps {
        painter = draw_trap(painter, t, e.char_at(t));
    }
    // Search matches: 'hlsearch', 'incsearch', and the :s///c question.
    for (at, len) in e.search_highlights() {
        for col in at.col..at.col + len {
            painter = draw_match(painter, Pos { row: at.row, col });
        }
    }

    let (Some(diff), Some(goal)) = (diff, goal) else { return painter };
    // Lines missing at the end show as ghost rows below the isle; lines
    // missing in between as a seam where they go.
    let mut extra = 0;
    let mut seams: Vec<(usize, Vec<String>)> = Vec::new();
    for &(before, g) in &diff.missing_lines {
        if before >= e.lines.len() {
            let row = e.lines.len() + extra;
            extra += 1;
            if goal[g].is_empty() {
                painter = draw_ghost(painter, Pos { row, col: 0 }, ' ');
            }
            for (col, &want) in goal[g].iter().enumerate() {
                painter = draw_ghost(painter, Pos { row, col }, want);
            }
        } else {
            let text: String = goal[g].iter().collect();
            match seams.last_mut() {
                Some((b, texts)) if *b == before => texts.push(text),
                _ => seams.push((before, vec![text])),
            }
        }
    }
    for (before, texts) in seams {
        let shown: Vec<String> = texts.iter().map(|t| if t.is_empty() { "(empty line)".into() } else { t.clone() }).collect();
        painter = draw_seam(painter, before, &e.lines, &format!("+ {}", shown.join("  ⏎ ")));
    }
    // Full text on the cursor's row; elsewhere how much, to keep busy
    // isles readable.
    for (at, text) in &diff.inserts {
        painter = draw_insert(painter, *at, text, at.row == e.cursor.row);
    }
    painter
}

/// A trap: a dark plate with three spikes, red at the tips.
/// How far a trap rises above the tiles.
const LIFT: f32 = 16.0;
const TRAP: u32 = 0x7a3442;
fn trap_top() -> Hsla {
    shade(TRAP, 1.15)
}

/// A trap: a raised block of dark stone, spiked along its back edge, that
/// the cursor can never stand on. Its letter stays on top, for f and t.
fn draw_trap(painter: Painter, p: Pos, c: Option<char>) -> Painter {
    let (x, y) = tile_origin(p);
    let yt = y - LIFT;
    let mut painter = painter
        // Left side and front, from the raised top down to the tile.
        .fill_color(shade(TRAP, 0.45))
        .begin_path()
        .move_to(x, yt + CH)
        .line_to(x + SKEW, yt)
        .line_to(x + SKEW, y)
        .line_to(x, y + CH)
        .close_path()
        .fill()
        .fill_color(shade(TRAP, 0.62))
        .fill_rect(x, yt + CH, CW, LIFT)
        .fill_color(trap_top())
        .begin_path()
        .move_to(x + SKEW, yt)
        .line_to(x + SKEW + CW, yt)
        .line_to(x + CW, yt + CH)
        .line_to(x, yt + CH)
        .close_path()
        .fill()
        .stroke_color(hexa(0xff6b6b, 0.55))
        .line_width(1.2)
        .stroke();
    // Spikes along the back edge.
    for k in 0..4 {
        let bx = x + SKEW + 6.0 + k as f32 * 9.5;
        let by = yt + 3.0;
        painter = painter
            .fill_color(hex(0xc3cbdd))
            .begin_path()
            .move_to(bx - 3.5, by)
            .line_to(bx, by - 11.0)
            .line_to(bx + 3.5, by)
            .close_path()
            .fill()
            .fill_color(hex(0x737d97))
            .begin_path()
            .move_to(bx, by)
            .line_to(bx, by - 11.0)
            .line_to(bx + 3.5, by)
            .close_path()
            .fill()
            .fill_color(hex(0xff4d4d))
            .begin_path()
            .arc(bx, by - 10.5, 1.3, 0.0, TAU)
            .fill();
    }
    if let Some(ch) = c.filter(|c| !matches!(c, ' ' | '.')) {
        let (cx, cy) = (x + CW / 2.0 + SKEW / 2.0, yt + CH / 2.0 + 2.0);
        painter = painter
            .font("700 19px monospace")
            .text_align(CanvasTextAlign::Center)
            .text_baseline(CanvasTextBaseline::Middle)
            .fill_color(hexa(0x000000, 0.4))
            .fill_text(&ch.to_string(), cx + 1.0, cy + 1.5)
            .fill_color(hex(0xffe4e0))
            .fill_text(&ch.to_string(), cx, cy + 0.5);
    }
    painter
}

/// A search match on a tile: a warm glow on its top face.
fn draw_match(painter: Painter, p: Pos) -> Painter {
    let (x, y) = tile_origin(p);
    painter
        .fill_color(hexa(0xffd76b, 0.38))
        .stroke_color(hexa(0xffd76b, 0.95))
        .line_width(1.5)
        .begin_path()
        .move_to(x + SKEW + 1.0, y + 1.0)
        .line_to(x + SKEW + CW - 1.0, y + 1.0)
        .line_to(x + CW - 1.0, y + CH - 1.0)
        .line_to(x + 1.0, y + CH - 1.0)
        .close_path()
        .fill()
        .stroke()
}

/// A small label in a rounded pill, its left edge at `x`, centred on `y`.
fn draw_pill(painter: Painter, x: f32, y: f32, label: &str, color: u32) -> Painter {
    let w = label.chars().count() as f32 * 7.4 + 12.0;
    painter
        .fill_color(hexa(0x1a0f05, 0.88))
        .stroke_color(hexa(color, 0.95))
        .line_width(1.2)
        .begin_path()
        .round_rect(Bounds { origin: point(px(x), px(y - 9.0)), size: size(px(w), px(18.0)) }, px(9.0))
        .fill()
        .stroke()
        .font("700 12px monospace")
        .text_align(CanvasTextAlign::Left)
        .text_baseline(CanvasTextBaseline::Middle)
        .fill_color(hex(color))
        .fill_text(label, x + 6.0, y + 0.5)
}

/// Missing text inside a line: a glowing slot where it goes, and what.
fn draw_insert(painter: Painter, at: Pos, text: &str, full: bool) -> Painter {
    let (x, y) = tile_origin(at);
    let label = if full { format!("+{}", text.replace(' ', "␣")) } else { format!("+{}", text.chars().count()) };
    let painter = painter
        .stroke_color(hexa(MISSING, 0.35))
        .line_width(7.0)
        .line_cap(LineCap::Round)
        .begin_path()
        .move_to(x + SKEW, y + 2.0)
        .line_to(x + 1.0, y + CH - 2.0)
        .stroke()
        .stroke_color(hex(MISSING))
        .line_width(2.5)
        .begin_path()
        .move_to(x + SKEW, y + 2.0)
        .line_to(x + 1.0, y + CH - 2.0)
        .stroke();
    let w = label.chars().count() as f32 * 7.4 + 12.0;
    draw_pill(painter, x + SKEW - w / 2.0, y - 11.0, &label, MISSING)
}

/// Whole lines missing before buffer row `before`: a dashed seam between
/// the rows, and their text.
fn draw_seam(painter: Painter, before: usize, lines: &[Vec<char>], label: &str) -> Painter {
    let width = lines[before.saturating_sub(1)..=before.min(lines.len() - 1)].iter().map(|l| l.len()).max().unwrap_or(1).max(1);
    let (x, y) = tile_origin(Pos { row: before, col: 0 });
    let painter = painter
        .stroke_color(hex(MISSING))
        .line_width(2.5)
        .line_dash(&[px(6.0), px(4.0)])
        .begin_path()
        .move_to(x + SKEW, y)
        .line_to(x + SKEW + width as f32 * CW, y)
        .stroke()
        .line_dash(&[]);
    draw_pill(painter, x + SKEW + width as f32 * CW + 8.0, y, label, MISSING)
}

/// Line numbers down the left (absolute, and how far each line is from
/// the cursor, for counts like 5j) and column numbers along the top.
fn draw_rulers(painter: Painter, session: &Session) -> Painter {
    let e = &session.editor;
    let cur = e.cursor;
    let accent = mode_color(e);
    let mut painter = painter.text_baseline(CanvasTextBaseline::Middle);
    for row in 0..e.lines.len() {
        let (x, y) = tile_origin(Pos { row, col: 0 });
        let (gx, gy) = (x + SKEW / 2.0 - 8.0, y + CH / 2.0 + 0.5);
        let here = row == cur.row;
        painter = painter
            .font(if here { "700 13px monospace" } else { "500 12px monospace" })
            .text_align(CanvasTextAlign::Right)
            .fill_color(if here { hex(accent) } else { hexa(0xece6ff, 0.38) })
            .fill_text(&(row + 1).to_string(), gx - 22.0, gy);
        if !here {
            let rel = row.abs_diff(cur.row);
            painter = painter
                .font("700 11px monospace")
                .fill_color(hexa(0xfff3b0, 0.7))
                .fill_text(&format!("{}{}", if row < cur.row { '↑' } else { '↓' }, rel), gx, gy);
        }
    }
    // Columns along the top edge of the first row.
    let cols = e.lines.iter().map(|l| l.len()).max().unwrap_or(0);
    let (_, top) = tile_origin(Pos { row: 0, col: 0 });
    let y = top - 30.0;
    for col in 0..cols {
        let (x, _) = tile_origin(Pos { row: 0, col });
        let cx = x + SKEW + CW / 2.0;
        let n = col + 1;
        let here = col == cur.col;
        let major = n == 1 || n % 5 == 0;
        painter = painter
            .fill_color(hexa(0xece6ff, if n % 10 == 0 { 0.5 } else { 0.22 }))
            .begin_path()
            .arc(cx, y + 11.0, if major { 1.8 } else { 1.1 }, 0.0, TAU)
            .fill();
        if major || here {
            painter = painter
                .font(if here { "700 12px monospace" } else { "500 11px monospace" })
                .text_align(CanvasTextAlign::Center)
                .fill_color(if here { hex(accent) } else { hexa(0xece6ff, if n % 10 == 0 { 0.6 } else { 0.38 }) })
                .fill_text(&n.to_string(), cx, y);
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
    Trap,
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
            TileLook::Grass | TileLook::Flower => theme().grass,
            TileLook::Stone => theme().stone,
            TileLook::Amber => AMBER,
            TileLook::Teal => TEAL,
            TileLook::Plank => WOOD,
            TileLook::Wrong => EMBER,
            TileLook::Selected => SELECT,
            TileLook::Trap => TRAP,
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

/// The minimap's cell width and height for a buffer of `rows` × `cols`:
/// big enough for letters on small isles, a few pixels on large ones.
fn minimap_cell(rows: usize, cols: usize) -> (f32, f32) {
    let rows = rows.max(1) as f32;
    let w = (280.0 / cols.max(1) as f32).min(150.0 / rows).clamp(2.0, 11.0);
    let h = (w * 1.4).max(11.0).min((150.0 / rows).max(3.0));
    (w, h)
}

/// Whether the minimap's cells are big enough to print their letters.
fn minimap_letters(cw: f32, ch: f32) -> bool {
    cw >= 6.0 && ch >= 8.0
}

fn minimap_size(session: &Session) -> (f32, f32) {
    let e = &session.editor;
    let goal_rows = session.level().goal_text().map_or(0, |g| g.lines().count());
    let rows = e.lines.len().max(goal_rows);
    let cols = e.lines.iter().map(|l| l.len()).max().unwrap_or(1).max(1);
    let (cw, ch) = minimap_cell(rows, cols);
    (cols as f32 * cw + MINI_GUTTER.0, rows as f32 * ch + MINI_GUTTER.1)
}

/// Room around the minimap for its line and column numbers.
const MINI_GUTTER: (f32, f32) = (20.0, 12.0);

/// The whole isle in a corner: its tiles, the crystals left, the cursor
/// and the part the stage shows, which follows the camera natively.
fn draw_minimap(session: &Session, motion: &Motion, stage: (f32, f32)) -> Painter {
    let e = &session.editor;
    let goal_rows = session.level().goal_text().map_or(0, |g| g.lines().count());
    let rows = e.lines.len().max(goal_rows);
    let cols = e.lines.iter().map(|l| l.len()).max().unwrap_or(1).max(1);
    let (cw, ch) = minimap_cell(rows, cols);
    let (mw, mh) = (cols as f32 * cw, rows as f32 * ch);
    let goal = goal_lines(session);
    let diff = session_diff(session);
    let letters = minimap_letters(cw, ch);
    let font = format!("700 {}px monospace", (ch * 0.78).min(cw * 1.45).floor());
    let mut painter = draw_minimap_rulers(Painter::new(), e.cursor, rows.min(e.lines.len()), cols, cw, ch)
        .save()
        .translate(MINI_GUTTER.0, MINI_GUTTER.1)
        .font(&font)
        .text_align(CanvasTextAlign::Center)
        .text_baseline(CanvasTextBaseline::Middle);
    for (row, line) in e.lines.iter().enumerate() {
        let y = row as f32 * ch;
        if line.is_empty() {
            painter = painter.fill_color(hexa(WOOD, 0.9)).fill_rect(0.0, y + 1.0, cw - 0.5, ch - 2.0);
        }
        for (col, &c) in line.iter().enumerate() {
            let p = Pos { row, col };
            let is_wrong = diff.as_ref().is_some_and(|d| d.wrong[row][col]);
            let color = if is_selected(e, p) {
                SELECT
            } else if is_wrong {
                EMBER
            } else if goal.is_some() && c == '.' {
                AMBER
            } else {
                TileLook::of(c).color()
            };
            painter = painter.fill_color(hex(color)).fill_rect(col as f32 * cw, y, cw - 0.5, ch - 0.5);
            if letters && !c.is_whitespace() && !(goal.is_none() && c == '.') {
                painter = painter.fill_color(hexa(GLYPH, 0.95)).fill_text(&c.to_string(), col as f32 * cw + cw / 2.0, y + ch / 2.0 + 0.5);
            }
        }
    }
    // What's missing: ghost rows at the end, slots where text goes.
    if let (Some(diff), Some(goal)) = (&diff, &goal) {
        let mut extra = 0;
        for &(before, g) in &diff.missing_lines {
            if before < e.lines.len() {
                painter = painter.fill_color(hex(MISSING)).fill_rect(0.0, before as f32 * ch - 1.0, mw, 2.0);
                continue;
            }
            let y = (e.lines.len() + extra) as f32 * ch;
            extra += 1;
            for (col, &c) in goal[g].iter().enumerate().take(cols) {
                let x = col as f32 * cw;
                painter = painter.stroke_color(hexa(MISSING, 0.6)).line_width(1.0).begin_path().move_to(x + 0.5, y + 0.5).line_to(x + cw - 1.0, y + 0.5).line_to(x + cw - 1.0, y + ch - 1.0).line_to(x + 0.5, y + ch - 1.0).close_path().stroke();
                if letters && !c.is_whitespace() {
                    painter = painter.fill_color(hexa(MISSING, 0.8)).fill_text(&c.to_string(), x + cw / 2.0, y + ch / 2.0 + 0.5);
                }
            }
        }
        for (at, _) in &diff.inserts {
            painter = painter.fill_color(hex(MISSING)).fill_rect(at.col as f32 * cw - 1.0, at.row as f32 * ch - 1.0, 2.5, ch + 1.0);
        }
    }
    for &t in &session.traps {
        let (x, y) = (t.col as f32 * cw, t.row as f32 * ch);
        painter = painter
            .fill_color(hex(0xff4d4d))
            .begin_path()
            .move_to(x + cw / 2.0, y + 1.0)
            .line_to(x + cw - 0.5, y + ch - 1.0)
            .line_to(x + 0.5, y + ch - 1.0)
            .close_path()
            .fill();
    }
    for &(p, got) in &session.gems {
        if !got {
            let (x, y) = (p.col as f32 * cw + cw / 2.0, p.row as f32 * ch + ch / 2.0);
            painter = painter.fill_color(hex(GEM)).begin_path().arc(x, y, (cw * 0.55).max(2.0), 0.0, TAU).fill();
        }
    }
    for bug in &session.bugs {
        let (x, y) = (bug.at.col as f32 * cw + cw / 2.0, bug.at.row as f32 * ch + ch / 2.0);
        painter = painter.fill_color(hex(BUG)).begin_path().arc(x, y, (cw * 0.6).max(2.5), 0.0, TAU).fill();
    }
    let cur = e.cursor;
    painter = painter
        .fill_color(hex(mode_color(e)))
        .stroke_color(hex(0xffffff))
        .line_width(1.0)
        .fill_rect(cur.col as f32 * cw - 1.0, cur.row as f32 * ch - 1.0, cw + 1.5, ch + 1.5);
    if letters && let Some(c) = e.char_at(cur).filter(|c| !c.is_whitespace()) {
        painter = painter.fill_color(hex(0x1a1430)).fill_text(&c.to_string(), cur.col as f32 * cw + cw / 2.0, cur.row as f32 * ch + ch / 2.0 + 0.5);
    }

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
    painter.restore()
}

/// The minimap's line numbers down the left and column numbers along the
/// top: every 5th (or 10th, when cells are small), and the cursor's own.
fn draw_minimap_rulers(painter: Painter, cur: Pos, rows: usize, cols: usize, cw: f32, ch: f32) -> Painter {
    let (gx, gy) = MINI_GUTTER;
    let accent = 0xfff3b0;
    let mut painter = painter.font("600 9px monospace").text_baseline(CanvasTextBaseline::Middle);
    let row_step = if ch >= 9.0 { 5 } else { 10 };
    painter = painter.text_align(CanvasTextAlign::Right);
    for row in 0..rows {
        let n = row + 1;
        let here = row == cur.row;
        // Numbers closer than a label's height to the cursor's would overlap it.
        let near = row.abs_diff(cur.row) as f32 * ch < 9.0 && !here;
        if (here || n == 1 || n % row_step == 0) && !near {
            painter = painter
                .fill_color(if here { hex(accent) } else { hexa(0xece6ff, 0.5) })
                .fill_text(&n.to_string(), gx - 4.0, gy + row as f32 * ch + ch / 2.0);
        }
    }
    let col_step = if cw >= 7.0 { 5 } else { 10 };
    painter = painter.text_align(CanvasTextAlign::Center);
    for col in 0..cols {
        let n = col + 1;
        let here = col == cur.col;
        let width = n.to_string().len() as f32 * 6.0;
        let near = (col.abs_diff(cur.col) as f32 * cw) < width && !here;
        if (here || n == 1 || n % col_step == 0) && !near {
            painter = painter
                .fill_color(if here { hex(accent) } else { hexa(0xece6ff, 0.5) })
                .fill_text(&n.to_string(), gx + col as f32 * cw + cw / 2.0, gy / 2.0);
        }
    }
    painter
}

fn draw_burst(at: (f32, f32), color: u32, burst: &AnimatedValue) -> Painter {
    let (cx, cy) = at;
    let radius = burst.mul(30.0).add(4.0);
    let fade = burst.interpolate(&[0.0, 1.0], &[1.0, 0.0]);
    let mut painter = Painter::new()
        .global_alpha(&fade)
        .stroke_color(hex(color))
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
            .fill_color(if k % 2 == 0 { hex(0xffffff) } else { hex(color) })
            .fill_rect(-1.5, -1.5, 3.0, 3.0)
            .restore();
    }
    painter
}

/// A typo bug: a little green beetle, facing the way it crawls.
fn draw_bugs(session: &Session, bob: &AnimatedValue) -> Painter {
    let mut painter = Painter::new();
    for bug in &session.bugs {
        let (cx, cy) = tile_center(bug.at);
        let d = bug.dir as f32;
        painter = painter
            .fill_color(hexa(0x000000, 0.3))
            .begin_path()
            .ellipse_at(cx, cy + 4.0, 11.0, 4.0)
            .fill()
            .save()
            .translate(cx, cy - 6.0)
            .translate(&bob.mul(0.4), 0.0)
            .stroke_color(hex(0x1d3a10))
            .line_width(1.6)
            .line_cap(LineCap::Round);
        // Legs, three a side.
        for k in [-5.0f32, 0.0, 5.0] {
            painter = painter.begin_path().move_to(k, -2.0).line_to(k - 2.0, -8.0).move_to(k, 2.0).line_to(k - 2.0, 8.0).stroke();
        }
        painter = painter
            .fill_color(hex(BUG))
            .begin_path()
            .ellipse_at(0.0, 0.0, 9.0, 6.5)
            .fill()
            .stroke_color(hex(0x2d5a1a))
            .line_width(1.0)
            .begin_path()
            .move_to(-8.0, 0.0)
            .line_to(8.0, 0.0)
            .stroke()
            .fill_color(hex(0x2d5a1a))
            .begin_path()
            .arc(d * 9.0, 0.0, 4.2, 0.0, TAU)
            .fill()
            .fill_color(hex(0xffffff))
            .begin_path()
            .arc(d * 10.5, -1.8, 1.5, 0.0, TAU)
            .arc(d * 10.5, 1.8, 1.5, 0.0, TAU)
            .fill()
            .stroke_color(hex(0x2d5a1a))
            .begin_path()
            .move_to(d * 11.0, -2.0)
            .line_to(d * 15.0, -6.0)
            .move_to(d * 11.0, 2.0)
            .line_to(d * 15.0, 6.0)
            .stroke()
            .restore();
    }
    painter
}

/// Dust puffs along a long jump, fading as they spread.
fn draw_trail(path: ((f32, f32), (f32, f32)), trail: &AnimatedValue) -> Painter {
    let ((x0, y0), (x1, y1)) = path;
    let fade = trail.interpolate(&[0.0, 1.0], &[0.7, 0.0]);
    let grow = trail.mul(5.0).add(2.0);
    let n = ((x1 - x0).hypot(y1 - y0) / 14.0).clamp(3.0, 40.0) as usize;
    let mut painter = Painter::new().global_alpha(&fade).fill_color(hex(0xfff3b0));
    for k in 0..n {
        let t = k as f32 / n as f32;
        let (x, y) = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t - 4.0);
        painter = painter.save().translate(x, y + (k % 2) as f32 * 3.0).begin_path().arc(0.0, 0.0, &grow, 0.0, TAU).fill().restore();
    }
    painter
}

fn draw_sky(size: (f32, f32), motion: &Motion, theme: story::Theme) -> Painter {
    let (w, h) = size;
    let [top, mid, low] = theme.sky;
    let sky = CanvasGradient::linear(point(px(0.0), px(0.0)), point(px(0.0), px(h)))
        .add_color_stop(percentage(0.0), hex(top))
        .add_color_stop(percentage(0.6), hex(mid))
        .add_color_stop(percentage(1.0), hex(low));
    let mut painter = Painter::new().fill_style(sky).fill_rect(0.0, 0.0, w, h);
    let (mx, my) = (w * 0.82, h * 0.16);
    if theme.day {
        // A low, warm sun.
        painter = painter
            .fill_style(
                CanvasGradient::radial(point(px(mx), px(my)), px(10.0), point(px(mx), px(my)), px(120.0))
                    .add_color_stop(percentage(0.0), hexa(0xffe6a8, 0.45))
                    .add_color_stop(percentage(1.0), hexa(0xffe6a8, 0.0)),
            )
            .begin_path()
            .arc(mx, my, 120.0, 0.0, TAU)
            .fill()
            .fill_color(hex(0xfff0c4))
            .begin_path()
            .arc(mx, my, 26.0, 0.0, TAU)
            .fill();
        return draw_weather(draw_clouds(painter, size, motion), size, motion, theme.weather);
    }
    // The moon.
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
        .fill_color(hexa(mid, 0.9))
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
    draw_weather(draw_clouds(painter, size, motion), size, motion, theme.weather)
}

/// Drifting clouds low on the horizon.
fn draw_clouds(mut painter: Painter, size: (f32, f32), motion: &Motion) -> Painter {
    let (w, h) = size;
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
    painter
}

/// The chapter's weather: fireflies and motes drift; snow, rain, leaves
/// and embers fall (or rise) on a loop.
fn draw_weather(mut painter: Painter, size: (f32, f32), motion: &Motion, weather: story::Weather) -> Painter {
    use story::Weather;
    let (w, h) = size;
    if !matches!(weather, Weather::Fireflies | Weather::Motes) {
        // Two copies a screen apart, moving one screen per loop: seamless.
        let rising = weather == Weather::Embers;
        let shift = if rising { motion.fall.mul(-h) } else { motion.fall.mul(h) };
        painter = painter.save().translate(0.0, &shift);
        for copy in [0.0f32, if rising { 1.0 } else { -1.0 }] {
            for i in 0..60u32 {
                let x = noise(Pos { row: i as usize + 90, col: 13 }) * w;
                let y = noise(Pos { row: 17, col: i as usize + 120 }) * h + copy * h;
                let s = 0.6 + noise(Pos { row: i as usize, col: 99 });
                painter = match weather {
                    Weather::Snow => painter.fill_color(hexa(0xffffff, 0.75)).begin_path().arc(x, y, 1.4 * s + 0.6, 0.0, TAU).fill(),
                    Weather::Rain => painter
                        .stroke_color(hexa(0xb8d0ff, 0.35))
                        .line_width(1.0)
                        .begin_path()
                        .move_to(x, y)
                        .line_to(x - 3.0, y + 12.0 * s)
                        .stroke(),
                    Weather::Leaves => painter
                        .fill_color(hexa(if i % 2 == 0 { 0xd98a3a } else { 0xc9b04a }, 0.7))
                        .begin_path()
                        .ellipse_at(x, y, 3.5 * s, 1.8 * s)
                        .fill(),
                    _ => painter.fill_color(hexa(0xffa04a, 0.7)).begin_path().arc(x, y, 1.2 * s, 0.0, TAU).fill(),
                };
            }
        }
        return painter.restore();
    }
    let glow = if weather == Weather::Motes { 0xcfe0ff } else { 0xfff7a0 };
    painter = painter.save().translate(&motion.fly_x, &motion.fly_y);
    for i in 0..14u32 {
        let fx = noise(Pos { row: i as usize + 40, col: 3 }) * w;
        let fy = h * 0.35 + noise(Pos { row: 5, col: i as usize + 60 }) * h * 0.6;
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
        beat: signal(0_u64),
        burst_color: signal(GEM),
        trail: signal(((0.0_f32, 0.0_f32), (0.0_f32, 0.0_f32))),
        chapter: signal(0_usize),
    };
    set_interval(std::time::Duration::from_millis(520), move || ui.beat.update(|b| *b += 1));
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
        fall: animated_value(0.0),
        trail: animated_value(1.0),
        boss: animated_value(0.0),
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
    motion.fall.keyframes(
        [Keyframe::new(0.0), Keyframe::new(1.0)],
        KeyframeTiming::new(9_000.0).easing(Easing::Linear).infinite().frame_rate(30.0),
    );

    audio::load_all();
    audio::start_ambience();
    bind_keys(ui, motion.clone());
    follow_url(ui);
    with_state(|s| s.new_run());
    sync_theme(ui);

    let sky_motion = motion.clone();
    let screen_motion = motion.clone();
    view()
        .flex_1()
        .bg(hex(SKY_TOP))
        .text_color(hex(0xece6ff))
        .on_layout(move |b: &Bounds<Points>| ui.window.set((b.size.width.0, b.size.height.0)))
        .child(
            canvas(move || {
                let chapter = ui.chapter.get();
                draw_sky(ui.window.get(), &sky_motion, story::CHAPTERS[chapter.min(story::CHAPTERS.len() - 1)].theme)
            })
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0(),
        )
        .child(
            view().flex_1().p_safe_area().child(dynamic(move || {
                ui.scene.get();
                let which = with_state(|s| match s.screen {
                    Screen::Play(_) => 0,
                    Screen::Menu { .. } => 1,
                    Screen::Grammar { .. } => 2,
                    Screen::Replay(_) => 3,
                });
                match which {
                    0 => play_screen(ui, screen_motion.clone()).into_element(),
                    1 => worldmap::menu_screen(ui, screen_motion.clone()).into_element(),
                    2 => screens::grammar_screen(ui).into_element(),
                    _ => replay::replay_screen(ui).into_element(),
                }
            })),
        )
}

// --------------------------------------------------------------------- url

/// An isle's name in the address: its title in lower case, words joined by
/// dashes ("Tag, You're It" is `tag-youre-it`), so links survive new isles.
fn slug(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if c == ' ' && !out.ends_with('-') {
            out.push('-');
        }
    }
    out
}

/// The isle an address names (`…#looking-back`), if any.
fn isle_in(url: &str) -> Option<usize> {
    let (_, fragment) = url.split_once('#')?;
    LEVELS.iter().position(|l| slug(l.title) == fragment)
}

thread_local! {
    /// Set once the address the app opened with has been read: before
    /// that, the screen must not overwrite it.
    static URL_READ: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Keeps the current isle in the address (`#slug`, `#map` on the map), so
/// reloading the page restarts the isle; opens the isle an address names,
/// at start and when the browser's back or forward changes it.
fn follow_url(ui: Ui) {
    effect(move || {
        ui.scene.get();
        if !URL_READ.with(|r| r.get()) {
            return;
        }
        let here = with_state(|s| match &s.screen {
            Screen::Play(p) if p.level().chapter == "Daily Tide" => "daily".to_string(),
            Screen::Play(p) if !p.in_campaign() => "review".to_string(),
            Screen::Play(p) => slug(LEVELS[p.index].title),
            Screen::Grammar { .. } => "grammar".to_string(),
            Screen::Menu { .. } | Screen::Replay(_) => "map".to_string(),
        });
        let _ = set_app_url(format!("#{here}"));
    });
    spawn(async move {
        let url = app_url().await;
        if let Some(i) = url.as_deref().and_then(isle_in) {
            with_state(|s| s.open(i));
        } else if url.as_deref().is_some_and(|u| u.ends_with("#daily")) {
            with_state(open_daily);
        }
        sync_theme(ui);
        URL_READ.with(|r| r.set(true));
        ui.scene.update(|s| *s += 1);
        loop {
            let change = next_app_url_change().await;
            let target = isle_in(&change.url);
            with_state(|s| match target {
                Some(i) => s.open(i),
                None => {
                    let selected = match &s.screen {
                        Screen::Play(p) => p.index,
                        Screen::Replay(r) => r.session.index,
                        Screen::Menu { selected } | Screen::Grammar { selected } => *selected,
                    };
                    s.screen = Screen::Menu { selected };
                    s.intro = false;
                }
            });
            sync_theme(ui);
            ui.scene.update(|s| *s += 1);
            ui.tick.update(|t| *t += 1);
        }
    });
}

fn session_value<R>(f: impl FnOnce(&Session) -> R) -> Option<R> {
    with_state(|s| match &s.screen {
        Screen::Play(session) => Some(f(session)),
        _ => None,
    })
}

fn play_screen(ui: Ui, motion: Rc<Motion>) -> impl IntoElement {
    let Some((index, level, campaign)) = session_value(|s| (s.index, s.level(), s.in_campaign())) else { return view().into_element() };
    // Place the wisp at once when the level opens.
    place_hero(ui, &motion, false);

    let gems = motion.clone();
    let hero = motion.clone();
    let burst = motion.clone();
    let card = motion.clone();
    let minimap = motion.clone();
    let bugs = motion.clone();
    let trail = motion.clone();
    let boss = motion.clone();
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
                        .child(canvas(move || draw_trail(ui.trail.get(), &trail.trail)).absolute().top_0().left_0().right_0().bottom_0())
                        .child(
                            canvas(move || {
                                ui.tick.get();
                                session_value(|s| draw_bugs(s, &bugs.bob)).unwrap_or_default()
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
                            canvas(move || draw_burst(ui.burst_at.get(), ui.burst_color.get(), &burst.burst))
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
        .child(screens::boss_banner(ui, boss))
        .child(screens::hint_card(ui))
        .child(status_line(ui))
        .child(complete_card(ui, card));

    let top = view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_4()
        .child(
            view()
                .flex_1()
                .min_w(px(220.0))
                .gap_1()
                .child(text(level.chapter).text_size(px(12.0)).letter_spacing(px(1.5)).text_color(hexa(0xc9b8ff, 0.75)))
                .child(
                    text(if campaign { format!("{}. {}", index + 1, level.title) } else { level.title.to_string() })
                        .text_size(px(24.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(hex(0xfff3b0)),
                ),
        )
        .child(stat("KEYS", move || {
            ui.tick.get();
            session_value(|s| match s.level().budget() {
                Some(b) => format!("{}/{b}", s.keys.len()),
                None => s.keys.len().to_string(),
            })
            .unwrap_or_default()
        }))
        .child(stat("TIME", move || {
            ui.tick.get();
            ui.beat.get();
            with_state(|s| match (s.took, s.started) {
                (Some(t), _) => clock(t),
                (None, Some(t)) => clock(now_ms() - t),
                (None, None) => clock(0.0),
            })
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
            if !campaign {
                return "–".to_string();
            }
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
            let chips = view().flex_row().flex_wrap().gap_2().items_center().children(level.teaches.iter().map(|(keys, what)| {
                view()
                    .flex_row()
                    .gap_1()
                    .items_center()
                    .child(chip(*keys, 0xfff3b0))
                    .child(text(*what).text_size(px(12.0)).text_color(hexa(0xece6ff, 0.7)).mr(px(6.0)))
            }));
            let goal = level.goal_text().map(|_| {
                view()
                    .flex_row()
                    .gap_3()
                    .items_start()
                    .px(px(10.0))
                    .py(px(6.0))
                    .rounded(px(8.0))
                    .bg(hexa(0x000000, 0.35))
                    .child(text("GOAL").text_size(px(11.0)).letter_spacing(px(1.5)).text_color(hexa(0xc9b8ff, 0.7)).mt(px(3.0)))
                    .child(view().flex_1().child(goal_view(ui, 12.0)))
            });
            view().gap_2().child(chips).child(lesson_checklist(ui, level.title)).child(goal)
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
                .child(dynamic(move || (ui.window.get().0 >= 980.0).then(|| side_panel(level, ui)))),
        )
        .child(intro::intro_modal(ui))
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
                    Mode::Block => "-- VISUAL BLOCK --".to_string(),
                    Mode::Replace => "-- REPLACE --".to_string(),
                    Mode::Command { .. } => "COMMAND".to_string(),
                    Mode::Confirm => "CONFIRM".to_string(),
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
                // Read before the session: both live in the state.
                let msg = with_state_toast();
                session_value(|s| match s.editor.mode {
                    Mode::Command { .. } => format!("{}{}▏", s.editor.cmd_prefix(), s.editor.cmdline),
                    Mode::Confirm => s.editor.confirm_prompt(),
                    _ => {
                        if !msg.is_empty() {
                            msg
                        } else if !s.message.is_empty() {
                            s.message.clone()
                        } else if !s.tip.is_empty() {
                            format!("💡 {}", s.tip)
                        } else {
                            String::new()
                        }
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
                session_value(|s| match s.editor.recording_macro() {
                    Some(r) => format!("recording @{r}  {}", s.editor.pending()),
                    None => s.editor.pending(),
                })
                .unwrap_or_default()
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

/// Goal characters already in the buffer.
const PRESENT: u32 = 0x8fe3a0;

/// The commands the isle must see used, ticked off as they are.
fn lesson_checklist(ui: Ui, title: &'static str) -> impl IntoElement {
    if lessons::requirements(title).is_empty() {
        return view().into_element();
    }
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(text("USE ON THIS ISLE").text_size(px(11.0)).letter_spacing(px(1.5)).text_color(hexa(0xc9b8ff, 0.7)).mr(px(4.0)))
        .child(dynamic(move || {
            ui.tick.get();
            let left = session_value(|s| s.lesson_left()).unwrap_or_default();
            Some(view().flex_row().flex_wrap().gap_2().children(lessons::requirements(title).iter().map(move |(label, _)| {
                let done = !left.contains(label);
                text(format!("{} {label}", if done { "✓" } else { "○" }))
                    .font_family(MONO)
                    .text_size(px(12.5))
                    .text_color(if done { hex(PRESENT) } else { hexa(0xece6ff, 0.6) })
            })))
        }))
        .into_element()
}

/// The goal text, each character green when the buffer already has it and
/// amber while it's missing, and a count of what's left to do.
fn goal_view(ui: Ui, size: f32) -> impl IntoElement {
    dynamic(move || {
        ui.tick.get();
        let (goal, d) = session_value(|s| (goal_lines(s), session_diff(s)))?;
        let (goal, d) = (goal?, d?);
        let (have, add, remove) = (d.present_count(), d.missing_count(), d.wrong_count());
        let rows = goal.into_iter().enumerate().map(move |(g, line)| {
            let mut runs: Vec<(String, bool)> = Vec::new();
            for (j, c) in line.iter().enumerate() {
                let ok = d.present[g][j];
                // Spaces still to type show as a dot, so they can be seen.
                let shown = match (*c, ok) {
                    (' ', true) => '\u{a0}',
                    (' ', false) => '␣',
                    (c, _) => c,
                };
                match runs.last_mut() {
                    Some((t, o)) if *o == ok => t.push(shown),
                    _ => runs.push((shown.to_string(), ok)),
                }
            }
            if line.is_empty() {
                let missing = d.missing_lines.iter().any(|&(_, mg)| mg == g);
                runs.push((if missing { "⏎ empty line".into() } else { "\u{a0}".into() }, !missing));
            }
            view().flex_row().flex_wrap().children(runs.into_iter().map(move |(t, ok)| {
                text(t)
                    .font_family(MONO)
                    .text_size(px(size))
                    .line_height(px(size * 1.45))
                    .font_weight(if ok { FontWeight::NORMAL } else { FontWeight::BOLD })
                    .text_color(hex(if ok { PRESENT } else { MISSING }))
            }))
        });
        let summary = view()
            .flex_row()
            .flex_wrap()
            .gap_2()
            .mt(px(6.0))
            .child(text(format!("✓ {have} in place")).text_size(px(12.0)).text_color(hex(PRESENT)))
            .child(text(format!("+ {add} to add")).text_size(px(12.0)).text_color(hex(MISSING)))
            .child(text(format!("✕ {remove} to remove")).text_size(px(12.0)).text_color(hex(0xff8a70)));
        Some(view().child(view().children(rows)).child(summary))
    })
}

fn side_panel(level: &'static Level, ui: Ui) -> impl IntoElement {
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
        .child(lesson_checklist(ui, level.title))
        .child(level.goal_text().map(|_| {
            view()
                .gap_2()
                .mt(px(4.0))
                .child(text("MAKE THE TEXT READ").text_size(px(11.0)).letter_spacing(px(1.5)).text_color(hexa(0xc9b8ff, 0.7)))
                .child(
                    view()
                        .p(px(10.0))
                        .rounded(px(8.0))
                        .bg(hexa(0x000000, 0.35))
                        .child(goal_view(ui, 13.0)),
                )
                .child(
                    text("Green is in place, amber still missing; red tiles on the isle are extra.")
                        .text_size(px(11.5))
                        .line_height(px(16.0))
                        .text_color(hexa(0xece6ff, 0.5)),
                )
        }))
        .child((!level.files().is_empty() || !level.want_files().is_empty()).then(|| {
            let shelf = level.files().iter().map(|(n, _)| *n).collect::<Vec<_>>().join("  ");
            view()
                .gap_1()
                .mt(px(4.0))
                .child(text("ON THE SHELF").text_size(px(11.0)).letter_spacing(px(1.5)).text_color(hexa(0xc9b8ff, 0.7)))
                .child(text(shelf).font_family(MONO).text_size(px(12.5)).text_color(hexa(0xece6ff, 0.8)))
                .children(level.want_files().iter().map(move |&(name, want)| {
                    view()
                        .gap_1()
                        .mt(px(4.0))
                        .child(
                            text(move || {
                                ui.tick.get();
                                let done = session_value(|s| s.editor.files.get(name).is_some_and(|f| f == want)).unwrap_or(false);
                                format!("{} AND WRITE {name}", if done { "✓" } else { "…" })
                            })
                            .text_size(px(11.0))
                            .letter_spacing(px(1.2))
                            .text_color(hex(PRESENT)),
                        )
                        .child(text(want.to_string()).font_family(MONO).text_size(px(12.5)).text_color(hexa(0xece6ff, 0.75)))
                }))
        }))
        .child((level.goal.is_none() && !level.gems().is_empty()).then(|| {
            text("Collect every crystal ◆ to clear the isle.").text_size(px(13.0)).text_color(hex(GEM)).mt(px(4.0))
        }))
        .child((!level.bugs().is_empty()).then(|| {
            text("Squash every bug: land on it. Each command moves them a tile, and they hatch twins every 8.")
                .text_size(px(13.0))
                .line_height(px(18.0))
                .text_color(hex(BUG))
                .mt(px(4.0))
        }))
        .child(view().flex_1())
        .child(text(format!("Tip: {}", level.tip)).text_size(px(12.5)).line_height(px(18.0)).text_color(hexa(0xece6ff, 0.55)))
        .child(
            text(":h lessons · :e! restart · :q menu · :n skip · :mute")
                .font_family(MONO)
                .text_size(px(11.5))
                .text_color(hexa(0xece6ff, 0.4)),
        )
}

/// The golf solution, hidden until the pointer is over it (or it is
/// pressed, on touch screens), so it doesn't spoil a retry.
fn spoiler(solution: String) -> impl IntoElement {
    let shown = signal(false);
    let masked: String = solution.chars().map(|_| '•').collect();
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .px(px(8.0))
        .py(px(4.0))
        .ml(px(-8.0))
        .rounded(px(6.0))
        .bg(move || if shown.get() { hexa(GEM, 0.08) } else { hexa(0xffffff, 0.05) })
        .cursor_pointer()
        .on_mouse_enter(move |_: &MouseEvent| shown.set(true))
        .on_mouse_leave(move |_: &MouseEvent| shown.set(false))
        .on_press(move || shown.update(|s| *s = !*s))
        .child(
            text(move || if shown.get() { solution.clone() } else { masked.clone() })
                .font_family(MONO)
                .text_size(px(14.0))
                .text_color(move || if shown.get() { hex(GEM) } else { hexa(GEM, 0.35) }),
        )
        .child(dynamic(move || {
            (!shown.get()).then(|| text("hover to reveal").text_size(px(11.0)).italic().text_color(hexa(0xffffff, 0.4)))
        }))
}

fn complete_card(ui: Ui, motion: Rc<Motion>) -> impl IntoElement {
    let scale = motion.card.interpolate(&[0.0, 1.0], &[0.85, 1.0]);
    dynamic(move || {
        ui.tick.get();
        let done = session_value(|s| s.complete).unwrap_or(false);
        if !done {
            return None;
        }
        let (index, keys, stars, par, level, campaign, boss) =
            session_value(|s| (s.index, format_keys(&s.keys), s.stars(), s.par(), s.level(), s.in_campaign(), is_boss(s))).unwrap();
        let (took, best_time, drills_left) = with_state(|s| (s.took.unwrap_or(0.0), s.times[index], s.drills.len()));
        let count = session_value(|s| s.keys.len()).unwrap();
        let (habits, trapped) = session_value(|s| (s.coach.spotted.iter().map(|h| h.label.clone()).collect::<Vec<_>>(), s.trapped)).unwrap();
        let last = campaign && index + 1 == LEVELS.len();
        let missed = session_value(|s| s.missed.clone()).unwrap_or_default();
        let failed = !missed.is_empty();
        let verdict = if failed {
            "Lesson missed: the isle isn't cleared."
        } else if count < par {
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
                            text(if failed { "✗".to_string() } else { format!("{}{}", "★".repeat(stars as usize), "☆".repeat(3 - stars as usize)) })
                                .text_size(px(40.0))
                                .text_color(hex(if failed { 0xff8a70 } else { 0xffd76b })),
                        )
                        .child(text(verdict).text_size(px(18.0)).font_weight(FontWeight::BOLD).text_color(hex(0xfff3b0)))
                        .child((boss && !failed).then(|| {
                            let name = story::chapter(level.chapter).map_or("", |c| c.boss);
                            text(format!("⚔ {name} is defeated!")).text_size(px(14.0)).font_weight(FontWeight::BOLD).text_color(hex(0xff9a6b))
                        }))
                        .child(
                            text(format!(
                                "⏱ {}{}",
                                clock(took),
                                match best_time {
                                    Some(t) if campaign && !failed => format!("   ·   best {}", clock(t as f64 * 100.0)),
                                    _ => String::new(),
                                }
                            ))
                            .font_family(MONO)
                            .text_size(px(13.0))
                            .text_color(hexa(0xece6ff, 0.75)),
                        )
                        .child((last && !failed).then(|| {
                            // The ending: the archipelago restored, and the stars of the whole journey.
                            let (stars, cleared) = with_state(|s| {
                                let stars: usize = s.best.iter().enumerate().filter_map(|(i, b)| b.map(|n| game::stars(n, LEVELS[i].par()) as usize)).sum();
                                (stars, s.best.iter().filter(|b| b.is_some()).count())
                            });
                            view()
                                .w_full()
                                .gap_2()
                                .p(px(12.0))
                                .rounded(px(10.0))
                                .bg(hexa(0xfff3b0, 0.08))
                                .border_1()
                                .border_color(hexa(0xfff3b0, 0.35))
                                .items_center()
                                .child(text("THE END").text_size(px(13.0)).letter_spacing(px(3.0)).font_weight(FontWeight::BOLD).text_color(hex(0xfff3b0)))
                                .child(
                                    text(
                                        "The last door of the Archive swings open. Every isle of the Buffer Archipelago answers to \
                                         your keys now: no arrows, no mouse, just the home row. The wisp settles on the summit \
                                         and looks out over the clouds, already counting keystrokes it could have saved.",
                                    )
                                    .text_size(px(13.5))
                                    .line_height(px(19.0))
                                    .italic()
                                    .text_center()
                                    .text_color(hexa(0xece6ff, 0.85)),
                                )
                                .child(
                                    text(format!("{cleared} of {} isles cleared · ★ {stars} of {}", LEVELS.len(), LEVELS.len() * 3))
                                        .font_family(MONO)
                                        .text_size(px(13.0))
                                        .text_color(hex(0xffd76b)),
                                )
                                .child(
                                    text("Now open a real file in vim. Every command here works there too.")
                                        .text_size(px(12.5))
                                        .text_color(hexa(0xece6ff, 0.6)),
                                )
                        }))
                        .child(failed.then(|| {
                            view()
                                .w_full()
                                .gap_2()
                                .p(px(10.0))
                                .rounded(px(10.0))
                                .bg(hexa(0xff8a70, 0.12))
                                .border_1()
                                .border_color(hexa(0xff8a70, 0.4))
                                .child(
                                    text("You reached the goal without using what this isle teaches. Use these, then it counts:")
                                        .text_size(px(13.5))
                                        .line_height(px(19.0))
                                        .text_color(hexa(0xece6ff, 0.85)),
                                )
                                .child(view().flex_row().flex_wrap().gap_2().children(missed.iter().map(|m| chip(m.clone(), 0xff8a70))))
                        }))
                        .child(
                            view()
                                .w_full()
                                .gap_1()
                                .mt(px(6.0))
                                .child(text(format!("YOUR KEYS · {count}")).text_size(px(11.0)).letter_spacing(px(1.2)).text_color(hexa(0xffffff, 0.5)))
                                .child(text(keys).font_family(MONO).text_size(px(14.0)).text_color(hex(0xece6ff)))
                                .child(text(format!("GOLF SOLUTION · {par}")).text_size(px(11.0)).letter_spacing(px(1.2)).text_color(hexa(0xffffff, 0.5)).mt(px(8.0)))
                                .child(spoiler(format_keys(&vim::parse_keys(level.solution))))
                                .child(
                                    view()
                                        .flex_row()
                                        .items_center()
                                        .gap_2()
                                        .mt(px(6.0))
                                        .child(chip("p", GEM))
                                        .child(text("replay your run beside the golf line, step by step").text_size(px(12.5)).text_color(hexa(0xffffff, 0.6))),
                                ),
                        )
                        .child((!habits.is_empty() || trapped > 0).then(|| {
                            view()
                                .w_full()
                                .gap_1()
                                .mt(px(4.0))
                                .child(text("HABITS TO BREAK").text_size(px(11.0)).letter_spacing(px(1.2)).text_color(hexa(0xffffff, 0.5)))
                                .children(habits.into_iter().take(4).map(|h| text(format!("• {h}")).font_family(MONO).text_size(px(13.0)).text_color(hex(0xffd76b))))
                                .child((trapped > 0).then(|| {
                                    text(format!("• thrown back by a trap {trapped}×: jump past it in one move"))
                                        .text_size(px(13.0))
                                        .text_color(hex(0xff8a70))
                                }))
                        }))
                        .child(
                            view()
                                .flex_row()
                                .gap_2()
                                .mt(px(10.0))
                                .items_center()
                                .child(chip("Enter", 0xfff3b0))
                                .child(
                                    text(if failed {
                                        "try again".to_string()
                                    } else if drills_left > 0 {
                                        format!("next drill ({drills_left} left)")
                                    } else if last || !campaign {
                                        "back to the map".to_string()
                                    } else {
                                        "next isle".to_string()
                                    })
                                    .text_color(hexa(0xffffff, 0.65))
                                    .mr(px(10.0)),
                                )
                                .child(chip(if failed { "n" } else { "r" }, 0xfff3b0))
                                .child(text(if failed { "skip it" } else { "retry" }).text_color(hexa(0xffffff, 0.65)).mr(px(10.0)))
                                .child(chip("q", 0xfff3b0))
                                .child(text("map").text_color(hexa(0xffffff, 0.65))),
                        ),
                ),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_isle_has_its_own_address() {
        assert_eq!(slug("Tag, You're It"), "tag-youre-it");
        assert_eq!(slug("Wind-up Golem"), "windup-golem");
        let mut slugs: Vec<String> = LEVELS.iter().map(|l| slug(l.title)).collect();
        slugs.sort();
        slugs.dedup();
        assert_eq!(slugs.len(), LEVELS.len());
        for (i, level) in LEVELS.iter().enumerate() {
            let url = format!("https://antonygarand.github.io/hjkl-quest/#{}", slug(level.title));
            assert_eq!(isle_in(&url), Some(i));
        }
        assert_eq!(isle_in("https://x/hjkl-quest/#map"), None);
        assert_eq!(isle_in("https://x/hjkl-quest/"), None);
    }
}
