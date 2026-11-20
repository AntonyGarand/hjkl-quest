//! Searching: `f` and `t` on the line, `/` and `?` through the buffer,
//! `n`, `*`, `#`, the match `gn` picks, and the matches that light up.

use super::words::class;
use super::{Editor, Mode, Outcome, Parse, Pos, pos};
use crate::regex::{Match, Regex};

/// Whether `pat` matches `line` at `col`, and how long the match is (a
/// vim pattern: see `regex`).
pub fn match_at(pat: &str, line: &[char], col: usize, icase: bool) -> Option<usize> {
    Regex::new(pat)
        .match_here(line, col, icase)
        .filter(|m| m.start == col)
        .map(|m| m.end - m.start)
}

/// The non-overlapping matches of `pat` in `line`, as `:s` and `:g` see
/// them: where each starts, and how long it is.
pub fn find_all(pat: &str, line: &[char], icase: bool) -> Vec<(usize, usize)> {
    find_matches(&Regex::new(pat), line, icase)
        .into_iter()
        .map(|m| (m.start, m.end - m.start))
        .collect()
}

/// The non-overlapping matches of `re` in `line`, with their groups.
pub fn find_matches(re: &Regex, line: &[char], icase: bool) -> Vec<Match> {
    let mut out: Vec<Match> = Vec::new();
    let mut col = 0;
    while col <= line.len() {
        match re.match_here(line, col, icase) {
            Some(m)
                if out
                    .last()
                    .is_none_or(|l| m.start >= l.end && (m.start > l.start || m.end > l.end)) =>
            {
                col = if m.end > col { m.end } else { col + 1 };
                out.push(m);
            }
            _ => col += 1,
        }
    }
    out
}

/// The next `target` in `line` from `from` (not counting it), forward or
/// back; `skip_adjacent` passes over one right next to `from`.
fn find_in_line(
    line: &[char],
    from: usize,
    target: char,
    forward: bool,
    skip_adjacent: bool,
) -> Option<usize> {
    let mut c = from;
    loop {
        if forward {
            c += 1;
            if c >= line.len() {
                return None;
            }
        } else {
            c = c.checked_sub(1)?;
        }
        if line[c] == target && !(skip_adjacent && c.abs_diff(from) == 1) {
            return Some(c);
        }
    }
}

/// Whether the match `col..col + len` stands alone: no word character
/// just before or after it.
fn is_whole_word(line: &[char], col: usize, len: usize) -> bool {
    let before = col > 0 && class(line[col - 1], false) == 2;
    let after = col + len < line.len() && class(line[col + len], false) == 2;
    !before && !after
}

