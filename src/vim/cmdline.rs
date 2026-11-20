//! The command line: typing after `:`, `/` or `?`, and Tab completion
//! (vimtutor 7.3).

use super::options::OPTION_NAMES;
use super::{Editor, Key, Mode, Outcome};

/// Ex commands Tab completes.
const EX_NAMES: &[&str] = &[
    "copy",
    "delete",
    "edit",
    "global",
    "help",
    "registers",
    "join",
    "move",
    "nohlsearch",
    "normal",
    "quit",
    "read",
    "set",
    "sort",
    "substitute",
    "vglobal",
    "write",
    "wq",
];

/// Commands that take no argument: completing them adds no space.
const NO_ARGUMENT: &[&str] = &["nohlsearch", "quit", "wq", "sort", "join", "delete"];

impl Editor {
    /// A key on the command line; `search` for `/` and `?`.
    pub(super) fn command_key(&mut self, key: Key, search: bool) -> Outcome {
        if !matches!(key, Key::Tab | Key::CtrlD) {
            self.completion = None;
        }
        match key {
            Key::Tab | Key::CtrlD if !search => self.complete(key == Key::CtrlD),
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
            Key::Enter => {
                self.mode = Mode::Normal;
                let line = std::mem::take(&mut self.cmdline);
                if search {
                    self.submit_search(line)
                } else {
                    self.ex_line(&line)
                }
            }
            _ => Outcome::Ok,
        }
    }

    /// Tab completes the command line; Ctrl-d lists the choices.
    fn complete(&mut self, list: bool) -> Outcome {
        if !list && let Some((choices, at, head)) = &mut self.completion {
            // Tab again: the next choice.
            *at = (*at + 1) % choices.len();
            self.cmdline = format!("{head}{}", choices[*at]);
            return Outcome::Ok;
        }
        let line = self.cmdline.clone();
        let (head, word, choices) = self.completion_choices(&line);
        let found: Vec<String> = choices.into_iter().filter(|c| c.starts_with(word)).collect();
        if found.is_empty() {
            return Outcome::Bell("No match".into());
        }
        if list {
            return Outcome::Info(found.join("  "));
        }
        self.cmdline = format!("{head}{}", found[0]);
        if found.len() > 1 {
            self.completion = Some((found, 0, head.to_string()));
        } else if !line.contains(' ') && !NO_ARGUMENT.contains(&found[0].as_str()) {
            self.cmdline.push(' ');
        }
        Outcome::Ok
    }

    /// What Tab completes in `line`: the text before the word being
    /// completed, the word, and what it may become (a command name, an
    /// option after `:set`, else a file name).
    fn completion_choices<'a>(&self, line: &'a str) -> (&'a str, &'a str, Vec<String>) {
        let Some(sp) = line.rfind(' ') else {
            // The command name, after any range.
            let start = line.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(line.len());
            let names = EX_NAMES.iter().map(ToString::to_string).collect();
            return (&line[..start], &line[start..], names);
        };
        let (head, word) = (&line[..=sp], &line[sp + 1..]);
        let cmd = head
            .trim()
            .trim_start_matches(|c: char| !c.is_ascii_alphabetic() && c != '!');
        let names = if cmd.starts_with("se") {
            OPTION_NAMES.iter().map(ToString::to_string).collect()
        } else {
            self.files.keys().cloned().collect()
        };
        (head, word, names)
    }
}
