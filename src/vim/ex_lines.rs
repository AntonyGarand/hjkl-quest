//! Ex commands on whole lines: `:d`, `:m`, `:t`, `:normal`, `:g`, `:sort`,
//! `:j`, `:>` and `:<`. Each is one undo step, and `:g` follows the rows
//! it still has to visit as lines come and go.

use super::ex::ExCmd;
use super::operator::Span;
use super::substitute::split_unescaped;
use super::{Editor, Key, Outcome, find_all, parse_keys, pos};

/// Row `r` moved by `delta` lines (never above the first).
fn shifted(r: usize, delta: i64) -> usize {
    (r as i64 + delta).max(0) as usize
}

impl Editor {
    /// Lines `a..=b` disappear: the rows `:g` still visits follow them.
    fn retarget_removed(&mut self, a: usize, b: usize) {
        if let Some(targets) = &mut self.g_targets {
            let n = b - a + 1;
            targets.retain(|&t| t < a || t > b);
            for t in targets.iter_mut().filter(|t| **t > b) {
                *t -= n;
            }
        }
    }

    /// `n` lines appear at row `at`.
    pub(super) fn retarget_inserted(&mut self, at: usize, n: usize) {
        if let Some(targets) = &mut self.g_targets {
            for t in targets.iter_mut().filter(|t| **t >= at) {
                *t += n;
            }
        }
    }

    /// The `n` lines up to row `b` move to row `at` (counted once they are out).
    fn retarget_moved(&mut self, b: usize, n: usize, at: usize) {
        if let Some(targets) = &mut self.g_targets {
            for t in targets.iter_mut() {
                let out = if *t > b { *t - n } else { *t };
                *t = if out >= at { out + n } else { out };
            }
        }
    }

    /// The lines after `row` moved by `delta`.
    fn retarget_after(&mut self, row: usize, delta: i64) {
        if let Some(targets) = &mut self.g_targets {
            for t in targets.iter_mut().filter(|t| **t > row) {
                *t = shifted(*t, delta);
            }
        }
    }

    /// `:d`: deletes rows `a..=b` into a register.
    pub(super) fn ex_delete(&mut self, a: usize, b: usize) -> Outcome {
        let before = self.checkpoint();
        let text = self.span_text(Span::Lines(a, b));
        self.store(text, false);
        self.delete_span(Span::Lines(a, b));
        self.retarget_removed(a, b);
        self.commit(before);
        Outcome::Ok
    }

    /// `:m {address}` moves the lines below the address; `:t` copies them.
    pub(super) fn ex_move(&mut self, ex: &ExCmd<'_>) -> Outcome {
        let copy = ex.name.starts_with('t') || ex.name.starts_with('c');
        let (a, b) = (ex.a, ex.b);
        let dest = match self.parse_addr(ex.arg.trim()) {
            Ok((Some(d), r)) if r.trim().is_empty() => d,
            Ok(_) => return Outcome::Bell("E14: Missing address".into()),
            Err(msg) => return Outcome::Bell(msg),
        };
        if !copy && dest >= a as i64 && dest < b as i64 {
            return Outcome::Bell("E134: Cannot move a range of lines into itself".into());
        }
        let before = self.checkpoint();
        let block: Vec<Vec<char>> = self.lines[a..=b].to_vec();
        let n = block.len();
        let at = if copy {
            let at = (dest + 1) as usize;
            self.retarget_inserted(at, n);
            at
        } else {
            self.lines.drain(a..=b);
            // Where the lines go, once they are out.
            let at = if dest > b as i64 {
                (dest + 1) as usize - n
            } else {
                (dest + 1) as usize
            };
            self.retarget_moved(b, n, at);
            at
        };
        self.lines.splice(at..at, block);
        self.cursor = self.line_start(at + n - 1);
        self.commit(before);
        Outcome::Ok
    }

    /// `:normal {keys}`: runs normal-mode keys on each line of the range.
    pub(super) fn ex_normal(&mut self, ex: &ExCmd<'_>) -> Outcome {
        let keys: Vec<Key> = ex.arg.trim_start().chars().map(Key::Char).collect();
        if keys.is_empty() {
            return Outcome::Bell("E471: Argument required".into());
        }
        let before = self.checkpoint();
        self.quiet += 1;
        let mut rows: Vec<usize> = (ex.a..=ex.b).collect();
        while !rows.is_empty() {
            let row = rows.remove(0);
            if row >= self.lines.len() {
                break;
            }
            let len = self.lines.len();
            self.run_normal_at(row, &keys);
            let delta = self.lines.len() as i64 - len as i64;
            for r in &mut rows {
                *r = shifted(*r, delta);
            }
        }
        self.quiet -= 1;
        self.commit_if_changed(before);
        Outcome::Ok
    }

