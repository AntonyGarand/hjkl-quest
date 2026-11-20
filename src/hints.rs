//! What can follow a half-typed command, as vim's which-key plugins show
//! it: after `d`, the motions and text objects; after `"`, the registers;
//! after `f`, a reminder that a character comes next. No UI here.

/// A hint card: a heading for what's pending, and the keys that may come
/// next with what each does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    pub title: String,
    pub next: Vec<(String, &'static str)>,
}

/// The operators: (keys, what they do).
const OPERATORS: &[(&str, &str)] = &[
    ("d", "delete"),
    ("c", "change (delete, then insert)"),
    ("y", "yank (copy)"),
    (">", "indent"),
    ("<", "dedent"),
    ("gu", "lower-case"),
    ("gU", "upper-case"),
    ("g~", "toggle case"),
];

/// Motions an operator can take: (keys, what, the key that must be learned).
const MOTIONS: &[(&str, &str, char)] = &[
    ("w", "to the next word", 'w'),
    ("e", "to the end of the word", 'e'),
    ("b", "back a word", 'b'),
    ("W E B", "WORDs: spaces only", 'W'),
    ("$", "to the end of the line", '$'),
    ("0", "to the start of the line", '0'),
    ("^", "to the first non-blank", '^'),
    ("j k", "this line and the next / previous", 'j'),
    ("G gg", "to the last / first line", 'G'),
    ("f{c} t{c}", "up to a character (t stops before it)", 'f'),
    ("} {", "to the paragraph's end / start", '}'),
    ("%", "to the matching bracket", '%'),
    ("/text", "up to a search match", '/'),
    ("n", "up to the next match", 'n'),
    ("H L", "to the top / bottom line of the screen", 'H'),
    ("{n}|", "to column n", '|'),
    ("_", "this line, and count-1 more", '_'),
    (") (", "to the next / previous sentence", ')'),
];

/// Text objects, after `i` (inside) or `a` (around, with the delimiters).
const OBJECTS: &[(&str, &str)] = &[
    ("w", "a word"),
    ("W", "a WORD"),
    ("s", "a sentence"),
    ("p", "a paragraph"),
    ("( b", "a ( ) block"),
    ("{ B", "a { } block"),
    ("[", "a [ ] block"),
    ("<", "a < > block"),
    ("t", "an <xml> tag"),
    ("\"", "a \"string\""),
    ("'", "a 'string'"),
];

/// The registers, after `"`.
const REGISTERS: &[(&str, &str)] = &[
    ("a-z", "a named register (A-Z appends)"),
    ("0", "the last yank"),
    ("\"", "the unnamed register"),
    ("_", "the black hole: forget it"),
];

/// What a register is for, after `"a`.
const REGISTER_USES: &[(&str, &str)] = &[
    ("yy y{motion}", "copy into it"),
    ("dd d{motion}", "cut into it"),
    ("p P", "paste from it"),
];

/// The commands after `g`: (keys, what, the key that must be learned).
const G_COMMANDS: &[(&str, &str, char)] = &[
    ("gg", "to the first line", 'g'),
    ("ge", "back to a word's end", 'e'),
    ("gu gU g~", "lower / upper / toggle case (then a motion)", 'U'),
    ("gn", "select the next match (cgn changes it)", 'n'),
    ("g&", "repeat the last :s on every line", '&'),
    ("g_", "to the line's last non-blank", '_'),
    ("g* g#", "search the word, inside longer words too", '*'),
    ("gJ", "join lines without a space", 'J'),
    ("gi", "insert where you last stopped", 'i'),
    ("gv", "select the last selection again", 'v'),
    ("gp gP", "put, and land after it", 'p'),
];

/// Commands that take a character next, and what to type.
const PROMPTS: &[(char, &str)] = &[
    ('r', "r — type the character to put here"),
    ('m', "m — name the mark: a to z"),
    ('\'', "' — jump to a mark's line: a to z, or ' for the last jump"),
    (
        '`',
        "` — jump to a mark's exact spot: a to z, or ` for the last jump",
    ),
    ('q', "q — record a macro into a register: a to z (q again stops)"),
    (
        '@',
        "@ — play the macro in a register: a to z, or @ for the last one",
    ),
];

