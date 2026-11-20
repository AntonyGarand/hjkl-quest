//! Vim's patterns, the part of them a learner meets: `.` `*` `\+` `\=` `\?`
//! `\{n,m}` (and `\{-}` lazily), `[abc]` `[^a-z]`, `\d \w \s \a \l \u \x`
//! and their capitals, `^` `$` `\<` `\>`, `\(…\)` `\%(…\)` `\1`, `\|`,
//! `\zs` `\ze`, and `\v` (very magic: `( ) | + ? = { } < >` without the
//! backslash). `\c` and `\C` are read by the caller (they set the case).
//!
//! A pattern never fails to compile: what doesn't parse is taken literally,
//! as vim mostly does.

mod lex;
mod matcher;
mod parse;
#[cfg(test)]
mod tests;

use matcher::{Matcher, State};
use parse::{Node, Parser};

/// A compiled pattern.
#[derive(Debug, Clone)]
pub struct Regex {
    root: Node,
}

/// One match: where it starts and ends (after `\zs` and `\ze`), and the
/// groups `\1`…`\9` (0 is the whole match).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub start: usize,
    pub end: usize,
    pub groups: [Option<(usize, usize)>; 10],
}

impl Regex {
    /// Compiles `pat`; what doesn't parse is taken literally.
    pub fn new(pat: &str) -> Regex {
        let mut p = Parser::new(lex::tokenize(pat));
        let mut root = p.alt();
        // A stray `\)`: the rest is taken literally.
        while p.at < p.toks.len() {
            p.at += 1;
            let rest = p.alt();
            root = Node::Concat(vec![root, Node::Char(')'), rest]);
        }
        Regex { root }
    }

    /// The match that starts at `col`, if any.
    pub fn match_here(&self, line: &[char], col: usize, icase: bool) -> Option<Match> {
        if col > line.len() {
            return None;
        }
        let mut st = State {
            groups: [None; 10],
            zs: None,
            ze: None,
        };
        let mut found = None;
        let m = Matcher { line, icase };
        m.node(&self.root, col, &mut st, &mut |end, st| {
            let mut groups = st.groups;
            let start = st.zs.unwrap_or(col);
            let end = st.ze.unwrap_or(end).max(start);
            groups[0] = Some((start, end));
            found = Some(Match { start, end, groups });
            true
        });
        found
    }

    /// The first match at or after `col`.
    pub fn find_from(&self, line: &[char], col: usize, icase: bool) -> Option<Match> {
        (col..=line.len()).find_map(|c| self.match_here(line, c, icase))
    }
}

/// The replacement of `:s` for match `m` of `line`: `&` and `\0` the
/// match, `\1`…`\9` its groups, `\n` or `\r` a line break, `\u` `\l` the
/// next character's case, `\U` `\L` up to `\e` or `\E`, `\&` an ampersand.
pub fn expand(rep: &str, line: &[char], m: &Match) -> Vec<char> {
    let mut out = Vec::new();
    let mut one: Option<bool> = None;
    let mut all: Option<bool> = None;
    let push = |out: &mut Vec<char>, c: char, one: &mut Option<bool>, all: Option<bool>| {
        let upper = one.take().or(all);
        match upper {
            Some(true) => out.extend(c.to_uppercase()),
            Some(false) => out.extend(c.to_lowercase()),
            None => out.push(c),
        }
    };
    let group = |g: usize| m.groups[g].map_or_default(|(a, b)| line[a..b].to_vec());
    let mut chars = rep.chars();
    while let Some(c) = chars.next() {
        match c {
            '&' => {
                for ch in group(0) {
                    push(&mut out, ch, &mut one, all);
                }
            }
            '\\' => match chars.next() {
                Some(d @ '0'..='9') => {
                    for ch in group(d as usize - '0' as usize) {
                        push(&mut out, ch, &mut one, all);
                    }
                }
                Some('n' | 'r') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('u') => one = Some(true),
                Some('l') => one = Some(false),
                Some('U') => all = Some(true),
                Some('L') => all = Some(false),
                Some('e' | 'E') => all = None,
                Some(other) => push(&mut out, other, &mut one, all),
                None => out.push('\\'),
            },
            c => push(&mut out, c, &mut one, all),
        }
    }
    out
}
