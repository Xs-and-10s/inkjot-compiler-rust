//! The engine, as the platform calls it: states are values, a reading is
//! its log, the same log plays the same way, and errors are data.

use inkjot::engine::{
    catalogue_markdown, Action, Event, Inklink, RefusedKind, Start, State, CATALOGUE,
    ENGINE_VERSION,
};
use inkjot::json::Value;
use proptest::prelude::*;

const NAMES: [&str; 3] = [
    "ex1-the-night-train",
    "ex2-the-ninefold-cell",
    "ex3-the-fall-of-the-old-regime",
];

fn source(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/examples/{name}.twee",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// Compile an example and load it through the JSON text, as the platform
/// will.
fn load(which: usize) -> Inklink {
    let ir = inkjot::build(&source(NAMES[which])).unwrap();
    Inklink::load(&ir.to_string()).unwrap()
}

/// Where a random reading may begin.
fn starts() -> Vec<(usize, Start)> {
    let at = |nib: Option<&str>, pov: Option<&str>| Start {
        nib: nib.map(str::to_string),
        pov: pov.map(str::to_string),
        hints: true,
        seats: Vec::new(),
    };
    vec![
        (0, at(Some("cast"), None)),
        (0, at(Some("dining@1"), Some("wren"))),
        (0, at(Some("sleeper@3"), Some("rook"))),
        (1, Start::default()),
        (1, at(Some("orrery"), Some("eldric"))),
        (1, at(Some("ossuary"), Some("roland"))),
        (2, Start::default()),
    ]
}

/// Something a Linker could do here, chosen by `pick`: answer what is
/// asked (always when nothing else is offered), or take a Link.
fn something(state: &State, pick: usize) -> Option<Action> {
    let view = state.view();
    if let Some(ask) = view.asks.first() {
        if view.links.is_empty() || pick.is_multiple_of(3) {
            match ask.kind.as_str() {
                "name input" => return Some(Action::Answer(vec!["Ash".into()])),
                "choose input" if !ask.options.is_empty() => {
                    let n = ask.length.unwrap_or(1);
                    let words = (0..n)
                        .map(|i| ask.options[(pick + i * 7) % ask.options.len()].clone())
                        .collect();
                    return Some(Action::Answer(words));
                }
                "accuse" if !ask.voted && !ask.options.is_empty() => {
                    return Some(Action::Vote {
                        who: ask.options[pick % ask.options.len()].clone(),
                        how: ask
                            .methods
                            .get(pick % ask.methods.len().max(1))
                            .cloned()
                            .unwrap_or_default(),
                        proof: Vec::new(),
                    });
                }
                _ => {}
            }
        }
    }
    let link = view.links.get(pick % view.links.len().max(1))?;
    Some(Action::TakeLink(link.id.clone()))
}

/// Play a reading by `picks`, keeping every state on the way. Stops at the
/// end, or at a feature the interpreter does not run yet; any other
/// refusal is a failure.
fn play(inklink: &Inklink, seed: u64, start: &Start, picks: &[usize]) -> Vec<State> {
    let mut states = vec![inklink.start(seed, start).unwrap()];
    for &pick in picks {
        let now = states.last().unwrap();
        let Some(action) = something(now, pick) else {
            break;
        };
        match now.act(&action) {
            Ok(next) => states.push(next),
            Err(r) if r.kind == RefusedKind::Unsupported => break,
            Err(r) => panic!("unexpected refusal of {action:?}: {r}"),
        }
    }
    states
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Replaying a reading's log to any point gives the state the reading
    /// had there, and carrying on from it goes the same way.
    #[test]
    fn replay_equals_playing_step_by_step(
        start in 0..7usize,
        seed in 1..1000u64,
        picks in proptest::collection::vec(0..20usize, 1..30),
        cut in any::<prop::sample::Index>(),
    ) {
        let (which, start) = starts().swap_remove(start);
        let inklink = load(which);
        let states = play(&inklink, seed, &start, &picks);
        let last = states.last().unwrap();
        let k = cut.index(states.len());

        let replayed = inklink.replay(seed, &start, last.actions(), Some(k)).unwrap();
        prop_assert_eq!(replayed.view(), states[k].view());
        prop_assert_eq!(replayed.actions(), states[k].actions());

        let mut carried = replayed;
        for action in &last.actions()[k..] {
            carried = carried.act(action).unwrap();
        }
        prop_assert_eq!(carried.view(), last.view());
    }

    /// Acting never changes the state acted on: it is still there, as it
    /// was, and acting on it again goes the same way.
    #[test]
    fn states_are_values(
        start in 0..7usize,
        seed in 1..1000u64,
        picks in proptest::collection::vec(0..20usize, 1..30),
    ) {
        let (which, start) = starts().swap_remove(start);
        let inklink = load(which);
        let states = play(&inklink, seed, &start, &picks);
        for pair in states.windows(2) {
            let (before, after) = (&pair[0], &pair[1]);
            let action = after.actions().last().unwrap();
            prop_assert_eq!(before.actions(), &after.actions()[..after.actions().len() - 1]);
            let again = before.act(action).unwrap();
            prop_assert_eq!(again.view(), after.view());
        }
    }

    /// Two readings with the same seed and the same picks see the same
    /// thing at every step.
    #[test]
    fn the_same_log_reads_the_same_way(
        start in 0..7usize,
        seed in 1..1000u64,
        picks in proptest::collection::vec(0..20usize, 1..30),
    ) {
        let (which, start) = starts().swap_remove(start);
        let one = play(&load(which), seed, &start, &picks);
        let two = play(&load(which), seed, &start, &picks);
        prop_assert_eq!(one.len(), two.len());
        for (a, b) in one.iter().zip(&two) {
            prop_assert_eq!(a.view(), b.view());
        }
    }

    /// A Link that is not offered, or an answer when nothing asks, is
    /// refused, and the state is as it was.
    #[test]
    fn what_is_not_offered_is_refused(
        start in 0..7usize,
        seed in 1..1000u64,
        picks in proptest::collection::vec(0..20usize, 0..20),
        bogus in "[a-z_]{1,12}",
    ) {
        let (which, start) = starts().swap_remove(start);
        let states = play(&load(which), seed, &start, &picks);
        let state = states.last().unwrap();
        let before = state.view().clone();
        if !state.view().links.iter().any(|l| l.id == bogus) {
            let r = state.take_link(&bogus).unwrap_err();
            prop_assert_eq!(r.kind, RefusedKind::NotOffered);
        }
        if state.view().asks.is_empty() {
            let r = state.answer(&["Ash".to_string()]).unwrap_err();
            prop_assert_eq!(r.kind, RefusedKind::NotAsking);
        }
        prop_assert_eq!(state.view(), &before);
    }
}

#[test]
fn meta_describes_the_inklink() {
    let meta = load(2).meta().clone();
    assert_eq!(meta.title, "The Fall of the Old Regime");
    assert_eq!(meta.kind, "story");
    assert_eq!(meta.mode, "solo");
    assert_eq!(meta.ir, "3");
    assert_eq!(meta.start, "arrival");
    assert_eq!(meta.povs, ["skittles"]);
    assert!(meta.tags.iter().any(|t| t == "cozy"));
    assert_eq!(meta.engine, ENGINE_VERSION);
}

#[test]
fn the_committed_ir_loads() {
    for name in NAMES {
        let path = format!("{}/examples/ir/{name}.json", env!("CARGO_MANIFEST_DIR"));
        let ir = std::fs::read_to_string(path).unwrap();
        Inklink::load(&ir).unwrap_or_else(|r| panic!("{name}: {r}"));
    }
}

#[test]
fn a_reading_ends_at_a_finale() {
    let inklink = load(2);
    let mut state = inklink.start(7, &Start::default()).unwrap();
    while let Some(link) = state.view().links.last().cloned() {
        state = state.take_link(&link.id).unwrap();
    }
    assert!(state.view().over);
    assert!(!state.view().text.is_empty());
    let kinds: Vec<&str> = state.events().iter().map(|e| e.kind).collect();
    assert_eq!(kinds, ["nib_shown", "ending_reached", "reading_finished"]);
    let ending = |e: &Event| e.data.get("ending").cloned();
    assert_eq!(ending(&state.events()[1]), Some(Value::str("the_list")));
    assert_eq!(ending(&state.events()[2]), Some(Value::str("the_list")));
    assert_eq!(state.ending(), Some("the_list"));
}

#[test]
fn bad_ir_is_refused() {
    for ir in [
        "",
        "{",
        "[1, 2]",
        r#"{"ir": "2", "config": [], "passages": []}"#,
    ] {
        let r = Inklink::load(ir).unwrap_err();
        assert_eq!(r.kind, RefusedKind::BadIr, "{ir}: {r}");
    }
}

#[test]
fn a_bad_start_is_refused() {
    let inklink = load(2);
    let nowhere = Start {
        nib: Some("nowhere".into()),
        ..Start::default()
    };
    assert_eq!(
        inklink.start(1, &nowhere).unwrap_err().kind,
        RefusedKind::StoryError
    );
    let nobody = Start {
        pov: Some("nobody".into()),
        ..Start::default()
    };
    assert_eq!(
        inklink.start(1, &nobody).unwrap_err().kind,
        RefusedKind::StoryError
    );
}

#[test]
fn a_refused_replay_says_which_action() {
    let inklink = load(2);
    let state = inklink.start(1, &Start::default()).unwrap();
    let first = state.view().links[0].id.clone();
    let log = [Action::TakeLink(first), Action::TakeLink("nowhere".into())];
    let r = inklink
        .replay(1, &Start::default(), &log, None)
        .unwrap_err();
    assert_eq!((r.kind, r.at), (RefusedKind::NotOffered, Some(1)));
    // Up to the bad action, the log replays.
    assert!(inklink.replay(1, &Start::default(), &log, Some(1)).is_ok());
}

/// Two passages that send the reader to each other forever.
const ROUND_AND_ROUND: &str = r#":: StoryTitle
Round and Round

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "hall"}

