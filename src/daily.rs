//! The daily tide: one editing puzzle per day, the same for everyone. A
//! setter (this module) writes a few lines, then edits them with a random
//! but sensible program; the goal is the result and the program the par.

use crate::levels::Level;
use crate::vim::{Editor, Mode, parse_keys};

/// The lines a puzzle is made of.
const LINES: &[&str] = &[
    "the quick fox jumps over the lazy dog",
    "pack my box with five dozen jugs",
    "a wisp hops across the quiet meadow",
    "never trust a mouse near the keyboard",
    "every tide brings new words to shore",
    "the ferry leaves when the owl hoots",
    "count your steps and then skip them",
    "old maps hide the shortest roads",
    "the lighthouse blinks twice at dusk",
    "salt and ink stain the sailor's log",
    "bright crystals grow in the dark caves",
    "the bridge creaks under heavy carts",
    "small keys open the largest doors",
    "seven gulls argue over one fish",
    "write it once and repeat it often",
    "the archive smells of dust and rain",
];

/// The words the setter types.
const WORDS: &[&str] = &[
    "tide", "owl", "salt", "lamp", "reef", "kelp", "fern", "moss", "gale", "dune", "cove", "pearl",
];

/// A tiny deterministic generator (xorshift), seeded by the day.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        let mut r = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
        for _ in 0..4 {
            r.next();
        }
        r
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// Keeps `s` for the rest of the run, as the level wants.
fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// The keys that take the cursor from where it is to word `word` of line
/// `row` (both from 0), in the setter's plain style.
fn go_to(e: &Editor, row: usize, word: usize) -> String {
    let line = if row != e.cursor.row || e.cursor.col != 0 {
        format!("{}G", row + 1)
    } else {
        String::new()
    };
    let word = match word {
        0 => String::new(),
        1 => "w".into(),
        n => format!("{n}w"),
    };
    line + &word
}

/// A few different lines from `LINES`.
fn pick_lines(rng: &mut Rng) -> Vec<&'static str> {
    let count = 3 + rng.below(3);
    let mut lines: Vec<&str> = Vec::new();
    while lines.len() < count {
        let l = *rng.pick(LINES);
        if !lines.contains(&l) {
            lines.push(l);
        }
    }
    lines
}

/// One random edit of `e`: the keys that go to its place and make it, or
/// `None` if the dice picked one that doesn't fit. Whole lines go at most
/// once (`line_gone`): the puzzle is about editing them.
fn random_edit(rng: &mut Rng, e: &Editor, line_gone: &mut bool) -> Option<String> {
    let row = rng.below(e.lines.len());
    let words = e.lines[row].iter().collect::<String>().split_whitespace().count();
    if words == 0 {
        return None;
    }
    let word = rng.below(words);
    let new = *rng.pick(WORDS);
    let action = match rng.below(9) {
        0 => "dw".to_string(),
        1 => format!("cw{new}<Esc>"),
        2 => "~".to_string(),
        3 => format!("r{}", (b'a' + rng.below(26) as u8) as char),
        4 => "x".to_string(),
        5 if word > 0 => "D".to_string(),
        6 if e.lines.len() > 3 && !*line_gone => {
            *line_gone = true;
            "dd".to_string()
        }
        7 => format!("A {new}<Esc>"),
        8 if row > 0 && e.lines.len() > 2 => "ddp".to_string(),
        _ => return None,
    };
    // A whole-line action doesn't need the word.
    let whole_line = matches!(action.as_str(), "dd" | "ddp") || action.starts_with('A');
    Some(go_to(e, row, if whole_line { 0 } else { word }) + &action)
}

/// The setter edits `e` with a few random edits that each change it, and
/// returns the program it typed.
fn set(rng: &mut Rng, e: &mut Editor) -> String {
    let mut program = String::new();
    let edits = 4 + rng.below(3);
    let mut done = 0;
    let mut tries = 0;
    let mut line_gone = false;
    while done < edits && tries < 200 {
        tries += 1;
        let Some(keys) = random_edit(rng, e, &mut line_gone) else {
            continue;
        };
        let mut trial = e.clone();
        let before = trial.text();
        trial.feed_all(&parse_keys(&keys));
        if trial.mode != Mode::Normal || trial.text() == before || !trial.pending().is_empty() {
            continue;
        }
        *e = trial;
        program.push_str(&keys);
        done += 1;
    }
    program
}

/// Today's puzzle, for `day` days since 1970-01-01.
pub fn daily(day: u64) -> Level {
    let mut rng = Rng::new(day);
    let start = pick_lines(&mut rng).join("\n");
    let mut e = Editor::new(&start);
    let program = set(&mut rng, &mut e);
    let goal = e.text();
    Level {
        chapter: "Daily Tide",
        title: leak(format!("Daily · {}", day_label(day))),
        story: "Each day the tide washes up new text, and Pip's setter makes a few edits to it. \
                Match them, in as few keys as the setter used or fewer.",
        teaches: &[("anything", "every key you've learned is allowed")],
        unlock: "",
        map: leak(format!("\n{start}")),
        marks: "@",
        goal: Some(leak(format!("\n{goal}"))),
        solution: leak(program),
        tip: "The setter moves the plain way ({n}G, {n}w). Search, f, t and text objects can beat it.",
    }
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// "Nov 18" for a day number.
pub fn day_label(day: u64) -> String {
    // Howard Hinnant's civil_from_days.
    let z = day as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{} {d}", MONTHS[(m - 1) as usize])
}

/// Today's day number, from the clock.
pub fn today() -> u64 {
    (nsh_core::time::unix_ms() / 86_400_000.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Session;
    use crate::levels::LEVELS;

    #[test]
    fn day_labels() {
        assert_eq!(day_label(0), "Jan 1");
        assert_eq!(day_label(20_775), "Nov 18"); // 2026-11-18
        assert_eq!(day_label(20_820), "Jan 2"); // 2027-01-02
    }

    #[test]
    fn every_day_is_a_fair_puzzle() {
        for day in 20_700..20_760 {
            let level: &'static Level = Box::leak(Box::new(daily(day)));
            assert_eq!(
                daily(day).solution,
                level.solution,
                "the same day, the same puzzle"
            );
            assert_ne!(
                level.text(),
                level.goal_text().unwrap(),
                "day {day} changes nothing"
            );
            let mut s = Session::custom(level, LEVELS.len() - 1);
            for k in parse_keys(level.solution) {
                s.press(k);
            }
            assert!(
                s.complete && s.cleared(),
                "day {day}: {} doesn't solve\n{}\n->\n{}",
                level.solution,
                level.text(),
                level.goal_text().unwrap()
            );
            assert_eq!(s.stars(), 3);
        }
        assert_ne!(daily(1).goal, daily(2).goal);
    }
}
