//! Ex 8, *The Inquest*, on today's engine.
//!
//! The draft uses a few v0.6 constructs not built yet (`told:`,
//! `[if …; then …]`, blockquotes, redaction, `expect events:`). Everything
//! else is built: forks, analysis tags, emphasis, `~ track`, names as the
//! reader knows them. So these tests take the draft, rewrite the unbuilt
//! parts into what they will mean, and run it: its own `[test]` passages
//! through the interpreter, and its Inquests through the engine API.

use inkjot::engine::{Inklink, Start, State};
use inkjot::json::{self, Value};
use inkjot::run::test::run_all;

fn draft() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    std::fs::read_to_string(format!("{dir}/examples/drafts/ex8-the-inquest.twee")).unwrap()
}

/// The draft in what today's engine reads:
///
/// - `told: …` lines go (the engine does not need them to run);
/// - `[quote …]` and `[redacted until …]` become `[if yes]`, so their text
///   shows and their `[end]` still closes them;
/// - `[if c; then e]` becomes `[if c]`, with `e when c` added to the
///   passage's header, which is what it will do;
/// - `expect events:` lines, and tests that take forks by label, go.
fn built_subset(src: &str) -> String {
    let mut passages: Vec<Vec<String>> = Vec::new();
    for line in src.lines() {
        if line.starts_with("::") || passages.is_empty() {
            passages.push(Vec::new());
        }
        passages.last_mut().unwrap().push(line.to_string());
    }
    let mut out = Vec::new();
    for p in passages {
        if p.iter()
            .any(|l| l.starts_with("take fork:") || l.starts_with("expect fork:"))
        {
            continue;
        }
        let mut header_adds = Vec::new();
        let mut body = Vec::new();
        for line in &p {
            let t = line.trim();
            if t.starts_with("told:") || t.starts_with("expect events:") {
                continue;
            }
            if t.starts_with("[quote") || t.starts_with("[redacted") {
                body.push("[if yes]".to_string());
                continue;
            }
            if let Some(inner) = t.strip_prefix("[if ").and_then(|r| r.strip_suffix(']')) {
                if let Some((cond, effect)) = inner.split_once("; then ") {
                    header_adds.push(format!("{effect} when {cond}"));
                    body.push(format!("[if {cond}]"));
                    continue;
                }
            }
            body.push(line.clone());
        }
        if !header_adds.is_empty() {
            match body.iter().position(|l| l.trim() == "--") {
                Some(i) => {
                    for (k, add) in header_adds.into_iter().enumerate() {
                        body.insert(i + k, add);
                    }
                }
                None => {
                    let mut with = vec![body[0].clone()];
                    with.extend(header_adds);
                    with.push("--".into());
                    with.extend(body[1..].iter().cloned());
                    body = with;
                }
            }
        }
        out.extend(body);
    }
    out.join("\n") + "\n"
}

fn inklink() -> Inklink {
    let ir = inkjot::build(&built_subset(&draft())).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    Inklink::from_ir(&ir).unwrap()
}

#[test]
fn the_built_subset_checks_clean() {
    let src = built_subset(&draft());
    let errors: Vec<_> = inkjot::check(&src)
        .diags
        .into_iter()
        .filter(|d| d.severity == inkjot::Severity::Error)
        .map(|d| format!("line {}: {}", d.line, d.msg))
        .collect();
    assert_eq!(errors, Vec::<String>::new());
}

#[test]
fn its_own_tests_pass() {
    let src = built_subset(&draft());
    let ir = inkjot::build(&src).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    let outcomes = run_all(&src, &ir).unwrap();
    assert!(outcomes.len() >= 2);
    for outcome in outcomes {
        assert!(
            outcome.passed(),
            "{:#?}\n{}",
            outcome.failures,
            outcome.transcript.join("\n")
        );
    }
}

fn take(s: &State, id: &str) -> State {
    s.take_link(id)
        .unwrap_or_else(|r| panic!("{id}: {r:?}; offered {:?}", ids(s)))
}

fn ids(s: &State) -> Vec<String> {
    s.view().links.iter().map(|l| l.id.clone()).collect()
}

fn changes(s: &State) -> Value {
    s.events()
        .iter()
        .find(|e| e.kind == "nib_shown")
        .and_then(|e| e.data.get("changes").cloned())
        .unwrap_or(Value::Null)
}

