//! Replays every golf line, and checks the rules of play.

use super::*;
use crate::vim::{format_keys, parse_keys};

#[test]
fn every_solution_solves_its_level_at_par() {
    for (i, level) in LEVELS.iter().enumerate() {
        let mut s = Session::new(i);
        assert!(!s.complete, "level {i} {} starts complete", level.title);
        let keys = parse_keys(level.solution);
        for (k, &key) in keys.iter().enumerate() {
            assert!(
                !s.complete,
                "level {i} {} completes early after {}",
                level.title,
                format_keys(&keys[..k])
            );
            let event = s.press(key);
            assert!(
                !matches!(event, Event::Locked(_) | Event::Bell | Event::Trapped(_)),
                "level {i} {}: key {} ({k}) gave {event:?}: {}\nbuffer:\n{}",
                level.title,
                key,
                s.message,
                s.editor.text()
            );
        }
        assert!(
            s.complete,
            "level {i} {} not solved by {}: {} gems left, cursor {:?}, buffer:\n{}",
            level.title,
            level.solution,
            s.gems_left(),
            s.editor.cursor,
            s.editor.text()
        );
        assert!(
            s.missed.is_empty(),
            "level {i} {}: the golf line skips the lesson: {:?} (used {:?})",
            level.title,
            s.missed,
            s.used
        );
        assert_eq!(s.keys.len(), level.par());
        assert_eq!(s.stars(), 3);
    }
}

#[test]
fn marks_lie_on_the_map() {
    for level in LEVELS {
        let e = Editor::new(level.text());
        for p in level
            .gems()
            .into_iter()
            .chain(level.traps())
            .chain([level.start()])
        {
            assert!(
                p.row < e.lines.len() && (p.col < e.line_len(p.row) || p.col == 0),
                "{}: mark {p:?} is off the map",
                level.title
            );
        }
    }
}

#[test]
fn locked_keys_do_not_count() {
    let mut s = Session::new(0);
    assert_eq!(s.press(Key::Char('w')), Event::Locked('w'));
    assert_eq!(s.keys.len(), 0, "locked keys cost nothing");
    assert_eq!(s.press(Key::Char('l')), Event::Moved);
    assert_eq!(s.keys.len(), 1);
}

#[test]
fn game_commands_work_before_ex_is_learned_and_cost_nothing() {
    let mut s = Session::new(0);
    s.press(Key::Char('l'));
    let mut last = Event::None;
    for k in parse_keys(":e!<CR>") {
        last = s.press(k);
    }
    assert_eq!(last, Event::Command("e!".into()));
    assert_eq!(s.keys.len(), 1);
    // A real ex command is refused, and costs nothing.
    for k in parse_keys(":s/a/b/<CR>") {
        last = s.press(k);
    }
    assert_eq!(last, Event::Locked(':'));
    assert_eq!(s.keys.len(), 1);
    assert_eq!(s.editor.mode, Mode::Normal);
    // Once learned, ex keys count as in vim golf.
    let mut s = Session::new(crate::levels::ex_isle() - 1);
    for k in parse_keys(":%s/foo/baz/g<CR>") {
        s.press(k);
    }
    assert!(s.complete);
    assert_eq!(s.keys.len(), 14);
}

#[test]
fn a_wrong_edit_can_always_be_undone() {
    // Chomp comes before undo's own isle.
    let mut s = Session::new(LEVELS.iter().position(|l| l.title == "Chomp").unwrap());
    let before = s.editor.text();
    assert_eq!(s.press(Key::Char('x')), Event::Edited);
    assert_ne!(s.editor.text(), before);
    assert_eq!(s.press(Key::Char('u')), Event::Edited);
    assert_eq!(s.editor.text(), before);
    assert_eq!(s.press(Key::CtrlR), Event::Edited);
    assert_ne!(s.editor.text(), before);
}

#[test]
fn traps_throw_back_single_steps_but_not_jumps() {
    // Counting Sheep: traps every four tiles along the first row.
    let mut s = Session::new(1);
    let mut events = Vec::new();
    for _ in 0..5 {
        events.push(s.press(Key::Char('l')));
    }
    assert_eq!(events[3], Event::Trapped(Pos { row: 0, col: 4 }));
    assert_eq!(events[4], Event::Trapped(Pos { row: 0, col: 4 }));
    assert_eq!(s.editor.cursor, Pos { row: 0, col: 3 });
    assert_eq!(s.trapped, 2);
    // The keys still count.
    assert_eq!(s.keys.len(), 5);
    // A count jumps over the traps.
    let mut s = Session::new(1);
    for k in parse_keys("15l") {
        assert!(!matches!(s.press(k), Event::Trapped(_)));
    }
    assert_eq!(s.editor.cursor, Pos { row: 0, col: 15 });
    for level in LEVELS {
        for t in level.traps() {
            assert!(
                !level.gems().contains(&t) && level.start() != t,
                "{}: trap on a mark",
                level.title
            );
        }
    }
}

