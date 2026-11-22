//! Milestone 20b, time and motion: a turn for each action, effects that
//! last `for N turns` or wait `after N turns`, handlers on every thing of
//! a kind, someone who `follows me`, and NPC goals that keep to some places
//! (`~ where:`), move only every so often (`~ every 2 turns`), or wander.

use inkjot::engine::{Inklink, Para, Start, State};
use proptest::prelude::*;

/// A row of places: yard, hall, cellar, pit, forge. Everything below the
/// yard is salted; the cellar and the pit are vaults; the forge is hot.
const ROW: &str = r#":: StoryTitle
Row

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "yard"}

:: Inklink [config]
kind: game
mode: solo
pov: ann
clock: 0.. ~ shown as "turn" ~ each action
character ann: "Ann" ~ selectable ~ in yard
archetype vault ~ state trips: 0
place yard: "the yard"
place hall: "the hall" ~ salted
place cellar: "the cellar" ~ salted ~ like vault
place pit: "the pit" ~ salted ~ like vault
place forge: "the forge" ~ salted ~ hot
way a: yard - hall
way b: hall - cellar
way c: cellar - pit
way d: pit - forge
state ticks: 0
state salt_steps: 0
state caught: 0
state door_open: no
state hushed: no
state cat_moves: 0
state calm: yes
npc dog: "the dog" ~ in yard
~ follows me when calm
npc hound: "the hound" ~ in pit
npc cat: "the cat" ~ in hall

on turn ends
~ then raise ticks 1

on enter vault by me
~ then raise entered.trips 1

on enter place by me when entered.salted
~ then raise salt_steps 1

on move hound when hound.here
~ then raise caught 1

on move cat
~ then raise cat_moves 1

:: hound [npc]
state awake: no
goal hunt:
~ toward me
~ when self.awake
~ every 2 turns
~ where: it.salted and not it.hot

:: cat [npc]
goal roam:
~ wanders
~ where: it.salted and not it.hot

:: acts [lib everywhere]
> [[{clock.now}/{ticks}/{hushed}/{door_open}/{salt_steps}/{caught}/{dog.place}/{hound.place}/{cat.place}->acts:wait]]
> [[Hush->acts:hush]]
> [[Ring->acts:ring]]
> [[Whistle->here]] ~ then set hushed: yes ~ for 2 turns
> [[Wake the hound->here]] ~ then set hound.awake: yes
> [[Shoo the dog->here]] ~ then set calm: no

:: acts:wait [act]
--
You wait.

:: acts:hush [act]
set hushed: yes ~ for 3 turns
--
Hush.

:: acts:ring [act]
set door_open: yes ~ after 2 turns when not door_open
--
The bell rings.

:: yard
The yard.

:: hall
The hall.

:: cellar
The cellar. Visits: {cellar.trips}.

:: pit
The pit. Visits: {pit.trips}.

:: forge
The forge.
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
        .find(|l| l.label.plain().starts_with(label))
        .unwrap_or_else(|| panic!("no Link `{label}`; there are {:?}", labels(s)))
        .id
        .clone();
    s.take_link(&id).unwrap()
}

