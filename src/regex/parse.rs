//! Tokens into a tree of nodes: branches, sequences, groups and repeats.

use super::lex::{RangeTok, Tok};

/// What one item of a pattern matches.
#[derive(Debug, Clone)]
pub(super) enum Node {
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
    Repeat(Repeat),
}

/// A node matched `min` to `max` times, as many as can be if `greedy`,
/// else as few.
#[derive(Debug, Clone)]
pub(super) struct Repeat {
    pub(super) node: Box<Node>,
    pub(super) min: usize,
    pub(super) max: usize,
    pub(super) greedy: bool,
}

/// A character class item: an inclusive range, or a named class.
#[derive(Debug, Clone, Copy)]
pub(super) enum Range {
    Span(char, char),
    Named(char),
}

pub(super) struct Parser {
    pub(super) toks: Vec<Tok>,
    pub(super) at: usize,
    groups: usize,
}

/// A token that matches by itself, as a node.
fn atom(t: Tok) -> Node {
    match t {
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
        // Groups and branches are the parser's; anything else here (a
        // multi with nothing before it) is taken literally.
        Tok::Open(_) => Node::Char('('),
        Tok::Close => Node::Char(')'),
        Tok::Bar => Node::Char('|'),
        Tok::Star => Node::Char('*'),
        Tok::Plus => Node::Char('+'),
        Tok::Opt => Node::Char('='),
        Tok::Brace(..) => Node::Char('{'),
    }
}

impl Parser {
    pub(super) fn new(toks: Vec<Tok>) -> Self {
        Parser {
            toks,
            at: 0,
            groups: 0,
        }
    }

    /// Branches separated by `\|`.
    pub(super) fn alt(&mut self) -> Node {
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

    /// Items one after another, up to the end of the branch.
    fn concat(&mut self) -> Node {
        let mut items = Vec::new();
        while let Some(t) = self.toks.get(self.at).cloned() {
            if matches!(t, Tok::Bar | Tok::Close) {
                break;
            }
            self.at += 1;
            let item = match t {
                Tok::Open(capture) => self.group(capture),
                t => atom(t),
            };
            items.push(self.multi(item));
        }
        if items.len() == 1 {
            items.pop().unwrap()
        } else {
            Node::Concat(items)
        }
    }

    /// A group, after its `\(` (numbered if it captures).
    fn group(&mut self, capture: bool) -> Node {
        let n = capture.then(|| {
            self.groups += 1;
            self.groups
        });
        let inner = self.alt();
        // A missing `\)` closes at the end.
        if self.toks.get(self.at) == Some(&Tok::Close) {
            self.at += 1;
        }
        Node::Group(Box::new(inner), n.filter(|&n| n <= 9))
    }

    /// `atom`, with the multi after it if there is one.
    fn multi(&mut self, atom: Node) -> Node {
        let (min, max, greedy) = match self.toks.get(self.at) {
            Some(Tok::Star) => (0, usize::MAX, true),
            Some(Tok::Plus) => (1, usize::MAX, true),
            Some(Tok::Opt) => (0, 1, true),
            Some(&Tok::Brace(a, b, g)) => (a, b, g),
            _ => return atom,
        };
        self.at += 1;
        Node::Repeat(Repeat {
            node: Box::new(atom),
            min,
            max,
            greedy,
        })
    }
}
