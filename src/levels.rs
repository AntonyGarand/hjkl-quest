//! The campaign: twenty isles of the Buffer Archipelago, each teaching a few
//! commands. Every level's `solution` is a vim-golf line: the tests replay it
//! and check that it solves the level in exactly `par` keystrokes.

use crate::vim::{Pos, parse_keys, pos};

pub struct Level {
    pub chapter: &'static str,
    pub title: &'static str,
    pub story: &'static str,
    /// The commands this level introduces: (keys, what they do).
    pub teaches: &'static [(&'static str, &'static str)],
    /// The normal-mode keys this level unlocks.
    pub unlock: &'static str,
    /// The buffer: the island the player walks on.
    pub map: &'static str,
    /// An overlay of `map`: `@` the start, `*` a crystal to collect.
    pub marks: &'static str,
    /// For editing levels, the text the buffer must become.
    pub goal: Option<&'static str>,
    /// A shortest known solution, in vim-golf notation.
    pub solution: &'static str,
    pub tip: &'static str,
}

impl Level {
    pub fn start(&self) -> Pos {
        self.marked('@').into_iter().next().unwrap_or_default()
    }

    pub fn gems(&self) -> Vec<Pos> {
        self.marked('*')
    }

    fn marked(&self, mark: char) -> Vec<Pos> {
        let mut out = Vec::new();
        for (row, line) in self.marks.trim_start_matches('\n').split('\n').enumerate() {
            for (col, c) in line.chars().enumerate() {
                if c == mark {
                    out.push(pos(row, col));
                }
            }
        }
        out
    }

    pub fn text(&self) -> &'static str {
        self.map.trim_start_matches('\n')
    }

    pub fn goal_text(&self) -> Option<&'static str> {
        self.goal.map(|g| g.trim_start_matches('\n'))
    }

    pub fn par(&self) -> usize {
        parse_keys(self.solution).len()
    }
}

/// The normal-mode keys allowed in level `index`: everything unlocked so far.
pub fn unlocked(index: usize) -> String {
    LEVELS[..=index.min(LEVELS.len() - 1)].iter().map(|l| l.unlock).collect()
}

