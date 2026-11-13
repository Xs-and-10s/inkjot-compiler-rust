//! End-to-end tests for `inkjot build`: the JSON IR.

use inkjot::json::{self, Value};
use inkjot::{build, check};

const NAMES: [&str; 3] = [
    "ex1-the-night-train",
    "ex2-the-ninefold-cell",
    "ex3-the-fall-of-the-old-regime",
];

fn example(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/examples/{name}.twee",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn ir(name: &str) -> Value {
    build(&example(name))
        .unwrap_or_else(|report| panic!("{name} did not build: {:#?}", report.diags))
}

/// The passage called `name` in an IR.
fn passage<'a>(ir: &'a Value, name: &str) -> &'a Value {
    ir.get("passages")
        .and_then(Value::as_arr)
        .unwrap()
        .iter()
        .find(|p| p.get("name").and_then(Value::as_str) == Some(name))
        .unwrap_or_else(|| panic!("no passage `{name}`"))
}

/// Every node of `v`, depth first.
fn walk<'a>(v: &'a Value, visit: &mut impl FnMut(&'a Value)) {
    visit(v);
    match v {
        Value::Arr(items) => items.iter().for_each(|i| walk(i, visit)),
        Value::Obj(pairs) => pairs.iter().for_each(|(_, i)| walk(i, visit)),
        _ => {}
    }
}

#[test]
fn the_examples_build_to_valid_json() {
    for name in NAMES {
        let text = ir(name).to_string();
        let reread =
            json::parse(&text).unwrap_or_else(|e| panic!("{name}: the IR is not valid JSON: {e}"));
        assert_eq!(
            reread,
            ir(name),
            "{name}: the IR changed when printed and read back"
        );
    }
}

#[test]
fn building_is_deterministic() {
    for name in NAMES {
        assert_eq!(ir(name).to_string(), ir(name).to_string(), "{name}");
    }
}

/// The checked-in IR under `examples/ir/` is what the compiler produces now.
/// A change to the IR's shape shows up as a diff to these files in review.
/// To accept a change: `UPDATE_IR=1 cargo test --test build`.
#[test]
fn the_checked_in_ir_is_current() {
    for name in NAMES {
        let path = format!("{}/examples/ir/{name}.json", env!("CARGO_MANIFEST_DIR"));
        let fresh = format!("{}\n", ir(name));
        if std::env::var_os("UPDATE_IR").is_some() {
            std::fs::write(&path, &fresh).unwrap();
        }
        let saved = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            saved == fresh,
            "{path} is out of date; run `UPDATE_IR=1 cargo test --test build`"
        );
    }
}

#[test]
fn nothing_is_left_unparsed_and_no_note_survives() {
    for name in NAMES {
        let ir = ir(name);
        walk(&ir, &mut |v| {
            assert!(
                v.get("unparsed").is_none(),
                "{name}: an expression was not lowered: {v}"
            );
            if let Some(text) = v.as_str() {
                assert!(
                    !text.contains("Paper only for now"),
                    "{name}: an author's note reached the IR"
                );
            }
        });
    }
}

#[test]
fn source_with_errors_is_not_built() {
    let broken = example(NAMES[0]).replacen("~ take notebook", "~ take notebok", 1);
    let report = build(&broken).unwrap_err();
    assert_eq!(report.errors(), check(&broken).errors());
    assert!(report.errors() > 0);
}

/// The Link the working doc uses as its example of the IR: Rook's priced
/// question, in the shared `dining_table` passage.
#[test]
fn a_link_is_expanded_with_its_acts_defaults() {
    let ir = ir(NAMES[0]);
    let table = passage(&ir, "dining_table");
    let link = table
        .get("body")
        .and_then(Value::as_arr)
        .unwrap()
        .iter()
        .find(|n| n.get("id").and_then(Value::as_str) == Some("dining_table#4"))
        .unwrap();
    let expected = r#"{
        "type": "link",
        "id": "dining_table#4",
        "lib": "dining_table",
        "label": "Ask Mrs. Ardent about her letter",
        "target": "dining_table:letter",
        "kind": "act",
        "act": "ask",
        "arg": null,
        "cost": [{"vults": 1}, {"vults": 2, "when": {"is": ["me", "rook"]}}],
        "scope": "private",
        "capacity": null,
        "clash": "none",
        "check": null,
        "when": null,
        "hidden_until": {"knows": ["me", "two_hands"]},
        "for": null,
        "effects": [],
        "ends_turn": false,
        "overrides": ["cost"]
    }"#;
    assert_eq!(link, &json::parse(expected).unwrap());
}

