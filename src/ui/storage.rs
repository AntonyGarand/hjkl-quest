//! Progress kept between visits, in the platform's key-value storage:
//! records per isle as `title=value|…`, so new isles don't shift them.

use std::collections::BTreeSet;

use nsh_core::storage::Storage;

use crate::levels::LEVELS;
use crate::ui::state::{State, start_screen};

const PROGRESS: &str = "hjkl-quest.progress";
const SEEN: &str = "hjkl-quest.seen-isles";
const GRAMMAR: &str = "hjkl-quest.grammar";
const TIMES: &str = "hjkl-quest.times";
const FREE: &str = "hjkl-quest.free";
const PRACTICED: &str = "hjkl-quest.practiced";
const CLEARS: &str = "hjkl-quest.clears";
const DAILY: &str = "hjkl-quest.daily";

impl State {
    /// The state a visit starts with: the saved records, on the map.
    pub(crate) fn load() -> State {
        State {
            screen: start_screen(),
            best: load_progress(),
            toast: String::new(),
            intro: false,
            seen: load_seen(),
            grammar: load_grammar(),
            times: load_numbers(TIMES)
                .into_iter()
                .map(|t| t.map(|t| t as u32))
                .collect(),
            free_best: load_numbers(FREE)
                .into_iter()
                .map(|t| t.map(|t| t as u32))
                .collect(),
            practiced: load_numbers(PRACTICED)
                .into_iter()
                .map(|t| t.unwrap_or(0) as u32)
                .collect(),
            clears: Storage::get_item(CLEARS)
                .and_then(|c| c.parse().ok())
                .unwrap_or(0),
            started: None,
            took: None,
            work_start: 0,
            drills: Vec::new(),
            daily_best: Storage::get_item(DAILY).and_then(|d| {
                let (day, keys) = d.split_once('=')?;
                Some((day.parse().ok()?, keys.parse().ok()?))
            }),
        }
    }

    /// Counts a clear of isle `i`: review drills pick the stalest isles.
    pub(crate) fn practice(&mut self, i: usize) {
        self.clears += 1;
        self.practiced[i] = self.clears;
        save_numbers(PRACTICED, self.practiced.iter().map(|&p| Some(p)));
        let _ = Storage::set_item(CLEARS, self.clears.to_string());
    }

    pub(crate) fn save_progress(&self) {
        save_entries(
            PROGRESS,
            self.best
                .iter()
                .enumerate()
                .filter_map(|(i, b)| b.map(|n| (i, n.to_string()))),
        );
    }

    pub(crate) fn save_times(&self) {
        save_numbers(TIMES, self.times.iter().copied());
    }

    pub(crate) fn save_free_best(&self) {
        save_numbers(FREE, self.free_best.iter().copied());
    }

    pub(crate) fn save_grammar(&self) {
        let _ = Storage::set_item(
            GRAMMAR,
            self.grammar.iter().cloned().collect::<Vec<_>>().join("|"),
        );
    }

    pub(crate) fn save_daily(&self) {
        if let Some((day, keys)) = self.daily_best {
            let _ = Storage::set_item(DAILY, format!("{day}={keys}"));
        }
    }
}

/// Keeps which isles' intros have been seen.
pub(crate) fn save_seen(seen: &[bool]) {
    save_entries(
        SEEN,
        seen.iter()
            .enumerate()
            .filter(|(_, s)| **s)
            .map(|(i, _)| (i, "1".to_string())),
    );
}

/// The isle with this title.
pub(crate) fn index_of(title: &str) -> Option<usize> {
    LEVELS.iter().position(|l| l.title == title)
}

fn load_progress() -> Vec<Option<usize>> {
    let mut best = vec![None; LEVELS.len()];
    for (i, v) in load_entries(PROGRESS, "hjkl-quest.best", ':') {
        best[i] = v.parse().ok();
    }
    best
}

fn load_seen() -> Vec<bool> {
    let mut seen = vec![false; LEVELS.len()];
    for (i, _) in load_entries(SEEN, "hjkl-quest.seen", ',') {
        seen[i] = true;
    }
    seen
}

fn load_grammar() -> BTreeSet<String> {
    Storage::get_item(GRAMMAR)
        .map_or_default(|g| g.split('|').filter(|p| !p.is_empty()).map(String::from).collect())
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
    save_entries(
        key,
        values
            .enumerate()
            .filter_map(|(i, v)| v.map(|v| (i, v.to_string()))),
    );
}

/// Saved `title=value` pairs, or (from older saves) `index:value` pairs.
fn load_entries(key: &str, old_key: &str, old_sep: char) -> Vec<(usize, String)> {
    if let Some(saved) = Storage::get_item(key) {
        return saved
            .split('|')
            .filter_map(|e| e.split_once('='))
            .filter_map(|(t, v)| Some((index_of(t)?, v.to_string())))
            .collect();
    }
    let Some(saved) = Storage::get_item(old_key) else {
        return Vec::new();
    };
    saved
        .split(',')
        .filter_map(|e| {
            let (i, v) = if old_sep == ':' {
                e.split_once(':')?
            } else {
                (e, "1")
            };
            let title = OLD_ORDER.get(i.parse::<usize>().ok()?)?;
            Some((index_of(title)?, v.to_string()))
        })
        .collect()
}

fn save_entries(key: &str, entries: impl Iterator<Item = (usize, String)>) {
    let text = entries
        .map(|(i, v)| format!("{}={v}", LEVELS[i].title))
        .collect::<Vec<_>>()
        .join("|");
    let _ = Storage::set_item(key, text);
}

/// The isles in the order the first saves used, when progress was kept by
/// position: isles added since then would shift it, so it is now kept by title.
const OLD_ORDER: &[&str] = &[
    "First Steps",
    "Counting Sheep",
    "Word Hopping",
    "Punctuation Pass",
    "Edge of the World",
    "Long Jumps",
    "Seek and Find",
    "Paragraph Isles",
    "Mirror Matching",
    "Search Party",
    "Chomp",
    "Demolition",
    "Change of Heart",
    "Insert Coin",
    "Copycat",
    "Time Travel",
    "Inside Out",
    "Highlights",
    "Ex Marks the Spot",
    "The Refactor",
    "Named Vaults",
    "Waymarks",
    "Tally Marks",
    "Wind-up Golem",
    "Assembly Line",
    "Terraces",
    "Shouting Match",
    "Overwrite",
    "Ice Columns",
    "Next Match",
    "Tag, You're It",
    "Global Decree",
    "Moving Day",
    "Normal Order",
    "The Grand Refactor",
];
