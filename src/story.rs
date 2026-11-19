//! The story that runs through the chapters: Pip, the ferry owl who rows
//! the wisp from isle to isle, the boss guarding each chapter's last isle,
//! and each chapter's look (its sky, its ground, its weather).

/// What falls or floats in a chapter's sky.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weather {
    Fireflies,
    Motes,
    Embers,
    Snow,
    Rain,
    Leaves,
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// The sky, top to horizon.
    pub sky: [u32; 3],
    /// Grass (spaces) and stone (letters) tiles.
    pub grass: u32,
    pub stone: u32,
    pub weather: Weather,
    /// A sun instead of the moon.
    pub day: bool,
}

pub struct Chapter {
    /// As in `Level::chapter`, after the numeral: "The Meadow".
    pub name: &'static str,
    /// What Pip says rowing up to the chapter.
    pub pip: &'static str,
    /// The creature guarding the chapter's last isle, and its taunt.
    pub boss: &'static str,
    pub taunt: &'static str,
    pub theme: Theme,
    /// Commands like the chapter's, shown when its boss falls (keys, what
    /// they do), and the keys among them unlocked from then on.
    pub toolkit: &'static [(&'static str, &'static str)],
    pub unlock: &'static str,
}

const NIGHT: [u32; 3] = [0x0a0f24, 0x221a46, 0x40285c];

