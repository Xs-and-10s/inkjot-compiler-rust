//! Ex 9's promise: no dead ends. Whatever a reader does (burns the
//! ladderway, wears out the winze, loses the tools to the Gleaner, burns
//! the timber yard with the staves in it, spends every crystal), Agnes
//! Thwaite can still be brought out.
//!
//! The draft can't run yet, so this is a model of its map and the moves
//! that change it, read from the draft itself: places and their tags, ways
//! and what opens them, items and where they lie, what each spell costs
//! and where it is learned. The mechanics (what a spell does) are written
//! here, and the draft's lines for them are checked first.
//!
//! Each case plays at random for a while, with fires, thefts and shafts
//! falling, and then asks a search: can the goal still be reached? It asks
//! twice, once with the pick and once without, because the pick can break
//! and must never be the only way through.

use std::collections::{HashMap, HashSet, VecDeque};

fn draft() -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    std::fs::read_to_string(format!("{dir}/examples/drafts/ex9-saltmarrow.twee")).unwrap()
}

/// The lines of the draft whose meaning the model writes out by hand.
#[test]
fn the_model_follows_the_draft() {
    let src = draft();
    for line in [
        "archetype shaft ~ timbered",
        "~ state crossings: 3",
        "on enter shaft when entered.burnt",
        "~ then lower entered.crossings 1",
        "on leave shaft when left.burnt and left.crossings == 0",
        "~ then raise here.crossings 3",
        "~ then set target.crossings: 1",
        "~ then lower stair_water 2 when here in [brine_pans, sump]",
        "~ then lower rubble 2",
        "~ then set target.lightened: yes ~ for 6 turns",
        "get way_free: thwaite.lightened or me.holds.stretcher or warden.paid",
        "recipe stretcher: stave, stave, sacking",
        "recipe pawl: horseshoe ~ using hammer",
        "~ breaks 1 in 6",
        "~ along way ~ within 1",
        "destroy all(item, in: ablaze, where: it.burns)",
        "raise crusting.crust 1 when clock.now % 4 == 0",
        "set stair_water: 0",
        "lose stave 2",
        "gain salt 2",
        "gain salt 3",
        "learn boilers_charm",
    ] {
        assert!(src.contains(line), "the draft no longer says `{line}`");
    }
}

// ---------------------------------------------------------------- the map

#[derive(Clone, Copy, Debug, PartialEq)]
enum Cond {
    Always,
    NotFallen(usize),
    CageFixed,
    StairDry,
    RubbleClear,
    PlanksOut,
}

struct Way {
    a: usize,
    b: usize,
    one_way: bool,
    open: Cond,
}

#[derive(Default)]
struct Place {
    name: String,
    timbered: bool,
    shaft: bool,
    salted: bool,
    crusts: bool,
    way_out: bool,
    depth: u32,
}

struct Map {
    places: Vec<Place>,
    ways: Vec<Way>,
    /// Spell name to (cost, the facts that teach it and its parts).
    spells: HashMap<String, (u8, Vec<String>)>,
    /// Fact name to the place where a passage teaches it ("" for the start).
    taught_at: HashMap<String, String>,
    /// Item name to the place it starts in.
    home: HashMap<String, String>,
}

impl Map {
    fn id(&self, name: &str) -> usize {
        self.places
            .iter()
            .position(|p| p.name == name)
            .unwrap_or_else(|| panic!("no place `{name}`"))
    }
    fn shafts(&self) -> Vec<usize> {
        (0..self.places.len())
            .filter(|&i| self.places[i].shaft)
            .collect()
    }
    /// Upper workings, where the Gleaner wanders and trades.
    fn upper(&self, p: usize) -> bool {
        let p = &self.places[p];
        p.salted && p.depth < 300 && !p.shaft
    }
    fn next_to(&self, a: usize, b: usize) -> bool {
        self.ways
            .iter()
            .any(|w| (w.a == a && w.b == b) || (w.a == b && w.b == a))
    }
}

