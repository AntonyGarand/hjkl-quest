//! The campaign: forty-seven isles of the Buffer Archipelago, each teaching a few
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
    /// An overlay of `map`: `@` the start, `*` a crystal to collect, `^` a
    /// trap the cursor must jump over, never land on, `>` and `<` typo bugs.
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

    pub fn traps(&self) -> Vec<Pos> {
        self.marked('^')
    }

    /// Typo bugs: `>` crawling right, `<` left.
    pub fn bugs(&self) -> Vec<crate::game::Bug> {
        let right = self.marked('>').into_iter().map(|at| crate::game::Bug { at, dir: 1 });
        right.chain(self.marked('<').into_iter().map(|at| crate::game::Bug { at, dir: -1 })).collect()
    }

    /// The scrolls on the isle's shelf, for `:r`, `:!ls` and Tab.
    pub fn files(&self) -> &'static [(&'static str, &'static str)] {
        match self.title {
            "Scrolls" => &[("ingredients.txt", "flour\neggs\nmilk"), ("notes.txt", "Pancakes: mix, rest, fry."), ("weather.txt", "sunny")],
            "Copy Desk" => &[("old.txt", "nothing new")],
            "Two Scrolls" => &[("recipe.txt", "flour\nsugar"), ("notes.txt", "bake at 180")],
            _ => &[],
        }
    }

    /// The most keystrokes the isle allows, if it limits them.
    pub fn budget(&self) -> Option<usize> {
        match self.title {
            "Edge of the World" => Some(10),
            _ => None,
        }
    }

    /// Files the goal also wants written, with their text.
    pub fn want_files(&self) -> &'static [(&'static str, &'static str)] {
        match self.title {
            "Copy Desk" => &[("keep.txt", "buy milk\ncall mom")],
            "Two Scrolls" => &[("recipe.txt", "flour\nsugar\neggs")],
            _ => &[],
        }
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
/// The isle that teaches ex commands (`:`), counted from 1.
pub fn ex_isle() -> usize {
    LEVELS.iter().position(|l| l.unlock.contains(':')).map_or(0, |i| i + 1)
}

pub fn unlocked(index: usize) -> String {
    LEVELS[..=index.min(LEVELS.len() - 1)].iter().map(|l| l.unlock).collect()
}

