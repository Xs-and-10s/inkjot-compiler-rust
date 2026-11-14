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

/// A tiny two-room story for NPC agendas: a statue in the hall creeps toward
/// whoever is alive and unwatching, and strikes when it gets there.
const STATUE: &str = r#":: StoryTitle
Statue

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.5.0", "start": "hall"}

:: Inklink [config]
kind: story
mode: solo
pov: ann, bob
start: hall for ann, study for bob
character ann: "Ann" ~ playable
character bob: "Bob" ~ playable
place hall: "the hall"
place study: "the study"
way door: hall - study
archetype statue: ~ lifeless
npc angel: "the angel" ~ like statue ~ in hall

:: statue [archetype]
--
goal stalk:
~ toward nearest(living)
~ weight 1
~ when not self.watched

reaction strike:
~ when not self.watched and count(living, in: here) >= 1
~ then kill one(living, in: here)

:: hall
The hall.
> [[Wait->here]] ~ pass
> [[Look away->study]] ~ become bob

:: study
The study.
> [[Wait->here]] ~ pass

:: watched [test]
start: hall ~ as ann
do: "Wait"
expect: angel.in(hall)
expect: not ann.dead

:: unwatched [test]
start: hall ~ as ann
do: "Look away"
expect at: study
do: "Wait"
expect: ann.dead
expect: angel.in(hall)
do: "Wait"
expect: angel.in(study)
"#;

#[test]
fn a_statue_moves_and_strikes_only_when_no_one_is_looking() {
    let ir = inkjot::build(STATUE).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    for outcome in run_all(STATUE, &ir).unwrap() {
        assert!(
            outcome.passed(),
            "{}: {:#?}\n{}",
            outcome.name,
            outcome.failures,
            outcome.transcript.join("\n")
        );
    }
}

/// Play Ex 1 with a cohort: `cast` picks who sits, and each step serves the
/// `seat`-th Linker who can still act and takes their `pick`-th Link.
/// Returns everything shown, or the error that stopped play, and checks the
/// stop's invariants after every step.
fn cohort_play(
    cast: &[&str],
    seed: u64,
    steps: &[(usize, usize)],
) -> Result<Vec<String>, TestCaseError> {
    let src = std::fs::read_to_string(format!(
        "{}/examples/ex1-the-night-train.twee",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let ir = inkjot::build(&src).unwrap();
    let story = Story::new(&ir).unwrap();
    let mut run = Run::new(&story, seed);
    let mut text = Vec::new();
    if let Err(why) = run.start_cohort(Some("lounge@0"), cast) {
        prop_assert!(why.contains("not supported"), "{why}");
        return Ok(text);
    }
    for &(seat, pick) in steps {
        // Whoever can still act this stop.
        let mut free = Vec::new();
        for who in run.seated().to_vec() {
            run.sit(&who).unwrap();
            if !run.waiting() && !run.choices().unwrap().is_empty() {
                free.push(who);
            }
        }
        let Some(who) = free.get(seat % free.len().max(1)).cloned() else {
            break;
        };
        run.sit(&who).unwrap();
        let choices = run.choices().unwrap();
        let (id, label) = choices[pick % choices.len()].clone();
        let before = run.now();
        text.push(format!("[{who}] > {label}"));
        if let Err(why) = run.choose(&id) {
            prop_assert!(why.contains("not supported"), "unexpected failure: {why}");
            break;
        }
        text.extend(run.take_output());
        // The clock only moves forward, and only once everyone has moved:
        // then no one is left waiting and no claim is left open.
        prop_assert!(run.now() >= before);
        prop_assert!(run.now() <= 5, "the clock ran past its last stop");
        if run.now() > before {
            prop_assert_eq!(run.open_claims(), 0);
            for other in run.seated().to_vec() {
                run.sit(&other).unwrap();
                prop_assert!(
                    !run.waiting(),
                    "{other} still waiting after the stop resolved"
                );
                prop_assert!(!run.here().is_empty());
            }
        }
    }
    Ok(text)
}

fn cast() -> impl Strategy<Value = Vec<&'static str>> {
    proptest::sample::subsequence(vec!["ada", "rook", "wren", "sallow"], 2..=4).prop_shuffle()
}

proptest! {
    /// A cohort of two to four Linkers, acting in random order, never makes
    /// the interpreter fail, and every stop resolves cleanly.
    #[test]
    fn cohorts_play_cleanly(
        who in cast(),
        seed in 1..1000u64,
        steps in proptest::collection::vec((0..4usize, 0..20usize), 1..60),
    ) {
        cohort_play(&who, seed, &steps)?;
    }

    /// The same seed and the same choices tell the same story, word for word.
    #[test]
    fn cohort_play_is_deterministic(
        who in cast(),
        seed in 1..1000u64,
        steps in proptest::collection::vec((0..4usize, 0..20usize), 1..40),
    ) {
        let first = cohort_play(&who, seed, &steps)?;
        let second = cohort_play(&who, seed, &steps)?;
        prop_assert_eq!(first, second);
    }
}
