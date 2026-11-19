//! The habit coach: watches the normal-mode commands a player types and
//! points out the known slow patterns (`llll`, `0i`, `$a`, `dwi`...), naming
//! the faster command, but only once the player has learned it.

/// One habit spotted: a short label for the summary, and the tip shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Habit {
    pub label: String,
    pub tip: String,
}

#[derive(Debug, Default)]
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

fn advise(tokens: &[String], allowed: &str) -> Option<Habit> {
    let knows = |keys: &str| keys.chars().all(|c| allowed.contains(c));
    let last = tokens.last()?.as_str();
    let prev = tokens.len().checked_sub(2).map(|i| tokens[i].as_str());
    let habit = |label: String, tip: String| Some(Habit { label, tip });

    // The same step over and over.
    let n = run_of(tokens);
    let steps = || last.repeat(n);
    let count = |k: &str| knows("123456789").then(|| format!("{n}{k}"));
    match (last, n) {
        ("l" | "h", 4..) => {
            let mut ways: Vec<String> = count(last).into_iter().collect();
            if last == "l" && knows("we") {
                ways.push("w or e".into());
            }
            if last == "h" && knows("b") {
                ways.push("b".into());
            }
            if knows("fF") {
                ways.push(if last == "l" { "f{c}" } else { "F{c}" }.into());
            }
            if ways.is_empty() {
                return None;
            }
            return habit(format!("{} → {}", last.repeat(4), ways.join(" / ").replace(&format!("{n}{last}"), &format!("{{n}}{last}"))), format!("{}: one key per step. Jump instead: {}", steps(), ways.join(", ")));
        }
        ("j" | "k", 4..) => {
            let mut ways: Vec<String> = count(last).into_iter().collect();
            if knows("{}") {
                ways.push(if last == "j" { "}" } else { "{" }.into());
            }
            if knows("G") {
                ways.push("{n}G".into());
            }
            if ways.is_empty() {
                return None;
            }
            return habit(format!("{} → {}", last.repeat(4), ways.join(" / ").replace(&format!("{n}{last}"), &format!("{{n}}{last}"))), format!("{}: one line at a time. Try {}", steps(), ways.join(", ")));
        }
        ("x", 3..) => {
            let mut ways: Vec<String> = count("x").into_iter().collect();
            if knows("dwe") {
                ways.push("dw or de".into());
            }
            if ways.is_empty() {
                return None;
            }
            return habit(format!("xxx → {}", ways.join(" / ").replace(&format!("{n}x"), "{n}x")), format!("{}: deleting one by one. Try {}", steps(), ways.join(", ")));
        }
        ("dd", 3..) if knows("1") => {
            return habit("dd dd dd → 3dd".into(), format!("dd {n} times in a row: {n}dd does it at once"));
        }
        _ => {}
    }

    // A command and the one after it that a single key does.
    let pair = match (prev, last) {
        (Some("0" | "^"), "i") => Some(("I", "insert at the first non-blank")),
        (Some("$"), "a") => Some(("A", "append at the end of the line")),
        (Some("x"), "i") => Some(("s", "delete a character and insert")),
        (Some("dw"), "i") => Some(("cw", "change the word")),
        (Some("de"), "i") => Some(("ce", "change to the end of the word")),
        (Some("d$" | "D"), "a") => Some(("C", "change to the end of the line")),
        (Some("dd"), "O") => Some(("S", "change the whole line")),
        (Some("k"), "o") => Some(("O", "open a line above")),
        (Some("h"), "x") => Some(("X", "delete the character before the cursor")),
        _ => None,
    };
    if let (Some((better, what)), Some(prev)) = (pair, prev)
        && knows(better) {
            return habit(format!("{prev}{last} → {better}"), format!("{prev} then {last}: {better} does both ({what})"));
        }

    // A longer way to type a one-key command.
    let single = match last {
        "d$" => Some(("D", "delete to the end of the line")),
        "c$" => Some(("C", "change to the end of the line")),
        "dl" => Some(("x", "delete the character")),
        "dh" => Some(("X", "delete the character before")),
        "cl" => Some(("s", "substitute the character")),
        "cc" => Some(("S", "change the line")),
        _ => None,
    };
    if let Some((better, what)) = single
        && knows(better) {
            return habit(format!("{last} → {better}"), format!("{last} is {better} in one key ({what})"));
        }

    // Two moves that cancel out.
    let undoes = matches!((prev, last), (Some("l"), "h") | (Some("h"), "l") | (Some("j"), "k") | (Some("k"), "j") | (Some("w"), "b") | (Some("b"), "w"));
    if undoes {
        let prev = prev.unwrap_or_default();
        return habit(format!("{prev}{last} → plan the jump"), format!("{prev}{last}: the second move undoes the first. Aim before you jump"));
    }
    None
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
