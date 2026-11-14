//! Several Linkers at once: a cohort playing one story, stop by stop.
//!
//! Each Linker sits at a *seat* and plays one character. Within a stop they
//! act independently, in any order:
//!
//! - A **private** act (`examine`, `ask`) happens at once, and only its
//!   actor sees the result.
//! - A **shared** act with a capacity (`take`, `force`) is a *claim*. It is
//!   settled as soon as no one else in the place could still reach for the
//!   same thing; if more reach than there is room for, they *clash*, and the
//!   Link's `clash` rule decides who gets it.
//! - An act that **ends the turn** (`walk`, `dash`, `slip`, `observe`,
//!   staying) *commits* the Linker's move. They wait.
//!
//! When everyone has committed, the stop resolves, in the order the paper
//! playtest used: open claims settle; every Linker moves, and those who end
//! in the same place see each other come and go; the turn ends, which moves
//! the NPCs; and each Linker arrives at their cell for the next stop.
//!
//! The interpreter keeps the Linker being served in [`Run`]'s own fields
//! (`pov`, `links`, `output`, `locals`). The others wait in [`Seat`]s, and
//! [`Run::sit`] swaps them, like a context switch.

use std::collections::HashMap;

use super::agenda::Walker;
use super::{f, place_of, strs, Local, Run, R, V};
use crate::json::Value;

/// A Linker who is not the one being served right now.
#[derive(Default)]
pub(super) struct Seat {
    links: Vec<Value>,
    output: Vec<String>,
    locals: HashMap<String, Local>,
    asking: Vec<Value>,
    paused: Option<super::inputs::Paused>,
}

/// A shared act waiting to be settled.
pub(super) struct Claim {
    /// The character reaching for it.
    who: String,
    link: Value,
    place: String,
}

impl Claim {
    /// Two claims contend if they are in the same place and reach for the
    /// same thing: the same item, or else the same Link.
    fn key(&self) -> (String, String) {
        let thing = f(&self.link, "arg")
            .as_str()
            .or(f(&self.link, "id").as_str())
            .unwrap_or("")
            .to_string();
        (self.place.clone(), thing)
    }
}

/// Who gets the thing when `claims` Linkers reach for something with room
/// for `capacity` (`None`: room for everyone). One `(success, clashed)` per
/// claimant, in the order they claimed. `roll(n)` gives a number in `1..=n`.
///
/// | `clash` | Over capacity |
/// |---|---|
/// | `random_winner` (default) | `capacity` claimants, picked at random, succeed |
/// | `random` | each claimant succeeds on a coin flip |
/// | `all` | everyone succeeds |
/// | `none` | no one succeeds |
pub(super) fn verdicts(
    claims: usize,
    capacity: Option<usize>,
    rule: &str,
    roll: &mut impl FnMut(u64) -> u64,
) -> Vec<(bool, bool)> {
    if capacity.is_none_or(|room| claims <= room) {
        return vec![(true, false); claims];
    }
    let room = capacity.unwrap_or(0);
    match rule {
        "all" => vec![(true, true); claims],
        "none" => vec![(false, true); claims],
        "random" => (0..claims).map(|_| (roll(2) == 1, true)).collect(),
        _ => {
            // Shuffle the claimants (Fisher–Yates) and the first `room` win.
            let mut order: Vec<usize> = (0..claims).collect();
            for i in (1..claims).rev() {
                let j = roll(i as u64 + 1) as usize - 1;
                order.swap(i, j);
            }
            let mut out = vec![(false, true); claims];
            for &winner in &order[..room] {
                out[winner].0 = true;
            }
            out
        }
    }
}

