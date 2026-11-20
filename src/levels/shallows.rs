//! The Shallows: moving around (chapters I to III).

use super::Level;

pub(super) const LEVELS: &[Level] = &[
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
            (
                ":q!  :wq",
                "in vim, quit without saving, or save and quit; here both go back to the map",
            ),
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
            (
                "traps",
                "spiked blocks: you can never land on one, but a count jumps clean over",
            ),
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
        teaches: &[
            ("}", "forward to the next blank line"),
            ("{", "back to the previous blank line"),
        ],
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
];
