//! Patterns and replacements, against short lines.

use super::*;

fn find(pat: &str, text: &str) -> Option<(usize, usize)> {
    let line: Vec<char> = text.chars().collect();
    Regex::new(pat)
        .find_from(&line, 0, false)
        .map(|m| (m.start, m.end))
}

fn sub(pat: &str, rep: &str, text: &str) -> String {
    let line: Vec<char> = text.chars().collect();
    let m = Regex::new(pat).find_from(&line, 0, false).unwrap();
    let mut out = line[..m.start].to_vec();
    out.extend(expand(rep, &line, &m));
    out.extend_from_slice(&line[m.end..]);
    out.into_iter().collect()
}

#[test]
fn literals_and_dots() {
    assert_eq!(find("cat", "a cat sat"), Some((2, 5)));
    assert_eq!(find("c.t", "a cot"), Some((2, 5)));
    assert_eq!(find(r"1\.0", "1x0 1.0"), Some((4, 7)));
    assert_eq!(find("^a", "ba"), None);
    assert_eq!(find("a$", "ab a"), Some((3, 4)));
    assert_eq!(find("a$b", "xa$b"), Some((1, 4)));
    assert_eq!(find("*", "a*"), Some((1, 2)));
}

#[test]
fn repeats() {
    assert_eq!(find("lo*", "hellooo"), Some((2, 3)));
    assert_eq!(find(r"o\+", "hellooo!"), Some((4, 7)));
    assert_eq!(find(r"colou\=r", "color"), Some((0, 5)));
    assert_eq!(find(r"colou\?r", "colour"), Some((0, 6)));
    assert_eq!(find(r"a\{2}", "a aa"), Some((2, 4)));
    assert_eq!(find(r"a\{2,3}", "aaaa"), Some((0, 3)));
    assert_eq!(find(r"<.\{-}>", "<a><b>"), Some((0, 3)));
    assert_eq!(find("<.*>", "<a><b>"), Some((0, 6)));
}

#[test]
fn classes() {
    assert_eq!(find(r"\d\+", "abc 123 x"), Some((4, 7)));
    assert_eq!(find("[aeiou]", "xyz e"), Some((4, 5)));
    assert_eq!(find("[^a-z ]", "abc D"), Some((4, 5)));
    assert_eq!(find(r"\s\+$", "abc   "), Some((3, 6)));
    assert_eq!(find(r"\u\l\+", "abc Def"), Some((4, 7)));
    assert_eq!(find(r"\w\+", "  foo_1 "), Some((2, 7)));
    assert_eq!(find("[]x]", "a]"), Some((1, 2)));
}

#[test]
fn alternation_groups_and_words() {
    assert_eq!(find(r"cat\|dog", "a dog"), Some((2, 5)));
    assert_eq!(find(r"\<is\>", "this is"), Some((5, 7)));
    assert_eq!(find(r"\(ab\)\+", "xababc"), Some((1, 5)));
    assert_eq!(find(r"\(a\)\1", "a aa"), Some((2, 4)));
    assert_eq!(find(r"foo\zsbar", "foobar"), Some((3, 6)));
    assert_eq!(find(r"foo\zebar", "foo foobar"), Some((4, 7)));
    assert_eq!(find(r"\v(cat|dog)s?", "dogs"), Some((0, 4)));
    assert_eq!(find(r"\v<\d{3}>", "12 345 6789"), Some((3, 6)));
}

#[test]
fn replacements() {
    assert_eq!(sub(r"\(\w\+\) \(\w\+\)", r"\2 \1", "hello world"), "world hello");
    assert_eq!(sub(r"\w\+", r"\u&", "word"), "Word");
    assert_eq!(sub(r"\w\+", r"\U&\E!", "loud"), "LOUD!");
    assert_eq!(sub("a", r"\&&", "a"), "&a");
}

#[test]
fn case() {
    let line: Vec<char> = "Hello".chars().collect();
    assert!(Regex::new("hello").match_here(&line, 0, true).is_some());
    assert!(Regex::new("[a-z]ello").match_here(&line, 0, true).is_some());
    assert!(Regex::new("hello").match_here(&line, 0, false).is_none());
}