impl Run<'_> {
    /// Seat several Linkers, one per character, all starting at `passage`
    /// (or each at their own `start:`). The first is served first.
    pub fn start_cohort(&mut self, passage: Option<&str>, who: &[&str]) -> R<()> {
        self.order = who.iter().map(|w| w.to_string()).collect();
        // Everyone is in place before anyone looks around.
        if let Some(at) = passage {
            for w in who {
                self.at.insert(w.to_string(), at.to_string());
            }
        }
        for (i, w) in who.iter().enumerate() {
            if i > 0 {
                self.park();
            }
            self.pov = w.to_string();
            self.start(passage, Some(w))?;
        }
        // The log begins with everyone seated.
        self.history = vec![self.snapshot()];
        self.sit(who[0])
    }

    /// The characters at the table, in seating order. Empty for a run with
    /// one Linker.
    pub fn seated(&self) -> &[String] {
        &self.order
    }

    /// Serve the Linker playing `who`: their Links, text and names.
    pub fn sit(&mut self, who: &str) -> R<()> {
        if who == self.pov {
            return Ok(());
        }
        let seat = self
            .parked
            .remove(who)
            .ok_or_else(|| format!("no Linker is playing `{who}`"))?;
        self.park();
        self.pov = who.to_string();
        self.links = seat.links;
        self.output = seat.output;
        self.locals = seat.locals;
        self.asking = seat.asking;
        self.paused = seat.paused;
        Ok(())
    }

    /// Has the Linker being served locked in their move for this stop?
    pub fn waiting(&self) -> bool {
        self.committed.contains_key(&self.pov)
    }

    /// Put the Linker being served back in their seat.
    fn park(&mut self) {
        let seat = Seat {
            links: std::mem::take(&mut self.links),
            output: std::mem::take(&mut self.output),
            locals: std::mem::take(&mut self.locals),
            asking: std::mem::take(&mut self.asking),
            paused: self.paused.take(),
        };
        self.parked.insert(self.pov.clone(), seat);
    }

    /// Show a line to one Linker, whether or not they are being served.
    fn tell(&mut self, who: &str, line: String) {
        if who == self.pov {
            self.output.push(line);
        } else if let Some(seat) = self.parked.get_mut(who) {
            seat.output.push(line);
        }
    }

    /// Does this Link end the turn: a move, staying, or an act that says so?
    pub(super) fn ends_turn(&self, link: &Value) -> bool {
        let act = f(link, "act").as_str().unwrap_or("");
        f(link, "ends_turn") == &Value::Bool(true)
            || self.story.field(act, "ends_turn") == &Value::Bool(true)
    }

    /// Is this Link a claim: a shared act with only so much room?
    pub(super) fn is_claim(&self, link: &Value) -> bool {
        f(link, "scope").as_str() == Some("shared") && f(link, "capacity").as_u64().is_some()
    }

    /// Lock in the Linker's move for this stop, and resolve the stop if
    /// everyone has.
    pub(super) fn commit(&mut self, link: &Value, paid: bool) -> R<()> {
        if !paid {
            self.pay(link)?;
        }
        self.committed.insert(self.pov.clone(), link.clone());
        self.settle(false)?;
        if self.everyone_committed() {
            self.resolve_stop()
        } else {
            self.output.push("(Waiting for the others.)".into());
            Ok(())
        }
    }

    /// Reach for something shared. It is settled at once if no one else
    /// here could still reach for it; otherwise when they have chosen.
    pub(super) fn claim(&mut self, link: &Value) -> R<()> {
        self.pay(link)?;
        let place = place_of(self.here()).to_string();
        self.claims.push(Claim {
            who: self.pov.clone(),
            link: link.clone(),
            place,
        });
        let before = self.claims.len();
        self.settle(false)?;
        if self.claims.len() == before {
            self.output
                .push("(Waiting to see whether anyone else here reaches for it.)".into());
        }
        Ok(())
    }

    /// Has the Linker already reached for what this Link reaches for?
    pub(super) fn claimed(&self, link: &Value) -> bool {
        let here = place_of(self.here());
        let probe = Claim {
            who: String::new(),
            link: link.clone(),
            place: here.to_string(),
        };
        self.claims
            .iter()
            .any(|c| c.who == self.pov && c.key() == probe.key())
    }

    fn everyone_committed(&self) -> bool {
        self.linkers()
            .iter()
            .all(|l| self.committed.contains_key(l))
    }

    /// The Linkers in `place`: the one being served first, then the others
    /// in seating order.
    pub(super) fn linkers_here(&self, place: &str) -> Vec<String> {
        let mut here = vec![self.pov.clone()];
        for other in &self.order {
            if *other != self.pov && self.location(other) == place {
                here.push(other.clone());
            }
        }
        here
    }

    /// The moves a Linker may make from here in a world of stops: walk,
    /// dash and slip as far as their character's `moves` allow, staying
    /// put, and observing.
    pub(super) fn move_links(&mut self) -> R<Vec<Value>> {
        let here = place_of(self.here()).to_string();
        let moves = self.moves_of(&self.pov.clone());
        let mut places: Vec<(String, usize)> = self
            .distances(&here, None, Walker::Linker)?
            .into_iter()
            .filter(|(_, d)| *d > 0)
            .collect();
        places.sort_by(|a, b| (a.1, &a.0).cmp(&(b.1, &b.0)));
        let mut links = Vec::new();
        for (act, verb) in [("walk", "Go to"), ("dash", "Dash to"), ("slip", "Slip to")] {
            if !moves.iter().any(|m| m == act) || self.story.kind(act) != Some("act") {
                continue;
            }
            let (lo, hi) = match self.constant(self.story.field(act, "steps")) {
                V::Num(n) => (n as usize, n as usize),
                V::Range { lo, hi, .. } => (lo.unwrap_or(1.0) as usize, hi.unwrap_or(1.0) as usize),
                _ => (1, 1),
            };
            for (to, d) in &places {
                if *d < lo || *d > hi || self.arrival(to).is_none() {
                    continue;
                }
                let route = self
                    .route(&here, to, None, Walker::Linker)?
                    .unwrap_or_default();
                let passed: Vec<Value> = route[..route.len().saturating_sub(1)]
                    .iter()
                    .map(|p| Value::str(p.clone()))
                    .collect();
                let way = self
                    .story
                    .ways
                    .iter()
                    .find(|(_, a, b)| *a == here && b == to)
                    .map_or(String::new(), |(w, _, _)| w.clone());
                let requires = strs(self.story.field(act, "requires"));
                links.push(Value::obj([
                    ("type", Value::str("link")),
                    ("id", Value::str(format!("{act}:{to}"))),
                    ("way", Value::str(way)),
                    ("label", Value::str(format!("{verb} {}", self.label(to)))),
                    ("target", Value::str(to.clone())),
                    ("kind", Value::str("walk")),
                    ("act", Value::str(act)),
                    ("cost", self.cost_for_steps(act, *d)),
                    ("ends_turn", Value::Bool(true)),
                    ("via", Value::Arr(passed)),
                    (
                        "requires_full",
                        Value::Bool(requires.iter().any(|r| r == "full")),
                    ),
                ]));
            }
        }
        if self.arrival(&here).is_some() || self.finale_due() {
            links.push(Value::obj([
                ("type", Value::str("link")),
                ("id", Value::str("stay")),
                ("label", Value::str("Stay")),
                ("target", Value::str(here.clone())),
                ("kind", Value::str("stay")),
                ("ends_turn", Value::Bool(true)),
            ]));
            if self.story.kind("observe") == Some("act") {
                links.push(Value::obj([
                    ("type", Value::str("link")),
                    ("id", Value::str("observe")),
                    ("label", Value::str("Observe")),
                    ("target", Value::str(here)),
                    ("kind", Value::str("stay")),
                    ("act", Value::str("observe")),
                    ("cost", self.story.field("observe", "cost").clone()),
                    ("min", self.story.field("observe", "min").clone()),
                    ("ends_turn", Value::Bool(true)),
                ]));
            }
        }
        Ok(links)
    }

    /// The acts a character may move by: its own `moves`, the config's
    /// `default moves`, or just `walk`.
    fn moves_of(&self, who: &str) -> Vec<String> {
        match self
            .story
            .decls
            .get(who)
            .and_then(|d| d.fields.get("moves"))
        {
            Some(own) => strs(own),
            None => match self.story.defaults.get("moves") {
                Some(default) => strs(default),
                None => vec![String::from("walk")],
            },
        }
    }

    /// An act's cost for a move of `steps`: `cost 1 per step` multiplies.
    fn cost_for_steps(&self, act: &str, steps: usize) -> Value {
        let entries = self
            .story
            .field(act, "cost")
            .as_arr()
            .unwrap_or(&[])
            .to_vec();
        Value::Arr(
            entries
                .into_iter()
                .map(|e| {
                    if f(&e, "per").as_str() != Some("step") {
                        return e;
                    }
                    let each = match f(&e, "vults") {
                        Value::Num(n) => n.parse::<f64>().unwrap_or(0.0),
                        _ => 0.0,
                    };
                    Value::obj([("vults", Value::Num(format!("{}", each * steps as f64)))])
                })
                .collect(),
        )
    }

    /// Everyone playing: the seated cohort, or the one pov.
    fn linkers(&self) -> Vec<String> {
        if self.order.is_empty() {
            vec![self.pov.clone()]
        } else {
            self.order.clone()
        }
    }

    /// Settle every claim that can be settled: all of them if `force`, or
    /// else those no one else in the place could still contend for.
    fn settle(&mut self, force: bool) -> R<()> {
        let mut keys: Vec<(String, String)> = Vec::new();
        for c in &self.claims {
            if !keys.contains(&c.key()) {
                keys.push(c.key());
            }
        }
        for key in keys {
            let claimants: Vec<String> = self
                .claims
                .iter()
                .filter(|c| c.key() == key)
                .map(|c| c.who.clone())
                .collect();
            let rivals = self.linkers().into_iter().any(|l| {
                !claimants.contains(&l)
                    && !self.committed.contains_key(&l)
                    && self.location(&l) == key.0
            });
            if rivals && !force {
                continue;
            }
            self.resolve_claims(&key, &claimants)?;
        }
        Ok(())
    }

    /// Everyone who reached for the same thing finds out what happened.
    fn resolve_claims(&mut self, key: &(String, String), claimants: &[String]) -> R<()> {
        let link = self
            .claims
            .iter()
            .find(|c| c.key() == *key)
            .map(|c| c.link.clone())
            .unwrap_or(Value::Null);
        self.claims.retain(|c| c.key() != *key);
        let capacity = f(&link, "capacity").as_u64().map(|n| n as usize);
        let rule = f(&link, "clash")
            .as_str()
            .unwrap_or("random_winner")
            .to_string();
        let mut roll = |n| self.roll(n);
        let outcomes = verdicts(claimants.len(), capacity, &rule, &mut roll);
        let served = self.pov.clone();
        for (who, (success, clashed)) in claimants.iter().zip(outcomes) {
            self.sit(who)?;
            let others: Vec<String> = claimants
                .iter()
                .filter(|c| *c != who)
                .map(|c| self.label(c))
                .collect();
            self.claimants = others.join(" and ");
            self.verdict = Some((success, clashed));
            let link = link.clone();
            self.perform(&link, true)?;
            self.echo_act(who, &link, &key.0);
        }
        self.claimants.clear();
        self.sit(&served)
    }

    /// Others in the place see a shared act done.
    fn echo_act(&mut self, who: &str, link: &Value, place: &str) {
        if self.vals.get("attempt.success") != Some(&V::Bool(true)) {
            return;
        }
        let name = self.label(who);
        let line = match (f(link, "act").as_str(), f(link, "arg").as_str()) {
            (Some("take"), Some(item)) => format!("{name} takes {}.", self.label(item)),
            _ => format!("{name}: “{}”", f(link, "label").as_str().unwrap_or("")),
        };
        for other in self.linkers() {
            if other != who && self.location(&other) == place {
                self.tell(&other, line.clone());
            }
        }
    }

    /// Everyone has moved: resolve the stop.
    fn resolve_stop(&mut self) -> R<()> {
        let served = self.pov.clone();
        self.settle(true)?;

        // Was this the last stop? Then the finale follows it.
        let ending = self.finale_due();

        // Every Linker moves, at the same moment.
        let mut moves = Vec::new();
        for who in self.linkers() {
            let link = self.committed.get(&who).cloned().unwrap_or(Value::Null);
            self.sit(&who)?;
            let passage = self.here().to_string();
            let from = place_of(&passage).to_string();
            let to = self.destination(&link)?.unwrap_or_else(|| from.clone());
            self.at.insert(who.clone(), to.clone());
            moves.push((who, from, to, link, passage));
        }
        self.committed.clear();
        for (who, from, to, link, _) in &moves {
            self.echo_move(who, from, to, link);
        }

        // The turn ends: the clock, refills, fuses, then the NPCs.
        self.end_turn()?;
        // A dash leaves its mark on the places it passed: `here.passed`.
        for (who, _, _, link, _) in &moves {
            for place in strs(f(link, "via")) {
                self.passed
                    .entry((self.chronon, place))
                    .or_default()
                    .push(who.clone());
            }
        }

        // Each Linker arrives at their cell for the new stop.
        let finale = if ending { self.finale() } else { None };
        for (who, _, to, _, passage) in moves {
            self.sit(&who)?;
            // The clock has run out: everyone goes to the finale.
            if let Some(finale) = &finale {
                self.enter(finale, HashMap::new())?;
                continue;
            }
            match self.arrival_at(&to, self.chronon) {
                Some(cell) => self.enter(&cell, HashMap::new())?,
                // Past the last stop (an act that ends the turn, taken at
                // the end): they stay in the passage they were in.
                None => {
                    self.at.insert(who.clone(), passage);
                    self.relist()?;
                }
            }
        }
        self.sit(&served)
    }

    /// Where a committed move takes the Linker, after any `cross`
    /// handlers have had their say. `None`: they stay.
    fn destination(&mut self, link: &Value) -> R<Option<String>> {
        let to = match f(link, "kind").as_str() {
            Some("walk") => f(link, "target").as_str().map(str::to_string),
            _ => None,
        };
        let way = f(link, "way").as_str().unwrap_or("").to_string();
        if to.is_some() && !way.is_empty() {
            let actor = self.pov.clone();
            if self.fire(super::Event {
                on: "cross",
                what: &way,
                actor: &actor,
                before: true,
            })? {
                return Ok(None);
            }
            self.fire(super::Event {
                on: "cross",
                what: &way,
                actor: &actor,
                before: false,
            })?;
        }
        let act = f(link, "act").as_str().unwrap_or("").to_string();
        if self.story.has_tag(&act, "loud") {
            let actor = self.pov.clone();
            self.fire(super::Event {
                on: "loud",
                what: &act,
                actor: &actor,
                before: false,
            })?;
        }
        Ok(to)
    }

    /// Who sees a Linker come and go. Moves are simultaneous, so it is
    /// whoever *ends* the stop in the place: a walk is seen leaving and
    /// arriving; a loud move (a dash) is seen everywhere it passes too; a
    /// quiet one (a slip) only where it arrives.
    fn echo_move(&mut self, who: &str, from: &str, to: &str, link: &Value) {
        if from == to {
            return;
        }
        let act = f(link, "act").as_str().unwrap_or("walk").to_string();
        let quiet = self.story.has_tag(&act, "quiet");
        let loud = self.story.has_tag(&act, "loud");
        let name = self.label(who);
        let passed: Vec<String> = strs_of(f(link, "via"));
        for other in self.linkers() {
            if other == who {
                continue;
            }
            let at = self.location(&other);
            let line = if at == to {
                if quiet {
                    format!("{name} slips in.")
                } else if loud {
                    format!("{name} comes in at a run from {}.", self.label(from))
                } else {
                    format!("{name} comes in from {}.", self.label(from))
                }
            } else if at == from && !quiet {
                format!("{name} leaves for {}.", self.label(to))
            } else if loud && passed.contains(&at) {
                format!("{name} runs through, toward {}.", self.label(to))
            } else {
                continue;
            };
            self.tell(&other, line);
        }
    }

    /// At the end of each turn, every NPC notes who is in its place and what
    /// they hold. `x.last_seen(holding: tag)` reads these sightings.
    pub(super) fn look(&mut self) {
        for npc in self.story.ids_of("npc") {
            let place = self.location(&npc);
            if place.is_empty() {
                continue;
            }
            let mut seen = Vec::new();
            for who in self
                .story
                .ids_of("character")
                .into_iter()
                .chain(self.story.ids_of("npc"))
            {
                if who != npc && self.location(&who) == place {
                    let held: Vec<String> = self
                        .holder
                        .iter()
                        .filter(|(_, h)| **h == who)
                        .map(|(item, _)| item.clone())
                        .collect();
                    seen.push((who, held));
                }
            }
            let log = self.sightings.entry(npc).or_default();
            log.push((self.chronon, seen));
        }
    }
}

