//! Vim's patterns, the part of them a learner meets: `.` `*` `\+` `\=` `\?`
//! `\{n,m}` (and `\{-}` lazily), `[abc]` `[^a-z]`, `\d \w \s \a \l \u \x`
//! and their capitals, `^` `$` `\<` `\>`, `\(…\)` `\%(…\)` `\1`, `\|`,
//! `\zs` `\ze`, and `\v` (very magic: `( ) | + ? = { } < >` without the
//! backslash). `\c` and `\C` are read by the caller (they set the case).
//!
//! A pattern never fails to compile: what doesn't parse is taken literally,
//! as vim mostly does.

/// What one item of a pattern matches.
#[derive(Debug, Clone)]
enum Node {
    Char(char),
    Any,
    Class(Vec<Range>, bool),
    Start,
    End,
    WordStart,
    WordEnd,
    Zs,
    Ze,
    Backref(usize),
    Group(Box<Node>, Option<usize>),
    Alt(Vec<Node>),
    Concat(Vec<Node>),
    Repeat {
        node: Box<Node>,
        min: usize,
        max: usize,
        greedy: bool,
    },
}

/// A character class item: an inclusive range, or a named class.
#[derive(Debug, Clone, Copy)]
enum Range {
    Span(char, char),
    Named(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Lit(char),
    Any,
    Class(Vec<RangeTok>, bool),
    Named(char),
    Start,
    End,
    WordStart,
    WordEnd,
    Zs,
    Ze,
    Backref(usize),
    Open(bool),
    Close,
    Bar,
    Star,
    Plus,
    Opt,
    Brace(usize, usize, bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RangeTok {
    Span(char, char),
    Named(char),
}

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

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

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

fn tokenize(pat: &str) -> Vec<Tok> {
    let chars: Vec<char> = pat.chars().collect();
    let mut out = Vec::new();
    let mut very = false;
    let mut i = 0;
    // Whether `^` here starts a branch, so anchors.
    let starts_branch = |out: &Vec<Tok>| matches!(out.last(), None | Some(Tok::Open(_) | Tok::Bar));
    while i < chars.len() {
        let c = chars[i];
        // In very magic mode these need no backslash.
        if very && "()|+?={}<>%".contains(c) {
            match c {
                '(' => out.push(Tok::Open(true)),
                ')' => out.push(Tok::Close),
                '|' => out.push(Tok::Bar),
                '+' => out.push(Tok::Plus),
                '?' | '=' => out.push(Tok::Opt),
                '<' => out.push(Tok::WordStart),
                '>' => out.push(Tok::WordEnd),
                '%' if chars.get(i + 1) == Some(&'(') => {
                    out.push(Tok::Open(false));
                    i += 1;
                }
                '{' => {
                    let (tok, used) = brace(&chars[i + 1..]);
                    match tok {
                        Some(t) => {
                            out.push(t);
                            i += used;
                        }
                        None => out.push(Tok::Lit('{')),
                    }
                }
                other => out.push(Tok::Lit(other)),
            }
            i += 1;
            continue;
        }
        match c {
            '\\' => {
                let Some(&n) = chars.get(i + 1) else {
                    out.push(Tok::Lit('\\'));
                    break;
                };
                i += 2;
                match n {
                    'v' => very = true,
                    'm' | 'M' | 'V' => very = false,
                    'c' | 'C' => {}
                    '|' if !very => out.push(Tok::Bar),
                    '(' if !very => out.push(Tok::Open(true)),
                    ')' if !very => out.push(Tok::Close),
                    '%' if !very && chars.get(i) == Some(&'(') => {
                        out.push(Tok::Open(false));
                        i += 1;
                    }
                    '+' if !very => out.push(Tok::Plus),
                    '=' | '?' if !very => out.push(Tok::Opt),
                    '<' if !very => out.push(Tok::WordStart),
                    '>' if !very => out.push(Tok::WordEnd),
                    '{' if !very => {
                        let (tok, used) = brace(&chars[i..]);
                        match tok {
                            Some(t) => {
                                out.push(t);
                                i += used;
                            }
                            None => out.push(Tok::Lit('{')),
                        }
                    }
                    'z' if matches!(chars.get(i), Some('s' | 'e')) => {
                        out.push(if chars[i] == 's' { Tok::Zs } else { Tok::Ze });
                        i += 1;
                    }
                    '1'..='9' => out.push(Tok::Backref(n as usize - '0' as usize)),
                    't' => out.push(Tok::Lit('\t')),
                    'n' | 'r' => out.push(Tok::Lit('\n')),
                    'd' | 'D' | 'w' | 'W' | 's' | 'S' | 'a' | 'A' | 'l' | 'L' | 'u' | 'U' | 'x' | 'X'
                    | 'h' | 'H' => out.push(Tok::Named(n)),
                    other => out.push(Tok::Lit(other)),
                }
                continue;
            }
            '.' => out.push(Tok::Any),
            '*' if !starts_branch(&out) => out.push(Tok::Star),
            '^' if starts_branch(&out) => out.push(Tok::Start),
            '$' => {
                let next_ends = match chars.get(i + 1) {
                    None => true,
                    Some('\\') => matches!(chars.get(i + 2), Some('|' | ')')) && !very,
                    Some('|' | ')') => very,
                    _ => false,
                };
                out.push(if next_ends { Tok::End } else { Tok::Lit('$') });
            }
            '[' => match class(&chars[i + 1..]) {
                Some((tok, used)) => {
                    out.push(tok);
                    i += used;
                }
                None => out.push(Tok::Lit('[')),
            },
            other => out.push(Tok::Lit(other)),
        }
        i += 1;
    }
    out
}

/// `\{n,m}` after the brace: the token and the characters it used.
fn brace(rest: &[char]) -> (Option<Tok>, usize) {
    let Some(close) = rest.iter().position(|&c| c == '}') else {
        return (None, 0);
    };
    let mut body: String = rest[..close].iter().collect();
    // `\{…\}` closes with a backslash in magic mode.
    if body.ends_with('\\') {
        body.pop();
    }
    let lazy = body.starts_with('-');
    let body = body.trim_start_matches('-');
    let num = |s: &str| {
        if s.is_empty() {
            None
        } else {
            s.parse::<usize>().ok()
        }
    };
    let (min, max) = match body.split_once(',') {
        Some((a, b)) => (num(a).unwrap_or(0), num(b).unwrap_or(usize::MAX)),
        None if body.is_empty() => (0, usize::MAX),
        None => match num(body) {
            Some(n) => (n, n),
            None => return (None, 0),
        },
    };
    (Some(Tok::Brace(min, max, !lazy)), close + 1)
}

/// `[…]` after the bracket: the token and the characters it used.
fn class(rest: &[char]) -> Option<(Tok, usize)> {
    let mut i = 0;
    let neg = rest.first() == Some(&'^');
    if neg {
        i += 1;
    }
    let mut items = Vec::new();
    let mut first = true;
    loop {
        let c = *rest.get(i)?;
        if c == ']' && !first {
            return Some((Tok::Class(items, neg), i + 1));
        }
        first = false;
        let lit = if c == '\\' {
            let n = *rest.get(i + 1)?;
            i += 2;
            match n {
                'd' | 'D' | 'w' | 'W' | 's' | 'S' | 'a' | 'A' | 'l' | 'L' | 'u' | 'U' | 'x' | 'X' | 'h'
                | 'H' => {
                    items.push(RangeTok::Named(n));
                    continue;
                }
                't' => '\t',
                other => other,
            }
        } else {
            i += 1;
            c
        };
        if rest.get(i) == Some(&'-') && rest.get(i + 1).is_some_and(|&c| c != ']') {
            let hi = rest[i + 1];
            items.push(RangeTok::Span(lit, hi));
            i += 2;
        } else {
            items.push(RangeTok::Span(lit, lit));
        }
    }
}

struct Parser {
    toks: Vec<Tok>,
    at: usize,
    groups: usize,
}

impl Parser {
    fn alt(&mut self) -> Node {
        let mut branches = vec![self.concat()];
        while self.toks.get(self.at) == Some(&Tok::Bar) {
            self.at += 1;
            branches.push(self.concat());
        }
        if branches.len() == 1 {
            branches.pop().unwrap()
        } else {
            Node::Alt(branches)
        }
    }

    fn concat(&mut self) -> Node {
        let mut items = Vec::new();
        while let Some(t) = self.toks.get(self.at).cloned() {
            let atom = match t {
                Tok::Bar | Tok::Close => break,
                Tok::Lit(c) => Node::Char(c),
                Tok::Any => Node::Any,
                Tok::Named(c) => Node::Class(vec![Range::Named(c)], false),
                Tok::Class(items, neg) => Node::Class(
                    items
                        .into_iter()
                        .map(|r| match r {
                            RangeTok::Span(a, b) => Range::Span(a, b),
                            RangeTok::Named(c) => Range::Named(c),
                        })
                        .collect(),
                    neg,
                ),
                Tok::Start => Node::Start,
                Tok::End => Node::End,
                Tok::WordStart => Node::WordStart,
                Tok::WordEnd => Node::WordEnd,
                Tok::Zs => Node::Zs,
                Tok::Ze => Node::Ze,
                Tok::Backref(n) => Node::Backref(n),
                Tok::Open(capture) => {
                    self.at += 1;
                    let n = capture.then(|| {
                        self.groups += 1;
                        self.groups
                    });
                    let inner = self.alt();
                    // A missing `\)` closes at the end.
                    if self.toks.get(self.at) == Some(&Tok::Close) {
                        self.at += 1;
                    }
                    items.push(self.multi(Node::Group(Box::new(inner), n.filter(|&n| n <= 9))));
                    continue;
                }
                // A multi with nothing before it is taken literally.
                Tok::Star => Node::Char('*'),
                Tok::Plus => Node::Char('+'),
                Tok::Opt => Node::Char('='),
                Tok::Brace(..) => Node::Char('{'),
            };
            self.at += 1;
            items.push(self.multi(atom));
        }
        if items.len() == 1 {
            items.pop().unwrap()
        } else {
            Node::Concat(items)
        }
    }

    fn multi(&mut self, atom: Node) -> Node {
        let (min, max, greedy) = match self.toks.get(self.at) {
            Some(Tok::Star) => (0, usize::MAX, true),
            Some(Tok::Plus) => (1, usize::MAX, true),
            Some(Tok::Opt) => (0, 1, true),
            Some(&Tok::Brace(a, b, g)) => (a, b, g),
            _ => return atom,
        };
        self.at += 1;
        Node::Repeat {
            node: Box::new(atom),
            min,
            max,
            greedy,
        }
    }
}

#[derive(Clone)]
struct State {
    groups: [Option<(usize, usize)>; 10],
    zs: Option<usize>,
    ze: Option<usize>,
}

type K<'a> = &'a mut dyn FnMut(usize, &mut State) -> bool;

impl Regex {
    pub fn new(pat: &str) -> Regex {
        let mut p = Parser {
            toks: tokenize(pat),
            at: 0,
            groups: 0,
        };
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

struct Matcher<'l> {
    line: &'l [char],
    icase: bool,
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

    fn node(&self, n: &Node, pos: usize, st: &mut State, k: K) -> bool {
        let line = self.line;
        let one = |ok: bool, st: &mut State, k: K| ok && k(pos + 1, st);
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
            Node::WordStart => {
                let here = line.get(pos).is_some_and(|&c| is_word(c));
                let before = pos > 0 && is_word(line[pos - 1]);
                here && !before && k(pos, st)
            }
            Node::WordEnd => {
                let before = pos > 0 && is_word(line[pos - 1]);
                let here = line.get(pos).is_some_and(|&c| is_word(c));
                before && !here && k(pos, st)
            }
            Node::Zs | Node::Ze => {
                let zs = matches!(n, Node::Zs);
                let saved = if zs {
                    st.zs.replace(pos)
                } else {
                    st.ze.replace(pos)
                };
                if k(pos, st) {
                    return true;
                }
                if zs {
                    st.zs = saved;
                } else {
                    st.ze = saved;
                }
                false
            }
            Node::Backref(g) => {
                let Some((a, b)) = st.groups[*g] else {
                    return k(pos, st);
                };
                let len = b - a;
                if pos + len > line.len() || !(0..len).all(|i| self.same(line[pos + i], line[a + i])) {
                    return false;
                }
                k(pos + len, st)
            }
            Node::Group(inner, g) => {
                let g = *g;
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
            Node::Alt(branches) => branches.iter().any(|b| self.node(b, pos, st, &mut *k)),
            Node::Concat(items) => self.seq(items, pos, st, k),
            Node::Repeat {
                node,
                min,
                max,
                greedy,
            } => self.repeat(node, *min, *max, *greedy, 0, pos, st, k),
        }
    }

    fn seq(&self, items: &[Node], pos: usize, st: &mut State, k: K) -> bool {
        match items.split_first() {
            None => k(pos, st),
            Some((first, rest)) => self.node(first, pos, st, &mut |p, st| self.seq(rest, p, st, k)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn repeat(
        &self,
        n: &Node,
        min: usize,
        max: usize,
        greedy: bool,
        count: usize,
        pos: usize,
        st: &mut State,
        k: K,
    ) -> bool {
        let more = |st: &mut State, k: K| {
            count < max
                && self.node(n, pos, st, &mut |p, st| {
                    // A match of nothing ends the repeat (or it would loop).
                    if p == pos {
                        count + 1 >= min && k(p, st)
                    } else {
                        self.repeat(n, min, max, greedy, count + 1, p, st, k)
                    }
                })
        };
        if greedy {
            more(st, &mut *k) || (count >= min && k(pos, st))
        } else {
            (count >= min && k(pos, &mut *st)) || more(st, k)
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn find(pat: &str, text: &str) -> Option<(usize, usize)> {
        let line: Vec<char> = text.chars().collect();
        Regex::new(pat)
            .find_from(&line, 0, false)
            .map(|m| (m.start, m.end))
    }

    fn sub(pat: &str, rep: &str, text: &str) -> String {
        let line: Vec<char> = text.chars().collect();
        let m = Regex::new(pat).find_from(&line, 0, false).unwrap();
        let mut out = line[..m.start].to_vec();
        out.extend(expand(rep, &line, &m));
        out.extend_from_slice(&line[m.end..]);
        out.into_iter().collect()
    }

    #[test]
    fn literals_and_dots() {
        assert_eq!(find("cat", "a cat sat"), Some((2, 5)));
        assert_eq!(find("c.t", "a cot"), Some((2, 5)));
        assert_eq!(find(r"1\.0", "1x0 1.0"), Some((4, 7)));
        assert_eq!(find("^a", "ba"), None);
        assert_eq!(find("a$", "ab a"), Some((3, 4)));
        assert_eq!(find("a$b", "xa$b"), Some((1, 4)));
        assert_eq!(find("*", "a*"), Some((1, 2)));
    }

    #[test]
    fn repeats() {
        assert_eq!(find("lo*", "hellooo"), Some((2, 3)));
        assert_eq!(find(r"o\+", "hellooo!"), Some((4, 7)));
        assert_eq!(find(r"colou\=r", "color"), Some((0, 5)));
        assert_eq!(find(r"colou\?r", "colour"), Some((0, 6)));
        assert_eq!(find(r"a\{2}", "a aa"), Some((2, 4)));
        assert_eq!(find(r"a\{2,3}", "aaaa"), Some((0, 3)));
        assert_eq!(find(r"<.\{-}>", "<a><b>"), Some((0, 3)));
        assert_eq!(find("<.*>", "<a><b>"), Some((0, 6)));
    }

    #[test]
    fn classes() {
        assert_eq!(find(r"\d\+", "abc 123 x"), Some((4, 7)));
        assert_eq!(find("[aeiou]", "xyz e"), Some((4, 5)));
        assert_eq!(find("[^a-z ]", "abc D"), Some((4, 5)));
        assert_eq!(find(r"\s\+$", "abc   "), Some((3, 6)));
        assert_eq!(find(r"\u\l\+", "abc Def"), Some((4, 7)));
        assert_eq!(find(r"\w\+", "  foo_1 "), Some((2, 7)));
        assert_eq!(find("[]x]", "a]"), Some((1, 2)));
    }

    #[test]
    fn alternation_groups_and_words() {
        assert_eq!(find(r"cat\|dog", "a dog"), Some((2, 5)));
        assert_eq!(find(r"\<is\>", "this is"), Some((5, 7)));
        assert_eq!(find(r"\(ab\)\+", "xababc"), Some((1, 5)));
        assert_eq!(find(r"\(a\)\1", "a aa"), Some((2, 4)));
        assert_eq!(find(r"foo\zsbar", "foobar"), Some((3, 6)));
        assert_eq!(find(r"foo\zebar", "foo foobar"), Some((4, 7)));
        assert_eq!(find(r"\v(cat|dog)s?", "dogs"), Some((0, 4)));
        assert_eq!(find(r"\v<\d{3}>", "12 345 6789"), Some((3, 6)));
    }

    #[test]
    fn replacements() {
        assert_eq!(sub(r"\(\w\+\) \(\w\+\)", r"\2 \1", "hello world"), "world hello");
        assert_eq!(sub(r"\w\+", r"\u&", "word"), "Word");
        assert_eq!(sub(r"\w\+", r"\U&\E!", "loud"), "LOUD!");
        assert_eq!(sub("a", r"\&&", "a"), "&a");
    }

    #[test]
    fn case() {
        let line: Vec<char> = "Hello".chars().collect();
        assert!(Regex::new("hello").match_here(&line, 0, true).is_some());
        assert!(Regex::new("[a-z]ello").match_here(&line, 0, true).is_some());
        assert!(Regex::new("hello").match_here(&line, 0, false).is_none());
    }
}
