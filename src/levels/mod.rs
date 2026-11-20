//! The campaign: fifty-seven isles of the Buffer Archipelago, each teaching a few
//! commands. Every level's `solution` is a vim-golf line: the tests replay it
//! and check that it solves the level in exactly `par` keystrokes.

use crate::game::{Bug, Sentry};
use crate::vim::{Pos, parse_keys, pos};

mod beyond;
mod highlands;
mod mainland;
mod mending;
mod power;
mod shallows;
mod straits;

/// One isle: its story, what it teaches, the map and how to clear it.
#[derive(Clone, Copy)]
pub struct Level {
    pub chapter: &'static str,
    pub title: &'static str,
    pub story: &'static str,
    /// The commands this level introduces: (keys, what they do).
    pub teaches: &'static [(&'static str, &'static str)],
    /// The normal-mode keys this level unlocks.
    pub unlock: &'static str,
    /// The buffer: the island the player walks on.
    pub map: &'static str,
    /// An overlay of `map`: `@` the start, `*` a crystal to collect, `^` a
    /// trap the cursor must jump over, never land on, `>` and `<` typo bugs.
    pub marks: &'static str,
    /// For editing levels, the text the buffer must become.
    pub goal: Option<&'static str>,
    /// A shortest known solution, in vim-golf notation.
    pub solution: &'static str,
    pub tip: &'static str,
}

impl Level {
    pub fn start(&self) -> Pos {
        self.marked('@').into_iter().next().unwrap_or_default()
    }

    /// Crystals, the fading ones too.
    pub fn gems(&self) -> Vec<Pos> {
        let mut gems = self.marked('*');
        gems.extend(self.fading().into_iter().map(|(p, _)| p));
        gems
    }

    pub fn traps(&self) -> Vec<Pos> {
        self.marked('^')
    }

    /// Crystals that fade: a digit 1-9 in the marks is a crystal with that
    /// many commands to live (0 is ten).
    pub fn fading(&self) -> Vec<(Pos, usize)> {
        let mut out = Vec::new();
        for d in '0'..='9' {
            let life = if d == '0' { 10 } else { d as usize - '0' as usize };
            out.extend(self.marked(d).into_iter().map(|p| (p, life)));
        }
        out
    }

    /// Sentries: `V` marching down, `A` up.
    pub fn sentries(&self) -> Vec<Sentry> {
        let down = self.marked('V').into_iter().map(|at| Sentry { at, dir: 1 });
        down.chain(self.marked('A').into_iter().map(|at| Sentry { at, dir: -1 }))
            .collect()
    }

    /// Frozen lines: a `#` anywhere in a row of the marks.
    pub fn frozen(&self) -> Vec<usize> {
        let mut rows: Vec<usize> = self.marked('#').into_iter().map(|p| p.row).collect();
        rows.dedup();
        rows
    }

    /// Keys this isle forbids, for a challenge.
    pub fn banned(&self) -> &'static str {
        match self.title {
            "No Arrows" => "hjkl",
            "No Delete" => "xXdD",
            _ => "",
        }
    }

    /// Typo bugs: `>` crawling right, `<` left.
    pub fn bugs(&self) -> Vec<Bug> {
        let right = self.marked('>').into_iter().map(|at| Bug { at, dir: 1 });
        right
            .chain(self.marked('<').into_iter().map(|at| Bug { at, dir: -1 }))
            .collect()
    }

    /// The scrolls on the isle's shelf, for `:r`, `:!ls` and Tab.
    pub fn files(&self) -> &'static [(&'static str, &'static str)] {
        match self.title {
            "Scrolls" => &[
                ("ingredients.txt", "flour\neggs\nmilk"),
                ("notes.txt", "Pancakes: mix, rest, fry."),
                ("weather.txt", "sunny"),
            ],
            "Copy Desk" => &[("old.txt", "nothing new")],
            "Two Scrolls" => &[("recipe.txt", "flour\nsugar"), ("notes.txt", "bake at 180")],
            _ => &[],
        }
    }

    /// The most keystrokes the isle allows, if it limits them.
    pub fn budget(&self) -> Option<usize> {
        match self.title {
            "Edge of the World" => Some(10),
            "Every Key Counts" => Some(5),
            _ => None,
        }
    }

    /// Files the goal also wants written, with their text.
    pub fn want_files(&self) -> &'static [(&'static str, &'static str)] {
        match self.title {
            "Copy Desk" => &[("keep.txt", "buy milk\ncall mom")],
            "Two Scrolls" => &[("recipe.txt", "flour\nsugar\neggs")],
            _ => &[],
        }
    }

    fn marked(&self, mark: char) -> Vec<Pos> {
        let mut out = Vec::new();
        for (row, line) in self.marks.trim_start_matches('\n').split('\n').enumerate() {
            for (col, c) in line.chars().enumerate() {
                if c == mark {
                    out.push(pos(row, col));
                }
            }
        }
        out
    }

    pub fn text(&self) -> &'static str {
        self.map.trim_start_matches('\n')
    }

    pub fn goal_text(&self) -> Option<&'static str> {
        self.goal.map(|g| g.trim_start_matches('\n'))
    }

    pub fn par(&self) -> usize {
        parse_keys(self.solution).len()
    }
}

/// The isle whose clearing ends the story; the chapters after it are
/// challenges.
pub const ENDING: &str = "The Grand Refactor";

/// The isle that teaches ex commands (`:`), counted from 1.
pub fn ex_isle() -> usize {
    LEVELS
        .iter()
        .position(|l| l.unlock.contains(':'))
        .map_or(0, |i| i + 1)
}

/// The keys learned by isle `index`: those its isle and the ones before
/// unlock, and the toolkits of the chapters finished before it.
pub fn unlocked(index: usize) -> String {
    let index = index.min(LEVELS.len() - 1);
    let mut keys: String = LEVELS[..=index].iter().map(|l| l.unlock).collect();
    for k in 0..index {
        if LEVELS[k + 1].chapter != LEVELS[k].chapter
            && let Some(c) = crate::story::chapter(LEVELS[k].chapter)
        {
            keys.push_str(c.unlock);
        }
    }
    keys
}

/// The campaign's regions, in order of play.
const REGIONS: &[&[Level]] = &[
    shallows::LEVELS,
    straits::LEVELS,
    mending::LEVELS,
    highlands::LEVELS,
    power::LEVELS,
    mainland::LEVELS,
    beyond::LEVELS,
];

/// How many isles the regions hold together.
const COUNT: usize = {
    let mut count = 0;
    let mut r = 0;
    while r < REGIONS.len() {
        count += REGIONS[r].len();
        r += 1;
    }
    count
};

/// Every isle of every region, one after another.
const fn campaign() -> [Level; COUNT] {
    let mut out = [REGIONS[0][0]; COUNT];
    let mut n = 0;
    let mut r = 0;
    while r < REGIONS.len() {
        let mut i = 0;
        while i < REGIONS[r].len() {
            out[n] = REGIONS[r][i];
            n += 1;
            i += 1;
        }
        r += 1;
    }
    out
}

/// The campaign, isle by isle.
pub static LEVELS: &[Level] = &campaign();
