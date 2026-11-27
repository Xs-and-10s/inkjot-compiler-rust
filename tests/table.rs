//! Milestone 21e: the engine seats a table. Ex 7, through the engine as the
//! platform will call it: seats, a view for each, actions done as a seat,
//! and a log that replays.

use inkjot::engine::{Action, Inklink, RefusedKind, Start, State};
use proptest::prelude::*;

fn ex7() -> Inklink {
    let dir = env!("CARGO_MANIFEST_DIR");
    let src = std::fs::read_to_string(format!("{dir}/examples/drafts/ex7-mafia.twee")).unwrap();
    let ir = inkjot::build(&src).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    Inklink::from_ir(&ir).unwrap()
}

fn table(n: usize) -> Start {
    Start {
        seats: vec!["guest".to_string(); n],
        ..Start::default()
    }
}

fn link(s: &State, seat: &str, label: &str) -> String {
    let view = s.view_of(seat).unwrap();
    view.links
        .iter()
        .find(|l| l.label.plain() == label)
        .unwrap_or_else(|| panic!("no Link `{label}` for {seat}: {:?}", view.links))
        .id
        .clone()
}

fn take(s: &State, seat: &str, label: &str) -> State {
    s.act_as(seat, &Action::TakeLink(link(s, seat, label)))
        .unwrap()
}

#[test]
fn a_table_is_seated_and_each_seat_sees_its_own() {
    let s = ex7().start(1, &table(5)).unwrap();
    assert_eq!(
        s.seats(),
        ["guest_1", "guest_2", "guest_3", "guest_4", "guest_5"]
    );
    for seat in s.seats() {
        let v = s.view_of(seat).unwrap();
        assert_eq!(v.nib, "gathering");
        assert_eq!(v.pov, *seat);
    }
    assert_eq!(s.view().pov, "guest_1");
    assert_eq!(ex7().meta().cohort, Some((5, Some(12))));
}

#[test]
fn what_cannot_be_done_is_refused() {
    let inklink = ex7();
    let small = inklink.start(1, &table(3)).unwrap_err();
    assert_eq!(small.kind, RefusedKind::StoryError);
    assert!(small.message.contains("a table of 3"), "{}", small.message);
    let stranger = Start {
        seats: vec!["lamplighter".into(); 5],
        ..Start::default()
    };
    assert!(inklink.start(1, &stranger).is_err());
    let s = inklink.start(1, &table(5)).unwrap();
    let id = link(&s, "guest_1", "Ask for a special syndicate");
    // At a table, an action is someone's.
    let anon = s.act(&Action::TakeLink(id.clone())).unwrap_err();
    assert_eq!(anon.kind, RefusedKind::NotOffered);
    let nobody = s.act_as("guest_9", &Action::TakeLink(id)).unwrap_err();
    assert_eq!(nobody.kind, RefusedKind::NotOffered);
}

#[test]
fn a_link_for_the_table_moves_everyone() {
    let s = ex7().start(1, &table(5)).unwrap();
    let s = s
        .act_as("guest_2", &Action::Answer(vec!["Bea".into()]))
        .unwrap();
    let s = take(&s, "guest_3", "Draw the syndicate");
    assert_eq!(s.view().pov, "guest_3");
    for seat in s.seats() {
        assert_eq!(s.view_of(seat).unwrap().nib, "drawn", "{seat}");
        assert!(!s.view_of(seat).unwrap().text.is_empty());
    }
    let s = take(&s, "guest_1", "Deal the cards");
    // Each sees their own card.
    let cards: Vec<String> = s
        .seats()
        .iter()
        .map(|seat| {
            s.view_of(seat)
                .unwrap()
                .text
                .iter()
                .map(|p| p.plain())
                .find(|t| t.starts_with("You are"))
                .unwrap_or_default()
        })
        .collect();
    assert!(cards.iter().all(|c| !c.is_empty()), "{cards:?}");
    assert!(s.view_of("guest_2").unwrap().text[0]
        .plain()
        .starts_with("Bea,"));
}

/// Play `picks` at a table of `n`: each pick is a seat and something it
/// may do. The states, from the start.
fn play(n: usize, seed: u64, picks: &[(u8, u8)]) -> Vec<State> {
    let inklink = ex7();
    let mut states = vec![inklink.start(seed, &table(n)).unwrap()];
    for (who, what) in picks {
        let s = states.last().unwrap();
        let seat = s.seats()[*who as usize % n].clone();
        let v = s.view_of(&seat).unwrap();
        let open: Vec<&String> = v
            .links
            .iter()
            .filter(|l| l.due_in.is_none())
            .map(|l| &l.id)
            .collect();
        let action = if let Some(ask) = v.asks.iter().find(|a| !a.voted && *what % 3 == 0) {
            match ask.kind.as_str() {
                "accuse" => Action::Vote {
                    who: ask.options[*what as usize % ask.options.len()].clone(),
                    how: String::new(),
                    proof: Vec::new(),
                },
                "name input" | "text input" => Action::Answer(vec![format!("N{what}")]),
                _ => Action::Answer(vec![ask.options[*what as usize % ask.options.len()].clone()]),
            }
        } else if !open.is_empty() && *what % 7 != 0 {
            Action::TakeLink(open[*what as usize % open.len()].clone())
        } else {
            let next = s.wait(60_000).unwrap();
            states.push(next);
            continue;
        };
        // A seat that may not do it is refused; the state stands.
        if let Ok(next) = s.act_as(&seat, &action) {
            states.push(next);
        }
    }
    states
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    /// A table's log replays to the same table: every seat sees the same,
    /// and the engine reports the same.
    #[test]
    fn a_tables_log_replays(n in 5usize..=7, seed in 1u64..1000, picks in prop::collection::vec((any::<u8>(), any::<u8>()), 1..60)) {
        let states = play(n, seed, &picks);
        let last = states.last().unwrap();
        let again = ex7().replay(seed, &table(n), last.actions(), None).unwrap();
        prop_assert_eq!(again.view(), last.view());
        for seat in last.seats() {
            prop_assert_eq!(again.view_of(seat), last.view_of(seat));
        }
        prop_assert_eq!(again.events(), last.events());
    }
}
