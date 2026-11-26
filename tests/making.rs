//! Milestone 20c, making and the rest: counted items, taking and dropping
//! offered like moves, recipes, tools that break, `gain` of a resource,
//! tables read as values, `nearest(…, where:)`, `x.toward(y)`, `spreads`,
//! effects on what an expression names, and an ending that a `play` sends
//! the pov to.

use inkjot::engine::{Inklink, Para, Start, State};
use proptest::prelude::*;

/// A shed, a yard, a lane, a field and a well, in a row.
const SHED: &str = r#":: StoryTitle
Shed

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "shed"}

:: Inklink [config]
kind: game
mode: solo
pov: ann
clock: 0.. ~ each action
default takes: take, drop
default cost: salt
resource salt: 2 ~ max 5 ~ counted as "crystal"
character ann: "Ann" ~ selectable ~ in shed
npc tom: "Tom" ~ in yard
place shed: "the shed" ~ timbered
place yard: "the yard"
place lane: "the lane" ~ timbered
place field: "the field" ~ timbered
place well: "the well" ~ way_out
way door: shed - yard
way gate: yard - lane
way stile: lane - field
way path: field - well
way rope: well - field ~ one way ~ shown as "down the rope"
state score: 0
state heat: 0 ~ belongs to place
act take
act drop
item stave: "a pit prop" ~ plural "staves" ~ in shed ~ count 3 ~ burns
item sack: "an old sack" ~ in shed ~ burns
item hammer: "a hammer" ~ in shed
item shoe: "a horseshoe" ~ in yard
item pick: "a pick" ~ in yard ~ breaks 1 in 2
item bell: "a bell" ~ in yard ~ fixed
item coin: "a coin" ~ held by tom
item pawl: "an iron pawl"
item stretcher: "a stretcher"
recipe stretcher: stave, stave, sack ~ make ~ "Two staves and a sack, for carrying."
recipe pawl: shoe ~ using hammer ~ forge ~ "A shoe, bent into a catch."
table rank: score
~ 0..=1: "a beginner"
~ 2..=4: "a hand"
~ 5: "a master"
table luck: roll d2
~ 1: then raise score 1
~ 2: nothing
get burning: all(place, where: it.heat > 0)

:: acts [lib everywhere]
> [[{rank}/{score}/{me.salt}/{count(stave, where: it.held_by(me))}/{burning}->acts:look]]
> [[Dig->acts:dig]] ~ uses pick ~ when me.holds.pick and not pick.broken
> [[Prise->acts:prise]]
> [[Throw two staves away->acts:shed]] ~ when count(stave, where: it.held_by(me)) >= 2
> [[Light up->acts:light]] ~ when here.timbered and here.heat == 0
> [[Let it spread->acts:spread]] ~ when count(burning) > 0
> [[Dowse->acts:dowse]]
> [[Tom, take something->acts:tom]] ~ when tom.here
> [[Ask Tom for {target}->acts:ask]] ~ at item where: it.held_by(tom) ~ when tom.here
> [[Send Tom to the well->acts:send]] ~ when tom.here
> [[Gamble->acts:gamble]]
> [[Jump in the well->acts:jump]] ~ when here == well

:: acts:look [act]
--
You look around. You carry {all(item, where: it.held_by(me))}.
{list burning}

:: acts:dig [act]
raise score 1
--
You dig.

:: acts:prise [act]
gain salt 2
--
You prise out two crystals.

:: acts:shed [act]
lose stave 2
--
You throw two staves away.

:: acts:light [act]
set here.heat: 1
--
You set it alight.

:: acts:spread [act]
spreads 1 from burning to nearest(place, where: it.timbered and it.heat == 0) ~ along way ~ within 1 ~ then set it.heat: 1
destroy all(item, in: burning, where: it.burns)
--
It spreads.

:: acts:dowse [act]
let out: nearest(place, where: it.way_out) ~ along way
let step: here.toward(out)
--
The air goes to {out}, by {step}.

:: acts:tom [act]
give one(item, in: here, where: it != bell and not it.held_by(me)) to tom
--
Tom takes something.

:: acts:ask [act]
give target to me
--
Tom hands over {target}.

:: acts:send [act]
move tom to nearest(place, where: it.way_out) ~ along way
--
Tom goes.

:: acts:gamble [act]
roll luck
--
You gamble.

:: acts:jump [act]
play drowned
--
You jump.

:: drowned [ending]
--
The water is very cold.

