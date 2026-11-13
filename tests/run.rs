//! The interpreter: the `[test]` passages of the three examples, and
//! properties of random playthroughs.

use inkjot::run::{test::run_all, Run, Story};
use proptest::prelude::*;

const NAMES: [&str; 3] = [
    "ex1-the-night-train",
    "ex2-the-ninefold-cell",
    "ex3-the-fall-of-the-old-regime",
];

/// A story and its tests, read as one source, as `inkjot test` reads them.
fn story_and_tests(name: &str) -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    let story = std::fs::read_to_string(format!("{dir}/examples/{name}.twee")).unwrap();
    let tests = std::fs::read_to_string(format!("{dir}/examples/tests/{name}.tests.twee")).unwrap();
    format!("{story}\n{tests}")
}

#[test]
fn every_example_test_passes() {
    for name in NAMES {
        let src = story_and_tests(name);
        let ir =
            inkjot::build(&src).unwrap_or_else(|r| panic!("{name} did not build: {:#?}", r.diags));
        let outcomes = run_all(&src, &ir).unwrap();
        assert!(outcomes.len() >= 6, "{name} lost its tests");
        for o in outcomes {
            assert!(
                o.passed(),
                "{name}: test `{}` failed: {:#?}\ntranscript:\n{}",
                o.name,
                o.failures,
                o.transcript.join("\n")
            );
        }
    }
}

#[test]
fn tests_are_left_out_of_the_published_ir() {
    let src = story_and_tests(NAMES[2]);
    let ir = inkjot::build(&src).unwrap().to_string();
    assert!(!ir.contains("the_shortest_path_to_the_end"));
}

#[test]
fn a_failing_test_says_what_it_saw() {
    let src = format!(
        "{}\n:: wrong [test]\nstart: arrival\ndo: \"“They named me Biscuit.”\"\nexpect: me.grievance == name\n",
        std::fs::read_to_string(format!("{}/examples/{}.twee", env!("CARGO_MANIFEST_DIR"), NAMES[2])).unwrap()
    );
    let ir = inkjot::build(&src).unwrap();
    let outcome = &run_all(&src, &ir).unwrap()[0];
    assert!(!outcome.passed());
    let (_, why) = &outcome.failures[0];
    assert!(why.contains("no available Link matches"), "{why}");
    assert!(
        why.contains("“They named me Skittles.”"),
        "the failure should list what was on offer: {why}"
    );
    // The run stopped at the failed `do`, so the `expect` after it was not tried.
    assert_eq!(outcome.failures.len(), 1);
}

/// The same seed and the same choices always give the same story.
#[test]
fn a_run_is_deterministic() {
    let ir = inkjot::build(
        &std::fs::read_to_string(format!(
            "{}/examples/{}.twee",
            env!("CARGO_MANIFEST_DIR"),
            NAMES[2]
        ))
        .unwrap(),
    )
    .unwrap();
    let story = Story::new(&ir).unwrap();
    let play = || {
        let mut run = Run::new(&story, 7);
        run.start(None, None).unwrap();
        let mut text = run.take_output();
        while let Some((id, _)) = run.choices().unwrap().last().cloned() {
            run.choose(&id).unwrap();
            text.extend(run.take_output());
        }
        text
    };
    assert_eq!(play(), play());
}

/// Where a random playthrough of each example may begin.
fn starts() -> Vec<(usize, Option<&'static str>, Option<&'static str>)> {
    vec![
        (0, Some("cast"), None),
        (0, Some("dining@1"), Some("wren")),
        (0, Some("sleeper@3"), Some("rook")),
        (1, None, None),
        (1, Some("orrery"), Some("eldric")),
        (1, Some("ossuary"), Some("roland")),
        (2, None, None),
    ]
}

proptest! {
    /// Taking random Links never crashes the interpreter, and when it
    /// refuses, it says the feature is not supported yet: it never fails
    /// for any other reason.
    #[test]
    fn random_playthroughs_never_fail_unexpectedly(
        start in 0..7usize,
        seed in 1..1000u64,
        picks in proptest::collection::vec(0..20usize, 1..40),
    ) {
        let (which, at, who) = starts()[start];
        let src = std::fs::read_to_string(format!("{}/examples/{}.twee", env!("CARGO_MANIFEST_DIR"), NAMES[which])).unwrap();
        let ir = inkjot::build(&src).unwrap();
        let story = Story::new(&ir).unwrap();
        let mut run = Run::new(&story, seed);
        let mut steps = || -> Result<(), String> {
            run.start(at, who)?;
            for pick in &picks {
                let choices = run.choices()?;
                if choices.is_empty() {
                    break;
                }
                let (id, _) = &choices[pick % choices.len()];
                run.choose(&id.clone())?;
                let _ = run.take_output();
            }
            Ok(())
        };
        if let Err(why) = steps() {
            prop_assert!(why.contains("not supported by the interpreter yet"), "unexpected failure: {why}");
        }
    }
}

/// A tiny story where `let` and `get` must disagree: a counter is raised in
/// the header, after both are bound by the parent passage.
const LET_AND_GET: &str = r#":: StoryTitle
Counting

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.5.0", "start": "room"}

:: Inklink [config]
kind: story
mode: solo
pov: ann
character ann: "Ann" ~ playable
fact count: 0
fact pinned: 0

:: room
let before: count
get now: count
get double: now * 2
--
> [[Count->room:count]]

:: room:count [act]
raise count 1
--
Before: {before}. Now: {now}. Double: {double}.

:: pin [test]
start: room
do: "Count"
expect text: "Before: 0. Now: 1. Double: 2."
do: "Count"
expect text: "Before: 1. Now: 2. Double: 4."
"#;

#[test]
fn let_is_fixed_on_arrival_and_get_is_read_when_used() {
    let ir = inkjot::build(LET_AND_GET).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    let outcomes = run_all(LET_AND_GET, &ir).unwrap();
    assert!(
        outcomes[0].passed(),
        "{:#?}\n{}",
        outcomes[0].failures,
        outcomes[0].transcript.join("\n")
    );
}

#[test]
fn a_get_that_depends_on_itself_is_an_error() {
    let src = LET_AND_GET.replace(
        "get double: now * 2",
        "get double: now * other\nget other: double + 1",
    );
    let report = inkjot::check(&src);
    let msgs: Vec<&str> = report.diags.iter().map(|d| d.msg.as_str()).collect();
    assert!(
        msgs.iter()
            .any(|m| m.contains("depends on itself") && m.contains("double → other → double")),
        "{msgs:#?}"
    );
}
