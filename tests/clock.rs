//! Milestone 21c, time: periods and named clocks, `wait until`, `turn ends
//! when`, and Links timed in real time.

use inkjot::period::Calendar;
use inkjot::run::{test::run_all, Run, Story};
use proptest::prelude::*;
use std::sync::Arc;

/// Two days of watches, a turn an action: a dawn, two turns of day, a dusk.
const WATCH: &str = r#":: StoryTitle
Watch

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "camp"}

:: Inklink [config]
kind: game
mode: solo
pov: ann
period watch: dawn 1, day 2, dusk 1
period weekday: monday, tuesday ~ of watch
clock: monday.dawn ..= tuesday.dusk ~ each action
character ann: "Ann" ~ selectable ~ in camp
place camp: "the camp"
state dawns: 0
state days_ended: 0
state turns: 0
state tuesdays: 0
state signalled: no
on watch begins dawn
~ then raise dawns 1
on watch ends day
~ then raise days_ended 1
on weekday begins tuesday
~ then raise tuesdays 1
on turn begins
~ then raise turns 1

:: camp
It is {clock.weekday}, {clock.watch}.
> [[Keep watch->camp]]
> [[Sleep until dawn->camp]] ~ then wait until dawn
> [[Rest till Tuesday dusk->camp]] ~ then wait until tuesday.dusk
> [[Light the fire->camp]] ~ when clock.watch >= day and clock.watch < dusk
> [[Signal->camp]] ~ after 30s ~ then set signalled: yes

:: the_clock_reads_in_names [test]
start: camp
expect text: "It is monday, dawn."
expect: clock.now == 0 and turns == 0 and dawns == 0
do: "Keep watch"
expect: clock.watch == day and clock.weekday == monday
expect: turns == 1
(((
Each action is a turn: what it shows is told before its turn ends.
)))
do: "Keep watch"
expect text: "It is monday, day."

:: names_compare_in_their_order [test]
start: camp
expect no link: "Light the fire"
do: "Keep watch"
expect link: "Light the fire"
do: "Keep watch"
do: "Keep watch"
expect: clock.watch == dusk
expect no link: "Light the fire"

:: beginnings_and_endings_fire [test]
start: camp
do: "Keep watch"
do: "Keep watch"
expect: days_ended == 0
do: "Keep watch"
expect: days_ended == 1 and dawns == 0
do: "Keep watch"
expect: dawns == 1 and tuesdays == 1
expect: clock.weekday == tuesday and clock.watch == dawn

:: waiting_ends_each_turn_between [test]
start: camp
do: "Keep watch"
do: "Sleep until dawn"
expect: clock.now == 4
expect: turns == 4 and days_ended == 1 and dawns == 1
expect text: "It is tuesday, dawn."

:: waiting_for_a_path [test]
start: camp
do: "Rest till Tuesday dusk"
expect: clock.now == 7
expect text: "It is tuesday, dusk."

:: a_timed_link_waits_for_its_time [test]
start: camp
expect link: "Signal"
do: wait 20s
expect: not signalled
do: wait 10s
expect: signalled
expect: clock.now == 1

:: given_can_wait_too [test]
start: camp
given: wait until tuesday.day
expect: clock.now == 5 and clock.weekday == tuesday
"#;

/// Two heroes, two breaths each a turn: the turn ends when both are done.
const YARD: &str = r#":: StoryTitle
Yard

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "yard"}

:: Inklink [config]
kind: game
mode: solo
pov: ann, bo ~ switch freely
clock: 0.. ~ shown as "turn"
~ turn ends when pov.all.spent or pov.all.passed
start: yard for ann, yard for bo
default cost: breath
resource breath: 2 ~ per character ~ refill each chronon
character ann: "Ann" ~ selectable ~ in yard
character bo: "Bo" ~ in yard
place yard: "the yard"
state turns_begun: 0
on turn begins
~ then raise turns_begun 1

:: yard
The yard.
> [[Dig->here]] ~ cost 1
> [[Switch->here]] ~ switch
> [[Done->here]] ~ pass
> [[Give up->here]] ~ pass ~ ends turn

:: one_pass_is_not_enough [test]
start: yard ~ as ann
do: "Done"
expect: ann.passed and not bo.passed
expect: clock.now == 0
do: "Switch"
do: "Done"
expect: clock.now == 1 and turns_begun == 1
expect: not ann.passed and not bo.passed

:: both_spent_ends_it [test]
start: yard ~ as ann
do: "Dig"
do: "Dig"
expect: ann.spent and clock.now == 0
do: "Switch"
do: "Dig"
do: "Dig"
expect: clock.now == 1
expect: ann.breath == 2 and bo.breath == 2

:: ends_turn_ends_it_at_once [test]
start: yard ~ as ann
do: "Done"
do: "Give up"
expect: clock.now == 1
expect: not ann.passed
"#;