:: Inklink [config]
kind: story
mode: solo
pov: ann
character ann: "Ann" ~ selectable

:: hall
The hall.
> [[Go round->left]]

:: left
next: right
--
Left.

:: right
next: left
--
Right.
"#;

/// A `next:` loop runs into the step limit and is refused, on a stack far
/// smaller than a main thread's, and the state before it is untouched.
#[test]
fn a_next_loop_hits_the_step_limit() {
    let ir = inkjot::build(ROUND_AND_ROUND).unwrap();
    let inklink = Inklink::from_ir(&ir).unwrap();
    let small = std::thread::Builder::new().stack_size(512 * 1024);
    let outcome = small
        .spawn(move || {
            let hall = inklink.start(1, &Start::default()).unwrap();
            let before = hall.view().clone();
            let r = hall.take_link(&before.links[0].id).unwrap_err();
            (r.kind, hall.view() == &before)
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(outcome, (RefusedKind::StepLimit, true));
}

/// The keys of an event's data, in order.
fn keys(e: &Event) -> Vec<&str> {
    match &e.data {
        Value::Obj(pairs) => pairs.iter().map(|(k, _)| k.as_str()).collect(),
        _ => Vec::new(),
    }
}

fn data<'a>(e: &'a Event, key: &str) -> &'a Value {
    e.data.get(key).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Every event the engine reports is in the catalogue, with exactly
    /// its keys. Every action reports one `nib_shown`, naming the Nib and
    /// the Links in the view; `ending_reached` only on arriving at a
    /// finale; `reading_finished` exactly when the reading is over.
    #[test]
    fn every_event_is_in_the_catalogue(
        start in 0..7usize,
        seed in 1..1000u64,
        picks in proptest::collection::vec(0..20usize, 1..40),
    ) {
        let (which, start) = starts().swap_remove(start);
        for state in play(&load(which), seed, &start, &picks) {
            let view = state.view();
            for e in state.events() {
                let entry = CATALOGUE.iter().find(|t| t.name == e.kind);
                prop_assert!(entry.is_some(), "{} is not in the catalogue", e.kind);
                prop_assert_eq!(keys(e), entry.unwrap().keys.to_vec());
            }
            let shown: Vec<&Event> = state.events().iter().filter(|e| e.kind == "nib_shown").collect();
            prop_assert_eq!(shown.len(), 1);
            prop_assert_eq!(data(shown[0], "nib"), &Value::str(&view.nib));
            let links: Vec<Value> = view.links.iter().map(|l| Value::str(&l.id)).collect();
            prop_assert_eq!(data(shown[0], "links"), &Value::Arr(links));
            let finished = state.events().iter().any(|e| e.kind == "reading_finished");
            prop_assert_eq!(finished, view.over);
            if state.events().iter().any(|e| e.kind == "ending_reached") {
                prop_assert!(view.tags.iter().any(|t| t == "finale"));
            }
        }
    }

    /// Replaying a reading with the engine alone, from its seed and its
    /// actions, reports exactly its events, in order: the check the
    /// platform runs against its log.
    #[test]
    fn replay_reports_exactly_the_readings_events(
        start in 0..7usize,
        seed in 1..1000u64,
        picks in proptest::collection::vec(0..20usize, 1..30),
    ) {
        let (which, start) = starts().swap_remove(start);
        let inklink = load(which);
        let live = play(&inklink, seed, &start, &picks);
        let replayed = inklink
            .replay_all(seed, &start, live.last().unwrap().actions())
            .unwrap();
        prop_assert_eq!(replayed.len(), live.len());
        for (a, b) in live.iter().zip(&replayed) {
            prop_assert_eq!(a.events(), b.events());
            prop_assert_eq!(a.view(), b.view());
        }
    }
}

