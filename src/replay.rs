//! A finished run played back beside the golf solution, key by key: where
//! the two part ways is where the keystrokes went.

use super::{
    Accessible, BUG, FluentBuilder, FontWeight, GEM, IntoElement, Key, MONO, ParentElement, Pos, SELECT,
    Screen, Session, Styled, Ui, chip, dynamic, effect, format_keys, game, hex, hexa, is_selected,
    mode_color, px, text, view, vim, with_state,
};
use game::Bug;

/// The isle after some keys.
#[derive(Clone)]
struct Snap {
    editor: vim::Editor,
    gems: Vec<(Pos, bool)>,
    bugs: Vec<Bug>,
}

pub(crate) struct Replay {
    /// The finished run, to go back to.
    pub session: Session,
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
    let e = &snap.editor;
    let done = step >= keys.len();
    let rows = e.lines.iter().enumerate().map(|(row, line)| {
        let width = line.len().max(1) + usize::from(row == e.cursor.row && e.cursor.col >= line.len());
        view().flex_row().children((0..width).map(move |col| {
            let p = Pos { row, col };
            let c = line.get(col).copied().unwrap_or(' ');
            let gem = snap.gems.iter().any(|&(g, got)| g == p && !got);
            let bug = snap.bugs.iter().any(|b| b.at == p);
            let cursor = p == e.cursor;
            let shown = if bug && !cursor {
                '✱'
            } else if gem && !cursor {
                '◆'
            } else if c == ' ' {
                '\u{a0}'
            } else {
                c
            };
            let glyph = text(shown.to_string())
                .font_family(MONO)
                .text_size(px(15.0))
                .line_height(px(21.0))
                .text_color(if cursor {
                    hex(0x1a1430)
                } else if bug {
                    hex(BUG)
                } else if gem {
                    hex(GEM)
                } else {
                    hex(0xece6ff)
                });
            let cell = view().child(glyph);
            if cursor {
                cell.bg(hex(mode_color(e))).rounded(px(2.0))
            } else if is_selected(e, p) {
                cell.bg(hexa(SELECT, 0.75))
            } else {
                cell
            }
        }))
    });
    view()
        .flex_1()
        .min_w(px(280.0))
        .gap_2()
        .p(px(14.0))
        .rounded(px(14.0))
        .bg(hexa(0x0b0820, 0.7))
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
                    .text_color(hexa(0xece6ff, 0.7)),
                ),
        )
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .child(
                    text(format_keys(&keys[..at]))
                        .font_family(MONO)
                        .text_size(px(14.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(hex(0xffd76b)),
                )
                .child(
                    text(format_keys(&keys[at..]))
                        .font_family(MONO)
                        .text_size(px(14.0))
                        .text_color(hexa(0xffffff, 0.3)),
                ),
        )
        .child(
            view()
                .p(px(8.0))
                .rounded(px(8.0))
                .bg(hexa(0x05030f, 0.6))
                .children(rows),
        )
}

pub(crate) fn replay_screen(ui: Ui) -> impl IntoElement {
    // Steps along with the beat while playing.
    effect(move || {
        ui.beat.get();
        if with_state(|s| match &mut s.screen {
            Screen::Replay(r) => r.advance(),
            _ => false,
        }) {
            ui.tick.update(|t| *t += 1);
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
                        .text_color(hexa(0xc9b8ff, 0.75)),
                )
                .child(
                    text(title)
                        .text_size(px(26.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(hex(0xfff3b0)),
                ),
        )
        .child(dynamic(move || {
            ui.tick.get();
            replay_value(|r| {
                view()
                    .flex_row()
                    .flex_wrap()
                    .gap_4()
                    .child(side("YOUR RUN", &r.yours.0, &r.yours.1, r.step, 0xfff3b0))
                    .child(side("GOLF LINE", &r.golf.0, &r.golf.1, r.step, GEM))
            })
        }))
        .child(dynamic(move || {
            ui.tick.get();
            replay_value(|r| {
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
                    .child(text(note).text_size(px(14.0)).text_color(hexa(0xece6ff, 0.85)))
                    .children(habits.into_iter().take(4).map(|h| {
                        text(h)
                            .text_size(px(13.0))
                            .line_height(px(18.0))
                            .text_color(hex(0xffd76b))
                    }))
            })
        }))
        .child(view().flex_1())
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(chip("Space", 0xfff3b0))
                .child(
                    text(move || {
                        ui.tick.get();
                        if replay_value(|r| r.playing).unwrap_or(false) {
                            "pause"
                        } else {
                            "play"
                        }
                    })
                    .text_color(hexa(0xffffff, 0.6))
                    .mr(px(10.0)),
                )
                .child(chip("h l", 0xfff3b0))
                .child(
                    text("step back / forward")
                        .text_color(hexa(0xffffff, 0.6))
                        .mr(px(10.0)),
                )
                .child(chip("0 $", 0xfff3b0))
                .child(text("start / end").text_color(hexa(0xffffff, 0.6)).mr(px(10.0)))
                .child(chip("q", 0xfff3b0))
                .child(text("back").text_color(hexa(0xffffff, 0.6))),
        )
}