pub static LEVELS: &[Level] = &[
    Level {
        chapter: "I · The Meadow",
        title: "First Steps",
        story: "You wake on a quiet meadow as a small cursor-wisp. Your arrows are gone. \
                Your fingers rest on the home row, where they belong.",
        teaches: &[
            ("h", "step left"),
            ("j", "step down"),
            ("k", "step up"),
            ("l", "step right"),
            (":q!  :wq", "in vim, quit without saving, or save and quit; here both go back to the map"),
            (":h  :help", "vim's manual; here, this isle's lessons again"),
        ],
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
        story: "The meadow stretches far, and spiked blocks stand in the grass. Stepping \
                one tile at a time runs into them. Say how many steps you want, and leap over.",
        teaches: &[
            ("{n}l", "a count repeats a motion: 5l moves five right"),
            ("3j", "three lines down"),
            ("traps", "spiked blocks: you can never land on one, but a count jumps clean over"),
        ],
        unlock: "123456789",
        map: "
....................
....................
....................
....................
....................
....................",
        marks: "
@...^...^...^..*....
....................
...............^....
...............^....
....................
..*...^...^....*....",
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
          *  ^      @ ^ ^ * ^       ^ *",
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
        story: "The cliffs are ragged and indented, and crumbling: this isle allows only \
                10 keystrokes. Jump straight to the edges of a line.",
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
    @   ^   ^     *
  *     ^    ^
*        ^     ^      *",
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
^

@

^

*

^

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
@     ^     ^    * *   ^    *   *   *  ^  *",
        goal: None,
        solution: "f,;tdf!Fb;",
        tip: "f and t are the fastest way to move inside a line. ; repeats them for free.",
    },
    Level {
        chapter: "III · The Tower",
        title: "Bug Hunt",
        story: "Typo bugs! They crawl one tile per command, and every few commands they hatch twins. Don't chase them: work out where a bug will be, and land on it.",
        teaches: &[
            ("{n}f{c}", "the nth c on the line: 3fb"),
            ("predict", "each command moves every bug one tile"),
        ],
        unlock: "",
        map: "
the bugs nibble at every letter here
watch them crawl and time your jumps",
        marks: "
@...........>
..............................<",
        goal: None,
        solution: "3fbjfu",
        tip: "A bug moves after your command lands. Land where it is, not where it was.",
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
^
*

^

*
^

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
        chapter: "IV · The Machine",
        title: "Looking Back",
        story: "The machine keeps a log of every leap you make. Search backward for what's \
                behind you, then retrace your jumps without counting lines.",
        teaches: &[
            ("?text⏎", "search backward (n keeps going backward, N turns around)"),
            ("Ctrl-o", "jump back to where you were before the last jump"),
            ("Ctrl-i  Tab", "jump forward again"),
            ("Ctrl-g", "show where you are: line, total lines, column"),
        ],
        unlock: "?\u{f}\t\u{7}",
        map: "
a gold coin
sand and stone
sand and stone
you stand here and the ruby glints at the far end
sand and stone
the silver spoon
sand
sand",
        marks: "
  *


@                                               *

    *",
        goal: None,
        solution: "?go<CR><C-o>$/si<CR>",
        tip: "Searches, G, gg, % and marks are jumps: Ctrl-o walks back through them, one by one.",
    },
    Level {
        chapter: "IV · The Machine",
        title: "Infestation",
        story: "The machine's code is crawling with bugs, one per line, and each is heading for the word bug. A search lands on every match in turn: be there when they are.",
        teaches: &[
            ("/bug⏎ n", "jump from match to match, wherever they are"),
            ("hatching", "every 8 commands each bug hatches a twin: be quick"),
        ],
        unlock: "",
        map: "
fix the bug in the parser
then the bug in the lexer
and the bug in the printer",
        marks: "
@.......>
..........<
......>",
        goal: None,
        solution: "/bug<CR>nn",
        tip: "Each command moves every bug one tile. Count ahead: where will it be when you land?",
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
            ("u  Ctrl-r", "undo a mistake, redo it (always available from here on)"),
            ("U", "undo every change on the last changed line at once"),
            ("~", "toggle the case of the character, and step right"),
        ],
        unlock: "xXrU~",
        map: "
thee cat zat onn thee mat",
        marks: "@",
        goal: Some("
The cat sat on the mat"),
        solution: "~lxfzrsfnxfex",
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
            ("2dd  d3w", "counts work with operators too"),
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
                repeats your last change, as many times as you like.",
        teaches: &[
            (".", "repeat the last change"),
            ("u  Ctrl-r", "undo and redo, which you've had since the Workshop"),
        ],
        unlock: ".",
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
        title: "Case Closed",
        story: "Gold is written every which way up here. Searches can ignore case, and light \
                up every match as you type.",
        teaches: &[
            (":set ic  :set noic", "searches (and :s) ignore case, or not"),
            ("\\c", "in a pattern: ignore case for this search only"),
            (":set hls  :set is", "highlight every match; show matches while typing"),
            (":noh", "turn the highlight off until the next search"),
        ],
        unlock: "",
        map: "
Gold, GOLD and gold
gold is GoLd",
        marks: "@",
        goal: Some("
lead, lead and lead
lead is lead"),
        solution: ":%s/gold\\c/lead/g<CR>",
        tip: "With :set ic smartcase, a capital in the pattern makes it exact again.",
    },
    Level {
        chapter: "VIII · The Summit",
        title: "Ask First",
        story: "Not every cat is a cat. Add c to a substitute and it asks before each change.",
        teaches: &[
            (":%s/a/b/gc", "substitute, confirming each match"),
            ("y  n", "replace this one, or skip it"),
            ("a  q  l", "replace all the rest, quit, or replace this one and stop"),
        ],
        unlock: "",
        map: "
cat food for the cat
the catalog of cats
my cat sat",
        marks: "@",
        goal: Some("
dog food for the dog
the catalog of dogs
my dog sat"),
        solution: ":%s/cat/dog/gc<CR>yynyy",
        tip: "The match being asked about is highlighted. Esc stops, keeping what you changed.",
    },
    Level {
        chapter: "VIII · The Summit",
        title: "The Refactor",
        story: "The summit's last trial: the old function needs a new name, wider types and a typo \
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
    // ------------------------------------------------- beyond the summit
    Level {
        chapter: "IX · The Scriptorium",
        title: "Scrolls",
        story: "The scribes keep their scrolls as files. Read one straight into your text, \
                peek at the shelf with the shell, and let Tab finish the names for you.",
        teaches: &[
            (":r FILE", "insert a file's text below the cursor"),
            (":!ls  :!cat FILE", "run a shell command (here: ls, cat, echo, pwd, date)"),
            ("Tab  Ctrl-d", "on the : line, complete a command or file name; list the choices"),
            (":help", "vim's manual (here, :h shows these lessons again)"),
        ],
        unlock: "",
        map: "
# pancakes",
        marks: "@",
        goal: Some("
# pancakes
flour
eggs
milk"),
        solution: ":r i<Tab><CR>",
        tip: "Tab completes command names too: :so<Tab> gives :sort. In vim, :help and your ~/.vimrc are next.",
    },
    Level {
        chapter: "IX · The Scriptorium",
        title: "Copy Desk",
        story: "Copy the errands to their own scroll, then clear them from the inbox. :w takes \
                a file name, and a range, to write just part of the text.",
        teaches: &[
            (":w FILE", "write the text to a file"),
            ("v  :w FILE", "select, then : gives :'<,'>w FILE: write just the selection"),
            (":r !ls", "insert a shell command's output"),
        ],
        unlock: "",
        map: "
# inbox
buy milk
call mom
spam spam",
        marks: "@",
        goal: Some("
# inbox
spam spam"),
        solution: "jVj:w keep.txt<CR>dk",
        tip: "The goal also wants keep.txt to hold the two errands. :!cat keep.txt shows what's in it.",
    },
    Level {
        chapter: "IX · The Scriptorium",
        title: "Two Scrolls",
        story: "Each list needs a line from the other. Open the recipe, trade lines, write it, \
                and come back: :e opens another file, and :e # returns to the one before.",
        teaches: &[
            (":e FILE", "edit another file (your text stays open, as with 'hidden')"),
            (":e #", "back to the file you were in before"),
            (":w", "write the file you're editing"),
        ],
        unlock: "",
        map: "
# shopping
eggs",
        marks: "@",
        goal: Some("
# shopping
eggs
sugar"),
        solution: "jyy:e r<Tab><CR>jpkyy:w<CR>:e #<CR>p",
        tip: "The isle's own text is isle.txt: end there, with recipe.txt written.",
    },
    Level {
        chapter: "X · The Vaults",
        title: "Named Vaults",
        story: "Below the summit lie the vaults: twenty-six named registers, a hand for \
                every letter. Carry two things at once without one pushing out the other.",
        teaches: &[
            ("\"a  \"b", "name a register before a yank, delete or put: \"add, \"ap"),
            ("\"0", "the last yank, untouched by later deletes"),
            ("\"_", "the black hole: delete without losing what you yanked"),
            ("\"A", "an upper-case name appends to the register"),
            (":reg", "show what every register holds"),
        ],
        unlock: "\"",
        map: "
lamp
crate
vault A:
vault B:",
        marks: "@",
        goal: Some("
vault A:
lamp
vault B:
crate"),
        solution: "\"add\"bdd\"apj\"bp",
        tip: "Registers shine when you carry several things at once. Here plain dd and p can beat par: can you find how?",
    },
    Level {
        chapter: "X · The Vaults",
        title: "Waymarks",
        story: "The vault keepers leave waymarks to find their way back. Drop a mark, wander off, \
                and return to it, or cut everything in between.",
        teaches: &[
            ("ma", "set mark a here (any letter a-z)"),
            ("`a", "jump to mark a, exactly"),
            ("'a", "jump to the line of mark a"),
            ("``", "jump back to where you were before the last jump"),
            ("d`a", "marks are motions: delete up to the mark"),
        ],
        unlock: "m`'",
        map: "
keep this line
cut from HERE and keep
these lines all go
away completely
until THERE the rest
the end",
        marks: "@",
        goal: Some("
keep this line
cut from the rest
the end"),
        solution: "jfHma/the r<CR>d`a",
        tip: "Search, G and % are jumps: `` brings you back. A visual selection can also do this one, a little shorter.",
    },
    Level {
        chapter: "XI · The Clockwork",
        title: "Tally Marks",
        story: "The clockwork city counts everything. Why retype a number when you can wind it up \
                or down?",
        teaches: &[
            ("Ctrl-a", "add 1 to the number under or after the cursor"),
            ("Ctrl-x", "subtract 1"),
            ("{n}Ctrl-a", "add n: 27 Ctrl-a"),
        ],
        unlock: "\u{1}\u{18}",
        map: "
apples: 3
pears: 10
debt: -4
year 1999",
        marks: "@",
        goal: Some("
apples: 5
pears: 1
debt: 0
year 2026"),
        solution: "2<C-a>j9<C-x>j4<C-a>j27<C-a>",
        tip: "Ctrl-a finds the next number on the line by itself: no need to move onto it first.",
    },
    Level {
        chapter: "XI · The Clockwork",
        title: "Wind-up Golem",
        story: "A golem repeats whatever you show it. Record your keys once, then let it do the \
                rest of the list.",
        teaches: &[
            ("qa ... q", "record keys into register a, until q again"),
            ("@a", "play the macro in register a"),
            ("@@  5@a", "play the last macro again; a count plays it n times"),
        ],
        unlock: "q@",
        map: "
alice
bob
carol
dave
erin
frank",
        marks: "@",
        goal: Some("
\"Alice\",
\"Bob\",
\"Carol\",
\"Dave\",
\"Erin\",
\"Frank\","),
        solution: "qaI\"<Esc>l~A\",<Esc>jq5@a",
        tip: "End a macro on the next line (j) so it can run again. If a motion fails, the macro stops, so 99@a is safe.",
    },
    Level {
        chapter: "XI · The Clockwork",
        title: "Assembly Line",
        story: "The recipe needs numbered steps. A macro that copies a line and winds its number \
                builds the whole list.",
        teaches: &[("qayyp Ctrl-a q", "a macro that copies and counts"), ("4@a", "four more")],
        unlock: "",
        map: "
step 1: stir",
        marks: "@",
        goal: Some("
step 1: stir
step 2: stir
step 3: stir
step 4: stir
step 5: stir
step 6: stir"),
        solution: "qayyp<C-a>q4@a",
        tip: "Recording also runs the keys, so the first copy is made while you record.",
    },
    Level {
        chapter: "XII · The Terraces",
        title: "Terraces",
        story: "On the terraced hills every line sits at its own depth. Shift whole lines in and \
                out (shiftwidth is 4 here).",
        teaches: &[
            (">>  <<", "indent, or dedent, the line"),
            (">{motion}", "indent the lines a motion covers: >j, >G, >ip"),
            ("3>>", "three lines at once"),
        ],
        unlock: "<>",
        map: "
def tidy(xs):
for x in xs:
if x:
keep(x)
return xs",
        marks: "@",
        goal: Some("
def tidy(xs):
    for x in xs:
        if x:
            keep(x)
    return xs"),
        solution: "j>Gj>jj>>",
        tip: "Indent the outer block first, then the inner ones: each shift adds to the last.",
    },
    Level {
        chapter: "XII · The Terraces",
        title: "Shouting Match",
        story: "The terrace criers argue in capitals. Case operators work like d and c: with any \
                motion or text object.",
        teaches: &[
            ("gU{motion}", "upper-case: gUiw, gU$"),
            ("gu{motion}", "lower-case"),
            ("g~{motion}", "toggle case"),
            ("gUU  guu", "the whole line"),
        ],
        unlock: "u",
        map: "
const max_size = 10
WARNING: Do Not Touch
hello",
        marks: "@",
        goal: Some("
const MAX_SIZE = 10
warning: do not touch
HELLO"),
        solution: "wgUiwjguujgUU",
        tip: "iw takes the underscore with it: max_size is one word.",
    },
    Level {
        chapter: "XII · The Terraces",
        title: "Overwrite",
        story: "The hero board has fixed columns. Replace mode types over the old text instead of \
                pushing it right, so the columns stay lined up.",
        teaches: &[("R", "replace mode: every key overwrites a character"), ("<BS>", "in replace mode, brings the old character back")],
        unlock: "R",
        map: "
| name     | hp |
| ???????? | 00 |",
        marks: "@",
        goal: Some("
| name     | hp |
| wisp     | 42 |"),
        solution: "jf?Rwisp    <Esc>f0R42<Esc>",
        tip: "R is ideal for tables and fixed-width data, where c would shift everything after it.",
    },
    Level {
        chapter: "XIII · The Glacier",
        title: "Ice Columns",
        story: "The glacier is cut in straight columns. Visual block mode selects a rectangle, \
                and what you type on its first line appears on all of them.",
        teaches: &[
            ("Ctrl-v", "visual block mode: select a column"),
            ("I  A", "in a block: insert before it, or append after it, on every line"),
            ("$A", "append at the end of every line, however long"),
            ("c  d", "change, or delete, the block"),
        ],
        unlock: "\u{16}",
        map: "
# red = 1
# green = 2
# blue = 3",
        marks: "@",
        goal: Some("
let red = 1;
let green = 2;
let blue = 3;"),
        solution: "<C-v>jjlclet <Esc><C-v>jj$A;<Esc>",
        tip: "The other lines are filled in when you press <Esc>, not while you type.",
    },
    Level {
        chapter: "XIII · The Glacier",
        title: "Next Match",
        story: "A variable needs a new name, but not where its name is part of another word. \
                gn selects the next match of your search, so cgn changes it, and . does the next.",
        teaches: &[
            ("*", "search the word under the cursor (whole words only)"),
            ("gn", "select the next match of the last search"),
            ("cgn", "change the next match; . repeats on the following one"),
        ],
        unlock: "",
        map: "
let tmp = load();
print(tmp);
save(tmp, tmpdir);",
        marks: "@",
        goal: Some("
let data = load();
print(data);
save(data, tmpdir);"),
        solution: "w*cgndata<Esc>..",
        tip: "cgn with . is like a find-and-replace you confirm one match at a time, skipping any with n.",
    },
    Level {
        chapter: "XIII · The Glacier",
        title: "Tag, You're It",
        story: "Ice-bound markup and stray paragraphs. Text objects know tags and paragraphs as \
                well as words and brackets.",
        teaches: &[
            ("it  at", "inside, or around, an HTML tag: cit, dat"),
            ("ip  ap", "a paragraph, or one with the blank lines after it"),
        ],
        unlock: "",
        map: "
<ul>
  <li>old</li>
  <li><b>x</b></li>
</ul>

junk line
more junk",
        marks: "@",
        goal: Some("
<ul>
  <li>new</li>
  <li></li>
</ul>"),
        solution: "jwcitnew<Esc>jdatGdap",
        tip: "it picks the innermost tag around the cursor; dap at the last paragraph takes the blank lines before it.",
    },
    Level {
        chapter: "XIV · The Mainland",
        title: "What's in a Name",
        story: "Land! A real Rust function, and a variable called x that nobody understands. \
                Rename it to size, everywhere it's used and nowhere else.",
        teaches: &[
            ("*", "search for the word under the cursor, whole words only"),
            ("ciw  n  .", "change one, find the next, repeat the change"),
            (":%s/\\<x\\>/size/g", "or all at once, \\< and \\> keeping it to whole words"),
        ],
        unlock: "",
        map: "
fn area(w: f64, h: f64) -> f64 {
    let x = w * h;
    println!(\"{}\", x);
    x
}",
        marks: "
.
........@",
        goal: Some("
fn area(w: f64, h: f64) -> f64 {
    let size = w * h;
    println!(\"{}\", size);
    size
}"),
        solution: "*ciwsize<Esc>n.n.",
        tip: "* jumps to the next x first; it wraps around to the one you started on.",
    },
    Level {
        chapter: "XIV · The Mainland",
        title: "Broken Config",
        story: "The server won't start: its JSON is broken. Single quotes, a missing comma, a \
                trailing one, a wrong port, and debug left on.",
        teaches: &[
            ("f'  ;", "hop between the quotes"),
            ("r\"", "swap a character"),
            ("A,<Esc>", "add the missing comma"),
            ("ciw", "change a whole value"),
        ],
        unlock: "",
        map: "
{
  'name': \"hjkl\",
  \"port\": 8000
  \"debug\": true,
}",
        marks: "
@",
        goal: Some("
{
  \"name\": \"hjkl\",
  \"port\": 8080,
  \"debug\": false
}"),
        solution: "jf'r\";r\"j$hr8A,<Esc>jciwfalse<Esc>lx",
        tip: "JSON wants double quotes, a comma between entries, and none after the last.",
    },
    Level {
        chapter: "XIV · The Mainland",
        title: "Release Notes",
        story: "The changelog is out of order: the newest release should come first. Move the \
                1.0 notes to the bottom.",
        teaches: &[
            ("2dd", "cut two lines"),
            ("G p", "to the end, and put them below"),
            (":2,3m$", "or move lines 2 and 3 to the end"),
        ],
        unlock: "",
        map: "
# Changelog
## 1.0
- first release
## 1.2
- faster maps
## 1.1
- fixed bugs",
        marks: "
@",
        goal: Some("
# Changelog
## 1.2
- faster maps
## 1.1
- fixed bugs
## 1.0
- first release"),
        solution: "j2ddGp",
        tip: "Linewise text (from dd or yy) is always put on whole lines, below the cursor's.",
    },
    Level {
        chapter: "XIV · The Mainland",
        title: "Import Duty",
        story: "The imports are a tangle: out of order, and one is there twice. Sort them, and \
                keep only one of each.",
        teaches: &[
            (":1,5sort", "sort lines 1 to 5"),
            (":sort u", "u keeps only the first of identical lines"),
            ("vip:sort u", "or select the paragraph first"),
        ],
        unlock: "",
        map: "
use std::io;
use std::fs;
use crate::game;
use std::fs;
use crate::audio;

fn main() {}",
        marks: "
@",
        goal: Some("
use crate::audio;
use crate::game;
use std::fs;
use std::io;

fn main() {}"),
        solution: ":1,5sor u<CR>",
        tip: ":sort sorts the whole file without a range: here that would move fn main() too.",
    },
    Level {
        chapter: "XV · The Archive",
        title: "Global Decree",
        story: "The archive's ledgers are a mess. :g runs a command on every line that matches \
                a pattern; :v on every line that doesn't.",
        teaches: &[
            (":g/pat/d", "delete every line containing pat"),
            (":v/pat/d", "delete every line that doesn't"),
            (":sort", "sort the lines"),
        ],
        unlock: "",
        map: "
4 date
# fruit list
2 banana
5 elder
# sorted soon
1 apple
3 cherry
# end",
        marks: "@",
        goal: Some("
1 apple
2 banana
3 cherry
4 date
5 elder"),
        solution: ":g/#/d<CR>:sort<CR>",
        tip: "Patterns may start with ^ or end with $ to anchor them: :g/^#/d.",
    },
    Level {
        chapter: "XV · The Archive",
        title: "Moving Day",
        story: "Whole sections need to move. Ex commands take a range of lines and move or copy \
                them in one go.",
        teaches: &[
            (":4,6m0", "move lines 4 to 6 to the top (after line 0)"),
            (":$t0", "copy the last line to the top"),
            (":'<,'>", "the lines of the visual selection (press : in visual mode)"),
            (". $ +1 'a", "addresses: this line, the last, offsets, marks"),
        ],
        unlock: "",
        map: "
## Usage
run it
run it fast
## Install
get it
build it
## License
MIT",
        marks: "@",
        goal: Some("
MIT
## Install
get it
build it
## Usage
run it
run it fast
## License
MIT"),
        solution: ":4,6m0<CR>:$t0<CR>",
        tip: "The line numbers on the left are what ranges count with. :m moves, :t copies (:co too).",
    },
    Level {
        chapter: "XV · The Archive",
        title: "Normal Order",
        story: "The scribes want every function name shouted, and only those. :normal types \
                normal-mode keys on each line that :g finds.",
        teaches: &[
            (":norm keys", "run normal-mode keys on each line of the range"),
            (":g/pat/norm keys", "on each matching line"),
            (":%norm A;", "append to every line"),
        ],
        unlock: "",
        map: "
fn load() {}
// a
// b
fn parse() {}
// c
fn check() {}
// d
// e
// f
fn print() {}
// g
fn save() {}
// h
fn quit() {}",
        marks: "@",
        goal: Some("
fn LOAD() {}
// a
// b
fn PARSE() {}
// c
fn CHECK() {}
// d
// e
// f
fn PRINT() {}
// g
fn SAVE() {}
// h
fn QUIT() {}"),
        solution: ":g/fn/norm wgUiw<CR>",
        tip: ":normal starts each line at its first column and ends insert mode for you.",
    },
    Level {
        chapter: "XV · The Archive",
        title: "The Grand Refactor",
        story: "The archive's last door. Turn an old config into constants: prune it with :g, \
                then teach a macro the rest. Everything you know is fair game.",
        teaches: &[("all of it", "registers, macros, blocks, :g, :norm, text objects")],
        unlock: "",
        map: "
// config
name=hjkl
size=10
// old
debug=true
trace=true
// end
mode=fast",
        marks: "@",
        goal: Some("
const NAME = \"hjkl\";
const SIZE = \"10\";
const MODE = \"fast\";"),
        solution: ":g#//#d<CR>:g/=t/d<CR>ggqaIconst <Esc>wgUiwf=s = \"<Esc>A\";<Esc>jq2@a",
        tip: "Any punctuation can delimit a pattern: :g#//#d saves escaping the slashes.",
    },
];
