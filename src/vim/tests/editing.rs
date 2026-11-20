//! Normal-mode changes: deletes, changes, inserts, puts, undo, registers, macros.

use super::run;
use crate::vim::*;

#[test]
fn deletes() {
    assert_eq!(run("hello world", pos(0, 0), "x").text(), "ello world");
    assert_eq!(run("hello world", pos(0, 0), "3x").text(), "lo world");
    assert_eq!(run("hello world", pos(0, 0), "dw").text(), "world");
    assert_eq!(run("hello world", pos(0, 6), "dw").text(), "hello ");
    assert_eq!(run("one two\nthree", pos(0, 4), "dw").text(), "one \nthree");
    assert_eq!(run("hello world", pos(0, 0), "de").text(), " world");
    assert_eq!(run("hello world", pos(0, 3), "D").text(), "hel");
    assert_eq!(run("hello world", pos(0, 3), "d0").text(), "lo world");
    assert_eq!(run("a\nb\nc", pos(0, 0), "dd").text(), "b\nc");
    assert_eq!(run("a\nb\nc", pos(0, 0), "2dd").text(), "c");
    assert_eq!(run("a\nb\nc", pos(0, 0), "dj").text(), "c");
    assert_eq!(run("a\nb\nc", pos(2, 0), "dk").text(), "a");
    assert_eq!(run("a b c d", pos(0, 0), "d2w").text(), "c d");
    assert_eq!(run("a b c d", pos(0, 0), "2dw").text(), "c d");
    assert_eq!(run("a,b,c", pos(0, 0), "df,").text(), "b,c");
    assert_eq!(run("a,b,c", pos(0, 0), "dt,").text(), ",b,c");
    assert_eq!(run("abc", pos(0, 2), "X").text(), "ac");
}

#[test]
fn changes() {
    assert_eq!(run("hello world", pos(0, 0), "cwbye<Esc>").text(), "bye world");
    assert_eq!(run("hello world", pos(0, 2), "ciwbye<Esc>").text(), "bye world");
    assert_eq!(run("hello world", pos(0, 0), "Cnope<Esc>").text(), "nope");
    assert_eq!(
        run("say \"hi there\" now", pos(0, 0), "ci\"yo<Esc>").text(),
        "say \"yo\" now"
    );
    assert_eq!(run("f(a, b)", pos(0, 3), "ci(x<Esc>").text(), "f(x)");
    assert_eq!(run("f(a, b)", pos(0, 3), "da(").text(), "f");
    assert_eq!(run("a word here", pos(0, 3), "daw").text(), "a here");
    assert_eq!(run("abc", pos(0, 0), "sX<Esc>").text(), "Xbc");
    assert_eq!(run("abc\ndef", pos(0, 1), "Sxyz<Esc>").text(), "xyz\ndef");
    assert_eq!(run("abc", pos(0, 0), "rZ").text(), "Zbc");
    assert_eq!(run("abc", pos(0, 0), "3rZ").text(), "ZZZ");
}

#[test]
fn inserts() {
    assert_eq!(run("bc", pos(0, 0), "ia<Esc>").text(), "abc");
    assert_eq!(run("ac", pos(0, 0), "ab<Esc>").text(), "abc");
    assert_eq!(run("  bc", pos(0, 3), "Ia<Esc>").text(), "  abc");
    assert_eq!(run("ab", pos(0, 0), "Ac<Esc>").text(), "abc");
    assert_eq!(run("a\nc", pos(0, 0), "ob<Esc>").text(), "a\nb\nc");
    assert_eq!(run("b", pos(0, 0), "Oa<Esc>").text(), "a\nb");
    assert_eq!(run("ab", pos(0, 1), "ix<BS>y<Esc>").text(), "ayb");
    let e = run("abc", pos(0, 0), "Ad<Esc>");
    assert_eq!(e.cursor, pos(0, 3));
}

