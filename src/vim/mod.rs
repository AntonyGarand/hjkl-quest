//! A small, self-contained vim emulator: the buffer, the cursor, the modes
//! and the commands the campaign teaches. No UI here, so it is tested with
//! plain `cargo test`.
//!
//! Everything happens on one [`Editor`]. Its methods are spread over the
//! submodules by concern: keys come in through `feed`, go to the mode they
//! are typed in (`normal`, `insert`, `visual`, `cmdline`), and end up as
//! motions, operators or ex commands.

mod block;
mod buffer;
mod changes;
mod cmdline;
mod ex;
mod ex_files;
mod ex_lines;
mod feed;
mod insert;
mod jumps;
mod keys;
mod motion;
mod normal;
mod objects;
mod operator;
mod options;
mod prose;
mod registers;
mod search;
mod substitute;
mod undo;
mod view;
mod visual;
mod words;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap};

pub use keys::{Key, format_keys, parse_keys};
pub use options::Options;
pub use search::{find_all, find_matches, match_at};

use block::BlockInsert;
use substitute::ConfirmSub;
use undo::Snapshot;

/// A place in the buffer: a row and a column, both from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Pos {
    pub row: usize,
    pub col: usize,
}

/// Shorthand for a [`Pos`].
pub const fn pos(row: usize, col: usize) -> Pos {
    Pos { row, col }
}

/// The mode the editor is in: it decides what a key does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    /// Visual mode (`v`), or visual line mode (`V`).
    Visual {
        line: bool,
    },
    /// Visual block mode (`Ctrl-v`): a rectangle of columns.
    Block,
    /// Replace mode (`R`): typing overwrites.
    Replace,
    /// The command line: `:`, or a search (`/`, `?`).
    Command {
        search: bool,
    },
    /// `:s///c`: asking whether to replace each match.
    Confirm,
}

/// What a key did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The key was taken.
    Ok,
    /// The key did nothing useful (a bell in vim).
    Bell(String),
    /// A `:` command the editor does not know: for the game to handle.
    Command(String),
    /// Something to tell the player (Ctrl-g, `:!ls`, a prompt).
    Info(String),
}

/// The text in a register, and whether it holds whole lines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Register {
    pub text: String,
    pub linewise: bool,
}

/// The name of the isle's own text, as `:e` and `:ls` see it.
pub const ISLE: &str = "isle.txt";

/// shiftwidth, for `>` and `<`.
pub const SHIFT: usize = 4;

/// Why the keys typed so far are not a command yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Parse {
    /// More keys may make it one.
    Incomplete,
    /// No more keys will.
    Invalid,
}

/// The editor: the buffer, the cursor, the mode, and all the state vim
/// keeps between commands (registers, marks, undo, searches...).
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
    /// The last `:s`: its pattern, replacement and whether it was global.
    last_sub: Option<(String, String, bool)>,
    /// Rows `:g` still has to visit, kept up to date as lines move.
    g_targets: Option<Vec<usize>>,
    /// The last search went backward (`?`, `#`): `n` keeps that way.
    pub search_back: bool,
    /// The jump list (Ctrl-o, Ctrl-i) and where we are in it.
    jumps: Vec<Pos>,
    jump_at: usize,
    /// The line `U` restores: its row and its text before the latest changes.
    line_undo: Option<(usize, Vec<char>)>,
    /// A small file system for `:w`, `:r` and the shell.
    pub files: BTreeMap<String, String>,
    /// The file being edited (`:e`): the isle itself, or one of `files`.
    pub buffer_name: String,
    /// The file `:e #` goes back to.
    alternate: Option<String>,
    /// The last visual selection (its anchor, cursor and mode), for `gv`.
    last_visual: Option<(Pos, Pos, Mode)>,
    /// Ctrl-r was typed in insert mode: the next key names a register.
    insert_reg: bool,
    /// Where the cursor was in each file left with `:e`.
    cursors: HashMap<String, Pos>,
    /// The `:set` options.
    pub options: Options,
    confirm: Option<ConfirmSub>,
    /// Command-line completion: the candidates, which one is shown, and the
    /// text before the word being completed.
    completion: Option<(Vec<String>, usize, String)>,
    /// The keys of the command being typed.
    pending: Vec<Key>,
    /// The remembered column for `j` and `k` (`usize::MAX` after `$`).
    want: usize,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// The last `f`, `F`, `t` or `T`, and its character, for `;` and `,`.
    last_find: Option<(char, char)>,
    pub last_search: Option<String>,
    /// The keys of the last change, for `.`.
    last_change: Vec<Key>,
    /// The keys of the change being typed.
    recording: Option<Vec<Key>>,
    /// `.` is replaying `last_change`.
    replaying: bool,
    /// The number of buffer changes so far (for the UI to redraw).
    pub revision: u64,
}

impl Editor {
    /// An editor on `text`, in normal mode at the top.
    pub fn new(text: &str) -> Self {
        let mut lines = buffer::to_lines(text);
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
            search_back: false,
            jumps: Vec::new(),
            jump_at: 0,
            line_undo: None,
            files: BTreeMap::new(),
            buffer_name: ISLE.to_string(),
            alternate: None,
            last_visual: None,
            insert_reg: false,
            cursors: HashMap::new(),
            options: Options::default(),
            confirm: None,
            completion: None,
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
}
