//! Ex 5, *Thistledown*, played: its own tests, and random years, every
//! one of which must end: at the Fair, or in the thistle.

use inkjot::run::test::run_all;

fn draft() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    std::fs::read_to_string(format!("{dir}/examples/drafts/ex5-thistledown.twee")).unwrap()
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
fn it_checks_clean() {
    assert_eq!(errors(&draft()), Vec::<String>::new());
}

#[test]
fn ex5s_own_tests_pass() {
    let src = draft();
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

/// One random year, `seed` deciding the dice and `picks` every choice.
/// The ending it reached; an error if it is stuck, or still going after
/// `limit` actions.
fn random_game(seed: u64, picks: &[u16], limit: usize) -> Result<String, String> {
    let src = draft();
    let ir = inkjot::build(&src).map_err(|r| format!("{:?}", r.diags))?;
    let mut run = Run::new(Arc::new(Story::new(&ir)?), seed);
    run.start(None, None)?;
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
        let tags = run.story().passage_tags(run.here()).to_vec();
        if tags.iter().any(|t| t == "ending") {
            return Ok(run.here().to_string());
        }
        let asks = run.asks()?;
        let choices = run.choices()?;
        // An open input is answered now and then, and always when it is
        // all there is to do.
        let answering = !asks.is_empty() && (choices.is_empty() || next(3) == 0);
        if let Some(ask) = asks.first().filter(|_| answering) {
            let done = match ask.kind.as_str() {
                "note" => run.answer(&[
                    ask.options[next(ask.options.len())].clone(),
                    ask.methods[next(ask.methods.len())].clone(),
                ]),
                "name input" | "text input" => run.answer(&[format!("Name{}", next(100))]),
                _ => run.answer(&[ask.options[next(ask.options.len())].clone()]),
            };
            done?;
        } else if !choices.is_empty() {
            let (id, _) = &choices[next(choices.len())];
            run.choose(id)?;
        } else {
            return Err(format!("nothing to do at `{}`", run.here()));
        }
    }
    Err(format!(
        "no ending after {limit} actions, at `{}`",
        run.here()
    ))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Whatever the gardener does, every year ends in one of its endings.
    #[test]
    fn every_year_ends(seed in 1u64..10_000, picks in prop::collection::vec(any::<u16>(), 50..200)) {
        match random_game(seed, &picks, 2_000) {
            Ok(end) => prop_assert!(end.starts_with("ending_"), "{}", end),
            Err(e) => prop_assert!(false, "{}", e),
        }
    }
}