fn run(src: &str) -> Vec<inkjot::run::test::Outcome> {
    let ir = inkjot::build(src).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    run_all(src, &ir).unwrap()
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
fn they_check_clean() {
    assert_eq!(errors(WATCH), Vec::<String>::new());
    assert_eq!(errors(YARD), Vec::<String>::new());
}

#[test]
fn their_tests_pass() {
    for src in [WATCH, YARD] {
        for o in run(src) {
            assert!(
                o.passed(),
                "`{}`: {:#?}\n{}",
                o.name,
                o.failures,
                o.transcript.join("\n")
            );
        }
    }
}

#[test]
fn the_new_words_are_checked() {
    let says = |from: &str, to: &str, what: &str| {
        let bad = WATCH.replace(from, to);
        assert!(
            errors(&bad).iter().any(|e| e.contains(what)),
            "`{to}` should say {what:?}: {:?}",
            errors(&bad)
        );
    };
    says(
        "on watch begins dawn",
        "on wotch begins dawn",
        "`wotch` is not a declared period",
    );
    says(
        "on watch begins dawn",
        "on watch begins dusky",
        "`dusky` is not one of the `watch`'s names",
    );
    says(
        "on turn begins",
        "on turn begins dawn",
        "a turn has no names",
    );
    says(
        "wait until dawn",
        "wait until dusky",
        "`dusky` is not a time",
    );
    says(
        "wait until dawn",
        "wait for dawn",
        "`wait` is written `wait until`",
    );
    says("~ of watch", "~ of wotch", "not a period");
    says(
        "dawn 1, day 2, dusk 1",
        "dawn 1, day, dusk 1",
        "needs a length",
    );
    says(
        "do: wait 20s",
        "do: wait a while",
        "`do: wait` takes a time",
    );
    says("~ after 30s", "~ after 30 turns", "takes a real time");
}

#[test]
fn periods_are_in_the_ir() {
    let ir = inkjot::build(WATCH).unwrap();
    let text: String = ir.to_string().split_whitespace().collect();
    for want in [
        r#""declare": "period", "id": "watch", "names": ["dawn", "day", "dusk"], "lengths": [1, 2, 1]"#,
        r#""declare": "period", "id": "weekday", "names": ["monday", "tuesday"], "of": "watch""#,
        r#""on": "begins", "period": "watch", "timing": "after", "what": ["dawn"]"#,
        r#""on": "ends", "period": "watch""#,
        r#""on": "begins", "period": "turn""#,
        r#""do": "wait", "until": "dawn""#,
        r#""after": "30s""#,
    ] {
        let want: String = want.split_whitespace().collect();
        assert!(text.contains(&want), "the IR should have {want}");
    }
}

/// A run of WATCH, started.
fn watch() -> Run {
    let ir = inkjot::build(WATCH).unwrap();
    let mut run = Run::new(Arc::new(Story::new(&ir).unwrap()), 1);
    run.start(None, None).unwrap();
    run
}

fn eval(run: &mut Run, e: &str) -> String {
    run.eval(&inkjot::ir::lower_expression(e))
        .unwrap()
        .to_string()
}

fn take(run: &mut Run, label: &str) {
    let id = run
        .choices()
        .unwrap()
        .into_iter()
        .find(|(_, l)| l == label)
        .unwrap_or_else(|| panic!("no Link `{label}`"))
        .0;
    run.choose(&id).unwrap();
}

#[test]
fn a_timed_link_cannot_be_taken_early() {
    let mut run = watch();
    let id = run
        .choices()
        .unwrap()
        .into_iter()
        .find(|(_, l)| l == "Signal")
        .unwrap()
        .0;
    let err = run.choose(&id).unwrap_err();
    assert!(err.contains("taken by itself"), "{err}");
    // A turn ending starts its time again.
    run.wait(20_000).unwrap();
    take(&mut run, "Keep watch");
    run.wait(20_000).unwrap();
    assert_eq!(eval(&mut run, "signalled"), "no");
    run.wait(10_000).unwrap();
    assert_eq!(eval(&mut run, "signalled"), "yes");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Whatever is done, `clock.watch` and `clock.weekday` are what the
    /// calendar says of the turn; every dawn after the first fired its
    /// handler once; and the clock never passes its last stop.
    #[test]
    fn the_clock_agrees_with_the_calendar(acts in prop::collection::vec(0usize..3, 0..12)) {
        let cal = Calendar::parse(
            "period watch: dawn 1, day 2, dusk 1\nperiod weekday: monday, tuesday ~ of watch".lines(),
        ).unwrap();
        let mut run = watch();
        for a in acts {
            let label = ["Keep watch", "Sleep until dawn", "Light the fire"][a];
            let offered = run.choices().unwrap().iter().any(|(_, l)| l == label);
            if offered {
                take(&mut run, label);
            }
            let now = run.now();
            prop_assert!(now <= 7);
            let names = cal.at(now);
            prop_assert_eq!(eval(&mut run, "clock.weekday"), names[0].1.clone());
            prop_assert_eq!(eval(&mut run, "clock.watch"), names[1].1.clone());
            let dawns = (1..=now).filter(|t| cal.begins_on("dawn", *t)).count();
            prop_assert_eq!(eval(&mut run, "dawns"), dawns.to_string());
            prop_assert_eq!(eval(&mut run, "turns"), now.to_string());
        }
    }
}

/// The engine: a timed Link says how long it has left, cannot be taken,
/// and is taken by itself once enough time is said to have passed.
#[test]
fn the_engine_waits() {
    use inkjot::engine::{Action, Inklink, Start};
    let ir = inkjot::build(WATCH).unwrap();
    let s = Inklink::from_ir(&ir)
        .unwrap()
        .start(1, &Start::default())
        .unwrap();
    let signal = |s: &inkjot::engine::State| {
        s.view()
            .links
            .iter()
            .find(|l| l.label.plain() == "Signal")
            .cloned()
            .unwrap()
    };
    assert_eq!(signal(&s).due_in, Some(30_000));
    let keep = s
        .view()
        .links
        .iter()
        .find(|l| l.label.plain() == "Keep watch")
        .unwrap();
    assert_eq!(keep.due_in, None);
    let refused = s.take_link(&signal(&s).id).unwrap_err();
    assert_eq!(refused.kind.code(), "not_offered");
    let s = s.wait(20_000).unwrap();
    assert_eq!(signal(&s).due_in, Some(10_000));
    assert_eq!(s.view().now, 0);
    let s = s.act(&Action::Wait(10_000)).unwrap();
    assert_eq!(s.view().now, 1, "the Signal was taken, and that is a turn");
    assert_eq!(signal(&s).due_in, Some(30_000));
}
