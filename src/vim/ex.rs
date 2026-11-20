//! Ex commands: splitting a command line at `|`, line addresses and
//! ranges, and handing each command to the code that runs it.

use super::{Editor, ISLE, Outcome};

/// What was parsed, and the text left after it, or an error message.
type Parsed<'a, T> = Result<(T, &'a str), String>;

/// One ex command, taken apart.
pub(super) struct ExCmd<'a> {
    /// The whole command, as typed (for the game, or an error).
    pub(super) line: &'a str,
    /// Its name: letters (`s`, `move`), or a single sign (`&`, `>`, `!`).
    pub(super) name: &'a str,
    /// What follows the name.
    pub(super) arg: &'a str,
    /// The name and what follows (`:>>` counts its `>`s here).
    pub(super) rest: &'a str,
    /// The range given, if any.
    pub(super) range: Option<(usize, usize)>,
    /// The rows it covers: the range, or the cursor's row.
    pub(super) a: usize,
    pub(super) b: usize,
}

/// The digits `s` starts with.
fn leading_digits(s: &str) -> &str {
    &s[..s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len())]
}

/// The letters a command starts with, after any range: its name.
fn command_name(part: &str) -> &str {
    let part = part
        .trim_start()
        .trim_start_matches(|c: char| !c.is_ascii_alphabetic());
    &part[..part
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(part.len())]
}

/// Splits a command line at its `|`s (`\|` is no split); `:g` and
/// `:normal` take the rest of the line, `|` and all.
fn split_bars(line: &str) -> Vec<String> {
    let mut parts: Vec<String> = vec![String::new()];
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        let current = parts.last_mut().unwrap();
        let takes_rest = matches!(
            command_name(current),
            "g" | "global" | "v" | "vglobal" | "norm" | "normal"
        );
        if c == '\\' && chars.peek() == Some(&'|') {
            current.push('\\');
            current.push('|');
            chars.next();
        } else if c == '|' && !takes_rest {
            parts.push(String::new());
        } else {
            current.push(c);
        }
    }
    parts
}

impl Editor {
    /// A command line: commands joined by `|` run one after the other,
    /// until one fails or is the game's.
    pub(super) fn ex_line(&mut self, line: &str) -> Outcome {
        let parts = split_bars(line);
        if parts.len() == 1 {
            return self.ex_command(line);
        }
        let mut last = Outcome::Ok;
        for part in parts.iter().filter(|p| !p.trim().is_empty()) {
            last = self.ex_command(part);
            if matches!(last, Outcome::Bell(_) | Outcome::Command(_)) {
                break;
            }
        }
        last
    }

