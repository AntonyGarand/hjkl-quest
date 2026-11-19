//! A small, self-contained vim emulator: the buffer, the cursor, the modes
//! and the commands the campaign teaches. No UI here, so it is tested with
//! plain `cargo test`.

use std::collections::HashMap;
use std::fmt;

/// One keystroke given to the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Esc,
    Enter,
    Backspace,
    CtrlR,
    CtrlA,
    CtrlX,
    CtrlV,
}

impl Key {
    /// The key as vim stores it in a register: the character itself, or the
    /// control character it sends (so a recorded macro is plain text).
    pub fn raw(self) -> char {
        match self {
            Key::Char(c) => c,
            Key::Esc => '\u{1b}',
            Key::Enter => '\r',
            Key::Backspace => '\u{8}',
            Key::CtrlR => '\u{12}',
            Key::CtrlA => '\u{1}',
            Key::CtrlX => '\u{18}',
            Key::CtrlV => '\u{16}',
        }
    }

    pub fn from_raw(c: char) -> Key {
        match c {
            '\u{1b}' => Key::Esc,
            '\r' | '\n' => Key::Enter,
            '\u{8}' => Key::Backspace,
            '\u{12}' => Key::CtrlR,
            '\u{1}' => Key::CtrlA,
            '\u{18}' => Key::CtrlX,
            '\u{16}' => Key::CtrlV,
            c => Key::Char(c),
        }
    }
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
            Key::CtrlA => write!(f, "<C-a>"),
            Key::CtrlX => write!(f, "<C-x>"),
            Key::CtrlV => write!(f, "<C-v>"),
        }
    }
}

/// Parses vim-golf notation: plain characters, plus `<Esc>`, `<CR>`, `<BS>`,
/// `<C-r>`, `<C-a>`, `<C-x>`, `<C-v>` and `<Space>`.
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
                    "c-a" => Some(Key::CtrlA),
                    "c-x" => Some(Key::CtrlX),
                    "c-v" => Some(Key::CtrlV),
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
    /// Visual block mode (`Ctrl-v`): a rectangle of columns.
    Block,
    /// Replace mode (`R`): typing overwrites.
    Replace,
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
    /// A jump (G, %, n, marks...): sets the `` ` `` mark to where it left.
    jump: bool,
}

/// After a block `I`, `A` or `c`, the text typed on the first line goes to
/// the others on `<Esc>`.
#[derive(Debug, Clone)]
struct BlockInsert {
    row: usize,
    start: usize,
    rows: std::ops::RangeInclusive<usize>,
    col: usize,
    /// `$A`: at the end of every line.
    to_end: bool,
    /// `A`: short lines are padded out to the column; else skipped.
    pad: bool,
}