    /// `:g/pattern/command` runs a command on each line that matches;
    /// `:v` (or `:g!`) on each that doesn't.
    pub(super) fn ex_global(&mut self, ex: &ExCmd<'_>) -> Outcome {
        const USAGE: &str = "Usage: :g/pattern/command";
        let (bang, arg) = match ex.arg.strip_prefix('!') {
            Some(r) => (true, r),
            None => (false, ex.arg),
        };
        let invert = bang ^ ex.name.starts_with('v');
        let Some(delim) = arg.chars().next().filter(|c| !c.is_alphanumeric() && *c != ' ') else {
            return Outcome::Bell(USAGE.into());
        };
        let parts = split_unescaped(&arg[delim.len_utf8()..], delim, 2);
        let pat = parts[0].to_string();
        let sub = parts.get(1).copied().unwrap_or("").trim();
        if sub.is_empty() || pat.is_empty() {
            return Outcome::Bell(USAGE.into());
        }
        self.last_search = Some(pat.clone());
        let (a, b) = ex.range.unwrap_or((0, self.lines.len() - 1));
        let icase = self.options.icase(&pat);
        let targets: Vec<usize> = (a..=b)
            .filter(|&r| find_all(&pat, &self.lines[r], icase).is_empty() == invert)
            .collect();
        if targets.is_empty() {
            return Outcome::Bell(format!("Pattern not found: {pat}"));
        }
        let before = self.checkpoint();
        self.quiet += 1;
        let outer = self.g_targets.replace(targets);
        self.run_global(sub);
        self.g_targets = outer;
        self.quiet -= 1;
        self.clamp();
        self.commit_if_changed(before);
        Outcome::Ok
    }

    /// Runs `sub` on each row `:g` marked, while there are any left.
    fn run_global(&mut self, sub: &str) {
        // `:d`, `:m` and `:t` keep the rows up to date themselves.
        let name: String = sub.chars().take_while(char::is_ascii_alphabetic).collect();
        let structural = matches!(
            name.as_str(),
            "d" | "de" | "del" | "delete" | "m" | "mo" | "move" | "t" | "co" | "copy"
        );
        while let Some(row) = self
            .g_targets
            .as_mut()
            .filter(|t| !t.is_empty())
            .map(|t| t.remove(0))
        {
            if row >= self.lines.len() {
                continue;
            }
            self.cursor = pos(row, 0);
            let len = self.lines.len();
            if let Outcome::Bell(msg) = self.ex_line(sub)
                && !msg.starts_with("Pattern not found")
            {
                break;
            }
            if !structural {
                self.retarget_after(row, self.lines.len() as i64 - len as i64);
            }
        }
    }

    /// `:sort` (`:sort!` reversed, `:sort u` without repeats).
    pub(super) fn ex_sort(&mut self, ex: &ExCmd<'_>) -> Outcome {
        let (a, b) = ex.range.unwrap_or((0, self.lines.len() - 1));
        let before = self.checkpoint();
        let mut part: Vec<Vec<char>> = self.lines[a..=b].to_vec();
        part.sort();
        if ex.arg.starts_with('!') {
            part.reverse();
        }
        if ex.arg.contains('u') {
            part.dedup();
        }
        self.lines.splice(a..=b, part);
        self.cursor = pos(a, 0);
        self.commit_if_changed(before);
        Outcome::Ok
    }

    /// `:j`: joins the range's lines (the line and the next, alone).
    pub(super) fn ex_join(&mut self, ex: &ExCmd<'_>) -> Outcome {
        let a = ex.a;
        let b = if ex.range.is_some_and(|(x, y)| x != y) {
            ex.b
        } else {
            (a + 1).min(self.lines.len() - 1)
        };
        if b == a {
            return Outcome::Bell(String::new());
        }
        self.cursor = pos(a, 0);
        let keys = parse_keys(&format!("{}J", b - a + 1));
        self.run_normal_at(a, &keys);
        Outcome::Ok
    }

    /// `:>` and `:<` (`:>>` twice as far): shifts the range's lines.
    pub(super) fn ex_shift(&mut self, ex: &ExCmd<'_>) -> Outcome {
        let before = self.checkpoint();
        let depth = ex.rest.chars().take_while(|c| c.to_string() == ex.name).count();
        for _ in 0..depth {
            self.shift_lines(ex.a, ex.b, ex.name == ">");
        }
        self.cursor = self.line_start(ex.b);
        self.commit(before);
        Outcome::Ok
    }
}