pub static CHAPTERS: &[Chapter] = &[
    Chapter {
        name: "The Meadow",
        pip: "Hoo! You're awake. I'm Pip, and this is my ferry. The arrows washed out to sea \
              long ago; out here we walk with h, j, k and l. Try it, I'll wait.",
        boss: "The Sheep King",
        taunt: "Baa! You'll never cross my field one step at a time!",
        theme: Theme { sky: [0x14244a, 0x2f4f7a, 0x6a7fa8], grass: 0x4c8f57, stone: 0x5f6f93, weather: Weather::Fireflies, day: true },
        toolkit: &[
            ("H  M  L", "to the top, middle or bottom line of the screen (3H: the third line)"),
            ("+  -", "down or up a line, straight onto its first letter"),
        ],
        unlock: "HML+-",
    },
    Chapter {
        name: "The Word Bridges",
        pip: "These bridges are built of words. Folk who count every plank grow old here; \
              folk who hop word to word get home for supper.",
        boss: "The Troll of the Last Plank",
        taunt: "Nobody crosses my bridge without knowing where the words end!",
        theme: Theme { sky: [0x1d1438, 0x4a2b5e, 0xb0606a], grass: 0x5d8a4a, stone: 0x7a6a5a, weather: Weather::Leaves, day: true },
        toolkit: &[
            ("{n}|", "to column n of the line: 1| is the very start"),
            ("_", "the first letter, like ^; 3_ is that of two lines down"),
        ],
        unlock: "|_",
    },
    Chapter {
        name: "The Tower",
        pip: "The Tower is tall, and the stairs are a lie. Wise wisps jump: to a line, to a \
              letter, to the next paragraph.",
        boss: "The Gargoyle",
        taunt: "Stone and silence. Find me if you can, little light.",
        theme: Theme { sky: NIGHT, grass: 0x4c7a6a, stone: 0x6a6f8a, weather: Weather::Motes, day: false },
        toolkit: &[
            ("ge  gE", "back to the end of the word before"),
            ("g_", "the last letter of the line, before any spaces"),
            ("(  )", "back or on a sentence"),
        ],
        unlock: "()",
    },
    Chapter {
        name: "The Machine",
        pip: "Gears and brackets, everything paired with its twin. Listen for the clicks: \
              % finds the other half, / finds anything at all.",
        boss: "The Clockwork Spider",
        taunt: "Tick, tick. I hide in the brackets. Search all you like!",
        theme: Theme { sky: [0x0d1a1f, 0x1f3a40, 0x3a5a52], grass: 0x3f7a6a, stone: 0x5a6a7a, weather: Weather::Embers, day: false },
        toolkit: &[
            ("g*  g#", "like * and #, but finds the word inside longer words too"),
            ("{n}%", "n percent through the file: 50% is the middle"),
            ("/\\d\\+", "patterns: . any letter, * repeats, \\d a digit, \\w a word letter, \\| or"),
        ],
        unlock: "",
    },
    Chapter {
        name: "The Workshop",
        pip: "Mind the sawdust! Now you don't just look at the text, you cut it. \
              An operator and a motion: that's a whole sentence in vim.",
        boss: "The Rust Golem",
        taunt: "My words are welded on. You can't delete what you can't reach!",
        theme: Theme { sky: [0x1c120c, 0x4a2a1a, 0x8a4a2a], grass: 0x6a7a3a, stone: 0x7a5f4a, weather: Weather::Embers, day: false },
        toolkit: &[
            ("s  S", "change one character, or the whole line"),
            ("dis  das", "a sentence, as a text object"),
            ("d/end⏎", "delete up to a search match: any motion works after d"),
        ],
        unlock: "",
    },
    Chapter {
        name: "The Library",
        pip: "Shh. Every book here was typed in insert mode, and every mistake undone with u. \
              Esc is the way out, always.",
        boss: "The Ink Wraith",
        taunt: "I blot. I smudge. Every copy you make, I'll copy twice!",
        theme: Theme { sky: [0x120d1f, 0x2a1f3f, 0x4a3a5a], grass: 0x5a6a4a, stone: 0x6a5a7a, weather: Weather::Motes, day: false },
        toolkit: &[
            ("gi", "insert again right where you last stopped"),
            ("gp  gP", "put, and land just after what you put"),
            ("Ctrl-r {reg}", "in insert mode: type out a register (Ctrl-r \" the last yank)"),
        ],
        unlock: "",
    },
    Chapter {
        name: "The Observatory",
        pip: "Look up. The stars come in groups: words, quotes, brackets. Grab one whole \
              with a text object, or light a stretch of sky in visual mode.",
        boss: "The Comet Eater",
        taunt: "I swallow stars inside their brackets. Come and fetch them!",
        theme: Theme { sky: [0x05061a, 0x141a40, 0x2a2060], grass: 0x3a6a6a, stone: 0x4a5a8a, weather: Weather::Motes, day: false },
        toolkit: &[
            ("gv", "select the last selection again"),
            ("o", "in visual mode: hop to the other end of the selection"),
            ("vis  vas", "select a sentence"),
            ("gJ", "join lines without adding a space"),
        ],
        unlock: "",
    },
    Chapter {
        name: "The Summit",
        pip: "Thin air up here, and long views. From the colon you command the whole isle at \
              once: one :s and every line obeys.",
        boss: "The Storm Hydra",
        taunt: "Cut one head and two grow back! Unless... you get them all at once.",
        theme: Theme { sky: [0x1a2030, 0x3a4a60, 0x7a8aa0], grass: 0x5a7a6a, stone: 0x6a7a90, weather: Weather::Rain, day: true },
        toolkit: &[
            ("&  g&", "repeat the last :s on this line, or on every line"),
            (":s/\\(\\w\\+\\) \\(\\w\\+\\)/\\2 \\1/", "groups, put back in any order with \\1 \\2"),
            (":s/a/b/ | s/c/d/", "| runs one command after another"),
            (":s/\\w\\+/\\u&/g", "\\u \\U \\l \\L change the case of what's put"),
        ],
        unlock: "&",
    },
    Chapter {
        name: "The Scriptorium",
        pip: "Monks copy scrolls here, from shelf to shelf. Read one in, write one out, or \
              step into another file and back.",
        boss: "The Scroll Moth",
        taunt: "Munch munch. Which file was I in? Which file are YOU in?",
        theme: Theme { sky: [0x1f1608, 0x4a3818, 0x8a6a38], grass: 0x7a7a4a, stone: 0x8a7a5a, weather: Weather::Motes, day: true },
        toolkit: &[
            (":r !cat FILE", "read in a command's output"),
            (":2,4w FILE", "write only some lines to a file"),
        ],
        unlock: "",
    },
    Chapter {
        name: "The Vaults",
        pip: "Twenty-six vaults, a to z. Put text in one and it waits there for you. \
              Marks work the same way, for places.",
        boss: "The Vault Mimic",
        taunt: "Is it your register or mine? Choose the wrong one and lose it all!",
        theme: Theme { sky: [0x0a0a14, 0x1a1a2a, 0x2a2a3a], grass: 0x4a5a4a, stone: 0x5a5a6a, weather: Weather::Motes, day: false },
        toolkit: &[
            ("\"0p", "register 0 keeps the last yank, even after deletes"),
            ("\"_dd", "the black hole: delete without touching your registers"),
            ("\"Ayw", "a capital register adds to the end of a"),
            ("``", "back to where you jumped from"),
        ],
        unlock: "",
    },
    Chapter {
        name: "The Clockwork",
        pip: "Wind it once, it runs forever. Record a macro, and let the golem do the \
              boring bits.",
        boss: "The Wind-up King",
        taunt: "I repeat myself. I repeat myself. Can YOU repeat yourself?",
        theme: Theme { sky: [0x14100a, 0x3a2a14, 0x6a4a1a], grass: 0x6a6a3a, stone: 0x8a7040, weather: Weather::Embers, day: false },
        toolkit: &[
            ("@@", "play the last macro again"),
            ("5@a", "play it five times"),
            ("qA", "record more onto the end of macro a"),
        ],
        unlock: "",
    },
    Chapter {
        name: "The Terraces",
        pip: "Rice terraces, step by step. Indent them, shout them, overwrite them. \
              Careful on the edges.",
        boss: "The Terrace Dragon",
        taunt: "MY SCALES ARE ALL CAPITALS AND YOU CANNOT LOWER THEM!",
        theme: Theme { sky: [0x10202a, 0x2a5a5a, 0x8ab08a], grass: 0x5a9a4a, stone: 0x6a8a6a, weather: Weather::Leaves, day: true },
        toolkit: &[
            ("gUU  guu  g~~", "change the case of the whole line"),
            (">ip  <ip", "shift a whole paragraph"),
            ("3>>", "shift three lines at once"),
        ],
        unlock: "",
    },
    Chapter {
        name: "The Glacier",
        pip: "Brr! Ice comes in columns here. Ctrl-v cuts a block straight down, and \
              the next match is always one gn away.",
        boss: "The Frost Wyrm",
        taunt: "Every column frozen. Every line the same. Edit them one by one, wisp!",
        theme: Theme { sky: [0x0a1a2a, 0x2a4a6a, 0x8ab0d0], grass: 0xa0b8c8, stone: 0x7a90b0, weather: Weather::Snow, day: true },
        toolkit: &[
            ("Ctrl-v $ A", "append at the end of every line, however long"),
            ("o  O", "in a block: hop to the other corners"),
            ("dgn  .", "delete the next match, then . for the one after"),
        ],
        unlock: "",
    },
    Chapter {
        name: "The Mainland",
        pip: "Land ho! Real files, real code. Out here nobody counts your keys but you. \
              Everything you learned on the isles works the same.",
        boss: "The Legacy Code",
        taunt: "I have been here since before you were born. Nobody dares refactor me.",
        theme: Theme { sky: [0x101828, 0x283850, 0x506078], grass: 0x4c7a57, stone: 0x5a6a80, weather: Weather::Rain, day: false },
        toolkit: &[
            ("* :s//new/", "an empty pattern is the last search"),
            (":%s/\\<x\\>/size/g", "\\< and \\> keep it to whole words"),
        ],
        unlock: "",
    },
    Chapter {
        name: "The Archive",
        pip: "The last door. Global commands, normal mode on every line, macros upon macros. \
              Whatever's behind it, you're ready. I'll keep the ferry warm.",
        boss: "The Archivist",
        taunt: "Every line, every file, in order. Or not at all.",
        theme: Theme { sky: [0x0a0614, 0x2a1a3a, 0x5a3a5a], grass: 0x5a6a5a, stone: 0x6a5a7a, weather: Weather::Motes, day: false },
        toolkit: &[
            (":g/pat/norm A;", "run normal-mode keys on every matching line"),
            (":v/pat/d", "delete every line that doesn't match"),
            (":g/^$/d", "patterns again: ^$ is an empty line"),
        ],
        unlock: "",
    },
];

/// The chapter a `Level::chapter` string names ("III · The Tower").
pub fn chapter(label: &str) -> Option<&'static Chapter> {
    let name = label.split_once(" · ").map_or(label, |(_, n)| n);
    CHAPTERS.iter().find(|c| c.name == name)
}

/// The look of a chapter, or the meadow's for puzzles outside the campaign.
pub fn theme(label: &str) -> Theme {
    chapter(label).unwrap_or(&CHAPTERS[0]).theme
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels::LEVELS;

    #[test]
    fn every_chapter_has_a_story() {
        for level in LEVELS {
            assert!(chapter(level.chapter).is_some(), "no story for {}", level.chapter);
        }
    }
}