#[test]
fn the_coach_spots_slow_habits() {
    let mut s = Session::new(2); // Word Hopping: counts, w and e known.
    for k in parse_keys("hhhh") {
        s.press(k);
    }
    assert!(s.tip.contains("4h"), "{}", s.tip);
    assert_eq!(s.coach.spotted.len(), 1);
}

#[test]
fn reaching_the_goal_without_the_lesson_fails_the_isle() {
    // Search Party: three searches instead of / then n.
    let i = LEVELS.iter().position(|l| l.title == "Search Party").unwrap();
    let mut s = Session::new(i);
    assert_eq!(s.lesson_left(), vec!["/text", "n or N"]);
    let mut last = Event::None;
    for k in parse_keys("/g<CR>/g<CR>/g<CR>") {
        last = s.press(k);
    }
    assert_eq!(last, Event::Complete);
    assert!(s.complete && !s.cleared());
    assert_eq!(s.missed, vec!["n or N".to_string()]);
    // The golf line clears it.
    let mut s = Session::new(i);
    for k in parse_keys(LEVELS[i].solution) {
        s.press(k);
    }
    assert!(s.cleared());
}

#[test]
fn a_keystroke_budget_ends_the_isle() {
    let i = LEVELS
        .iter()
        .position(|l| l.title == "Edge of the World")
        .unwrap();
    let mut s = Session::new(i);
    let mut last = Event::None;
    for k in 0..10 {
        last = s.press(Key::Char(if k % 2 == 0 { 'l' } else { 'h' }));
    }
    assert_eq!(last, Event::Complete);
    assert!(s.complete && !s.cleared());
    assert_eq!(s.missed, vec!["finish within 10 keys".to_string()]);
}

fn isle(title: &str) -> usize {
    LEVELS.iter().position(|l| l.title == title).unwrap()
}

#[test]
fn crystals_fade_and_sentries_catch() {
    // Stepping is too slow for a fading crystal.
    let mut s = Session::new(isle("Fading Lights"));
    for k in parse_keys("ll") {
        s.press(k);
    }
    assert!(s.complete && !s.cleared(), "{:?}", s.missed);
    assert!(s.missed[0].contains("faded"));
    // Standing still where a sentry is going sends the wisp home.
    let mut s = Session::new(isle("Night Watch"));
    let mut last = Event::None;
    for k in parse_keys("3j6l7|") {
        last = s.press(k);
    }
    assert_eq!(last, Event::Caught(Pos { row: 3, col: 6 }));
    assert_eq!(
        (s.caught, s.editor.cursor),
        (1, LEVELS[isle("Night Watch")].start())
    );
}

#[test]
fn banned_keys_and_frozen_lines_hold() {
    let mut s = Session::new(isle("No Arrows"));
    assert_eq!(s.press(Key::Char('l')), Event::Locked('l'));
    assert_eq!(s.editor.cursor, Pos { row: 0, col: 0 });
    assert_eq!(s.keys.len(), 0, "locked keys cost nothing");
    let mut s = Session::new(isle("Thin Ice"));
    let before = s.editor.text();
    let mut last = Event::None;
    for k in parse_keys(":%s/teh/the/g<CR>") {
        last = s.press(k);
    }
    assert_eq!(last, Event::Bell);
    assert_eq!(s.editor.text(), before);
    assert!(s.message.contains("frozen"));
}

#[test]
fn free_play_allows_everything_and_asks_for_nothing() {
    let i = isle("Search Party");
    let mut s = Session::free_play(i);
    for k in parse_keys("/g<CR>/g<CR>/g<CR>") {
        s.press(k);
    }
    assert!(s.cleared(), "{:?}", s.missed);
    // Keys from much later isles work.
    let mut s = Session::free_play(0);
    assert_ne!(s.press(Key::Char('w')), Event::Locked('w'));
}

#[test]
fn star_thresholds() {
    assert_eq!(stars(10, 10), 3);
    assert_eq!(stars(15, 10), 2);
    assert_eq!(stars(16, 10), 1);
}