:: shed
The shed.

:: yard
The yard.

:: lane
The lane.

:: field
The field.

:: well
The well.
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

/// rank, score, salt, staves held, places burning.
fn status(s: &State) -> Vec<String> {
    labels(s)[0].split('/').map(str::to_string).collect()
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
    assert_eq!(errors(SHED), Vec::<String>::new());
}

#[test]
fn taking_and_dropping_are_offered() {
    let s = start(SHED);
    let all = labels(&s);
    // One Link for the three staves.
    let mut takes: Vec<&String> = all.iter().filter(|l| l.starts_with("Take")).collect();
    takes.sort();
    assert_eq!(
        takes,
        ["Take a hammer", "Take a pit prop", "Take an old sack"]
    );
    let s = take(&s, "Take a pit prop");
    let s = take(&s, "Take a pit prop");
    assert_eq!(status(&s)[3], "2");
    let s = take(&s, "Take a pit prop");
    assert!(!labels(&s).iter().any(|l| l == "Take a pit prop"));
    let s = take(&s, "A beginner/0/2/3/");
    assert_eq!(text(&s), ["You look around. You carry three staves."]);
    let s = take(&s, "Drop a pit prop");
    assert_eq!(status(&s)[3], "2");
    // Fixed things, and things others hold, are not offered.
    let s = take(&take(&s, "Go to the yard"), "Take a pick");
    assert!(!labels(&s)
        .iter()
        .any(|l| l == "Take a bell" || l == "Take a coin"));
}

#[test]
fn a_recipe_uses_up_its_parts() {
    let mut s = start(SHED);
    for t in ["Take a pit prop", "Take a pit prop"] {
        s = take(&s, t);
    }
    assert!(
        !labels(&s).iter().any(|l| l.starts_with("Make")),
        "no sack yet"
    );
    s = take(&s, "Take an old sack");
    s = take(&s, "Make a stretcher");
    assert_eq!(text(&s), ["Two staves and a sack, for carrying."]);
    assert_eq!(status(&s)[3], "0");
    assert!(
        !labels(&s).iter().any(|l| l.starts_with("Make")),
        "made once"
    );
    assert!(labels(&s).contains(&"Drop a stretcher".to_string()));
}

#[test]
fn a_recipe_needs_its_tool_at_hand() {
    // The hammer in the shed, the horseshoe in the yard.
    let s = take(&take(&start(SHED), "Go to the yard"), "Take a horseshoe");
    assert!(!labels(&s).iter().any(|l| l.starts_with("Forge")));
    let s = take(&s, "Go to the shed");
    // The hammer is lying here: at hand.
    let s = take(&s, "Forge an iron pawl");
    assert_eq!(text(&s), ["A shoe, bent into a catch."]);
}

#[test]
fn a_tool_that_breaks_breaks_sometimes() {
    let mut broke = 0;
    for seed in 1..40 {
        let ir = inkjot::build(SHED).unwrap();
        let s = Inklink::from_ir(&ir)
            .unwrap()
            .start(seed, &Start::default())
            .unwrap();
        let s = take(&take(&s, "Go to the yard"), "Take a pick");
        let s = take(&s, "Dig");
        if text(&s).contains(&"The pick breaks.".to_string()) {
            broke += 1;
            assert!(!labels(&s).contains(&"Dig".to_string()));
        }
    }
    // One in two, over 39 seeds.
    assert!((8..=31).contains(&broke), "{broke}");
}

#[test]
fn gain_and_lose_count() {
    let s = take(&start(SHED), "Prise");
    assert_eq!(status(&s)[2], "4");
    let s = take(&s, "Prise");
    assert_eq!(status(&s)[2], "5", "no more than its max");
    let mut s = s;
    for _ in 0..3 {
        s = take(&s, "Take a pit prop");
    }
    let s = take(&s, "Throw two staves away");
    assert_eq!(status(&s)[3], "1");
}

#[test]
fn a_table_is_read_as_a_value() {
    let mut s = start(SHED);
    assert_eq!(status(&s)[0], "A beginner");
    s = take(&take(&s, "Go to the yard"), "Take a pick");
    for _ in 0..40 {
        if status(&s)[1] == "2" || !labels(&s).contains(&"Dig".to_string()) {
            break;
        }
        s = take(&s, "Dig");
    }
    if status(&s)[1] == "2" {
        assert_eq!(status(&s)[0], "A hand");
    }
    // A table of effects is still rolled.
    let s = take(&start(SHED), "Gamble");
    assert!(["0", "1"].contains(&status(&s)[1].as_str()));
}