fn word_after<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let i = s.find(key)? + key.len();
    s[i..].split([' ', ',', '~']).find(|w| !w.is_empty())
}

fn parse(src: &str) -> Map {
    let mut places = Vec::new();
    let mut raw_ways = Vec::new();
    let mut costs: HashMap<String, u8> = HashMap::new();
    let mut parts: HashMap<String, Vec<String>> = HashMap::new();
    let mut learned_from: HashMap<String, String> = HashMap::new();
    let mut taught_at = HashMap::new();
    let mut home = HashMap::new();
    let mut passage = String::new();
    let mut in_comment = false;
    for line in src.lines() {
        // Author's comments are prose, even when a line starts "place …".
        if line.trim() == "(((" {
            in_comment = true;
        }
        if in_comment {
            in_comment = line.trim() != ")))";
            continue;
        }
        if let Some(h) = line.strip_prefix(":: ") {
            passage = h.split([' ', '[']).next().unwrap_or("").to_string();
            continue;
        }
        if let Some(rest) = line.strip_prefix("place ") {
            let (name, tail) = rest.split_once(':').unwrap();
            places.push(Place {
                name: name.to_string(),
                timbered: tail.contains("timbered") || tail.contains("like shaft"),
                shaft: tail.contains("like shaft"),
                salted: tail.contains("salted"),
                crusts: tail.contains("~ crusts"),
                way_out: tail.contains("way_out"),
                depth: word_after(tail, "depth: ").map_or(0, |d| d.parse().unwrap()),
            });
        } else if let Some(rest) = line.strip_prefix("way ") {
            let (_, tail) = rest.split_once(':').unwrap();
            let ends = tail.split('~').next().unwrap();
            let (a, b) = ends.split_once(" - ").unwrap();
            let open = tail
                .split_once("open when ")
                .map(|(_, c)| c.split('~').next().unwrap().trim().to_string());
            raw_ways.push((
                a.trim().to_string(),
                b.trim().to_string(),
                tail.contains("one way"),
                open,
            ));
        } else if let Some(rest) = line.strip_prefix("act ") {
            if let Some((name, tail)) = rest.split_once(':') {
                if tail.contains("~ spell") {
                    costs.insert(
                        name.into(),
                        word_after(tail, "cost ").unwrap().parse().unwrap(),
                    );
                    learned_from.insert(
                        name.into(),
                        word_after(tail, "learned from ").unwrap().into(),
                    );
                }
            }
        } else if let Some(rest) = line.strip_prefix("combo ") {
            let (name, tail) = rest.split_once(':').unwrap();
            let list = tail.split('~').next().unwrap();
            parts.insert(
                name.into(),
                list.split(',').map(|p| p.trim().to_string()).collect(),
            );
            learned_from.insert(
                name.into(),
                word_after(tail, "learned from ").unwrap().into(),
            );
        } else if let Some(fact) = line.strip_prefix("learn ") {
            let at = if passage == "prologue" {
                String::new()
            } else {
                passage.split(':').next().unwrap().to_string()
            };
            taught_at.insert(fact.trim().to_string(), at);
        } else if let Some(rest) = line.strip_prefix("item ") {
            let (name, tail) = rest.split_once(':').unwrap();
            if let Some(p) = word_after(tail, "~ in ") {
                home.insert(name.to_string(), p.to_string());
            }
        }
    }
    let mut map = Map {
        places,
        ways: Vec::new(),
        spells: HashMap::new(),
        taught_at,
        home,
    };
    for (a, b, one_way, open) in raw_ways {
        let open = match open.as_deref() {
            None => Cond::Always,
            Some("cage_fixed") => Cond::CageFixed,
            Some("stair_water == 0") => Cond::StairDry,
            Some("rubble == 0") => Cond::RubbleClear,
            Some("planks_out") => Cond::PlanksOut,
            Some(c) if c.starts_with("not ") && c.ends_with(".fallen") => {
                Cond::NotFallen(map.id(&c[4..c.len() - 7]))
            }
            Some(c) => panic!("the model does not know the way condition `{c}`"),
        };
        let (a, b) = (map.id(&a), map.id(&b));
        map.ways.push(Way {
            a,
            b,
            one_way,
            open,
        });
    }
    for (name, fact) in &learned_from {
        let (cost, mut facts) = match parts.get(name) {
            Some(ps) => {
                let mut facts = Vec::new();
                let mut cost = 0;
                for p in ps {
                    cost += costs[p];
                    facts.push(learned_from[p].clone());
                }
                (cost, facts)
            }
            None => (costs[name], Vec::new()),
        };
        facts.push(fact.clone());
        map.spells.insert(name.clone(), (cost, facts));
    }
    map
}

