//! The `:set` options (vimtutor 6.5): case in searches, and how matches
//! light up.

use super::{Editor, Outcome};

/// Options `:set` knows, for completion.
pub(super) const OPTION_NAMES: &[&str] = &[
    "hlsearch",
    "ignorecase",
    "incsearch",
    "nohlsearch",
    "noignorecase",
    "noincsearch",
    "nosmartcase",
    "smartcase",
];

/// The options `:set` changes, each on or off.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
// One field per vim option, named as `:set` names it: grouping them any
// further would only hide the names.
#[allow(clippy::struct_excessive_bools)]
pub struct Options {
    /// Searches ignore case...
    pub ignorecase: bool,
    /// ...unless the pattern has a capital.
    pub smartcase: bool,
    /// The last search's matches stay lit.
    pub hlsearch: bool,
    /// Matches light up while the pattern is typed.
    pub incsearch: bool,
}

/// `name`, or `noname` when `on` is false.
fn state(on: bool, name: &str) -> String {
    format!("{}{name}", if on { "" } else { "no" })
}

impl Options {
    /// The flag a `:set` name (long or short) stands for.
    fn flag(&mut self, name: &str) -> Option<&mut bool> {
        match name {
            "ic" | "ignorecase" => Some(&mut self.ignorecase),
            "scs" | "smartcase" => Some(&mut self.smartcase),
            "hls" | "hlsearch" => Some(&mut self.hlsearch),
            "is" | "incsearch" => Some(&mut self.incsearch),
            _ => None,
        }
    }

    /// One `:set` word: `name` (on), `noname` (off), `name!` (toggled) or
    /// `name?` (shown: returned as text).
    fn set(&mut self, word: &str) -> Result<Option<String>, String> {
        let (name, on) = match word.strip_prefix("no") {
            Some(rest) if !rest.is_empty() => (rest, false),
            _ => (word, true),
        };
        let (name, on, query) = match name.strip_suffix('!') {
            Some(n) => (n, None, false),
            None => match name.strip_suffix('?') {
                Some(n) => (n, Some(on), true),
                None => (name, Some(on), false),
            },
        };
        let flag = self
            .flag(name)
            .ok_or_else(|| format!("E518: Unknown option: {word}"))?;
        if query {
            return Ok(Some(state(*flag, name)));
        }
        *flag = on.unwrap_or(!*flag);
        Ok(None)
    }

    /// Whether a pattern ignores case: `\c` / `\C` in it decide, else
    /// 'ignorecase', unless 'smartcase' and it has a capital.
    pub(super) fn icase(self, pat: &str) -> bool {
        if pat.contains(r"\c") {
            true
        } else if pat.contains(r"\C") {
            false
        } else {
            self.ignorecase && !(self.smartcase && pat.chars().any(char::is_uppercase))
        }
    }
}

impl Editor {
    /// `:set` with one or more options; alone, it shows them all.
    pub(super) fn set_options(&mut self, arg: &str) -> Outcome {
        if arg.trim().is_empty() {
            let o = self.options;
            return Outcome::Info(format!(
                "{}  {}  {}  {}",
                state(o.ignorecase, "ignorecase"),
                state(o.smartcase, "smartcase"),
                state(o.hlsearch, "hlsearch"),
                state(o.incsearch, "incsearch")
            ));
        }
        let mut shown = Vec::new();
        for word in arg.split_whitespace() {
            match self.options.set(word) {
                Ok(Some(text)) => shown.push(text),
                Ok(None) => {}
                Err(msg) => return Outcome::Bell(msg),
            }
        }
        if shown.is_empty() {
            Outcome::Ok
        } else {
            Outcome::Info(shown.join("  "))
        }
    }
}
