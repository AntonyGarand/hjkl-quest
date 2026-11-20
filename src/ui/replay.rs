//! A finished run played back beside the golf solution, key by key: where
//! the two part ways is where the keystrokes went.

use nsh_core::prelude::*;

use crate::game::{Bug, Session};
use crate::ui::colors::{
    BUG, CREAM, GEM, GOLD, INK, LAVENDER, NIGHT, PANEL, SELECT, TEXT, WHITE, hex, hexa, mode_color,
};
use crate::ui::draw::is_selected;
use crate::ui::motion::Ui;
use crate::ui::state::{Screen, with_state};
use crate::ui::widgets::{MONO, chip, legend};
use crate::vim::{self, Editor, Key, Pos, format_keys};

/// The isle after some keys.
#[derive(Clone)]
struct Snap {
    editor: Editor,
    gems: Vec<(Pos, bool)>,
    bugs: Vec<Bug>,
}

pub(crate) struct Replay {
    /// The finished run, to go back to.
    pub(crate) session: Session,
    yours: (Vec<Key>, Vec<Snap>),
    golf: (Vec<Key>, Vec<Snap>),
    step: usize,
    playing: bool,
}

/// Plays `keys` on a fresh copy of the isle, keeping a snapshot per key.
fn play(session: &Session, keys: &[Key]) -> Vec<Snap> {
    let mut s = Session::custom(session.level(), session.index);
    let snap = |s: &Session| Snap {
        editor: s.editor.clone(),
        gems: s.gems.clone(),
        bugs: s.bugs.clone(),
    };
    let mut out = vec![snap(&s)];
    for &k in keys {
        s.press(k);
        out.push(snap(&s));
    }
    out
}

impl Replay {
    pub(crate) fn new(session: Session) -> Replay {
        let golf = vim::parse_keys(session.level().solution);
        let yours = session.keys.clone();
        Replay {
            yours: (yours.clone(), play(&session, &yours)),
            golf: (golf.clone(), play(&session, &golf)),
            session,
            step: 0,
            playing: true,
        }
    }

    fn last(&self) -> usize {
        self.yours.0.len().max(self.golf.0.len())
    }

    /// A key on the replay screen; true to leave it.
    pub(crate) fn key(&mut self, key: Key) -> bool {
        match key {
            Key::Char(' ' | 'p') => {
                if self.step >= self.last() {
                    self.step = 0;
                }
                self.playing = !self.playing;
            }
            Key::Char('l' | 'w') => {
                self.playing = false;
                self.step = (self.step + 1).min(self.last());
            }
            Key::Char('h' | 'b') => {
                self.playing = false;
                self.step = self.step.saturating_sub(1);
            }
            Key::Char('0' | 'g') => self.step = 0,
            Key::Char('$' | 'G') => self.step = self.last(),
            Key::Esc | Key::Enter | Key::Char('q') => return true,
            _ => {}
        }
        false
    }

    /// Steps along while playing; true when something moved.
    fn advance(&mut self) -> bool {
        if !self.playing {
            return false;
        }
        if self.step >= self.last() {
            self.playing = false;
        } else {
            self.step += 1;
        }
        true
    }
}

fn replay_value<R>(f: impl FnOnce(&Replay) -> R) -> Option<R> {
    with_state(|s| match &s.screen {
        Screen::Replay(r) => Some(f(r)),
        _ => None,
    })
}

/// One side: its keys (typed ones lit), and the text with the cursor,
/// crystals and bugs.
fn side(label: &str, keys: &[Key], snaps: &[Snap], step: usize, color: u32) -> impl IntoElement {
    let at = step.min(keys.len());
    let snap = &snaps[at];
    let done = step >= keys.len();
    let rows = snap
        .editor
        .lines
        .iter()
        .enumerate()
        .map(|(row, line)| snap_row(snap, row, line));
    view()
        .flex_1()
        .min_w(px(280.0))
        .gap_2()
        .p(px(14.0))
        .rounded(px(14.0))
        .bg(hexa(PANEL, 0.7))
        .border_1()
        .border_color(hexa(color, 0.4))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(label.to_string())
                        .text_size(px(12.0))
                        .letter_spacing(px(1.5))
                        .font_weight(FontWeight::BOLD)
                        .text_color(hex(color)),
                )
                .child(view().flex_1())
                .child(
                    text(format!(
                        "{at} / {} keys{}",
                        keys.len(),
                        if done { "  ✓ done" } else { "" }
                    ))
                    .font_family(MONO)
                    .text_size(px(13.0))
                    .text_color(hexa(TEXT, 0.7)),
                ),
        )
        .child(keys_line(keys, at))
        .child(
            view()
                .p(px(8.0))
                .rounded(px(8.0))
                .bg(hexa(NIGHT, 0.6))
                .children(rows),
        )
}

