//! The Ex Highlands: the command line (chapters VIII and IX).

use super::Level;

pub(super) const LEVELS: &[Level] = &[
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
        goal: Some(
            "
baz is baz
the baz bar
no baz here",
        ),
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
            (
                ":set hls  :set is",
                "highlight every match; show matches while typing",
            ),
            (":noh", "turn the highlight off until the next search"),
        ],
        unlock: "",
        map: "
Gold, GOLD and gold
gold is GoLd",
        marks: "@",
        goal: Some(
            "
lead, lead and lead
lead is lead",
        ),
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
            (
                "a  q  l",
                "replace all the rest, quit, or replace this one and stop",
            ),
        ],
        unlock: "",
        map: "
cat food for the cat
the catalog of cats
my cat sat",
        marks: "@",
        goal: Some(
            "
dog food for the dog
the catalog of dogs
my dog sat",
        ),
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
        goal: Some(
            "
fn double(x: i64) -> i64 {
    let result = x * 2;
    result
}",
        ),
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
            (
                ":!ls  :!cat FILE",
                "run a shell command (here: ls, cat, echo, pwd, date)",
            ),
            (
                "Tab  Ctrl-d",
                "on the : line, complete a command or file name; list the choices",
            ),
            (":help", "vim's manual (here, :h shows these lessons again)"),
        ],
        unlock: "",
        map: "
# pancakes",
        marks: "@",
        goal: Some(
            "
# pancakes
flour
eggs
milk",
        ),
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
            (
                "v  :w FILE",
                "select, then : gives :'<,'>w FILE: write just the selection",
            ),
            (":r !ls", "insert a shell command's output"),
        ],
        unlock: "",
        map: "
# inbox
buy milk
call mom
spam spam",
        marks: "@",
        goal: Some(
            "
# inbox
spam spam",
        ),
        solution: "jVj:w keep.txt<CR>dk",
        tip: "The goal also wants keep.txt to hold the two errands. :!cat keep.txt shows what's in it.",
    },
    Level {
        chapter: "IX · The Scriptorium",
        title: "Two Scrolls",
        story: "Each list needs a line from the other. Open the recipe, trade lines, write it, \
                and come back: :e opens another file, and :e # returns to the one before.",
        teaches: &[
            (
                ":e FILE",
                "edit another file (your text stays open, as with 'hidden')",
            ),
            (":e #", "back to the file you were in before"),
            (":w", "write the file you're editing"),
        ],
        unlock: "",
        map: "
# shopping
eggs",
        marks: "@",
        goal: Some(
            "
# shopping
eggs
sugar",
        ),
        solution: "jyy:e r<Tab><CR>jpkyy:w<CR>:e #<CR>p",
        tip: "The isle's own text is isle.txt: end there, with recipe.txt written.",
    },
];
