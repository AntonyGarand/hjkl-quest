//! A small, self-contained vim emulator: the buffer, the cursor, the modes
//! and the commands the campaign teaches. No UI here, so it is tested with
//! plain `cargo test`.

use std::fmt;

/// One keystroke given to the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Esc,
    Enter,
    Backspace,
    CtrlR,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Char(' ') => write!(f, "␣"),
            Key::Char(c) => write!(f, "{c}"),
            Key::Esc => write!(f, "<Esc>"),
            Key::Enter => write!(f, "<CR>"),
            Key::Backspace => write!(f, "<BS>"),
            Key::CtrlR => write!(f, "<C-r>"),
        }
    }
}

/// Parses vim-golf notation: plain characters, plus `<Esc>`, `<CR>`, `<BS>`,
/// `<C-r>` and `<Space>`.
pub fn parse_keys(s: &str) -> Vec<Key> {
    let mut keys = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<'
            && let Some(end) = chars[i..].iter().position(|&c| c == '>') {
                let name: String = chars[i + 1..i + end].iter().collect();
                let key = match name.to_ascii_lowercase().as_str() {
                    "esc" => Some(Key::Esc),
                    "cr" | "enter" => Some(Key::Enter),
                    "bs" => Some(Key::Backspace),
                    "c-r" => Some(Key::CtrlR),
                    "space" => Some(Key::Char(' ')),
                    "lt" => Some(Key::Char('<')),
                    _ => None,
                };
                if let Some(key) = key {
                    keys.push(key);
                    i += end + 1;
                    continue;
                }
            }
        keys.push(Key::Char(chars[i]));
        i += 1;
    }
    keys
}