/// `word` as a pattern that matches it literally.
fn escape_pattern(word: &str) -> String {
    let mut out = String::new();
    for c in word.chars() {
        if "\\.*[~^$/".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

impl Editor {
    /// `f`, `F`, `t` and `T` (`kind`): the `n`th `target` on the line.
    /// `repeat` is for `;` and `,`, which don't stop right before the target.
    pub(super) fn find(&self, kind: char, target: char, n: usize, repeat: bool) -> Option<Pos> {
        let line = &self.lines[self.cursor.row];
        let forward = kind == 'f' || kind == 't';
        let till = kind == 't' || kind == 'T';
        let mut col = self.cursor.col;
        for k in 0..n {
            // `;` after `t` skips the character right before the target.
            col = find_in_line(line, col, target, forward, till && repeat && k == 0)?;
        }
        let col = if till {
            if forward { col - 1 } else { col + 1 }
        } else {
            col
        };
        Some(pos(self.cursor.row, col))
    }

    /// Every match of `pat` in the buffer, overlapping as vim's search.
    fn all_hits(&self, pat: &str, whole_word: bool) -> Vec<(Pos, usize)> {
        let icase = self.options.icase(pat);
        let re = Regex::new(pat);
        let mut hits: Vec<(Pos, usize)> = Vec::new();
        for (row, line) in self.lines.iter().enumerate() {
            for from in 0..=line.len() {
                let Some(m) = re.match_here(line, from, icase) else {
                    continue;
                };
                let (at, len) = (pos(row, m.start), m.end - m.start);
                let seen = hits.last().is_some_and(|&(p, _)| p == at);
                if !seen && (!whole_word || is_whole_word(line, m.start, len)) {
                    hits.push((at, len));
                }
            }
        }
        hits
    }

    /// The match `gn` picks: the one under the cursor, else the next.
    pub(super) fn match_at_or_after(&self, forward: bool) -> Option<(Pos, Pos)> {
        let pat = self.last_search.clone()?;
        let hits = self.all_hits(&pat, false);
        let cur = self.cursor;
        let under = hits
            .iter()
            .find(|(p, len)| p.row == cur.row && p.col <= cur.col && cur.col < p.col + (*len).max(1));
        let (at, len) = match under {
            Some(h) => *h,
            None if forward => *hits.iter().find(|(p, _)| *p > cur).or(hits.first())?,
            None => *hits.iter().rev().find(|(p, _)| *p < cur).or(hits.last())?,
        };
        Some((at, pos(at.row, at.col + len)))
    }

    /// The next match of the last search after (or before) `from`,
    /// wrapping around the buffer.
    fn search_from(&self, from: Pos, forward: bool, whole_word: bool) -> Option<Pos> {
        let pat = self.last_search.as_ref()?;
        if pat.is_empty() {
            return None;
        }
        let hits: Vec<Pos> = self
            .all_hits(pat, whole_word)
            .into_iter()
            .map(|(p, _)| p)
            .collect();
        if forward {
            hits.iter().find(|&&h| h > from).or(hits.first()).copied()
        } else {
            hits.iter().rev().find(|&&h| h < from).or(hits.last()).copied()
        }
    }

    /// The `n`th match from the cursor.
    fn search_times(&self, forward: bool, whole_word: bool, n: usize) -> Result<Pos, Parse> {
        let mut p = self.cursor;
        for _ in 0..n {
            p = self.search_from(p, forward, whole_word).ok_or(Parse::Invalid)?;
        }
        Ok(p)
    }

    /// `n` / `N`: the last search again, `n` the way it went.
    pub(super) fn search_next(&self, same_way: bool, n: usize) -> Result<Pos, Parse> {
        if self.last_search.is_none() {
            return Err(Parse::Invalid);
        }
        self.search_times(same_way != self.search_back, false, n)
    }

    /// `*` / `#`: the word under the cursor, as a whole word.
    pub(super) fn search_word(&mut self, forward: bool, n: usize) -> Result<Pos, Parse> {
        let word = self.word_under(self.cursor).ok_or(Parse::Invalid)?;
        self.last_search = Some(format!(r"\<{word}\>"));
        self.search_back = !forward;
        self.search_times(forward, true, n)
    }

    /// `g*` / `g#`: the word under the cursor, also inside longer words.
    pub(super) fn search_partial_word(&mut self, forward: bool, n: usize) -> Result<Pos, Parse> {
        let word = self.word_under(self.cursor).ok_or(Parse::Invalid)?;
        self.last_search = Some(escape_pattern(&word));
        self.search_back = !forward;
        self.search_times(forward, false, n)
    }

    /// `/pattern<CR>` or `?pattern<CR>` (an empty pattern: the last one).
    pub(super) fn submit_search(&mut self, pattern: String) -> Outcome {
        if pattern.is_empty() && self.last_search.is_none() {
            return Outcome::Bell("No previous search".into());
        }
        if !pattern.is_empty() {
            self.last_search = Some(pattern);
        }
        match self.search_from(self.cursor, !self.search_back, false) {
            Some(p) => {
                self.jump_from_here();
                self.cursor = p;
                self.want = p.col;
                Outcome::Ok
            }
            None => Outcome::Bell("Pattern not found".into()),
        }
    }

    /// The matches to light up: what the search being typed finds
    /// ('incsearch'), else the last search ('hlsearch').
    pub fn search_highlights(&self) -> Vec<(Pos, usize)> {
        let pat = match self.mode {
            Mode::Command { search: true } if self.options.incsearch && !self.cmdline.is_empty() => {
                self.cmdline.clone()
            }
            Mode::Confirm => return self.confirm_highlight(),
            _ if self.options.hlsearch => match &self.last_search {
                Some(p) => p.clone(),
                None => return Vec::new(),
            },
            _ => return Vec::new(),
        };
        self.all_hits(&pat, false)
            .into_iter()
            .filter(|(_, l)| *l > 0)
            .collect()
    }
}
