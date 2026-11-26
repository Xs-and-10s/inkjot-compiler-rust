//! Milestone 21b, cards: a character for each Linker, decks built and
//! shuffled, piles dealt to, cards moved between them.

use inkjot::run::{test::run_all, Run, Story};
use proptest::prelude::*;
use std::sync::Arc;

/// A card table for four to six, as Ex 7 deals one: roles face down,
/// keepsakes face up, and a pile of loot each.
const TABLE: &str = r#":: StoryTitle
Table

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "table"}

:: Inklink [config]
kind: game
mode: cohort
cohort: 4..=6
character guest: "a guest" ~ selectable ~ one per linker
~ state role: nobody
get players: count(guest)
get members: max(1, players / 4)
deck roles
~ card mafia: members
~ card doctor: 1
~ card townsfolk: rest
pile hand ~ per guest ~ private ~ face down ~ shown as "your card"
deck keepsakes
~ card locket, compass, cameo, hatpin, tin_soldier, music_box
pile keepsake ~ per guest ~ face up
pile loot ~ per guest ~ private
get haul: all(guest, where: it.role == mafia).loot

:: table
{players} at the table.
> [[Deal->dealt]] ~ for cohort ~ then shuffle roles ~ then deal 1 from roles to each hand ~ then shuffle keepsakes ~ then deal 1 from keepsakes to each keepsake

:: dealt
set me.role: top of my hand
--
You are {me.role}. Your keepsake: {top of my keepsake}.
{list hand of each guest}
> [[Steal a keepsake->stolen]] ~ at guest where: it != me and count(it.keepsake) > 0

:: stolen
let taken: top of target.keepsake
set taken.owner: target
move taken to me.loot
--
You take {taken}. In your loot: {count(my loot)}.
> [[Give it all back->returned]]

:: returned
let found: me.loot
move each found to it.owner.keepsake
--
Loot: {count(my loot)}. Haul: {count(haul)}.

:: a_table_of_four [test]
start: table ~ as guest, guest, guest, guest
expect: players == 4 and members == 1
expect text: "4 at the table."
do: "Deal"
expect: count(my hand) == 1 and count(my keepsake) == 1
expect: count(roles) == 0

:: a_table_of_six_has_one_mafia [test]
seed: 7
start: table ~ as guest, guest, guest, guest, guest, guest
do: "Deal"
expect: count(all(guest, where: it.role == mafia)) <= 1
expect: count(keepsakes) == 0

:: a_stolen_keepsake_goes_home [test]
start: table ~ as guest, guest, guest, guest
do: "Deal"
expect: count(my loot) == 0
"#;

fn run(src: &str) -> Vec<inkjot::run::test::Outcome> {
    let ir = inkjot::build(src).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    run_all(src, &ir).unwrap()
}

#[test]
fn it_checks_clean() {
    let errors: Vec<String> = inkjot::check(TABLE)
        .diags
        .into_iter()
        .filter(|d| d.severity == inkjot::Severity::Error)
        .map(|d| d.msg)
        .collect();
    assert_eq!(errors, Vec::<String>::new());
}

#[test]
fn the_tables_tests_pass() {
    for o in run(TABLE) {
        assert!(
            o.passed(),
            "`{}`: {:#?}\n{}",
            o.name,
            o.failures,
            o.transcript.join("\n")
        );
    }
}

#[test]
fn the_new_words_are_checked() {
    let errors = |src: &str| -> Vec<String> {
        inkjot::check(src)
            .diags
            .into_iter()
            .map(|d| d.msg)
            .collect()
    };
    let bad = TABLE.replace("shuffle roles", "shuffle rolls");
    assert!(errors(&bad)
        .iter()
        .any(|e| e.contains("`rolls` is not a declared deck")));
    let bad = TABLE.replace("deal 1 from roles to each hand", "deal 1 to each hand");
    assert!(errors(&bad)
        .iter()
        .any(|e| e.contains("`deal 1 from deck to each pile`")));
    let bad = TABLE.replace("pile loot ~ per guest", "pile loot ~ per gest");
    assert!(errors(&bad).iter().any(|e| e.contains("`gest` is not one")));
    let bad = TABLE.replace("card mafia: members", "card mafia: membres");
    assert!(errors(&bad).iter().any(|e| e.contains("`membres`")));
}

