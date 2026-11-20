//! The Mending Coast: changing text (chapters V to VII).

use super::Level;

pub(super) const LEVELS: &[Level] = &[
    Level {
        chapter: "V · The Workshop",
        title: "Chomp",
        story: "Typos have crept into the old sign. Remove the extra letters and fix \
                the wrong one.",
        teaches: &[
            ("x", "delete the character under the cursor"),
            ("X", "delete the character before the cursor"),
            ("r{c}", "replace the character under the cursor with c"),
            (
                "u  Ctrl-r",
                "undo a mistake, redo it (always available from here on)",
            ),
            ("U", "undo every change on the last changed line at once"),
            ("~", "toggle the case of the character, and step right"),
        ],
        unlock: "xXrU~",
        map: "
thee cat zat onn thee mat",
        marks: "@",
        goal: Some(
            "
The cat sat on the mat",
        ),
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
        goal: Some(
            "
keep this line
keep this too
cut here:
very good",
        ),
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
        goal: Some(
            "
I love vim and I squash bugs
status: fixed",
        ),
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
        goal: Some(
            "
my list:
- apple
- ripe pear!
- fig",
        ),
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
        goal: Some(
            "
hip hip hooray
==
==
==",
        ),
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
        goal: Some(
            "
x = 1;
y = 2;
z = 3;
good",
        ),
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
        goal: Some(
            "
print(\"bye\")
items = []
call()",
        ),
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
        goal: Some(
            "
SHOUT THIS out
keep
keep",
        ),
        solution: "veeU2jVjd",
        tip: "In visual mode, o jumps to the other end of the selection.",
    },
];
