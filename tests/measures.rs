//! `measure`: values an Inklink reports when a reading ends, so its Inker
//! can tune it from real readings (Ex 7: who won, at what size, in how
//! many nights).

use inkjot::engine::{Inklink, Start};
use inkjot::json::Value;

const MEASURED: &str = r#":: StoryTitle
Measured

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "room"}

:: Inklink [config]
kind: game
mode: solo
pov: ann
character ann: "Ann" ~ selectable
npc leo: "Leo"
state nights: 0
state side: "town"
measure players: count(person)
measure nights: nights
measure won_by: side
measure long: nights > 1

:: room
raise nights 1
--
Night {nights}.
> [[Again->room]] ~ when nights < 3
> [[The end->end]]

:: end [finale]
The town wins.
"#;

#[test]
fn measures_are_reported_at_the_ending() {
    let ir = inkjot::build(MEASURED).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    let inklink = Inklink::from_ir(&ir).unwrap();
    let mut state = inklink.start(1, &Start::default()).unwrap();
    // Nothing is measured before the end.
    assert!(state.events().iter().all(|e| e.kind != "measured"));
    for label in ["Again", "Again", "The end"] {
        let id = state
            .view()
            .links
            .iter()
            .find(|l| l.label.plain() == label)
            .unwrap()
            .id
            .clone();
        state = state.take_link(&id).unwrap();
    }
    let measured = state
        .events()
        .iter()
        .find(|e| e.kind == "measured")
        .expect("a measured event at the ending");
    assert_eq!(
        measured.data.get("ending").and_then(Value::as_str),
        Some("end")
    );
    let values = measured.data.get("values").unwrap();
    assert_eq!(values.get("players").unwrap().to_string(), "2");
    assert_eq!(values.get("nights").unwrap().to_string(), "3");
    assert_eq!(values.get("won_by").and_then(Value::as_str), Some("town"));
    assert_eq!(values.get("long"), Some(&Value::Bool(true)));
    // In the order written.
    let keys: Vec<&str> = match values {
        Value::Obj(pairs) => pairs.iter().map(|(k, _)| k.as_str()).collect(),
        _ => panic!("values is an object"),
    };
    assert_eq!(keys, ["players", "nights", "won_by", "long"]);
}

#[test]
fn a_measure_is_about_the_whole_reading() {
    let errors = |src: &str| -> Vec<String> {
        inkjot::check(src)
            .diags
            .into_iter()
            .filter(|d| d.severity == inkjot::Severity::Error)
            .map(|d| d.msg)
            .collect()
    };
    assert_eq!(errors(MEASURED), Vec::<String>::new());
    let mine = MEASURED.replace("measure nights: nights", "measure nights: me.nights");
    assert!(errors(&mine).iter().any(|e| e.contains("not one Linker")));
    let shapeless = MEASURED.replace("measure nights: nights", "measure nights");
    assert!(errors(&shapeless)
        .iter()
        .any(|e| e.contains("measure name: expression")));
}

#[test]
fn a_test_can_expect_the_measures() {
    let src = format!(
        "{MEASURED}\n:: t [test]\nstart: room\ndo: \"The end\"\nexpect events: ending_reached, measured\n"
    );
    let ir = inkjot::build(&src).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    for outcome in inkjot::run::test::run_all(&src, &ir).unwrap() {
        assert!(outcome.passed(), "{:?}", outcome.failures);
    }
}
