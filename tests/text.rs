//! Text, end to end: braces in every string, escapes, emphasis, and how the
//! interpreter and the engine show them.

use inkjot::engine::{Inklink, Start};
use inkjot::json::Value;
use inkjot::{build, check, Severity};

const STORY: &str = r#":: StoryTitle
Tobin's Table

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "yard"}

:: Inklink [config]
kind: story
mode: solo
pov: nell
character nell: "Nell" ~ selectable
npc tobin: "Old Tobin" ~ in yard
place yard: "the yard"
state cards: 3
state coins: 1
state lied: yes
state title: "nobody"
fact stakes: "Tobin plays for {coins} {plural(coins, "coin")}." ~ clue

:: yard
set title: "Keeper of {yard}"
--
Tobin holds {cards} {plural(cards, "card")}, and *he is **very** pleased* about it.
A \*real\* asterisk, and \{braces\}.
> [[Ask *nicely* for {coins} {plural(coins, "coin")}->yard:ask]]
> [[Leave->yard:leave]]

:: yard:ask [act]
say tobin "You kneel to {if lied then "a liar" else "an honest girl"}, {title}."
--
He thinks about it.

:: yard:leave [act]
--
You leave.
"#;

fn errors(src: &str) -> Vec<String> {
    check(src)
        .diags
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.msg)
        .collect()
}

#[test]
fn the_story_checks() {
    assert_eq!(errors(STORY), Vec::<String>::new());
}

#[test]
fn names_in_braces_are_checked_in_every_string() {
    for (from, to) in [
        (
            "{plural(coins, \"coin\")}.\" ~ clue",
            "{plural(coinz, \"coin\")}.\" ~ clue",
        ),
        ("\"Keeper of {yard}\"", "\"Keeper of {yrad}\""),
        ("{title}.\"", "{titel}.\""),
        ("for {coins} {plural", "for {conis} {plural"),
    ] {
        assert!(STORY.contains(from), "{from}");
        let errs = errors(&STORY.replacen(from, to, 1));
        assert_eq!(errs.len(), 1, "{to}: {errs:?}");
        assert!(errs[0].contains("is not declared"), "{to}: {errs:?}");
    }
}

#[test]
fn bad_braces_are_refused() {
    let stray = STORY.replace("and \\{braces\\}.", "and braces}.");
    assert!(errors(&stray)[0].contains("closes nothing"));
    let inklog = STORY.replace("He thinks about it.", "He thinks {{ tobin thinks }}.");
    assert!(errors(&inklog)[0].contains("Inklog"));
    let open = STORY.replace("\"Keeper of {yard}\"", "\"Keeper of {yard\"");
    assert!(!errors(&open).is_empty());
}

/// Does any node of `v` equal `want`?
fn has(v: &Value, want: &Value) -> bool {
    v == want
        || match v {
            Value::Arr(items) => items.iter().any(|i| has(i, want)),
            Value::Obj(pairs) => pairs.iter().any(|(_, i)| has(i, want)),
            _ => false,
        }
}

fn json(s: &str) -> Value {
    inkjot::json::parse(s).unwrap()
}