/// The keys, those typed so far lit.
fn keys_line(keys: &[Key], at: usize) -> impl IntoElement {
    view()
        .flex_row()
        .flex_wrap()
        .child(
            text(format_keys(&keys[..at]))
                .font_family(MONO)
                .text_size(px(14.0))
                .font_weight(FontWeight::BOLD)
                .text_color(hex(GOLD)),
        )
        .child(
            text(format_keys(&keys[at..]))
                .font_family(MONO)
                .text_size(px(14.0))
                .text_color(hexa(WHITE, 0.3)),
        )
}

/// A line of text at a snapshot, a cell per character.
fn snap_row(snap: &Snap, row: usize, line: &[char]) -> View {
    let e = &snap.editor;
    let width = line.len().max(1) + usize::from(row == e.cursor.row && e.cursor.col >= line.len());
    view()
        .flex_row()
        .children((0..width).map(|col| snap_cell(snap, Pos { row, col }, line)))
}

/// A character: a bug ✱ or a crystal ◆ over it, the cursor on it, lit
/// when selected.
fn snap_cell(snap: &Snap, p: Pos, line: &[char]) -> View {
    let e = &snap.editor;
    let c = line.get(p.col).copied().unwrap_or(' ');
    let gem = snap.gems.iter().any(|&(g, got)| g == p && !got);
    let bug = snap.bugs.iter().any(|b| b.at == p);
    let cursor = p == e.cursor;
    let (shown, color) = if cursor {
        (if c == ' ' { '\u{a0}' } else { c }, INK)
    } else if bug {
        ('✱', BUG)
    } else if gem {
        ('◆', GEM)
    } else {
        (if c == ' ' { '\u{a0}' } else { c }, TEXT)
    };
    let cell = view().child(
        text(shown.to_string())
            .font_family(MONO)
            .text_size(px(15.0))
            .line_height(px(21.0))
            .text_color(hex(color)),
    );
    if cursor {
        cell.bg(hex(mode_color(e))).rounded(px(2.0))
    } else if is_selected(e, p) {
        cell.bg(hexa(SELECT, 0.75))
    } else {
        cell
    }
}

pub(crate) fn replay_screen(ui: Ui) -> impl IntoElement {
    // Steps along with the beat while playing.
    effect(move || {
        ui.beat.get();
        if with_state(|s| match &mut s.screen {
            Screen::Replay(r) => r.advance(),
            _ => false,
        }) {
            ui.bump_tick();
        }
    });
    let title = replay_value(|r| r.session.level().title).unwrap_or_default();
    view()
        .flex_1()
        .p(px(22.0))
        .gap_3()
        .child(
            view()
                .gap_1()
                .child(
                    text("REPLAY")
                        .text_size(px(12.0))
                        .letter_spacing(px(2.0))
                        .text_color(hexa(LAVENDER, 0.75)),
                )
                .child(
                    text(title)
                        .text_size(px(26.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(hex(CREAM)),
                ),
        )
        .child(dynamic(move || {
            ui.tick.get();
            replay_value(|r| {
                view()
                    .flex_row()
                    .flex_wrap()
                    .gap_4()
                    .child(side("YOUR RUN", &r.yours.0, &r.yours.1, r.step, CREAM))
                    .child(side("GOLF LINE", &r.golf.0, &r.golf.1, r.step, GEM))
            })
        }))
        .child(dynamic(move || {
            ui.tick.get();
            replay_value(notes)
        }))
        .child(view().flex_1())
        .child(controls(ui))
}

/// How the run compares with the golf line, and the habits it showed.
fn notes(r: &Replay) -> View {
    let (y, g) = (r.yours.0.len(), r.golf.0.len());
    let note = if y <= g {
        format!("You matched the golf line: {y} keys against {g}.")
    } else {
        format!(
            "{} keys over par. Watch where the golf line moves further per key.",
            y - g
        )
    };
    let habits: Vec<String> = r
        .session
        .coach
        .spotted
        .iter()
        .map(|h| format!("• {}: {}", h.label, h.tip))
        .collect();
    view()
        .gap_1()
        .child(text(note).text_size(px(14.0)).text_color(hexa(TEXT, 0.85)))
        .children(habits.into_iter().take(4).map(|h| {
            text(h)
                .text_size(px(13.0))
                .line_height(px(18.0))
                .text_color(hex(GOLD))
        }))
}

/// The replay's keys.
fn controls(ui: Ui) -> impl IntoElement {
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(chip("Space", CREAM))
        .child(
            text(move || {
                ui.tick.get();
                if replay_value(|r| r.playing).unwrap_or(false) {
                    "pause"
                } else {
                    "play"
                }
            })
            .text_color(hexa(WHITE, 0.6))
            .mr(px(10.0)),
        )
        .child(chip("h l", CREAM))
        .child(legend("step back / forward").mr(px(10.0)))
        .child(chip("0 $", CREAM))
        .child(legend("start / end").mr(px(10.0)))
        .child(chip("q", CREAM))
        .child(legend("back"))
}