#[test]
fn the_map_is_what_the_draft_says() {
    let map = parse(&draft());
    assert_eq!(map.places.len(), 15);
    let timbered = map.places.iter().filter(|p| p.timbered).count();
    assert!(
        (3..=5).contains(&timbered),
        "about a quarter is timbered: {timbered}"
    );
    assert_eq!(map.shafts().len(), 2);
    assert_eq!(map.spells.len(), 19, "nineteen spells");
    assert_eq!(map.spells["bridge"].0, 5, "a combo costs its parts");
    assert_eq!(
        map.taught_at["boilers_charm"], "marrow",
        "the last teacher of draw"
    );
}

// ---------------------------------------------------------------- the state

const PICK: usize = 0;
const HAMMER: usize = 1;
const SPANNER: usize = 2;
const HORSESHOE: usize = 3;
const SACKING: usize = 4;
const PAWL: usize = 5;
const STRETCHER: usize = 6;
const ITEMS: [&str; 5] = ["pick", "hammer", "spanner", "horseshoe", "sacking"];

/// Where an item is.
const HOME: u8 = 0;
const HELD: u8 = 1;
const GLEANER: u8 = 2;
const GONE: u8 = 3;

/// A shaft: standing, burnt with n crossings left (1 + n), or fallen.
const STANDING: u8 = 0;
const FALLEN: u8 = 9;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct State {
    pos: u8,
    salt: u8,
    known: Vec<bool>,
    items: [u8; 7],
    pick_broken: bool,
    staves_yard: u8,
    staves_held: u8,
    shafts: Vec<u8>,
    cage: bool,
    stair: u8,
    rubble: u8,
    propped: bool,
    planks: bool,
    veins: u8,
    found: bool,
    satchel: bool,
    thwaite: u8,
    with: bool,
    light: u8,
}

/// What changes only in the random part: fire, gas, the Gleaner.
#[derive(Clone)]
struct World {
    s: State,
    heat: Vec<u8>,
    burnt: Vec<bool>,
    gas: u8,
    turn: u32,
}

struct Game<'a> {
    map: &'a Map,
    facts: Vec<String>,
    item_home: [usize; 5],
    yard: usize,
}

impl<'a> Game<'a> {
    fn new(map: &'a Map) -> Self {
        let mut facts: Vec<String> = map.taught_at.keys().cloned().collect();
        facts.sort();
        let item_home = ITEMS.map(|i| map.id(&map.home[i]));
        Game {
            map,
            facts,
            item_home,
            yard: map.id(&map.home["stave"]),
        }
    }

    fn start(&self) -> State {
        let map = self.map;
        State {
            pos: map.id("pithead") as u8,
            salt: 4,
            known: self
                .facts
                .iter()
                .map(|f| map.taught_at[f].is_empty())
                .collect(),
            items: [HOME; 7],
            pick_broken: false,
            staves_yard: 4,
            staves_held: 0,
            shafts: vec![STANDING; map.shafts().len()],
            cage: false,
            stair: 4,
            rubble: 3,
            propped: false,
            planks: false,
            veins: 3,
            found: false,
            satchel: false,
            thwaite: map.id("marrow") as u8,
            with: false,
            light: 0,
        }
    }

