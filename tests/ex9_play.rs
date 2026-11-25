//! Ex 9, *Saltmarrow*, played by the interpreter. The encounter with the
//! Warden is milestone 20d, so the draft does not check clean yet: it is
//! lowered without its check, and play stops at the encounter.

use inkjot::engine::{Inklink, Para, Start, State};
use proptest::prelude::*;

fn draft() -> String {
    std::fs::read_to_string("examples/drafts/ex9-saltmarrow.twee").unwrap()
}

fn start(seed: u64) -> State {
    let ir = inkjot::ir::lower_unchecked(&draft());
    Inklink::from_ir(&ir)
        .unwrap()
        .start(seed, &Start::default())
        .unwrap()
}

/// What play may run into before 20d: the encounter and its words (the
/// ward's `cover`), and nothing else.
fn not_yet(message: &str) -> bool {
    message.contains("encounter")
        || message.contains("not supported")
        || message.contains("`cover`")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Ex 9 at random, up to 120 Links: every Link it offers can be taken,
    /// short of the encounter.
    #[test]
    fn ex9_plays_at_random(seed in 1u64..10_000, moves in prop::collection::vec(0usize..64, 1..120)) {
        let mut s = start(seed);
        for m in moves {
            if s.view().over || s.view().links.is_empty() {
                break;
            }
            let links = s.view().links.clone();
            let id = links[m % links.len()].id.clone();
            match s.take_link(&id) {
                Ok(next) => s = next,
                Err(e) if not_yet(&e.message) => break,
                Err(e) => prop_assert!(false, "taking `{}` at `{}`: {}", id, s.view().nib, e.message),
            }
        }
    }
}

/// How far random play gets, and why it stops: `cargo test --test
/// ex9_play survey -- --ignored --nocapture`.
#[test]
#[ignore]
fn survey() {
    use std::collections::BTreeMap;
    let mut stops: BTreeMap<String, usize> = BTreeMap::new();
    let mut nibs: BTreeMap<String, usize> = BTreeMap::new();
    let mut rng = 7u64;
    for seed in 1..300 {
        let mut s = start(seed);
        let mut why = "ran out".to_string();
        for _ in 0..300 {
            if s.view().over || s.view().links.is_empty() {
                why = format!("ended at {}", s.view().nib);
                break;
            }
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let links = s.view().links.clone();
            let id = links[(rng % links.len() as u64) as usize].id.clone();
            match s.take_link(&id) {
                Ok(n) => s = n,
                Err(e) => {
                    why = e.message.chars().take(70).collect();
                    break;
                }
            }
            *nibs.entry(s.view().nib.clone()).or_default() += 1;
        }
        *stops.entry(why).or_default() += 1;
    }
    eprintln!("{stops:#?}\n{nibs:?}");
}

/// Take the Link with this id, or say what there was instead.
fn go(s: &State, id: &str) -> State {
    if !s.view().links.iter().any(|l| l.id == id) {
        let ids: Vec<&str> = s.view().links.iter().map(|l| l.id.as_str()).collect();
        panic!("no Link `{id}` at `{}`; there are {ids:?}", s.view().nib);
    }
    s.take_link(id)
        .unwrap_or_else(|e| panic!("`{id}`: {}", e.message))
}

/// Down for Thwaite with a stretcher made in the timber yard, and up again
/// with her, leaving the shard: the way that needs no encounter.
#[test]
fn ex9_can_be_won_without_a_fight() {
    let mut s = start(1);
    s = go(&s, "prologue->pithead");
    for id in ["walk:ladderway", "walk:gallery", "walk:timber_yard"] {
        s = go(&s, id);
    }
    for id in ["take:stave", "take:stave", "take:sacking", "make:stretcher"] {
        s = go(&s, id);
    }
    let made: Vec<String> = s.view().text.iter().map(Para::plain).collect();
    assert_eq!(
        made,
        ["Two staves and a sack, for carrying someone who cannot walk."]
    );
    for id in [
        "walk:gallery",
        "walk:incline",
        "walk:drift",
        "walk:cribbing",
        "walk:wardens_hall",
        "walk:marrow",
        "marrow->marrow:found",
        "marrow->marrow:take_her",
    ] {
        s = go(&s, id);
    }
    // Thwaite follows, up through the winze and the ladderway.
    for id in [
        "walk:wardens_hall",
        "walk:cribbing",
        "walk:drift",
        "walk:incline",
        "walk:gallery",
        "walk:ladderway",
        "walk:pithead",
        "pithead->home",
    ] {
        s = go(&s, id);
    }
    assert_eq!(s.view().nib, "home");
    let text: Vec<String> = s.view().text.iter().map(Para::plain).collect();
    // Finding her 10, home with her 20, the Warden left whole 10.
    assert!(
        text.contains(&"Your score is 40 of 100, which makes you a crust scraper.".to_string()),
        "{text:?}"
    );
}
