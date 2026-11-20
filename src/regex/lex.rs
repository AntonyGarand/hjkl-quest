//! Reading a pattern into tokens, magic or very magic (`\v`).

/// One token of a pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Tok {
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

/// An item of a `[…]` class: an inclusive range, or a named class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RangeTok {
    Span(char, char),
    Named(char),
}

/// The named classes, `\d` to `\H`.
const NAMED: &str = "dDwWsSaAlLuUxXhH";

/// What very magic mode reads without a backslash.
const VERY_MAGIC: &str = "()|+?={}<>%";

struct Lexer {
    chars: Vec<char>,
    i: usize,
    /// Very magic mode (`\v`).
    very: bool,
    out: Vec<Tok>,
}

pub(super) fn tokenize(pat: &str) -> Vec<Tok> {
    let mut lx = Lexer {
        chars: pat.chars().collect(),
        i: 0,
        very: false,
        out: Vec::new(),
    };
    while let Some(&c) = lx.chars.get(lx.i) {
        if lx.very && VERY_MAGIC.contains(c) {
            lx.very_magic(c);
        } else if c == '\\' {
            if !lx.escaped() {
                break;
            }
        } else {
            lx.plain(c);
        }
    }
    lx.out
}

impl Lexer {
    /// Whether a `^` here starts a branch, so anchors.
    fn starts_branch(&self) -> bool {
        matches!(self.out.last(), None | Some(Tok::Open(_) | Tok::Bar))
    }

    /// `\{n,m}` with its body at `from`, or a literal brace.
    fn brace_at(&mut self, from: usize) {
        match brace(&self.chars[from..]) {
            (Some(t), used) => {
                self.out.push(t);
                self.i += used;
            }
            (None, _) => self.out.push(Tok::Lit('{')),
        }
    }

    /// A character that needs no backslash in very magic mode.
    fn very_magic(&mut self, c: char) {
        match c {
            '(' => self.out.push(Tok::Open(true)),
            ')' => self.out.push(Tok::Close),
            '|' => self.out.push(Tok::Bar),
            '+' => self.out.push(Tok::Plus),
            '?' | '=' => self.out.push(Tok::Opt),
            '<' => self.out.push(Tok::WordStart),
            '>' => self.out.push(Tok::WordEnd),
            '%' if self.chars.get(self.i + 1) == Some(&'(') => {
                self.out.push(Tok::Open(false));
                self.i += 1;
            }
            '{' => self.brace_at(self.i + 1),
            other => self.out.push(Tok::Lit(other)),
        }
        self.i += 1;
    }

    /// A backslash and what follows it. Returns false at a trailing
    /// backslash, which ends the pattern.
    fn escaped(&mut self) -> bool {
        let Some(&n) = self.chars.get(self.i + 1) else {
            self.out.push(Tok::Lit('\\'));
            return false;
        };
        self.i += 2;
        if self.very {
            self.escaped_char(n);
        } else {
            self.magic(n);
        }
        true
    }

    /// In magic mode, what a backslash makes special.
    fn magic(&mut self, n: char) {
        match n {
            '|' => self.out.push(Tok::Bar),
            '(' => self.out.push(Tok::Open(true)),
            ')' => self.out.push(Tok::Close),
            '%' if self.chars.get(self.i) == Some(&'(') => {
                self.out.push(Tok::Open(false));
                self.i += 1;
            }
            '+' => self.out.push(Tok::Plus),
            '=' | '?' => self.out.push(Tok::Opt),
            '<' => self.out.push(Tok::WordStart),
            '>' => self.out.push(Tok::WordEnd),
            '{' => self.brace_at(self.i),
            _ => self.escaped_char(n),
        }
    }

    /// A backslash item the same in every mode.
    fn escaped_char(&mut self, n: char) {
        match n {
            'v' => self.very = true,
            'm' | 'M' | 'V' => self.very = false,
            'c' | 'C' => {}
            'z' if matches!(self.chars.get(self.i), Some('s' | 'e')) => {
                self.out.push(if self.chars[self.i] == 's' {
                    Tok::Zs
                } else {
                    Tok::Ze
                });
                self.i += 1;
            }
            '1'..='9' => self.out.push(Tok::Backref(n as usize - '0' as usize)),
            't' => self.out.push(Tok::Lit('\t')),
            'n' | 'r' => self.out.push(Tok::Lit('\n')),
            n if NAMED.contains(n) => self.out.push(Tok::Named(n)),
            other => self.out.push(Tok::Lit(other)),
        }
    }

    /// A character without a backslash.
    fn plain(&mut self, c: char) {
        match c {
            '.' => self.out.push(Tok::Any),
            '*' if !self.starts_branch() => self.out.push(Tok::Star),
            '^' if self.starts_branch() => self.out.push(Tok::Start),
            '$' => {
                let tok = if self.ends_branch() {
                    Tok::End
                } else {
                    Tok::Lit('$')
                };
                self.out.push(tok);
            }
            '[' => match class(&self.chars[self.i + 1..]) {
                Some((tok, used)) => {
                    self.out.push(tok);
                    self.i += used;
                }
                None => self.out.push(Tok::Lit('[')),
            },
            other => self.out.push(Tok::Lit(other)),
        }
        self.i += 1;
    }

    /// Whether the `$` here ends a branch, so anchors.
    fn ends_branch(&self) -> bool {
        match self.chars.get(self.i + 1) {
            None => true,
            Some('\\') => matches!(self.chars.get(self.i + 2), Some('|' | ')')) && !self.very,
            Some('|' | ')') => self.very,
            _ => false,
        }
    }
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
                n if NAMED.contains(n) => {
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