    fn knows(&self, s: &State, spell: &str) -> bool {
        self.map.spells[spell]
            .1
            .iter()
            .all(|f| s.known[self.facts.iter().position(|g| g == f).unwrap()])
    }

    /// Cast a spell if it is known and affordable.
    fn cast(&self, s: &State, spell: &str) -> Option<State> {
        let cost = self.map.spells[spell].0;
        (self.knows(s, spell) && s.salt >= cost).then(|| {
            let mut t = s.clone();
            t.salt -= cost;
            t
        })
    }

    fn shaft_index(&self, p: usize) -> Option<usize> {
        self.map.shafts().iter().position(|&q| q == p)
    }

    fn open(&self, s: &State, w: &Way) -> bool {
        match w.open {
            Cond::Always => true,
            Cond::NotFallen(p) => s.shafts[self.shaft_index(p).unwrap()] != FALLEN,
            Cond::CageFixed => s.cage,
            Cond::StairDry => s.stair == 0,
            Cond::RubbleClear => s.rubble == 0,
            Cond::PlanksOut => s.planks,
        }
    }

    /// Walk to `to`, with the shafts' crossings, and Thwaite if she can come.
    fn walk(&self, s: &State, to: usize) -> State {
        let mut t = s.clone();
        let from = s.pos as usize;
        if let Some(k) = self.shaft_index(from) {
            if t.shafts[k] == 1 {
                t.shafts[k] = FALLEN;
            }
        }
        if let Some(k) = self.shaft_index(to) {
            if (1..FALLEN).contains(&t.shafts[k]) {
                t.shafts[k] -= 1;
            }
        }
        let carried = t.items[STRETCHER] == HELD || t.light > 0;
        if t.with && t.thwaite as usize == from && carried {
            t.thwaite = to as u8;
            t.light = t.light.saturating_sub(1);
        }
        t.pos = to as u8;
        t
    }