/// The first Link's label, split: clock, ticks, hushed, door, salt
/// steps, caught, the dog's place, the hound's place, the cat's place.
fn status(s: &State) -> Vec<String> {
    labels(s)[0].split('/').map(str::to_string).collect()
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

/// The walks offered: `Go to the hall`, and so on.
fn walk(s: &State, to: &str) -> State {
    let id = s
        .view()
        .links
        .iter()
        .find(|l| l.label.plain() == format!("Go to {to}"))
        .unwrap_or_else(|| panic!("no walk to `{to}`; there are {:?}", labels(s)))
        .id
        .clone();
    s.take_link(&id).unwrap()
}

#[test]
fn it_checks_clean() {
    assert_eq!(errors(ROW), Vec::<String>::new());
}

#[test]
fn each_action_is_a_turn() {
    let s = start(ROW);
    assert_eq!(status(&s)[..2], ["0", "0"]);
    let s = take(&s, "0/0");
    assert_eq!(text(&s), ["You wait."]);
    assert_eq!(status(&s)[..2], ["1", "1"], "the clock and `on turn ends`");
    let s = walk(&s, "the hall");
    assert_eq!(s.view().now, 2, "a walk is an action too");
}

#[test]
fn for_n_turns_puts_it_back() {
    // Hushed on turn 0, for turns 0, 1 and 2.
    let s = take(&start(ROW), "Hush");
    assert_eq!(status(&s)[2], "yes");
    let s = take(&s, "1/1");
    assert_eq!(status(&s)[..3], ["2", "2", "yes"]);
    let s = take(&s, "2/2");
    assert_eq!(status(&s)[..3], ["3", "3", "no"], "turn 3: it has worn off");
}

#[test]
fn again_while_it_lasts_lasts_from_then() {
    let s = take(&start(ROW), "Hush"); // turns 0 to 2
    let s = take(&s, "1/1");
    let s = take(&s, "Whistle"); // turns 2 and 3
    assert_eq!(
        status(&s)[..3],
        ["3", "3", "yes"],
        "the first would have ended here"
    );
    let s = take(&s, "3/3");
    assert_eq!(status(&s)[..3], ["4", "4", "no"]);
}

#[test]
fn after_n_turns_waits() {
    let s = take(&start(ROW), "Ring");
    assert_eq!(text(&s), ["The bell rings."]);
    assert_eq!(status(&s)[3], "no");
    let s = take(&s, "1/1");
    assert_eq!(status(&s)[3], "yes", "two turns after the ring");
}

#[test]
fn the_clock_s_effects_come_back_with_a_rewind() {
    let src = ROW.replace(
        "clock: 0.. ~ shown as \"turn\" ~ each action",
        "clock: 0.. ~ shown as \"turn\" ~ each action\nresource rewinds: 3\nrewind: undo\n~ to any visited turn",
    )
    .replace("> [[Hush->acts:hush]]", "> [[Hush->acts:hush]]\n> [[Undo->rewind]]");
    let s = take(&start(&src), "Hush");
    let s = take(&s, "1/1");
    let s = take(&s, "2/2");
    assert_eq!(status(&s)[..3], ["3", "3", "no"]);
    // Going back is not a turn; back at the start of turn 1, still hushed,
    // and it still wears off at 3.
    let s = take(&s, "Undo");
    assert_eq!(s.view().now, 3);
    let s = take(&s, "Back to the start of turn 1");
    assert_eq!(status(&s)[..3], ["1", "1", "yes"]);
    let s = take(&s, "1/1");
    assert_eq!(status(&s)[..3], ["2", "2", "yes"]);
    let s = take(&s, "2/2");
    assert_eq!(status(&s)[..3], ["3", "3", "no"]);
}

#[test]
fn handlers_on_a_kind_see_what_it_happened_to() {
    let s = walk(&start(ROW), "the hall");
    assert_eq!(status(&s)[4], "1", "the hall is salted");
    let s = walk(&s, "the cellar");
    assert_eq!(text(&s)[0], "The cellar. Visits: 1.");
    assert_eq!(status(&s)[4], "2");
    let s = walk(&s, "the hall");
    let s = walk(&s, "the cellar");
    assert_eq!(
        text(&s)[0],
        "The cellar. Visits: 2.",
        "`entered` is the place"
    );
    // The cat wanders the salted places, but never `by me`: no salt steps
    // or visits for it.
    assert_eq!(status(&s)[4], "4");
}

#[test]
fn a_follower_comes_along_while_its_when_holds() {
    let s = walk(&start(ROW), "the hall");
    assert_eq!(status(&s)[6], "the hall", "the dog came too");
    let s = take(&s, "Shoo the dog");
    let s = walk(&s, "the cellar");
    assert_eq!(status(&s)[6], "the hall", "not calm: it stays");
}

#[test]
fn a_hunter_keeps_its_pace_and_its_ground() {
    // Ann in the yard; the hound in the pit, two places from the hall.
    // The clock as each action ends: the hound moves when it is even.
    let s = walk(&start(ROW), "the hall"); // 1
    let s = take(&s, "Wake the hound"); // 2: it moves
    assert_eq!(status(&s)[7], "the cellar");
    let s = take(&s, "2/2"); // 3: it rests
    assert_eq!(status(&s)[7], "the cellar");
    let s = walk(&s, "the yard"); // 4: Ann is on unsalted ground; it stays
    assert_eq!(status(&s)[7], "the cellar");
    assert_eq!(status(&s)[5], "0");
    let s = walk(&s, "the hall"); // 5: rests
    assert_eq!(status(&s)[7], "the cellar");
    let s = take(&s, "5/5"); // 6: into the hall, where Ann is
    assert_eq!(status(&s)[7], "the hall");
    assert_eq!(status(&s)[5], "1", "`on move hound when hound.here`");
}

#[test]
fn the_new_words_are_checked() {
    let bad = ROW.replace("on enter vault by me", "on enter vaults by me");
    assert!(errors(&bad)
        .iter()
        .any(|e| e.contains("`vaults` is not a declared place")));
    let bad = ROW.replace("on move hound", "on move yard");
    assert!(errors(&bad)
        .iter()
        .any(|e| e.contains("`yard` is not a declared person")));
    let bad = ROW.replace("~ follows me when calm", "~ follows ann");
    assert!(errors(&bad).iter().any(|e| e.contains("`~ follows me`")));
    let bad = ROW.replace("~ follows me when calm", "~ follows me when calmm");
    assert!(errors(&bad).iter().any(|e| e.contains("`calmm`")));
    let bad = ROW.replace("~ for 2 turns", "~ for two turns");
    assert!(
        errors(&bad).iter().any(|e| e.contains("two")),
        "{:?}",
        errors(&bad)
    );
}

#[test]
fn the_same_wait_is_not_set_going_twice() {
    // Rung on turns 0 and 1: the door opens at 2, once.
    let s = take(&start(ROW), "Ring");
    let s = take(&s, "Ring");
    assert_eq!(status(&s)[..4], ["2", "2", "no", "yes"]);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// `for n turns` holds on turns 0 to n - 1; `after n turns` happens on
    /// turn n.
    #[test]
    fn a_time_is_that_many_turns(n in 1u32..7, waits in 1usize..10) {
        let src = ROW
            .replace("~ for 3 turns", &format!("~ for {n} turns"))
            .replace("~ after 2 turns", &format!("~ after {n} turns"));
        let mut hush = take(&start(&src), "Hush");
        let mut ring = take(&start(&src), "Ring");
        for _ in 0..waits {
            let (h, r) = (status(&hush), status(&ring));
            let now: u32 = h[0].parse().unwrap();
            prop_assert_eq!(h[2] == "yes", now < n, "hushed at {}", now);
            let now: u32 = r[0].parse().unwrap();
            prop_assert_eq!(r[3] == "yes", now >= n, "open at {}", now);
            hush = take(&hush, &format!("{}/", h[0]));
            ring = take(&ring, &format!("{}/", r[0]));
        }
    }

    /// However the Linker goes, every action is one turn, the cat never
    /// sets foot outside the salted places that are not vaults, and the
    /// hound never on unsalted or hot ground.
    #[test]
    fn the_clock_and_the_ground_hold(moves in prop::collection::vec(0usize..8, 1..24)) {
        let mut s = start(&ROW.replace("~ when self.awake\n", ""));
        for (n, m) in moves.iter().enumerate() {
            let links = s.view().links.clone();
            let id = links[m % links.len()].id.clone();
            s = s.take_link(&id).unwrap();
            prop_assert_eq!(s.view().now as usize, n + 1);
            let st = status(&s);
            prop_assert_eq!(&st[1], &(n + 1).to_string());
            for at in [&st[7], &st[8]] {
                prop_assert!(["the hall", "the cellar", "the pit"].contains(&at.as_str()), "{:?}", st);
            }
        }
    }
}
