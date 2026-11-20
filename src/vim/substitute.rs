//! `:s/pattern/replacement/flags`, `&` and `g&` to run it again, and
//! `:s///c`, which asks before each replacement.

use super::ex::ExCmd;
use super::{Editor, Key, Mode, Outcome, Pos, find_all, find_matches, pos};
use crate::regex::{Match, Regex, expand};

/// A `:s///c` in progress: the matches left, and what to put instead.
#[derive(Debug, Clone)]
pub(super) struct ConfirmSub {
    /// (row, col, length) of each match, in order.
    matches: Vec<(usize, usize, usize)>,
    rep: String,
    pat: String,
    icase: bool,
}

/// Splits `s` at `delim` into at most `n` parts, skipping a delimiter with
/// a backslash before it (`:s/a\/b/c/` has the pattern `a\/b`).
pub(super) fn split_unescaped(s: &str, delim: char, n: usize) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        if parts.len() + 1 == n {
            break;
        }
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == delim {
            parts.push(&s[start..i]);
            start = i + c.len_utf8();
        }
    }
    parts.push(&s[start..]);
    parts
}

/// `line` with each match in `hits` replaced by `rep` (`&` and `\1` in it
/// stand for the match and its groups).
fn replaced(line: &[char], hits: &[Match], rep: &str) -> Vec<char> {
    let mut out = Vec::new();
    let mut at = 0;
    for m in hits {
        out.extend_from_slice(&line[at..m.start]);
        out.extend(expand(rep, line, m));
        at = m.end;
    }
    out.extend_from_slice(&line[at..]);
    out
}

impl Editor {
    /// `:s`, or `:&` / `:s` alone (the last one again).
    pub(super) fn ex_substitute(&mut self, ex: &ExCmd<'_>) -> Outcome {
        let (a, b, arg) = (ex.a, ex.b, ex.arg);
        let (pat, rep, global) = if ex.name == "&" || arg.trim().is_empty() {
            let Some((p, r, g)) = &self.last_sub else {
                return Outcome::Bell("E35: No previous substitute".into());
            };
            (p.clone(), r.clone(), *g && arg.contains('&'))
        } else {
            let (pat, rep, flags) = match self.parse_substitute(arg) {
                Ok(parts) => parts,
                Err(msg) => return Outcome::Bell(msg),
            };
            let global = flags.contains('g');
            if flags.contains('c') {
                self.last_search = Some(pat.clone());
                return self.start_confirm(a, b, &pat, &rep, global);
            }
            (pat, rep, global)
        };
        self.last_search = Some(pat.clone());
        self.last_sub = Some((pat.clone(), rep.clone(), global));
        self.substitute(a, b, &pat, &rep, global)
    }