/// Emphasis is lowered to styled parts in prose and in Link labels; the
/// escapes reach the IR as plain characters.
#[test]
fn emphasis_and_escapes_in_the_ir() {
    let ir = build(STORY).unwrap();
    assert!(has(
        &ir,
        &json(
            r#"{"style": ["italic"], "parts": ["he is ", {"style": ["bold"], "parts": ["very"]}, " pleased"]}"#
        )
    ));
    assert!(has(&ir, &json(r#"["A *real* asterisk, and {braces}."]"#)));
    assert!(has(
        &ir,
        &json(
            r#"["Ask ", {"style": ["italic"], "parts": ["nicely"]}, " for ", {"value": "coins"}, " ", {"value": {"plural": ["coins", {"str": "coin"}]}}]"#
        )
    ));
    // Quoted text with braces is worked out when used.
    assert!(has(
        &ir,
        &json(r#"{"text": ["Keeper of ", {"value": "yard"}]}"#)
    ));
}

#[test]
fn the_engine_shows_styles_and_works_out_text() {
    let inklink = Inklink::from_ir(&build(STORY).unwrap()).unwrap();
    let yard = inklink.start(1, &Start::default()).unwrap();
    let view = yard.view();
    let first = &view.text[0];
    assert_eq!(
        first.plain(),
        "Tobin holds 3 cards, and he is very pleased about it."
    );
    let styled: Vec<(&str, Vec<&str>)> = first
        .runs
        .iter()
        .map(|r| (r.text.as_str(), r.style.clone()))
        .collect();
    assert_eq!(
        styled,
        [
            ("Tobin holds 3 cards, and ", vec![]),
            ("he is ", vec!["italic"]),
            ("very", vec!["italic", "bold"]),
            (" pleased", vec!["italic"]),
            (" about it.", vec![]),
        ]
    );
    assert_eq!(view.text[1].plain(), "A *real* asterisk, and {braces}.");
    let ask = &view.links[0];
    assert_eq!(ask.label.plain(), "Ask nicely for 1 coin");
    assert_eq!(ask.label.runs[1].style, ["italic"]);

    let asked = yard.take_link(&ask.id).unwrap();
    let said: Vec<String> = asked.view().text.iter().map(|p| p.plain()).collect();
    assert!(
        said.contains(&"Old Tobin: You kneel to a liar, Keeper of the yard.".to_string()),
        "{said:?}"
    );
}

#[test]
fn a_fact_with_braces_reads_its_value_when_shown() {
    let ir = build(STORY).unwrap();
    let fact = ir
        .get("config")
        .and_then(Value::as_arr)
        .unwrap()
        .iter()
        .find(|s| s.get("id").and_then(Value::as_str) == Some("stakes"))
        .unwrap();
    assert!(fact.get("label").unwrap().get("text").is_some());
}

const NAMES: &str = r#":: StoryTitle
Names

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "room"}

:: Inklink [config]
kind: story
mode: solo
pov: skittles
character skittles: "Skittles" ~ selectable
npc widow: "Marge" ~ shown as "the Surviving Authority"
npc leo: "Leo" ~ aka "Leo-Leo", "Lapkins", "Le Le" ~ pick cycle
npc jojo: "Jojo" ~ aka "Jojo Bear", "Juju" ~ pick first

:: room
{widow} is here. Her name is {widow.name}; you call her {widow.shown_as}.
Leo is {leo.aka}, then {leo.aka}, then {leo.aka}, then {leo.aka}.
Jojo is {jojo.aka}, and {jojo.aka}. Leo goes by {all leo.aka}.
@widow Hello, Skittles.
> [[Listen->room:listen]]

:: room:listen [act]
reveal widow
--
{widow} says her own name.
"#;

#[test]
fn the_reader_sees_what_they_know_until_a_reveal() {
    assert_eq!(errors(NAMES), Vec::<String>::new());
    let inklink = Inklink::from_ir(&build(NAMES).unwrap()).unwrap();
    let room = inklink.start(1, &Start::default()).unwrap();
    let text: Vec<String> = room.view().text.iter().map(|p| p.plain()).collect();
    assert_eq!(
        text[0],
        // A name starting a sentence is capitalised (v0.6, 21 Nov 2026).
        "The Surviving Authority is here. Her name is Marge; you call her the Surviving Authority."
    );
    assert_eq!(
        text[1],
        "Leo is Leo-Leo, then Lapkins, then Le Le, then Leo-Leo."
    );
    assert_eq!(
        text[2],
        "Jojo is Jojo Bear, and Jojo Bear. Leo goes by Leo-Leo, Lapkins and Le Le."
    );
    assert_eq!(text[3], "The Surviving Authority: Hello, Skittles.");
    let link = room.view().links[0].id.clone();
    let after = room.take_link(&link).unwrap();
    assert_eq!(after.view().text[0].plain(), "Marge says her own name.");
}

#[test]
fn aka_picks_and_reveals_are_checked() {
    let bad_pick = NAMES.replace("~ pick cycle", "~ pick backwards");
    assert_eq!(
        errors(&bad_pick),
        ["`pick` is one of: shuffled, cycle, random, first"]
    );
    let old_form = NAMES.replace("~ pick cycle", "~ aka shuffled");
    assert_eq!(
        errors(&old_form),
        ["how `aka` picks is its own clause in v0.6: `~ pick shuffled`"]
    );
    let nobody = NAMES.replace("reveal widow", "reveal widdow");
    assert!(errors(&nobody)[0].contains("`widdow` is not a declared thing"));
}

const READ: &str = r#":: StoryTitle
Quoted

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "deck"}

:: Inklink [config]
kind: story
mode: solo
pov: nell
character nell: "Nell" ~ selectable
npc tobin: "*Old* Tobin" ~ in deck
place deck: "the deck"
state motto: "*never* literal"
fact ship: "She sailed on the *Endeavour*." ~ clue
list log: "the log" ~ holds entry: "a **bold** entry"

:: deck
learn ship
say tobin "Mind the *rope*, {nell}."
--
{list log}
Motto: {motto}.
"#;

/// Text that is read takes emphasis: a `say`, a fact's text, a list entry.
/// A name does not, and a value is data.
#[test]
fn quoted_text_that_is_read_takes_emphasis() {
    assert_eq!(errors(READ), Vec::<String>::new());
    let inklink = Inklink::from_ir(&build(READ).unwrap()).unwrap();
    let deck = inklink.start(1, &Start::default()).unwrap();
    let text = &deck.view().text;
    let styled = |p: &inkjot::engine::Para| -> Vec<(String, Vec<&str>)> {
        p.runs
            .iter()
            .map(|r| (r.text.clone(), r.style.clone()))
            .collect()
    };
    let said = text
        .iter()
        .find(|p| p.plain().contains("Mind the"))
        .unwrap();
    assert_eq!(said.plain(), "*Old* Tobin: Mind the rope, Nell.");
    assert!(styled(said).contains(&("rope".into(), vec!["italic"])));
    let entry = text.iter().find(|p| p.plain().contains("entry")).unwrap();
    assert!(styled(entry).contains(&("bold".into(), vec!["bold"])));
    let motto = text
        .iter()
        .find(|p| p.plain().starts_with("Motto"))
        .unwrap();
    assert_eq!(motto.plain(), "Motto: *never* literal.");
    let ir = build(READ).unwrap();
    assert!(has(
        &ir,
        &json(
            r#"{"text": ["She sailed on the ", {"style": ["italic"], "parts": ["Endeavour"]}, "."]}"#
        )
    ));
}