/// shiftwidth, for `>` and `<`.
pub const SHIFT: usize = 4;

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
    /// The unnamed register `""`.
    pub register: Register,
    registers: HashMap<char, Register>,
    /// The register named by `"x` for the command being run.
    active_reg: Option<char>,
    marks: HashMap<char, Pos>,
    /// A macro being recorded with `q`: its register and keys.
    macro_rec: Option<(char, Vec<Key>)>,
    last_macro: Option<char>,
    /// How deep we are in replayed keys (`.`, `@`, `:normal`).
    depth: usize,
    /// Undo snapshots are held while an ex command runs as one change.
    quiet: usize,
    block_insert: Option<BlockInsert>,
    /// What `R` overwrote, for `<BS>`.
    replaced: Vec<Option<char>>,
    last_sub: Option<(String, String, bool)>,
    /// Rows `:g` still has to visit, kept up to date as lines move.
    g_targets: Option<Vec<usize>>,
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
            registers: HashMap::new(),
            active_reg: None,
            marks: HashMap::new(),
            macro_rec: None,
            last_macro: None,
            depth: 0,
            quiet: 0,
            block_insert: None,
            replaced: Vec::new(),
            last_sub: None,
            g_targets: None,
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
        let keys = &self.pending;
        let visual = self.in_visual();
        let mut i = 0;
        while i < keys.len() {
            let k = keys[i];
            let before_is_count = keys[..i].iter().all(|k| matches!(k, Key::Char('0'..='9')));
            let takes_arg = matches!(k, Key::Char('f' | 'F' | 't' | 'T' | 'r' | 'm' | '`' | '\'' | '"' | 'q' | '@'))
                // i and a pick a text object after an operator, or in visual mode.
                || matches!(k, Key::Char('i' | 'a')) && (visual || !before_is_count);
            if takes_arg {
                if i + 1 == keys.len() {
                    return true;
                }
                i += 2;
            } else {
                i += 1;
            }
        }
        false
    }

    pub fn in_visual(&self) -> bool {
        matches!(self.mode, Mode::Visual { .. } | Mode::Block)
    }

    /// The register a macro is being recorded into.
    pub fn recording_macro(&self) -> Option<char> {
        self.macro_rec.as_ref().map(|(r, _)| *r)
    }

    /// The selected rectangle in visual block mode: rows, columns, and
    /// whether it runs to the end of every line (`$`).
    pub fn block_rect(&self) -> (usize, usize, usize, usize, bool) {
        let (r0, r1) = (self.anchor.row.min(self.cursor.row), self.anchor.row.max(self.cursor.row));
        let (c0, c1) = (self.anchor.col.min(self.cursor.col), self.anchor.col.max(self.cursor.col));
        (r0, r1, c0, c1, self.want == usize::MAX)
    }

    /// Whether the pending keys end with a count digit.
    pub fn register_text(&self, name: char) -> Option<String> {
        if name == '"' {
            return Some(self.register.text.clone());
        }
        self.registers.get(&name.to_ascii_lowercase()).map(|r| r.text.clone())
    }

    pub fn in_count(&self) -> bool {
        matches!(self.pending.last(), Some(Key::Char('1'..='9')))
            || (matches!(self.pending.last(), Some(Key::Char('0'))) && self.pending.len() > 1 && {
                let before = &self.pending[..self.pending.len() - 1];
                matches!(before.last(), Some(Key::Char('0'..='9')))
            })
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
            Mode::Insert | Mode::Replace => self.line_len(self.cursor.row),
            _ => self.last_col(self.cursor.row),
        };
        self.cursor.col = self.cursor.col.min(max);
    }

    fn first_non_blank(&self, row: usize) -> usize {
        let line = &self.lines[row];
        line.iter().position(|c| !c.is_whitespace()).unwrap_or(line.len().saturating_sub(1))
    }

    fn snapshot(&mut self) {
        if self.replaying || self.quiet > 0 {
            return;
        }
        self.undo.push(Snapshot { lines: self.lines.clone(), cursor: self.cursor });
        self.redo.clear();
    }

    /// Stores yanked or deleted text: in the register named with `"x`,
    /// else the unnamed one plus `"0` (yanks) or `"1`/`"-` (deletes).
    fn store(&mut self, text: Register, yank: bool) {
        match self.active_reg {
            Some('_') => return,
            Some(c) if c.is_ascii_uppercase() => {
                let entry = self.registers.entry(c.to_ascii_lowercase()).or_default();
                if entry.linewise || text.linewise {
                    if !entry.text.is_empty() || entry.linewise {
                        entry.text.push('\n');
                    }
                    entry.linewise = true;
                }
                entry.text.push_str(&text.text);
                self.register = entry.clone();
                return;
            }
            Some(c) if c != '"' => {
                self.registers.insert(c, text.clone());
            }
            _ => {
                if yank {
                    self.registers.insert('0', text.clone());
                } else if text.linewise || text.text.contains('\n') {
                    for k in (1..9).rev() {
                        if let Some(r) = self.registers.get(&char::from_digit(k, 10).unwrap()).cloned() {
                            self.registers.insert(char::from_digit(k + 1, 10).unwrap(), r);
                        }
                    }
                    self.registers.insert('1', text.clone());
                } else {
                    self.registers.insert('-', text.clone());
                }
            }
        }
        self.register = text;
    }

    /// The register a put reads: `"x`, or the unnamed one.
    fn read_reg(&self) -> Register {
        match self.active_reg {
            Some(c) if c != '"' => self.registers.get(&c.to_ascii_lowercase()).cloned().unwrap_or_default(),
            _ => self.register.clone(),
        }
    }

    fn valid_register(c: char) -> bool {
        c.is_ascii_alphanumeric() || matches!(c, '"' | '_' | '-')
    }

    /// Feeds replayed keys (a macro, `:normal`); stops at the first error,
    /// as vim aborts a macro. Returns whether all keys ran.
    fn replay(&mut self, keys: &[Key]) -> bool {
        self.depth += 1;
        let mut ok = true;
        for &k in keys {
            if let Outcome::Bell(msg) = self.feed(k)
                && !msg.is_empty() {
                    ok = false;
                    break;
                }
        }
        self.depth -= 1;
        ok
    }

    fn changed(&mut self) {
        self.revision += 1;
    }

    // ----------------------------------------------------------------- keys

    /// Takes one key.
    pub fn feed(&mut self, key: Key) -> Outcome {
        if self.depth == 0
            && let Some((_, keys)) = &mut self.macro_rec {
                keys.push(key);
            }
        if let Some(rec) = &mut self.recording {
            rec.push(key);
        }
        let outcome = match self.mode {
            Mode::Insert => self.insert_key(key),
            Mode::Replace => self.replace_key(key),
            Mode::Command { search } => self.command_key(key, search),
            Mode::Normal | Mode::Visual { .. } | Mode::Block => self.normal_key(key),
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
                if let Some(b) = self.block_insert.take()
                    && self.cursor.row == b.row && self.cursor.col >= b.start {
                        let typed: Vec<char> = self.lines[b.row][b.start..self.cursor.col].to_vec();
                        for row in b.rows.clone() {
                            let len = self.line_len(row);
                            let col = if b.to_end { len } else { b.col };
                            if len < col {
                                if !b.pad {
                                    continue;
                                }
                                self.lines[row].resize(col, ' ');
                            }
                            for (k, &c) in typed.iter().enumerate() {
                                self.lines[row].insert(col + k, c);
                            }
                        }
                        self.changed();
                        self.mode = Mode::Normal;
                        self.cursor = pos(b.row, b.start);
                        self.clamp();
                        self.want = self.cursor.col;
                        return Outcome::Ok;
                    }
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
            _ => return Outcome::Bell("That key does nothing in insert mode here".into()),
        }
        Outcome::Ok
    }

    fn replace_key(&mut self, key: Key) -> Outcome {
        let row = self.cursor.row;
        match key {
            Key::Esc => {
                self.mode = Mode::Normal;
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                }
                self.clamp();
                self.want = self.cursor.col;
            }
            Key::Backspace => {
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                    match self.replaced.pop() {
                        Some(Some(c)) => self.lines[row][self.cursor.col] = c,
                        Some(None) => {
                            self.lines[row].remove(self.cursor.col);
                        }
                        None => {}
                    }
                    self.changed();
                }
            }
            Key::Enter => {
                self.replaced.clear();
                let rest: Vec<char> = self.lines[row].split_off(self.cursor.col);
                self.lines.insert(row + 1, rest);
                self.cursor = pos(row + 1, 0);
                self.changed();
            }
            Key::Char(c) => {
                if self.cursor.col < self.line_len(row) {
                    self.replaced.push(Some(self.lines[row][self.cursor.col]));
                    self.lines[row][self.cursor.col] = c;
                } else {
                    self.replaced.push(None);
                    self.lines[row].push(c);
                }
                self.cursor.col += 1;
                self.changed();
            }
            _ => return Outcome::Bell("That key does nothing in replace mode here".into()),
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
            Key::CtrlR | Key::CtrlA | Key::CtrlX | Key::CtrlV => Outcome::Ok,
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
                            self.marks.insert('\'', self.cursor);
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

    fn push_undo(&mut self, lines: Vec<Vec<char>>, cursor: Pos) {
        if self.replaying || self.quiet > 0 {
            return;
        }
        self.undo.push(Snapshot { lines, cursor });
        self.redo.clear();
    }

    /// One ex address: a number, `.`, `$`, `'x`, with `+n`/`-n` offsets.
    /// Rows are 0-based; line "0" (before the first) is -1.
    fn parse_addr<'a>(&self, s: &'a str) -> Result<(Option<i64>, &'a str), String> {
        let cur = self.cursor.row as i64;
        let last = self.lines.len() as i64 - 1;
        let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
        let (mut row, mut rest) = if !digits.is_empty() {
            (Some(digits.parse::<i64>().unwrap_or(0) - 1), &s[digits.len()..])
        } else if let Some(r) = s.strip_prefix('.') {
            (Some(cur), r)
        } else if let Some(r) = s.strip_prefix('$') {
            (Some(last), r)
        } else if let Some(r) = s.strip_prefix('\'') {
            let mut chars = r.chars();
            let name = chars.next().ok_or("E20: Mark not set")?;
            let name = if name == '`' { '\'' } else { name };
            let p = self.marks.get(&name).ok_or("E20: Mark not set")?;
            (Some(p.row as i64), chars.as_str())
        } else {
            (None, s)
        };
        while let Some(sign) = rest.chars().next().filter(|c| *c == '+' || *c == '-') {
            let after = &rest[1..];
            let n: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            let by = if n.is_empty() { 1 } else { n.parse::<i64>().unwrap_or(0) };
            let base = row.unwrap_or(cur);
            row = Some(if sign == '+' { base + by } else { base - by });
            rest = &after[n.len()..];
        }
        if let Some(r) = row
            && (r < -1 || r > last) {
                return Err("E16: Invalid range".into());
            }
        Ok((row, rest))
    }

    /// An optional range: `%`, `a`, or `a,b`.
    fn parse_range<'a>(&self, s: &'a str) -> Result<(Option<(usize, usize)>, &'a str), String> {
        if let Some(rest) = s.strip_prefix('%') {
            return Ok((Some((0, self.lines.len() - 1)), rest));
        }
        let (a, rest) = self.parse_addr(s)?;
        let Some(a) = a else { return Ok((None, s)) };
        let (b, rest) = match rest.strip_prefix(',') {
            Some(r) => {
                let (b, r) = self.parse_addr(r)?;
                (b.ok_or("E14: Missing address")?, r)
            }
            None => (a, rest),
        };
        let (a, b) = (a.max(0) as usize, b.max(0) as usize);
        Ok((Some((a.min(b), a.max(b))), rest))
    }

    /// Lines `a..=b` disappear: the rows `:g` still visits follow them.
    fn retarget_removed(&mut self, a: usize, b: usize) {
        if let Some(targets) = &mut self.g_targets {
            let n = b - a + 1;
            targets.retain(|&t| t < a || t > b);
            for t in targets.iter_mut() {
                if *t > b {
                    *t -= n;
                }
            }
        }
    }

    fn retarget_inserted(&mut self, at: usize, n: usize) {
        if let Some(targets) = &mut self.g_targets {
            for t in targets.iter_mut() {
                if *t >= at {
                    *t += n;
                }
            }
        }
    }

    fn ex_command(&mut self, line: &str) -> Outcome {
        self.active_reg = None;
        let cmd = line.trim_start();
        let (range, rest) = match self.parse_range(cmd) {
            Ok(r) => r,
            Err(msg) => return Outcome::Bell(msg),
        };
        let rest = rest.trim_start();
        let cur = self.cursor.row;
        if rest.trim().is_empty() {
            if let Some((_, b)) = range {
                self.marks.insert('\'', self.cursor);
                self.cursor = pos(b, self.first_non_blank(b));
                self.want = self.cursor.col;
            }
            return Outcome::Ok;
        }
        let name: String = rest.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
        let name = if name.is_empty() { rest.chars().next().map(String::from).unwrap_or_default() } else { name };
        let arg = &rest[name.len()..];
        let (a, b) = range.unwrap_or((cur, cur));
        match name.as_str() {
            "s" | "su" | "substitute" | "&" => {
                let (pat, rep, global) = if name == "&" || arg.trim().is_empty() {
                    match &self.last_sub {
                        Some((p, r, g)) => (p.clone(), r.clone(), *g && arg.contains('&')),
                        None => return Outcome::Bell("E35: No previous substitute".into()),
                    }
                } else {
                    let Some(delim) = arg.chars().next().filter(|c| !c.is_alphanumeric() && *c != ' ') else {
                        return Outcome::Bell("Usage: :s/old/new/g".into());
                    };
                    let parts: Vec<&str> = arg[delim.len_utf8()..].splitn(3, delim).collect();
                    let pat = match parts[0] {
                        "" => match &self.last_search {
                            Some(p) => p.clone(),
                            None => return Outcome::Bell("E35: No previous regular expression".into()),
                        },
                        p => p.to_string(),
                    };
                    let rep = parts.get(1).copied().unwrap_or("").to_string();
                    let global = parts.get(2).is_some_and(|f| f.contains('g'));
                    (pat, rep, global)
                };
                self.last_search = Some(pat.clone());
                self.last_sub = Some((pat.clone(), rep.clone(), global));
                self.substitute(a, b, &pat, &rep, global)
            }
            "d" | "de" | "del" | "delete" => {
                let before = self.lines.clone();
                let cursor = self.cursor;
                let text = self.span_text(Span::Lines(a, b));
                self.store(text, false);
                self.delete_span(Span::Lines(a, b));
                self.retarget_removed(a, b);
                self.push_undo(before, cursor);
                self.changed();
                Outcome::Ok
            }
            "m" | "mo" | "move" | "t" | "co" | "copy" => {
                let copy = name.starts_with('t') || name.starts_with('c');
                let dest = match self.parse_addr(arg.trim()) {
                    Ok((Some(d), r)) if r.trim().is_empty() => d,
                    Ok(_) => return Outcome::Bell("E14: Missing address".into()),
                    Err(msg) => return Outcome::Bell(msg),
                };
                if !copy && dest >= a as i64 && dest < b as i64 {
                    return Outcome::Bell("E134: Cannot move a range of lines into itself".into());
                }
                let before = self.lines.clone();
                let cursor = self.cursor;
                let block: Vec<Vec<char>> = self.lines[a..=b].to_vec();
                let n = block.len();
                let at = if copy {
                    let at = (dest + 1) as usize;
                    self.retarget_inserted(at, n);
                    at
                } else {
                    self.lines.drain(a..=b);
                    // Where the lines go, once they are out.
                    let at = if dest > b as i64 { (dest + 1) as usize - n } else { (dest + 1) as usize };
                    if let Some(targets) = &mut self.g_targets {
                        for t in targets.iter_mut() {
                            let t1 = if *t > b { *t - n } else { *t };
                            *t = if t1 >= at { t1 + n } else { t1 };
                        }
                    }
                    at
                };
                for (k, l) in block.into_iter().enumerate() {
                    self.lines.insert(at + k, l);
                }
                let row = at + n - 1;
                self.cursor = pos(row, self.first_non_blank(row));
                self.push_undo(before, cursor);
                self.changed();
                Outcome::Ok
            }
            "norm" | "normal" => {
                let keys: Vec<Key> = arg.trim_start().chars().map(Key::Char).collect();
                if keys.is_empty() {
                    return Outcome::Bell("E471: Argument required".into());
                }
                let before = self.lines.clone();
                let cursor = self.cursor;
                self.quiet += 1;
                let mut rows: Vec<usize> = (a..=b).collect();
                while !rows.is_empty() {
                    let row = rows.remove(0);
                    if row >= self.lines.len() {
                        break;
                    }
                    let len = self.lines.len();
                    self.run_normal_at(row, &keys);
                    let delta = self.lines.len() as i64 - len as i64;
                    for r in rows.iter_mut() {
                        *r = (*r as i64 + delta).max(0) as usize;
                    }
                }
                self.quiet -= 1;
                if self.lines != before {
                    self.push_undo(before, cursor);
                    self.changed();
                }
                Outcome::Ok
            }
            "g" | "global" | "v" | "vglobal" => {
                let (invert, arg) = match arg.strip_prefix('!') {
                    Some(r) => (true, r),
                    None => (false, arg),
                };
                let invert = invert ^ name.starts_with('v');
                let Some(delim) = arg.chars().next().filter(|c| !c.is_alphanumeric() && *c != ' ') else {
                    return Outcome::Bell("Usage: :g/pattern/command".into());
                };
                let parts: Vec<&str> = arg[delim.len_utf8()..].splitn(2, delim).collect();
                let pat = parts[0].to_string();
                let sub = parts.get(1).copied().unwrap_or("").trim().to_string();
                if sub.is_empty() || pat.is_empty() {
                    return Outcome::Bell("Usage: :g/pattern/command".into());
                }
                self.last_search = Some(pat.clone());
                let (a, b) = range.unwrap_or((0, self.lines.len() - 1));
                let targets: Vec<usize> = (a..=b).filter(|&r| find_all(&pat, &self.lines[r]).is_empty() == invert).collect();
                if targets.is_empty() {
                    return Outcome::Bell(format!("Pattern not found: {pat}"));
                }
                let before = self.lines.clone();
                let cursor = self.cursor;
                self.quiet += 1;
                let outer = self.g_targets.replace(targets);
                loop {
                    let Some(row) = self.g_targets.as_mut().and_then(|t| if t.is_empty() { None } else { Some(t.remove(0)) }) else { break };
                    if row >= self.lines.len() {
                        continue;
                    }
                    self.cursor = pos(row, 0);
                    let len = self.lines.len();
                    let structural = {
                        let n: String = sub.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
                        matches!(n.as_str(), "d" | "de" | "del" | "delete" | "m" | "mo" | "move" | "t" | "co" | "copy")
                    };
                    if let Outcome::Bell(msg) = self.ex_command(&sub)
                        && !msg.starts_with("Pattern not found") {
                            break;
                        }
                    if !structural {
                        let delta = self.lines.len() as i64 - len as i64;
                        if let Some(t) = &mut self.g_targets {
                            for r in t.iter_mut() {
                                if *r > row {
                                    *r = (*r as i64 + delta).max(0) as usize;
                                }
                            }
                        }
                    }
                }
                self.g_targets = outer;
                self.quiet -= 1;
                self.clamp();
                if self.lines != before {
                    self.push_undo(before, cursor);
                    self.changed();
                }
                Outcome::Ok
            }
            "sor" | "sort" => {
                let (a, b) = range.unwrap_or((0, self.lines.len() - 1));
                let before = self.lines.clone();
                let cursor = self.cursor;
                let mut part: Vec<Vec<char>> = self.lines[a..=b].to_vec();
                part.sort();
                if arg.starts_with('!') {
                    part.reverse();
                }
                if arg.contains('u') {
                    part.dedup();
                }
                self.lines.splice(a..=b, part);
                self.cursor = pos(a, 0);
                if self.lines != before {
                    self.push_undo(before, cursor);
                    self.changed();
                }
                Outcome::Ok
            }
            "j" | "join" => {
                let b = if range.is_some_and(|(x, y)| x != y) { b } else { (a + 1).min(self.lines.len() - 1) };
                if b == a {
                    return Outcome::Bell(String::new());
                }
                self.cursor = pos(a, 0);
                let keys = parse_keys(&format!("{}J", b - a + 1));
                self.run_normal_at(a, &keys);
                Outcome::Ok
            }
            ">" | "<" => {
                let before = self.lines.clone();
                let cursor = self.cursor;
                let depth = rest.chars().take_while(|c| c.to_string() == name).count();
                for _ in 0..depth {
                    self.shift_lines(a, b, name == ">");
                }
                self.cursor = pos(b, self.first_non_blank(b));
                self.push_undo(before, cursor);
                self.changed();
                Outcome::Ok
            }
            _ if range.is_none() => Outcome::Command(cmd.trim().to_string()),
            _ => Outcome::Bell(format!("E492: Not an editor command: {}", cmd.trim())),
        }
    }

    /// `:s` over rows `a..=b`, with `^` and `$` anchors and `&` for the
    /// match in the replacement.
    fn substitute(&mut self, a: usize, b: usize, pat: &str, rep: &str, global: bool) -> Outcome {
        let before = self.lines.clone();
        let cursor = self.cursor;
        let mut last = None;
        for row in a..=b.min(self.lines.len() - 1) {
            let line = self.lines[row].clone();
            let mut hits = find_all(pat, &line);
            if hits.is_empty() {
                continue;
            }
            if !global {
                hits.truncate(1);
            }
            let mut out = Vec::new();
            let mut at = 0;
            for (s, len) in hits {
                out.extend_from_slice(&line[at..s]);
                let mut chars = rep.chars().peekable();
                while let Some(c) = chars.next() {
                    match c {
                        '&' => out.extend_from_slice(&line[s..s + len]),
                        '\\' => match chars.next() {
                            Some('n') | Some('r') => out.push('\n'),
                            Some(other) => out.push(other),
                            None => out.push('\\'),
                        },
                        c => out.push(c),
                    }
                }
                at = s + len;
            }
            out.extend_from_slice(&line[at..]);
            self.lines[row] = out;
            last = Some(row);
        }
        let Some(row) = last else { return Outcome::Bell(format!("Pattern not found: {pat}")) };
        // A `\r` in the replacement splits lines.
        let mut split: Vec<Vec<char>> = Vec::new();
        let mut cursor_row = row;
        for (r, line) in std::mem::take(&mut self.lines).into_iter().enumerate() {
            let parts: Vec<Vec<char>> = line.split(|c| *c == '\n').map(|p| p.to_vec()).collect();
            if r <= row {
                cursor_row += parts.len() - 1;
            }
            split.extend(parts);
        }
        self.lines = split;
        self.push_undo(before, cursor);
        let row = cursor_row.min(self.lines.len() - 1);
        self.cursor = pos(row, self.first_non_blank(row));
        self.changed();
        Outcome::Ok
    }

    /// Runs normal-mode keys from the start of `row`, as `:normal` does,
    /// and ends any mode they leave open.
    fn run_normal_at(&mut self, row: usize, keys: &[Key]) {
        self.mode = Mode::Normal;
        self.pending.clear();
        self.cursor = pos(row, 0);
        self.want = 0;
        self.replay(keys);
        self.pending.clear();
        if self.mode != Mode::Normal {
            self.depth += 1;
            self.feed(Key::Esc);
            if self.mode != Mode::Normal {
                self.feed(Key::Esc);
            }
            self.depth -= 1;
        }
        self.mode = Mode::Normal;
        self.clamp();
    }

    fn shift_lines(&mut self, a: usize, b: usize, right: bool) {
        for row in a..=b {
            let line = &mut self.lines[row];
            if right {
                if !line.is_empty() {
                    for _ in 0..SHIFT {
                        line.insert(0, ' ');
                    }
                }
            } else {
                let n = line.iter().take(SHIFT).take_while(|c| **c == ' ').count();
                let n = if n == 0 && line.first() == Some(&'\t') { 1 } else { n };
                line.drain(..n);
            }
        }
    }

    fn normal_key(&mut self, key: Key) -> Outcome {
        if key == Key::Esc {
            let had = !self.pending.is_empty();
            self.pending.clear();
            self.recording = None;
            if self.in_visual() {
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
        let result = if self.in_visual() { self.run_visual(&keys) } else { self.run_normal(&keys) };
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
        let m = |to: Pos, kind: MotionKind| Motion { to, kind, keeps_want: true, want_end: false, jump: false };
        let jm = |to: Pos, kind: MotionKind| Motion { to, kind, keeps_want: true, want_end: false, jump: true };
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
                Motion { to: pos(row, col), kind: MotionKind::Linewise, keeps_want: false, want_end: false, jump: false }
            }
            '0' => m(pos(cur.row, 0), MotionKind::Exclusive),
            '^' => m(pos(cur.row, self.first_non_blank(cur.row)), MotionKind::Exclusive),
            '$' => {
                let row = (cur.row + n - 1).min(self.lines.len() - 1);
                Motion { to: pos(row, self.last_col(row)), kind: MotionKind::Inclusive, keeps_want: true, want_end: true, jump: false }
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
                Motion { to: pos(row, self.first_non_blank(row)), kind: MotionKind::Linewise, keeps_want: true, want_end: false, jump: true }
            }
            'g' => {
                let Some(&next) = keys.get(i + 1) else { return Err(Parse::Incomplete) };
                match next {
                    Key::Char('g') => {
                        let row = count.map_or(0, |n| n.max(1).min(self.lines.len()) - 1);
                        return Ok(Some((
                            Motion { to: pos(row, self.first_non_blank(row)), kind: MotionKind::Linewise, keeps_want: true, want_end: false, jump: true },
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
            '%' => jm(self.match_pair(cur).ok_or(Parse::Invalid)?, MotionKind::Inclusive),
            '`' | '\'' => {
                let Some(&next) = keys.get(i + 1) else { return Err(Parse::Incomplete) };
                let Key::Char(name) = next else { return Err(Parse::Invalid) };
                let name = if name == '`' { '\'' } else { name };
                let at = *self.marks.get(&name).ok_or(Parse::Invalid)?;
                let row = at.row.min(self.lines.len() - 1);
                let motion = if c == '`' {
                    jm(pos(row, at.col.min(self.last_col(row))), MotionKind::Exclusive)
                } else {
                    jm(pos(row, self.first_non_blank(row)), MotionKind::Linewise)
                };
                return Ok(Some((motion, i + 2)));
            }
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
                jm(to, MotionKind::Exclusive)
            }
            'n' | 'N' => {
                if self.last_search.is_none() {
                    return Err(Parse::Invalid);
                }
                let mut p = cur;
                for _ in 0..n {
                    p = self.search_from(p, c == 'n', false).ok_or(Parse::Invalid)?;
                }
                jm(p, MotionKind::Exclusive)
            }
            '*' | '#' => {
                let word = self.word_under(cur).ok_or(Parse::Invalid)?;
                self.last_search = Some(format!(r"\<{word}\>"));
                let mut p = cur;
                for _ in 0..n {
                    p = self.search_from(p, c == '*', true).ok_or(Parse::Invalid)?;
                }
                jm(p, MotionKind::Exclusive)
            }
            _ => return Ok(None),
        };
        Ok(Some((motion, i + 1)))
    }

    fn apply_motion(&mut self, motion: Motion) {
        if motion.jump {
            self.marks.insert('\'', self.cursor);
        }
        self.cursor = motion.to;
        if motion.keeps_want {
            self.want = if motion.want_end { usize::MAX } else { motion.to.col };
        }
        self.clamp();
    }

    /// The count, an optional `"x` register and the count after it; returns
    /// the total count and where the command itself starts.
    fn read_prefix(&mut self, keys: &[Key]) -> Result<(Option<usize>, usize), Parse> {
        let mut i = 0;
        let count = Self::read_count(keys, &mut i);
        if keys.get(i) != Some(&Key::Char('"')) {
            self.active_reg = None;
            return Ok((count, i));
        }
        let Some(&r) = keys.get(i + 1) else { return Err(Parse::Incomplete) };
        let Key::Char(r) = r else { return Err(Parse::Invalid) };
        if !Self::valid_register(r) {
            return Err(Parse::Invalid);
        }
        self.active_reg = Some(r);
        i += 2;
        let count2 = Self::read_count(keys, &mut i);
        Ok((mul_counts(count, count2), i))
    }

    fn run_normal(&mut self, keys: &[Key]) -> Result<Outcome, Parse> {
        let (count, i) = self.read_prefix(keys)?;
        let Some(&key) = keys.get(i) else { return Err(Parse::Incomplete) };
        let n = count.unwrap_or(1);
        match key {
            Key::Char(op @ ('d' | 'c' | 'y' | '>' | '<')) => return self.run_operator(keys, i + 1, count, op),
            Key::Char('g') => match keys.get(i + 1) {
                Some(&Key::Char(op @ ('u' | 'U' | '~'))) => return self.run_operator(keys, i + 2, count, op),
                Some(&Key::Char(d @ ('n' | 'N'))) => {
                    if keys.len() > i + 2 {
                        return Err(Parse::Invalid);
                    }
                    let (s, e) = self.match_at_or_after(d == 'n').ok_or(Parse::Invalid)?;
                    self.recording = None;
                    self.mode = Mode::Visual { line: false };
                    self.anchor = s;
                    self.cursor = if e > s { self.before(e) } else { s };
                    return Ok(Outcome::Ok);
                }
                Some(&Key::Char('&')) => {
                    if keys.len() > i + 2 {
                        return Err(Parse::Invalid);
                    }
                    let Some((pat, rep, global)) = self.last_sub.clone() else {
                        return Ok(Outcome::Bell("No previous substitute".into()));
                    };
                    return Ok(self.substitute(0, self.lines.len() - 1, &pat, &rep, global));
                }
                _ => {}
            },
            _ => {}
        }
        if let Some((motion, end)) = self.parse_motion(keys, i, count)? {
            if end != keys.len() {
                return Err(Parse::Invalid);
            }
            self.recording = None;
            self.apply_motion(motion);
            return Ok(Outcome::Ok);
        }
        let Key::Char(c) = key else {
            return match key {
                Key::CtrlR => {
                    self.recording = None;
                    Ok(self.redo_change(n))
                }
                Key::CtrlA => Ok(self.increment(n as i64)),
                Key::CtrlX => Ok(self.increment(-(n as i64))),
                Key::CtrlV => {
                    self.recording = None;
                    self.mode = Mode::Block;
                    self.anchor = self.cursor;
                    Ok(Outcome::Ok)
                }
                _ => Err(Parse::Invalid),
            };
        };
        if c == 'q' && self.macro_rec.is_some() {
            if keys.len() > i + 1 {
                return Err(Parse::Invalid);
            }
            let (reg, mut rec) = self.macro_rec.take().unwrap();
            rec.pop();
            let text: String = rec.iter().map(|k| k.raw()).collect();
            if reg == '"' {
                self.register = Register { text, linewise: false };
            } else if reg.is_ascii_uppercase() {
                self.registers.entry(reg.to_ascii_lowercase()).or_default().text.push_str(&text);
            } else {
                self.registers.insert(reg, Register { text, linewise: false });
            }
            self.recording = None;
            return Ok(Outcome::Ok);
        }
        if matches!(c, 'r' | 'm' | 'q' | '@') {
            let Some(&next) = keys.get(i + 1) else { return Err(Parse::Incomplete) };
            let Key::Char(arg) = next else { return Err(Parse::Invalid) };
            if keys.len() > i + 2 {
                return Err(Parse::Invalid);
            }
            return self.char_command(c, arg, n);
        }
        if keys.len() > i + 1 {
            return Err(Parse::Invalid);
        }
        match c {
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
                let reg = self.read_reg();
                if reg.text.is_empty() && !reg.linewise {
                    return Ok(Outcome::Bell("Nothing in that register yet".into()));
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
                self.depth += 1;
                self.snapshot_force();
                for k in change.clone() {
                    self.feed(k);
                }
                if matches!(self.mode, Mode::Insert | Mode::Replace) {
                    self.feed(Key::Esc);
                }
                self.depth -= 1;
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
            '&' => {
                let Some((pat, rep, _)) = self.last_sub.clone() else {
                    return Ok(Outcome::Bell("No previous substitute".into()));
                };
                let row = self.cursor.row;
                Ok(self.substitute(row, row, &pat, &rep, false))
            }
            'R' => {
                self.snapshot();
                self.replaced.clear();
                self.mode = Mode::Replace;
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

    /// `r`, `m`, `q` and `@`, with the character after them.
    fn char_command(&mut self, c: char, arg: char, n: usize) -> Result<Outcome, Parse> {
        match c {
            'r' => {
                let row = self.cursor.row;
                if self.cursor.col + n > self.line_len(row) {
                    return Err(Parse::Invalid);
                }
                self.snapshot();
                for col in self.cursor.col..self.cursor.col + n {
                    self.lines[row][col] = arg;
                }
                self.cursor.col += n - 1;
                self.changed();
                Ok(Outcome::Ok)
            }
            'm' => {
                if !arg.is_ascii_alphabetic() && arg != '\'' && arg != '`' {
                    return Err(Parse::Invalid);
                }
                self.recording = None;
                let name = if arg == '`' { '\'' } else { arg };
                self.marks.insert(name, self.cursor);
                Ok(Outcome::Ok)
            }
            'q' => {
                if !arg.is_ascii_alphanumeric() && arg != '"' {
                    return Err(Parse::Invalid);
                }
                self.recording = None;
                if arg.is_ascii_lowercase() || arg.is_ascii_digit() {
                    self.registers.remove(&arg);
                }
                self.macro_rec = Some((arg, Vec::new()));
                Ok(Outcome::Ok)
            }
            _ => {
                let reg = if arg == '@' { self.last_macro.ok_or(Parse::Invalid)? } else { arg };
                if !Self::valid_register(reg) {
                    return Err(Parse::Invalid);
                }
                self.recording = None;
                let text = self.register_text(reg).unwrap_or_default();
                if text.is_empty() {
                    return Ok(Outcome::Bell(format!("Register {reg} is empty")));
                }
                if self.depth > 60 {
                    return Ok(Outcome::Bell("Macro nested too deep".into()));
                }
                self.last_macro = Some(reg);
                self.pending.clear();
                let keys: Vec<Key> = text.chars().map(Key::from_raw).collect();
                for _ in 0..n {
                    if !self.replay(&keys) {
                        break;
                    }
                }
                Ok(Outcome::Ok)
            }
        }
    }

    /// An operator (`d c y > <`, or `gu gU g~` as `u U ~`) from `keys[j..]`:
    /// doubled for whole lines, then a text object, `gn`, or a motion.
    fn run_operator(&mut self, keys: &[Key], j: usize, count: Option<usize>, op: char) -> Result<Outcome, Parse> {
        let mut k = j;
        let count2 = Self::read_count(keys, &mut k);
        let Some(&next) = keys.get(k) else { return Err(Parse::Incomplete) };
        let total = mul_counts(count, count2);
        let tn = total.unwrap_or(1);
        let case_op = matches!(op, 'u' | 'U' | '~');
        let doubled = if next == Key::Char(op) {
            Some(k + 1)
        } else if case_op && next == Key::Char('g') {
            match keys.get(k + 1) {
                None => return Err(Parse::Incomplete),
                Some(&n2) if n2 == Key::Char(op) => Some(k + 2),
                _ => None,
            }
        } else {
            None
        };
        let span = if let Some(end) = doubled {
            if end != keys.len() {
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
            self.object_span(next == Key::Char('i'), obj).ok_or(Parse::Invalid)?
        } else if next == Key::Char('g') && matches!(keys.get(k + 1), Some(Key::Char('n' | 'N'))) {
            if k + 2 != keys.len() {
                return Err(Parse::Invalid);
            }
            let (s, e) = self.match_at_or_after(keys[k + 1] == Key::Char('n')).ok_or(Parse::Invalid)?;
            Span::Chars(s, e)
        } else {
            let Some((motion, end)) = self.parse_motion(keys, k, total)? else { return Err(Parse::Invalid) };
            if end != keys.len() {
                return Err(Parse::Invalid);
            }
            self.motion_span(op, motion, matches!(keys.get(k), Some(Key::Char('w' | 'W'))))
        };
        Ok(self.operate(op, span))
    }

    /// Ctrl-a / Ctrl-x: adds `by` to the number at or after the cursor.
    fn increment(&mut self, by: i64) -> Outcome {
        let row = self.cursor.row;
        let line = &self.lines[row];
        let mut col = self.cursor.col;
        if line.get(col).is_some_and(|c| c.is_ascii_digit()) {
            while col > 0 && line[col - 1].is_ascii_digit() {
                col -= 1;
            }
        } else {
            match (col..line.len()).find(|&k| line[k].is_ascii_digit()) {
                Some(k) => col = k,
                None => return Outcome::Bell("No number under or after the cursor".into()),
            }
        }
        let mut end = col;
        while end < line.len() && line[end].is_ascii_digit() {
            end += 1;
        }
        let negative = col > 0 && line[col - 1] == '-';
        let start = if negative { col - 1 } else { col };
        let digits: String = line[col..end].iter().collect();
        let value = digits.parse::<i64>().unwrap_or(0) * if negative { -1 } else { 1 };
        let new: Vec<char> = (value + by).to_string().chars().collect();
        self.snapshot();
        let len = new.len();
        self.lines[row].splice(start..end, new);
        self.cursor.col = start + len - 1;
        self.want = self.cursor.col;
        self.changed();
        Outcome::Ok
    }

    fn snapshot_force(&mut self) {
        if self.quiet > 0 {
            return;
        }
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
        let (count, i) = self.read_prefix(keys)?;
        let Some(&key) = keys.get(i) else { return Err(Parse::Incomplete) };
        let (line, block) = match self.mode {
            Mode::Visual { line } => (line, false),
            Mode::Block => (false, true),
            _ => return Err(Parse::Invalid),
        };
        if key == Key::CtrlV {
            self.mode = if block { Mode::Normal } else { Mode::Block };
            return Ok(Outcome::Ok);
        }
        if matches!(key, Key::Char('i' | 'a')) {
            let Some(&obj) = keys.get(i + 1) else { return Err(Parse::Incomplete) };
            let Key::Char(obj) = obj else { return Err(Parse::Invalid) };
            match self.object_span(key == Key::Char('i'), obj).ok_or(Parse::Invalid)? {
                Span::Lines(a, b) => {
                    self.mode = Mode::Visual { line: true };
                    self.anchor = pos(a, 0);
                    self.cursor = pos(b, 0);
                }
                Span::Chars(s, e) => {
                    if e <= s {
                        return Ok(Outcome::Bell(String::new()));
                    }
                    self.anchor = s;
                    self.cursor = self.before(e);
                }
            }
            return Ok(Outcome::Ok);
        }
        if key == Key::Char('g') && matches!(keys.get(i + 1), Some(Key::Char('n' | 'N'))) {
            let (_, e) = self.match_at_or_after(keys[i + 1] == Key::Char('n')).ok_or(Parse::Invalid)?;
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
        let (s, e) = if self.anchor <= self.cursor { (self.anchor, self.cursor) } else { (self.cursor, self.anchor) };
        let cells = self.visual_cells();
        if c == 'r' {
            let Some(&next) = keys.get(i + 1) else { return Err(Parse::Incomplete) };
            let Key::Char(with) = next else { return Err(Parse::Invalid) };
            if keys.len() > i + 2 {
                return Err(Parse::Invalid);
            }
            self.snapshot();
            for &(row, a, b) in &cells {
                for col in a..b {
                    self.lines[row][col] = with;
                }
            }
            self.mode = Mode::Normal;
            self.cursor = cells.first().map_or(s, |&(row, a, _)| pos(row, a));
            self.changed();
            self.clamp();
            return Ok(Outcome::Ok);
        }
        if keys.len() > i + 1 {
            return Err(Parse::Invalid);
        }
        match c {
            'o' => {
                std::mem::swap(&mut self.anchor, &mut self.cursor);
                return Ok(Outcome::Ok);
            }
            '~' | 'u' | 'U' => {
                self.mode = Mode::Normal;
                self.snapshot();
                self.recase(&cells, c);
                self.cursor = cells.first().map_or(s, |&(row, a, _)| pos(row, a));
                self.changed();
                self.clamp();
                return Ok(Outcome::Ok);
            }
            '>' | '<' => {
                self.mode = Mode::Normal;
                let (a, b) = (s.row, e.row);
                return Ok(self.operate(c, Span::Lines(a, b)));
            }
            ':' => {
                self.marks.insert('<', pos(s.row, 0));
                self.marks.insert('>', pos(e.row, 0));
                self.recording = None;
                self.mode = Mode::Command { search: false };
                self.cmdline = "'<,'>".into();
                return Ok(Outcome::Ok);
            }
            _ => {}
        }
        if block {
            return self.block_command(c);
        }
        let span = if line {
            Span::Lines(s.row, e.row)
        } else {
            Span::Chars(s, self.after(e))
        };
        match c {
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

    /// The selected columns of each row: `(row, from, to)`.
    fn visual_cells(&self) -> Vec<(usize, usize, usize)> {
        match self.mode {
            Mode::Block => {
                let (r0, r1, c0, c1, to_end) = self.block_rect();
                (r0..=r1)
                    .map(|r| {
                        let len = self.line_len(r);
                        (r, c0.min(len), if to_end { len } else { (c1 + 1).min(len) })
                    })
                    .collect()
            }
            Mode::Visual { line } => {
                let (s, e) = if self.anchor <= self.cursor { (self.anchor, self.cursor) } else { (self.cursor, self.anchor) };
                (s.row..=e.row)
                    .map(|r| {
                        let len = self.line_len(r);
                        let from = if line || r != s.row { 0 } else { s.col.min(len) };
                        let to = if line || r != e.row { len } else { (e.col + 1).min(len) };
                        (r, from, to.max(from))
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn recase(&mut self, cells: &[(usize, usize, usize)], how: char) {
        for &(row, a, b) in cells {
            for col in a..b.min(self.line_len(row)) {
                let ch = self.lines[row][col];
                self.lines[row][col] = match how {
                    'u' => ch.to_lowercase().next().unwrap_or(ch),
                    'U' => ch.to_uppercase().next().unwrap_or(ch),
                    _ => toggle_case(ch),
                };
            }
        }
    }

    fn block_command(&mut self, c: char) -> Result<Outcome, Parse> {
        let (r0, r1, c0, c1, to_end) = self.block_rect();
        let cells = self.visual_cells();
        match c {
            'O' => {
                std::mem::swap(&mut self.anchor.col, &mut self.cursor.col);
            }
            'v' | 'V' => self.mode = Mode::Visual { line: c == 'V' },
            'd' | 'x' | 'y' | 'c' | 's' => {
                let text = cells
                    .iter()
                    .map(|&(r, a, b)| self.lines[r][a..b].iter().collect::<String>())
                    .collect::<Vec<_>>()
                    .join("\n");
                self.store(Register { text, linewise: false }, c == 'y');
                self.mode = Mode::Normal;
                if c != 'y' {
                    self.snapshot();
                    for &(r, a, b) in &cells {
                        self.lines[r].drain(a..b);
                    }
                    self.changed();
                }
                if c == 'c' || c == 's' {
                    let start = c0.min(self.line_len(r0));
                    self.mode = Mode::Insert;
                    self.cursor = pos(r0, start);
                    self.block_insert = Some(BlockInsert { row: r0, start, rows: r0 + 1..=r1, col: c0, to_end: false, pad: false });
                } else {
                    self.cursor = pos(r0, c0);
                    self.clamp();
                    self.want = self.cursor.col;
                }
            }
            'I' | 'A' => {
                self.snapshot();
                let append_end = c == 'A' && to_end;
                let col = if c == 'I' { c0 } else { c1 + 1 };
                let start = if append_end {
                    self.line_len(r0)
                } else {
                    if c == 'A' && self.line_len(r0) < col {
                        self.lines[r0].resize(col, ' ');
                    }
                    col.min(self.line_len(r0))
                };
                self.mode = Mode::Insert;
                self.cursor = pos(r0, start);
                self.block_insert = Some(BlockInsert { row: r0, start, rows: r0 + 1..=r1, col, to_end: append_end, pad: c == 'A' });
            }
            _ => return Err(Parse::Invalid),
        }
        Ok(Outcome::Ok)
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
            '>' | '<' => {
                let (a, b) = match span {
                    Span::Lines(a, b) => (a, b),
                    Span::Chars(s, e) => (s.row, if e.col == 0 && e.row > s.row { e.row - 1 } else { e.row }),
                };
                self.snapshot();
                self.shift_lines(a, b, op == '>');
                self.cursor = pos(a, self.first_non_blank(a));
                self.want = self.cursor.col;
                self.changed();
                Outcome::Ok
            }
            'u' | 'U' | '~' => {
                self.snapshot();
                let cells = self.span_cells(span);
                self.recase(&cells, op);
                self.cursor = match span {
                    Span::Lines(a, _) => pos(a, self.cursor.col),
                    Span::Chars(s, _) => s,
                };
                self.clamp();
                self.want = self.cursor.col;
                self.changed();
                Outcome::Ok
            }
            'y' => {
                self.store(text, true);
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
                self.store(text, false);
                self.delete_span(span);
                self.changed();
                self.mode = Mode::Normal;
                self.clamp();
                self.want = self.cursor.col;
                Outcome::Ok
            }
            _ => {
                self.snapshot();
                self.store(text, false);
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

    fn span_cells(&self, span: Span) -> Vec<(usize, usize, usize)> {
        match span {
            Span::Lines(a, b) => (a..=b).map(|r| (r, 0, self.line_len(r))).collect(),
            Span::Chars(s, e) => (s.row..=e.row)
                .map(|r| {
                    let len = self.line_len(r);
                    let from = if r == s.row { s.col.min(len) } else { 0 };
                    let to = if r == e.row { e.col.min(len) } else { len };
                    (r, from, to.max(from))
                })
                .collect(),
        }
    }

    fn put(&mut self, after: bool, n: usize) {
        let reg = self.read_reg();
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

    /// Every match of `pat` in the buffer, overlapping as vim's search.
    fn all_hits(&self, pat: &str, whole_word: bool) -> Vec<(Pos, usize)> {
        let mut hits = Vec::new();
        for (row, line) in self.lines.iter().enumerate() {
            for col in 0..=line.len() {
                if let Some(len) = match_at(pat, line, col) {
                    if whole_word {
                        let before = col > 0 && class(line[col - 1], false) == 2;
                        let after = col + len < line.len() && class(line[col + len], false) == 2;
                        if before || after {
                            continue;
                        }
                    }
                    hits.push((pos(row, col), len));
                }
            }
        }
        hits
    }

    /// The match `gn` picks: the one under the cursor, else the next.
    fn match_at_or_after(&self, forward: bool) -> Option<(Pos, Pos)> {
        let pat = self.last_search.clone()?;
        let hits = self.all_hits(&pat, false);
        let cur = self.cursor;
        let under = hits.iter().find(|(p, len)| p.row == cur.row && p.col <= cur.col && cur.col < p.col + (*len).max(1));
        let (at, len) = match under {
            Some(h) => *h,
            None if forward => *hits.iter().find(|(p, _)| *p > cur).or(hits.first())?,
            None => *hits.iter().rev().find(|(p, _)| *p < cur).or(hits.last())?,
        };
        Some((at, pos(at.row, at.col + len)))
    }

    fn search_from(&self, from: Pos, forward: bool, whole_word: bool) -> Option<Pos> {
        let pat = self.last_search.as_ref()?;
        if pat.is_empty() {
            return None;
        }
        let hits: Vec<Pos> = self.all_hits(pat, whole_word).into_iter().map(|(p, _)| p).collect();
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

    fn object_span(&self, inner: bool, obj: char) -> Option<Span> {
        match obj {
            'p' => self.paragraph_object(inner).map(|(a, b)| Span::Lines(a, b)),
            't' => self.tag_object(inner).map(|(s, e)| Span::Chars(s, e)),
            _ => self.text_object(inner, obj).map(|(s, e)| Span::Chars(s, e)),
        }
    }

    /// `ip` / `ap`: the paragraph (or run of blank lines) around the
    /// cursor; `ap` takes the blank lines after it too.
    fn paragraph_object(&self, inner: bool) -> Option<(usize, usize)> {
        let blank = |r: usize| self.lines[r].iter().all(|c| c.is_whitespace());
        let row = self.cursor.row;
        let on_blank = blank(row);
        let (mut a, mut b) = (row, row);
        while a > 0 && blank(a - 1) == on_blank {
            a -= 1;
        }
        while b + 1 < self.lines.len() && blank(b + 1) == on_blank {
            b += 1;
        }
        if !inner {
            let mut f = b;
            while f + 1 < self.lines.len() && blank(f + 1) != on_blank {
                f += 1;
                if !on_blank && !blank(f) {
                    f -= 1;
                    break;
                }
            }
            if f > b {
                b = f;
            } else if !on_blank {
                while a > 0 && blank(a - 1) {
                    a -= 1;
                }
            }
        }
        Some((a, b))
    }

    /// `it` / `at`: inside, or around, the innermost `<tag>...</tag>`.
    fn tag_object(&self, inner: bool) -> Option<(Pos, Pos)> {
        let text: Vec<char> = self.text().chars().collect();
        let mut starts = Vec::with_capacity(self.lines.len());
        let mut at = 0;
        for l in &self.lines {
            starts.push(at);
            at += l.len() + 1;
        }
        let to_pos = |k: usize| {
            let row = starts.iter().rposition(|&s| s <= k).unwrap_or(0);
            pos(row, k - starts[row])
        };
        let cur = starts[self.cursor.row] + self.cursor.col;
        let mut open: Vec<(String, usize, usize)> = Vec::new();
        let mut pairs: Vec<(usize, usize, usize, usize)> = Vec::new();
        let mut k = 0;
        while k < text.len() {
            if text[k] == '<'
                && let Some(len) = text[k..].iter().position(|&c| c == '>') {
                    let end = k + len + 1;
                    let body: String = text[k + 1..end - 1].iter().collect();
                    if let Some(name) = body.strip_prefix('/') {
                        if let Some(i) = open.iter().rposition(|(n, _, _)| n == name.trim()) {
                            let (_, os, oe) = open[i].clone();
                            open.truncate(i);
                            pairs.push((os, oe, k, end));
                        }
                    } else if !body.ends_with('/') {
                        let name: String = body.chars().take_while(|c| !c.is_whitespace()).collect();
                        if !name.is_empty() {
                            open.push((name, k, end));
                        }
                    }
                    k = end;
                    continue;
                }
            k += 1;
        }
        let &(os, oe, cs, ce) = pairs.iter().filter(|p| p.0 <= cur && cur < p.3).min_by_key(|p| p.3 - p.0)?;
        if inner { Some((to_pos(oe), to_pos(cs))) } else { Some((to_pos(os), to_pos(ce))) }
    }

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

fn mul_counts(a: Option<usize>, b: Option<usize>) -> Option<usize> {
    match (a, b) {
        (None, None) => None,
        (a, b) => Some((a.unwrap_or(1) * b.unwrap_or(1)).min(9999)),
    }
}

/// Whether `pat` matches `line` at `col`, and how long the match is. A
/// pattern is literal text, anchored by a leading `^` or a trailing `$`.
pub fn match_at(pat: &str, line: &[char], col: usize) -> Option<usize> {
    let (start, body) = match pat.strip_prefix('^') {
        Some(b) => (true, b),
        None => (false, pat),
    };
    let (end, body) = match body.strip_suffix('$') {
        Some(b) if !b.ends_with('\\') => (true, b),
        _ => (false, body),
    };
    // `\<` and `\>`: the match starts, or ends, a word (as `*` searches).
    let (word_start, body) = match body.strip_prefix(r"\<") {
        Some(b) => (true, b),
        None => (false, body),
    };
    let (word_end, body) = match body.strip_suffix(r"\>") {
        Some(b) => (true, b),
        None => (false, body),
    };
    if start && col != 0 {
        return None;
    }
    let is_word = |c: char| class(c, false) == 2;
    if word_start && col > 0 && line.get(col - 1).is_some_and(|c| is_word(*c)) {
        return None;
    }
    let mut want: Vec<char> = Vec::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            want.push(chars.next().unwrap_or('\\'));
        } else {
            want.push(c);
        }
    }
    if col + want.len() > line.len() || line[col..col + want.len()] != want[..] {
        return None;
    }
    if end && col + want.len() != line.len() {
        return None;
    }
    if word_end && line.get(col + want.len()).is_some_and(|c| is_word(*c)) {
        return None;
    }
    Some(want.len())
}

/// The non-overlapping matches of `pat` in `line`, as `:s` and `:g` see them.
pub fn find_all(pat: &str, line: &[char]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut col = 0;
    while col <= line.len() {
        match match_at(pat, line, col) {
            Some(len) => {
                out.push((col, len));
                col += len.max(1);
            }
            None => col += 1,
        }
    }
    out
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
    fn registers() {
        assert_eq!(run("a\nb", pos(0, 0), "\"ayyjdd\"ap").text(), "a\na");
        assert_eq!(run("one two", pos(0, 0), "\"ayw\"bywwdw\"aP").text(), "oneone  ");
        // A delete doesn't clobber "0, so the yank survives.
        assert_eq!(run("keep\nzap\nx", pos(0, 0), "yyjddj\"0p").text(), "keep\nx\nkeep");
        // The black hole swallows the delete.
        assert_eq!(run("keep\nzap", pos(0, 0), "yyj\"_ddp").text(), "keep\nkeep");
        // Upper case appends.
        assert_eq!(run("a\nb\nc", pos(0, 0), "\"ayyj\"Ayyj\"ap").text(), "a\nb\nc\na\nb");
    }

    #[test]
    fn marks() {
        assert_eq!(run("a\nb\nc\nd", pos(0, 0), "majjj`a").cursor, pos(0, 0));
        assert_eq!(run("ab\ncd\nef", pos(0, 1), "majj'a").cursor, pos(0, 0));
        assert_eq!(run("a\nb\nc\nd", pos(1, 0), "G``").cursor, pos(1, 0));
        assert_eq!(run("a\nb\nc\nd", pos(1, 0), "majjd'a").text(), "a");
    }

    #[test]
    fn increment() {
        assert_eq!(run("x = 7", pos(0, 0), "<C-a>").text(), "x = 8");
        assert_eq!(run("x = 7", pos(0, 0), "5<C-x>").text(), "x = 2");
        assert_eq!(run("x = 7", pos(0, 0), "10<C-x>").text(), "x = -3");
        assert_eq!(run("a -5 b", pos(0, 0), "6<C-a>").text(), "a 1 b");
        assert_eq!(run("v9", pos(0, 0), "<C-a>").cursor, pos(0, 2));
        assert_eq!(run("1\n1", pos(0, 0), "<C-a>j.").text(), "2\n2");
    }

    #[test]
    fn macros() {
        assert_eq!(run("a\nb\nc", pos(0, 0), "qaA;<Esc>jq2@a").text(), "a;\nb;\nc;");
        // A failing motion ends the macro, so a big count is safe.
        assert_eq!(run("1\n1\n1\n1", pos(0, 0), "qa<C-a>jq99@a").text(), "2\n2\n2\n2");
        assert_eq!(run("a\nb\nc", pos(0, 0), "qaI-<Esc>jq@a@@").text(), "-a\n-b\n-c");
        let e = run("x", pos(0, 0), "qaq");
        assert_eq!(e.register_text('a').as_deref(), Some(""));
    }

    #[test]
    fn indent_and_case_operators() {
        assert_eq!(run("a\nb", pos(0, 0), ">>").text(), "    a\nb");
        assert_eq!(run("a\nb", pos(0, 0), ">j").text(), "    a\n    b");
        assert_eq!(run("      a", pos(0, 0), "<<").text(), "  a");
        assert_eq!(run("a\n\nb", pos(0, 0), "3>>").text(), "    a\n\n    b");
        assert_eq!(run("hello world", pos(0, 0), "gUiw").text(), "HELLO world");
        assert_eq!(run("Hello World", pos(0, 0), "guu").text(), "hello world");
        assert_eq!(run("Hello World", pos(0, 0), "gUgU").text(), "HELLO WORLD");
        assert_eq!(run("abc def", pos(0, 0), "g~w").text(), "ABC def");
        assert_eq!(run("a\nb\nc", pos(0, 0), "Vj>").text(), "    a\n    b\nc");
    }

    #[test]
    fn replace_mode() {
        assert_eq!(run("abcdef", pos(0, 1), "RXY<Esc>").text(), "aXYdef");
        assert_eq!(run("ab", pos(0, 1), "RXYZ<Esc>").text(), "aXYZ");
        assert_eq!(run("abcd", pos(0, 0), "RXY<BS><BS>Z<Esc>").text(), "Zbcd");
    }

    #[test]
    fn visual_block() {
        let t = "abc\ndef\nghi";
        assert_eq!(run(t, pos(0, 0), "<C-v>jjd").text(), "bc\nef\nhi");
        assert_eq!(run(t, pos(0, 0), "<C-v>jjI# <Esc>").text(), "# abc\n# def\n# ghi");
        assert_eq!(run("a\nbbb\ncc", pos(0, 0), "<C-v>jj$A;<Esc>").text(), "a;\nbbb;\ncc;");
        assert_eq!(run(t, pos(0, 1), "<C-v>jlcX<Esc>").text(), "aX\ndX\nghi");
        assert_eq!(run(t, pos(0, 0), "<C-v>jlrx").text(), "xxc\nxxf\nghi");
        assert_eq!(run(t, pos(0, 0), "<C-v>jU").text(), "Abc\nDef\nghi");
        assert_eq!(run("ab\nc\nde", pos(0, 1), "<C-v>jjAX<Esc>").text(), "abX\nc X\ndeX");
    }

    #[test]
    fn search_next_match_operator() {
        let e = run("foo bar foo baz foo", pos(0, 0), "/foo<CR>cgnqux<Esc>..");
        assert_eq!(e.text(), "qux bar qux baz qux");
        assert_eq!(run("a foo b", pos(0, 0), "*").text(), "a foo b");
        assert_eq!(run("x foo y foo", pos(0, 2), "*Ncgnbar<Esc>").text(), "x bar y foo");
        assert_eq!(run("a foo b", pos(0, 0), "/foo<CR>0dgn").text(), "a  b");
    }

    #[test]
    fn paragraph_and_tag_objects() {
        let t = "a\nb\n\nc\nd";
        assert_eq!(run(t, pos(0, 0), "dip").text(), "\nc\nd");
        assert_eq!(run(t, pos(0, 0), "dap").text(), "c\nd");
        assert_eq!(run(t, pos(4, 0), "dap").text(), "a\nb");
        assert_eq!(run(t, pos(3, 0), "yipggP").text(), "c\nd\na\nb\n\nc\nd");
        let h = "<p><b>bold</b> text</p>";
        assert_eq!(run(h, pos(0, 7), "dit").text(), "<p><b></b> text</p>");
        assert_eq!(run(h, pos(0, 16), "cithi<Esc>").text(), "<p>hi</p>");
        assert_eq!(run(h, pos(0, 7), "dat").text(), "<p> text</p>");
        assert_eq!(run("<a>\n  x\n</a>", pos(1, 2), "dit").text(), "<a></a>");
    }

    #[test]
    fn ex_ranges_and_commands() {
        let t = "a\nb\nc\nd";
        assert_eq!(run(t, pos(0, 0), ":2,3d<CR>").text(), "a\nd");
        assert_eq!(run(t, pos(0, 0), ":m$<CR>").text(), "b\nc\nd\na");
        assert_eq!(run(t, pos(0, 0), ":3m0<CR>").text(), "c\na\nb\nd");
        assert_eq!(run(t, pos(0, 0), ":1,2t$<CR>").text(), "a\nb\nc\nd\na\nb");
        assert_eq!(run(t, pos(0, 0), ":t.<CR>").text(), "a\na\nb\nc\nd");
        assert_eq!(run(t, pos(0, 0), ":g/^/m0<CR>").text(), "d\nc\nb\na");
        assert_eq!(run("x1\ny\nx2\nz", pos(0, 0), ":g/x/d<CR>").text(), "y\nz");
        assert_eq!(run("x1\ny\nx2\nz", pos(0, 0), ":v/x/d<CR>").text(), "x1\nx2");
        assert_eq!(run("x1\ny\nx2", pos(0, 0), ":g/x/norm A!<CR>").text(), "x1!\ny\nx2!");
        assert_eq!(run("a\nb", pos(0, 0), ":%norm I- <CR>").text(), "- a\n- b");
        assert_eq!(run("a\nb\nc", pos(0, 0), ":%s/^/> /<CR>").text(), "> a\n> b\n> c");
        assert_eq!(run("a,b", pos(0, 0), ":s/,/\\r/<CR>").text(), "a\nb");
        assert_eq!(run("b\nc\na", pos(0, 0), ":sort<CR>").text(), "a\nb\nc");
        assert_eq!(run("a\nb\nc\nd", pos(0, 0), "Vj:s/$/;/<CR>").text(), "a;\nb;\nc\nd");
        assert_eq!(run("a\nb\nc\nd", pos(0, 0), "jma:'a,$d<CR>").text(), "a");
        assert_eq!(run("cat cat\ncat", pos(0, 0), ":s/cat/dog/<CR>j&").text(), "dog cat\ndog");
        assert_eq!(run("cat cat\ncat", pos(0, 0), ":s/cat/dog/g<CR>g&").text(), "dog dog\ndog");
        assert_eq!(run("a\nb\nc", pos(0, 0), ":.,+1j<CR>").text(), "a b\nc");
        assert_eq!(run("a\nb", pos(0, 0), ":%><CR>").text(), "    a\n    b");
        // One undo step for the whole command.
        assert_eq!(run("x1\ny\nx2", pos(0, 0), ":g/x/d<CR>u").text(), "x1\ny\nx2");
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
