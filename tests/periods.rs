//! Periods: names to turns and back (`src/period.rs`).

use inkjot::period::{Calendar, Each, Period};
use proptest::prelude::*;

fn cal(src: &str) -> Calendar {
    Calendar::parse(src.lines()).unwrap()
}

const WEEK: &str = "period watch: morning 1, afternoon 2, evening 1, night 1
period day: monday, tuesday, wednesday, thursday, friday, saturday ~ of watch";

#[test]
fn ex6s_week_is_thirty_turns() {
    let c = cal(WEEK);
    assert_eq!(c.round("watch"), Some(5));
    assert_eq!(c.round("day"), Some(30));
    assert_eq!(c.clock("monday.morning ..= saturday.night"), Some(0..30));
    assert_eq!(c.span("monday.evening"), Some(3..4));
    assert_eq!(c.span("tuesday.afternoon"), Some(6..8));
    assert_eq!(c.span("saturday"), Some(25..30));
    assert_eq!(
        c.at(7),
        [
            ("day".to_string(), "tuesday".to_string()),
            ("watch".to_string(), "afternoon".to_string())
        ]
    );
}

#[test]
fn the_public_cuts_evening_is_eight_turns() {
    let c = cal("period hour: eight 1, nine 2, ten 2, eleven 2, midnight 1
period evening: new_years_eve ~ of hour");
    assert_eq!(
        c.clock("new_years_eve.eight ..= new_years_eve.midnight"),
        Some(0..8)
    );
    assert_eq!(c.span("new_years_eve.ten"), Some(3..5));
}

#[test]
fn seasons_of_equal_length() {
    let c = cal("period season: spring, summer, autumn ~ 3 each");
    assert_eq!(c.round("season"), Some(9));
    assert_eq!(c.span("summer"), Some(3..6));
}

#[test]
fn mistakes_are_refused() {
    for (src, says) in [
        ("period day: monday ~ of week", "not a period"),
        ("period a: x ~ of b\nperiod b: y ~ of a", "made of itself"),
        ("period watch: morning 1, afternoon", "needs a length"),
        ("period watch: morning 0", "1 or more"),
        ("period watch: morning 1 ~ 2 each", "lengths twice"),
        ("period day: monday\nperiod day: tuesday", "declared twice"),
        ("period day:", "names nothing"),
    ] {
        let err = Calendar::parse(src.lines()).unwrap_err();
        assert!(err.contains(says), "{src}: {err}");
    }
}

#[test]
fn a_path_that_names_nothing_has_no_span() {
    let c = cal(WEEK);
    assert_eq!(c.span("sunday"), None);
    assert_eq!(c.span("monday.dawn"), None);
    assert_eq!(c.span("monday.morning.early"), None);
    assert_eq!(c.clock("saturday ..= monday"), None);
}

/// A random calendar: an inner period with its own lengths, and an outer
/// one made of it.
fn calendar() -> impl Strategy<Value = Calendar> {
    (
        prop::collection::vec(1..=4u32, 1..=5),
        1..=6usize,
        any::<bool>(),
    )
        .prop_map(|(lens, outer, equal)| {
            let inner_names: Vec<String> = (0..lens.len()).map(|i| format!("i{i}")).collect();
            let each = if equal {
                Each::Turns(lens[0])
            } else {
                Each::Own(lens)
            };
            Calendar {
                periods: vec![
                    Period {
                        name: "inner".into(),
                        names: inner_names,
                        each,
                    },
                    Period {
                        name: "outer".into(),
                        names: (0..outer).map(|i| format!("o{i}")).collect(),
                        each: Each::Of("inner".into()),
                    },
                ],
            }
        })
}

/// The source a calendar is written as.
fn source(c: &Calendar) -> String {
    c.periods
        .iter()
        .map(|p| {
            let names = match &p.each {
                Each::Own(lens) => p
                    .names
                    .iter()
                    .zip(lens)
                    .map(|(n, l)| format!("{n} {l}"))
                    .collect::<Vec<_>>()
                    .join(", "),
                _ => p.names.join(", "),
            };
            let tail = match &p.each {
                Each::Own(_) => String::new(),
                Each::Turns(n) => format!(" ~ {n} each"),
                Each::Of(u) => format!(" ~ of {u}"),
            };
            format!("period {}: {names}{tail}", p.name)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

proptest! {
    /// Every turn falls in exactly one name of each period, and the span of
    /// that path holds the turn.
    #[test]
    fn every_turn_has_one_name_and_its_span_holds_it(c in calendar()) {
        let total = c.round("outer").unwrap();
        for t in 0..total {
            let path: Vec<String> = c.at(t).into_iter().map(|(_, n)| n).collect();
            prop_assert_eq!(path.len(), 2);
            let span = c.span(&path.join(".")).unwrap();
            prop_assert!(span.contains(&t), "{} not in {:?}", t, span);
        }
    }

    /// The names of the outer period cover the clock end to end, in order,
    /// without gaps or overlaps; so do the inner names within each.
    #[test]
    fn spans_tile_the_clock(c in calendar()) {
        let outer = c.get("outer").unwrap().clone();
        let inner = c.get("inner").unwrap().clone();
        let mut at = 0;
        for o in &outer.names {
            let s = c.span(o).unwrap();
            prop_assert_eq!(s.start, at);
            let mut inner_at = s.start;
            for i in &inner.names {
                let t = c.span(&format!("{o}.{i}")).unwrap();
                prop_assert_eq!(t.start, inner_at);
                inner_at = t.end;
            }
            prop_assert_eq!(inner_at, s.end);
            at = s.end;
        }
        prop_assert_eq!(at, c.round("outer").unwrap());
    }

    /// A clock from the first time to the last runs every turn.
    #[test]
    fn the_whole_clock(c in calendar()) {
        let outer = c.get("outer").unwrap();
        let inner = c.get("inner").unwrap();
        let first = format!("{}.{}", outer.names[0], inner.names[0]);
        let last = format!("{}.{}", outer.names.last().unwrap(), inner.names.last().unwrap());
        prop_assert_eq!(c.clock(&format!("{first} ..= {last}")), Some(0..c.round("outer").unwrap()));
    }

    /// Written out and read back, a calendar is the same calendar.
    #[test]
    fn source_round_trips(c in calendar()) {
        let src = source(&c);
        prop_assert_eq!(Calendar::parse(src.lines()).unwrap(), c);
    }

    /// Going round: a turn past the end names what the same turn of the
    /// first round named.
    #[test]
    fn the_clock_goes_round(c in calendar(), t in 0..200u32) {
        let round = c.round("outer").unwrap();
        prop_assert_eq!(c.at(t), c.at(t % round));
    }

    /// A name begins on the first turn of its span, and only then: every
    /// path's span starts with a beginning, and every beginning is the
    /// start of the span of where the turn is.
    #[test]
    fn a_name_begins_where_its_span_starts(c in calendar(), t in 0..200u32) {
        let round = c.round("outer").unwrap();
        let path: Vec<String> = c.at(t).into_iter().map(|(_, n)| n).collect();
        let mut expected = Vec::new();
        for (depth, named) in c.at(t).into_iter().enumerate() {
            let span = c.span(&path[..=depth].join(".")).unwrap();
            let starts = t % round == span.start;
            prop_assert_eq!(c.begins_on(&path[..=depth].join("."), t), starts);
            if starts {
                expected.push(named);
            }
        }
        prop_assert_eq!(c.begins(t), expected);
    }

    /// `wait until` a name finds the first turn, from now on, that it
    /// begins on, and the name is the one it is then.
    #[test]
    fn the_next_start_is_the_first(c in calendar(), from in 0..60u32, which in 0..5usize) {
        let inner = c.get("inner").unwrap().clone();
        let name = &inner.names[which % inner.names.len()];
        let next = c.next_start(name, from).unwrap();
        prop_assert!(next >= from);
        prop_assert!(c.begins_on(name, next));
        prop_assert!(c.at(next).iter().any(|(_, n)| n == name));
        prop_assert!((from..next).all(|t| !c.begins_on(name, t)));
    }
}

#[test]
fn beginnings_and_the_next_start() {
    let c = cal(WEEK);
    let pair = |p: &str, n: &str| (p.to_string(), n.to_string());
    assert_eq!(
        c.begins(5),
        [pair("day", "tuesday"), pair("watch", "morning")]
    );
    assert_eq!(c.begins(6), [pair("watch", "afternoon")]);
    assert_eq!(c.begins(7), []);
    assert_eq!(c.next_start("evening", 0), Some(3));
    assert_eq!(c.next_start("evening", 4), Some(8));
    assert_eq!(c.next_start("tuesday.evening", 4), Some(8));
    assert_eq!(c.next_start("saturday", 1), Some(25));
    assert_eq!(c.next_start("monday", 1), Some(30), "round again");
    assert_eq!(c.next_start("sunday", 0), None);
    assert!(c.is_time("tuesday.evening") && c.is_time("evening"));
    assert!(!c.is_time("evening.tuesday") && !c.is_time("dusk"));
    assert_eq!(c.index("night"), Some(3));
}
