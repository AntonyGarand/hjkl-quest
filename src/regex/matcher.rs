//! Matching a node tree against a line, by backtracking: each node is
//! given the rest of the match as a continuation.

use super::parse::{Node, Range, Repeat};

/// The groups and `\zs` `\ze` marks of the match so far.
#[derive(Clone)]
pub(super) struct State {
    pub(super) groups: [Option<(usize, usize)>; 10],
    pub(super) zs: Option<usize>,
    pub(super) ze: Option<usize>,
}

/// The rest of the match, from a position: whether it succeeds.
pub(super) type K<'a> = &'a mut dyn FnMut(usize, &mut State) -> bool;

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether `c` is in named class `class` (`\d`, `\W`, ...).
fn named(class: char, c: char) -> bool {
    let yes = match class.to_ascii_lowercase() {
        'd' => c.is_ascii_digit(),
        'w' => c.is_ascii_alphanumeric() || c == '_',
        's' => c == ' ' || c == '\t',
        'a' => c.is_alphabetic(),
        'l' => c.is_lowercase(),
        'u' => c.is_uppercase(),
        'x' => c.is_ascii_hexdigit(),
        'h' => c.is_ascii_alphabetic() || c == '_',
        _ => false,
    };
    // A capital is the opposite, as `\D` is "not a digit".
    if class.is_ascii_uppercase() { !yes } else { yes }
}

pub(super) struct Matcher<'l> {
    pub(super) line: &'l [char],
    pub(super) icase: bool,
}

impl Matcher<'_> {
    fn same(&self, a: char, b: char) -> bool {
        a == b || self.icase && a.to_lowercase().eq(b.to_lowercase())
    }

    fn class_has(&self, ranges: &[Range], c: char) -> bool {
        let test = |c: char| {
            ranges.iter().any(|r| match *r {
                Range::Span(a, b) => a <= c && c <= b,
                Range::Named(n) => named(n, c),
            })
        };
        test(c) || self.icase && (c.to_lowercase().any(test) || c.to_uppercase().any(test))
    }

    /// Matches `n` at `pos`, then the rest `k`.
    pub(super) fn node(&self, n: &Node, pos: usize, st: &mut State, k: K<'_>) -> bool {
        let line = self.line;
        let one = |ok: bool, st: &mut State, k: K<'_>| ok && k(pos + 1, st);
        match n {
            Node::Char(c) => one(line.get(pos).is_some_and(|&x| self.same(x, *c)), st, k),
            Node::Any => one(pos < line.len(), st, k),
            Node::Class(ranges, neg) => one(
                line.get(pos).is_some_and(|&x| self.class_has(ranges, x) != *neg),
                st,
                k,
            ),
            Node::Start => pos == 0 && k(pos, st),
            Node::End => pos == line.len() && k(pos, st),
            Node::WordStart => self.word_edge(pos, true) && k(pos, st),
            Node::WordEnd => self.word_edge(pos, false) && k(pos, st),
            Node::Zs | Node::Ze => mark(matches!(n, Node::Zs), pos, st, k),
            Node::Backref(g) => self.backref(*g, pos, st, k),
            Node::Group(inner, g) => self.group(inner, *g, pos, st, k),
            Node::Alt(branches) => branches.iter().any(|b| self.node(b, pos, st, &mut *k)),
            Node::Concat(items) => self.seq(items, pos, st, k),
            Node::Repeat(r) => self.repeat(r, 0, pos, st, k),
        }
    }

    /// Whether a word starts (or ends) at `pos`.
    fn word_edge(&self, pos: usize, start: bool) -> bool {
        let here = self.line.get(pos).is_some_and(|&c| is_word(c));
        let before = pos > 0 && is_word(self.line[pos - 1]);
        if start { here && !before } else { before && !here }
    }

    /// `\1`…`\9`: the text group `g` matched, again. A group that matched
    /// nothing yet matches the empty string.
    fn backref(&self, g: usize, pos: usize, st: &mut State, k: K<'_>) -> bool {
        let Some((a, b)) = st.groups[g] else {
            return k(pos, st);
        };
        let len = b - a;
        let line = self.line;
        if pos + len > line.len() || !(0..len).all(|i| self.same(line[pos + i], line[a + i])) {
            return false;
        }
        k(pos + len, st)
    }

    /// A group: `inner`, remembered as group `g` if it captures.
    fn group(&self, inner: &Node, g: Option<usize>, pos: usize, st: &mut State, k: K<'_>) -> bool {
        self.node(inner, pos, st, &mut |end, st| {
            let Some(g) = g else { return k(end, st) };
            let saved = st.groups[g];
            st.groups[g] = Some((pos, end));
            if k(end, st) {
                return true;
            }
            st.groups[g] = saved;
            false
        })
    }

    fn seq(&self, items: &[Node], pos: usize, st: &mut State, k: K<'_>) -> bool {
        match items.split_first() {
            None => k(pos, st),
            Some((first, rest)) => self.node(first, pos, st, &mut |p, st| self.seq(rest, p, st, k)),
        }
    }

    /// Repeat `r`, matched `count` times so far, from `pos`.
    fn repeat(&self, r: &Repeat, count: usize, pos: usize, st: &mut State, k: K<'_>) -> bool {
        let more = |st: &mut State, k: K<'_>| {
            count < r.max
                && self.node(&r.node, pos, st, &mut |p, st| {
                    // A match of nothing ends the repeat (or it would loop).
                    if p == pos {
                        count + 1 >= r.min && k(p, st)
                    } else {
                        self.repeat(r, count + 1, p, st, k)
                    }
                })
        };
        if r.greedy {
            more(st, &mut *k) || (count >= r.min && k(pos, st))
        } else {
            (count >= r.min && k(pos, &mut *st)) || more(st, k)
        }
    }
}

/// `\zs` or `\ze`: the match starts (or ends) here, if the rest matches.
fn mark(zs: bool, pos: usize, st: &mut State, k: K<'_>) -> bool {
    let saved = slot(st, zs).replace(pos);
    if k(pos, st) {
        return true;
    }
    *slot(st, zs) = saved;
    false
}

fn slot(st: &mut State, zs: bool) -> &mut Option<usize> {
    if zs { &mut st.zs } else { &mut st.ze }
}
