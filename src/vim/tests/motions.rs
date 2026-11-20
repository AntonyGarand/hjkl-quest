//! Motions: moving around lines, words, sentences, paragraphs, marks and the jump list.

use super::run;
use crate::vim::*;

#[test]
fn hjkl() {
    let e = run("abc\ndef\nghi", pos(0, 0), "jjll");
    assert_eq!(e.cursor, pos(2, 2));
    let e = run("abc\ndef", pos(0, 0), "5l");
    assert_eq!(e.cursor, pos(0, 2));
}

#[test]
fn remembered_column() {
    let e = run("abcdef\nab\nabcdef", pos(0, 4), "jj");
    assert_eq!(e.cursor, pos(2, 4));
    let e = run("abcdef\nab\nabcdefgh", pos(0, 0), "$jj");
    assert_eq!(e.cursor, pos(2, 7));
}

#[test]
fn words() {
    let text = "foo.bar baz\n\nqux";
    let e = run(text, pos(0, 0), "w");
    assert_eq!(e.cursor, pos(0, 3));
    let e = run(text, pos(0, 0), "W");
    assert_eq!(e.cursor, pos(0, 8));
    let e = run(text, pos(0, 0), "WW");
    assert_eq!(e.cursor, pos(1, 0));
    let e = run(text, pos(0, 0), "WWW");
    assert_eq!(e.cursor, pos(2, 0));
    let e = run(text, pos(0, 0), "e");
    assert_eq!(e.cursor, pos(0, 2));
    let e = run(text, pos(0, 0), "ee");
    assert_eq!(e.cursor, pos(0, 3));
    let e = run(text, pos(0, 0), "E");
    assert_eq!(e.cursor, pos(0, 6));
    let e = run(text, pos(2, 0), "b");
    assert_eq!(e.cursor, pos(1, 0));
    let e = run(text, pos(2, 0), "bb");
    assert_eq!(e.cursor, pos(0, 8));
    let e = run(text, pos(0, 10), "B");
    assert_eq!(e.cursor, pos(0, 8));
    let e = run("one two", pos(0, 5), "ge");
    assert_eq!(e.cursor, pos(0, 2));
}

#[test]
fn line_motions() {
    let e = run("   abc def", pos(0, 5), "0");
    assert_eq!(e.cursor, pos(0, 0));
    let e = run("   abc def", pos(0, 0), "^");
    assert_eq!(e.cursor, pos(0, 3));
    let e = run("   abc def", pos(0, 0), "$");
    assert_eq!(e.cursor, pos(0, 9));
    let e = run("a\nb\nc\nd", pos(0, 0), "G");
    assert_eq!(e.cursor, pos(3, 0));
    let e = run("a\nb\nc\nd", pos(3, 0), "gg");
    assert_eq!(e.cursor, pos(0, 0));
    let e = run("a\nb\nc\nd", pos(0, 0), "3G");
    assert_eq!(e.cursor, pos(2, 0));
}

#[test]
fn find_and_till() {
    let t = "a,b,c,d";
    assert_eq!(run(t, pos(0, 0), "f,").cursor, pos(0, 1));
    assert_eq!(run(t, pos(0, 0), "f,;").cursor, pos(0, 3));
    assert_eq!(run(t, pos(0, 0), "2f,").cursor, pos(0, 3));
    assert_eq!(run(t, pos(0, 0), "t,").cursor, pos(0, 0));
    assert_eq!(run(t, pos(0, 0), "tc").cursor, pos(0, 3));
    assert_eq!(run(t, pos(0, 0), "t,;").cursor, pos(0, 2));
    assert_eq!(run(t, pos(0, 6), "F,").cursor, pos(0, 5));
    assert_eq!(run(t, pos(0, 0), "f,;;,").cursor, pos(0, 3));
}

#[test]
fn percent_and_paragraphs() {
    assert_eq!(run("if (a(b)) x", pos(0, 0), "%").cursor, pos(0, 8));
    assert_eq!(run("if (a(b)) x", pos(0, 8), "%").cursor, pos(0, 3));
    assert_eq!(run("a\nb\n\nc\n\nd", pos(0, 0), "}").cursor, pos(2, 0));
    assert_eq!(run("a\nb\n\nc\n\nd", pos(0, 0), "}}").cursor, pos(4, 0));
    assert_eq!(run("a\nb\n\nc\n\nd", pos(0, 0), "}}}").cursor, pos(5, 0));
    assert_eq!(run("a\nb\n\nc\n\nd", pos(5, 0), "{").cursor, pos(4, 0));
}

