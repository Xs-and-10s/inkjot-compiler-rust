//! Milestone 20a, targets and filters: `where:` with `it`, `all(…)`,
//! `~ at` (one Link per target, with `target`), acts that carry their
//! conditions and effects to every Link that names them, Link groups,
//! effects on every member of a set, and the config's `let`s and `get`s in
//! every passage.

use inkjot::engine::{Inklink, Para, Start, State};
use proptest::prelude::*;

const HALL: &str = r#":: StoryTitle
Hall

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "hall"}

:: Inklink [config]
kind: game
mode: solo
pov: ann
default cost: salt
resource salt: 5
character ann: "Ann" ~ selectable ~ in hall
place hall: "the hall"
place yard: "the yard" ~ outdoors
place cellar: "the cellar"
way door: hall - yard
way stair: hall - cellar
archetype wick ~ catches
item lamp: "a lamp" ~ in hall ~ like wick
~ state lit: no
item candle: "a candle" ~ in hall ~ like wick
~ state lit: no
item torch: "a torch" ~ in yard ~ like wick
~ state lit: no
item key: "a key" ~ held by ann
npc leo: "Leo" ~ in hall
fact primer: "Kindle: a flame at your fingertip."
fact notes: "Gust: one breath of wind."
fact greater: "Kindle and gust, said as one."
act kindle: cost 1 ~ learned from primer
~ at item here where: it.catches and not it.lit
~ then set target.lit: yes
act gust: cost 1 ~ learned from notes
combo firebrand: kindle, gust ~ learned from greater
get unlit: all(item, where: it.catches and not it.lit)
get lit_here: count(item, in: here, where: it.lit)

:: hall
Unlit: {unlit}. Lit here: {lit_here}. Next door: {all(place, where: it.next_to(here))}.
> [[Read the primer->hall:read]] ~ once
> [[Light every wick->hall:all]]

> ~ for ann ~ when salt > 0
> [[Wave at {target}->hall:wave]] ~ at person here where: it != me
> [[Look at {target}->hall:look]] ~ at item where: it.held_by(me) or it.like(wick)

> [[Firebrand->hall]] ~ firebrand

:: hall:read [act]
learn primer
--
You read it.

:: hall:wave [act]
--
You wave at {target}.

:: hall:look [act]
--
{target}: {if target.lit then "burning" else "dark"}.

:: hall:all [act]
let wicks: all(item, where: it.like(wick))
set wicks.lit: yes
--
Every wick is burning.

:: spellbook [lib everywhere]
> ~ for ann
> [[Kindle {target}->spellbook:kindle]] ~ kindle

:: spellbook:kindle [act]
--
A flame stands up on {target}.
"#;

fn start(src: &str) -> State {
    let ir = inkjot::build(src).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    Inklink::from_ir(&ir)
        .unwrap()
        .start(1, &Start::default())
        .unwrap()
}

fn labels(s: &State) -> Vec<String> {
    s.view().links.iter().map(|l| l.label.plain()).collect()
}

fn take(s: &State, label: &str) -> State {
    let id = s
        .view()
        .links
        .iter()
        .find(|l| l.label.plain() == label)
        .unwrap_or_else(|| panic!("no Link `{label}`; there are {:?}", labels(s)))
        .id
        .clone();
    s.take_link(&id).unwrap()
}

fn text(s: &State) -> Vec<String> {
    s.view().text.iter().map(Para::plain).collect()
}

fn errors(src: &str) -> Vec<String> {
    inkjot::check(src)
        .diags
        .into_iter()
        .filter(|d| d.severity == inkjot::Severity::Error)
        .map(|d| d.msg)
        .collect()
}

#[test]
fn it_checks_clean() {
    assert_eq!(errors(HALL), Vec::<String>::new());
}

#[test]
fn where_filters_with_it() {
    let s = start(HALL);
    assert_eq!(
        text(&s)[0],
        "Unlit: a candle, a lamp and a torch. Lit here: 0. Next door: the cellar and the yard."
    );
    // `set wicks.lit: yes`, `wicks` a list: every one of them.
    let lit = take(&s, "Light every wick");
    assert_eq!(text(&lit), ["Every wick is burning."]);
    assert_eq!(text(&take(&lit, "Look at a torch")), ["A torch: burning."]);
    assert_eq!(text(&take(&lit, "Look at a key")), ["A key: dark."]);
}

#[test]
fn a_link_at_a_kind_is_one_link_per_target() {
    let s = start(HALL);
    let all = labels(&s);
    assert!(all.contains(&"Wave at Leo".to_string()), "{all:?}");
    assert!(!all.iter().any(|l| l == "Wave at Ann"), "not at me");
    // `held_by(me) or like(wick)`, anywhere: the key, and all three wicks.
    let looks: Vec<&String> = all.iter().filter(|l| l.starts_with("Look at")).collect();
    assert_eq!(looks.len(), 4, "{looks:?}");
    // The target reaches the passage the Link goes to.
    assert_eq!(text(&take(&s, "Look at a torch")), ["A torch: dark."]);
    assert_eq!(text(&take(&s, "Wave at Leo")), ["You wave at Leo."]);
}

