//! Beyond the Map: challenges, after the ending (chapters XVI to XVIII).

use super::Level;

pub(super) const LEVELS: &[Level] = &[
    Level {
        chapter: "XVI · The Rhythm Reef",
        title: "Fading Lights",
        story: "Past the Archive the reef glows, but its lights fade: each crystal counts your \
                commands, not your seconds. Cross the isle in one jump, not twenty steps.",
        teaches: &[
            ("$  2j  -", "one command can cross the whole isle"),
            ("a fading crystal", "its number is the commands it has left"),
        ],
        unlock: "",
        map: "
the tide pulls every light out to sea
grab them before the water takes them
jump with counts and never take a step",
        marks: "
@...................................2
3
.....................................3",
        goal: None,
        solution: "$2j-",
        tip: "After $, j and k stay at the end of every line: the column sticks.",
    },
    Level {
        chapter: "XVI · The Rhythm Reef",
        title: "Night Watch",
        story: "Lantern-bearing sentries march the reef, one row for every command you finish. \
                Land where one stands, or where one is going, and it marches you back to the start.",
        teaches: &[
            (
                "count the beats",
                "every finished command is one step for every sentry",
            ),
            ("wait a beat", "a short move buys time when the way is guarded"),
        ],
        unlock: "",
        map: "
march of the lanterns
up the yard and back
slip past the guards
when their backs turn",
        marks: "
@.....V......*
...................*
..
......*......A",
        goal: None,
        solution: "3j6l7lk2kj$",
        tip: "A sentry turns at the edge of the isle. Count its steps ahead before you jump.",
    },
    Level {
        chapter: "XVI · The Rhythm Reef",
        title: "Tide Clock",
        story: "The reef's clock strikes, the bugs come out, and the last light starts to fade. \
                Three commands, if you see it; a dark reef, if you don't.",
        teaches: &[
            ("$", "land on a bug before it crawls away"),
            ("{n}|", "to column n, wherever the cursor is"),
        ],
        unlock: "",
        map: "
the clock strikes and the bugs come out
catch them all before the lights go dim",
        marks: "
@.....................................<
>.....................................2",
        goal: None,
        solution: "$j3|",
        tip: "A bug crawls after your command lands: aim where it will be, or where it is before it moves.",
    },
    Level {
        chapter: "XVII · The Gauntlet",
        title: "Name Swap",
        story: "The Gauntlet: no new keys, just hard isles where all of vim is fair game. First, a \
                list of names, surname first. Turn them around, every line at once.",
        teaches: &[
            ("\\(…\\)  \\1", "groups in a pattern, put back in any order"),
            ("\\v", "very magic: ( ) + ? | without the backslashes"),
            ("qa … q", "or a macro, if patterns aren't your thing"),
        ],
        unlock: "",
        map: "
Lovelace, Ada
Hopper, Grace
Torvalds, Linus
Moolenaar, Bram",
        marks: "@",
        goal: Some(
            "
Ada Lovelace
Grace Hopper
Linus Torvalds
Bram Moolenaar",
        ),
        solution: ":%s/\\v(.*), (.*)/\\2 \\1<CR>",
        tip: ".* is greedy, but the comma pins it: everything before \", \", then everything after.",
    },
    Level {
        chapter: "XVII · The Gauntlet",
        title: "Count Off",
        story: "A shopping list wants numbers. Number the first line yourself, then let a macro \
                copy the number down and add one, line after line.",
        teaches: &[
            ("qa … q  3@a", "record one line's work, play it for the rest"),
            ("Ctrl-a", "add one to the number at or after the cursor"),
        ],
        unlock: "",
        map: "
1. apples
pears
plums
figs
dates",
        marks: "@",
        goal: Some(
            "
1. apples
2. pears
3. plums
4. figs
5. dates",
        ),
        solution: "qa0y3lj0P0<C-a>q3@a",
        tip: "y3l yanks \"1. \"; after P puts it in front, Ctrl-a turns the 1 into a 2.",
    },
    Level {
        chapter: "XVII · The Gauntlet",
        title: "Tag Soup",
        story: "Someone bolded and italicised a shopping list. Strip the <b> and <i> tags, keep \
                the words, and indent the items under their list.",
        teaches: &[
            ("[bi]  \\=", "one of b or i; an optional /"),
            (":2,4>", "shift a range of lines"),
            (
                "vatp",
                "or with a tag object: select the tag, put the word over it",
            ),
        ],
        unlock: "",
        map: "
<ul>
<li><b>bread</b></li>
<li><i>milk</i></li>
<li>eggs</li>
</ul>",
        marks: "@",
        goal: Some(
            "
<ul>
    <li>bread</li>
    <li>milk</li>
    <li>eggs</li>
</ul>",
        ),
        solution: ":%s/<\\/\\=[bi]>//g<CR>:2,4><CR>",
        tip: "\\= makes the / optional, so one pattern takes both <b> and </b>.",
    },
    Level {
        chapter: "XVIII · The Trials",
        title: "No Arrows",
        story: "The Trials: each isle takes something away. Here, h j k and l are banned. \
                Reach the crystals by words, finds and leaps.",
        teaches: &[
            ("f{c}  +  $  b", "everything but the home row"),
            ("banned keys", "pressing one does nothing: find another way"),
        ],
        unlock: "",
        map: "
no arrows and no home row either
only words and jumps will carry you
find the far crystals by name",
        marks: "
@................*
..........................*
.........................*",
        goal: None,
        solution: "fh+fc+$b",
        tip: "+ is the next line's first letter; - the one above.",
    },
    Level {
        chapter: "XVIII · The Trials",
        title: "Thin Ice",
        story: "Fix the typos, but the icy lines are frozen: they keep their teh, and any edit \
                that touches them simply doesn't happen.",
        teaches: &[
            (":v/ice/s/teh/the", "on every line that doesn't match ice"),
            (":2s  :4s", "or a line at a time"),
        ],
        unlock: "",
        map: "
teh ice holds: this line is frozen
fix teh typo on this line
teh ice holds: this line is frozen
and teh one on this line
teh ice holds: this line is frozen",
        marks: "
@#
.
#
.
#",
        goal: Some(
            "
teh ice holds: this line is frozen
fix the typo on this line
teh ice holds: this line is frozen
and the one on this line
teh ice holds: this line is frozen",
        ),
        solution: ":v/ice/s/teh/the<CR>",
        tip: ":%s would change the frozen lines too, so the ice stops it whole.",
    },
    Level {
        chapter: "XVIII · The Trials",
        title: "No Delete",
        story: "Words stutter on this isle, and x, X, d and D are banned. Delete without deleting: \
                change, substitute, or a pattern that finds a word said twice.",
        teaches: &[
            ("\\v(\\w+) \\1", "a word, a space, and the same word again"),
            ("cw  s", "or change what you'd have deleted"),
        ],
        unlock: "",
        map: "
the the cat sat on on the mat
a a dog dug up up a bone",
        marks: "@",
        goal: Some(
            "
the cat sat on the mat
a dog dug up a bone",
        ),
        solution: ":%s/\\v<(\\w+) \\1>/\\1/g<CR>",
        tip: "\\1 in the pattern is whatever the first group matched: a doubled word.",
    },
    Level {
        chapter: "XVIII · The Trials",
        title: "Every Key Counts",
        story: "The last trial: a budget of five keys for three crystals. Spend one too many and \
                the isle sinks. There's a trick to it.",
        teaches: &[
            ("$  j", "after $, j keeps to the end of every line"),
            ("a key budget", "the isle ends when the keys run out"),
        ],
        unlock: "",
        map: "
every key you press is one you spend
spend them well and the crystals are yours
waste them and the island sinks",
        marks: "
@..................................*
.........................................*
..............................*",
        goal: None,
        solution: "$jj",
        tip: "$ sets the column to 'the end', so j and k land on each line's last letter.",
    },
];
