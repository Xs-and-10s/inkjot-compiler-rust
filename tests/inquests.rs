//! Inquests (IR 3 §1), analysis tags, `~ track`, and the story map, end to
//! end: the checker, the IR, and the engine.

use inkjot::engine::{Action, Inklink, RefusedKind, Start, State};
use inkjot::json::{self, Value};
use inkjot::{build, check, Severity};

const FORD: &str = r#":: StoryTitle
The Ford

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "ford"}

:: Inklink [config]
kind: story
mode: solo
pov: nell
character nell: "Nell" ~ playable
resource inquests: 3
theme mercy: "Who deserves saving"
state trust: 0 ~ track
state crossed_free: no

:: ford
title: "The Ford"
--
The ferryman holds out his hand.
> [[Pay the ferryman->bank]] ~ id pay ~ then raise trust 1
~ choice inactive ~ beat inciting ~ weighs mercy ~ arc nell rises ~ stakes self ~ plants coin ~ predict 70%
> [[Swim across in the dark->bank]] ~ inquest ~ id swim ~ then set crossed_free: yes
> [[Steal the boat->bank]] ~ inquest for pay ~ id steal ~ inquest from bank ~ credit monkeypaw
> [[Wait for dawn->dawn]] ~ id wait ~ pays off coin
> [[Something else happens->told]] ~ inquest by storyteller ~ rejoin bank ~ id told

:: dawn
next: bank
--
Dawn.

:: told
The story goes somewhere the Inker did not write.
> [[On->bank]] ~ id on

:: bank [finale]
title: "The Far Bank"
--
The far bank.
"#;

fn diags(src: &str, severity: Severity) -> Vec<String> {
    check(src)
        .diags
        .into_iter()
        .filter(|d| d.severity == severity)
        .map(|d| d.msg)
        .collect()
}

fn errors(src: &str) -> Vec<String> {
    diags(src, Severity::Error)
}

fn ford() -> Inklink {
    Inklink::from_ir(&build(FORD).unwrap()).unwrap()
}

fn ids(links: &[inkjot::engine::Link]) -> Vec<&str> {
    links.iter().map(|l| l.id.as_str()).collect()
}

/// The Link with this id in a passage of the IR.
fn link<'a>(ir: &'a Value, passage: &str, id: &str) -> &'a Value {
    ir.get("passages")
        .and_then(Value::as_arr)
        .unwrap()
        .iter()
        .find(|p| p.get("name").and_then(Value::as_str) == Some(passage))
        .and_then(|p| p.get("body"))
        .and_then(Value::as_arr)
        .unwrap()
        .iter()
        .find(|n| n.get("id").and_then(Value::as_str) == Some(id))
        .unwrap_or_else(|| panic!("no Link {id}"))
}

#[test]
fn the_ford_checks_clean() {
    assert_eq!(errors(FORD), Vec::<String>::new());
    assert_eq!(diags(FORD, Severity::Notice), Vec::<String>::new());
}