#[test]
fn an_act_carries_its_target_conditions_and_effects() {
    let s = start(HALL);
    // Not learned yet: no Kindle, from the spellbook offered everywhere.
    assert!(!labels(&s).iter().any(|l| l.starts_with("Kindle")));
    let s = take(&s, "Read the primer");
    let kindles: Vec<String> = labels(&s)
        .into_iter()
        .filter(|l| l.starts_with("Kindle"))
        .collect();
    // Only the unlit wicks here: not the torch in the yard.
    assert_eq!(kindles, ["Kindle a candle", "Kindle a lamp"]);
    let s = take(&s, "Kindle a lamp");
    assert_eq!(text(&s), ["A flame stands up on a lamp."]);
    let kindles: Vec<String> = labels(&s)
        .into_iter()
        .filter(|l| l.starts_with("Kindle"))
        .collect();
    assert_eq!(kindles, ["Kindle a candle"], "the lamp is lit now");
}

#[test]
fn a_combo_needs_its_acts_known_too() {
    let src = HALL.replace(
        "> [[Read the primer->hall:read]] ~ once",
        "> [[Read the primer->hall:read]] ~ once\n> [[Read the notes->hall]] ~ then learn notes ~ then learn greater",
    );
    let s = start(&src);
    assert!(!labels(&s).contains(&"Firebrand".to_string()));
    let s = take(&s, "Read the notes");
    assert!(
        !labels(&s).contains(&"Firebrand".to_string()),
        "kindle is not known yet"
    );
    let s = take(&s, "Read the primer");
    assert!(labels(&s).contains(&"Firebrand".to_string()));
}

#[test]
fn a_link_group_lends_its_clauses_until_a_blank_line() {
    let src = HALL.replace("resource salt: 5", "resource salt: 0");
    let s = start(&src);
    let all = labels(&s);
    // `~ when salt > 0` held back the group's Links, not the ones after it.
    assert!(
        !all.iter()
            .any(|l| l.starts_with("Wave") || l.starts_with("Look")),
        "{all:?}"
    );
    assert!(all.contains(&"Read the primer".to_string()));
}

#[test]
fn the_new_words_are_checked() {
    let outside = HALL.replace(
        "get lit_here: count(item, in: here, where: it.lit)",
        "get lit_here: count(item, in: here) + it.lit",
    );
    assert!(errors(&outside)
        .iter()
        .any(|e| e.contains("`it` is not declared")));
    let bad_kind = HALL.replace("~ at person here where: it != me", "~ at persn here");
    assert!(errors(&bad_kind)
        .iter()
        .any(|e| e.contains("names a kind of thing")));
    let bad_word = HALL.replace("~ at person here where: it != me", "~ at person there");
    assert!(errors(&bad_word).iter().any(|e| e.contains("not `there`")));
    let no_link = HALL.replace("> ~ for ann ~ when salt > 0", "> for ann");
    assert!(errors(&no_link)
        .iter()
        .any(|e| e.contains("a Link group is `> ~ clause`")));
}

/// A set of things with a flag each: what `count`, `all` and `~ at` find
/// is exactly those whose flag holds.
fn hall_of(flags: &[bool]) -> String {
    let mut items = String::new();
    for (i, on) in flags.iter().enumerate() {
        items.push_str(&format!(
            "item t{i}: \"thing {i}\" ~ in hall\n~ state on: {}\n",
            if *on { "yes" } else { "no" }
        ));
    }
    format!(
        r#":: StoryTitle
Many

:: StoryData
{{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "hall"}}

:: Inklink [config]
kind: game
mode: solo
pov: ann
character ann: "Ann" ~ selectable ~ in hall
place hall: "the hall"
{items}
:: hall
{{count(item, where: it.on)}} on; {{count(item, where: not it.on)}} off.
> [[Switch off {{target}}->hall:off]] ~ at item here where: it.on

:: hall:off [act]
set target.on: no
--
Off.
"#
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// `count(…, where:)` and the Links `~ at … where:` agree with the
    /// flags, and taking a Link changes exactly its target.
    #[test]
    fn filters_and_targets_agree(flags in prop::collection::vec(any::<bool>(), 1..8)) {
        let src = hall_of(&flags);
        let s = start(&src);
        let on = flags.iter().filter(|f| **f).count();
        prop_assert_eq!(&text(&s)[0], &format!("{on} on; {} off.", flags.len() - on));
        let links = labels(&s);
        prop_assert_eq!(links.iter().filter(|l| l.starts_with("Switch off")).count(), on);
        if on > 0 {
            let first = links.iter().find(|l| l.starts_with("Switch off")).unwrap().clone();
            let s2 = take(&s, &first);
            let left = labels(&s2).iter().filter(|l| l.starts_with("Switch off")).count();
            prop_assert_eq!(left, on - 1);
        }
    }
}