/// `pairs` as a hint's list of next keys.
fn listed(pairs: &[(&str, &'static str)]) -> Vec<(String, &'static str)> {
    pairs.iter().map(|(k, w)| ((*k).to_string(), *w)).collect()
}

/// The hint for `pending` keys (as `Editor::pending` shows them) in normal
/// or visual mode, given the keys the player has learned. `objects` says
/// whether text objects have been taught yet.
pub fn hint(pending: &str, visual: bool, allowed: &str, objects: bool) -> Option<Hint> {
    // A register prefix and a count don't change what comes next.
    let (register, p) = match pending.strip_prefix('"') {
        Some("") => {
            return Some(Hint {
                title: "\" — pick a register".into(),
                next: listed(REGISTERS),
            });
        }
        Some(rest) => {
            let mut chars = rest.chars();
            (chars.next(), chars.as_str())
        }
        None => (None, pending),
    };
    let p = p.trim_start_matches(|c: char| c.is_ascii_digit());
    let learned = |c: char| allowed.contains(c);
    if p.is_empty() {
        return Some(counted(register, &learned));
    }
    operator(p, &learned, objects)
        .or_else(|| object(p, visual))
        .or_else(|| argument(p, allowed))
}

/// After a register or a count alone.
fn counted(register: Option<char>, learned: &impl Fn(char) -> bool) -> Hint {
    match register {
        Some(r) => Hint {
            title: format!("\"{r} — then y, d, c or p to use register {r}"),
            next: listed(REGISTER_USES),
        },
        None => Hint {
            title: "a count — then a motion or operator: it repeats".into(),
            next: motions(learned),
        },
    }
}

/// An operator, maybe followed by a count: a motion or object comes next.
fn operator(p: &str, learned: &impl Fn(char) -> bool, objects: bool) -> Option<Hint> {
    let op_end = p.trim_end_matches(|c: char| c.is_ascii_digit());
    let (op, what) = OPERATORS.iter().find(|(op, _)| *op == op_end)?;
    let mut next = motions(learned);
    let line = if op.len() == 2 {
        format!("{op}{}", &op[1..])
    } else {
        format!("{op}{op}")
    };
    next.insert(0, (line, "the whole line"));
    if objects {
        next.push(("i{obj} a{obj}".into(), "a text object: iw, a(, i\", ap…"));
    }
    Some(Hint {
        title: format!("{op} — {what}: then where to"),
        next,
    })
}

/// An operator and i or a (or i / a in visual mode): a text object.
fn object(p: &str, visual: bool) -> Option<Hint> {
    let (op, side) = p
        .strip_suffix('i')
        .map(|o| (o, "inside"))
        .or_else(|| p.strip_suffix('a').map(|o| (o, "around")))?;
    let after_operator = OPERATORS.iter().any(|(o, _)| *o == op);
    if !(after_operator || visual && op.is_empty()) {
        return None;
    }
    Some(Hint {
        title: format!("{p} — {side}: which object?"),
        next: listed(OBJECTS),
    })
}

/// A command that takes an argument: `g` and what follows it, or a
/// character.
fn argument(p: &str, allowed: &str) -> Option<Hint> {
    if p == "g" {
        let next = G_COMMANDS
            .iter()
            .filter(|(k, _, need)| {
                *k == "gg" || allowed.contains(*need) || allowed.contains(':') && *k == "g&"
            })
            .map(|(k, w, _)| ((*k).to_string(), *w))
            .collect();
        return Some(Hint {
            title: "g — more commands".into(),
            next,
        });
    }
    let c = p.chars().last()?;
    let title = match c {
        'f' | 'F' | 't' | 'T' if p.len() <= 3 => {
            let way = match c {
                'f' => "jump onto",
                'F' => "jump back onto",
                't' => "jump to just before",
                _ => "jump back to just after",
            };
            format!("{c} — type a character to {way}, on this line (; repeats, , reverses)")
        }
        _ => PROMPTS.iter().find(|(k, _)| *k == c)?.1.to_string(),
    };
    Some(Hint {
        title,
        next: Vec::new(),
    })
}

/// The motions the player has learned.
fn motions(learned: &impl Fn(char) -> bool) -> Vec<(String, &'static str)> {
    MOTIONS
        .iter()
        .filter(|(_, _, need)| learned(*need))
        .map(|(k, w, _)| (k.to_string(), *w))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operators_list_their_motions() {
        let h = hint("d", false, "hjklwbe$", false).unwrap();
        assert!(h.title.starts_with("d — delete"));
        assert_eq!(h.next[0], ("dd".to_string(), "the whole line"));
        assert!(h.next.iter().any(|(k, _)| k == "w"));
        assert!(!h.next.iter().any(|(k, _)| k == "%"), "% isn't learned yet");
        assert!(!h.next.iter().any(|(k, _)| k.starts_with("i{")));
        let h = hint("2gU", false, "hjklw", true).unwrap();
        assert_eq!(h.next[0].0, "gUU");
        assert!(h.next.iter().any(|(k, _)| k.starts_with("i{")));
        // A register and counts in front.
        assert!(
            hint("\"a3d2", false, "w", false)
                .unwrap()
                .title
                .starts_with("d —")
        );
    }

    #[test]
    fn objects_registers_and_arguments() {
        assert!(hint("ci", false, "", true).unwrap().title.contains("inside"));
        assert!(hint("a", true, "", true).unwrap().title.contains("around"));
        assert!(hint("\"", false, "", false).unwrap().title.contains("register"));
        assert!(
            hint("\"a", false, "", false)
                .unwrap()
                .title
                .contains("register a")
        );
        assert!(hint("f", false, "", false).unwrap().title.contains("character"));
        assert!(hint("3", false, "hjkl", false).unwrap().title.contains("count"));
        assert!(
            hint("g", false, "", false)
                .unwrap()
                .next
                .iter()
                .any(|(k, _)| k == "gg")
        );
        assert_eq!(hint("z", false, "", false), None);
    }
}
