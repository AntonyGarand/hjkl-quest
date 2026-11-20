//! How the buffer differs from an editing level's goal: which characters
//! are extra, which of the goal's are already there, and where the missing
//! ones go. Lines are matched first, then the characters of lines that
//! differ, so one edit doesn't light up everything after it.

use crate::vim::{Pos, pos};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Diff {
    /// Per buffer character: not where the goal wants it (to delete or change).
    pub wrong: Vec<Vec<bool>>,
    /// Per goal character: already in the buffer.
    pub present: Vec<Vec<bool>>,
    /// Text missing inside a buffer line: where it goes, and what it is.
    pub inserts: Vec<(Pos, String)>,
    /// Whole lines missing: the buffer row they go before (the buffer's
    /// length for the end), and the goal row they are.
    pub missing_lines: Vec<(usize, usize)>,
}

impl Diff {
    pub fn wrong_count(&self) -> usize {
        self.wrong.iter().flatten().filter(|w| **w).count()
    }

    /// Goal characters still missing (line breaks of missing lines count too).
    pub fn missing_count(&self) -> usize {
        self.present.iter().flatten().filter(|p| !**p).count() + self.missing_lines.len()
    }

    pub fn present_count(&self) -> usize {
        self.present.iter().flatten().filter(|p| **p).count()
    }
}

/// The longest common subsequence of `a` and `b`, as pairs of indices.
/// On a tie, `late` matches later in `b`, so what `a` lacks goes before
/// what it keeps ("pear" -> "ripe pear" inserts "ripe ", not "ri" + "pe");
/// otherwise earlier, so missing lines go at the end.
pub fn lcs<T: PartialEq>(a: &[T], b: &[T], late: bool) -> Vec<(usize, usize)> {
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![vec![0u16; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < n && j < m {
        if a[i] == b[j] && !(late && dp[i][j + 1] == dp[i][j]) {
            out.push((i, j));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

pub fn diff(buf: &[Vec<char>], goal: &[Vec<char>]) -> Diff {
    let mut d = Diff {
        wrong: buf.iter().map(|l| vec![true; l.len()]).collect(),
        present: goal.iter().map(|l| vec![false; l.len()]).collect(),
        inserts: Vec::new(),
        missing_lines: Vec::new(),
    };
    let mut anchors = lcs(buf, goal, false);
    anchors.push((buf.len(), goal.len()));
    let (mut bi, mut gi) = (0, 0);
    for (ba, ga) in anchors {
        // Between matched lines, pair the leftover lines in order.
        let paired = (ba - bi).min(ga - gi);
        for k in 0..paired {
            let (b, g) = (bi + k, gi + k);
            let same = lcs(&buf[b], &goal[g], true);
            for &(i, j) in &same {
                d.wrong[b][i] = false;
                d.present[g][j] = true;
            }
            // Runs of goal characters with no partner go before the next
            // matched buffer character (or at the end of the line).
            let mut run = String::new();
            let mut next = same.iter().peekable();
            for (j, &c) in goal[g].iter().enumerate() {
                while next.peek().is_some_and(|&&(_, mj)| mj < j) {
                    next.next();
                }
                if d.present[g][j] {
                    continue;
                }
                run.push(c);
                let ends = j + 1 == goal[g].len() || d.present[g][j + 1];
                if ends {
                    let col = same
                        .iter()
                        .find(|&&(_, mj)| mj > j)
                        .map_or(buf[b].len(), |&(i, _)| i);
                    d.inserts.push((pos(b, col), std::mem::take(&mut run)));
                }
            }
        }
        for g in gi + paired..ga {
            d.missing_lines.push((ba, g));
        }
        if ba < buf.len() {
            d.wrong[ba].iter_mut().for_each(|w| *w = false);
            d.present[ga].iter_mut().for_each(|p| *p = true);
        }
        bi = ba + 1;
        gi = ga + 1;
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &str) -> Vec<Vec<char>> {
        s.split('\n').map(|l| l.chars().collect()).collect()
    }

    #[test]
    fn missing_text_inside_a_line() {
        let d = diff(&lines("list\n- pear"), &lines("my list:\n- ripe pear"));
        assert_eq!(
            d.inserts,
            vec![
                (pos(0, 0), "my ".into()),
                (pos(0, 4), ":".into()),
                (pos(1, 1), " ripe".into())
            ]
        );
        assert_eq!(d.wrong_count(), 0);
        assert_eq!(d.missing_count(), 9);
    }

    #[test]
    fn extra_and_changed_characters() {
        let d = diff(&lines("bad bad good"), &lines("good"));
        assert_eq!(d.wrong_count(), 8);
        assert!(d.inserts.is_empty());
        let d = diff(&lines("cat"), &lines("cut"));
        assert_eq!(d.wrong[0], vec![false, true, false]);
        assert_eq!(d.inserts, vec![(pos(0, 2), "u".into())]);
    }

    #[test]
    fn missing_lines_in_the_middle_and_at_the_end() {
        let d = diff(&lines("a\nc"), &lines("a\nb\nc\nd"));
        assert_eq!(d.missing_lines, vec![(1, 1), (2, 3)]);
        assert_eq!(d.present[1], vec![false]);
        assert_eq!(d.present[2], vec![true]);
    }

    #[test]
    fn identical_text_has_nothing_to_do() {
        let d = diff(&lines("x\ny"), &lines("x\ny"));
        assert_eq!(d.wrong_count() + d.missing_count(), 0);
    }
}
