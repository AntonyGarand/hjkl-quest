//! Review drills: the isles' intro examples, turned into tiny isles to
//! play again later, when what they teach may be fading. A drill's par is
//! the example's own keys.

use crate::examples;
use crate::levels::Level;
use crate::vim::{Editor, Mode, Outcome, Pos, parse_keys, pos};

/// Keeps `s` for the rest of the run, as the level wants.
fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// Keys that reach outside the drill: the shelf's files, the shell, the
/// file info and the register list.
const OUTSIDE: &[&str] = &[":r", ":e", ":w", "!", "<C-g>", ":reg"];

/// The marks overlay of `text`: `@` at `start`, and a crystal at `gem`.
fn overlay(text: &str, start: Pos, gem: Option<Pos>) -> String {
    let mut marks = String::new();
    for (r, line) in text.split('\n').enumerate() {
        if r > 0 {
            marks.push('\n');
        }
        for c in 0..line.chars().count().max(1) {
            let p = pos(r, c);
            marks.push(if p == start {
                '@'
            } else if gem == Some(p) {
                '*'
            } else {
                '.'
            });
        }
    }
    marks
}

/// Example `which` of isle `title` as a drill, unless it can't stand on its
/// own (it needs the shelf's files, the shell, or ends where it began).
pub fn drill(title: &str, which: usize) -> Option<Level> {
    let all = examples::examples(title);
    if all.is_empty() {
        return None;
    }
    let (marked, keys) = all[which % all.len()];
    if OUTSIDE.iter().any(|k| keys.contains(k)) {
        return None;
    }
    let (text, start) = examples::unmark(marked);
    let mut e = Editor::new(&text).with_cursor(start);
    for k in parse_keys(keys) {
        if let Outcome::Bell(_) = e.feed(k) {
            return None;
        }
    }
    if e.mode != Mode::Normal || !e.pending().is_empty() {
        return None;
    }
    let changed = e.text() != text;
    if !changed && e.cursor == start {
        return None;
    }
    // The overlay: @ at the start, and for a motion a crystal where it ends.
    let marks = overlay(&text, start, (!changed).then_some(e.cursor));
    Some(Level {
        chapter: "Review",
        title: leak(format!("Review · {title}")),
        story: "A drill from an isle you cleared a while ago. Do it from memory: the par is \
                the example's own keys.",
        teaches: crate::levels::LEVELS
            .iter()
            .find(|l| l.title == title)
            .map_or(&[], |l| l.teaches),
        unlock: "",
        map: leak(format!("\n{text}")),
        marks: leak(format!("\n{marks}")),
        goal: changed.then(|| leak(format!("\n{}", e.text()))),
        solution: keys,
        tip: "Stuck? :q, then open the isle itself from the map and press :h for its lessons.",
    })
}

/// Up to `count` drills from `stale` isle titles (most stale first): one
/// per isle, the example picked by `seed`.
pub fn pick(stale: &[&str], count: usize, seed: u64) -> Vec<Level> {
    let mut out = Vec::new();
    for (k, title) in stale.iter().enumerate() {
        if out.len() >= count {
            break;
        }
        let n = examples::examples(title).len();
        for d in 0..n {
            if let Some(level) = drill(title, seed as usize + k + d) {
                out.push(level);
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Session;
    use crate::levels::LEVELS;

    #[test]
    fn drills_are_solved_by_their_examples() {
        let mut isles = 0;
        for (i, isle) in LEVELS.iter().enumerate() {
            let mut any = false;
            for which in 0..examples::examples(isle.title).len() {
                let Some(level) = drill(isle.title, which) else {
                    continue;
                };
                let level: &'static Level = Box::leak(Box::new(level));
                let mut s = Session::custom(level, i.max(LEVELS.len() - 1));
                assert!(!s.complete, "{}: starts complete", level.title);
                for k in parse_keys(level.solution) {
                    s.press(k);
                }
                assert!(
                    s.cleared(),
                    "{} #{which}: {} doesn't clear it:\n{}",
                    level.title,
                    level.solution,
                    s.editor.text()
                );
                any = true;
            }
            isles += usize::from(any);
        }
        assert!(isles >= 30, "only {isles} isles have drills");
        assert_eq!(pick(&["Chomp", "Word Hopping"], 5, 0).len(), 2);
    }
}
