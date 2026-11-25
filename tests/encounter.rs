//! Milestone 20d, encounters: an opponent's pattern of threats, round by
//! round, and the counters that answer them.

use inkjot::engine::{Inklink, Para, Start, State};
use proptest::prelude::*;

/// An ogre in a cave, one threat a round; a shield that drinks blows.
const CAVE: &str = r#":: StoryTitle
Cave

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "cave"}

:: Inklink [config]
kind: game
mode: solo
pov: ann
clock: 0.. ~ each action
resource hp: 9
character ann: "Ann" ~ selectable ~ in cave ~ hp 9
npc ogre: "the ogre" ~ in cave
place cave: "the cave"
place hill: "the hill"
way mouth: cave - hill
item shield: "a shield" ~ held by ann ~ absorbs any
state scared: no
state soaked: 0

on absorb with shield
~ then raise soaked 1

:: ogre [npc]
stat might: 3
pattern moves: smash, roar ~ repeat ~ one per round
threat smash: blunt ~ at me ~ answered by prevent, block, cover, absorb
~ then wound target 1
threat roar ~ at me ~ answered by cover
~ then set me.scared: yes

:: cave
The cave. The ogre is here.
> [[Fight->fight]]

:: hill
The hill.

:: fight [encounter]
encounter ogre ~ round by round ~ acts last ~ until ogre.might == 0 or me.down
--
The ogre gets up.
> [[Begin->fight:round]]

:: fight:round [round]
--
Next: {ogre.moves.next}.
> [[Stop it->fight:act]] ~ then prevent ogre
> [[Brace->fight:act]] ~ then cover me
> [[Raise the shield->fight:act]] ~ then absorb with shield
> [[Hit it->fight:act]] ~ then lower ogre.might 1
> [[Wait->fight:act]]
> [[Run->here]] ~ then end encounter

:: fight:act [act]
--
You act.

:: fight:after [resolve]
--
Told: {threat.name}/{threat.outcome}.
> [[Next round->fight:round]] ~ when ogre.might > 0 and me.up
> [[Leave it lying->here]] ~ when ogre.might == 0
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

/// The last `Told: threat/outcome.` line shown.
fn told(s: &State) -> String {
    text(s)
        .into_iter()
        .rev()
        .find(|t| t.starts_with("Told: "))
        .unwrap_or_default()
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
    assert_eq!(errors(CAVE), Vec::<String>::new());
}

#[test]
fn a_round_ends_with_the_opponents_threat() {
    let s = take(&take(&start(CAVE), "Fight"), "Begin");
    assert_eq!(text(&s), ["Next: smash."]);
    let s = take(&s, "Wait");
    assert_eq!(text(&s), ["You act.", "Told: smash/hit."]);
    assert_eq!(s.view().now, 1);
    // Reading the next round is not an action.
    let s = take(&s, "Next round");
    assert_eq!(text(&s), ["Next: roar."]);
    assert_eq!(s.view().now, 1);
}

#[test]
fn each_counter_answers_what_it_may() {
    let s = take(&take(&start(CAVE), "Fight"), "Begin");
    assert_eq!(told(&take(&s, "Stop it")), "Told: smash/prevent.");
    assert_eq!(told(&take(&s, "Brace")), "Told: smash/cover.");
    let soaked = take(&s, "Raise the shield");
    assert_eq!(told(&soaked), "Told: smash/absorb.");
    // The roar may only be answered by a cover: stopping it does nothing.
    let roar = take(&take(&s, "Wait"), "Next round");
    assert_eq!(told(&take(&roar, "Stop it")), "Told: roar/hit.");
    assert_eq!(told(&take(&roar, "Brace")), "Told: roar/cover.");
}

#[test]
fn a_cover_lasts_until_a_threat_uses_it() {
    let s = take(&take(&start(CAVE), "Fight"), "Begin");
    // Braced for the smash: the cover takes it, and is gone.
    let s = take(&s, "Brace");
    assert_eq!(told(&s), "Told: smash/cover.");
    let s = take(&take(&s, "Next round"), "Wait");
    assert_eq!(
        told(&s),
        "Told: roar/hit.",
        "the cover was used on the smash"
    );
}

#[test]
fn the_encounter_ends_when_its_until_holds() {
    let mut s = take(&take(&start(CAVE), "Fight"), "Begin");
    for _ in 0..2 {
        s = take(&take(&s, "Hit it"), "Next round");
    }
    let s = take(&s, "Hit it");
    assert!(labels(&s).contains(&"Leave it lying".to_string()));
    let s = take(&s, "Leave it lying");
    assert_eq!(s.view().nib, "cave");
    assert!(
        labels(&s).contains(&"Go to the hill".to_string()),
        "walks again: {:?}",
        labels(&s)
    );
}

#[test]
fn running_away_ends_it_at_once() {
    let s = take(&take(&take(&start(CAVE), "Fight"), "Begin"), "Run");
    assert_eq!(s.view().nib, "cave");
}

#[test]
fn the_new_words_are_checked() {
    let bad = CAVE.replace("answered by prevent, block", "answered by prevent, dodge");
    assert!(errors(&bad)
        .iter()
        .any(|e| e.contains("`dodge` is not a counter")));
    let bad = CAVE.replace("pattern moves: smash, roar", "pattern moves: smash, rour");
    assert!(errors(&bad).iter().any(|e| e.contains("`rour`")));
    let bad = CAVE.replace("encounter ogre", "encounter orge");
    assert!(errors(&bad).iter().any(|e| e.contains("`orge`")));
    let bad = CAVE.replace("then absorb with shield", "then absorb shield");
    assert!(errors(&bad)
        .iter()
        .any(|e| e.contains("`absorb with item`")));
    let bad = CAVE.replace("~ then cover me", "~ then shield me");
    assert!(errors(&bad).iter().any(|e| e.contains("is not an effect")));
    let old = CAVE.replace("threat roar ~ at me", "spell roar ~ at me");
    assert!(errors(&old).iter().any(|e| e.contains("now `threat`")));
}

/// What the counters do, as the docs say, round by round.
fn model(acts: &[usize]) -> Vec<String> {
    let mut covered = false;
    let mut out = Vec::new();
    for (round, act) in acts.iter().enumerate() {
        let smash = round % 2 == 0;
        if *act == 1 {
            covered = true;
        }
        let outcome = match (smash, act) {
            (true, 0) => "prevent",
            (true, 2) => "absorb",
            _ if covered => {
                covered = false;
                "cover"
            }
            _ => "hit",
        };
        out.push(format!(
            "Told: {}/{outcome}.",
            if smash { "smash" } else { "roar" }
        ));
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Any order of counters gives the outcomes the model says.
    #[test]
    fn counters_follow_the_rules(acts in prop::collection::vec(0usize..4, 1..7)) {
        let names = ["Stop it", "Brace", "Raise the shield", "Wait"];
        let mut s = take(&take(&start(CAVE), "Fight"), "Begin");
        let mut seen = Vec::new();
        for (i, a) in acts.iter().enumerate() {
            if i > 0 {
                s = take(&s, "Next round");
            }
            s = take(&s, names[*a]);
            seen.push(told(&s));
        }
        prop_assert_eq!(seen, model(&acts));
    }
}