#[test]
fn a_fork_is_a_link_with_an_inquest_field() {
    let ir = build(FORD).unwrap();
    let j = |s: &str| json::parse(s).unwrap();
    assert_eq!(
        link(&ir, "ford", "swim").get("inquest"),
        Some(&j(r#"{"stands_in_for": "pay", "credit": "inker"}"#))
    );
    assert_eq!(
        link(&ir, "ford", "steal").get("inquest"),
        Some(&j(
            r#"{"stands_in_for": "pay", "from": ["bank"], "credit": "monkeypaw"}"#
        ))
    );
    assert_eq!(
        link(&ir, "ford", "told").get("inquest"),
        Some(&j(
            r#"{"stands_in_for": "wait", "credit": "inker", "pending": {"by": "storyteller", "rejoin": ["bank"]}}"#
        ))
    );
    // A fork keeps its own kind: a move here.
    assert_eq!(
        link(&ir, "ford", "swim").get("kind"),
        Some(&Value::str("go"))
    );
    assert_eq!(
        link(&ir, "ford", "pay").get("analysis"),
        Some(&j(
            r#"{"beat": "inciting", "weighs": ["mercy"], "arc": {"who": "nell", "way": "rises"}, "stakes": "self", "plants": ["coin"], "predict": 70}"#
        ))
    );
    assert_eq!(
        link(&ir, "ford", "wait").get("analysis"),
        Some(&j(r#"{"pays_off": ["coin"]}"#))
    );
}

/// Forks are never in the view; they cross one offered Link at a time, in
/// the order written, and taking one goes where it leads.
#[test]
fn forks_cross_only_when_asked() {
    let inklink = ford();
    assert_eq!(inklink.meta().inquests, Some(3));
    let at_ford = inklink.start(1, &Start::default()).unwrap();
    assert_eq!(ids(&at_ford.view().links), ["pay", "wait"]);
    assert_eq!(ids(&at_ford.forks("pay", None).unwrap()), ["swim", "steal"]);
    assert_eq!(ids(&at_ford.forks("wait", None).unwrap()), ["told"]);
    assert!(at_ford.any_forks(None).unwrap());
    assert_eq!(
        at_ford.forks("pay", None).unwrap()[1].label.plain(),
        "Steal the boat"
    );

    // A fork is not a Link the Linker can take.
    let r = at_ford.take_link("swim").unwrap_err();
    assert_eq!(r.kind, RefusedKind::NotOffered);
    let r = at_ford.take_fork("nowhere", None).unwrap_err();
    assert_eq!(r.kind, RefusedKind::NotOffered);

    let swum = at_ford.take_fork("swim", None).unwrap();
    assert_eq!(swum.view().nib, "bank");
    assert_eq!(swum.ending(), Some("bank"));
    // The log replays it.
    let again = inklink
        .replay(1, &Start::default(), swum.actions(), None)
        .unwrap();
    assert_eq!(again.view(), swum.view());
    assert_eq!(
        swum.actions(),
        [Action::TakeFork {
            fork: "swim".into(),
            from: None
        }]
    );
}

/// Asked from a later passage, only the forks marked `~ inquest from` it
/// are there, taken from the earlier visit's state.
#[test]
fn a_fork_may_be_asked_for_from_later() {
    let inklink = ford();
    assert_eq!(inklink.asked_from("bank"), ["ford"]);
    assert!(inklink.asked_from("dawn").is_empty());
    let at_ford: State = inklink.start(1, &Start::default()).unwrap();
    assert_eq!(ids(&at_ford.forks("pay", Some("bank")).unwrap()), ["steal"]);
    assert!(at_ford.forks("pay", Some("dawn")).unwrap().is_empty());
    let stolen = at_ford.take_fork("steal", Some("bank")).unwrap();
    assert_eq!(stolen.view().nib, "bank");
    assert_eq!(
        at_ford.take_fork("swim", Some("bank")).unwrap_err().kind,
        RefusedKind::NotOffered
    );
}

/// `~ track` values report how an action changed them, on `nib_shown`.
#[test]
fn tracked_values_report_their_changes() {
    let at_ford = ford().start(1, &Start::default()).unwrap();
    let shown = |s: &State| {
        s.events()
            .iter()
            .find(|e| e.kind == "nib_shown")
            .unwrap()
            .data
            .get("changes")
            .unwrap()
            .clone()
    };
    assert_eq!(shown(&at_ford), json::parse("{}").unwrap());
    let paid = at_ford.take_link("pay").unwrap();
    assert_eq!(shown(&paid), json::parse(r#"{"trust": [0, 1]}"#).unwrap());
    let waited = at_ford.take_link("wait").unwrap();
    assert_eq!(shown(&waited), json::parse("{}").unwrap());
}

#[test]
fn the_story_map_has_every_way_through() {
    let map = ford().map();
    let titles: Vec<(&str, Option<&str>)> = map
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n.title.as_deref()))
        .collect();
    assert_eq!(
        titles,
        [
            ("ford", Some("The Ford")),
            ("dawn", None),
            ("told", None),
            ("bank", Some("The Far Bank")),
        ]
    );
    let edges: Vec<(&str, &str, &str, Option<&str>)> = map
        .edges
        .iter()
        .map(|e| {
            (
                e.from.as_str(),
                e.to.as_str(),
                e.kind,
                e.stands_in_for.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        edges,
        [
            ("ford", "bank", "link", None),
            ("ford", "bank", "fork", Some("pay")),
            ("ford", "bank", "fork", Some("pay")),
            ("ford", "dawn", "link", None),
            ("ford", "told", "fork", Some("wait")),
            ("dawn", "bank", "next", None),
            ("told", "bank", "link", None),
        ]
    );
    assert_eq!(map.edges[0].label, "Pay the ferryman");
}

#[test]
fn forks_and_analysis_are_checked() {
    let cases = [
        (
            "> [[Pay the ferryman->bank]] ~ id pay ~ then raise trust 1\n~ choice inactive ~ beat inciting ~ weighs mercy ~ arc nell rises ~ stakes self ~ plants coin ~ predict 70%\n> [[Swim across in the dark->bank]] ~ inquest ~ id swim",
            "> [[Swim across in the dark->bank]] ~ inquest ~ id swim\n> [[Pay the ferryman->bank]] ~ id pay ~ then raise trust 1\n~ choice inactive ~ beat inciting ~ weighs mercy ~ arc nell rises ~ stakes self ~ plants coin ~ predict 70%",
            "a fork stands in for an offered Link: put it below one, or name it with `~ inquest for id`",
        ),
        ("~ inquest for pay", "~ inquest for pai", "`pai` is not an offered Link in this passage"),
        ("mode: solo", "mode: cohort\ncohort: 2..=4", "Inquests are for `mode: solo`; a cohort cannot take forks"),
        ("~ rejoin bank ~ id told", "~ id told", "a pending fork says where it rejoins: `~ rejoin ch5`"),
        ("~ weighs mercy", "~ weighs mercey", "`mercey` is not a declared theme"),
        ("~ beat inciting", "~ beat inciteful", "`beat` is one of: opening, inciting, turn, midpoint, crisis, climax, resolution"),
        ("~ predict 70%", "~ predict 170%", "a prediction is a share of Linkers: `~ predict 40%`"),
        ("~ inquest by storyteller", "~ inquest by mark", "a pending fork is written by a role: monkeypaw, ghostwriter or storyteller"),
        ("~ credit monkeypaw", "~ credit mark", "`credit` is one of: inker, monkeypaw, ghostwriter, storyteller"),
    ];
    for (from, to, want) in cases {
        assert!(FORD.contains(from), "{from}");
        let errs = errors(&FORD.replacen(from, to, 1));
        assert!(errs.iter().any(|e| e == want), "{to}: {errs:?}");
    }
    // A planted thing with no payoff, and a Link without an id beside forks.
    let unpaid = FORD.replace("~ pays off coin", "");
    assert!(diags(&unpaid, Severity::Notice)
        .contains(&"`coin` is planted but never paid off".to_string()));
    let no_id = FORD.replace("~ id wait ", "");
    assert!(diags(&no_id, Severity::Notice)
        .iter()
        .any(|n| n.starts_with("give this Link an id")));
}