#[test]
fn a_shared_act_brings_its_scope_and_the_default_clash() {
    let ir = ir(NAMES[0]);
    let link = passage(&ir, "dining_table")
        .get("body")
        .and_then(Value::as_arr)
        .unwrap()[1]
        .clone();
    assert_eq!(link.get("act").and_then(Value::as_str), Some("take"));
    assert_eq!(link.get("arg").and_then(Value::as_str), Some("cufflink"));
    assert_eq!(link.get("scope").and_then(Value::as_str), Some("shared"));
    assert_eq!(link.get("capacity").and_then(Value::as_u64), Some(1));
    assert_eq!(
        link.get("clash").and_then(Value::as_str),
        Some("random_winner")
    );
}

#[test]
fn a_combo_costs_the_sum_of_its_acts() {
    let ir = ir(NAMES[1]);
    let lance = ir.get("acts").and_then(|a| a.get("ember_lance")).unwrap();
    let cost = lance.get("cost").and_then(Value::as_arr).unwrap();
    assert_eq!(cost[0].get("vults").and_then(Value::as_u64), Some(30)); // spark + gust + spark
}

#[test]
fn speech_keeps_its_speaker_direction_and_inserts() {
    let ir = ir(NAMES[2]);
    let body = passage(&ir, "interval")
        .get("body")
        .and_then(Value::as_arr)
        .unwrap();
    let speech = &body[0];
    assert_eq!(speech.get("type").and_then(Value::as_str), Some("speech"));
    assert_eq!(speech.get("who").and_then(Value::as_str), Some("aunt"));
    // `{leo.aka}` is lowered to a value insert, not left as text.
    let insert = json::parse(r#"{"value": {"get": ["leo", "aka"]}}"#).unwrap();
    let mut found = false;
    walk(speech, &mut |v| found |= *v == insert);
    assert!(found, "{speech}");
}

#[test]
fn yes_and_no_become_booleans() {
    let ir = ir(NAMES[0]);
    let header = passage(&ir, "berth:shoes")
        .get("header")
        .and_then(Value::as_arr)
        .unwrap();
    let set = header
        .iter()
        .find(|s| s.to_string().contains(r#""do": "set""#))
        .unwrap();
    assert_eq!(
        set.get("effect").and_then(|e| e.get("value")),
        Some(&Value::Bool(true))
    );
}

/// Phrases the IR may leave untyped. All are the Ex 2 duel's spell
/// vocabulary, deferred to Inkjot v0.6.
const UNTYPED_V06: &[&str] = &[
    "absorbs spells",
    "cracks glass",
    "interrupts flame, frost",
    "needs two to hold it",
    "parries flame",
    "shields target from the next spell",
    "thaws ice",
];

#[test]
fn every_phrase_is_typed_except_the_v06_duel() {
    for name in NAMES {
        let ir = ir(name);
        walk(&ir, &mut |v| {
            for other in v.get("other").and_then(Value::as_arr).unwrap_or(&[]) {
                let text = other.get("text").and_then(Value::as_str).unwrap_or("");
                assert!(
                    UNTYPED_V06.contains(&text),
                    "{name}: `{text}` was left untyped in {v}"
                );
            }
        });
    }
}

/// Find the config statement that declares `id`.
fn declared<'a>(ir: &'a Value, id: &str) -> &'a Value {
    ir.get("config")
        .and_then(Value::as_arr)
        .unwrap()
        .iter()
        .find(|s| s.get("id").and_then(Value::as_str) == Some(id))
        .unwrap_or_else(|| panic!("nothing declares `{id}`"))
}

#[test]
fn declarations_are_typed() {
    let ir = ir(NAMES[0]);
    let expected = r#"{"declare": "npc", "id": "marsh", "label": "Colonel Marsh", "tags": ["witness"],
        "moves": [], "in": "sleeper", "asleep_during": {"range": [2, 4], "inclusive": true},
        "wakes_on": "loud", "stays_awake": 1}"#;
    assert_eq!(declared(&ir, "marsh"), &json::parse(expected).unwrap());

    let expected = r#"{"declare": "way", "id": "vestibule", "from": "baggage", "to": "platform",
        "one_way": true, "for": ["npc"], "open_when": {">=": [{"get": ["clock", "now"]}, 1]},
        "tags": ["loud"], "shown_as": "the vestibule door"}"#;
    assert_eq!(declared(&ir, "vestibule"), &json::parse(expected).unwrap());
}

#[test]
fn stats_on_characters_and_list_entries_are_typed() {
    let ex2 = ir(NAMES[1]);
    let stats = json::parse(r#"{"wits": 4, "might": 1, "hp": 1}"#).unwrap();
    assert_eq!(declared(&ex2, "roland").get("stats"), Some(&stats));

    let ex3 = ir(NAMES[2]);
    let holds = declared(&ex3, "the_list")
        .get("holds")
        .and_then(Value::as_arr)
        .unwrap();
    assert_eq!(
        holds[0],
        json::parse(r#"{"id": "vacuum", "label": "the vacuum"}"#).unwrap()
    );
    assert_eq!(
        holds[4],
        json::parse(r#"{"id": "leo", "mark": "pending"}"#).unwrap()
    );
}