#[test]
fn columns_lines_and_screen() {
    assert_eq!(run("abcdef", pos(0, 0), "4|").cursor, pos(0, 3));
    assert_eq!(run("abcdef", pos(0, 4), "|").cursor, pos(0, 0));
    assert_eq!(run("a\n  b\n  c", pos(0, 0), "+").cursor, pos(1, 2));
    assert_eq!(run("  a\n  b", pos(1, 3), "-").cursor, pos(0, 2));
    assert_eq!(run("a\n  b\nc", pos(0, 0), "2_").cursor, pos(1, 2));
    assert_eq!(run("a b  ", pos(0, 0), "g_").cursor, pos(0, 2));
    assert_eq!(run("1\n2\n3\n4\n5", pos(2, 0), "H").cursor, pos(0, 0));
    assert_eq!(run("1\n2\n3\n4\n5", pos(0, 0), "L").cursor, pos(4, 0));
    assert_eq!(run("1\n2\n3\n4\n5", pos(0, 0), "M").cursor, pos(2, 0));
    assert_eq!(
        run("1\n2\n3\n4\n5\n6\n7\n8\n9\n10", pos(0, 0), "50%").cursor,
        pos(4, 0)
    );
    assert_eq!(run("a\nb\nc", pos(0, 0), "dL").text(), "");
}

#[test]
fn sentences() {
    let text = "One two. Three four! Five.";
    assert_eq!(run(text, pos(0, 0), ")").cursor, pos(0, 9));
    assert_eq!(run(text, pos(0, 0), "2)").cursor, pos(0, 21));
    assert_eq!(run(text, pos(0, 12), "(").cursor, pos(0, 9));
    assert_eq!(run(text, pos(0, 12), "dis").text(), "One two.  Five.");
    assert_eq!(run(text, pos(0, 12), "das").text(), "One two. Five.");
    assert_eq!(run(text, pos(0, 22), "das").text(), "One two. Three four!");
}

#[test]
fn marks() {
    assert_eq!(run("a\nb\nc\nd", pos(0, 0), "majjj`a").cursor, pos(0, 0));
    assert_eq!(run("ab\ncd\nef", pos(0, 1), "majj'a").cursor, pos(0, 0));
    assert_eq!(run("a\nb\nc\nd", pos(1, 0), "G``").cursor, pos(1, 0));
    assert_eq!(run("a\nb\nc\nd", pos(1, 0), "majjd'a").text(), "a");
}

#[test]
fn backward_search_and_jumps() {
    let t = "cat\ndog\ncat\nbird";
    assert_eq!(run(t, pos(3, 0), "?cat<CR>").cursor, pos(2, 0));
    // n keeps going backward, N turns around.
    assert_eq!(run(t, pos(3, 0), "?cat<CR>n").cursor, pos(0, 0));
    assert_eq!(run(t, pos(3, 0), "?cat<CR>nN").cursor, pos(2, 0));
    assert_eq!(run(t, pos(0, 0), "#").cursor, pos(2, 0));
    // Ctrl-o goes back through jumps, Ctrl-i (Tab) forward again.
    assert_eq!(run(t, pos(0, 0), "G<C-o>").cursor, pos(0, 0));
    assert_eq!(run(t, pos(0, 0), "G<C-o><Tab>").cursor, pos(3, 0));
    // As in vim, the line you leave replaces its older entry.
    assert_eq!(run(t, pos(0, 0), "/bird<CR>gg<C-o><C-o>").cursor, pos(3, 0));
    assert_eq!(run(t, pos(0, 0), "/bird<CR>gg<C-o><Tab>").cursor, pos(0, 0));
    assert_eq!(run(t, pos(0, 0), "/bird<CR>gg<C-o>").cursor, pos(3, 0));
    assert_eq!(run(t, pos(0, 0), ":3<CR><C-o>").cursor, pos(0, 0));
    let mut e = run(t, pos(1, 1), "");
    assert_eq!(
        e.feed(Key::CtrlG),
        Outcome::Info("\"isle\" 4 lines --50%-- line 2 of 4, col 2".into())
    );
}

#[test]
fn search() {
    let t = "cat dog\nbird cat\ncat";
    let e = run(t, pos(0, 0), "/cat<CR>");
    assert_eq!(e.cursor, pos(1, 5));
    let e = run(t, pos(0, 0), "/cat<CR>n");
    assert_eq!(e.cursor, pos(2, 0));
    let e = run(t, pos(0, 0), "/cat<CR>nn");
    assert_eq!(e.cursor, pos(0, 0));
    let e = run(t, pos(0, 0), "/cat<CR>N");
    assert_eq!(e.cursor, pos(0, 0));
    let e = run(t, pos(0, 0), "*");
    assert_eq!(e.cursor, pos(1, 5));
}