pub fn format_keys(keys: &[Key]) -> String {
    keys.iter().map(|k| k.to_string()).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Pos {
    pub row: usize,
    pub col: usize,
}

pub const fn pos(row: usize, col: usize) -> Pos {
    Pos { row, col }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual { line: bool },
    Command { search: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The key was taken.
    Ok,
    /// The key did nothing useful (a bell in vim).
    Bell(String),
    /// A `:` command the editor does not know: for the game to handle.
    Command(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Register {
    pub text: String,
    pub linewise: bool,
}

#[derive(Debug, Clone)]
struct Snapshot {
    lines: Vec<Vec<char>>,
    cursor: Pos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MotionKind {
    Exclusive,
    Inclusive,
    Linewise,
}

#[derive(Debug, Clone, Copy)]
struct Motion {
    to: Pos,
    kind: MotionKind,
    /// Whether the motion sets the remembered column (all but j/k).
    keeps_want: bool,
    want_end: bool,
}

/// A charwise range [start, end) or a linewise range of rows.
#[derive(Debug, Clone, Copy)]
enum Span {
    Chars(Pos, Pos),
    Lines(usize, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Parse {
    Incomplete,
    Invalid,
}

#[derive(Debug, Clone)]
pub struct Editor {
    pub lines: Vec<Vec<char>>,
    pub cursor: Pos,
    pub mode: Mode,
    /// The anchor of a visual selection.
    pub anchor: Pos,
    pub cmdline: String,
    pub register: Register,
    pending: Vec<Key>,
    want: usize,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    last_find: Option<(char, char)>,
    pub last_search: Option<String>,
    last_change: Vec<Key>,
    recording: Option<Vec<Key>>,
    replaying: bool,
    /// The number of buffer changes so far (for the UI to redraw).
    pub revision: u64,
}

fn class(c: char, big: bool) -> u8 {
    if c.is_whitespace() {
        0
    } else if big || c.is_alphanumeric() || c == '_' {
        2
    } else {
        1
    }
}

fn matching(c: char) -> Option<(char, char, bool)> {
    match c {
        '(' => Some(('(', ')', true)),
        ')' => Some(('(', ')', false)),
        '[' => Some(('[', ']', true)),
        ']' => Some(('[', ']', false)),
        '{' => Some(('{', '}', true)),
        '}' => Some(('{', '}', false)),
        _ => None,
    }
}

impl Editor {
    pub fn new(text: &str) -> Self {
        let mut lines: Vec<Vec<char>> = text.split('\n').map(|l| l.chars().collect()).collect();
        if lines.is_empty() {
            lines.push(Vec::new());
        }
        Editor {
            lines,
            cursor: Pos::default(),
            mode: Mode::Normal,
            anchor: Pos::default(),
            cmdline: String::new(),
            register: Register::default(),
            pending: Vec::new(),
            want: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            last_find: None,
            last_search: None,
            last_change: Vec::new(),
            recording: None,
            replaying: false,
            revision: 0,
        }
    }

    pub fn with_cursor(mut self, at: Pos) -> Self {
        self.cursor = at;
        self.clamp();
        self.want = self.cursor.col;
        self
    }

    pub fn text(&self) -> String {
        self.lines.iter().map(|l| l.iter().collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    /// The keys of the command being typed (vim's `showcmd`).
    pub fn pending(&self) -> String {
        format_keys(&self.pending)
    }

    /// Whether the next key is a character argument (after `f`, `t`, `r`, ...).
    pub fn awaiting_char(&self) -> bool {
        let keys = self.pending_without_count();
        matches!(
            keys.last(),
            Some(Key::Char('f' | 'F' | 't' | 'T' | 'r'))
        ) && !(keys.len() >= 2 && matches!(keys[keys.len() - 2], Key::Char('f' | 'F' | 't' | 'T' | 'r')))
            || matches!(keys.last(), Some(Key::Char('i' | 'a'))) && keys.len() >= 2 && matches!(keys[0], Key::Char('d' | 'c' | 'y'))
    }

    /// Whether the pending keys end with a count digit.
    pub fn in_count(&self) -> bool {
        matches!(self.pending.last(), Some(Key::Char('1'..='9')))
            || (matches!(self.pending.last(), Some(Key::Char('0'))) && self.pending.len() > 1 && {
                let before = &self.pending[..self.pending.len() - 1];
                matches!(before.last(), Some(Key::Char('0'..='9')))
            })
    }

    fn pending_without_count(&self) -> Vec<Key> {
        self.pending.iter().copied().filter(|k| !matches!(k, Key::Char('0'..='9'))).collect()
    }

    pub fn line_len(&self, row: usize) -> usize {
        self.lines.get(row).map_or(0, |l| l.len())
    }

    pub fn char_at(&self, p: Pos) -> Option<char> {
        self.lines.get(p.row).and_then(|l| l.get(p.col)).copied()
    }

    fn last_col(&self, row: usize) -> usize {
        self.line_len(row).saturating_sub(1)
    }

    fn clamp(&mut self) {
        let last_row = self.lines.len() - 1;
        self.cursor.row = self.cursor.row.min(last_row);
        let max = match self.mode {
            Mode::Insert => self.line_len(self.cursor.row),
            _ => self.last_col(self.cursor.row),
        };
        self.cursor.col = self.cursor.col.min(max);
    }

    fn first_non_blank(&self, row: usize) -> usize {
        let line = &self.lines[row];
        line.iter().position(|c| !c.is_whitespace()).unwrap_or(line.len().saturating_sub(1))
    }

    fn snapshot(&mut self) {
        if self.replaying {
            return;
        }
        self.undo.push(Snapshot { lines: self.lines.clone(), cursor: self.cursor });
        self.redo.clear();
    }

    fn changed(&mut self) {
        self.revision += 1;
    }

    // ----------------------------------------------------------------- keys

    /// Takes one key.
    pub fn feed(&mut self, key: Key) -> Outcome {
        if let Some(rec) = &mut self.recording {
            rec.push(key);
        }
        let outcome = match self.mode {
            Mode::Insert => self.insert_key(key),
            Mode::Command { search } => self.command_key(key, search),
            Mode::Normal | Mode::Visual { .. } => self.normal_key(key),
        };
        // A change ends when we are back in normal mode with nothing pending.
        if self.mode == Mode::Normal && self.pending.is_empty()
            && let Some(rec) = self.recording.take()
                && !self.replaying {
                    self.last_change = rec;
                }
        outcome
    }

    pub fn feed_all(&mut self, keys: &[Key]) {
        for &k in keys {
            self.feed(k);
        }
    }

    fn insert_key(&mut self, key: Key) -> Outcome {
        match key {
            Key::Esc => {
                self.mode = Mode::Normal;
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                }
                self.clamp();
                self.want = self.cursor.col;
            }
            Key::Enter => {
                let row = self.cursor.row;
                let rest: Vec<char> = self.lines[row].split_off(self.cursor.col);
                self.lines.insert(row + 1, rest);
                self.cursor = pos(row + 1, 0);
                self.changed();
            }
            Key::Backspace => {
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                    self.lines[self.cursor.row].remove(self.cursor.col);
                    self.changed();
                } else if self.cursor.row > 0 {
                    let line = self.lines.remove(self.cursor.row);
                    self.cursor.row -= 1;
                    self.cursor.col = self.line_len(self.cursor.row);
                    self.lines[self.cursor.row].extend(line);
                    self.changed();
                }
            }
            Key::Char(c) => {
                self.lines[self.cursor.row].insert(self.cursor.col, c);
                self.cursor.col += 1;
                self.changed();
            }
            Key::CtrlR => return Outcome::Bell("<C-r> does nothing in insert mode here".into()),
        }
        Outcome::Ok
    }

    fn command_key(&mut self, key: Key, search: bool) -> Outcome {
        match key {
            Key::Esc => {
                self.mode = Mode::Normal;
                self.cmdline.clear();
                Outcome::Ok
            }
            Key::Backspace => {
                if self.cmdline.pop().is_none() {
                    self.mode = Mode::Normal;
                }
                Outcome::Ok
            }
            Key::Char(c) => {
                self.cmdline.push(c);
                Outcome::Ok
            }
            Key::CtrlR => Outcome::Ok,
            Key::Enter => {
                self.mode = Mode::Normal;
                let line = std::mem::take(&mut self.cmdline);
                if search {
                    if line.is_empty() && self.last_search.is_none() {
                        return Outcome::Bell("No previous search".into());
                    }
                    if !line.is_empty() {
                        self.last_search = Some(line);
                    }
                    match self.search_from(self.cursor, true, false) {
                        Some(p) => {
                            self.cursor = p;
                            self.want = p.col;
                            Outcome::Ok
                        }
                        None => Outcome::Bell("Pattern not found".into()),
                    }
                } else {
                    self.ex_command(&line)
                }
            }
        }
    }

    fn ex_command(&mut self, line: &str) -> Outcome {
        let cmd = line.trim();
        if let Ok(n) = cmd.parse::<usize>() {
            let row = n.max(1).min(self.lines.len()) - 1;
            self.cursor = pos(row, self.first_non_blank(row));
            self.want = self.cursor.col;
            return Outcome::Ok;
        }
        let (all, rest) = if let Some(rest) = cmd.strip_prefix('%') { (true, rest) } else { (false, cmd) };
        if let Some(body) = rest.strip_prefix("s/") {
            let parts: Vec<&str> = body.splitn(3, '/').collect();
            if parts.len() < 2 || parts[0].is_empty() {
                return Outcome::Bell("Usage: :s/old/new/g".into());
            }
            let (pat, rep) = (parts[0], parts[1]);
            let global = parts.get(2).is_some_and(|f| f.contains('g'));
            let rows: Vec<usize> = if all { (0..self.lines.len()).collect() } else { vec![self.cursor.row] };
            let before = self.lines.clone();
            let cursor = self.cursor;
            let mut last = None;
            for row in rows {
                let text: String = self.lines[row].iter().collect();
                if !text.contains(pat) {
                    continue;
                }
                let new = if global { text.replace(pat, rep) } else { text.replacen(pat, rep, 1) };
                self.lines[row] = new.chars().collect();
                last = Some(row);
            }
            return match last {
                Some(row) => {
                    if !self.replaying {
                        self.undo.push(Snapshot { lines: before, cursor });
                        self.redo.clear();
                    }
                    self.cursor = pos(row, self.first_non_blank(row));
                    self.changed();
                    Outcome::Ok
                }
                None => Outcome::Bell(format!("Pattern not found: {pat}")),
            };
        }
        Outcome::Command(cmd.to_string())
    }

    fn normal_key(&mut self, key: Key) -> Outcome {
        if key == Key::Esc {
            let had = !self.pending.is_empty();
            self.pending.clear();
            self.recording = None;
            if let Mode::Visual { .. } = self.mode {
                self.mode = Mode::Normal;
                self.clamp();
                return Outcome::Ok;
            }
            return if had { Outcome::Ok } else { Outcome::Bell(String::new()) };
        }
        if self.pending.is_empty() && self.recording.is_none() && self.mode == Mode::Normal {
            self.recording = Some(vec![key]);
        }
        self.pending.push(key);
        let keys = self.pending.clone();
        let visual = matches!(self.mode, Mode::Visual { .. });
        let result = if visual { self.run_visual(&keys) } else { self.run_normal(&keys) };
        match result {
            Err(Parse::Incomplete) => Outcome::Ok,
            Err(Parse::Invalid) => {
                self.pending.clear();
                self.recording = None;
                Outcome::Bell(format!("Not a command: {}", format_keys(&keys)))
            }
            Ok(outcome) => {
                self.pending.clear();
                if self.mode == Mode::Normal {
                    self.clamp();
                }
                outcome
            }
        }
    }

    // --------------------------------------------------------------- parsing

    fn read_count(keys: &[Key], i: &mut usize) -> Option<usize> {
        let mut n: Option<usize> = None;
        while let Some(Key::Char(c)) = keys.get(*i) {
            if c.is_ascii_digit() && (*c != '0' || n.is_some()) {
                n = Some(n.unwrap_or(0).saturating_mul(10).saturating_add(c.to_digit(10).unwrap() as usize).min(9999));
                *i += 1;
            } else {
                break;
            }
        }
        n
    }

    /// Parses a motion at `keys[i..]`; `Ok(None)` when the key is no motion.
    fn parse_motion(&mut self, keys: &[Key], i: usize, count: Option<usize>) -> Result<Option<(Motion, usize)>, Parse> {
        let Some(&Key::Char(c)) = keys.get(i) else {
            return if i >= keys.len() { Err(Parse::Incomplete) } else { Ok(None) };
        };
        let n = count.unwrap_or(1);
        let cur = self.cursor;
        let m = |to: Pos, kind: MotionKind| Motion { to, kind, keeps_want: true, want_end: false };
        let motion = match c {
            'h' => m(pos(cur.row, cur.col.saturating_sub(n)), MotionKind::Exclusive),
            'l' | ' ' => {
                let max = self.line_len(cur.row);
                m(pos(cur.row, (cur.col + n).min(max)), MotionKind::Exclusive)
            }
            'j' | 'k' => {
                let row = if c == 'j' {
                    if cur.row + 1 >= self.lines.len() {
                        return Err(Parse::Invalid);
                    }
                    (cur.row + n).min(self.lines.len() - 1)
                } else {
                    if cur.row == 0 {
                        return Err(Parse::Invalid);
                    }
                    cur.row.saturating_sub(n)
                };
                let col = self.want.min(self.last_col(row));
                Motion { to: pos(row, col), kind: MotionKind::Linewise, keeps_want: false, want_end: false }
            }
            '0' => m(pos(cur.row, 0), MotionKind::Exclusive),
            '^' => m(pos(cur.row, self.first_non_blank(cur.row)), MotionKind::Exclusive),
            '$' => {
                let row = (cur.row + n - 1).min(self.lines.len() - 1);
                Motion { to: pos(row, self.last_col(row)), kind: MotionKind::Inclusive, keeps_want: true, want_end: true }
            }
            'w' | 'W' => {
                let mut p = cur;
                for _ in 0..n {
                    p = self.word_forward(p, c == 'W');
                }
                m(p, MotionKind::Exclusive)
            }
            'b' | 'B' => {
                let mut p = cur;
                for _ in 0..n {
                    p = self.word_backward(p, c == 'B');
                }
                m(p, MotionKind::Exclusive)
            }
            'e' | 'E' => {
                let mut p = cur;
                for _ in 0..n {
                    p = self.word_end(p, c == 'E');
                }
                m(p, MotionKind::Inclusive)
            }
            'G' => {
                let row = count.map_or(self.lines.len() - 1, |n| n.max(1).min(self.lines.len()) - 1);
                Motion { to: pos(row, self.first_non_blank(row)), kind: MotionKind::Linewise, keeps_want: true, want_end: false }
            }
            'g' => {
                let Some(&next) = keys.get(i + 1) else { return Err(Parse::Incomplete) };
                match next {
                    Key::Char('g') => {
                        let row = count.map_or(0, |n| n.max(1).min(self.lines.len()) - 1);
                        return Ok(Some((
                            Motion { to: pos(row, self.first_non_blank(row)), kind: MotionKind::Linewise, keeps_want: true, want_end: false },
                            i + 2,
                        )));
                    }
                    Key::Char(e @ ('e' | 'E')) => {
                        let mut p = cur;
                        for _ in 0..n {
                            p = self.word_end_backward(p, e == 'E');
                        }
                        return Ok(Some((m(p, MotionKind::Inclusive), i + 2)));
                    }
                    _ => return Err(Parse::Invalid),
                }
            }
            'f' | 'F' | 't' | 'T' => {
                let Some(&target) = keys.get(i + 1) else { return Err(Parse::Incomplete) };
                let Key::Char(target) = target else { return Err(Parse::Invalid) };
                self.last_find = Some((c, target));
                let to = self.find(c, target, n, false).ok_or(Parse::Invalid)?;
                let kind = if c == 'f' || c == 't' { MotionKind::Inclusive } else { MotionKind::Exclusive };
                return Ok(Some((m(to, kind), i + 2)));
            }
            ';' | ',' => {
                let (kind, target) = self.last_find.ok_or(Parse::Invalid)?;
                let kind = if c == ',' {
                    match kind {
                        'f' => 'F',
                        'F' => 'f',
                        't' => 'T',
                        _ => 't',
                    }
                } else {
                    kind
                };
                let to = self.find(kind, target, n, true).ok_or(Parse::Invalid)?;
                let mk = if kind == 'f' || kind == 't' { MotionKind::Inclusive } else { MotionKind::Exclusive };
                m(to, mk)
            }
            '%' => m(self.match_pair(cur).ok_or(Parse::Invalid)?, MotionKind::Inclusive),
            '}' | '{' => {
                let mut row = cur.row;
                for _ in 0..n {
                    row = self.paragraph(row, c == '}');
                }
                let to = if c == '}' && row == self.lines.len() - 1 && !self.lines[row].is_empty() {
                    pos(row, self.last_col(row))
                } else if c == '{' && row == 0 && !self.lines[0].is_empty() {
                    pos(0, 0)
                } else {
                    pos(row, 0)
                };
                m(to, MotionKind::Exclusive)
            }
            'n' | 'N' => {
                if self.last_search.is_none() {
                    return Err(Parse::Invalid);
                }
                let mut p = cur;
                for _ in 0..n {
                    p = self.search_from(p, c == 'n', false).ok_or(Parse::Invalid)?;
                }
                m(p, MotionKind::Exclusive)
            }
            '*' | '#' => {
                let word = self.word_under(cur).ok_or(Parse::Invalid)?;
                self.last_search = Some(word);
                let mut p = cur;
                for _ in 0..n {
                    p = self.search_from(p, c == '*', true).ok_or(Parse::Invalid)?;
                }
                m(p, MotionKind::Exclusive)
            }
            _ => return Ok(None),
        };
        Ok(Some((motion, i + 1)))
    }

    fn apply_motion(&mut self, motion: Motion) {
        self.cursor = motion.to;
        if motion.keeps_want {
            self.want = if motion.want_end { usize::MAX } else { motion.to.col };
        }
        self.clamp();
    }

    fn run_normal(&mut self, keys: &[Key]) -> Result<Outcome, Parse> {
        let mut i = 0;
        let count = Self::read_count(keys, &mut i);
        let Some(&key) = keys.get(i) else { return Err(Parse::Incomplete) };
        let n = count.unwrap_or(1);
        if let Some((motion, end)) = self.parse_motion(keys, i, count)? {
            if end != keys.len() {
                return Err(Parse::Invalid);
            }
            self.recording = None;
            self.apply_motion(motion);
            return Ok(Outcome::Ok);
        }
        let Key::Char(c) = key else {
            if key == Key::CtrlR {
                self.recording = None;
                return Ok(self.redo_change(n));
            }
            return Err(Parse::Invalid);
        };
        if keys.len() > i + 1 && !matches!(c, 'd' | 'c' | 'y' | 'r') {
            return Err(Parse::Invalid);
        }
        match c {
            'd' | 'c' | 'y' => {
                let j = i + 1;
                let mut k = j;
                let count2 = Self::read_count(keys, &mut k);
                let Some(&next) = keys.get(k) else { return Err(Parse::Incomplete) };
                let total = match (count, count2) {
                    (None, None) => None,
                    (a, b) => Some(a.unwrap_or(1) * b.unwrap_or(1)),
                };
                let tn = total.unwrap_or(1);
                let span = if next == Key::Char(c) {
                    if k + 1 != keys.len() {
                        return Err(Parse::Invalid);
                    }
                    let last = (self.cursor.row + tn - 1).min(self.lines.len() - 1);
                    Span::Lines(self.cursor.row, last)
                } else if matches!(next, Key::Char('i' | 'a')) {
                    let Some(&obj) = keys.get(k + 1) else { return Err(Parse::Incomplete) };
                    if k + 2 != keys.len() {
                        return Err(Parse::Invalid);
                    }
                    let Key::Char(obj) = obj else { return Err(Parse::Invalid) };
                    let (s, e) = self.text_object(next == Key::Char('i'), obj).ok_or(Parse::Invalid)?;
                    Span::Chars(s, e)
                } else {
                    let Some((motion, end)) = self.parse_motion(keys, k, total)? else { return Err(Parse::Invalid) };
                    if end != keys.len() {
                        return Err(Parse::Invalid);
                    }
                    self.motion_span(c, motion, keys.get(k) == Some(&Key::Char('w')) || keys.get(k) == Some(&Key::Char('W')))
                };
                Ok(self.operate(c, span))
            }
            'r' => {
                let Some(&next) = keys.get(i + 1) else { return Err(Parse::Incomplete) };
                let Key::Char(with) = next else { return Err(Parse::Invalid) };
                let row = self.cursor.row;
                if self.cursor.col + n > self.line_len(row) {
                    return Err(Parse::Invalid);
                }
                self.snapshot();
                for col in self.cursor.col..self.cursor.col + n {
                    self.lines[row][col] = with;
                }
                self.cursor.col += n - 1;
                self.changed();
                Ok(Outcome::Ok)
            }
            'x' | 'X' | 's' | 'D' | 'C' | 'S' | 'Y' => {
                let motion_keys = match c {
                    'x' => "dl",
                    'X' => "dh",
                    's' => "cl",
                    'D' => "d$",
                    'C' => "c$",
                    'S' => "cc",
                    _ => "yy",
                };
                let mut expanded: Vec<Key> = keys[..i].to_vec();
                expanded.extend(parse_keys(motion_keys));
                if c == 'x' || c == 'X' {
                    let row = self.cursor.row;
                    if self.lines[row].is_empty() || (c == 'X' && self.cursor.col == 0) {
                        return Ok(Outcome::Bell("Nothing to delete".into()));
                    }
                }
                self.run_normal(&expanded)
            }
            'p' | 'P' => {
                if self.register.text.is_empty() && !self.register.linewise {
                    return Ok(Outcome::Bell("Nothing yanked yet".into()));
                }
                self.snapshot();
                self.put(c == 'p', n);
                self.changed();
                Ok(Outcome::Ok)
            }
            'u' => {
                self.recording = None;
                let mut done = 0;
                for _ in 0..n {
                    if let Some(s) = self.undo.pop() {
                        self.redo.push(Snapshot { lines: self.lines.clone(), cursor: self.cursor });
                        self.lines = s.lines;
                        self.cursor = s.cursor;
                        done += 1;
                    }
                }
                if done == 0 {
                    return Ok(Outcome::Bell("Already at oldest change".into()));
                }
                self.changed();
                self.clamp();
                Ok(Outcome::Ok)
            }
            '.' => {
                self.recording = None;
                if self.last_change.is_empty() {
                    return Ok(Outcome::Bell("No change to repeat".into()));
                }
                let mut change = self.last_change.clone();
                if count.is_some() {
                    let mut j = 0;
                    Self::read_count(&change, &mut j);
                    let mut with: Vec<Key> = keys[..i].to_vec();
                    with.extend_from_slice(&change[j..]);
                    change = with;
                }
                self.pending.clear();
                self.replaying = true;
                self.snapshot_force();
                for k in change.clone() {
                    self.feed(k);
                }
                if self.mode == Mode::Insert {
                    self.feed(Key::Esc);
                }
                self.replaying = false;
                self.last_change = change;
                Ok(Outcome::Ok)
            }
            'J' => {
                let joins = n.saturating_sub(1).max(1);
                if self.cursor.row + 1 >= self.lines.len() {
                    return Ok(Outcome::Bell("Nothing below to join".into()));
                }
                self.snapshot();
                for _ in 0..joins {
                    let row = self.cursor.row;
                    if row + 1 >= self.lines.len() {
                        break;
                    }
                    let next: Vec<char> = self.lines.remove(row + 1);
                    let next: Vec<char> = next.into_iter().skip_while(|c| c.is_whitespace()).collect();
                    while self.lines[row].last().is_some_and(|c| *c == ' ') {
                        self.lines[row].pop();
                    }
                    let col = self.lines[row].len();
                    if !next.is_empty() && !self.lines[row].is_empty() && next[0] != ')' {
                        self.lines[row].push(' ');
                    }
                    self.lines[row].extend(next);
                    self.cursor.col = col;
                }
                self.changed();
                Ok(Outcome::Ok)
            }
            '~' => {
                let row = self.cursor.row;
                if self.lines[row].is_empty() {
                    return Ok(Outcome::Bell(String::new()));
                }
                self.snapshot();
                let end = (self.cursor.col + n).min(self.line_len(row));
                for col in self.cursor.col..end {
                    self.lines[row][col] = toggle_case(self.lines[row][col]);
                }
                self.cursor.col = end.min(self.last_col(row));
                self.changed();
                Ok(Outcome::Ok)
            }
            'i' | 'a' | 'I' | 'A' | 'o' | 'O' => {
                self.snapshot();
                let row = self.cursor.row;
                match c {
                    'i' => {}
                    'a' => self.cursor.col = (self.cursor.col + 1).min(self.line_len(row)),
                    'I' => self.cursor.col = self.first_non_blank(row).min(self.line_len(row)),
                    'A' => self.cursor.col = self.line_len(row),
                    'o' => {
                        self.lines.insert(row + 1, Vec::new());
                        self.cursor = pos(row + 1, 0);
                        self.changed();
                    }
                    _ => {
                        self.lines.insert(row, Vec::new());
                        self.cursor = pos(row, 0);
                        self.changed();
                    }
                }
                self.mode = Mode::Insert;
                Ok(Outcome::Ok)
            }
            'v' | 'V' => {
                self.recording = None;
                self.mode = Mode::Visual { line: c == 'V' };
                self.anchor = self.cursor;
                Ok(Outcome::Ok)
            }
            ':' | '/' => {
                self.recording = None;
                self.mode = Mode::Command { search: c == '/' };
                self.cmdline.clear();
                Ok(Outcome::Ok)
            }
            _ => Err(Parse::Invalid),
        }
    }

    fn snapshot_force(&mut self) {
        self.undo.push(Snapshot { lines: self.lines.clone(), cursor: self.cursor });
        self.redo.clear();
    }

    fn redo_change(&mut self, n: usize) -> Outcome {
        let mut done = 0;
        for _ in 0..n {
            if let Some(s) = self.redo.pop() {
                self.undo.push(Snapshot { lines: self.lines.clone(), cursor: self.cursor });
                self.lines = s.lines;
                self.cursor = s.cursor;
                done += 1;
            }
        }
        if done == 0 {
            return Outcome::Bell("Already at newest change".into());
        }
        self.changed();
        self.clamp();
        Outcome::Ok
    }

    fn run_visual(&mut self, keys: &[Key]) -> Result<Outcome, Parse> {
        let mut i = 0;
        let count = Self::read_count(keys, &mut i);
        let Some(&key) = keys.get(i) else { return Err(Parse::Incomplete) };
        let Mode::Visual { line } = self.mode else { return Err(Parse::Invalid) };
        if matches!(key, Key::Char('i' | 'a')) {
            let Some(&obj) = keys.get(i + 1) else { return Err(Parse::Incomplete) };
            let Key::Char(obj) = obj else { return Err(Parse::Invalid) };
            let (s, e) = self.text_object(key == Key::Char('i'), obj).ok_or(Parse::Invalid)?;
            if e <= s {
                return Ok(Outcome::Bell(String::new()));
            }
            self.anchor = s;
            self.cursor = self.before(e);
            return Ok(Outcome::Ok);
        }
        if let Some((motion, end)) = self.parse_motion(keys, i, count)? {
            if end != keys.len() {
                return Err(Parse::Invalid);
            }
            self.apply_motion(motion);
            return Ok(Outcome::Ok);
        }
        let Key::Char(c) = key else { return Err(Parse::Invalid) };
        if keys.len() > i + 1 {
            return Err(Parse::Invalid);
        }
        let (s, e) = if self.anchor <= self.cursor { (self.anchor, self.cursor) } else { (self.cursor, self.anchor) };
        let span = if line {
            Span::Lines(s.row, e.row)
        } else {
            Span::Chars(s, self.after(e))
        };
        match c {
            'o' => {
                std::mem::swap(&mut self.anchor, &mut self.cursor);
                Ok(Outcome::Ok)
            }
            'v' | 'V' => {
                let want_line = c == 'V';
                self.mode = if want_line == line { Mode::Normal } else { Mode::Visual { line: want_line } };
                Ok(Outcome::Ok)
            }
            'd' | 'x' | 'y' | 'c' | 's' => {
                self.mode = Mode::Normal;
                let op = match c {
                    'x' => 'd',
                    's' => 'c',
                    o => o,
                };
                Ok(self.operate(op, span))
            }
            '~' | 'u' | 'U' => {
                self.mode = Mode::Normal;
                self.snapshot();
                let (rs, re) = match span {
                    Span::Lines(a, b) => (pos(a, 0), pos(b, self.line_len(b))),
                    Span::Chars(a, b) => (a, b),
                };
                for row in rs.row..=re.row {
                    let from = if row == rs.row { rs.col } else { 0 };
                    let to = if row == re.row { re.col.min(self.line_len(row)) } else { self.line_len(row) };
                    for col in from..to {
                        let ch = self.lines[row][col];
                        self.lines[row][col] = match c {
                            'u' => ch.to_lowercase().next().unwrap_or(ch),
                            'U' => ch.to_uppercase().next().unwrap_or(ch),
                            _ => toggle_case(ch),
                        };
                    }
                }
                self.cursor = rs;
                self.changed();
                self.clamp();
                Ok(Outcome::Ok)
            }
            'J' => {
                self.mode = Mode::Normal;
                let (a, b) = (s.row, e.row.max(s.row + 1));
                self.cursor = pos(a, self.cursor.col);
                let keys = parse_keys(&format!("{}J", b - a + 1));
                self.run_normal(&keys)
            }
            _ => Err(Parse::Invalid),
        }
    }

    /// The position after `p` in the buffer (the end of its line when `p` is
    /// the last character).
    fn after(&self, p: Pos) -> Pos {
        pos(p.row, (p.col + 1).min(self.line_len(p.row)))
    }

    fn before(&self, p: Pos) -> Pos {
        if p.col > 0 {
            pos(p.row, p.col - 1)
        } else if p.row > 0 {
            pos(p.row - 1, self.last_col(p.row - 1))
        } else {
            p
        }
    }

    fn motion_span(&self, op: char, motion: Motion, word: bool) -> Span {
        let cur = self.cursor;
        if motion.kind == MotionKind::Linewise {
            let (a, b) = if motion.to.row < cur.row { (motion.to.row, cur.row) } else { (cur.row, motion.to.row) };
            return Span::Lines(a, b);
        }
        let (mut s, mut e) = if motion.to < cur { (motion.to, cur) } else { (cur, motion.to) };
        let mut inclusive = motion.kind == MotionKind::Inclusive;
        if op == 'c' && word && motion.to > cur {
            // `cw` changes to the end of the word, as `ce` would.
            if self.char_at(cur).is_some_and(|c| !c.is_whitespace()) {
                let big = false;
                let mut p = cur;
                let cls = class(self.char_at(cur).unwrap(), big);
                while self.char_at(pos(p.row, p.col + 1)).is_some_and(|c| class(c, big) == cls) {
                    p.col += 1;
                }
                e = p;
                inclusive = true;
            }
        }
        if !inclusive && e.row > s.row && e.col == 0 {
            // An exclusive motion ending at the start of a line stops at
            // the end of the line before (`dw` on a line's last word).
            e = pos(e.row - 1, self.line_len(e.row - 1));
            return Span::Chars(s, e);
        }
        if inclusive {
            e = self.after(e);
        }
        if s.col > self.line_len(s.row) {
            s.col = self.line_len(s.row);
        }
        Span::Chars(s, e)
    }

    fn span_text(&self, span: Span) -> Register {
        match span {
            Span::Lines(a, b) => Register {
                text: self.lines[a..=b].iter().map(|l| l.iter().collect::<String>()).collect::<Vec<_>>().join("\n"),
                linewise: true,
            },
            Span::Chars(s, e) => {
                let mut out = String::new();
                for row in s.row..=e.row {
                    let line = &self.lines[row];
                    let from = if row == s.row { s.col.min(line.len()) } else { 0 };
                    let to = if row == e.row { e.col.min(line.len()) } else { line.len() };
                    out.extend(line[from..to.max(from)].iter());
                    if row != e.row {
                        out.push('\n');
                    }
                }
                Register { text: out, linewise: false }
            }
        }
    }

    fn delete_span(&mut self, span: Span) {
        match span {
            Span::Lines(a, b) => {
                self.lines.drain(a..=b);
                if self.lines.is_empty() {
                    self.lines.push(Vec::new());
                }
                let row = a.min(self.lines.len() - 1);
                self.cursor = pos(row, self.first_non_blank(row));
            }
            Span::Chars(s, e) => {
                if s.row == e.row {
                    let len = self.line_len(s.row);
                    self.lines[s.row].drain(s.col.min(len)..e.col.min(len));
                } else {
                    let tail: Vec<char> = self.lines[e.row][e.col.min(self.line_len(e.row))..].to_vec();
                    self.lines[s.row].truncate(s.col);
                    self.lines[s.row].extend(tail);
                    self.lines.drain(s.row + 1..=e.row);
                }
                self.cursor = s;
            }
        }
    }

    fn operate(&mut self, op: char, span: Span) -> Outcome {
        if let Span::Chars(s, e) = span
            && e <= s && op != 'c' {
                return Outcome::Bell(String::new());
            }
        let text = self.span_text(span);
        match op {
            'y' => {
                self.register = text;
                self.cursor = match span {
                    Span::Lines(a, _) => pos(a, self.cursor.col.min(self.last_col(a))).min(self.cursor),
                    Span::Chars(s, _) => s,
                };
                if let Span::Lines(a, _) = span
                    && self.cursor.row != a {
                        self.cursor = pos(a, 0);
                    }
                self.recording = None;
                Outcome::Ok
            }
            'd' => {
                self.snapshot();
                self.register = text;
                self.delete_span(span);
                self.changed();
                self.mode = Mode::Normal;
                self.clamp();
                self.want = self.cursor.col;
                Outcome::Ok
            }
            _ => {
                self.snapshot();
                self.register = text;
                match span {
                    Span::Lines(a, b) => {
                        self.lines.drain(a + 1..=b);
                        self.lines[a].clear();
                        self.cursor = pos(a, 0);
                    }
                    Span::Chars(..) => self.delete_span(span),
                }
                self.changed();
                self.mode = Mode::Insert;
                self.clamp();
                Outcome::Ok
            }
        }
    }

    fn put(&mut self, after: bool, n: usize) {
        let reg = self.register.clone();
        if reg.linewise {
            let new: Vec<Vec<char>> = (0..n)
                .flat_map(|_| reg.text.split('\n').map(|l| l.chars().collect::<Vec<char>>()).collect::<Vec<_>>())
                .collect();
            let at = if after { self.cursor.row + 1 } else { self.cursor.row };
            for (k, line) in new.into_iter().enumerate() {
                self.lines.insert(at + k, line);
            }
            self.cursor = pos(at, self.first_non_blank(at));
        } else {
            let text: String = reg.text.repeat(n);
            let row = self.cursor.row;
            let col = if after && !self.lines[row].is_empty() { self.cursor.col + 1 } else { self.cursor.col };
            let col = col.min(self.line_len(row));
            let tail: Vec<char> = self.lines[row].split_off(col);
            let parts: Vec<&str> = text.split('\n').collect();
            let mut end = pos(row, col);
            for (k, part) in parts.iter().enumerate() {
                if k > 0 {
                    self.lines.insert(end.row + 1, Vec::new());
                    end = pos(end.row + 1, 0);
                }
                self.lines[end.row].extend(part.chars());
                end.col = self.line_len(end.row);
            }
            self.lines[end.row].extend(tail);
            self.cursor = if parts.len() > 1 { pos(row, col) } else { pos(end.row, end.col.saturating_sub(1)) };
        }
        self.clamp();
    }

    // ---------------------------------------------------------------- words

    /// The next position, and whether it crossed a line.
    fn next(&self, p: Pos) -> Option<(Pos, bool)> {
        if p.col + 1 < self.line_len(p.row) {
            Some((pos(p.row, p.col + 1), false))
        } else if p.row + 1 < self.lines.len() {
            Some((pos(p.row + 1, 0), true))
        } else {
            None
        }
    }

    fn prev(&self, p: Pos) -> Option<(Pos, bool)> {
        if p.col > 0 {
            Some((pos(p.row, p.col - 1), false))
        } else if p.row > 0 {
            Some((pos(p.row - 1, self.last_col(p.row - 1)), true))
        } else {
            None
        }
    }

    fn cls_at(&self, p: Pos, big: bool) -> u8 {
        self.char_at(p).map_or(0, |c| class(c, big))
    }

    fn empty_line(&self, row: usize) -> bool {
        self.lines[row].is_empty()
    }

    fn word_forward(&self, start: Pos, big: bool) -> Pos {
        let mut p = start;
        let cls = self.cls_at(p, big);
        if cls != 0 {
            loop {
                match self.next(p) {
                    None => return self.past_end(),
                    Some((q, nl)) => {
                        p = q;
                        if nl || self.cls_at(q, big) != cls {
                            break;
                        }
                    }
                }
            }
        }
        loop {
            if self.empty_line(p.row) && p != start {
                return p;
            }
            if self.cls_at(p, big) != 0 && p != start {
                return p;
            }
            match self.next(p) {
                None => return self.past_end(),
                Some((q, _)) => p = q,
            }
        }
    }

    /// Just past the buffer's last character: where `w` stops when no word
    /// follows (the cursor clamps back onto the last character).
    fn past_end(&self) -> Pos {
        let row = self.lines.len() - 1;
        pos(row, self.line_len(row))
    }

    fn word_end(&self, start: Pos, big: bool) -> Pos {
        let Some((mut p, _)) = self.next(start) else { return start };
        while self.cls_at(p, big) == 0 {
            match self.next(p) {
                None => return p,
                Some((q, _)) => p = q,
            }
        }
        let cls = self.cls_at(p, big);
        while let Some((q, nl)) = self.next(p) {
            if nl || self.cls_at(q, big) != cls {
                break;
            }
            p = q;
        }
        p
    }

    fn word_backward(&self, start: Pos, big: bool) -> Pos {
        let Some((mut p, _)) = self.prev(start) else { return start };
        while self.cls_at(p, big) == 0 {
            if self.empty_line(p.row) {
                return pos(p.row, 0);
            }
            match self.prev(p) {
                None => return p,
                Some((q, _)) => p = q,
            }
        }
        let cls = self.cls_at(p, big);
        while let Some((q, nl)) = self.prev(p) {
            if nl || self.cls_at(q, big) != cls {
                break;
            }
            p = q;
        }
        p
    }

    fn word_end_backward(&self, start: Pos, big: bool) -> Pos {
        let mut p = start;
        let cls = self.cls_at(p, big);
        if cls != 0 {
            loop {
                match self.prev(p) {
                    None => return pos(0, 0),
                    Some((q, nl)) => {
                        p = q;
                        if nl || self.cls_at(q, big) != cls {
                            break;
                        }
                    }
                }
            }
        }
        while self.cls_at(p, big) == 0 && !self.empty_line(p.row) {
            match self.prev(p) {
                None => return p,
                Some((q, _)) => p = q,
            }
        }
        p
    }

    fn word_under(&self, p: Pos) -> Option<String> {
        let line = &self.lines[p.row];
        let mut col = p.col;
        while col < line.len() && class(line[col], false) != 2 {
            col += 1;
        }
        if col >= line.len() {
            return None;
        }
        let mut s = col;
        while s > 0 && class(line[s - 1], false) == 2 {
            s -= 1;
        }
        let mut e = col;
        while e < line.len() && class(line[e], false) == 2 {
            e += 1;
        }
        Some(line[s..e].iter().collect())
    }

    // ------------------------------------------------------------ searching

    fn find(&self, kind: char, target: char, n: usize, repeat: bool) -> Option<Pos> {
        let line = &self.lines[self.cursor.row];
        let mut col = self.cursor.col;
        let forward = kind == 'f' || kind == 't';
        let till = kind == 't' || kind == 'T';
        for k in 0..n {
            // `;` after `t` skips the character right before the target.
            let skip = till && repeat && k == 0;
            let mut c = col;
            loop {
                if forward {
                    c += 1;
                    if c >= line.len() {
                        return None;
                    }
                } else {
                    if c == 0 {
                        return None;
                    }
                    c -= 1;
                }
                if line[c] == target {
                    if skip && ((forward && c == col + 1) || (!forward && c + 1 == col)) {
                        continue;
                    }
                    break;
                }
            }
            col = c;
        }
        let col = if till { if forward { col - 1 } else { col + 1 } } else { col };
        Some(pos(self.cursor.row, col))
    }

    fn search_from(&self, from: Pos, forward: bool, whole_word: bool) -> Option<Pos> {
        let pat: Vec<char> = self.last_search.as_ref()?.chars().collect();
        if pat.is_empty() {
            return None;
        }
        let mut hits = Vec::new();
        for (row, line) in self.lines.iter().enumerate() {
            if line.len() < pat.len() {
                continue;
            }
            for col in 0..=line.len() - pat.len() {
                if line[col..col + pat.len()] == pat[..] {
                    if whole_word {
                        let before = col > 0 && class(line[col - 1], false) == 2;
                        let after = col + pat.len() < line.len() && class(line[col + pat.len()], false) == 2;
                        if before || after {
                            continue;
                        }
                    }
                    hits.push(pos(row, col));
                }
            }
        }
        if forward {
            hits.iter().find(|&&h| h > from).or(hits.first()).copied()
        } else {
            hits.iter().rev().find(|&&h| h < from).or(hits.last()).copied()
        }
    }

    fn match_pair(&self, at: Pos) -> Option<Pos> {
        let line = &self.lines[at.row];
        let col = (at.col..line.len()).find(|&c| matching(line[c]).is_some())?;
        let (open, close, forward) = matching(line[col])?;
        let mut depth = 0i32;
        let mut p = pos(at.row, col);
        loop {
            let c = self.char_at(p);
            if c == Some(open) {
                depth += if forward { 1 } else { -1 };
            } else if c == Some(close) {
                depth += if forward { -1 } else { 1 };
            }
            if depth == 0 {
                return Some(p);
            }
            p = if forward { self.next(p)?.0 } else { self.prev(p)?.0 };
        }
    }

    fn paragraph(&self, row: usize, down: bool) -> usize {
        let mut r = row;
        let last = self.lines.len() - 1;
        // Skip the blank lines we are on, then the paragraph.
        if down {
            while r < last && self.lines[r].is_empty() {
                r += 1;
            }
            while r < last && !self.lines[r].is_empty() {
                r += 1;
            }
        } else {
            while r > 0 && self.lines[r].is_empty() {
                r -= 1;
            }
            while r > 0 && !self.lines[r].is_empty() {
                r -= 1;
            }
        }
        r
    }

    // --------------------------------------------------------- text objects

    /// The range [start, end) of a text object around the cursor.
    fn text_object(&self, inner: bool, obj: char) -> Option<(Pos, Pos)> {
        let cur = self.cursor;
        match obj {
            'w' | 'W' => {
                let big = obj == 'W';
                let line = &self.lines[cur.row];
                if line.is_empty() {
                    return None;
                }
                let cls = class(line[cur.col], big);
                let mut s = cur.col;
                while s > 0 && class(line[s - 1], big) == cls {
                    s -= 1;
                }
                let mut e = cur.col + 1;
                while e < line.len() && class(line[e], big) == cls {
                    e += 1;
                }
                if !inner {
                    if cls != 0 {
                        let ws_end = {
                            let mut k = e;
                            while k < line.len() && line[k].is_whitespace() {
                                k += 1;
                            }
                            k
                        };
                        if ws_end > e {
                            e = ws_end;
                        } else {
                            while s > 0 && line[s - 1].is_whitespace() {
                                s -= 1;
                            }
                        }
                    } else {
                        let mut k = e;
                        while k < line.len() && class(line[k], big) == class(line.get(e).copied().unwrap_or(' '), big) && !line[k].is_whitespace() {
                            k += 1;
                        }
                        e = k;
                    }
                }
                Some((pos(cur.row, s), pos(cur.row, e)))
            }
            '"' | '\'' | '`' => {
                let line = &self.lines[cur.row];
                let quotes: Vec<usize> = line.iter().enumerate().filter(|(_, c)| **c == obj).map(|(i, _)| i).collect();
                let pair = quotes
                    .chunks(2)
                    .filter(|p| p.len() == 2)
                    .find(|p| p[0] <= cur.col && cur.col <= p[1])
                    .or_else(|| quotes.chunks(2).filter(|p| p.len() == 2).find(|p| p[0] > cur.col))?;
                let (a, b) = (pair[0], pair[1]);
                if inner {
                    Some((pos(cur.row, a + 1), pos(cur.row, b)))
                } else {
                    let mut e = b + 1;
                    while e < line.len() && line[e].is_whitespace() {
                        e += 1;
                    }
                    Some((pos(cur.row, a), pos(cur.row, e)))
                }
            }
            '(' | ')' | 'b' | '[' | ']' | '{' | '}' | 'B' | '<' | '>' => {
                let (open, close) = match obj {
                    '(' | ')' | 'b' => ('(', ')'),
                    '[' | ']' => ('[', ']'),
                    '<' | '>' => ('<', '>'),
                    _ => ('{', '}'),
                };
                // Search backwards for the unmatched opening bracket.
                let mut p = cur;
                let mut depth = 0;
                if self.char_at(p) == Some(close) {
                    depth = -1;
                }
                let start = loop {
                    match self.char_at(p) {
                        Some(c) if c == open => {
                            if depth == 0 {
                                break p;
                            }
                            depth -= 1;
                        }
                        Some(c) if c == close => depth += 1,
                        _ => {}
                    }
                    if depth < 0 {
                        depth = 0;
                    }
                    p = self.prev(p)?.0;
                };
                let mut depth = 0;
                let mut q = start;
                let end = loop {
                    match self.char_at(q) {
                        Some(c) if c == open => depth += 1,
                        Some(c) if c == close => {
                            depth -= 1;
                            if depth == 0 {
                                break q;
                            }
                        }
                        _ => {}
                    }
                    q = self.next(q)?.0;
                };
                if inner {
                    Some((self.after(start), end))
                } else {
                    Some((start, self.after(end)))
                }
            }
            _ => None,
        }
    }
}

fn toggle_case(c: char) -> char {
    if c.is_lowercase() {
        c.to_uppercase().next().unwrap_or(c)
    } else if c.is_uppercase() {
        c.to_lowercase().next().unwrap_or(c)
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str, at: Pos, keys: &str) -> Editor {
        let mut e = Editor::new(text).with_cursor(at);
        e.feed_all(&parse_keys(keys));
        e
    }

    #[test]
    fn hjkl() {
        let e = run("abc\ndef\nghi", pos(0, 0), "jjll");
        assert_eq!(e.cursor, pos(2, 2));
        let e = run("abc\ndef", pos(0, 0), "5l");
        assert_eq!(e.cursor, pos(0, 2));
    }

    #[test]
    fn remembered_column() {
        let e = run("abcdef\nab\nabcdef", pos(0, 4), "jj");
        assert_eq!(e.cursor, pos(2, 4));
        let e = run("abcdef\nab\nabcdefgh", pos(0, 0), "$jj");
        assert_eq!(e.cursor, pos(2, 7));
    }

    #[test]
    fn words() {
        let text = "foo.bar baz\n\nqux";
        let e = run(text, pos(0, 0), "w");
        assert_eq!(e.cursor, pos(0, 3));
        let e = run(text, pos(0, 0), "W");
        assert_eq!(e.cursor, pos(0, 8));
        let e = run(text, pos(0, 0), "WW");
        assert_eq!(e.cursor, pos(1, 0));
        let e = run(text, pos(0, 0), "WWW");
        assert_eq!(e.cursor, pos(2, 0));
        let e = run(text, pos(0, 0), "e");
        assert_eq!(e.cursor, pos(0, 2));
        let e = run(text, pos(0, 0), "ee");
        assert_eq!(e.cursor, pos(0, 3));
        let e = run(text, pos(0, 0), "E");
        assert_eq!(e.cursor, pos(0, 6));
        let e = run(text, pos(2, 0), "b");
        assert_eq!(e.cursor, pos(1, 0));
        let e = run(text, pos(2, 0), "bb");
        assert_eq!(e.cursor, pos(0, 8));
        let e = run(text, pos(0, 10), "B");
        assert_eq!(e.cursor, pos(0, 8));
        let e = run("one two", pos(0, 5), "ge");
        assert_eq!(e.cursor, pos(0, 2));
    }

    #[test]
    fn line_motions() {
        let e = run("   abc def", pos(0, 5), "0");
        assert_eq!(e.cursor, pos(0, 0));
        let e = run("   abc def", pos(0, 0), "^");
        assert_eq!(e.cursor, pos(0, 3));
        let e = run("   abc def", pos(0, 0), "$");
        assert_eq!(e.cursor, pos(0, 9));
        let e = run("a\nb\nc\nd", pos(0, 0), "G");
        assert_eq!(e.cursor, pos(3, 0));
        let e = run("a\nb\nc\nd", pos(3, 0), "gg");
        assert_eq!(e.cursor, pos(0, 0));
        let e = run("a\nb\nc\nd", pos(0, 0), "3G");
        assert_eq!(e.cursor, pos(2, 0));
    }

    #[test]
    fn find_and_till() {
        let t = "a,b,c,d";
        assert_eq!(run(t, pos(0, 0), "f,").cursor, pos(0, 1));
        assert_eq!(run(t, pos(0, 0), "f,;").cursor, pos(0, 3));
        assert_eq!(run(t, pos(0, 0), "2f,").cursor, pos(0, 3));
        assert_eq!(run(t, pos(0, 0), "t,").cursor, pos(0, 0));
        assert_eq!(run(t, pos(0, 0), "tc").cursor, pos(0, 3));
        assert_eq!(run(t, pos(0, 0), "t,;").cursor, pos(0, 2));
        assert_eq!(run(t, pos(0, 6), "F,").cursor, pos(0, 5));
        assert_eq!(run(t, pos(0, 0), "f,;;,").cursor, pos(0, 3));
    }

    #[test]
    fn percent_and_paragraphs() {
        assert_eq!(run("if (a(b)) x", pos(0, 0), "%").cursor, pos(0, 8));
        assert_eq!(run("if (a(b)) x", pos(0, 8), "%").cursor, pos(0, 3));
        assert_eq!(run("a\nb\n\nc\n\nd", pos(0, 0), "}").cursor, pos(2, 0));
        assert_eq!(run("a\nb\n\nc\n\nd", pos(0, 0), "}}").cursor, pos(4, 0));
        assert_eq!(run("a\nb\n\nc\n\nd", pos(0, 0), "}}}").cursor, pos(5, 0));
        assert_eq!(run("a\nb\n\nc\n\nd", pos(5, 0), "{").cursor, pos(4, 0));
    }

    #[test]
    fn search() {
        let t = "cat dog\nbird cat\ncat";
        let e = run(t, pos(0, 0), "/cat<CR>");
        assert_eq!(e.cursor, pos(1, 5));
        let e = run(t, pos(0, 0), "/cat<CR>n");
        assert_eq!(e.cursor, pos(2, 0));
        let e = run(t, pos(0, 0), "/cat<CR>nn");
        assert_eq!(e.cursor, pos(0, 0));
        let e = run(t, pos(0, 0), "/cat<CR>N");
        assert_eq!(e.cursor, pos(0, 0));
        let e = run(t, pos(0, 0), "*");
        assert_eq!(e.cursor, pos(1, 5));
    }

    #[test]
    fn deletes() {
        assert_eq!(run("hello world", pos(0, 0), "x").text(), "ello world");
        assert_eq!(run("hello world", pos(0, 0), "3x").text(), "lo world");
        assert_eq!(run("hello world", pos(0, 0), "dw").text(), "world");
        assert_eq!(run("hello world", pos(0, 6), "dw").text(), "hello ");
        assert_eq!(run("one two\nthree", pos(0, 4), "dw").text(), "one \nthree");
        assert_eq!(run("hello world", pos(0, 0), "de").text(), " world");
        assert_eq!(run("hello world", pos(0, 3), "D").text(), "hel");
        assert_eq!(run("hello world", pos(0, 3), "d0").text(), "lo world");
        assert_eq!(run("a\nb\nc", pos(0, 0), "dd").text(), "b\nc");
        assert_eq!(run("a\nb\nc", pos(0, 0), "2dd").text(), "c");
        assert_eq!(run("a\nb\nc", pos(0, 0), "dj").text(), "c");
        assert_eq!(run("a\nb\nc", pos(2, 0), "dk").text(), "a");
        assert_eq!(run("a b c d", pos(0, 0), "d2w").text(), "c d");
        assert_eq!(run("a b c d", pos(0, 0), "2dw").text(), "c d");
        assert_eq!(run("a,b,c", pos(0, 0), "df,").text(), "b,c");
        assert_eq!(run("a,b,c", pos(0, 0), "dt,").text(), ",b,c");
        assert_eq!(run("abc", pos(0, 2), "X").text(), "ac");
    }

    #[test]
    fn changes() {
        assert_eq!(run("hello world", pos(0, 0), "cwbye<Esc>").text(), "bye world");
        assert_eq!(run("hello world", pos(0, 2), "ciwbye<Esc>").text(), "bye world");
        assert_eq!(run("hello world", pos(0, 0), "Cnope<Esc>").text(), "nope");
        assert_eq!(run("say \"hi there\" now", pos(0, 0), "ci\"yo<Esc>").text(), "say \"yo\" now");
        assert_eq!(run("f(a, b)", pos(0, 3), "ci(x<Esc>").text(), "f(x)");
        assert_eq!(run("f(a, b)", pos(0, 3), "da(").text(), "f");
        assert_eq!(run("a word here", pos(0, 3), "daw").text(), "a here");
        assert_eq!(run("abc", pos(0, 0), "sX<Esc>").text(), "Xbc");
        assert_eq!(run("abc\ndef", pos(0, 1), "Sxyz<Esc>").text(), "xyz\ndef");
        assert_eq!(run("abc", pos(0, 0), "rZ").text(), "Zbc");
        assert_eq!(run("abc", pos(0, 0), "3rZ").text(), "ZZZ");
    }

    #[test]
    fn inserts() {
        assert_eq!(run("bc", pos(0, 0), "ia<Esc>").text(), "abc");
        assert_eq!(run("ac", pos(0, 0), "ab<Esc>").text(), "abc");
        assert_eq!(run("  bc", pos(0, 3), "Ia<Esc>").text(), "  abc");
        assert_eq!(run("ab", pos(0, 0), "Ac<Esc>").text(), "abc");
        assert_eq!(run("a\nc", pos(0, 0), "ob<Esc>").text(), "a\nb\nc");
        assert_eq!(run("b", pos(0, 0), "Oa<Esc>").text(), "a\nb");
        assert_eq!(run("ab", pos(0, 1), "ix<BS>y<Esc>").text(), "ayb");
        let e = run("abc", pos(0, 0), "Ad<Esc>");
        assert_eq!(e.cursor, pos(0, 3));
    }

    #[test]
    fn yank_put() {
        assert_eq!(run("a\nb", pos(0, 0), "yyp").text(), "a\na\nb");
        assert_eq!(run("a\nb", pos(0, 0), "yyjP").text(), "a\na\nb");
        assert_eq!(run("a\nb", pos(0, 0), "ddp").text(), "b\na");
        assert_eq!(run("ab", pos(0, 0), "xp").text(), "ba");
        assert_eq!(run("one two", pos(0, 0), "yw$p").text(), "one twoone ");
        assert_eq!(run("a", pos(0, 0), "yy3p").text(), "a\na\na\na");
    }

    #[test]
    fn undo_redo_dot() {
        assert_eq!(run("abc", pos(0, 0), "xu").text(), "abc");
        assert_eq!(run("abc", pos(0, 0), "xxuu").text(), "abc");
        assert_eq!(run("abc", pos(0, 0), "xu<C-r>").text(), "bc");
        assert_eq!(run("abcd", pos(0, 0), "x..").text(), "d");
        assert_eq!(run("a b c", pos(0, 0), "dw.").text(), "c");
        assert_eq!(run("x\ny\nz", pos(0, 0), "A;<Esc>j.j.").text(), "x;\ny;\nz;");
        assert_eq!(run("cat cat cat", pos(0, 0), "cwdog<Esc>w.w.").text(), "dog dog dog");
        assert_eq!(run("abc", pos(0, 0), "x.u").text(), "bc");
    }

    #[test]
    fn visual() {
        assert_eq!(run("hello world", pos(0, 0), "vlld").text(), "lo world");
        assert_eq!(run("hello world", pos(0, 0), "vey$p").text(), "hello worldhello");
        assert_eq!(run("a\nb\nc", pos(0, 0), "Vjd").text(), "c");
        assert_eq!(run("hello world", pos(0, 0), "veU").text(), "HELLO world");
        assert_eq!(run("(a b)", pos(0, 1), "vi(d").text(), "()");
    }

    #[test]
    fn misc() {
        assert_eq!(run("a\n  b", pos(0, 0), "J").text(), "a b");
        assert_eq!(run("abc", pos(0, 0), "~~").text(), "ABc");
        assert_eq!(run("cat cat\ncat", pos(0, 0), ":s/cat/dog/<CR>").text(), "dog cat\ncat");
        assert_eq!(run("cat cat\ncat", pos(0, 0), ":%s/cat/dog/g<CR>").text(), "dog dog\ndog");
        assert_eq!(run("a\nb\nc", pos(0, 0), ":3<CR>").cursor, pos(2, 0));
    }

    #[test]
    fn awaiting_char() {
        let mut e = Editor::new("abc");
        e.feed(Key::Char('f'));
        assert!(e.awaiting_char());
        e.feed(Key::Char('c'));
        assert!(!e.awaiting_char());
        e.feed(Key::Char('d'));
        e.feed(Key::Char('t'));
        assert!(e.awaiting_char());
    }
}