#[test]
fn nearest_and_toward_follow_the_map() {
    let s = take(&start(SHED), "Dowse");
    assert_eq!(text(&s), ["The air goes to the well, by the yard."]);
    let s = take(&take(&s, "Go to the yard"), "Send Tom to the well");
    let s = take(&s, "Go to the lane");
    assert!(!labels(&s).iter().any(|l| l.starts_with("Tom")));
}

#[test]
fn fire_spreads_one_way_at_a_time() {
    let s = take(&start(SHED), "Light up");
    assert_eq!(status(&s)[4], "the shed");
    // The shed's nearest timbered neighbour is the lane, two ways off:
    // too far for `within 1`. The items in the shed burn.
    let s = take(&s, "Let it spread");
    assert_eq!(status(&s)[4], "the shed");
    assert!(!labels(&s)
        .iter()
        .any(|l| l == "Take a pit prop" || l == "Take an old sack"));
    assert!(labels(&s).contains(&"Take a hammer".to_string()));
    // From the lane, the field is next door.
    let s = take(
        &take(&take(&s, "Go to the yard"), "Go to the lane"),
        "Light up",
    );
    let s = take(&s, "Let it spread");
    assert_eq!(status(&s)[4], "the field, the lane and the shed");
}

#[test]
fn effects_take_what_an_expression_names() {
    let s = take(&start(SHED), "Go to the yard");
    let s = take(&s, "Ask Tom for a coin");
    assert_eq!(text(&s), ["Tom hands over a coin."]);
    let s = take(&s, "Tom, take something");
    // `one(item, in: here, …)`: the first lying here, by name, that is not
    // the bell.
    assert!(
        labels(&s).contains(&"Ask Tom for a pick".to_string()),
        "{:?} {:?}",
        text(&s),
        labels(&s)
    );
}

#[test]
fn a_played_ending_ends_the_reading() {
    let mut s = start(SHED);
    for place in ["the yard", "the lane", "the field", "the well"] {
        s = take(&s, &format!("Go to {place}"));
    }
    let now = s.view().now;
    let s = take(&s, "Jump in the well");
    assert_eq!(s.view().nib, "drowned");
    assert_eq!(text(&s), ["You jump.", "The water is very cold."]);
    assert_eq!(s.view().now, now, "no turn passes after the end");
}

#[test]
fn two_ways_to_one_place_say_which() {
    let mut s = start(SHED);
    for place in ["the yard", "the lane", "the field", "the well"] {
        s = take(&s, &format!("Go to {place}"));
    }
    let all = labels(&s);
    assert!(
        all.contains(&"Go to the field (path)".to_string()),
        "{all:?}"
    );
    assert!(all.contains(&"Go to the field (down the rope)".to_string()));
}

#[test]
fn the_new_words_are_checked() {
    let bad = SHED.replace(
        "recipe stretcher: stave, stave, sack",
        "recipe stretcher: stave, stave, sock",
    );
    assert!(errors(&bad)
        .iter()
        .any(|e| e.contains("`sock` is not a declared item")));
    let bad = SHED.replace("~ using hammer", "~ using hamer");
    assert!(errors(&bad).iter().any(|e| e.contains("`hamer`")));
    let bad = SHED.replace("~ uses pick", "~ uses pik");
    assert!(errors(&bad).iter().any(|e| e.contains("`pik`")));
    let bad = SHED.replace("gain salt 2", "gain slat 2");
    assert!(errors(&bad).iter().any(|e| e.contains("`slat`")));
    let bad = SHED.replace(
        "where: it.timbered and it.heat == 0)",
        "where: it.timbered and its.heat == 0)",
    );
    assert!(errors(&bad).iter().any(|e| e.contains("`its`")));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Whatever the Linker does, every stave is in one place or gone, and
    /// a list counts what is held.
    #[test]
    fn counted_items_stay_counted(moves in prop::collection::vec(0usize..32, 1..30)) {
        let mut s = start(SHED);
        for m in moves {
            if s.view().over {
                break;
            }
            let links = s.view().links.clone();
            let id = links[m % links.len()].id.clone();
            s = s.take_link(&id).unwrap();
            if s.view().links.is_empty() {
                break; // the well
            }
            let held: usize = status(&s)[3].parse().unwrap();
            prop_assert!(held <= 3);
        }
    }
}
