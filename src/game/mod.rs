//! A level being played: the editor, the crystals, the keystroke count and
//! which keys the player has learned. No UI here either.

use crate::coach::Coach;
use crate::levels::{LEVELS, Level, unlocked};
use crate::vim::{Editor, Key, Mode, Pos};

mod hazards;
mod press;
#[cfg(test)]
mod tests;

/// What a key did, for the UI to react with sound and motion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Moved,
    Edited,
    Gem(Pos),
    Bell,
    Locked(char),
    Complete,
    Command(String),
    /// The cursor landed on a trap (here) and was thrown back.
    Trapped(Pos),
    /// The cursor caught a bug, here.
    Squashed(Pos),
    /// A sentry caught the cursor, here, and sent it back to the start.
    Caught(Pos),
    None,
}

/// A typo bug: it crawls one tile along its line per keystroke, turning
/// at the ends, and hatches a twin now and then. Land on it to squash it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bug {
    pub at: Pos,
    /// +1 crawling right, -1 left.
    pub dir: i8,
}

/// Commands between hatchings, and the most bugs an isle can hold.
pub const HATCH_EVERY: usize = 8;
pub const MAX_BUGS: usize = 6;

/// A sentry: it marches up and down its column, one row per command, and
/// turns at the isle's edge or where its column runs out. Whoever it meets
/// goes back to the start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sentry {
    pub at: Pos,
    /// +1 marching down, -1 up.
    pub dir: i8,
}

/// An isle being played, key by key.
#[derive(Clone)]
pub struct Session {
    /// The campaign isle this is, or (for daily puzzles and drills) the
    /// isle whose keys it allows.
    pub index: usize,
    level: &'static Level,
    /// Bugs still crawling, and how many were squashed.
    pub bugs: Vec<Bug>,
    pub squashed: usize,
    /// Commands the bugs have crawled for.
    pub crawled: usize,
    /// Crystals that fade: where, and how many commands each has left.
    pub fading: Vec<(Pos, usize)>,
    pub sentries: Vec<Sentry>,
    /// Times a sentry caught the player this run.
    pub caught: usize,
    /// Free play: every key, no lesson to use, no budget.
    pub free: bool,
    /// The texts of the frozen lines, which no edit may change.
    frozen: Vec<String>,
    pub editor: Editor,
    pub gems: Vec<(Pos, bool)>,
    pub keys: Vec<Key>,
    pub complete: bool,
    pub message: String,
    allowed: String,
    /// Where the `:` that opened the command line is in `keys`.
    cmd_start: Option<usize>,
    /// Tiles that throw the cursor back when it lands on them (it may jump
    /// over them).
    pub traps: Vec<Pos>,
    /// Times a trap threw the player back this run.
    pub trapped: usize,
    pub coach: Coach,
    /// The last tip from the coach, until the next key.
    pub tip: String,
    /// Every command completed, as typed (`3e`, `"add`, `:%s/a/b/g`), to
    /// check the isle's lesson was used.
    pub used: Vec<String>,
    /// Set when the goal was reached without the lesson's commands: what
    /// was missing. The isle is over, but not cleared.
    pub missed: Vec<String>,
}

/// The game's own `:` commands: always available, and free.
pub const GAME_COMMANDS: &[&str] = &[
    "q", "q!", "wq", "x", "levels", "menu", "e!", "restart", "r", "n", "next", "mute", "sound", "help", "h",
    "intro", "learn", "w",
];

/// How a key of an unlock list reads: `w`, or `Ctrl-a`.
pub fn key_name(c: char) -> String {
    match Key::from_raw(c) {
        Key::Char(c) => c.to_string(),
        Key::CtrlA => "Ctrl-a".into(),
        Key::CtrlX => "Ctrl-x".into(),
        Key::CtrlV => "Ctrl-v".into(),
        Key::CtrlR => "Ctrl-r".into(),
        Key::CtrlO => "Ctrl-o".into(),
        Key::CtrlG => "Ctrl-g".into(),
        Key::CtrlD => "Ctrl-d".into(),
        Key::Tab => "Tab".into(),
        other => other.to_string(),
    }
}

/// Whether `cmd` is one of the game's own `:` commands.
pub fn is_game_command(cmd: &str) -> bool {
    GAME_COMMANDS.contains(&cmd.trim())
}

