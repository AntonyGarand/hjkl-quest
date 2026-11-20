//! The story that runs through the chapters: Pip, the ferry owl who rows
//! the wisp from isle to isle, the boss guarding each chapter's last isle,
//! and each chapter's look (its sky, its ground, its weather).

mod chapters;

pub use chapters::CHAPTERS;

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

/// A chapter's look.
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

/// A chapter: the story around its isles, and its look.
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

/// The chapter a `Level::chapter` string names ("III · The Tower").
pub fn chapter(label: &str) -> Option<&'static Chapter> {
    let name = label.split_once(" · ").map_or(label, |(_, n)| n);
    CHAPTERS.iter().find(|c| c.name == name)
}

/// The look of a chapter, or the meadow's for puzzles outside the campaign.
pub fn theme(label: &str) -> Theme {
    chapter(label).unwrap_or(&CHAPTERS[0]).theme
}

/// A stretch of the sea where the chapters share a topic.
pub struct Region {
    pub name: &'static str,
    /// What the region is about, in a few words.
    pub topic: &'static str,
    /// Its chapters, by name.
    pub chapters: &'static [&'static str],
    /// The tint of its waters on the map.
    pub tint: u32,
}

pub static REGIONS: &[Region] = &[
    Region {
        name: "The Shallows",
        topic: "moving around",
        chapters: &["The Meadow", "The Word Bridges", "The Tower"],
        tint: 0x3f8f7a,
    },
    Region {
        name: "The Searching Straits",
        topic: "finding things",
        chapters: &["The Machine"],
        tint: 0x3f6fa8,
    },
    Region {
        name: "The Mending Coast",
        topic: "changing text",
        chapters: &["The Workshop", "The Library", "The Observatory"],
        tint: 0x8f6a3f,
    },
    Region {
        name: "The Ex Highlands",
        topic: "the command line",
        chapters: &["The Summit", "The Scriptorium"],
        tint: 0x6a5aa8,
    },
    Region {
        name: "The Power Isles",
        topic: "registers, macros and blocks",
        chapters: &["The Vaults", "The Clockwork", "The Terraces", "The Glacier"],
        tint: 0xa85a7a,
    },
    Region {
        name: "The Mainland",
        topic: "real files",
        chapters: &["The Mainland", "The Archive"],
        tint: 0x5a8f4a,
    },
    Region {
        name: "Beyond the Map",
        topic: "challenges, after the ending",
        chapters: &["The Rhythm Reef", "The Gauntlet", "The Trials"],
        tint: 0xa8483f,
    },
];

/// The region chapter `label` lies in (its index in `REGIONS`).
pub fn region_of(label: &str) -> usize {
    let name = label.split_once(" · ").map_or(label, |(_, n)| n);
    REGIONS
        .iter()
        .position(|r| r.chapters.contains(&name))
        .unwrap_or(0)
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
