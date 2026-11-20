//! Visual modes, text objects and `gn`.

use super::run;
use crate::vim::*;

#[test]
fn visual() {
    assert_eq!(run("hello world", pos(0, 0), "vlld").text(), "lo world");
    assert_eq!(run("hello world", pos(0, 0), "vey$p").text(), "hello worldhello");
    assert_eq!(run("a\nb\nc", pos(0, 0), "Vjd").text(), "c");
    assert_eq!(run("hello world", pos(0, 0), "veU").text(), "HELLO world");
    assert_eq!(run("(a b)", pos(0, 1), "vi(d").text(), "()");
}

#[test]
fn visual_block() {
    let t = "abc\ndef\nghi";
    assert_eq!(run(t, pos(0, 0), "<C-v>jjd").text(), "bc\nef\nhi");
    assert_eq!(run(t, pos(0, 0), "<C-v>jjI# <Esc>").text(), "# abc\n# def\n# ghi");
    assert_eq!(
        run("a\nbbb\ncc", pos(0, 0), "<C-v>jj$A;<Esc>").text(),
        "a;\nbbb;\ncc;"
    );
    assert_eq!(run(t, pos(0, 1), "<C-v>jlcX<Esc>").text(), "aX\ndX\nghi");
    assert_eq!(run(t, pos(0, 0), "<C-v>jlrx").text(), "xxc\nxxf\nghi");
    assert_eq!(run(t, pos(0, 0), "<C-v>jU").text(), "Abc\nDef\nghi");
    assert_eq!(
        run("ab\nc\nde", pos(0, 1), "<C-v>jjAX<Esc>").text(),
        "abX\nc X\ndeX"
    );
}

#[test]
fn search_next_match_operator() {
    let e = run("foo bar foo baz foo", pos(0, 0), "/foo<CR>cgnqux<Esc>..");
    assert_eq!(e.text(), "qux bar qux baz qux");
    assert_eq!(run("a foo b", pos(0, 0), "*").text(), "a foo b");
    assert_eq!(
        run("x foo y foo", pos(0, 2), "*Ncgnbar<Esc>").text(),
        "x bar y foo"
    );
    assert_eq!(run("a foo b", pos(0, 0), "/foo<CR>0dgn").text(), "a  b");
}

#[test]
fn paragraph_and_tag_objects() {
    let t = "a\nb\n\nc\nd";
    assert_eq!(run(t, pos(0, 0), "dip").text(), "\nc\nd");
    assert_eq!(run(t, pos(0, 0), "dap").text(), "c\nd");
    assert_eq!(run(t, pos(4, 0), "dap").text(), "a\nb");
    assert_eq!(run(t, pos(3, 0), "yipggP").text(), "c\nd\na\nb\n\nc\nd");
    let h = "<p><b>bold</b> text</p>";
    assert_eq!(run(h, pos(0, 7), "dit").text(), "<p><b></b> text</p>");
    assert_eq!(run(h, pos(0, 16), "cithi<Esc>").text(), "<p>hi</p>");
    assert_eq!(run(h, pos(0, 7), "dat").text(), "<p> text</p>");
    assert_eq!(run("<a>\n  x\n</a>", pos(1, 2), "dit").text(), "<a></a>");
}