    /// Every move that does not set fire to anything. `no_pick` leaves the
    /// pick out, as if it had broken for good.
    fn moves(&self, s: &State, no_pick: bool) -> Vec<State> {
        let map = self.map;
        let here = s.pos as usize;
        let name = map.places[here].name.as_str();
        let mut out = Vec::new();
        for w in &map.ways {
            if !self.open(s, w) {
                continue;
            }
            if w.a == here {
                out.push(self.walk(s, w.b));
            } else if w.b == here && !w.one_way {
                out.push(self.walk(s, w.a));
            }
        }
        // Learning: every fact a passage in this place teaches.
        let teaches: Vec<usize> = (0..self.facts.len())
            .filter(|&i| map.taught_at[&self.facts[i]] == name && !s.known[i])
            .collect();
        if !teaches.is_empty() && (name != "marrow" || s.found) {
            let mut t = s.clone();
            for i in teaches {
                t.known[i] = true;
            }
            if name == "gallery" && !t.satchel {
                t.satchel = true;
                t.salt = (t.salt + 2).min(12);
            }
            out.push(t);
        }
        // Items.
        for (k, &home) in self.item_home.iter().enumerate() {
            if k == PICK && no_pick {
                continue;
            }
            if s.items[k] == HOME && home == here {
                let mut t = s.clone();
                t.items[k] = HELD;
                out.push(t);
            }
            if s.items[k] == GLEANER && map.upper(here) && s.salt >= 2 {
                let mut t = s.clone();
                t.items[k] = HELD;
                t.salt -= 2;
                out.push(t);
            }
        }
        if here == self.yard && s.staves_yard > 0 {
            let mut t = s.clone();
            t.staves_yard -= 1;
            t.staves_held += 1;
            out.push(t);
        }
        // Making things.
        if s.items[HORSESHOE] == HELD && s.items[HAMMER] == HELD && s.items[PAWL] == HOME {
            let mut t = s.clone();
            t.items[HORSESHOE] = GONE;
            t.items[PAWL] = HELD;
            out.push(t);
        }
        if s.staves_held >= 2 && s.items[SACKING] == HELD && s.items[STRETCHER] == HOME {
            let mut t = s.clone();
            t.staves_held -= 2;
            t.items[SACKING] = GONE;
            t.items[STRETCHER] = HELD;
            out.push(t);
        }
        let pick_ok = !no_pick && s.items[PICK] == HELD && !s.pick_broken;
        // Salt.
        if map.places[here].crusts && s.salt < 12 {
            let mut t = s.clone();
            t.salt += 1;
            out.push(t);
        }
        if name == "gallery" && s.veins > 0 && pick_ok && s.salt < 12 {
            let mut t = s.clone();
            t.veins -= 1;
            t.salt += 1;
            out.push(t);
        }
        if s.pick_broken && s.items[PICK] == HELD && !no_pick {
            if let Some(mut t) = self.cast(s, "knit") {
                t.pick_broken = false;
                out.push(t);
            }
        }
        // The places.
        match name {
            "incline" => {
                if s.items[PAWL] == HELD && !s.cage {
                    let mut t = s.clone();
                    t.cage = true;
                    t.items[PAWL] = GONE;
                    out.push(t);
                }
                if let Some(t) = self.cast(s, "updraught") {
                    out.push(self.walk(&t, map.id("pithead")));
                }
            }
            "brine_pans" | "sump" => {
                if name == "brine_pans" && s.items[SPANNER] == HELD && s.stair > 0 {
                    let mut t = s.clone();
                    t.stair = 0;
                    out.push(t);
                }
                if s.stair > 0 {
                    if let Some(mut t) = self.cast(s, "draw") {
                        t.stair = t.stair.saturating_sub(2);
                        out.push(t);
                    }
                }
            }
            "marrow" => {
                if !s.found {
                    let mut t = s.clone();
                    t.found = true;
                    t.salt = (t.salt + 3).min(12);
                    out.push(t);
                }
                if s.rubble > 0 && !s.propped && s.staves_held >= 2 {
                    let mut t = s.clone();
                    t.staves_held -= 2;
                    t.propped = true;
                    out.push(t);
                }
                if s.rubble > 0 && s.propped && pick_ok {
                    let mut t = s.clone();
                    t.rubble -= 1;
                    out.push(t);
                }
                if s.rubble > 0 {
                    if let Some(mut t) = self.cast(s, "clear") {
                        t.rubble = t.rubble.saturating_sub(2);
                        out.push(t);
                    }
                }
            }
            "adit" if !s.planks => {
                // A hammer, the lamp's flame, or kindle.
                let mut t = s.clone();
                t.planks = true;
                t.stair = 0;
                out.push(t);
            }
            _ => {}
        }
        // Thwaite.
        if s.found && s.thwaite as usize == here {
            if let Some(mut t) = self.cast(s, "lighten") {
                t.light = 6;
                t.with = true;
                out.push(t);
            }
            if s.items[STRETCHER] == HELD && !s.with {
                let mut t = s.clone();
                t.with = true;
                out.push(t);
            }
        }
        // Shafts: shore one that has burnt, bridge one that has fallen.
        for (k, &p) in map.shafts().iter().enumerate() {
            if p == here && (1..FALLEN).contains(&s.shafts[k]) {
                if let Some(mut t) = self.cast(s, "shore") {
                    t.shafts[k] = (t.shafts[k] + 3).min(8);
                    out.push(t);
                }
            }
            if s.shafts[k] == FALLEN && map.next_to(here, p) {
                if let Some(mut t) = self.cast(s, "bridge") {
                    t.shafts[k] = 2;
                    out.push(t);
                }
            }
        }
        out
    }

    fn won(&self, s: &State) -> bool {
        s.found && s.with && s.thwaite == s.pos && self.map.places[s.pos as usize].way_out
    }

