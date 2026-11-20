//! Keystrokes: the [`Key`] type, how a key is stored in a register, and
//! the vim-golf notation (`<Esc>`, `<C-r>`...) levels and replays use.

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
    /// Tab, which is also Ctrl-i.
    Tab,
    CtrlD,
    CtrlO,
    CtrlG,
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
            Key::Tab => '\t',
            Key::CtrlD => '\u{4}',
            Key::CtrlO => '\u{f}',
            Key::CtrlG => '\u{7}',
        }
    }

    /// The key a register character stands for: the reverse of [`Key::raw`].
    pub fn from_raw(c: char) -> Key {
        match c {
            '\u{1b}' => Key::Esc,
            '\r' | '\n' => Key::Enter,
            '\u{8}' => Key::Backspace,
            '\u{12}' => Key::CtrlR,
            '\u{1}' => Key::CtrlA,
            '\u{18}' => Key::CtrlX,
            '\u{16}' => Key::CtrlV,
            '\t' => Key::Tab,
            '\u{4}' => Key::CtrlD,
            '\u{f}' => Key::CtrlO,
            '\u{7}' => Key::CtrlG,
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
            Key::Tab => write!(f, "<Tab>"),
            Key::CtrlD => write!(f, "<C-d>"),
            Key::CtrlO => write!(f, "<C-o>"),
            Key::CtrlG => write!(f, "<C-g>"),
        }
    }
}

/// The key a `<name>` stands for, if it is one we know.
fn named_key(name: &str) -> Option<Key> {
    let key = match name.to_ascii_lowercase().as_str() {
        "esc" => Key::Esc,
        "cr" | "enter" => Key::Enter,
        "bs" => Key::Backspace,
        "c-r" => Key::CtrlR,
        "c-a" => Key::CtrlA,
        "c-x" => Key::CtrlX,
        "c-v" => Key::CtrlV,
        "tab" | "c-i" => Key::Tab,
        "c-d" => Key::CtrlD,
        "c-o" => Key::CtrlO,
        "c-g" => Key::CtrlG,
        "space" => Key::Char(' '),
        "lt" => Key::Char('<'),
        _ => return None,
    };
    Some(key)
}

/// Parses vim-golf notation: plain characters, plus `<Esc>`, `<CR>`, `<BS>`,
/// `<C-r>`, `<C-a>`, `<C-x>`, `<C-v>` and `<Space>`. A `<` that starts no
/// known name is the character itself.
pub fn parse_keys(s: &str) -> Vec<Key> {
    let mut keys = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<'
            && let Some(end) = chars[i..].iter().position(|&c| c == '>')
            && let Some(key) = named_key(&chars[i + 1..i + end].iter().collect::<String>())
        {
            keys.push(key);
            i += end + 1;
            continue;
        }
        keys.push(Key::Char(chars[i]));
        i += 1;
    }
    keys
}

/// Writes keys back in vim-golf notation (a space shows as `␣`).
pub fn format_keys(keys: &[Key]) -> String {
    keys.iter().map(ToString::to_string).collect()
}
