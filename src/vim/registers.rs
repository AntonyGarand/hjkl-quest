//! Registers: where yanks and deletes go (`"x`, `"0`, `"1`..., `"-`),
//! what puts read, `:registers`, and macros (`q`, `@`).

use super::{Editor, Key, Outcome, Parse, Register};

/// A register's text as `:registers` shows it: control characters as `^`,
/// cut short after 18 characters.
fn preview(text: &str) -> String {
    let shown: String = text
        .replace('\n', "^J")
        .chars()
        .map(|c| if c.is_control() { '^' } else { c })
        .collect();
    if shown.chars().count() > 18 {
        format!("{}…", shown.chars().take(18).collect::<String>())
    } else {
        shown
    }
}

impl Editor {
    /// The text in register `name` (`"` is the unnamed one).
    pub fn register_text(&self, name: char) -> Option<String> {
        if name == '"' {
            return Some(self.register.text.clone());
        }
        self.registers
            .get(&name.to_ascii_lowercase())
            .map(|r| r.text.clone())
    }

    /// Whether `c` names a register a command can use.
    pub(super) fn valid_register(c: char) -> bool {
        c.is_ascii_alphanumeric() || matches!(c, '"' | '_' | '-')
    }

    /// Stores yanked or deleted text: in the register named with `"x`,
    /// else the unnamed one plus `"0` (yanks) or `"1`/`"-` (deletes).
    pub(super) fn store(&mut self, text: Register, yank: bool) {
        match self.active_reg {
            Some('_') => return,
            Some(c) if c.is_ascii_uppercase() => {
                self.append_to(c.to_ascii_lowercase(), &text);
                return;
            }
            Some(c) if c != '"' => {
                self.registers.insert(c, text.clone());
            }
            _ if yank => {
                self.registers.insert('0', text.clone());
            }
            _ if text.linewise || text.text.contains('\n') => {
                self.shift_numbered();
                self.registers.insert('1', text.clone());
            }
            _ => {
                self.registers.insert('-', text.clone());
            }
        }
        self.register = text;
    }

    /// `"X` (upper case): adds to register `x` instead of replacing it.
    fn append_to(&mut self, name: char, text: &Register) {
        let entry = self.registers.entry(name).or_default();
        if entry.linewise || text.linewise {
            if !entry.text.is_empty() || entry.linewise {
                entry.text.push('\n');
            }
            entry.linewise = true;
        }
        entry.text.push_str(&text.text);
        self.register = entry.clone();
    }

    /// Makes room in `"1` for a new delete: `"1` goes to `"2`, and so on.
    fn shift_numbered(&mut self) {
        for k in (1..9).rev() {
            let from = char::from_digit(k, 10).unwrap();
            if let Some(r) = self.registers.get(&from).cloned() {
                self.registers.insert(char::from_digit(k + 1, 10).unwrap(), r);
            }
        }
    }

    /// The register a put reads: `"x`, or the unnamed one.
    pub(super) fn read_reg(&self) -> Register {
        match self.active_reg {
            Some(c) if c != '"' => self
                .registers
                .get(&c.to_ascii_lowercase())
                .cloned()
                .unwrap_or_default(),
            _ => self.register.clone(),
        }
    }

    /// `:registers`: every register that holds something.
    pub(super) fn show_registers(&self) -> Outcome {
        let mut out = Vec::new();
        if !self.register.text.is_empty() {
            out.push(format!("\"\" {}", preview(&self.register.text)));
        }
        let mut names: Vec<&char> = self.registers.keys().collect();
        names.sort();
        for name in names {
            let text = &self.registers[name].text;
            if !text.is_empty() {
                out.push(format!("\"{name} {}", preview(text)));
            }
        }
        if out.is_empty() {
            Outcome::Info("All registers are empty".into())
        } else {
            Outcome::Info(out.join("   "))
        }
    }

    /// `q{reg}`: starts recording a macro.
    pub(super) fn start_recording(&mut self, reg: char) -> Result<Outcome, Parse> {
        if !reg.is_ascii_alphanumeric() && reg != '"' {
            return Err(Parse::Invalid);
        }
        self.recording = None;
        if reg.is_ascii_lowercase() || reg.is_ascii_digit() {
            self.registers.remove(&reg);
        }
        self.macro_rec = Some((reg, Vec::new()));
        Ok(Outcome::Ok)
    }

    /// `q` while recording: stores the keys typed (all but this `q`).
    pub(super) fn stop_recording(&mut self) {
        let Some((reg, mut keys)) = self.macro_rec.take() else {
            return;
        };
        keys.pop();
        let text: String = keys.iter().map(|k| k.raw()).collect();
        if reg == '"' {
            self.register = Register {
                text,
                linewise: false,
            };
        } else if reg.is_ascii_uppercase() {
            self.registers
                .entry(reg.to_ascii_lowercase())
                .or_default()
                .text
                .push_str(&text);
        } else {
            self.registers.insert(
                reg,
                Register {
                    text,
                    linewise: false,
                },
            );
        }
        self.recording = None;
    }

    /// `@{reg}` (`@@`: the last one again): replays a macro `n` times,
    /// stopping when it fails.
    pub(super) fn play_macro(&mut self, arg: char, n: usize) -> Result<Outcome, Parse> {
        let reg = if arg == '@' {
            self.last_macro.ok_or(Parse::Invalid)?
        } else {
            arg
        };
        if !Self::valid_register(reg) {
            return Err(Parse::Invalid);
        }
        self.recording = None;
        let text = self.register_text(reg).unwrap_or_default();
        if text.is_empty() {
            return Ok(Outcome::Bell(format!("Register {reg} is empty")));
        }
        if self.depth > 60 {
            return Ok(Outcome::Bell("Macro nested too deep".into()));
        }
        self.last_macro = Some(reg);
        self.pending.clear();
        let keys: Vec<Key> = text.chars().map(Key::from_raw).collect();
        for _ in 0..n {
            if !self.replay(&keys) {
                break;
            }
        }
        Ok(Outcome::Ok)
    }
}
