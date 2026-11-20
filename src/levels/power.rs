//! The Power Isles: registers, macros and blocks (chapters X to XIII).

use super::Level;

pub(super) const LEVELS: &[Level] = &[
    Level {
        chapter: "X · The Vaults",
        title: "Named Vaults",
        story: "Below the summit lie the vaults: twenty-six named registers, a hand for \
                every letter. Carry two things at once without one pushing out the other.",
        teaches: &[
            (
                "\"a  \"b",
                "name a register before a yank, delete or put: \"add, \"ap",
            ),
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
        goal: Some(
            "
vault A:
lamp
vault B:
crate",
        ),
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
        goal: Some(
            "
keep this line
cut from the rest
the end",
        ),
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
        goal: Some(
            "
apples: 5
pears: 1
debt: 0
year 2026",
        ),
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
        goal: Some(
            "
\"Alice\",
\"Bob\",
\"Carol\",
\"Dave\",
\"Erin\",
\"Frank\",",
        ),
        solution: "qaI\"<Esc>l~A\",<Esc>jq5@a",
        tip: "End a macro on the next line (j) so it can run again. If a motion fails, the macro stops, so 99@a is safe.",
    },
    Level {
        chapter: "XI · The Clockwork",
        title: "Assembly Line",
        story: "The recipe needs numbered steps. A macro that copies a line and winds its number \
                builds the whole list.",
        teaches: &[
            ("qayyp Ctrl-a q", "a macro that copies and counts"),
            ("4@a", "four more"),
        ],
        unlock: "",
        map: "
step 1: stir",
        marks: "@",
        goal: Some(
            "
step 1: stir
step 2: stir
step 3: stir
step 4: stir
step 5: stir
step 6: stir",
        ),
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
        goal: Some(
            "
def tidy(xs):
    for x in xs:
        if x:
            keep(x)
    return xs",
        ),
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
        goal: Some(
            "
const MAX_SIZE = 10
warning: do not touch
HELLO",
        ),
        solution: "wgUiwjguujgUU",
        tip: "iw takes the underscore with it: max_size is one word.",
    },
    Level {
        chapter: "XII · The Terraces",
        title: "Overwrite",
        story: "The hero board has fixed columns. Replace mode types over the old text instead of \
                pushing it right, so the columns stay lined up.",
        teaches: &[
            ("R", "replace mode: every key overwrites a character"),
            ("<BS>", "in replace mode, brings the old character back"),
        ],
        unlock: "R",
        map: "
| name     | hp |
| ???????? | 00 |",
        marks: "@",
        goal: Some(
            "
| name     | hp |
| wisp     | 42 |",
        ),
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
            (
                "I  A",
                "in a block: insert before it, or append after it, on every line",
            ),
            ("$A", "append at the end of every line, however long"),
            ("c  d", "change, or delete, the block"),
        ],
        unlock: "\u{16}",
        map: "
# red = 1
# green = 2
# blue = 3",
        marks: "@",
        goal: Some(
            "
let red = 1;
let green = 2;
let blue = 3;",
        ),
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
        goal: Some(
            "
let data = load();
print(data);
save(data, tmpdir);",
        ),
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
        goal: Some(
            "
<ul>
  <li>new</li>
  <li></li>
</ul>",
        ),
        solution: "jwcitnew<Esc>jdatGdap",
        tip: "it picks the innermost tag around the cursor; dap at the last paragraph takes the blank lines before it.",
    },
];
