//! The app's state: the screen on show, the player's records, and the run
//! in progress. One per thread, borrowed through `with_state`.

use std::cell::RefCell;
use std::collections::BTreeSet;

use crate::daily;
use crate::diff::{self, Diff};
use crate::game::Session;
use crate::levels::{LEVELS, Level};
use crate::review;
use crate::ui::replay::Replay;
use crate::ui::storage::{index_of, save_seen};

/// The daily puzzle's chapter, as `daily` names it.
pub(crate) const DAILY_TIDE: &str = "Daily Tide";

pub(crate) enum Screen {
    Menu {
        selected: usize,
    },
    Play(Box<Session>),
    /// The verb × noun grid, from the map.
    Grammar {
        selected: usize,
    },
    /// A finished run played back beside the golf solution.
    Replay(Box<Replay>),
}

pub(crate) struct State {
    pub(crate) screen: Screen,
    /// The fewest keystrokes each level was solved in.
    pub(crate) best: Vec<Option<usize>>,
    pub(crate) toast: String,
    /// The isle's intro is showing (its lessons, before playing).
    pub(crate) intro: bool,
    /// Isles whose intro has been seen.
    pub(crate) seen: Vec<bool>,
    /// Verb + noun pairs the player has used (`grammar::key`).
    pub(crate) grammar: BTreeSet<String>,
    /// The fastest clear of each isle, in tenths of a second.
    pub(crate) times: Vec<Option<u32>>,
    /// Fewest keys on each isle in free play.
    pub(crate) free_best: Vec<Option<u32>>,
    /// When each isle was last cleared, counted in clears: what review
    /// drills pick from.
    pub(crate) practiced: Vec<u32>,
    pub(crate) clears: u32,
    /// The current run: when its first key came, and how long it took (ms).
    pub(crate) started: Option<f64>,
    pub(crate) took: Option<f64>,
    /// How much was left to do when the isle opened: a boss's full health.
    pub(crate) work_start: usize,
    /// Drills to play after this one: (drill, the isle it's from).
    pub(crate) drills: Vec<(&'static Level, usize)>,
    /// The best clear of a daily puzzle: (day, keys).
    pub(crate) daily_best: Option<(u64, usize)>,
}

impl State {
    /// Opens isle `i`, with its intro the first time.
    pub(crate) fn open(&mut self, i: usize) {
        self.screen = Screen::Play(Box::new(Session::new(i)));
        self.intro = !self.seen[i] && self.best[i].is_none();
        self.drills.clear();
        self.new_run();
    }

    /// Opens campaign isle `i` in free play: every key, no lesson to use.
    pub(crate) fn open_free(&mut self, i: usize) {
        self.screen = Screen::Play(Box::new(Session::free_play(i)));
        self.intro = false;
        self.drills.clear();
        self.new_run();
    }

    /// Opens a daily puzzle or a drill, with the keys of isle `index`.
    pub(crate) fn open_level(&mut self, level: &'static Level, index: usize) {
        self.screen = Screen::Play(Box::new(Session::custom(level, index)));
        self.intro = false;
        self.new_run();
    }

    /// Plays the same isle, puzzle or drill again.
    pub(crate) fn retry(&mut self, level: &'static Level, index: usize, campaign: bool) {
        if campaign {
            let drills = std::mem::take(&mut self.drills);
            self.open(index);
            self.intro = false;
            self.drills = drills;
        } else {
            self.open_level(level, index);
        }
    }

    /// Opens today's tide.
    pub(crate) fn open_daily(&mut self) {
        let level: &'static Level = Box::leak(Box::new(daily::daily(daily::today())));
        self.open_level(level, LEVELS.len() - 1);
    }