/// Start a table of `n`, everyone's card dealt; the run, to look inside.
fn dealt(n: usize, seed: u64) -> Run {
    let ir = inkjot::build(TABLE).unwrap();
    let story = Arc::new(Story::new(&ir).unwrap());
    let mut run = Run::new(story, seed);
    let who = vec!["guest"; n];
    run.start_cohort(None, &who).unwrap();
    let deal = run
        .choices()
        .unwrap()
        .into_iter()
        .find(|(_, l)| l == "Deal")
        .unwrap()
        .0;
    run.choose(&deal).unwrap();
    run
}

fn eval(run: &mut Run, e: &str) -> String {
    let lowered = inkjot::ir::lower_expression(e);
    run.eval(&lowered).unwrap().to_string()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Every seat gets one card from each deck; the role deck is all
    /// dealt, one mafia for every four players (at least one), one doctor,
    /// and the rest townsfolk; no keepsake is dealt twice.
    #[test]
    fn a_deal_is_fair(n in 4usize..=6, seed in 1u64..1000) {
        let mut run = dealt(n, seed);
        prop_assert_eq!(eval(&mut run, "count(guest)"), n.to_string());
        prop_assert_eq!(eval(&mut run, "count(roles)"), "0");
        prop_assert_eq!(eval(&mut run, "count(keepsakes)"), (6 - n).to_string());
        let mafia = (n / 4).max(1);
        let mut roles = Vec::new();
        let mut keepsakes = Vec::new();
        for seat in run.seated().to_vec() {
            run.sit(&seat).unwrap();
            prop_assert_eq!(eval(&mut run, "count(my hand)"), "1");
            prop_assert_eq!(eval(&mut run, "count(my keepsake)"), "1");
            roles.push(eval(&mut run, "top of my hand"));
            keepsakes.push(eval(&mut run, "top of my keepsake"));
        }
        prop_assert_eq!(roles.iter().filter(|r| *r == "mafia").count(), mafia);
        prop_assert_eq!(roles.iter().filter(|r| *r == "doctor").count(), 1);
        keepsakes.sort();
        keepsakes.dedup();
        prop_assert_eq!(keepsakes.len(), n);
    }

    /// A keepsake stolen is in the thief's loot and no longer its owner's;
    /// given back, it is its owner's again. No card is ever lost or copied.
    #[test]
    fn a_card_is_always_somewhere_once(n in 4usize..=6, seed in 1u64..1000, victim in 0usize..5) {
        let mut run = dealt(n, seed);
        let thief = run.seated()[0].clone();
        run.sit(&thief).unwrap();
        let steals: Vec<String> = run
            .choices()
            .unwrap()
            .into_iter()
            .filter(|(_, l)| l == "Steal a keepsake")
            .map(|(id, _)| id)
            .collect();
        prop_assert_eq!(steals.len(), n - 1);
        run.choose(&steals[victim % steals.len()]).unwrap();
        prop_assert_eq!(eval(&mut run, "count(my loot)"), "1");
        let total = "count(keepsakes) + count(all(guest).keepsake) + count(all(guest).loot)";
        prop_assert_eq!(eval(&mut run, total), "6");
        prop_assert_eq!(eval(&mut run, "count(all(guest, where: count(it.keepsake) == 0))"), "1");
        let back = run.choices().unwrap().into_iter().find(|(_, l)| l == "Give it all back").unwrap().0;
        run.choose(&back).unwrap();
        prop_assert_eq!(eval(&mut run, "count(my loot)"), "0");
        prop_assert_eq!(eval(&mut run, total), "6");
        prop_assert_eq!(eval(&mut run, "count(all(guest, where: count(it.keepsake) == 1))"), n.to_string());
    }
}
