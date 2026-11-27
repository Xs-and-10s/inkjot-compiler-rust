//! Ex 7, played. The draft keeps the Detective's findings in a knowledge
//! base (`knowledge findings`, `assert {{ target guilty }} in findings`),
//! which is milestone 22's; nothing in the game reads them back. Until
//! then this plays the draft with those lines set aside: it must check
//! clean but for them, and its own tests must pass.

use inkjot::run::test::run_all;

fn draft() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    std::fs::read_to_string(format!("{dir}/examples/drafts/ex7-mafia.twee")).unwrap()
}

/// The draft without its Inklog: the two `knowledge` lines, and the
/// `assert` effects on the Detective's Link.
fn playable() -> String {
    draft()
        .lines()
        .filter(|l| !l.starts_with("knowledge "))
        .map(|l| {
            l.replace(
                " ~ then assert {{ target guilty }} in findings when target.team != town",
                "",
            )
            .replace(
                " ~ then assert {{ target innocent }} in findings when target.team == town",
                "",
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn errors(src: &str) -> Vec<String> {
    inkjot::check(src)
        .diags
        .into_iter()
        .filter(|d| d.severity == inkjot::Severity::Error)
        .map(|d| format!("{}:{} {}", d.line, d.col, d.msg))
        .collect()
}

#[test]
fn only_inklog_is_left_to_build() {
    let left: Vec<String> = errors(&draft())
        .into_iter()
        .filter(|e| {
            !e.contains("`knowledge` is reserved") && !e.contains("`assert` is not an effect")
        })
        .collect();
    assert_eq!(left, Vec::<String>::new());
    assert!(
        !playable().contains("assert {{"),
        "the Inklog was not all set aside"
    );
    assert_eq!(errors(&playable()), Vec::<String>::new());
}

#[test]
fn ex7s_own_tests_pass() {
    let src = playable();
    let ir = inkjot::build(&src).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    for o in run_all(&src, &ir).unwrap() {
        assert!(
            o.passed(),
            "`{}`: {:#?}\n{}",
            o.name,
            o.failures,
            o.transcript.join("\n")
        );
    }
}

use inkjot::run::{Run, Story};
use proptest::prelude::*;
use std::sync::Arc;

/// One random game of `n`, `seed` deciding the cards and `picks` every
/// choice. The passages the table reached, in order; `None` if it is still
/// going after `limit` actions.
fn random_game(n: usize, seed: u64, picks: &[u16], limit: usize) -> Result<String, String> {
    let src = playable();
    let ir = inkjot::build(&src).map_err(|r| format!("{:?}", r.diags))?;
    let mut run = Run::new(Arc::new(Story::new(&ir)?), seed);
    let who = vec!["guest"; n];
    run.start_cohort(None, &who)?;
    // The picks seed a stream of choices that never repeats in step with
    // the table (cycling through them could pass a seat by for ever).
    let mut state = picks.iter().fold(0x9E37_79B9_7F4A_7C15u64, |h, p| {
        (h ^ u64::from(*p)).wrapping_mul(0x1000_0000_01B3)
    }) | 1;
    let mut next = |m: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % m.max(1) as u64) as usize
    };
    for _ in 0..limit {
        let seats = run.seated().to_vec();
        let seat = seats[next(seats.len())].clone();
        run.sit(&seat)?;
        let tags = run.story().passage_tags(run.here()).to_vec();
        if tags.iter().any(|t| t == "finale" || t == "epilogue") {
            return Ok(run.here().to_string());
        }
        let asks = run.asks()?;
        let choices = run.choices()?;
        // Answer what is asked now and then; else take a Link; else let
        // time pass, for the Links that wait for it.
        if let Some(ask) = asks.iter().find(|a| !a.voted && next(3) == 0) {
            let done = match ask.kind.as_str() {
                "accuse" => {
                    let who = &ask.options[next(ask.options.len())];
                    run.vote(who, "", &[])
                }
                "name input" => run.answer(&[format!("Name{}", next(100))]),
                "text input" => run.answer(&["A report.".to_string()]),
                _ => {
                    let pick = &ask.options[next(ask.options.len())];
                    run.answer(std::slice::from_ref(pick))
                }
            };
            // A seat with no vote is refused; that is not a failure.
            if let Err(e) = done {
                if !e.contains("has no vote") && !e.contains("over") {
                    return Err(e);
                }
            }
        } else if !choices.is_empty() && next(8) != 0 {
            let (id, _) = &choices[next(choices.len())];
            if let Err(e) = run.choose(id) {
                if !e.contains("taken by itself") {
                    return Err(e);
                }
            }
        } else {
            run.wait(60_000)?;
        }
    }
    let mut why = String::new();
    for e in [
        "accusation.done",
        "count(guest, where: it.alive and not it.voteless)",
        "clock.now",
        "syndicate",
        "moderator",
    ] {
        let v = run
            .eval(&inkjot::ir::lower_expression(e))
            .map(|v| v.to_string())
            .unwrap_or_default();
        why.push_str(&format!(" {e}={v};"));
    }
    for s in run.seated().to_vec() {
        run.sit(&s)?;
        let asks: Vec<String> = run
            .asks()?
            .iter()
            .map(|a| format!("{}:{}", a.kind, a.voted))
            .collect();
        let links = run.choices()?;
        why.push_str(&format!(
            " [{s} at {} asks {:?} links {:?}]",
            run.here(),
            asks,
            links
        ));
    }
    Err(format!(
        "no ending after {limit} actions, at `{}`:{why}",
        run.here()
    ))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Whatever anyone does, every game ends at a finale.
    #[test]
    fn every_game_ends(n in 5usize..=8, seed in 1u64..10_000, picks in prop::collection::vec(any::<u16>(), 50..200)) {
        match random_game(n, seed, &picks, 3_000) {
            Ok(end) => prop_assert!(end.ends_with("_wins") || end == "reveal", "{}", end),
            Err(e) => prop_assert!(false, "{}", e),
        }
    }
}
