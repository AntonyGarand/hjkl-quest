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

/// The hint for `pending` keys (as `Editor::pending` shows them) in normal
/// or visual mode, given the keys the player has learned. `objects` says
/// whether text objects have been taught yet.
pub fn hint(pending: &str, visual: bool, allowed: &str, objects: bool) -> Option<Hint> {
    // A register prefix and a count don't change what comes next.
    let mut p = pending;
    let mut register = None;
    if let Some(rest) = p.strip_prefix('"') {
        let mut chars = rest.chars();
        match chars.next() {
            None => {
                return Some(Hint {
                    title: "\" — pick a register".into(),
                    next: vec![
                        ("a-z".into(), "a named register (A-Z appends)"),
                        ("0".into(), "the last yank"),
                        ("\"".into(), "the unnamed register"),
                        ("_".into(), "the black hole: forget it"),
                    ],
                });
            }
            Some(r) => {
                register = Some(r);
                p = chars.as_str();
            }
        }
    }
    let p = p.trim_start_matches(|c: char| c.is_ascii_digit());
    let learned = |c: char| allowed.contains(c);

    if p.is_empty() {
        let title = match register {
            Some(r) => format!("\"{r} — then y, d, c or p to use register {r}"),
            None => "a count — then a motion or operator: it repeats".into(),
        };
        let next = match register {
            Some(_) => vec![("yy y{motion}".into(), "copy into it"), ("dd d{motion}".into(), "cut into it"), ("p P".into(), "paste from it")],
            None => motions(&learned),
        };
        return Some(Hint { title, next });
    }
    // An operator, maybe followed by a count: a motion or object comes next.
    let op_end = p.trim_end_matches(|c: char| c.is_ascii_digit());
    if let Some((op, what)) = OPERATORS.iter().find(|(op, _)| *op == op_end) {
        let mut next = motions(&learned);
        let line = if op.len() == 2 { format!("{op}{}", &op[1..]) } else { format!("{op}{op}") };
        next.insert(0, (line, "the whole line"));
        if objects {
            next.push(("i{obj} a{obj}".into(), "a text object: iw, a(, i\", ap…"));
        }
        return Some(Hint { title: format!("{op} — {what}: then where to"), next });
    }
    // An operator and i or a (or i / a in visual mode): a text object.
    let obj = p.strip_suffix('i').map(|o| (o, "inside")).or_else(|| p.strip_suffix('a').map(|o| (o, "around")));
    if let Some((op, side)) = obj
        && (OPERATORS.iter().any(|(o, _)| *o == op) || (visual && op.is_empty()))
    {
        let next = OBJECTS.iter().map(|(k, w)| (k.to_string(), *w)).collect();
        return Some(Hint { title: format!("{p} — {side}: which object?"), next });
    }
    let simple = |title: &str| Some(Hint { title: title.into(), next: Vec::new() });
    match p.chars().last()? {
        _ if p.ends_with('g') && p.len() == 1 => Some(Hint {
            title: "g — more commands".into(),
            next: [
                ("gg", "to the first line", 'g'),
                ("ge", "back to a word's end", 'e'),
                ("gu gU g~", "lower / upper / toggle case (then a motion)", 'U'),
                ("gn", "select the next match (cgn changes it)", 'n'),
                ("g&", "repeat the last :s on every line", '&'),
            ]
            .into_iter()
            .filter(|(k, _, need)| *k == "gg" || learned(*need) || allowed.contains(':') && *k == "g&")
            .map(|(k, w, _)| (k.to_string(), w))
            .collect(),
        }),
        'f' | 'F' | 't' | 'T' if p.len() <= 3 => {
            let c = p.chars().last().unwrap();
            let way = match c {
                'f' => "jump onto",
                'F' => "jump back onto",
                't' => "jump to just before",
                _ => "jump back to just after",
            };
            simple(&format!("{c} — type a character to {way}, on this line (; repeats, , reverses)"))
        }
        'r' => simple("r — type the character to put here"),
        'm' => simple("m — name the mark: a to z"),
        '\'' => simple("' — jump to a mark's line: a to z, or ' for the last jump"),
        '`' => simple("` — jump to a mark's exact spot: a to z, or ` for the last jump"),
        'q' => simple("q — record a macro into a register: a to z (q again stops)"),
        '@' => simple("@ — play the macro in a register: a to z, or @ for the last one"),
        _ => None,
    }
}

fn motions(learned: &impl Fn(char) -> bool) -> Vec<(String, &'static str)> {
    MOTIONS.iter().filter(|(_, _, need)| learned(*need)).map(|(k, w, _)| (k.to_string(), *w)).collect()
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
        assert!(hint("\"a3d2", false, "w", false).unwrap().title.starts_with("d —"));
    }

    #[test]
    fn objects_registers_and_arguments() {
        assert!(hint("ci", false, "", true).unwrap().title.contains("inside"));
        assert!(hint("a", true, "", true).unwrap().title.contains("around"));
        assert!(hint("\"", false, "", false).unwrap().title.contains("register"));
        assert!(hint("\"a", false, "", false).unwrap().title.contains("register a"));
        assert!(hint("f", false, "", false).unwrap().title.contains("character"));
        assert!(hint("3", false, "hjkl", false).unwrap().title.contains("count"));
        assert!(hint("g", false, "", false).unwrap().next.iter().any(|(k, _)| k == "gg"));
        assert_eq!(hint("z", false, "", false), None);
    }
}