    /// Opens a round of review drills from the isles cleared longest ago;
    /// false when there is nothing to review yet.
    pub(crate) fn open_review(&mut self) -> bool {
        let mut cleared: Vec<usize> = (0..LEVELS.len()).filter(|&i| self.best[i].is_some()).collect();
        cleared.sort_by_key(|&i| self.practiced[i]);
        let titles: Vec<&str> = cleared.iter().map(|&i| LEVELS[i].title).collect();
        let mut drills: Vec<(&'static Level, usize)> = review::pick(&titles, 5, u64::from(self.clears))
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
        self.open_level(first, from);
        self.drills = drills;
        true
    }

    /// Starts the clock and the boss's health afresh.
    pub(crate) fn new_run(&mut self) {
        self.started = None;
        self.took = None;
        self.work_start = match &self.screen {
            Screen::Play(s) => work(s),
            _ => 0,
        };
    }

    pub(crate) fn close_intro(&mut self) {
        self.intro = false;
        if let Screen::Play(s) = &self.screen {
            self.seen[s.index] = true;
            save_seen(&self.seen);
        }
    }

    /// The isle the screen is on or about (the map's selected one).
    pub(crate) fn current_isle(&self) -> usize {
        match &self.screen {
            Screen::Play(p) => p.index,
            Screen::Replay(r) => r.session.index,
            Screen::Menu { selected } | Screen::Grammar { selected } => *selected,
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::load());
}

pub(crate) fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

/// A look at the isle being played, if one is.
pub(crate) fn session_value<R>(f: impl FnOnce(&Session) -> R) -> Option<R> {
    with_state(|s| match &s.screen {
        Screen::Play(session) => Some(f(session)),
        _ => None,
    })
}

/// The toast, or nothing while the state is busy elsewhere.
pub(crate) fn current_toast() -> String {
    STATE.with(|s| s.try_borrow().map_or_default(|s| s.toast.clone()))
}

/// The menu, or with `HJKL_LEVEL=<n>` (desktop, for development) level n.
pub(crate) fn start_screen() -> Screen {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(n) = std::env::var("HJKL_LEVEL")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        && (1..=LEVELS.len()).contains(&n)
    {
        return Screen::Play(Box::new(Session::new(n - 1)));
    }
    Screen::Menu { selected: 0 }
}

/// The goal of an editing level, as lines of characters.
pub(crate) fn goal_lines(session: &Session) -> Option<Vec<Vec<char>>> {
    session
        .level()
        .goal_text()
        .map(|g| g.split('\n').map(|l| l.chars().collect()).collect())
}

/// How the buffer differs from the goal, on an editing level.
pub(crate) fn session_diff(session: &Session) -> Option<Diff> {
    goal_lines(session).map(|g| diff::diff(&session.editor.lines, &g))
}

/// What's left to do on an isle: crystals, bugs, characters to fix.
pub(crate) fn work(s: &Session) -> usize {
    s.gems_left() + s.bugs.len() + session_diff(s).map_or(0, |d| d.wrong_count() + d.missing_count())
}

/// Whether the isle is its chapter's last: a boss guards it.
pub(crate) fn is_boss(s: &Session) -> bool {
    s.in_campaign() && is_boss_isle(s.index)
}

/// Whether campaign isle `i` is its chapter's last, where a boss waits.
pub(crate) fn is_boss_isle(i: usize) -> bool {
    i + 1 == LEVELS.len() || LEVELS[i + 1].chapter != LEVELS[i].chapter
}

/// The first isle not yet cleared: the ones after the next are in fog.
pub(crate) fn frontier(best: &[Option<usize>]) -> usize {
    best.iter().position(Option::is_none).unwrap_or(best.len())
}

/// The first isle of the chapter isle `i` is in.
pub(crate) fn chapter_start(i: usize) -> usize {
    (0..=i)
        .rev()
        .find(|&k| k == 0 || LEVELS[k - 1].chapter != LEVELS[i].chapter)
        .unwrap_or(0)
}

/// "1:05.3" from milliseconds.
pub(crate) fn clock(ms: f64) -> String {
    let tenths = (ms / 100.0) as u64;
    format!("{}:{:02}.{}", tenths / 600, tenths / 10 % 60, tenths % 10)
}

pub(crate) fn now_ms() -> f64 {
    nsh_core::time::unix_ms()
}
