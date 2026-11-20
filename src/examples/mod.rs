//! Little worked examples for each isle's intro: a snippet of text, where
//! the cursor starts (marked `▮`, before its character) and the keys to
//! press. They run on the real engine, so they show exactly what happens.

mod isles;

use isles::EXAMPLES;

use crate::vim::{Editor, Key, Outcome, Pos, parse_keys, pos};

/// The cursor's place in an example's text: just before this mark.
pub const CURSOR: char = '▮';

/// One step of an example being played: the editor after `typed` keys.
pub struct Frame {
    pub editor: Editor,
    pub typed: usize,
    /// What the editor said after this key (Ctrl-g, `:!ls`, a prompt).
    pub message: Option<String>,
}

/// The files every example can see, for `:r`, `:w` and the shell.
const SHELF: &[(&str, &str)] = &[("notes.txt", "hello\nworld"), ("todo.txt", "buy milk")];

/// An example's text without its cursor mark, and where the mark was.
pub(crate) fn unmark(text: &str) -> (String, Pos) {
    let mut at = pos(0, 0);
    let mut plain = String::new();
    let (mut row, mut col) = (0, 0);
    for c in text.chars() {
        if c == CURSOR {
            at = pos(row, col);
            continue;
        }
        plain.push(c);
        if c == '\n' {
            row += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (plain, at)
}

/// The editor before any key, and after each one.
pub fn frames(text: &str, keys: &str) -> Vec<Frame> {
    let (plain, at) = unmark(text);
    let mut editor = Editor::new(&plain).with_cursor(at);
    editor.files = SHELF
        .iter()
        .map(|(n, t)| (n.to_string(), t.to_string()))
        .collect();
    let keys: Vec<Key> = parse_keys(keys);
    let mut out = vec![Frame {
        editor: editor.clone(),
        typed: 0,
        message: None,
    }];
    let mut message = None;
    for (i, &k) in keys.iter().enumerate() {
        match editor.feed(k) {
            Outcome::Info(m) => message = Some(m),
            Outcome::Bell(m) if !m.is_empty() => message = Some(m),
            _ if editor.mode == crate::vim::Mode::Normal && k != Key::Enter => message = None,
            _ => {}
        }
        out.push(Frame {
            editor: editor.clone(),
            typed: i + 1,
            message: message.clone(),
        });
    }
    out
}

/// The examples shown before isle `title`: (text with `▮`, keys).
pub fn examples(title: &str) -> &'static [(&'static str, &'static str)] {
    EXAMPLES
        .iter()
        .find(|(t, _)| *t == title)
        .map_or(&[], |(_, list)| list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels::LEVELS;
    use crate::vim::Outcome;

    #[test]
    fn every_isle_has_examples_that_run_cleanly() {
        for level in LEVELS {
            let list = examples(level.title);
            assert!(!list.is_empty(), "{} has no examples", level.title);
            for &(text, keys) in list {
                assert_eq!(
                    text.matches(CURSOR).count(),
                    1,
                    "{}: {text:?} needs one cursor mark",
                    level.title
                );
                let plain: String = text.chars().filter(|c| *c != CURSOR).collect();
                let at = frames(text, "")[0].editor.cursor;
                let mut e = Editor::new(&plain).with_cursor(at);
                e.files = SHELF
                    .iter()
                    .map(|(n, t)| (n.to_string(), t.to_string()))
                    .collect();
                for k in parse_keys(keys) {
                    let out = e.feed(k);
                    assert!(
                        !matches!(out, Outcome::Bell(ref m) if !m.is_empty()),
                        "{}: {keys} on {text:?}: {out:?}",
                        level.title
                    );
                }
                assert_ne!(frames(text, keys).last().map(|f| f.editor.text()), None);
            }
        }
    }

    #[test]
    fn frames_follow_the_keys() {
        let f = frames("▮the quick", "w");
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].editor.cursor, pos(0, 0));
        assert_eq!(f[1].editor.cursor, pos(0, 4));
        let f = frames("ab\n▮cd", "x");
        assert_eq!(f[1].editor.text(), "ab\nd");
    }
}
