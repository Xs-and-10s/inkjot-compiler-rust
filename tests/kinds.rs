//! The built-in kinds, `person`, `living` and `linker`, wherever a kind is
//! read: in `count`, `one`, `nearest`, and goals.

use inkjot::run::test::run_all;

const ROOM: &str = r#":: StoryTitle
Kinds

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "hall"}

:: Inklink [config]
kind: story
mode: solo
pov: ann
character ann: "Ann" ~ selectable
npc bo: "Bo" ~ in hall
npc cy: "Cy" ~ in hall ~ lifeless
npc di: "Di" ~ in yard
item lamp: "a lamp" ~ in hall
place hall: "the hall"
place yard: "the yard"

:: hall
Here.

:: counts [test]
start: hall
expect: count(person, in: here) == 3 ~ ((Ann, Bo and Cy; not the lamp))
expect: count(person) == 4
expect: count(living, in: here) == 2 ~ ((Cy is lifeless))
expect: count(linker, in: here) == 1
"#;

#[test]
fn person_is_any_character_or_npc() {
    let ir = inkjot::build(ROOM).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    for outcome in run_all(ROOM, &ir).unwrap() {
        assert!(
            outcome.passed(),
            "{:#?}\n{}",
            outcome.failures,
            outcome.transcript.join("\n")
        );
    }
}