#[test]
fn yank_put() {
    assert_eq!(run("a\nb", pos(0, 0), "yyp").text(), "a\na\nb");
    assert_eq!(run("a\nb", pos(0, 0), "yyjP").text(), "a\na\nb");
    assert_eq!(run("a\nb", pos(0, 0), "ddp").text(), "b\na");
    assert_eq!(run("ab", pos(0, 0), "xp").text(), "ba");
    assert_eq!(run("one two", pos(0, 0), "yw$p").text(), "one twoone ");
    assert_eq!(run("a", pos(0, 0), "yy3p").text(), "a\na\na\na");
}

#[test]
fn undo_redo_dot() {
    assert_eq!(run("abc", pos(0, 0), "xu").text(), "abc");
    assert_eq!(run("abc", pos(0, 0), "xxuu").text(), "abc");
    assert_eq!(run("abc", pos(0, 0), "xu<C-r>").text(), "bc");
    assert_eq!(run("abcd", pos(0, 0), "x..").text(), "d");
    assert_eq!(run("a b c", pos(0, 0), "dw.").text(), "c");
    assert_eq!(run("x\ny\nz", pos(0, 0), "A;<Esc>j.j.").text(), "x;\ny;\nz;");
    assert_eq!(
        run("cat cat cat", pos(0, 0), "cwdog<Esc>w.w.").text(),
        "dog dog dog"
    );
    assert_eq!(run("abc", pos(0, 0), "x.u").text(), "bc");
}

#[test]
fn misc() {
    assert_eq!(run("a\n  b", pos(0, 0), "J").text(), "a b");
    assert_eq!(run("abc", pos(0, 0), "~~").text(), "ABc");
    assert_eq!(
        run("cat cat\ncat", pos(0, 0), ":s/cat/dog/<CR>").text(),
        "dog cat\ncat"
    );
    assert_eq!(
        run("cat cat\ncat", pos(0, 0), ":%s/cat/dog/g<CR>").text(),
        "dog dog\ndog"
    );
    assert_eq!(run("a\nb\nc", pos(0, 0), ":3<CR>").cursor, pos(2, 0));
}

#[test]
fn registers() {
    assert_eq!(run("a\nb", pos(0, 0), "\"ayyjdd\"ap").text(), "a\na");
    assert_eq!(run("one two", pos(0, 0), "\"ayw\"bywwdw\"aP").text(), "oneone  ");
    // A delete doesn't clobber "0, so the yank survives.
    assert_eq!(
        run("keep\nzap\nx", pos(0, 0), "yyjddj\"0p").text(),
        "keep\nx\nkeep"
    );
    // The black hole swallows the delete.
    assert_eq!(run("keep\nzap", pos(0, 0), "yyj\"_ddp").text(), "keep\nkeep");
    // Upper case appends.
    assert_eq!(
        run("a\nb\nc", pos(0, 0), "\"ayyj\"Ayyj\"ap").text(),
        "a\nb\nc\na\nb"
    );
}

#[test]
fn increment() {
    assert_eq!(run("x = 7", pos(0, 0), "<C-a>").text(), "x = 8");
    assert_eq!(run("x = 7", pos(0, 0), "5<C-x>").text(), "x = 2");
    assert_eq!(run("x = 7", pos(0, 0), "10<C-x>").text(), "x = -3");
    assert_eq!(run("a -5 b", pos(0, 0), "6<C-a>").text(), "a 1 b");
    assert_eq!(run("v9", pos(0, 0), "<C-a>").cursor, pos(0, 2));
    assert_eq!(run("1\n1", pos(0, 0), "<C-a>j.").text(), "2\n2");
}