    /// `/pattern/replacement/flags` (an empty pattern: the last search).
    fn parse_substitute<'a>(&self, arg: &'a str) -> Result<(String, String, &'a str), String> {
        let Some(delim) = arg.chars().next().filter(|c| !c.is_alphanumeric() && *c != ' ') else {
            return Err("Usage: :s/old/new/g".into());
        };
        let parts = split_unescaped(&arg[delim.len_utf8()..], delim, 3);
        let pat = match parts[0] {
            "" => self
                .last_search
                .clone()
                .ok_or("E35: No previous regular expression")?,
            p => p.to_string(),
        };
        let rep = parts.get(1).copied().unwrap_or("").to_string();
        Ok((pat, rep, parts.get(2).copied().unwrap_or("")))
    }

    /// `:s` over rows `a..=b`: the first match of each line, or all with
    /// `global`. A `\r` in the replacement splits the line.
    pub(super) fn substitute(&mut self, a: usize, b: usize, pat: &str, rep: &str, global: bool) -> Outcome {
        let before = self.checkpoint();
        let Some(row) = self.replace_in_rows(a, b, pat, rep, global) else {
            return Outcome::Bell(format!("Pattern not found: {pat}"));
        };
        let row = self.split_returns(row).min(self.lines.len() - 1);
        self.cursor = self.line_start(row);
        self.commit(before);
        Outcome::Ok
    }

    /// Replaces the matches in rows `a..=b`; returns the last row changed.
    fn replace_in_rows(&mut self, a: usize, b: usize, pat: &str, rep: &str, global: bool) -> Option<usize> {
        let (re, icase) = (Regex::new(pat), self.options.icase(pat));
        let mut last = None;
        for row in a..=b.min(self.lines.len() - 1) {
            let mut hits = find_matches(&re, &self.lines[row], icase);
            if hits.is_empty() {
                continue;
            }
            if !global {
                hits.truncate(1);
            }
            self.lines[row] = replaced(&self.lines[row], &hits, rep);
            last = Some(row);
        }
        last
    }

    /// Splits lines at the newlines a replacement put in them; returns
    /// where `row` ends up.
    fn split_returns(&mut self, row: usize) -> usize {
        let mut split: Vec<Vec<char>> = Vec::new();
        let mut moved = row;
        for (r, line) in std::mem::take(&mut self.lines).into_iter().enumerate() {
            let parts: Vec<Vec<char>> = line.split(|c| *c == '\n').map(<[char]>::to_vec).collect();
            if r <= row {
                moved += parts.len() - 1;
            }
            split.extend(parts);
        }
        self.lines = split;
        moved
    }

    /// `&`: the last `:s` again, on this line.
    pub(super) fn substitute_line_again(&mut self) -> Outcome {
        let Some((pat, rep, _)) = self.last_sub.clone() else {
            return Outcome::Bell("No previous substitute".into());
        };
        let row = self.cursor.row;
        self.substitute(row, row, &pat, &rep, false)
    }

    /// `g&`: the last `:s` again, with its flags, on every line.
    pub(super) fn substitute_all_again(&mut self) -> Outcome {
        let Some((pat, rep, global)) = self.last_sub.clone() else {
            return Outcome::Bell("No previous substitute".into());
        };
        self.substitute(0, self.lines.len() - 1, &pat, &rep, global)
    }

    /// `:s///c`: finds the matches and asks about the first.
    fn start_confirm(&mut self, a: usize, b: usize, pat: &str, rep: &str, global: bool) -> Outcome {
        let icase = self.options.icase(pat);
        let last = b.min(self.lines.len() - 1);
        let mut matches = Vec::new();
        for (row, line) in self.lines.iter().enumerate().take(last + 1).skip(a) {
            let mut hits = find_all(pat, line, icase);
            if !global {
                hits.truncate(1);
            }
            matches.extend(hits.into_iter().filter(|(_, l)| *l > 0).map(|(c, l)| (row, c, l)));
        }
        let Some(&(row, col, _)) = matches.first() else {
            return Outcome::Bell(format!("Pattern not found: {pat}"));
        };
        self.snapshot_force();
        self.confirm = Some(ConfirmSub {
            matches,
            rep: rep.to_string(),
            pat: pat.to_string(),
            icase,
        });
        self.mode = Mode::Confirm;
        self.cursor = pos(row, col);
        Outcome::Info(self.confirm_prompt())
    }

    /// The question `:s///c` asks.
    pub fn confirm_prompt(&self) -> String {
        let rep = self.confirm.as_ref().map_or("", |c| c.rep.as_str());
        format!("replace with {rep} (y/n/a/q/l)?")
    }

    /// The match `:s///c` is asking about, to light up.
    pub(super) fn confirm_highlight(&self) -> Vec<(Pos, usize)> {
        self.confirm
            .as_ref()
            .and_then(|c| c.matches.first())
            .map_or_default(|&(r, c, l)| vec![(pos(r, c), l)])
    }

    /// An answer to `:s///c`: `y`es, `n`o, `a`ll, `q`uit, or `l`ast (this
    /// one, then stop).
    pub(super) fn confirm_key(&mut self, key: Key) -> Outcome {
        let Some(mut c) = self.confirm.take() else {
            self.mode = Mode::Normal;
            return Outcome::Ok;
        };
        let answer = match key {
            Key::Char(ch @ ('y' | 'n' | 'a' | 'q' | 'l')) => ch,
            Key::Esc => 'q',
            _ => {
                self.confirm = Some(c);
                return Outcome::Bell("Answer y, n, a, q or l".into());
            }
        };
        match answer {
            'y' => self.replace_next(&mut c),
            'l' => {
                self.replace_next(&mut c);
                c.matches.clear();
            }
            'a' => {
                while !c.matches.is_empty() {
                    self.replace_next(&mut c);
                }
            }
            'n' => {
                c.matches.remove(0);
            }
            _ => c.matches.clear(),
        }
        if let Some(&(row, col, _)) = c.matches.first() {
            self.cursor = pos(row, col);
            self.confirm = Some(c);
            Outcome::Info(self.confirm_prompt())
        } else {
            self.mode = Mode::Normal;
            self.clamp();
            Outcome::Ok
        }
    }

    /// Replaces the first match left, and moves the later ones on its
    /// line by how much it grew or shrank.
    fn replace_next(&mut self, c: &mut ConfirmSub) {
        let (row, col, len) = c.matches.remove(0);
        let line = self.lines[row].clone();
        let mut m = Regex::new(&c.pat)
            .match_here(&line, col, c.icase)
            .filter(|m| m.start == col && m.end == col + len)
            .unwrap_or(Match {
                start: col,
                end: col + len,
                groups: [None; 10],
            });
        m.groups[0] = Some((col, col + len));
        let new = expand(&c.rep, &line, &m);
        let delta = new.len() as i64 - len as i64;
        self.lines[row].splice(col..col + len, new);
        for m in c.matches.iter_mut().filter(|m| m.0 == row && m.1 > col) {
            m.1 = (m.1 as i64 + delta) as usize;
        }
        self.changed();
    }
}
