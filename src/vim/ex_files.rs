//! Files and the shell (vimtutor 5.1-5.5): `:w`, `:e`, `:r`, `:!`, over a
//! small pretend file system.

use super::buffer::to_lines;
use super::ex::ExCmd;
use super::{Editor, Mode, Outcome};

impl Editor {
    /// `:w` on a file being edited: saves the buffer to it.
    pub(super) fn write_buffer(&mut self) -> Outcome {
        let name = self.buffer_name.clone();
        self.files.insert(name.clone(), self.text());
        Outcome::Info(format!("\"{name}\" {}L written", self.lines.len()))
    }

    /// `:w {file}` (`:'<,'>w {file}`): writes the range, or every line.
    pub(super) fn write_lines(&mut self, ex: &ExCmd<'_>) -> Outcome {
        let name = ex.arg.trim().to_string();
        let (a, b) = ex.range.unwrap_or((0, self.lines.len() - 1));
        let text = self.lines_text(a, b);
        self.files.insert(name.clone(), text);
        Outcome::Info(format!("\"{name}\" {}L written", b - a + 1))
    }

    /// `:e {file}` (`:e #`: the one before): edits another file. The text
    /// being left stays open, as with 'hidden'.
    pub(super) fn edit_file(&mut self, arg: &str) -> Outcome {
        let target = if arg == "#" {
            match &self.alternate {
                Some(a) => a.clone(),
                None => return Outcome::Bell("E23: No alternate file".into()),
            }
        } else {
            arg.to_string()
        };
        if target == self.buffer_name {
            return Outcome::Ok;
        }
        let here = std::mem::replace(&mut self.buffer_name, target.clone());
        self.files.insert(here.clone(), self.text());
        self.cursors.insert(here.clone(), self.cursor);
        self.alternate = Some(here);
        let text = self.files.get(&target).cloned();
        let new = text.is_none();
        self.lines = to_lines(&text.unwrap_or_default());
        self.mode = Mode::Normal;
        self.set_cursor(self.cursors.get(&target).copied().unwrap_or_default());
        self.undo.clear();
        self.redo.clear();
        self.line_undo = None;
        self.changed();
        Outcome::Info(if new {
            format!("\"{target}\" [New]")
        } else {
            format!("\"{target}\" {}L", self.lines.len())
        })
    }

    /// `:r {file}` or `:r !{cmd}`: puts the text below the range's last line.
    pub(super) fn read_file(&mut self, ex: &ExCmd<'_>) -> Outcome {
        let arg = ex.arg.trim();
        let text = match arg.strip_prefix('!') {
            Some(cmd) => self.shell(cmd.trim()),
            None => self
                .files
                .get(arg)
                .cloned()
                .ok_or_else(|| format!("E484: Can't open file {arg}")),
        };
        let text = match text {
            Ok(text) => text,
            Err(msg) => return Outcome::Bell(msg),
        };
        let before = self.checkpoint();
        let new = to_lines(&text);
        let (row, n) = (ex.b + 1, new.len());
        self.lines.splice(row..row, new);
        self.retarget_inserted(row, n);
        self.cursor = self.line_start(row);
        self.commit(before);
        Outcome::Ok
    }

    /// A pretend shell for `:!cmd` and `:r !cmd`.
    pub(super) fn shell(&self, cmd: &str) -> Result<String, String> {
        let mut words = cmd.split_whitespace();
        let name = words.next().ok_or("E471: Argument required")?;
        let rest: Vec<&str> = words.collect();
        match name {
            "ls" | "dir" => Ok(self.files.keys().cloned().collect::<Vec<_>>().join("\n")),
            "cat" | "type" => rest
                .iter()
                .map(|f| {
                    self.files
                        .get(*f)
                        .cloned()
                        .ok_or(format!("cat: {f}: No such file"))
                })
                .collect::<Result<Vec<_>, _>>()
                .map(|t| t.join("\n")),
            "echo" => Ok(rest.join(" ")),
            "pwd" => Ok("/archipelago".into()),
            "date" => Ok("Thu Nov 19 09:00:00 UTC 2026".into()),
            "whoami" => Ok("wisp".into()),
            _ => Err(format!(
                "{name}: command not found (try ls, cat, echo, pwd, date)"
            )),
        }
    }
}