/// The widow's Suppose stands in for "Say that you stayed by the fire", and
/// what it brings out goes on the record.
#[test]
fn the_lantern_is_an_inquest_for_staying_by_the_fire() {
    let ink = inklink();
    assert_eq!(ink.meta().inquests, Some(3));
    let s = ink.start(1, &Start::default()).unwrap();
    let s = take(&s, "call_martha");
    let s = take(&s, "martha_truth");
    assert_eq!(ids(&s), ["stayed"]);
    let forks: Vec<String> = s
        .forks("stayed", None)
        .unwrap()
        .iter()
        .map(|l| l.id.clone())
        .collect();
    assert_eq!(forks, ["lantern"]);
    let supposed = s.take_fork("lantern", None).unwrap();
    assert_eq!(supposed.view().nib, "martha:lantern");
    assert_eq!(
        changes(&supposed),
        json::parse(r#"{"gig_heard": [false, true]}"#).unwrap()
    );
}

/// Every testimony has its Suppose, and the forks never show as Links.
#[test]
fn each_witness_has_a_suppose() {
    let ink = inklink();
    let s = ink.start(1, &Start::default()).unwrap();
    let s = take(&s, "call_martha");
    let s = take(&s, "martha_truth");
    let s = take(&s, "stayed");
    let s = take(&s, "martha_down2");
    let s = take(&s, "call_tom");
    let s = take(&s, "tom_checks");
    // The checks may be counted again, up to four times.
    let s = take(&s, "count_again");
    let s = take(&s, "count_more");
    let s = take(&s, "count_more");
    assert!(
        !ids(&s).contains(&"count_more".to_string()),
        "{:?}",
        ids(&s)
    );
    let s = take(&s, "half_nine2");
    let forks: Vec<String> = s
        .forks("ran", None)
        .unwrap()
        .iter()
        .map(|l| l.id.clone())
        .collect();
    assert_eq!(forks, ["tom_stayed"]);
    let s = s.take_fork("tom_stayed", None).unwrap();
    let s = take(&s, "tom_down3");
    let s = take(&s, "call_hesketh");
    let forks: Vec<String> = s
        .forks("cannot", None)
        .unwrap()
        .iter()
        .map(|l| l.id.clone())
        .collect();
    assert_eq!(forks, ["other_hands"]);
    let s = take(&s, "cannot");
    let s = take(&s, "hesketh_down2");
    let s = take(&s, "call_fisk");
    // The boy put Fisk at the frame, so he may confess.
    assert!(ids(&s).contains(&"fisk_truth".to_string()));
    let forks: Vec<String> = s
        .forks("fisk_lie", None)
        .unwrap()
        .iter()
        .map(|l| l.id.clone())
        .collect();
    assert_eq!(forks, ["shilling"]);
    let s = s.take_fork("shilling", None).unwrap();
    let s = take(&s, "fisk_lie2");
    let s = take(&s, "fisk_down");
    // The shilling makes Garrard a witness.
    let s = take(&s, "call_garrard");
    assert_eq!(
        s.view().text[0].plain(),
        "Mr. Fenwick, the coroner: Your name, sir, for the record."
    );
    let forks: Vec<String> = s
        .forks("dined", None)
        .unwrap()
        .iter()
        .map(|l| l.id.clone())
        .collect();
    assert_eq!(forks, ["at_home"]);
}

/// Fisk's confession, with the shilling, is the one way to the rider on
/// the Squire; the verdict reads only the record.
#[test]
fn a_confession_and_the_shilling_bring_the_rider() {
    let ink = inklink();
    let mut s = ink.start(1, &Start::default()).unwrap();
    for id in [
        "call_martha",
        "martha_truth",
        "stayed",
        "martha_down2",
        "call_tom",
        "tom_checks",
        "half_nine",
    ] {
        s = take(&s, id);
    }
    s = s.take_fork("tom_stayed", None).unwrap();
    for id in [
        "tom_down3",
        "call_hesketh",
        "cannot",
        "hesketh_down2",
        "call_fisk",
    ] {
        s = take(&s, id);
    }
    s = s.take_fork("shilling", None).unwrap();
    for id in [
        "fisk_truth2",
        "fisk_down2",
        "call_garrard",
        "drove",
        "garrard_down2",
        "retire",
    ] {
        s = take(&s, id);
    }
    let text: Vec<String> = s.view().text.iter().map(|p| p.plain()).collect();
    assert!(
        text.iter().any(|t| t.contains("Unlawful killing")),
        "{text:?}"
    );
    assert!(
        text.iter().any(|t| t.contains("answer for the shilling")),
        "{text:?}"
    );
    assert_eq!(ids(&s), ["after_unlawful"]);
}

use proptest::prelude::*;

/// Walk a reading: at each Nib take the Link `picks` says (or a fork in its
/// place, while Inquests last), until there is nothing left to take.
fn walk(ink: &Inklink, picks: &[(u8, bool)]) -> (State, Vec<String>) {
    walk_to(ink, picks, 80)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// Every reading, whatever is said and supposed, reaches the verdict
    /// once, hears exactly one verdict (and the rider only with an unlawful
    /// killing), is offered exactly one way on, and ends on the record.
    #[test]
    fn every_reading_reaches_one_verdict_and_the_record(
        picks in prop::collection::vec((any::<u8>(), any::<bool>()), 1..40)
    ) {
        let ink = inklink();
        let (end, nibs) = walk(&ink, &picks);
        prop_assert_eq!(end.view().nib.as_str(), "the_record", "{:?}", nibs);
        prop_assert_eq!(nibs.iter().filter(|n| n.as_str() == "verdict").count(), 1);
        // Back at the verdict: one verdict line, one Link on.
        let i = nibs.iter().position(|n| n == "verdict").unwrap();
        let ink2 = inklink();
        let (at_verdict, _) = walk_to(&ink2, &picks, i);
        let said: Vec<String> = at_verdict.view().text.iter().map(|p| p.plain()).collect();
        let verdicts = said.iter().filter(|t| {
            ["Unlawful killing", "misadventure", "Accidental death", "open verdict"]
                .iter().any(|v| t.contains(v))
        }).count();
        prop_assert_eq!(verdicts, 1, "{:?}", said);
        let rider = said.iter().any(|t| t.contains("answer for the shilling"));
        let unlawful = said.iter().any(|t| t.contains("Unlawful killing"));
        prop_assert!(!rider || unlawful);
        prop_assert_eq!(at_verdict.view().links.len(), 1);
    }
}

/// The walk, stopped after `steps` moves or when nothing is offered.
fn walk_to(ink: &Inklink, picks: &[(u8, bool)], steps: usize) -> (State, Vec<String>) {
    let mut s = ink.start(1, &Start::default()).unwrap();
    let mut nibs = vec![s.view().nib.clone()];
    for (i, (pick, suppose)) in picks.iter().cycle().take(steps).enumerate() {
        let links = s.view().links.clone();
        if links.is_empty() {
            break;
        }
        let link = &links[*pick as usize % links.len()];
        let forks = s.forks(&link.id, None).unwrap();
        s = if *suppose && !forks.is_empty() && i % 2 == 0 {
            match s.take_fork(&forks[0].id, None) {
                Ok(next) => next,
                Err(_) => s.take_link(&link.id).unwrap(),
            }
        } else {
            s.take_link(&link.id).unwrap()
        };
        nibs.push(s.view().nib.clone());
    }
    (s, nibs)
}

/// One reading, written out for a person to read: the path with every
/// Suppose taken that leads to the rider.
/// `cargo test --test ex8 a_reading -- --ignored --nocapture`
#[test]
#[ignore = "prints a reading"]
fn a_reading() {
    let ink = inklink();
    let mut s = ink.start(1, &Start::default()).unwrap();
    let show = |s: &State| {
        for p in &s.view().text {
            println!("{}\n", p.plain());
        }
    };
    show(&s);
    let steps: &[(&str, bool)] = &[
        ("call_martha", false),
        ("martha_truth", false),
        ("lantern", true),
        ("martha_down3", false),
        ("call_tom", false),
        ("tom_checks", false),
        ("count_again", false),
        ("half_nine2", false),
        ("tom_stayed", true),
        ("tom_down3", false),
        ("note_hands", false),
        ("call_hesketh2", false),
        ("cannot", false),
        ("hesketh_down2", false),
        ("call_fisk", false),
        ("shilling", true),
        ("fisk_truth2", false),
        ("fisk_down2", false),
        ("call_garrard", false),
        ("drove", false),
        ("garrard_down2", false),
        ("retire", false),
        ("after_unlawful", false),
        ("record_unlawful", false),
    ];
    for (id, fork) in steps {
        let label = if *fork {
            let offered: Vec<String> = s.view().links.iter().map(|l| l.id.clone()).collect();
            offered
                .iter()
                .flat_map(|o| s.forks(o, None).unwrap())
                .find(|f| f.id == *id)
                .map(|f| format!("[Inquest] {}", f.label.plain()))
                .unwrap()
        } else {
            s.view()
                .links
                .iter()
                .find(|l| l.id == *id)
                .unwrap()
                .label
                .plain()
        };
        println!("> {label}\n");
        s = if *fork {
            s.take_fork(id, None).unwrap()
        } else {
            s.take_link(id).unwrap()
        };
        show(&s);
    }
}

/// Every passage can be reached from the notice, along Links and forks,
/// and every passage but the last leads somewhere: no dead ends but the
/// record.
#[test]
fn every_passage_is_reached_and_only_the_record_ends() {
    let map = inklink().map();
    let mut reached = std::collections::BTreeSet::from(["notice".to_string()]);
    loop {
        let before = reached.len();
        for e in &map.edges {
            if reached.contains(&e.from) {
                reached.insert(e.to.clone());
            }
        }
        if reached.len() == before {
            break;
        }
    }
    let nodes: Vec<&str> = map.nodes.iter().map(|n| n.id.as_str()).collect();
    let unreached: Vec<&&str> = nodes
        .iter()
        .filter(|n| !reached.contains(**n) && **n != "Inklink")
        .collect();
    assert!(unreached.is_empty(), "never reached: {unreached:?}");
    let dead_ends: Vec<&&str> = nodes
        .iter()
        .filter(|n| **n != "the_record" && !map.edges.iter().any(|e| e.from == **n))
        .collect();
    assert!(dead_ends.is_empty(), "dead ends: {dead_ends:?}");
}
