//! Ex commands: ranges, `:g`, `:s`, files, options and the command line.

use super::run;
use crate::vim::*;

#[test]
fn ex_line_commands() {
    let t = "a\nb\nc\nd";
    assert_eq!(run(t, pos(0, 0), ":2,3d<CR>").text(), "a\nd");
    assert_eq!(run(t, pos(0, 0), ":m$<CR>").text(), "b\nc\nd\na");
    assert_eq!(run(t, pos(0, 0), ":3m0<CR>").text(), "c\na\nb\nd");
    assert_eq!(run(t, pos(0, 0), ":1,2t$<CR>").text(), "a\nb\nc\nd\na\nb");
    assert_eq!(run(t, pos(0, 0), ":t.<CR>").text(), "a\na\nb\nc\nd");
    assert_eq!(run(t, pos(0, 0), ":g/^/m0<CR>").text(), "d\nc\nb\na");
    assert_eq!(run("x1\ny\nx2\nz", pos(0, 0), ":g/x/d<CR>").text(), "y\nz");
    assert_eq!(run("x1\ny\nx2\nz", pos(0, 0), ":v/x/d<CR>").text(), "x1\nx2");
    assert_eq!(
        run("x1\ny\nx2", pos(0, 0), ":g/x/norm A!<CR>").text(),
        "x1!\ny\nx2!"
    );
    assert_eq!(run("a\nb", pos(0, 0), ":%norm I- <CR>").text(), "- a\n- b");
    assert_eq!(run("a\nb\nc", pos(0, 0), ":%s/^/> /<CR>").text(), "> a\n> b\n> c");
    assert_eq!(run("a,b", pos(0, 0), ":s/,/\\r/<CR>").text(), "a\nb");
    assert_eq!(run("b\nc\na", pos(0, 0), ":sort<CR>").text(), "a\nb\nc");
    // One undo step for the whole command.
    assert_eq!(run("x1\ny\nx2", pos(0, 0), ":g/x/d<CR>u").text(), "x1\ny\nx2");
}

#[test]
fn ex_ranges_and_repeats() {
    assert_eq!(
        run("a\nb\nc\nd", pos(0, 0), "Vj:s/$/;/<CR>").text(),
        "a;\nb;\nc\nd"
    );
    assert_eq!(run("a\nb\nc\nd", pos(0, 0), "jma:'a,$d<CR>").text(), "a");
    assert_eq!(
        run("cat cat\ncat", pos(0, 0), ":s/cat/dog/<CR>j&").text(),
        "dog cat\ndog"
    );
    assert_eq!(
        run("cat cat\ncat", pos(0, 0), ":s/cat/dog/g<CR>g&").text(),
        "dog dog\ndog"
    );
    assert_eq!(run("a\nb\nc", pos(0, 0), ":.,+1j<CR>").text(), "a b\nc");
    assert_eq!(run("a\nb", pos(0, 0), ":%><CR>").text(), "    a\n    b");
}

#[test]
fn substitute_with_confirmation() {
    let e = run("cat cat cat", pos(0, 0), ":s/cat/dog/gc<CR>yny");
    assert_eq!(e.text(), "dog cat dog");
    assert_eq!(e.mode, Mode::Normal);
    assert_eq!(
        run("cat cat\ncat", pos(0, 0), ":%s/cat/dog/gc<CR>na").text(),
        "cat dog\ndog"
    );
    assert_eq!(
        run("cat cat cat", pos(0, 0), ":s/cat/dog/gc<CR>l").text(),
        "dog cat cat"
    );
    assert_eq!(
        run("cat cat cat", pos(0, 0), ":s/cat/dog/gc<CR>yq").text(),
        "dog cat cat"
    );
    assert_eq!(
        run("cat cat cat", pos(0, 0), ":s/cat/dog/gc<CR>yyqu").text(),
        "cat cat cat"
    );
    let mut e = run("cat", pos(0, 0), "");
    let out = e.feed_all_last(&parse_keys(":s/cat/dog/c<CR>"));
    assert_eq!(out, Outcome::Info("replace with dog (y/n/a/q/l)?".into()));
}

#[test]
fn files_and_shell() {
    let mut e = Editor::new("# list\nkeep");
    e.files.insert("fruit.txt".into(), "apple\npear".into());
    e.feed_all(&parse_keys(":r fruit.txt<CR>"));
    assert_eq!(e.text(), "# list\napple\npear\nkeep");
    e.feed_all(&parse_keys(":2,3w basket.txt<CR>"));
    assert_eq!(e.files.get("basket.txt").map(String::as_str), Some("apple\npear"));
    e.feed_all(&parse_keys("G:r !ls<CR>"));
    assert_eq!(e.text(), "# list\napple\npear\nkeep\nbasket.txt\nfruit.txt");
    let out = e.feed_all_last(&parse_keys(":!echo hi there<CR>"));
    assert_eq!(out, Outcome::Info("hi there".into()));
    // Visual lines, then :w writes just them.
    let mut e = Editor::new("a\nb\nc");
    e.feed_all(&parse_keys("jVj:w part.txt<CR>"));
    assert_eq!(e.files.get("part.txt").map(String::as_str), Some("b\nc"));
    assert!(matches!(
        run("x", pos(0, 0), "").feed_all_last(&parse_keys(":r nope<CR>")),
        Outcome::Bell(_)
    ));
}