#[test]
fn macros() {
    assert_eq!(run("a\nb\nc", pos(0, 0), "qaA;<Esc>jq2@a").text(), "a;\nb;\nc;");
    // A failing motion ends the macro, so a big count is safe.
    assert_eq!(run("1\n1\n1\n1", pos(0, 0), "qa<C-a>jq99@a").text(), "2\n2\n2\n2");
    assert_eq!(run("a\nb\nc", pos(0, 0), "qaI-<Esc>jq@a@@").text(), "-a\n-b\n-c");
    let e = run("x", pos(0, 0), "qaq");
    assert_eq!(e.register_text('a').as_deref(), Some(""));
}

#[test]
fn indent_and_case_operators() {
    assert_eq!(run("a\nb", pos(0, 0), ">>").text(), "    a\nb");
    assert_eq!(run("a\nb", pos(0, 0), ">j").text(), "    a\n    b");
    assert_eq!(run("      a", pos(0, 0), "<<").text(), "  a");
    assert_eq!(run("a\n\nb", pos(0, 0), "3>>").text(), "    a\n\n    b");
    assert_eq!(run("hello world", pos(0, 0), "gUiw").text(), "HELLO world");
    assert_eq!(run("Hello World", pos(0, 0), "guu").text(), "hello world");
    assert_eq!(run("Hello World", pos(0, 0), "gUgU").text(), "HELLO WORLD");
    assert_eq!(run("abc def", pos(0, 0), "g~w").text(), "ABC def");
    assert_eq!(run("a\nb\nc", pos(0, 0), "Vj>").text(), "    a\n    b\nc");
}

#[test]
fn replace_mode() {
    assert_eq!(run("abcdef", pos(0, 1), "RXY<Esc>").text(), "aXYdef");
    assert_eq!(run("ab", pos(0, 1), "RXYZ<Esc>").text(), "aXYZ");
    assert_eq!(run("abcd", pos(0, 0), "RXY<BS><BS>Z<Esc>").text(), "Zbcd");
}

#[test]
fn undo_line() {
    assert_eq!(run("abc def", pos(0, 0), "xxwx").text(), "c ef");
    assert_eq!(run("abc def", pos(0, 0), "xxwxU").text(), "abc def");
    // U is itself a change: u undoes it, and U again redoes the line.
    assert_eq!(run("abc def", pos(0, 0), "xxwxUu").text(), "c ef");
    assert_eq!(run("abc def", pos(0, 0), "xxwxUU").text(), "c ef");
    // Only the last changed line.
    assert_eq!(run("ab\ncd", pos(0, 0), "xjxU").text(), "b\ncd");
}

#[test]
fn g_commands() {
    assert_eq!(run("ab\n  cd", pos(0, 0), "gJ").text(), "ab  cd");
    let e = run("hello", pos(0, 0), "Aworld<Esc>0gi!<Esc>");
    assert_eq!(e.text(), "helloworld!");
    let e = run("one two", pos(0, 0), "vey0gvd");
    assert_eq!(e.text(), " two");
    let e = run("ab", pos(0, 0), "ylgpix<Esc>");
    assert_eq!(e.text(), "aaxb");
    let e = run("a\nb", pos(0, 0), "yygpix<Esc>");
    assert_eq!(e.text(), "a\na\nxb");
    let e = run("foobar foo", pos(0, 7), "g*");
    assert_eq!(e.cursor, pos(0, 0));
}

#[test]
fn insert_ctrl_r_puts_a_register() {
    let e = run("cat", pos(0, 0), "yiwA <C-r>\"<Esc>");
    assert_eq!(e.text(), "cat cat");
    let e = run("x", pos(0, 0), "\"ayiwo<C-r>a!<Esc>");
    assert_eq!(e.text(), "x\nx!");
}

#[test]
fn awaiting_char() {
    let mut e = Editor::new("abc");
    e.feed(Key::Char('f'));
    assert!(e.awaiting_char());
    e.feed(Key::Char('c'));
    assert!(!e.awaiting_char());
    e.feed(Key::Char('d'));
    e.feed(Key::Char('t'));
    assert!(e.awaiting_char());
}
