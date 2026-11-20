//! The Mainland: real files (chapters XIV and XV).

use super::Level;

pub(super) const LEVELS: &[Level] = &[
    Level {
        chapter: "XIV · The Mainland",
        title: "What's in a Name",
        story: "Land! A real Rust function, and a variable called x that nobody understands. \
                Rename it to size, everywhere it's used and nowhere else.",
        teaches: &[
            ("*", "search for the word under the cursor, whole words only"),
            ("ciw  n  .", "change one, find the next, repeat the change"),
            (
                ":%s/\\<x\\>/size/g",
                "or all at once, \\< and \\> keeping it to whole words",
            ),
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
        goal: Some(
            "
fn area(w: f64, h: f64) -> f64 {
    let size = w * h;
    println!(\"{}\", size);
    size
}",
        ),
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
        goal: Some(
            "
{
  \"name\": \"hjkl\",
  \"port\": 8080,
  \"debug\": false
}",
        ),
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
        goal: Some(
            "
# Changelog
## 1.2
- faster maps
## 1.1
- fixed bugs
## 1.0
- first release",
        ),
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
        goal: Some(
            "
use crate::audio;
use crate::game;
use std::fs;
use std::io;

fn main() {}",
        ),
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
        goal: Some(
            "
1 apple
2 banana
3 cherry
4 date
5 elder",
        ),
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
            (
                ":'<,'>",
                "the lines of the visual selection (press : in visual mode)",
            ),
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
        goal: Some(
            "
MIT
## Install
get it
build it
## Usage
run it
run it fast
## License
MIT",
        ),
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
        goal: Some(
            "
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
fn QUIT() {}",
        ),
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
        goal: Some(
            "
const NAME = \"hjkl\";
const SIZE = \"10\";
const MODE = \"fast\";",
        ),
        solution: ":g#//#d<CR>:g/=t/d<CR>ggqaIconst <Esc>wgUiwf=s = \"<Esc>A\";<Esc>jq2@a",
        tip: "Any punctuation can delimit a pattern: :g#//#d saves escaping the slashes.",
    },
];