#[test]
fn the_catalogue_is_documented() {
    let doc =
        std::fs::read_to_string(format!("{}/docs/engine.md", env!("CARGO_MANIFEST_DIR"))).unwrap();
    assert!(
        doc.contains(&catalogue_markdown()),
        "docs/engine.md's event table differs from the catalogue:\n{}",
        catalogue_markdown()
    );
}

#[test]
fn an_event_carries_its_origin_and_version() {
    let state = load(2).start(1, &Start::default()).unwrap();
    let json = state.events()[0].to_json().to_string();
    assert!(json.contains(r#""type": "nib_shown""#), "{json}");
    assert!(json.contains(r#""origin": "engine""#), "{json}");
    assert!(json.contains(r#""type_version": 1"#), "{json}");
}

/// Take the offered Link with this label.
fn take(state: &State, label: &str) -> State {
    let link = state
        .view()
        .links
        .iter()
        .find(|l| l.label.plain() == label)
        .unwrap_or_else(|| panic!("no Link {label:?} in {:?}", state.view().links));
    state.take_link(&link.id).unwrap()
}

fn kinds(state: &State) -> Vec<&'static str> {
    state.events().iter().map(|e| e.kind).collect()
}

/// Ex 2: a turn ends, and the Backward Knot takes Roland back to its start.
#[test]
fn a_rewind_is_reported() {
    let inklink = load(1);
    let at = |nib: &str| Start {
        nib: Some(nib.into()),
        ..Start::default()
    };
    let s = inklink.start(1, &at("prologue")).unwrap();
    let s = take(&s, "Wake as Roland, in the pit");
    let s = take(&s, "Look up through the grate");
    // The turn ends when both heroes have passed.
    let s = take(&s, "End the turn");
    assert!(!kinds(&s).contains(&"turn_ended"));
    let s = take(&take(&s, "Switch to Eldric"), "End the turn");
    assert!(kinds(&s).contains(&"turn_ended"));
    let ended = s.events().iter().find(|e| e.kind == "turn_ended").unwrap();
    assert_eq!(data(ended, "stop"), &Value::Num("0".into()));
    let s = take(&s, "Switch to Roland");
    let s = take(&s, "Tug the bolt across the room");
    let s = take(&s, "Tie the Backward Knot");
    let s = take(&s, "Back to the start of turn 1");
    assert_eq!(kinds(&s), ["rewound", "nib_shown"]);
    let back = &s.events()[0];
    assert_eq!(data(back, "who"), &Value::str("roland"));
    assert_eq!(data(back, "stop"), &Value::Num("1".into()));
    assert_eq!(data(back, "checkpoint"), &Value::Null);
    assert_eq!(data(back, "echo"), &Value::Bool(false));
    assert_eq!(data(back, "cause"), &Value::str("rewind"));
}

/// A checkpoint is reached, a death is caught by it, and the world falls
/// back: reported in the order it happened.
const THE_PIT: &str = r#":: StoryTitle
The Pit

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "hall"}

:: Inklink [config]
kind: game
mode: solo
pov: ann
character ann: "Ann" ~ selectable
place hall: "the hall" ~ checkpoint
place pit: "the pit"
way ladder: hall - pit
checkpoint hall ~ on death in pit
~ resets world
~ to hall

:: hall
checkpoint hall
--
The hall.
> [[Jump into the pit->pit]]

:: pit
kill ann
--
You fall.
"#;

#[test]
fn a_death_and_a_fall_back_are_reported() {
    let inklink = Inklink::from_ir(&inkjot::build(THE_PIT).unwrap()).unwrap();
    let hall = inklink.start(1, &Start::default()).unwrap();
    assert_eq!(kinds(&hall), ["checkpoint_reached", "nib_shown"]);
    let fell = take(&hall, "Jump into the pit");
    assert_eq!(
        kinds(&fell),
        ["died", "rewound", "checkpoint_reached", "nib_shown"]
    );
    let died = &fell.events()[0];
    assert_eq!(
        (data(died, "who"), data(died, "at")),
        (&Value::str("ann"), &Value::str("pit"))
    );
    let back = &fell.events()[1];
    assert_eq!(data(back, "checkpoint"), &Value::str("hall"));
    assert_eq!(data(back, "cause"), &Value::str("death"));
    let shown = &fell.events()[3];
    assert_eq!(data(shown, "nib"), &Value::str("hall"));
    assert_eq!(data(shown, "passed"), &Value::Arr(vec![Value::str("pit")]));
    assert_eq!(data(shown, "visit"), &Value::Num("2".into()));
}
