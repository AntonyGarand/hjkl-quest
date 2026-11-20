//! Text objects (`iw`, `a"`, `i(`, `at`, ...) and the bracket matching
//! `%` and they share.

use super::operator::Span;
use super::words::class;
use super::{Editor, Pos, pos};

/// The pair a bracket belongs to: its opening and closing characters, and
/// whether it is the opening one.
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

/// The brackets a bracket object (`b`, `(`, `B`, `{`, `[`, `<`...) names.
fn brackets(obj: char) -> Option<(char, char)> {
    match obj {
        '(' | ')' | 'b' => Some(('(', ')')),
        '[' | ']' => Some(('[', ']')),
        '<' | '>' => Some(('<', '>')),
        '{' | '}' | 'B' => Some(('{', '}')),
        _ => None,
    }
}

/// The first non-blank at or after `from` in `line` (its length if none).
pub(super) fn skip_blanks(line: &[char], from: usize) -> usize {
    line[from.min(line.len())..]
        .iter()
        .position(|c| !c.is_whitespace())
        .map_or(line.len(), |k| from + k)
}

/// `aw` on a word: the blanks after it, or if there are none, before it.
fn word_with_blanks(line: &[char], s: usize, e: usize) -> (usize, usize) {
    let after = skip_blanks(line, e);
    if after > e {
        return (s, after);
    }
    let mut s = s;
    while s > 0 && line[s - 1].is_whitespace() {
        s -= 1;
    }
    (s, e)
}

/// `aw` on blanks: the word after them too.
fn blanks_with_word(line: &[char], e: usize, big: bool) -> usize {
    let cls = class(line.get(e).copied().unwrap_or(' '), big);
    let mut k = e;
    while k < line.len() && class(line[k], big) == cls && !line[k].is_whitespace() {
        k += 1;
    }
    k
}

/// Every `<tag>...</tag>` pair in `text`, as offsets: where the opening
/// tag starts and ends, then the closing one.
fn tag_pairs(text: &[char]) -> Vec<(usize, usize, usize, usize)> {
    let mut open: Vec<(String, usize, usize)> = Vec::new();
    let mut pairs = Vec::new();
    let mut k = 0;
    while k < text.len() {
        let tag_len = if text[k] == '<' {
            text[k..].iter().position(|&c| c == '>')
        } else {
            None
        };
        let Some(len) = tag_len else {
            k += 1;
            continue;
        };
        let end = k + len + 1;
        let body: String = text[k + 1..end - 1].iter().collect();
        if let Some(name) = body.strip_prefix('/') {
            if let Some(i) = open.iter().rposition(|(n, _, _)| n == name.trim()) {
                pairs.push((open[i].1, open[i].2, k, end));
                open.truncate(i);
            }
        } else if !body.ends_with('/') {
            let name: String = body.chars().take_while(|c| !c.is_whitespace()).collect();
            if !name.is_empty() {
                open.push((name, k, end));
            }
        }
        k = end;
    }
    pairs
}

impl Editor {
    /// The span of text object `obj`, inner (`i`) or around (`a`).
    pub(super) fn object_span(&self, inner: bool, obj: char) -> Option<Span> {
        let chars = match obj {
            'p' => {
                let (a, b) = self.paragraph_object(inner);
                return Some(Span::Lines(a, b));
            }
            't' => self.tag_object(inner),
            's' => self.sentence_object(inner),
            'w' | 'W' => self.word_object(inner, obj == 'W'),
            '"' | '\'' | '`' => self.quote_object(inner, obj),
            _ => self.bracket_object(inner, obj),
        };
        chars.map(|(s, e)| Span::Chars(s, e))
    }

    /// `iw` / `aw` (`iW` / `aW` with `big`): the word, or run of blanks,
    /// under the cursor.
    fn word_object(&self, inner: bool, big: bool) -> Option<(Pos, Pos)> {
        let cur = self.cursor;
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
            (s, e) = if cls == 0 {
                (s, blanks_with_word(line, e, big))
            } else {
                word_with_blanks(line, s, e)
            };
        }
        Some((pos(cur.row, s), pos(cur.row, e)))
    }

    /// `i"` / `a"` (and `'`, `` ` ``): the quoted text around, or after,
    /// the cursor on its line; `a` takes the blanks after it too.
    fn quote_object(&self, inner: bool, quote: char) -> Option<(Pos, Pos)> {
        let cur = self.cursor;
        let line = &self.lines[cur.row];
        let quotes: Vec<usize> = line
            .iter()
            .enumerate()
            .filter(|(_, c)| **c == quote)
            .map(|(i, _)| i)
            .collect();
        let pairs = || quotes.chunks(2).filter(|p| p.len() == 2);
        let pair = pairs()
            .find(|p| p[0] <= cur.col && cur.col <= p[1])
            .or_else(|| pairs().find(|p| p[0] > cur.col))?;
        let (a, b) = (pair[0], pair[1]);
        if inner {
            Some((pos(cur.row, a + 1), pos(cur.row, b)))
        } else {
            Some((pos(cur.row, a), pos(cur.row, skip_blanks(line, b + 1))))
        }
    }

    /// `i(` / `a(` and the other brackets: inside, or with, the innermost
    /// pair around the cursor.
    fn bracket_object(&self, inner: bool, obj: char) -> Option<(Pos, Pos)> {
        let (open, close) = brackets(obj)?;
        let start = self.unmatched_open(open, close)?;
        let end = self.matching_close(start, open, close)?;
        if inner {
            Some((self.after(start), end))
        } else {
            Some((start, self.after(end)))
        }
    }

    /// Searches back from the cursor for the bracket that opens around it.
    fn unmatched_open(&self, open: char, close: char) -> Option<Pos> {
        let mut p = self.cursor;
        let mut depth = if self.char_at(p) == Some(close) { -1 } else { 0 };
        loop {
            let c = self.char_at(p);
            if c == Some(open) {
                if depth == 0 {
                    return Some(p);
                }
                depth -= 1;
            } else if c == Some(close) {
                depth += 1;
            }
            depth = depth.max(0);
            p = self.prev(p)?.0;
        }
    }

    /// Searches forward from the bracket at `start` for the one closing it.
    fn matching_close(&self, start: Pos, open: char, close: char) -> Option<Pos> {
        let mut depth = 0;
        let mut q = start;
        loop {
            let c = self.char_at(q);
            if c == Some(open) {
                depth += 1;
            } else if c == Some(close) {
                depth -= 1;
                if depth == 0 {
                    return Some(q);
                }
            }
            q = self.next(q)?.0;
        }
    }

    /// `%`: the bracket matching the one at, or after, `at` on its line.
    pub(super) fn match_pair(&self, at: Pos) -> Option<Pos> {
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
        let (os, oe, cs, ce) = tag_pairs(&text)
            .into_iter()
            .filter(|p| p.0 <= cur && cur < p.3)
            .min_by_key(|p| p.3 - p.0)?;
        if inner {
            Some((to_pos(oe), to_pos(cs)))
        } else {
            Some((to_pos(os), to_pos(ce)))
        }
    }
}
