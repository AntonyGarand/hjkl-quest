//! A level being played: the editor, the crystals, the keystroke count and
//! which keys the player has learned. No UI here either.

use crate::coach::Coach;
use crate::levels::{LEVELS, Level, unlocked};
use crate::vim::{Editor, Key, Mode, Outcome, Pos};

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

pub fn is_game_command(cmd: &str) -> bool {
    GAME_COMMANDS.contains(&cmd.trim())
}

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

    fn is_allowed(&self, key: Key) -> Result<(), char> {
        let mode = self.editor.mode;
        let normalish = matches!(mode, Mode::Normal | Mode::Visual { .. } | Mode::Block);
        if !normalish || self.editor.awaiting_char() {
            return Ok(());
        }
        let pending = self.editor.pending();
        match key {
            Key::Esc => Ok(()),
            // Undo and redo are always there, so a wrong edit never strands
            // the player before the isle that teaches them properly.
            Key::CtrlR => Ok(()),
            Key::Char('u') if mode == Mode::Normal && pending.is_empty() => Ok(()),
            Key::Enter | Key::Backspace => Err('⏎'),
            // The command line always opens, for the game's own commands;
            // real ex commands wait until `:` is learned.
            Key::Char(':') if mode == Mode::Normal => Ok(()),
            // A macro being recorded can always be stopped.
            Key::Char('q') if self.editor.recording_macro().is_some() && pending.is_empty() => Ok(()),
            Key::Char(c) => {
                if c == '0' && self.editor.in_count() {
                    return Ok(());
                }
                // After an operator, i and a pick text objects, the
                // operator's own key repeats it (dd) and g starts gg.
                let op = pending.trim_end_matches(|c: char| c.is_ascii_digit());
                let after_operator = op.ends_with(['d', 'c', 'y', '>', '<'])
                    || op.ends_with("gu")
                    || op.ends_with("gU")
                    || op.ends_with("g~");
                if after_operator && (c == 'i' || c == 'a') {
                    return Ok(());
                }
                if self.editor.in_visual()
                    && matches!(
                        c,
                        'i' | 'a' | 'o' | 'd' | 'x' | 'y' | 'c' | 's' | 'u' | 'U' | '~' | 'J'
                    )
                {
                    return Ok(());
                }
                if self.allowed.contains(c) { Ok(()) } else { Err(c) }
            }
            other => {
                if self.allowed.contains(other.raw()) {
                    Ok(())
                } else {
                    Err(other.raw())
                }
            }
        }
    }

    /// Takes one key from the player.
    pub fn press(&mut self, key: Key) -> Event {
        if self.complete {
            return Event::None;
        }
        let normalish = matches!(self.editor.mode, Mode::Normal | Mode::Visual { .. } | Mode::Block);
        if normalish
            && !self.editor.awaiting_char()
            && let Key::Char(c) = key
            && self.level().banned().contains(c)
            && !(c == '0' && self.editor.in_count())
        {
            self.message = format!("`{c}` is banned on this isle: find another way");
            return Event::Locked(c);
        }
        if let Err(c) = self.is_allowed(key) {
            self.message = if c == '⏎' {
                "That key does nothing in normal mode".into()
            } else {
                format!("You haven't learned `{}` yet", key_name(c))
            };
            return Event::Locked(c);
        }
        let opens_cmdline = key == Key::Char(':')
            && (self.editor.mode == Mode::Normal || self.editor.in_visual())
            && self.editor.pending().is_empty();
        let in_cmdline = self.editor.mode == (Mode::Command { search: false });
        if opens_cmdline {
            self.cmd_start = Some(self.keys.len());
        }
        let ex_learned = self.allowed.contains(':');
        // Before ex commands are learned, only the game's own run.
        if in_cmdline && key == Key::Enter && !ex_learned && !is_game_command(&self.editor.cmdline) {
            let cmd = self.editor.cmdline.clone();
            self.editor.feed(Key::Esc);
            self.drop_cmdline_keys();
            self.message = format!(
                ":{cmd} needs ex commands, learned on isle {}. Try :e! :q :n :h :mute",
                crate::levels::ex_isle()
            );
            return Event::Locked(':');
        }
        self.keys.push(key);
        let revision = self.editor.revision;
        let cursor = self.editor.cursor;
        let (before_mode, before_pending) = (self.editor.mode, self.editor.pending());
        // A command line ends with Enter: remember it whole.
        let line = (matches!(before_mode, Mode::Command { .. }) && key == Key::Enter)
            .then(|| format!("{}{}", self.editor.cmd_prefix(), self.editor.cmdline));
        let before = (!self.frozen.is_empty()).then(|| self.editor.clone());
        let outcome = self.editor.feed(key);
        if let Some(before) = before
            && !self.frozen_intact()
        {
            // The ice holds: the edit didn't happen.
            self.editor = before;
            self.message = "❄ That line is frozen: change the others".into();
            return Event::Bell;
        }
        match line {
            Some(line) => self.used.push(line),
            None if matches!(before_mode, Mode::Normal | Mode::Visual { .. } | Mode::Block)
                && self.editor.pending().is_empty()
                && !matches!(outcome, Outcome::Bell(_)) =>
            {
                self.used.push(format!("{before_pending}{key}"));
            }
            None => {}
        }
        self.message.clear();
        self.tip.clear();
        // Landing on a trap throws the wisp back where it came from; the
        // keys still count.
        if !matches!(
            self.editor.mode,
            Mode::Command { .. } | Mode::Insert | Mode::Replace
        ) && self.editor.pending().is_empty()
            && self.editor.cursor != cursor
            && self.traps.contains(&self.editor.cursor)
        {
            let at = self.editor.cursor;
            self.editor.set_cursor(cursor);
            self.trapped += 1;
            self.message = "Ouch, a trap! It threw you back: jump over it in one move".into();
            return Event::Trapped(at);
        }
        // A finished normal-mode command goes to the coach.
        if before_mode == Mode::Normal && self.editor.pending().is_empty() && matches!(outcome, Outcome::Ok) {
            let token = format!("{before_pending}{key}");
            if let Some(tip) = self.coach.push(token, &self.allowed) {
                self.tip = tip;
            }
        }
        let left_cmdline = in_cmdline && self.editor.mode != (Mode::Command { search: false });
        let mut event = match outcome {
            Outcome::Ok => {
                if left_cmdline && !ex_learned {
                    // An abandoned command line costs nothing either.
                    self.drop_cmdline_keys();
                }
                if self.editor.revision != revision {
                    Event::Edited
                } else if self.editor.cursor != cursor {
                    Event::Moved
                } else {
                    Event::None
                }
            }
            Outcome::Bell(msg) => {
                self.message = msg;
                Event::Bell
            }
            Outcome::Info(msg) => {
                self.message = msg;
                if self.editor.revision != revision {
                    Event::Edited
                } else if self.editor.cursor != cursor {
                    Event::Moved
                } else {
                    Event::None
                }
            }
            Outcome::Command(cmd) => {
                // The game's own commands: none of their keys count.
                self.drop_cmdline_keys();
                return Event::Command(cmd);
            }
        };
        if let Some(gem) = self.collect() {
            event = Event::Gem(gem);
        }
        if let Some(at) = self.crawl() {
            event = Event::Squashed(at);
        }
        if let Some(at) = self.march() {
            event = Event::Caught(at);
        }
        if self.fade() {
            return Event::Complete;
        }
        if self.check_complete() {
            event = Event::Complete;
        } else if let Some(budget) = self.level().budget()
            && !self.free
            && self.keys.len() >= budget
        {
            // Out of keystrokes: the isle ends, not cleared.
            self.complete = true;
            self.missed = vec![format!("finish within {budget} keys")];
            event = Event::Complete;
        }
        event
    }

    /// A command was completed: a bug the cursor stands on is squashed;
    /// the others crawl a tile, and every `HATCH_EVERY` commands each hatches
    /// a twin.
    fn crawl(&mut self) -> Option<Pos> {
        // Typing a command line or text is one command, done when it ends.
        let typing = matches!(
            self.editor.mode,
            Mode::Command { .. } | Mode::Insert | Mode::Replace | Mode::Confirm
        );
        if self.bugs.is_empty() || !self.editor.pending().is_empty() || typing {
            return None;
        }
        let at = self.editor.cursor;
        let squash = |bugs: &mut Vec<Bug>| {
            let before = bugs.len();
            bugs.retain(|b| b.at != at);
            before - bugs.len()
        };
        let mut caught = squash(&mut self.bugs);
        let lines = &self.editor.lines;
        let traps = &self.traps;
        for bug in &mut self.bugs {
            let len = lines.get(bug.at.row).map_or(0, Vec::len);
            if len == 0 {
                continue;
            }
            bug.at.col = bug.at.col.min(len - 1);
            let open = |col: isize| {
                col >= 0
                    && (col as usize) < len
                    && !traps.contains(&Pos {
                        row: bug.at.row,
                        col: col as usize,
                    })
            };
            let ahead = bug.at.col as isize + bug.dir as isize;
            if !open(ahead) {
                bug.dir = -bug.dir;
            }
            let ahead = bug.at.col as isize + bug.dir as isize;
            if open(ahead) {
                bug.at.col = ahead as usize;
            }
        }
        caught += squash(&mut self.bugs);
        self.crawled += 1;
        if self.crawled.is_multiple_of(HATCH_EVERY) {
            let twins: Vec<Bug> = self
                .bugs
                .iter()
                .map(|b| Bug {
                    at: b.at,
                    dir: -b.dir,
                })
                .collect();
            for twin in twins {
                if self.bugs.len() < MAX_BUGS {
                    self.bugs.push(twin);
                }
            }
            if !self.bugs.is_empty() {
                self.message = "The bugs hatched! Catch them faster".into();
            }
        }
        self.squashed += caught;
        (caught > 0).then_some(at)
    }

    /// Whether a whole command was just done (not a half-typed one, and
    /// not in the middle of typing text or a command line).
    fn command_done(&self) -> bool {
        let typing = matches!(
            self.editor.mode,
            Mode::Command { .. } | Mode::Insert | Mode::Replace | Mode::Confirm
        );
        self.editor.pending().is_empty() && !typing
    }

    /// A command was done: the sentries march a row. One that meets the
    /// cursor (or that the cursor walked into) sends it back to the start.
    fn march(&mut self) -> Option<Pos> {
        if self.sentries.is_empty() || !self.command_done() {
            return None;
        }
        let at = self.editor.cursor;
        let walked_in = self.sentries.iter().any(|s| s.at == at);
        let lines = &self.editor.lines;
        for s in &mut self.sentries {
            let open =
                |row: isize| row >= 0 && (row as usize) < lines.len() && s.at.col < lines[row as usize].len();
            if !open(s.at.row as isize + s.dir as isize) {
                s.dir = -s.dir;
            }
            let next = s.at.row as isize + s.dir as isize;
            if open(next) {
                s.at.row = next as usize;
            }
        }
        if !walked_in && !self.sentries.iter().any(|s| s.at == at) {
            return None;
        }
        self.caught += 1;
        self.editor.set_cursor(self.level().start());
        self.message = "A sentry caught you! Back to the start: time your moves".into();
        Some(at)
    }

    /// A command was done: the fading crystals dim. One that fades out
    /// ends the isle, not cleared. Returns whether that happened.
    fn fade(&mut self) -> bool {
        if self.fading.is_empty() || !self.command_done() || self.complete {
            return false;
        }
        let gems = &self.gems;
        self.fading
            .retain(|(p, _)| gems.iter().any(|(g, got)| g == p && !got));
        for (_, left) in &mut self.fading {
            *left = left.saturating_sub(1);
        }
        if self.fading.iter().any(|(_, left)| *left == 0) {
            self.complete = true;
            self.missed = vec!["a crystal faded: reach them sooner, with bigger jumps".into()];
            return true;
        }
        false
    }

    fn drop_cmdline_keys(&mut self) {
        if let Some(start) = self.cmd_start.take() {
            self.keys.truncate(start);
        }
    }

    fn collect(&mut self) -> Option<Pos> {
        if !self.editor.pending().is_empty() {
            return None;
        }
        let at = self.editor.cursor;
        let gem = self.gems.iter_mut().find(|(p, got)| *p == at && !got)?;
        gem.1 = true;
        Some(at)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vim::{format_keys, parse_keys};

    #[test]
    fn every_solution_solves_its_level_at_par() {
        for (i, level) in LEVELS.iter().enumerate() {
            let mut s = Session::new(i);
            assert!(!s.complete, "level {i} {} starts complete", level.title);
            let keys = parse_keys(level.solution);
            for (k, &key) in keys.iter().enumerate() {
                assert!(
                    !s.complete,
                    "level {i} {} completes early after {}",
                    level.title,
                    format_keys(&keys[..k])
                );
                let event = s.press(key);
                assert!(
                    !matches!(event, Event::Locked(_) | Event::Bell | Event::Trapped(_)),
                    "level {i} {}: key {} ({k}) gave {event:?}: {}\nbuffer:\n{}",
                    level.title,
                    key,
                    s.message,
                    s.editor.text()
                );
            }
            assert!(
                s.complete,
                "level {i} {} not solved by {}: {} gems left, cursor {:?}, buffer:\n{}",
                level.title,
                level.solution,
                s.gems_left(),
                s.editor.cursor,
                s.editor.text()
            );
            assert!(
                s.missed.is_empty(),
                "level {i} {}: the golf line skips the lesson: {:?} (used {:?})",
                level.title,
                s.missed,
                s.used
            );
            assert_eq!(s.keys.len(), level.par());
            assert_eq!(s.stars(), 3);
        }
    }

    #[test]
    fn marks_lie_on_the_map() {
        for level in LEVELS {
            let e = Editor::new(level.text());
            for p in level
                .gems()
                .into_iter()
                .chain(level.traps())
                .chain([level.start()])
            {
                assert!(
                    p.row < e.lines.len() && (p.col < e.line_len(p.row) || p.col == 0),
                    "{}: mark {p:?} is off the map",
                    level.title
                );
            }
        }
    }

    #[test]
    fn locked_keys_do_not_count() {
        let mut s = Session::new(0);
        assert_eq!(s.press(Key::Char('w')), Event::Locked('w'));
        assert!(s.keys.is_empty());
        assert_eq!(s.press(Key::Char('l')), Event::Moved);
        assert_eq!(s.keys.len(), 1);
    }

    #[test]
    fn game_commands_work_before_ex_is_learned_and_cost_nothing() {
        let mut s = Session::new(0);
        s.press(Key::Char('l'));
        let mut last = Event::None;
        for k in parse_keys(":e!<CR>") {
            last = s.press(k);
        }
        assert_eq!(last, Event::Command("e!".into()));
        assert_eq!(s.keys.len(), 1);
        // A real ex command is refused, and costs nothing.
        for k in parse_keys(":s/a/b/<CR>") {
            last = s.press(k);
        }
        assert_eq!(last, Event::Locked(':'));
        assert_eq!(s.keys.len(), 1);
        assert_eq!(s.editor.mode, Mode::Normal);
        // Once learned, ex keys count as in vim golf.
        let mut s = Session::new(crate::levels::ex_isle() - 1);
        for k in parse_keys(":%s/foo/baz/g<CR>") {
            s.press(k);
        }
        assert!(s.complete);
        assert_eq!(s.keys.len(), 14);
    }

    #[test]
    fn a_wrong_edit_can_always_be_undone() {
        // Chomp comes before undo's own isle.
        let mut s = Session::new(LEVELS.iter().position(|l| l.title == "Chomp").unwrap());
        let before = s.editor.text();
        assert_eq!(s.press(Key::Char('x')), Event::Edited);
        assert_ne!(s.editor.text(), before);
        assert_eq!(s.press(Key::Char('u')), Event::Edited);
        assert_eq!(s.editor.text(), before);
        assert_eq!(s.press(Key::CtrlR), Event::Edited);
        assert_ne!(s.editor.text(), before);
    }

    #[test]
    fn traps_throw_back_single_steps_but_not_jumps() {
        // Counting Sheep: traps every four tiles along the first row.
        let mut s = Session::new(1);
        let mut events = Vec::new();
        for _ in 0..5 {
            events.push(s.press(Key::Char('l')));
        }
        assert_eq!(events[3], Event::Trapped(Pos { row: 0, col: 4 }));
        assert_eq!(events[4], Event::Trapped(Pos { row: 0, col: 4 }));
        assert_eq!(s.editor.cursor, Pos { row: 0, col: 3 });
        assert_eq!(s.trapped, 2);
        // The keys still count.
        assert_eq!(s.keys.len(), 5);
        // A count jumps over the traps.
        let mut s = Session::new(1);
        for k in parse_keys("15l") {
            assert!(!matches!(s.press(k), Event::Trapped(_)));
        }
        assert_eq!(s.editor.cursor, Pos { row: 0, col: 15 });
        for level in LEVELS {
            for t in level.traps() {
                assert!(
                    !level.gems().contains(&t) && level.start() != t,
                    "{}: trap on a mark",
                    level.title
                );
            }
        }
    }

    #[test]
    fn the_coach_spots_slow_habits() {
        let mut s = Session::new(2); // Word Hopping: counts, w and e known.
        for k in parse_keys("hhhh") {
            s.press(k);
        }
        assert!(s.tip.contains("4h"), "{}", s.tip);
        assert_eq!(s.coach.spotted.len(), 1);
    }

    #[test]
    fn reaching_the_goal_without_the_lesson_fails_the_isle() {
        // Search Party: three searches instead of / then n.
        let i = LEVELS.iter().position(|l| l.title == "Search Party").unwrap();
        let mut s = Session::new(i);
        assert_eq!(s.lesson_left(), vec!["/text", "n or N"]);
        let mut last = Event::None;
        for k in parse_keys("/g<CR>/g<CR>/g<CR>") {
            last = s.press(k);
        }
        assert_eq!(last, Event::Complete);
        assert!(s.complete && !s.cleared());
        assert_eq!(s.missed, vec!["n or N".to_string()]);
        // The golf line clears it.
        let mut s = Session::new(i);
        for k in parse_keys(LEVELS[i].solution) {
            s.press(k);
        }
        assert!(s.cleared());
    }

    #[test]
    fn a_keystroke_budget_ends_the_isle() {
        let i = LEVELS
            .iter()
            .position(|l| l.title == "Edge of the World")
            .unwrap();
        let mut s = Session::new(i);
        let mut last = Event::None;
        for k in 0..10 {
            last = s.press(Key::Char(if k % 2 == 0 { 'l' } else { 'h' }));
        }
        assert_eq!(last, Event::Complete);
        assert!(s.complete && !s.cleared());
        assert_eq!(s.missed, vec!["finish within 10 keys".to_string()]);
    }

    fn isle(title: &str) -> usize {
        LEVELS.iter().position(|l| l.title == title).unwrap()
    }

    #[test]
    fn crystals_fade_and_sentries_catch() {
        // Stepping is too slow for a fading crystal.
        let mut s = Session::new(isle("Fading Lights"));
        for k in parse_keys("ll") {
            s.press(k);
        }
        assert!(s.complete && !s.cleared(), "{:?}", s.missed);
        assert!(s.missed[0].contains("faded"));
        // Standing still where a sentry is going sends the wisp home.
        let mut s = Session::new(isle("Night Watch"));
        let mut last = Event::None;
        for k in parse_keys("3j6l7|") {
            last = s.press(k);
        }
        assert_eq!(last, Event::Caught(Pos { row: 3, col: 6 }));
        assert_eq!(
            (s.caught, s.editor.cursor),
            (1, LEVELS[isle("Night Watch")].start())
        );
    }

    #[test]
    fn banned_keys_and_frozen_lines_hold() {
        let mut s = Session::new(isle("No Arrows"));
        assert_eq!(s.press(Key::Char('l')), Event::Locked('l'));
        assert_eq!(s.editor.cursor, Pos { row: 0, col: 0 });
        assert!(s.keys.is_empty());
        let mut s = Session::new(isle("Thin Ice"));
        let before = s.editor.text();
        let mut last = Event::None;
        for k in parse_keys(":%s/teh/the/g<CR>") {
            last = s.press(k);
        }
        assert_eq!(last, Event::Bell);
        assert_eq!(s.editor.text(), before);
        assert!(s.message.contains("frozen"));
    }

    #[test]
    fn free_play_allows_everything_and_asks_for_nothing() {
        let i = isle("Search Party");
        let mut s = Session::free_play(i);
        for k in parse_keys("/g<CR>/g<CR>/g<CR>") {
            s.press(k);
        }
        assert!(s.cleared(), "{:?}", s.missed);
        // Keys from much later isles work.
        let mut s = Session::free_play(0);
        assert_ne!(s.press(Key::Char('w')), Event::Locked('w'));
    }

    #[test]
    fn star_thresholds() {
        assert_eq!(stars(10, 10), 3);
        assert_eq!(stars(15, 10), 2);
        assert_eq!(stars(16, 10), 1);
    }
}
