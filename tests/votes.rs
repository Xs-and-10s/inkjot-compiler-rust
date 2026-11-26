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

/// A table of four: two crows choose together at night, a vote by day, a
/// ballot for the weather, and a rumour that goes round the table.
const ROOST: &str = r#":: StoryTitle
Roost

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "dusk"}

:: Inklink [config]
kind: game
mode: cohort
cohort: 4..=6
clock: 0.. ~ shown as "turn"
period phase: night 1, day 1
character guest: "a guest" ~ selectable ~ custom ~ one per linker
~ state alive: yes
~ state team: town
~ state heard: no
~ state method: peck
relation seat: each guest ~ ring ~ in order joined
state victim: nobody ~ of guest
state second: nobody ~ of guest
state weather: nothing ~ of weathers
state rumour: no
list teams ~ holds town, crow
list weathers: "the weather" ~ holds rain: "rain", shine: "shine", fog: "fog"

:: dusk
{ballot for: weather, from: weathers, ties: draw}
> [[Nightfall->night]] ~ for cohort ~ when ballot.done ~ then wait until night

:: night
> ~ for guest where: it.alive and it.team == crow ~ consensus
> [[Peck {target}->night:done]] ~ at guest where: it.alive and it.team != crow ~ then set victim: target ~ then set me.method: peck
> [[Swoop on {target}->night:done]] ~ at guest where: it.alive and it.team != crow ~ then set victim: target ~ then set me.method: swoop

> [[Sleep->night:done]] ~ for guest where: it.team != crow

:: night:done
You wait.
> [[Morning->morning]] ~ for cohort ~ when cohort.all.chose ~ then wait until day

:: morning
let first: first(victim.chosen_by)
set victim.alive: no when victim != nobody
--
{if victim == nobody then "Nobody died." else "{victim} died, by {first.method}."}
> [[The vote->vote]] ~ for cohort

:: vote
{accuse who: all(guest, where: it.alive), voters: all(guest, where: it.alive), revote: 1, ties: draw}
> [[Counted->counted]] ~ for cohort ~ when accusation.done

:: counted
set accusation.who.alive: no
spreads 1 from all(guest, where: it.heard) to nearest(guest, where: not it.heard) ~ along seat ~ then set it.heard: yes ~ when rumour
--
{ballots}

:: marking
> ~ for guest where: it.team == crow ~ consensus 2
> [[Mark {target}->marking:done]] ~ at guest where: it.team != crow ~ then set victim: first(targets) ~ then set second: last(targets)

:: marking:done
> [[Dawn->marking:done]] ~ for cohort ~ then wait until day

:: the_weather_is_voted [test]
start: dusk ~ as guest, guest, guest, guest
answer: rain
as: guest_2
answer: rain
as: guest_3
answer: fog
expect: not ballot.done
as: guest_4
answer: shine
expect: ballot.done and weather == rain

:: crows_choose_together [test]
start: night ~ as guest, guest, guest, guest
given: set guest_1.team: crow
given: set guest_2.team: crow
given: set guest_3.name: "Cas"
given: set guest_4.name: "Dot"
as: guest_1
do: "Swoop on Dot"
expect: victim == nobody
as: guest_2
do: "Peck Cas"
as: guest_3
do: "Sleep"
as: guest_4
do: "Sleep"
do: "Morning"
(((
A tie: the first chosen, Dot, by the one who chose first, their way.
)))
expect: not guest_4.alive and guest_3.alive
expect text: "Dot died, by swoop."
expect: guest_4.chosen_by == [guest_1]

:: no_votes_no_kill [test]
start: night ~ as guest, guest, guest, guest
given: set guest_1.team: crow
as: guest_2
do: "Sleep"
as: guest_3
do: "Sleep"
as: guest_4
do: "Sleep"
expect no link: "Morning"
as: guest_1
expect link: "Peck a guest"

:: the_living_vote_and_the_rumour_goes_round [test]
start: vote ~ as guest, guest, guest, guest
given: set rumour: yes
given: set guest_1.heard: yes
given: set guest_1.team: crow
given: set guest_1.name: "Ash"
given: set guest_2.name: "Bel"
given: set guest_3.name: "Cas"
given: set guest_4.alive: no
vote: guest_3
as: guest_2
vote: guest_3
as: guest_4
expect no link: "Counted"
as: guest_3
vote: guest_2
expect: accusation.done and accusation.who == guest_3 and not accusation.drawn
expect: guest_2.voted_with(crow) and not guest_3.voted_with(crow)
do: "Counted"
expect text: "2 for Cas; 1 for Bel"
expect: not guest_3.alive
expect: guest_2.heard and not guest_3.heard

:: two_each_and_the_two_most_chosen [test]
start: marking ~ as guest, guest, guest, guest, guest
given: set guest_1.team: crow
given: set guest_2.team: crow
given: set guest_3.name: "Cas"
given: set guest_4.name: "Dot"
given: set guest_5.name: "Eve"
as: guest_1
do: "Mark Cas"
expect at: marking
expect no link: "Mark Cas"
do: "Mark Dot"
expect at: marking:done
as: guest_2
do: "Mark Dot"
do: "Mark Eve"
do: "Dawn"
expect: victim == guest_4 and second == guest_3
expect: guest_4.chosen_by == [guest_1, guest_2]

:: the_dead_have_no_vote [test]
start: vote ~ as guest, guest, guest, guest
given: set guest_4.alive: no
as: guest_4
expect: not (guest_4 in accusation.voters)
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
    assert_eq!(errors(ROOST), Vec::<String>::new());
}

#[test]
fn its_tests_pass() {
    for src in [LODGE, ROOST] {
        let ir = inkjot::build(src).unwrap_or_else(|r| panic!("{:#?}", r.diags));
        for o in run_all(src, &ir).unwrap() {
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