    /// One ex command: its range, name and argument.
    fn ex_command(&mut self, line: &str) -> Outcome {
        self.active_reg = None;
        let cmd = line.trim_start();
        let (range, rest) = match self.parse_range(cmd) {
            Ok(r) => r,
            Err(msg) => return Outcome::Bell(msg),
        };
        let rest = rest.trim_start();
        if rest.trim().is_empty() {
            self.goto_range(range);
            return Outcome::Ok;
        }
        let letters = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_alphabetic()).len();
        let name_len = if letters > 0 {
            letters
        } else {
            rest.chars().next().map_or(0, char::len_utf8)
        };
        let (a, b) = range.unwrap_or((self.cursor.row, self.cursor.row));
        self.run_ex(&ExCmd {
            line: cmd.trim(),
            name: &rest[..name_len],
            arg: &rest[name_len..],
            rest,
            range,
            a,
            b,
        })
    }

    /// Runs a parsed ex command. One the editor doesn't know is the
    /// game's (`:q`, `:help`...), unless it was given a range.
    fn run_ex(&mut self, ex: &ExCmd<'_>) -> Outcome {
        let has_arg = !ex.arg.trim().is_empty();
        match ex.name {
            "s" | "su" | "substitute" | "&" => self.ex_substitute(ex),
            "d" | "de" | "del" | "delete" => self.ex_delete(ex.a, ex.b),
            "m" | "mo" | "move" | "t" | "co" | "copy" => self.ex_move(ex),
            "norm" | "normal" => self.ex_normal(ex),
            "g" | "global" | "v" | "vglobal" => self.ex_global(ex),
            "sor" | "sort" => self.ex_sort(ex),
            // `:w` alone writes the file being edited; on the isle itself
            // it's the game's (progress saves by itself).
            "w" | "write" if !has_arg && ex.range.is_none() && self.buffer_name != ISLE => {
                self.write_buffer()
            }
            // (`:e!` is the game's: start the isle again.)
            "e" | "edit" if has_arg && !ex.arg.starts_with('!') => self.edit_file(ex.arg.trim()),
            "reg" | "registers" | "di" | "display" => self.show_registers(),
            "w" | "write" if has_arg => self.write_lines(ex),
            "r" | "re" | "read" if has_arg => self.read_file(ex),
            "!" => match self.shell(ex.arg.trim()) {
                Ok(out) => Outcome::Info(out.replace('\n', "  ")),
                Err(msg) => Outcome::Bell(msg),
            },
            "se" | "set" => self.set_options(ex.arg),
            "noh" | "nohl" | "nohlsearch" => {
                self.options.hlsearch = false;
                Outcome::Ok
            }
            "j" | "join" => self.ex_join(ex),
            ">" | "<" => self.ex_shift(ex),
            _ if ex.range.is_none() => Outcome::Command(ex.line.to_string()),
            _ => Outcome::Bell(format!("E492: Not an editor command: {}", ex.line)),
        }
    }

    /// A range alone (`:12`): goes to its last line.
    fn goto_range(&mut self, range: Option<(usize, usize)>) {
        if let Some((_, b)) = range {
            self.jump_from_here();
            self.cursor = self.line_start(b);
            self.want = self.cursor.col;
        }
    }

    /// One ex address: a number, `.`, `$`, `'x`, with `+n`/`-n` offsets.
    /// Rows are 0-based; line "0" (before the first) is -1.
    pub(super) fn parse_addr<'a>(&self, s: &'a str) -> Parsed<'a, Option<i64>> {
        let (mut row, mut rest) = self.addr_base(s)?;
        while let Some(sign) = rest.chars().next().filter(|c| *c == '+' || *c == '-') {
            let after = &rest[1..];
            let digits = leading_digits(after);
            let by = if digits.is_empty() {
                1
            } else {
                digits.parse::<i64>().unwrap_or(0)
            };
            let base = row.unwrap_or(self.cursor.row as i64);
            row = Some(if sign == '+' { base + by } else { base - by });
            rest = &after[digits.len()..];
        }
        let last = self.lines.len() as i64 - 1;
        if let Some(r) = row
            && (r < -1 || r > last)
        {
            return Err("E16: Invalid range".into());
        }
        Ok((row, rest))
    }

    /// The part of an address before its offsets.
    fn addr_base<'a>(&self, s: &'a str) -> Parsed<'a, Option<i64>> {
        let digits = leading_digits(s);
        if !digits.is_empty() {
            return Ok((Some(digits.parse::<i64>().unwrap_or(0) - 1), &s[digits.len()..]));
        }
        if let Some(r) = s.strip_prefix('.') {
            return Ok((Some(self.cursor.row as i64), r));
        }
        if let Some(r) = s.strip_prefix('$') {
            return Ok((Some(self.lines.len() as i64 - 1), r));
        }
        let Some(r) = s.strip_prefix('\'') else {
            return Ok((None, s));
        };
        let mut chars = r.chars();
        let name = chars.next().ok_or("E20: Mark not set")?;
        let name = if name == '`' { '\'' } else { name };
        let p = self.marks.get(&name).ok_or("E20: Mark not set")?;
        Ok((Some(p.row as i64), chars.as_str()))
    }

    /// An optional range: `%`, `a`, or `a,b`.
    fn parse_range<'a>(&self, s: &'a str) -> Parsed<'a, Option<(usize, usize)>> {
        if let Some(rest) = s.strip_prefix('%') {
            return Ok((Some((0, self.lines.len() - 1)), rest));
        }
        let (a, rest) = self.parse_addr(s)?;
        let Some(a) = a else { return Ok((None, s)) };
        let (b, rest) = match rest.strip_prefix(',') {
            Some(r) => {
                let (b, r) = self.parse_addr(r)?;
                (b.ok_or("E14: Missing address")?, r)
            }
            None => (a, rest),
        };
        let (a, b) = (a.max(0) as usize, b.max(0) as usize);
        Ok((Some((a.min(b), a.max(b))), rest))
    }
}
