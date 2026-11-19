//! What each isle insists on: the commands its lesson is about. Reaching
//! the goal without using them doesn't clear the isle.
//!
//! A requirement is shown as its label and checked with patterns over the
//! commands the player completed (`3e`, `"add`, `ci"`, `:%s/a/b/g`,
//! `/gold`). Patterns are alternatives joined by `|`, where `…` is any
//! text, `·` any one character and `⁰` a count (one or more digits). A
//! count in front of a command is ignored unless the pattern asks for one.

/// The commands isle `title` must see used: (label, patterns).
pub fn requirements(title: &str) -> &'static [(&'static str, &'static str)] {
    match title {
        "First Steps" => &[("j or k", "j|k"), ("h or l", "h|l")],
        "Counting Sheep" => &[("a count across: 5l", "⁰l|⁰h"), ("a count down: 3j", "⁰j|⁰k")],
        "Word Hopping" => &[("w", "w"), ("e", "e"), ("b", "b")],
        "Punctuation Pass" => &[("W or B", "W|B"), ("E", "E")],
        "Edge of the World" => &[("0", "0"), ("^", "^"), ("$", "$")],
        "Long Jumps" => &[("{n}G", "⁰G"), ("gg", "gg")],
        "Seek and Find" => &[("f{c}", "f·"), ("F{c} or T{c}", "F·|T·"), ("t{c}", "t·"), ("; or ,", ";|,")],
        "Bug Hunt" => &[("a count and f: 3fb", "⁰f·|⁰t·|⁰F·|⁰T·")],
        "Infestation" => &[("/text", "/·…"), ("n or N", "n|N")],
        "Paragraph Isles" => &[("} or {", "}|{")],
        "Mirror Matching" => &[("%", "%")],
        "Search Party" => &[("/text", "/·…"), ("n or N", "n|N")],
        "Looking Back" => &[("?text", "?·…"), ("Ctrl-o", "<C-o>")],
        "Chomp" => &[("x or X", "x|X"), ("r{c}", "r·"), ("~", "~")],
        "Demolition" => &[("dd", "dd"), ("D", "D"), ("d{motion}", "dw|de|db|dW|dE|dB|d$|d0|d^|dj|dk|dG|dgg|dt·|df·|dT·|dF·|d}|d{|d%|d⁰…")],
        "Change of Heart" => &[("c{motion}", "c·|c··"), ("C", "C")],
        "Insert Coin" => &[("I", "I"), ("A", "A"), ("o or O", "o|O"), ("i or a", "i|a")],
        "Copycat" => &[("y{motion}, yy or Y", "y·…|Y"), ("p or P", "p|P")],
        "Time Travel" => &[(".", ".")],
        "Inside Out" => &[("a text object: ci\", di(", "·i·|·a·")],
        "Highlights" => &[("v", "v"), ("V", "V")],
        "Ex Marks the Spot" => &[(":s/old/new/", ":…s/·…")],
        "Case Closed" => &[(":set ic or \\c", ":se…|…\\c…")],
        "Ask First" => &[(":s///c", ":…s/…/…/…c…")],
        "The Refactor" => &[(":s", ":…s/·…"), ("c{motion}", "c·…")],
        "Scrolls" => &[(":r FILE", ":r ·…|:read ·…")],
        "Copy Desk" => &[(":w FILE", ":…w ·…|:…write ·…")],
        "Two Scrolls" => &[(":e FILE", ":e ·…|:edit ·…"), (":e #", ":e #|:edit #"), (":w", ":w|:write")],
        "Named Vaults" => &[("\"x, a named register", "\"··…")],
        "Waymarks" => &[("m{a-z}", "m·"), ("`{a} or '{a}", "…`·|…'·")],
        "Tally Marks" => &[("Ctrl-a", "<C-a>"), ("Ctrl-x", "<C-x>")],
        "Wind-up Golem" => &[("q{a}", "q·"), ("@{a}", "@·")],
        "Assembly Line" => &[("q{a}", "q·"), ("@{a}", "@·")],
        "Terraces" => &[("> or <", ">…|<…")],
        "Shouting Match" => &[("gU, gu or g~", "gU…|gu…|g~…")],
        "Overwrite" => &[("R", "R")],
        "Ice Columns" => &[("Ctrl-v", "<C-v>")],
        "Next Match" => &[("gn: cgn, dgn", "…gn")],
        "Tag, You're It" => &[("it or at", "·it|·at"), ("ip or ap", "·ip|·ap")],
        "Global Decree" => &[(":g or :v", ":…g/…|:…v/…|:…g!/…"), (":sort", ":sor…")],
        "Moving Day" => &[(":m", ":·…m…"), (":t", ":·…t…|:·…co…")],
        "Normal Order" => &[(":norm", ":…norm…")],
        "The Grand Refactor" => &[(":g", ":…g·…"), ("a macro: q{a} … @{a}", "@·")],
        _ => &[],
    }
}

/// Whether `token`, a command as typed, satisfies `patterns`.
pub fn satisfied(patterns: &str, token: &str) -> bool {
    patterns.split('|').any(|p| {
        let want: Vec<char> = p.chars().collect();
        let mut have: Vec<char> = token.chars().collect();
        // A count in front doesn't change the command: 3e is still e.
        if !p.starts_with('⁰') && have.first().is_some_and(|c| ('1'..='9').contains(c)) {
            let n = have.iter().take_while(|c| c.is_ascii_digit()).count();
            have.drain(..n);
        }
        glob(&want, &have)
    })
}

fn glob(p: &[char], s: &[char]) -> bool {
    match p.first() {
        None => s.is_empty(),
        Some('…') => (0..=s.len()).any(|k| glob(&p[1..], &s[k..])),
        Some('·') => !s.is_empty() && glob(&p[1..], &s[1..]),
        Some('⁰') => {
            let n = s.iter().take_while(|c| c.is_ascii_digit()).count();
            (1..=n).any(|k| glob(&p[1..], &s[k..]))
        }
        Some(c) => s.first() == Some(c) && glob(&p[1..], &s[1..]),
    }
}

/// The labels of the requirements none of `used` satisfies.
pub fn missing(title: &str, used: &[String]) -> Vec<&'static str> {
    requirements(title).iter().filter(|(_, p)| !used.iter().any(|t| satisfied(p, t))).map(|(label, _)| *label).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels::LEVELS;

    #[test]
    fn every_isle_has_requirements() {
        for level in LEVELS {
            assert!(!requirements(level.title).is_empty(), "{} has no requirements", level.title);
        }
    }

    #[test]
    fn patterns() {
        assert!(satisfied("e", "3e"));
        assert!(!satisfied("⁰l", "l"));
        assert!(satisfied("⁰l", "15l"));
        assert!(satisfied("0", "0"));
        assert!(satisfied("f·", "f,"));
        assert!(!satisfied("f·", "f"));
        assert!(satisfied("/·…", "/g"));
        assert!(!satisfied("/·…", "/"));
        assert!(satisfied(":…s/·…", ":%s/foo/baz/g"));
        assert!(satisfied("·i·|·a·", "ci\""));
        assert!(satisfied("<C-a>", "27<C-a>"));
        assert!(satisfied("@·", "5@a"));
        assert!(satisfied("…`·|…'·", "d`a"));
        assert!(!satisfied("dd", "d"));
    }
}