/// The stars a run of `keys` keystrokes earns on an isle of par `par`.
pub fn stars(keys: usize, par: usize) -> u8 {
    if keys <= par {
        3
    } else if keys <= par + par.div_ceil(2) {
        2
    } else {
        1
    }
}

impl Session {
    pub fn new(index: usize) -> Self {
        Self::custom(&LEVELS[index], index)
    }

    /// A level from outside the campaign (a daily puzzle, a drill), with
    /// the keys of campaign isle `index` and before.
    pub fn custom(level: &'static Level, index: usize) -> Self {
        let mut editor = Editor::new(level.text()).with_cursor(level.start());
        editor.files = level
            .files()
            .iter()
            .map(|(n, t)| (n.to_string(), t.to_string()))
            .collect();
        let mut session = Session {
            index,
            level,
            bugs: level.bugs(),
            squashed: 0,
            crawled: 0,
            fading: level.fading(),
            sentries: level.sentries(),
            caught: 0,
            free: false,
            frozen: level
                .frozen()
                .into_iter()
                .filter_map(|r| level.text().lines().nth(r).map(String::from))
                .collect(),
            editor,
            gems: level.gems().into_iter().map(|p| (p, false)).collect(),
            keys: Vec::new(),
            complete: false,
            message: String::new(),
            allowed: unlocked(index),
            cmd_start: None,
            traps: level.traps(),
            trapped: 0,
            coach: Coach::default(),
            tip: String::new(),
            used: Vec::new(),
            missed: Vec::new(),
        };
        session.collect();
        session
    }

    /// Free play of campaign isle `index`: every key is there, and the
    /// isle clears on reaching the goal, whatever was used.
    pub fn free_play(index: usize) -> Self {
        let mut s = Self::new(index);
        s.free = true;
        s.allowed = unlocked(LEVELS.len() - 1);
        s
    }

    /// Whether row `row` holds a frozen line.
    pub fn is_frozen(&self, row: usize) -> bool {
        let line: String = self.editor.lines.get(row).map_or_default(|l| l.iter().collect());
        self.frozen.contains(&line)
    }

    /// Whether every frozen line is still there, unchanged.
    fn frozen_intact(&self) -> bool {
        let text = self.editor.text();
        let lines: Vec<&str> = text.split('\n').collect();
        self.frozen.iter().all(|f| {
            let want = self.frozen.iter().filter(|g| *g == f).count();
            lines.iter().filter(|l| **l == f.as_str()).count() >= want
        })
    }

    pub fn level(&self) -> &'static Level {
        self.level
    }

    /// Whether this is a campaign isle (not a daily puzzle or a drill).
    pub fn in_campaign(&self) -> bool {
        std::ptr::eq(self.level, &raw const LEVELS[self.index])
    }

    pub fn par(&self) -> usize {
        self.level().par()
    }

    /// The keys learned so far, as an unlock list.
    pub fn allowed(&self) -> &str {
        &self.allowed
    }

    pub fn stars(&self) -> u8 {
        stars(self.keys.len(), self.par())
    }

    pub fn gems_left(&self) -> usize {
        self.gems.iter().filter(|(_, got)| !got).count()
    }

    /// Commands left before the crystal at `p` fades, if it fades.
    pub fn fades_in(&self, p: Pos) -> Option<usize> {
        self.fading.iter().find(|(f, _)| *f == p).map(|(_, n)| *n)
    }

    /// Whether the buffer still differs from the goal.
    pub fn goal_met(&self) -> bool {
        self.editor.buffer_name == crate::vim::ISLE
            && self.level().goal_text().is_none_or(|g| self.editor.text() == g)
            && self
                .level()
                .want_files()
                .iter()
                .all(|(n, t)| self.editor.files.get(*n).is_some_and(|f| f == t))
    }

    /// The isle's required commands not used yet.
    pub fn lesson_left(&self) -> Vec<&'static str> {
        crate::lessons::missing(self.level().title, &self.used)
    }

    /// Whether the isle was cleared: the goal reached, with its lesson.
    pub fn cleared(&self) -> bool {
        self.complete && self.missed.is_empty()
    }

    fn check_complete(&mut self) -> bool {
        if self.gems_left() == 0
            && self.bugs.is_empty()
            && self.goal_met()
            && self.editor.mode == Mode::Normal
            && self.editor.pending().is_empty()
        {
            self.complete = true;
            self.missed = if self.free {
                Vec::new()
            } else {
                self.lesson_left().into_iter().map(String::from).collect()
            };
        }
        self.complete
    }
}