    /// Can the goal be reached from here, by any moves at all?
    fn can_win(&self, from: &State, no_pick: bool) -> bool {
        let mut seen = HashSet::new();
        let mut queue = VecDeque::from([from.clone()]);
        seen.insert(from.clone());
        while let Some(s) = queue.pop_front() {
            if self.won(&s) {
                return true;
            }
            for t in self.moves(&s, no_pick) {
                if seen.insert(t.clone()) {
                    queue.push_back(t);
                }
            }
        }
        false
    }
}

// ---------------------------------------------------------------- random play

/// xorshift, as the engine's dice are.
struct Dice(u64);
impl Dice {
    fn pick(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
    fn chance(&mut self, one_in: usize) -> bool {
        self.pick(one_in) == 0
    }
}

/// The world's turn, as `fire:turn` and the Gleaner's margin say.
fn world_turn(g: &Game, w: &mut World, dice: &mut Dice) {
    let map = g.map;
    let n = map.places.len();
    let ablaze: Vec<usize> = (0..n).filter(|&p| w.heat[p] == 3).collect();
    for &p in &ablaze {
        if let Some(q) = (0..n)
            .find(|&q| map.next_to(p, q) && map.places[q].timbered && w.heat[q] == 0 && !w.burnt[q])
        {
            w.heat[q] = 1;
        }
        if p == g.yard {
            w.s.staves_yard = 0;
            if w.s.items[SACKING] == HOME {
                w.s.items[SACKING] = GONE;
            }
        }
        w.heat[p] = 0;
        w.burnt[p] = true;
        if let Some(k) = g.shaft_index(p) {
            if w.s.shafts[k] == STANDING {
                w.s.shafts[k] = 4;
            }
        }
    }
    for p in 0..n {
        if (1..=2).contains(&w.heat[p]) && !ablaze.contains(&p) {
            w.heat[p] += 1;
        }
    }
    // The Gleaner: a loose tool, now and then; their candle, rarely.
    if dice.chance(6) {
        let k = dice.pick(ITEMS.len());
        if w.s.items[k] == HOME && map.upper(g.item_home[k]) && g.item_home[k] != w.s.pos as usize {
            w.s.items[k] = GLEANER;
        }
    }
    if dice.chance(25) {
        let candidates: Vec<usize> = (0..n)
            .filter(|&p| map.upper(p) && map.places[p].timbered && !w.burnt[p] && w.heat[p] == 0)
            .collect();
        if !candidates.is_empty() {
            w.heat[candidates[dice.pick(candidates.len())]] = 1;
        }
    }
    w.turn += 1;
}

/// Mischief a reader might do on purpose: set fires, use up the shafts.
fn mischief(g: &Game, w: &World) -> Vec<World> {
    let map = g.map;
    let here = w.s.pos as usize;
    let mut out = Vec::new();
    if map.places[here].name == "stables" && !w.burnt[here] && w.heat[here] == 0 {
        if let Some(s) = g.cast(&w.s, "kindle") {
            let mut v = w.clone();
            v.s = s;
            v.heat[here] = 1;
            out.push(v);
        }
    }
    for q in 0..map.places.len() {
        if map.next_to(here, q) && map.places[q].timbered && w.heat[q] == 0 && !w.burnt[q] {
            if let Some(s) = g.cast(&w.s, "firebrand") {
                let mut v = w.clone();
                v.s = s;
                v.heat[q] = 1;
                out.push(v);
            }
        }
    }
    if map.places[here].name == "drift" && w.gas > 0 {
        let crib = map.id("cribbing");
        let mut v = w.clone();
        v.gas = 0;
        if !v.burnt[crib] {
            v.heat[crib] = 1;
        }
        out.push(v);
    }
    out
}

/// Play at random, then let every fire burn out.
fn play(g: &Game, seed: u64, steps: usize) -> World {
    let mut dice = Dice(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let n = g.map.places.len();
    let mut w = World {
        s: g.start(),
        heat: vec![0; n],
        burnt: vec![false; n],
        gas: 3,
        turn: 0,
    };
    for _ in 0..steps {
        let mut options: Vec<World> = g
            .moves(&w.s, false)
            .into_iter()
            .filter(|s| w.heat[s.pos as usize] < 2)
            .map(|s| World { s, ..w.clone() })
            .collect();
        options.extend(mischief(g, &w));
        if options.is_empty() {
            world_turn(g, &mut w, &mut dice);
            continue;
        }
        let before = w.clone();
        w = options.swap_remove(dice.pick(options.len()));
        // The pick breaks one time in six when it is used.
        let used_pick = w.s.veins < before.s.veins || (w.s.rubble < before.s.rubble && w.s.propped);
        if used_pick && dice.chance(6) {
            w.s.pick_broken = true;
        }
        world_turn(g, &mut w, &mut dice);
        // Burned: the reader takes an undo.
        if w.heat[w.s.pos as usize] >= 2 {
            w = before;
        }
    }
    while w.heat.iter().any(|&h| h > 0) {
        world_turn(g, &mut w, &mut dice);
    }
    w
}

#[test]
fn from_the_start_both_ways_through() {
    let map = parse(&draft());
    let g = Game::new(&map);
    assert!(g.can_win(&g.start(), false));
    assert!(g.can_win(&g.start(), true), "without the pick");
}

/// The test's own check: random play really does burn and drop things, so
/// the search below is asked about the hard cases.
#[test]
fn random_play_reaches_the_hard_cases() {
    let map = parse(&draft());
    let g = Game::new(&map);
    let (mut fallen, mut yard_burnt, mut stolen) = (0, 0, 0);
    for seed in 1..=60u64 {
        let w = play(&g, seed, 400);
        fallen += w.s.shafts.iter().filter(|&&s| s == FALLEN).count();
        yard_burnt += usize::from(w.burnt[g.yard]);
        stolen += w.s.items.iter().filter(|&&i| i == GLEANER).count();
    }
    assert!(fallen > 0, "no shaft ever fell");
    assert!(yard_burnt > 0, "the timber yard never burned");
    assert!(stolen > 0, "the Gleaner never stole anything");
}

/// No dead ends: wherever random play leaves the reader, she can still be
/// brought out, with the pick or without it.
#[test]
fn there_are_no_dead_ends() {
    let map = parse(&draft());
    let g = Game::new(&map);
    for seed in 1..=60u64 {
        let w = play(&g, seed, 400);
        assert!(g.can_win(&w.s, false), "seed {seed}: stuck at {:?}", w.s);
        assert!(
            g.can_win(&w.s, true),
            "seed {seed}: stuck without the pick at {:?}",
            w.s
        );
    }
}

/// The cost of writing the spellbook in general words: each spell says its
/// numbers twice, once in the book ("Lasts 4 turns", "Power 2") and once in
/// its effect. This checks the two agree, which typed fields would make
/// unnecessary (`examples/drafts/ex9-spellbook-define.twee`).
#[test]
fn the_book_says_what_the_spells_do() {
    let src = draft();
    let mut blocks: Vec<(String, Vec<String>)> = Vec::new();
    for line in src.lines() {
        let is_spell =
            (line.starts_with("act ") || line.starts_with("combo ")) && line.contains("~ spell");
        if is_spell {
            let name = line.split([' ', ':']).nth(1).unwrap().to_string();
            blocks.push((name, Vec::new()));
        } else if line.starts_with("~ ") {
            if let Some((_, lines)) = blocks.last_mut() {
                lines.push(line.to_string());
            }
        } else if !blocks.is_empty() && !line.starts_with("~") {
            blocks.push((String::new(), Vec::new()));
        }
    }
    let number_after = |s: &str, key: &str| -> Option<String> {
        let i = s.find(key)? + key.len();
        Some(s[i..].chars().take_while(char::is_ascii_digit).collect())
    };
    let mut checked = 0;
    for (name, lines) in blocks.iter().filter(|(n, _)| !n.is_empty()) {
        let book = lines.iter().find(|l| l.starts_with("~ \"")).unwrap();
        let effects: Vec<&String> = lines.iter().filter(|l| l.starts_with("~ then")).collect();
        if let Some(n) = number_after(book, "Lasts ") {
            let says = format!("~ for {n} turns");
            assert!(
                effects.iter().any(|e| e.contains(&says)),
                "{name}: the book says it lasts {n} turns, and no effect does"
            );
            checked += 1;
        }
        if let Some(n) = number_after(book, "Power ") {
            let said = effects.iter().any(|e| {
                e.split(' ').any(|w| w == n) && (e.contains("lower ") || e.contains("raise "))
            });
            assert!(said, "{name}: the book says power {n}, and no effect does");
            checked += 1;
        }
    }
    assert!(checked >= 10, "only {checked} numbers checked");
}

/// Every spell in the book has its Link, and every book Link names a spell.
#[test]
fn every_spell_is_in_the_book() {
    let src = draft();
    let map = parse(&src);
    let start = src.find("> ~ for apprentice ~ everywhere").unwrap();
    let group: Vec<&str> = src[start..]
        .lines()
        .skip(1)
        .take_while(|l| l.starts_with("> [["))
        .collect();
    let named: HashSet<String> = group
        .iter()
        .map(|l| l.rsplit("~ ").next().unwrap().trim().to_string())
        .collect();
    let spells: HashSet<String> = map.spells.keys().cloned().collect();
    assert_eq!(named, spells, "the book's Links and the spells differ");
}

/// Zork's ranks cover every score from 0 to 100 exactly once.
#[test]
fn the_ranks_cover_every_score() {
    let src = draft();
    let start = src.find("table rank: score").unwrap();
    let mut covered = [0u8; 101];
    for row in src[start..]
        .lines()
        .skip(1)
        .take_while(|l| l.starts_with("~ "))
    {
        let range = row[2..].split(':').next().unwrap();
        let (lo, hi) = match range.split_once("..=") {
            Some((a, b)) => (a.parse::<usize>().unwrap(), b.parse::<usize>().unwrap()),
            None => {
                let n = range.parse::<usize>().unwrap();
                (n, n)
            }
        };
        for c in &mut covered[lo..=hi] {
            *c += 1;
        }
    }
    assert!(
        covered.iter().all(|&c| c == 1),
        "a score with no rank, or two"
    );
}

/// The best game scores exactly 100: every award once, the planks by
/// whichever way they go, and the three treasures in the chest.
#[test]
fn the_best_score_is_a_hundred() {
    let src = draft();
    let mut passage = "";
    let mut awards: HashMap<&str, u32> = HashMap::new();
    for line in src.lines() {
        if let Some(h) = line.strip_prefix(":: ") {
            passage = h.split(' ').next().unwrap();
        } else if let Some(rest) = line.strip_prefix("raise score ") {
            let n: u32 = rest.split(' ').next().unwrap().parse().unwrap();
            *awards.entry(passage).or_default() += n;
        }
    }
    // Ways to the same end score once.
    let planks = ["adit:planks", "adit:burn", "spell:kindle"];
    for p in planks {
        assert_eq!(awards.get(p), Some(&5), "{p} scores the planks");
    }
    let mut best: u32 = awards
        .iter()
        .filter(|(p, _)| !planks.contains(p) && **p != "pithead:chest")
        .map(|(_, n)| n)
        .sum();
    best += 5;
    let treasures = src
        .lines()
        .filter(|l| l.starts_with("item ") && l.contains("~ like treasure"))
        .count() as u32;
    best += awards["pithead:chest"] * treasures;
    assert_eq!(best, 100);
}
