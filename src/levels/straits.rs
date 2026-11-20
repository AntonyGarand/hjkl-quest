//! The Searching Straits: finding things (chapter IV).

use super::Level;

pub(super) const LEVELS: &[Level] = &[
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
            (
                "?text⏎",
                "search backward (n keeps going backward, N turns around)",
            ),
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
];