pub static LEVELS: &[Level] = &[
    Level {
        chapter: "I · The Meadow",
        title: "First Steps",
        story: "You wake on a quiet meadow as a small cursor-wisp. Your arrows are gone. \
                Your fingers rest on the home row, where they belong.",
        teaches: &[("h", "step left"), ("j", "step down"), ("k", "step up"), ("l", "step right")],
        unlock: "hjkl",
        map: "
.........
.........
.........
.........
.........",
        marks: "
@........
.........
....*....
.........
........*",
        goal: None,
        solution: "jjlllljjllll",
        tip: "j looks like an arrow pointing down. Keep your fingers on the home row.",
    },
    Level {
        chapter: "I · The Meadow",
        title: "Counting Sheep",
        story: "The meadow stretches far. Stepping one tile at a time is slow. \
                Say how many steps you want first.",
        teaches: &[("{n}l", "a count repeats a motion: 5l moves five right"), ("3j", "three lines down")],
        unlock: "123456789",
        map: "
....................
....................
....................
....................
....................
....................",
        marks: "
@..............*....
....................
....................
....................
....................
..*............*....",
        goal: None,
        solution: "15l5j13h",
        tip: "Counts work with almost every command in vim.",
    },
    Level {
        chapter: "II · The Word Bridges",
        title: "Word Hopping",
        story: "A bridge of words crosses the gorge. A wisp can leap whole words at once.",
        teaches: &[
            ("w", "to the start of the next word"),
            ("b", "back to the start of a word"),
            ("e", "to the end of a word"),
        ],
        unlock: "wbe",
        map: "
the quick brown fox jumps over the lazy dog",
        marks: "
          *         @     *           *",
        goal: None,
        solution: "w3e6b",
        tip: "Words are runs of letters, digits and _, or runs of punctuation.",
    },
    Level {
        chapter: "II · The Word Bridges",
        title: "Punctuation Pass",
        story: "Dots and dashes split this bridge into tiny words. WORDs ignore the \
                punctuation and only stop at spaces.",
        teaches: &[
            ("W", "next WORD (anything between spaces)"),
            ("B", "back one WORD"),
            ("E", "end of a WORD"),
        ],
        unlock: "WBE",
        map: "
a.b.c d.e.f g.h.i j.k.l",
        marks: "
    *       *         @",
        goal: None,
        solution: "2B2BE",
        tip: "Use the capital versions when code is full of dots, commas and brackets.",
    },
    Level {
        chapter: "II · The Word Bridges",
        title: "Edge of the World",
        story: "The cliffs are ragged and indented. Jump straight to the edges of a line.",
        teaches: &[
            ("0", "to the very first column"),
            ("^", "to the first non-blank character"),
            ("$", "to the end of the line"),
        ],
        unlock: "0^$",
        map: "
    the cliffs edge
  stand right here
        far   east  end",
        marks: "
    @             *
  *
*                     *",
        goal: None,
        solution: "$j^j0$",
        tip: "After $, moving with j or k stays at the end of each line.",
    },
    Level {
        chapter: "III · The Tower",
        title: "Long Jumps",
        story: "A tall tower of floors. Climbing with j and k would take ages. \
                Jump to any floor by its number.",
        teaches: &[
            ("gg", "to the first line"),
            ("G", "to the last line"),
            ("{n}G", "to line n (8G goes to line 8)"),
        ],
        unlock: "gG",
        map: "
roof
floor eleven
floor ten
floor nine
floor eight
floor seven
floor six
floor five
floor four
floor three
floor two
cellar",
        marks: "
*


@



*



*",
        goal: None,
        solution: "G8Ggg",
        tip: "Vim shows line numbers with :set number. Here, the floor names do the counting.",
    },
    Level {
        chapter: "III · The Tower",
        title: "Seek and Find",
        story: "The dog hid crystals next to punctuation. Look ahead along the line and \
                land right on (or just before) a character.",
        teaches: &[
            ("f{c}", "forward onto character c"),
            ("F{c}", "backward onto character c"),
            ("t{c}", "forward until just before c"),
            ("T{c}", "backward until just after c"),
            (";  ,", "repeat the last f/t/F/T, or repeat it the other way"),
        ],
        unlock: "fFtT;,",
        map: "
go fetch the ball, bring it back, good dog!",
        marks: "
@                * *        *   *   *     *",
        goal: None,
        solution: "f,;tdf!Fb;",
        tip: "f and t are the fastest way to move inside a line. ; repeats them for free.",
    },
    Level {
        chapter: "III · The Tower",
        title: "Paragraph Isles",
        story: "Small isles float apart, joined by empty planks. Leap from one blank line \
                to the next.",
        teaches: &[("}", "forward to the next blank line"), ("{", "back to the previous blank line")],
        unlock: "{}",
        map: "
the first isle
has two lines

second isle is
a bit bigger
than the first

third isle

last isle",
        marks: "
@

*



*


        *",
        goal: None,
        solution: "}}2}",
        tip: "In code, blank lines usually separate functions, so { and } jump between them.",
    },
    Level {
        chapter: "IV · The Machine",
        title: "Mirror Matching",
        story: "The old machine is all brackets. Every bracket has a twin, and % jumps \
                between them.",
        teaches: &[("%", "jump to the bracket that matches ( [ {")],
        unlock: "%",
        map: "
fn main() {
    if (ready(now)) {
        launch([1, 2, 3]);
    }
}",
        marks: "
@         *
                  *
                       *

*",
        goal: None,
        solution: "j%j%%G%",
        tip: "If the cursor isn't on a bracket, % uses the first bracket after it on the line.",
    },
    Level {
        chapter: "IV · The Machine",
        title: "Search Party",
        story: "Gold is buried somewhere under the sand. Search for it and let n take \
                you to the next hit.",
        teaches: &[
            ("/text⏎", "search forward for text"),
            ("n  N", "next or previous match"),
            ("*  #", "search for the word under the cursor"),
        ],
        unlock: "/nN*#",
        map: "
sand rock gold sand
rock sand rock rock
sand gold rock sand
rock rock sand gold",
        marks: "
@         *

     *
               *",
        goal: None,
        solution: "/g<CR>nn",
        tip: "Golfers search for the shortest unique pattern. Here a single g is enough.",
    },
    Level {
        chapter: "V · The Workshop",
        title: "Chomp",
        story: "Typos have crept into the old sign. Remove the extra letters and fix \
                the wrong one.",
        teaches: &[
            ("x", "delete the character under the cursor"),
            ("X", "delete the character before the cursor"),
            ("r{c}", "replace the character under the cursor with c"),
        ],
        unlock: "xXr",
        map: "
Thee cat zat onn thee mat",
        marks: "@",
        goal: Some("
The cat sat on the mat"),
        solution: "3lxfzrsfnxfex",
        tip: "Glowing tiles don't match the goal yet. Fix them all to finish.",
    },
    Level {
        chapter: "V · The Workshop",
        title: "Demolition",
        story: "Time to tear down whole chunks of text. d takes a motion: whatever the \
                motion passes over gets deleted.",
        teaches: &[
            ("d{motion}", "delete over a motion: dw, de, d$, dj, dt,"),
            ("dd", "delete the whole line"),
            ("D", "delete to the end of the line (d$)"),
        ],
        unlock: "dD",
        map: "
keep this line
remove remove this line
keep this too
cut here: and all of this
very very good",
        marks: "@",
        goal: Some("
keep this line
keep this too
cut here:
very good"),
        solution: "jddjf:lDjbdw",
        tip: "Operators + motions form a language: d (delete) w (word). Learn both and combine them.",
    },
    Level {
        chapter: "V · The Workshop",
        title: "Change of Heart",
        story: "Deleting and then typing is two steps. c does both: it deletes over a motion \
                and drops you into insert mode.",
        teaches: &[
            ("c{motion}", "change: cw, ce, c$, then type and press Esc"),
            ("C", "change to the end of the line"),
            ("s  S", "change one character, or the whole line"),
            ("Esc", "back to normal mode"),
        ],
        unlock: "cCsS",
        map: "
I hate vim and I hate bugs
status: broken forever",
        marks: "@",
        goal: Some("
I love vim and I squash bugs
status: fixed"),
        solution: "wcwlove<Esc>4wcwsquash<Esc>j2BCfixed<Esc>",
        tip: "cw acts like ce: it keeps the space after the word.",
    },
    Level {
        chapter: "VI · The Library",
        title: "Insert Coin",
        story: "The librarian's list is missing words. Choose where to start typing \
                so you don't have to move first.",
        teaches: &[
            ("i  a", "insert before, or append after, the cursor"),
            ("I  A", "insert at the start, or append at the end, of the line"),
            ("o  O", "open a new line below, or above"),
        ],
        unlock: "iaIAoO",
        map: "
list
- apple
- pear",
        marks: "@",
        goal: Some("
my list:
- apple
- ripe pear!
- fig"),
        solution: "Imy <Esc>A:<Esc>jjA!<Esc>biripe <Esc>o- fig<Esc>",
        tip: "A and o save keystrokes because they move and enter insert mode in one key.",
    },
    Level {
        chapter: "VI · The Library",
        title: "Copycat",
        story: "Why build a fence plank by plank? Yank one and put it as often as you like.",
        teaches: &[
            ("y{motion}", "yank (copy): yw, y$"),
            ("yy  Y", "yank the whole line"),
            ("p  P", "put after, or before, the cursor"),
        ],
        unlock: "yYpP",
        map: "
hip hooray
==",
        marks: "@",
        goal: Some("
hip hip hooray
==
==
=="),
        solution: "ywPjyy2p",
        tip: "Deleting also yanks: ddp swaps two lines and xp swaps two characters.",
    },
    Level {
        chapter: "VI · The Library",
        title: "Time Travel",
        story: "The archivist repeats the same edit everywhere. The dot command \
                repeats your last change, and u undoes mistakes.",
        teaches: &[
            (".", "repeat the last change"),
            ("u", "undo"),
            ("Ctrl-r", "redo"),
        ],
        unlock: "u.",
        map: "
x = 1
y = 2
z = 3
bad bad bad good",
        marks: "@",
        goal: Some("
x = 1;
y = 2;
z = 3;
good"),
        solution: "A;<Esc>j.j.j0d3w",
        tip: "Make each change a single repeatable action, then press . wherever it's needed.",
    },
    Level {
        chapter: "VII · The Observatory",
        title: "Inside Out",
        story: "The astronomers keep their data in brackets and quotes. Text objects \
                select a whole thing at once, wherever you are inside it.",
        teaches: &[
            ("ci\"", "change inside quotes (also i' and i`)"),
            ("di(  da(", "delete inside, or around, brackets (also [ { <)"),
            ("ciw  daw", "change inner word, delete a word (with its space)"),
        ],
        unlock: "",
        map: "
print(\"hello world\")
items = [1, 2, 3, 4]
call(a, b(c), d)",
        marks: "@",
        goal: Some("
print(\"bye\")
items = []
call()"),
        solution: "ci\"bye<Esc>jdi[jF(di(",
        tip: "i means inner (only the contents) and a means around (including the brackets or quotes).",
    },
    Level {
        chapter: "VII · The Observatory",
        title: "Highlights",
        story: "Sometimes it's easier to see what you're about to change. Visual mode \
                highlights a selection first, then you act on it.",
        teaches: &[
            ("v", "visual mode: select characters"),
            ("V", "visual line mode: select whole lines"),
            ("U  u  ~", "upper-case, lower-case, toggle case"),
            ("J", "join this line with the next"),
        ],
        unlock: "vV~UJ",
        map: "
shout this out
keep
drop
drop
keep",
        marks: "@",
        goal: Some("
SHOUT THIS out
keep
keep"),
        solution: "veeU2jVjd",
        tip: "In visual mode, o jumps to the other end of the selection.",
    },
    Level {
        chapter: "VIII · The Summit",
        title: "Ex Marks the Spot",
        story: "From the summit you can rewrite the whole land at once. Ex commands start \
                with a colon.",
        teaches: &[
            (":s/a/b/", "substitute a with b on this line"),
            (":%s/a/b/g", "substitute everywhere, every match"),
            (":{n}", "go to line n"),
        ],
        unlock: ":",
        map: "
foo is foo
the foo bar
no foo here",
        marks: "@",
        goal: Some("
baz is baz
the baz bar
no baz here"),
        solution: ":%s/foo/baz/g<CR>",
        tip: "% means every line, and g means every match on the line.",
    },
    Level {
        chapter: "VIII · The Summit",
        title: "The Refactor",
        story: "The final trial: the old function needs a new name, wider types and a typo \
                fixed. Use everything you've learned and try to beat par.",
        teaches: &[("all of it", "motions, operators, text objects, dot, :s")],
        unlock: "",
        map: "
fn compute(x: i32) -> i32 {
    let reslt = x * 2;
    reslt + 1
}",
        marks: "@",
        goal: Some("
fn double(x: i64) -> i64 {
    let result = x * 2;
    result
}"),
        solution: "wcwdouble<Esc>:%s/32/64/g<CR>:%s/sl/sul/g<CR>f D",
        tip: "Real vim golf: the shortest known answer often uses a clever substitute.",
    },
];