#[test]
fn search_options() {
    let t = "Gold gold GOLD";
    assert_eq!(run(t, pos(0, 0), "/gold<CR>n").cursor, pos(0, 5));
    assert_eq!(run(t, pos(0, 0), ":set ic<CR>/gold<CR>").cursor, pos(0, 5));
    assert_eq!(run(t, pos(0, 0), ":set ic<CR>/gold<CR>n").cursor, pos(0, 10));
    assert_eq!(run(t, pos(0, 0), ":set ic noic<CR>/gold<CR>n").cursor, pos(0, 5));
    assert_eq!(run(t, pos(0, 0), "/gold\\c<CR>").cursor, pos(0, 5));
    assert_eq!(run(t, pos(0, 0), "/gold\\c<CR>n").cursor, pos(0, 10));
    // smartcase: a capital in the pattern makes it exact again.
    assert_eq!(run(t, pos(0, 0), ":set ic scs<CR>/GOLD<CR>").cursor, pos(0, 10));
    let e = run(t, pos(0, 0), ":set hls<CR>/gold<CR>");
    assert_eq!(e.search_highlights(), vec![(pos(0, 5), 4)]);
    assert_eq!(
        run(t, pos(0, 0), ":set hls<CR>/gold<CR>:noh<CR>").search_highlights(),
        Vec::new()
    );
    // incsearch lights up matches while the pattern is typed.
    let e = run(t, pos(0, 0), ":set is ic<CR>/go");
    assert_eq!(e.search_highlights().len(), 3);
    assert_eq!(run(t, pos(0, 0), ":%s/gold/x/g<CR>").text(), "Gold x GOLD");
    assert_eq!(run(t, pos(0, 0), ":set ic<CR>:%s/gold/x/g<CR>").text(), "x x x");
}

#[test]
fn command_line_completion() {
    let mut e = Editor::new("x");
    e.files.insert("recipe.txt".into(), "r".into());
    e.files.insert("readme.txt".into(), "m".into());
    e.feed_all(&parse_keys(":so<Tab>"));
    assert_eq!(e.cmdline, "sort");
    let mut e2 = e.clone();
    e2.feed(Key::Esc);
    e2.feed_all(&parse_keys(":r rec<Tab>"));
    assert_eq!(e2.cmdline, "r recipe.txt");
    let mut e3 = Editor::new("x");
    e3.files = e.files.clone();
    e3.feed_all(&parse_keys(":r re<Tab>"));
    assert_eq!(e3.cmdline, "r readme.txt");
    e3.feed(Key::Tab);
    assert_eq!(e3.cmdline, "r recipe.txt");
    let mut e4 = Editor::new("x");
    e4.files = e.files.clone();
    let out = e4.feed_all_last(&parse_keys(":r re<C-d>"));
    assert_eq!(out, Outcome::Info("readme.txt  recipe.txt".into()));
    let mut e5 = Editor::new("x");
    e5.feed_all(&parse_keys(":set hl<Tab>"));
    assert_eq!(e5.cmdline, "set hlsearch");
}

#[test]
fn switching_files_and_registers() {
    let mut e = Editor::new("eggs");
    e.files.insert("recipe.txt".into(), "flour\nsugar".into());
    e.feed_all(&parse_keys("yy:e recipe.txt<CR>Gp"));
    assert_eq!(e.buffer_name, "recipe.txt");
    assert_eq!(e.text(), "flour\nsugar\neggs");
    assert_eq!(
        e.feed_all_last(&parse_keys(":w<CR>")),
        Outcome::Info("\"recipe.txt\" 3L written".into())
    );
    assert_eq!(e.files["recipe.txt"], "flour\nsugar\neggs");
    e.feed_all(&parse_keys("kyy:e #<CR>p"));
    assert_eq!(e.buffer_name, ISLE);
    assert_eq!(e.text(), "eggs\nsugar");
    // :w on the isle itself is the game's.
    assert_eq!(
        e.feed_all_last(&parse_keys(":w<CR>")),
        Outcome::Command("w".into())
    );
    let mut e = Editor::new("one two");
    e.feed_all(&parse_keys("\"ayw"));
    assert_eq!(
        e.feed_all_last(&parse_keys(":reg<CR>")),
        Outcome::Info("\"\" one    \"a one ".into())
    );
}

#[test]
fn regex_substitution_and_bars() {
    assert_eq!(
        run("hello world", pos(0, 0), r":s/\(\w\+\) \(\w\+\)/\2 \1/<CR>").text(),
        "world hello"
    );
    assert_eq!(
        run("a1 b22 c333", pos(0, 0), r":s/\d\+/#/g<CR>").text(),
        "a# b# c#"
    );
    assert_eq!(
        run("cat dog", pos(0, 0), r":s/cat\|dog/pet/g<CR>").text(),
        "pet pet"
    );
    assert_eq!(run("a b", pos(0, 0), ":s/a/x/ | s/b/y/<CR>").text(), "x y");
    assert_eq!(
        run("ab\nab", pos(0, 0), ":%s/a/1/ | %s/b/2/<CR>").text(),
        "12\n12"
    );
    assert_eq!(
        run("one\ntwo", pos(0, 0), ":g/o/s/o/0/g | s/0/O/<CR>").text(),
        "One\ntwO"
    );
    assert_eq!(run("x.y xzy", pos(0, 0), r"/x\.y<CR>:s//Q/<CR>").text(), "Q xzy");
    assert_eq!(
        run("aa bb", pos(0, 0), r":s/\v(\w)\1/<\1>/g<CR>").text(),
        "<a> <b>"
    );
}
