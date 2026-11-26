//! Milestone 21d, votes and seats: what Ex 7's table needs to be told and
//! to end. Quantifiers, `first`, `last` and `possessive`, tables with named
//! columns, lists of text, pronouns, faces worn (`shown_as`), the text and
//! list inputs, and `finale when:`.

use inkjot::run::test::run_all;

const LODGE: &str = r#":: StoryTitle
Lodge

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "hall"}

:: Inklink [config]
kind: game
mode: cohort
cohort: 3..=6
clock: 0.. ~ shown as "turn"
period phase: night 1, day 1
character guest: "a guest" ~ selectable ~ custom ~ one per linker ~ pronouns chosen
~ state alive: yes
~ state team: town
npc warden: "the Warden" ~ pronouns she
state lodge: nobody ~ of lodge
state teller: engine
state report: ""
state victim: nobody ~ of guest
list tellers: "how it is told" ~ holds engine: "the engine, plainly", storyteller: "a Storyteller, in their own words, at length"
table lodge: shuffled ~ one per game
~ columns name, member, special, deaths
~ row owls: "the Owls", "Owl", none, owl_deaths
~ row moths: "the Moths", "Moth", moths, moth_deaths
list owl_deaths ~ pick cycle
~ holds "found in the hayloft, feathers everywhere", "found by the pond"
list moth_deaths ~ pick shuffled
~ holds "found by the lamp"
get living: count(guest, where: it.alive)

:: hall
{choose input for: teller, from: tellers, length: 1}

The lodge meets.
> [[Draw the lodge->drawn]] ~ for cohort ~ then roll lodge
> [[Draw a lodge of moths->drawn]] ~ for cohort ~ then roll lodge where: it.special == moths

:: drawn
Tonight: {lodge.name}, and every one of them an {lodge.member}.
> [[Night falls->night]] ~ for cohort ~ then wait until night

:: night
> [[Strike at {target}->morning]] ~ for cohort ~ at guest where: it.alive and it != me ~ then set victim: target ~ then set victim.alive: no ~ then wait until day

:: morning
let flourish: one(lodge.deaths)
--
{capital(victim)} {flourish}. {capital(victim.they)} left {victim.their} coat; it is {possessive(victim)}.

{text input for: report, max: 20}
> [[Night falls->night]] ~ for cohort ~ then wait until night

:: all_gone [finale]
finale when: no guest where: it.alive and it != guest_1
--
Only one is left.

:: owls_win [finale]
finale when: lodge == owls and living <= 1
--
The owls have it.

:: the_lodge_is_drawn_and_read [test]
start: hall ~ as guest, guest, guest
given: draw lodge owls
expect: lodge == owls and lodge.special == none
do: "Draw the lodge"
expect text: "Tonight: the Owls, and every one of them an Owl."
expect: lodge == owls

:: a_draw_may_be_narrowed [test]
start: hall ~ as guest, guest, guest
do: "Draw a lodge of moths"
expect: lodge == moths and lodge.member == "Moth"

:: the_teller_is_chosen_from_a_list [test]
start: hall ~ as guest, guest, guest
answer: storyteller
expect: teller == storyteller

:: deaths_are_told_and_turn_in_order [test]
start: hall ~ as guest, guest, guest
given: draw lodge owls
do: "Draw the lodge"
do: "Night falls"
given: set guest_2.pronouns: she
given: set guest_2.name: "Bea"
do: "Strike at Bea"
expect: not guest_2.alive
expect: clock.phase == day
expect text: "Bea found in the hayloft, feathers everywhere. She left her coat; it is Bea’s."
answer: "A short report"
expect: report == "A short report"

:: quantifiers_count [test]
start: hall ~ as guest, guest, guest
expect: some guest where: it.alive
expect: every guest where: it.team == town
expect: not (no guest where: it.alive)
given: set guest_3.alive: no
expect: some guest where: not it.alive

:: the_game_ends_at_the_start_of_a_turn [test]
start: hall ~ as guest, guest, guest
given: draw lodge moths
given: set guest_2.alive: no
given: set guest_3.name: "Cy"
do: "Draw the lodge"
do: "Night falls"
(((
Not yet: the game ends only as a turn begins.
)))
expect at: night
do: "Strike at Cy"
expect at: all_gone
expect text: "Only one is left."
"#;

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
    assert_eq!(errors(LODGE), Vec::<String>::new());
}

#[test]
fn its_tests_pass() {
    let ir = inkjot::build(LODGE).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    for o in run_all(LODGE, &ir).unwrap() {
        assert!(
            o.passed(),
            "`{}`: {:#?}\n{}",
            o.name,
            o.failures,
            o.transcript.join("\n")
        );
    }
}