/// The strings of a JSON array, or none.
fn strs_of(v: &Value) -> Vec<String> {
    v.as_arr()
        .unwrap_or(&[])
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::verdicts;
    use proptest::prelude::*;

    /// A deterministic stand-in for the run's dice.
    fn dice(seed: u64) -> impl FnMut(u64) -> u64 {
        let mut state = seed.max(1);
        move |n| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            1 + state % n.max(1)
        }
    }

    proptest! {
        /// Every claimant gets exactly one verdict, and there is a clash
        /// exactly when more reached than there was room for.
        #[test]
        fn one_verdict_each_and_clashes_only_over_capacity(
            claims in 0..8usize,
            room in proptest::option::of(0..5usize),
            rule in prop::sample::select(vec!["random_winner", "random", "all", "none"]),
            seed in 1..10_000u64,
        ) {
            let out = verdicts(claims, room, rule, &mut dice(seed));
            prop_assert_eq!(out.len(), claims);
            let over = room.is_some_and(|r| claims > r);
            prop_assert!(out.iter().all(|(_, clashed)| *clashed == over));
            if !over {
                prop_assert!(out.iter().all(|(won, _)| *won));
            }
        }

        /// Over capacity, `random_winner` hands out exactly the room there
        /// is; `all` and `none` mean what they say.
        #[test]
        fn clash_rules_hand_out_the_right_number_of_wins(
            room in 0..5usize,
            extra in 1..5usize,
            seed in 1..10_000u64,
        ) {
            let claims = room + extra;
            let wins = |rule| verdicts(claims, Some(room), rule, &mut dice(seed))
                .iter().filter(|(won, _)| *won).count();
            prop_assert_eq!(wins("random_winner"), room);
            prop_assert_eq!(wins("all"), claims);
            prop_assert_eq!(wins("none"), 0);
        }

        /// The same dice give the same verdicts.
        #[test]
        fn verdicts_are_deterministic(claims in 0..8usize, room in 0..4usize, seed in 1..10_000u64) {
            let a = verdicts(claims, Some(room), "random_winner", &mut dice(seed));
            let b = verdicts(claims, Some(room), "random_winner", &mut dice(seed));
            prop_assert_eq!(a, b);
        }
    }
}
