//! Vim's grammar: an operator (a verb) and a motion or text object (a
//! noun). Every pair the player has used lights a cell of a grid, to show
//! that a handful of keys combine into hundreds of commands. No UI here.

/// The grid's rows: operators, as typed.
pub const VERBS: &[(&str, &str)] = &[
    ("d", "delete"),
    ("c", "change"),
    ("y", "yank"),
    (">", "indent"),
    ("<", "dedent"),
    ("gu", "lower"),
    ("gU", "upper"),
    ("g~", "toggle"),
];

/// The grid's columns: what an operator acts on.
pub const NOUNS: &[(&str, &str)] = &[
    ("line", "dd cc yy: the line"),
    ("w", "word"),
    ("e", "word end"),
    ("b", "back a word"),
    ("W", "WORD"),
    ("E", "WORD end"),
    ("B", "back a WORD"),
    ("$", "line end"),
    ("0", "line start"),
    ("^", "first non-blank"),
    ("j", "line below"),
    ("k", "line above"),
    ("G", "to the end"),
    ("gg", "to the top"),
    ("f", "f{c}: onto c"),
    ("t", "t{c}: before c"),
    ("F", "F{c}: back onto c"),
    ("T", "T{c}: back to c"),
    ("}", "paragraph end"),
    ("{", "paragraph start"),
    ("%", "matching bracket"),
    ("/", "/search"),
    ("n", "next match"),
    ("iw", "inside word"),
    ("aw", "a word"),
    ("i(", "inside ( )"),
    ("a(", "around ( )"),
    ("i{", "inside { }"),
    ("i[", "inside [ ]"),
    ("i\"", "inside quotes"),
    ("a\"", "around quotes"),
    ("it", "inside a tag"),
    ("at", "around a tag"),
    ("ip", "inside a paragraph"),
    ("ap", "a paragraph"),
];

/// The (verb, noun) a completed command is, if it's an operator and a
/// motion: `3dw`, `"ayy`, `ci"`, `d/end<CR>` (as `Session::used` keeps it).
pub fn pair(token: &str) -> Option<(&'static str, &'static str)> {
    let mut t = token;
    if let Some(rest) = t.strip_prefix('"') {
        t = rest.get(rest.chars().next()?.len_utf8()..)?;
    }
    let t = t.trim_start_matches(|c: char| c.is_ascii_digit());
    let (verb, rest) = VERBS
        .iter()
        .filter_map(|(v, _)| Some((*v, t.strip_prefix(v)?)))
        .max_by_key(|(v, _)| v.len())?;
    let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    let last = &verb[verb.len() - 1..];
    if rest == last || (verb.len() == 2 && rest == verb) {
        return Some((verb, "line"));
    }
    let noun = match rest {
        r if r.starts_with('/') || r.starts_with('?') => "/",
        r if r.chars().count() == 2 && matches!(r.chars().next(), Some('f' | 't' | 'F' | 'T')) => &r[..1],
        r if r.len() >= 2 && (r.starts_with('i') || r.starts_with('a')) => {
            return find_noun(&text_object(r)?).map(|n| (verb, n));
        }
        "N" => "n",
        r => r,
    };
    find_noun(noun).map(|n| (verb, n))
}

/// The grid's name for text object `r` (`ib` is `i(`, `a'` is `a"`).
fn text_object(r: &str) -> Option<String> {
    let side = &r[..1];
    let obj = match r[1..].chars().next()? {
        '(' | ')' | 'b' => "(",
        '{' | '}' | 'B' => "{",
        '[' | ']' => "[",
        '"' | '\'' | '`' => "\"",
        'w' => "w",
        't' => "t",
        'p' => "p",
        _ => return None,
    };
    Some(format!("{side}{obj}"))
}

/// The column named `name`, if the grid has one.
fn find_noun(name: &str) -> Option<&'static str> {
    NOUNS.iter().map(|(n, _)| *n).find(|n| *n == name)
}

/// The key a learned pair is saved under.
pub fn key(verb: &str, noun: &str) -> String {
    format!("{verb} {noun}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_registers_and_doubled_operators_are_skipped() {
        assert_eq!(pair("dw"), Some(("d", "w")));
        assert_eq!(pair("3dw"), Some(("d", "w")));
        assert_eq!(pair("d3w"), Some(("d", "w")));
        assert_eq!(pair("dd"), Some(("d", "line")));
        assert_eq!(pair("2yy"), Some(("y", "line")));
        assert_eq!(pair("gUU"), Some(("gU", "line")));
        assert_eq!(pair("gUgU"), Some(("gU", "line")));
        assert_eq!(pair("gUiw"), Some(("gU", "iw")));
        assert_eq!(pair("\"ayy"), Some(("y", "line")));
    }

    #[test]
    fn objects_and_motions_are_nouns() {
        assert_eq!(pair("ci\""), Some(("c", "i\"")));
        assert_eq!(pair("da'"), Some(("d", "a\"")));
        assert_eq!(pair("dib"), Some(("d", "i(")));
        assert_eq!(pair("ct,"), Some(("c", "t")));
        assert_eq!(pair("dap"), Some(("d", "ap")));
        assert_eq!(pair("d/end"), Some(("d", "/")));
        assert_eq!(pair("dgg"), Some(("d", "gg")));
        assert_eq!(pair(">}"), Some((">", "}")));
        assert_eq!(pair("w"), None);
        assert_eq!(pair("x"), None);
        assert_eq!(pair(":s/a/b/"), None);
    }
}
