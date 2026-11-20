//! The habit coach: watches the normal-mode commands a player types and
//! points out the known slow patterns (`llll`, `0i`, `$a`, `dwi`...), naming
//! the faster command, but only once the player has learned it.

/// One habit spotted: a short label for the summary, and the tip shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Habit {
    pub label: String,
    pub tip: String,
}

/// Watches one run's commands for slow habits.
#[derive(Debug, Default, Clone)]
pub struct Coach {
    /// Completed normal-mode commands, as typed (`l`, `4j`, `dw`, `i`).
    tokens: Vec<String>,
    /// Each habit seen this run, once.
    pub spotted: Vec<Habit>,
}

impl Coach {
    /// Takes a completed command; returns a tip when it ends a slow pattern.
    pub fn push(&mut self, token: String, allowed: &str) -> Option<String> {
        self.tokens.push(token);
        let habit = advise(&self.tokens, allowed)?;
        let tip = habit.tip.clone();
        if !self.spotted.iter().any(|h| h.label == habit.label) {
            self.spotted.push(habit);
        }
        Some(tip)
    }
}

/// How many times the last token repeats at the end of `tokens`.
fn run_of(tokens: &[String]) -> usize {
    let Some(last) = tokens.last() else { return 0 };
    tokens.iter().rev().take_while(|t| *t == last).count()
}

/// Faster ways to repeat a step: (the step, the keys needed, the way).
const JUMPS: &[(&str, &str, &str)] = &[
    ("l", "we", "w or e"),
    ("h", "b", "b"),
    ("l", "fF", "f{c}"),
    ("h", "fF", "F{c}"),
    ("j", "{}", "}"),
    ("k", "{}", "{"),
    ("j", "G", "{n}G"),
    ("k", "G", "{n}G"),
    ("x", "dwe", "dw or de"),
];

/// A command, then one that a single key does with it: (the firsts, the
/// second, the single key, what it does).
const PAIRS: &[(&[&str], &str, &str, &str)] = &[
    (&["0", "^"], "i", "I", "insert at the first non-blank"),
    (&["$"], "a", "A", "append at the end of the line"),
    (&["x"], "i", "s", "delete a character and insert"),
    (&["dw"], "i", "cw", "change the word"),
    (&["de"], "i", "ce", "change to the end of the word"),
    (&["d$", "D"], "a", "C", "change to the end of the line"),
    (&["dd"], "O", "S", "change the whole line"),
    (&["k"], "o", "O", "open a line above"),
    (&["h"], "x", "X", "delete the character before the cursor"),
];

/// A longer way to type a one-key command: (the long way, the key, what
/// it does).
const SINGLES: &[(&str, &str, &str)] = &[
    ("d$", "D", "delete to the end of the line"),
    ("c$", "C", "change to the end of the line"),
    ("dl", "x", "delete the character"),
    ("dh", "X", "delete the character before"),
    ("cl", "s", "substitute the character"),
    ("cc", "S", "change the line"),
];

/// Two moves that cancel out.
const UNDOES: &[(&str, &str)] = &[
    ("l", "h"),
    ("h", "l"),
    ("j", "k"),
    ("k", "j"),
    ("w", "b"),
    ("b", "w"),
];

fn advise(tokens: &[String], allowed: &str) -> Option<Habit> {
    let knows = |keys: &str| keys.chars().all(|c| allowed.contains(c));
    let last = tokens.last()?.as_str();
    // A run of steps can't also end one of the patterns below.
    if let Some(habit) = repeated(last, run_of(tokens), &knows) {
        return Some(habit);
    }
    let prev = tokens.len().checked_sub(2).map(|i| tokens[i].as_str());
    let habit = |label: String, tip: String| Some(Habit { label, tip });
    if let Some(prev) = prev
        && let Some((_, _, better, what)) = PAIRS
            .iter()
            .find(|(firsts, second, ..)| firsts.contains(&prev) && *second == last)
        && knows(better)
    {
        return habit(
            format!("{prev}{last} → {better}"),
            format!("{prev} then {last}: {better} does both ({what})"),
        );
    }
    if let Some((_, better, what)) = SINGLES.iter().find(|(long, ..)| *long == last)
        && knows(better)
    {
        return habit(
            format!("{last} → {better}"),
            format!("{last} is {better} in one key ({what})"),
        );
    }
    if let Some(prev) = prev
        && UNDOES.contains(&(prev, last))
    {
        return habit(
            format!("{prev}{last} → plan the jump"),
            format!("{prev}{last}: the second move undoes the first. Aim before you jump"),
        );
    }
    None
}

/// The same step `last`, `n` times over: the faster ways the player knows.
fn repeated(last: &str, n: usize, knows: &dyn Fn(&str) -> bool) -> Option<Habit> {
    let (min, what) = match last {
        "l" | "h" => (4, "one key per step. Jump instead:"),
        "j" | "k" => (4, "one line at a time. Try"),
        "x" => (3, "deleting one by one. Try"),
        "dd" if n >= 3 && knows("1") => {
            return Some(Habit {
                label: "dd dd dd → 3dd".into(),
                tip: format!("dd {n} times in a row: {n}dd does it at once"),
            });
        }
        _ => return None,
    };
    if n < min {
        return None;
    }
    let count = knows("123456789").then(|| format!("{n}{last}"));
    let jumps = JUMPS
        .iter()
        .filter(|(step, keys, _)| *step == last && knows(keys))
        .map(|(_, _, way)| (*way).to_string());
    let ways: Vec<String> = count.into_iter().chain(jumps).collect();
    if ways.is_empty() {
        return None;
    }
    let label = ways
        .join(" / ")
        .replace(&format!("{n}{last}"), &format!("{{n}}{last}"));
    Some(Habit {
        label: format!("{} → {label}", last.repeat(min)),
        tip: format!("{}: {what} {}", last.repeat(n), ways.join(", ")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(keys: &[&str], allowed: &str) -> (Option<String>, Coach) {
        let mut coach = Coach::default();
        let mut last = None;
        for k in keys {
            last = coach.push(k.to_string(), allowed);
        }
        (last, coach)
    }

    #[test]
    fn repeated_steps_suggest_learned_jumps() {
        let (tip, coach) = feed(&["l", "l", "l", "l"], "hjkl123456789we");
        let tip = tip.unwrap();
        assert!(tip.contains("4l") && tip.contains("w or e"), "{tip}");
        assert_eq!(coach.spotted.len(), 1);
        // Nothing to suggest before counts are learned.
        assert_eq!(feed(&["l", "l", "l", "l"], "hjkl").0, None);
        assert_eq!(feed(&["l", "l", "l"], "hjkl123456789").0, None);
    }

    #[test]
    fn pairs_that_one_key_does() {
        assert!(feed(&["0", "i"], "0iI").0.unwrap().contains("I does both"));
        assert!(feed(&["$", "a"], "$aA").0.unwrap().contains("A does both"));
        assert!(feed(&["dw", "i"], "dwic").0.unwrap().contains("cw"));
        assert_eq!(feed(&["$", "a"], "$a").0, None);
        assert!(feed(&["d$"], "dD$").0.unwrap().contains("D in one key"));
        assert!(feed(&["l", "h"], "hl").0.unwrap().contains("undoes"));
    }

    #[test]
    fn a_habit_is_listed_once() {
        let (_, coach) = feed(&["l", "l", "l", "l", "l", "l"], "hjkl123456789");
        assert_eq!(coach.spotted.len(), 1);
    }
}
