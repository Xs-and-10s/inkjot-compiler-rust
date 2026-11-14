//! Time: snapshots of the world, checkpoints, rewinds and echoes.
//!
//! The interpreter keeps a log, the way the paper Conductor did: a
//! [`Snapshot`] of the whole world at the start of every turn, and one for
//! each checkpoint. Three things read it.
//!
//! **Checkpoints** catch a death in their region and put the world back as
//! it was when the checkpoint was reached (or at the start, if it never
//! was), but only the parts they name:
//!
//! ```text
//! checkpoint cells ~ on death in prison
//! ~ resets world, held, trust, hp
//! ~ keeps knows
//! ~ to oubliette for roland, chapel for eldric
//! ```
//!
//! **An undo rewind** (`rewind: undo`) takes a Link to `rewind`, spends a
//! charge, and offers every visited turn and reached checkpoint. Going back
//! restores *everything* except what it keeps, and what the player owns
//! rather than the characters (charges and Inquests).
//!
//! **An echo** (`rewind: echo`, and every death of a Linker with a charge
//! left when no checkpoint catches it) sends one Linker back a stop or two
//! while everyone else stays in the present. An echo re-lives the stop:
//! they see their old cell (`me.past` holds), may look, ask and learn, but
//! change nothing and take nothing. Each move takes them a stop forward,
//! until they rejoin the present.
//!
//! What a checkpoint resets, or a rewind keeps, is a list of *categories*:
//!
//! | Category | Means |
//! |---|---|
//! | `world` | every fact, everyone's included (`eldric.chained`); NPCs' places; items lying anywhere; lists. Not resources |
//! | `held` | which character holds which item |
//! | `knows` | what everyone knows |
//! | a fact, stat, resource or relation (`trust`, `hp`) | its value, for everyone |
//! | a place or map (`sanctum`, `prison`) | the facts of the places in it, and the items lying there |
//! | a character or NPC (`hask`) | every fact about them |
//!
//! What a checkpoint keeps wins over what it resets: `resets world ~ keeps
//! hask` puts every lock back but leaves the Host his count of deaths.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::{f, place_of, strs, Entry, Event, Run, Sighting, R, V};
use crate::json::Value;

/// Resources that belong to the player, not the character: a rewind never
/// gives them back.
const META: &[&str] = &["rewinds", "inquests"];

/// The whole world at one moment.
#[derive(Clone, Default)]
pub(super) struct Snapshot {
    pub(super) chronon: u32,
    vals: HashMap<String, V>,
    knows: HashMap<String, BTreeSet<String>>,
    holder: HashMap<String, String>,
    lists: HashMap<String, Vec<Entry>>,
    visits: HashMap<String, u32>,
    once: HashSet<String>,
    place: HashMap<String, String>,
    pub(super) at: HashMap<String, String>,
    rng: u64,
    aka: HashMap<String, (Vec<usize>, usize)>,
    handler_counts: Vec<u32>,
    sightings: HashMap<String, Vec<Sighting>>,
}

impl Run<'_> {
    /// The world as it is now.
    pub(super) fn snapshot(&self) -> Snapshot {
        Snapshot {
            chronon: self.chronon,
            vals: self.vals.clone(),
            knows: self.knows.clone(),
            holder: self.holder.clone(),
            lists: self.lists.clone(),
            visits: self.visits.clone(),
            once: self.once.clone(),
            place: self.place.clone(),
            at: self.at.clone(),
            rng: self.rng,
            aka: self.aka.clone(),
            handler_counts: self.handler_counts.clone(),
            sightings: self.sightings.clone(),
        }
    }

    /// Make the world what `s` says.
    fn restore(&mut self, s: Snapshot) {
        self.chronon = s.chronon;
        self.vals = s.vals;
        self.knows = s.knows;
        self.holder = s.holder;
        self.lists = s.lists;
        self.visits = s.visits;
        self.once = s.once;
        self.place = s.place;
        self.at = s.at;
        self.rng = s.rng;
        self.aka = s.aka;
        self.handler_counts = s.handler_counts;
        self.sightings = s.sightings;
    }

    /// The first snapshot, and every declared checkpoint set to it.
    pub(super) fn start_the_log(&mut self) {
        let first = self.snapshot();
        for cp in &self.story.checkpoints {
            if let Some(name) = f(cp, "checkpoint").as_str() {
                self.saved.insert(name.to_string(), first.clone());
            }
        }
        self.history.push(first);
    }

    /// `checkpoint threshold` in a passage's header: reached.
    pub(super) fn reach_checkpoint(&mut self, name: &str) {
        self.saved.insert(name.to_string(), self.snapshot());
        if !self.reached.iter().any(|r| r == name) {
            self.reached.push(name.to_string());
        }
    }

    /// The stop the Linker being served is living in: their own, if they
    /// are an echo, else the clock's.
    pub(super) fn when(&self) -> u32 {
        self.past.get(&self.pov).copied().unwrap_or(self.chronon)
    }

    /// Is the Linker being served re-living the past?
    pub(super) fn in_past(&self) -> bool {
        self.past.contains_key(&self.pov)
    }

    // ------------------------------------------------------------- deaths

    /// After a Link: everyone who died is caught by a checkpoint, pulled
    /// back by a charge, or stays dead.
    pub fn settle_deaths(&mut self) -> R<()> {
        let dying = std::mem::take(&mut self.dying);
        for (who, place) in dying {
            if !self.vals.get(&format!("{who}.dead")).is_some_and(V::truthy) {
                continue; // already brought back by an earlier one
            }
            let caught = self.story.checkpoints.iter().find(|cp| {
                f(cp, "on_death_in")
                    .as_str()
                    .is_some_and(|region| self.inside(&place, region))
            });
            if let Some(cp) = caught.cloned() {
                self.fall_back(&cp)?;
                continue;
            }
            let charges = self.vals.get(&format!("{who}.rewinds")).and_then(V::num);
            if charges.is_some_and(|n| n > 0.0) {
                self.pull_back(&who)?;
            } else {
                self.drop_everything(&who, &place);
            }
        }
        Ok(())
    }

    /// The dead hold nothing: what they carried falls where they died, for
    /// anyone to find (unless it `survives death`, like a ghost's locket).
    fn drop_everything(&mut self, who: &str, place: &str) {
        let carried: Vec<String> = self
            .holder
            .iter()
            .filter(|(_, h)| h.as_str() == who)
            .map(|(item, _)| item.clone())
            .collect();
        for item in carried {
            if *self.story.field(&item, "survives") == Value::Null {
                self.holder.insert(item, place.to_string());
            }
        }
    }

    /// A checkpoint catches a death: reset what it names, bring the dead
    /// back, and send everyone where it says.
    fn fall_back(&mut self, cp: &Value) -> R<()> {
        let name = f(cp, "checkpoint").as_str().unwrap_or("");
        let saved = self.saved.get(name).cloned().unwrap_or_default();
        let resets = strs(f(cp, "resets"));
        let keeps = strs(f(cp, "keeps"));
        let current = self.snapshot();
        let mut now = current.clone();
        self.merge(&mut now, &saved, &resets);
        // What it keeps wins over what it resets, and so does a fact that
        // `survives death` (Ex 2's `fact deaths: 0 ~ survives death`).
        self.merge(&mut now, &current, &keeps);
        for (key, value) in &current.vals {
            let name = key.rsplit('.').next().unwrap_or(key);
            if *self.story.field(name, "survives") != Value::Null {
                now.vals.insert(key.clone(), value.clone());
            }
        }
        self.restore(now);
        for c in self.story.ids_of("character") {
            self.vals.remove(&format!("{c}.dead"));
            self.vals.remove(&format!("{c}.wounded"));
        }
        self.refill("checkpoint");
        // `to oubliette for roland, chapel for eldric`, or `to threshold`.
        let movers = self.players();
        for who in &movers {
            let to = match f(cp, "to") {
                Value::Obj(_) => f(f(cp, "to"), who).as_str(),
                other => other.as_str(),
            };
            if let Some(to) = to {
                self.at.insert(who.clone(), to.to_string());
            }
        }
        let served = self.pov.clone();
        for who in movers {
            if self.order.is_empty() && who != served {
                continue; // solo: the other pov is entered when switched to
            }
            self.sit(&who)?;
            let at = self.here().to_string();
            self.enter(&at, HashMap::new())?;
        }
        self.sit(&served)
    }

    /// Resources that refill on this occasion: `~ refill on checkpoint`.
    fn refill(&mut self, on: &str) {
        for id in self.story.ids_of("resource") {
            if self.story.field(&id, "refill").as_str() != Some(on) {
                continue;
            }
            let full = self.constant(self.story.field(&id, "value"));
            for c in self.story.ids_of("character") {
                let key = format!("{c}.{id}");
                match &full {
                    V::Null => self.vals.remove(&key),
                    v => self.vals.insert(key, v.clone()),
                };
            }
        }
    }

    /// A Linker killed with a charge left is pulled back instead: thrown
    /// back to the start of the stop they died in, as an echo.
    fn pull_back(&mut self, who: &str) -> R<()> {
        self.spend_charge(who);
        self.vals.remove(&format!("{who}.dead"));
        // Vell kills at the end of a stop, after the clock has moved on.
        let died_in = self.chronon.saturating_sub(1);
        let served = self.pov.clone();
        self.sit(who)?;
        self.output
            .push("Your watch fires, and the night runs backwards.".into());
        self.become_echo(died_in)?;
        self.sit(&served)
    }

    fn spend_charge(&mut self, who: &str) {
        let key = format!("{who}.rewinds");
        if let Some(n) = self.vals.get(&key).and_then(V::num) {
            self.vals.insert(key, V::Num((n - 1.0).max(0.0)));
        }
    }

    /// Everyone playing: the seated cohort, or every pov.
    fn players(&self) -> Vec<String> {
        if !self.order.is_empty() {
            self.order.clone()
        } else if !self.story.povs.is_empty() {
            self.story.povs.clone()
        } else {
            vec![self.pov.clone()]
        }
    }

    // ------------------------------------------------------------ rewinds

    /// A Link to `rewind`: spend a charge and offer the times to go back to.
    pub(super) fn begin_rewind(&mut self) -> R<()> {
        let setting = self.story.rewind.clone().unwrap_or(Value::Null);
        let actor = self.pov.clone();
        self.spend_charge(&actor);
        let word = self.clock_word();
        let mut menu = Vec::new();
        if f(&setting, "value").as_str() == Some("echo") {
            let (lo, hi) = match self.constant(f(&setting, "back")) {
                V::Range { lo, hi, .. } => (lo.unwrap_or(1.0) as u32, hi.unwrap_or(1.0) as u32),
                V::Num(n) => (n as u32, n as u32),
                _ => (1, 1),
            };
            for n in lo..=hi {
                // Only back to a stop the log has, with a cell to be in.
                let then = self.chronon.checked_sub(n).and_then(|when| {
                    let snap = self.history.iter().rev().find(|s| s.chronon == when)?;
                    let at = snap.at.get(&self.pov)?;
                    self.arrival_at(place_of(at), when)
                });
                if then.is_some() {
                    let label = if n == 1 {
                        format!("Back one {word}")
                    } else {
                        format!("Back {n} {word}s")
                    };
                    menu.push(rewind_link(&format!("echo:{n}"), &label));
                }
            }
        } else {
            let mut turns: Vec<u32> = self.history.iter().map(|s| s.chronon).collect();
            turns.sort_unstable();
            turns.dedup();
            for n in turns.into_iter().rev() {
                let label = format!("Back to the start of {word} {n}");
                menu.push(rewind_link(&format!("turn:{n}"), &label));
            }
            for name in self.reached.clone() {
                let label = format!("Back to the {} checkpoint", self.label(&name));
                menu.push(rewind_link(&format!("checkpoint:{name}"), &label));
            }
        }
        if menu.is_empty() {
            // Nowhere to go back to yet: the charge is not spent.
            if let Some(n) = self.vals.get(&format!("{actor}.rewinds")).and_then(V::num) {
                self.vals
                    .insert(format!("{actor}.rewinds"), V::Num(n + 1.0));
            }
            self.output
                .push("(There is nowhere to go back to yet.)".into());
            return self.relist();
        }
        self.links = menu;
        Ok(())
    }

    /// The word the clock is shown as: `turn`, `stop`.
    fn clock_word(&self) -> String {
        self.story
            .defaults
            .get("clock_shown_as")
            .and_then(Value::as_str)
            .unwrap_or("turn")
            .to_string()
    }

    /// Go back to the time a rewind Link names.
    pub(super) fn rewind_to(&mut self, link: &Value) -> R<()> {
        let to = f(link, "target").as_str().unwrap_or("").to_string();
        if let Some(n) = to.strip_prefix("echo:") {
            let back: u32 = n.parse().unwrap_or(1);
            let when = self.chronon.saturating_sub(back);
            return self.become_echo(when);
        }
        let snap = if let Some(name) = to.strip_prefix("checkpoint:") {
            self.saved.get(name).cloned()
        } else {
            let n: u32 = to.trim_start_matches("turn:").parse().unwrap_or(0);
            self.history.iter().rev().find(|s| s.chronon == n).cloned()
        };
        let snap = snap.ok_or_else(|| format!("there is no time `{to}` to go back to"))?;
        self.undo(snap)
    }

    /// Put everything back as it was, except what the rewind keeps.
    fn undo(&mut self, mut back: Snapshot) -> R<()> {
        let setting = self.story.rewind.clone().unwrap_or(Value::Null);
        let mut keeps = strs(f(&setting, "keeps"));
        keeps.extend(META.iter().map(|m| m.to_string()));
        let now = self.snapshot();
        self.merge(&mut back, &now, &keeps);
        // `moves me`: only the one rewinding goes back; by default, everyone.
        if strs(f(&setting, "moves")).iter().any(|m| m == "me") {
            let mine = back.at.get(&self.pov).cloned();
            back.at = now.at.clone();
            if let Some(mine) = mine {
                back.at.insert(self.pov.clone(), mine);
            }
        }
        // Later turns never happened.
        let chronon = back.chronon;
        self.history.retain(|s| s.chronon <= chronon);
        self.restore(back);
        let unseen = strs(f(&setting, "unseen_by"));
        let actor = self.pov.clone();
        self.unseen_by = unseen;
        let fired = self.fire(Event {
            on: "rewind",
            what: "",
            actor: &actor,
            before: false,
        });
        self.unseen_by.clear();
        fired?;
        self.resume()
    }

    /// Show the passage the pov is at again, without arriving: its header
    /// already ran, back then.
    fn resume(&mut self) -> R<()> {
        let at = self.here().to_string();
        self.asking.clear();
        self.locals.clear();
        self.parent_lets(&at)?;
        self.lets_of(&at)?;
        self.links = self.render(&at, false)?;
        let more = self.standing_links()?;
        self.links.extend(more);
        Ok(())
    }

    /// Re-live stop `when` as an echo, from wherever the Linker was then.
    fn become_echo(&mut self, when: u32) -> R<()> {
        let then = self
            .history
            .iter()
            .rev()
            .find(|s| s.chronon == when)
            .and_then(|s| s.at.get(&self.pov).cloned());
        let place = place_of(&then.unwrap_or_else(|| self.here().to_string())).to_string();
        if when >= self.chronon {
            self.past.remove(&self.pov);
        } else {
            self.past.insert(self.pov.clone(), when);
        }
        let cell = self
            .arrival_at(&place, when)
            .ok_or_else(|| format!("there is no `{place}` at {when} to go back to"))?;
        self.enter(&cell, HashMap::new())
    }

    /// An echo moves: one stop forward, until they are back in the present.
    pub(super) fn echo_moves(&mut self, link: &Value) -> R<()> {
        let to = match f(link, "kind").as_str() {
            Some("walk") => f(link, "target").as_str().unwrap_or("").to_string(),
            _ => place_of(self.here()).to_string(),
        };
        let next = self.when() + 1;
        if next >= self.chronon {
            self.past.remove(&self.pov);
            self.output.push("You are back in the present.".into());
        } else {
            self.past.insert(self.pov.clone(), next);
        }
        let cell = self
            .arrival_at(&to, next.min(self.chronon))
            .ok_or_else(|| format!("there is nowhere to arrive at `{to}`"))?;
        self.enter(&cell, HashMap::new())
    }

    /// May an echo apply this effect? An echo learns; it changes nothing.
    pub(super) fn echo_may(&self, effect: &Value) -> bool {
        matches!(
            f(effect, "do").as_str(),
            Some("learn" | "say" | "play" | "nothing")
        )
    }

    // ------------------------------------------------------- categories

    /// Into `base`, copy from `from` everything in these categories.
    fn merge(&self, base: &mut Snapshot, from: &Snapshot, categories: &[String]) {
        if categories.is_empty() {
            return;
        }
        let keys: HashSet<String> = base.vals.keys().chain(from.vals.keys()).cloned().collect();
        for key in keys {
            if categories.iter().any(|c| self.val_in(c, &key)) {
                match from.vals.get(&key) {
                    Some(v) => base.vals.insert(key, v.clone()),
                    None => base.vals.remove(&key),
                };
            }
        }
        let items: HashSet<String> = base
            .holder
            .keys()
            .chain(from.holder.keys())
            .cloned()
            .collect();
        for item in items {
            let held = |s: &Snapshot| s.holder.get(&item).cloned();
            let fits = categories.iter().any(|c| {
                [held(base), held(from)]
                    .iter()
                    .flatten()
                    .any(|h| self.holding_in(c, h))
            });
            if fits {
                match held(from) {
                    Some(h) => base.holder.insert(item, h),
                    None => base.holder.remove(&item),
                };
            }
        }
        if categories.iter().any(|c| c == "knows") {
            base.knows = from.knows.clone();
        }
        let npcs: HashSet<String> = base
            .place
            .keys()
            .chain(from.place.keys())
            .cloned()
            .collect();
        for npc in npcs {
            let fits = categories.iter().any(|c| {
                c == "world"
                    || [base.place.get(&npc), from.place.get(&npc)]
                        .iter()
                        .flatten()
                        .any(|p| self.inside(p, c))
            });
            if fits {
                match from.place.get(&npc) {
                    Some(p) => base.place.insert(npc, p.clone()),
                    None => base.place.remove(&npc),
                };
            }
        }
        if categories.iter().any(|c| c == "world") {
            base.lists = from.lists.clone();
        }
    }

    /// Is the stored value `key` in this category?
    fn val_in(&self, category: &str, key: &str) -> bool {
        let owner = key.split('.').next().unwrap_or(key);
        let name = key.rsplit('.').next().unwrap_or(key);
        match category {
            // Everything but resources (vults, hp, charges), which are
            // reset or kept by name.
            "world" => self.story.kind(name) != Some("resource"),
            "knows" | "held" => false,
            _ if self.is_region(category) => self.inside(owner, category),
            // A character or NPC: everything about them, `hask.deaths`.
            _ if self.is_person(category) => owner == category,
            // A fact, stat, resource or relation, for everyone.
            _ => name == category,
        }
    }

    /// Is an item held by `holder` in this category?
    fn holding_in(&self, category: &str, holder: &str) -> bool {
        let by_character = self.story.kind(holder) == Some("character");
        match category {
            "held" => by_character,
            "world" => !by_character,
            _ if self.is_region(category) => self.inside(place_of(holder), category),
            _ => false,
        }
    }

    fn is_person(&self, name: &str) -> bool {
        matches!(self.story.kind(name), Some("character" | "npc"))
    }

    fn is_region(&self, name: &str) -> bool {
        matches!(self.story.kind(name), Some("place" | "map"))
    }
}

/// A Link in the rewind menu.
fn rewind_link(to: &str, label: &str) -> Value {
    Value::obj([
        ("type", Value::str("link")),
        ("id", Value::str(format!("rewind:{to}"))),
        ("label", Value::str(label)),
        ("target", Value::str(to)),
        ("kind", Value::str("rewind")),
    ])
}

#[cfg(test)]
mod tests {
    use crate::run::{Run, Story};
    use proptest::prelude::*;

    const EX2: &str = include_str!("../../examples/ex2-the-ninefold-cell.twee");

    /// Take random Links; stop quietly at the first thing not supported.
    fn wander(run: &mut Run, picks: &[usize]) {
        for pick in picks {
            let Ok(choices) = run.choices() else { return };
            if choices.is_empty() {
                return;
            }
            let (id, _) = choices[pick % choices.len()].clone();
            if run.choose(&id).is_err() {
                return;
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// `merge` takes exactly the named categories: after merging `then`
        /// into `now`, every stored value in a category comes from `then`,
        /// and every other value is untouched.
        #[test]
        fn merging_takes_exactly_the_named_categories(
            seed in 1..500u64,
            first in proptest::collection::vec(0..20usize, 0..25),
            second in proptest::collection::vec(0..20usize, 0..25),
            pick in proptest::sample::subsequence(
                vec!["world", "held", "knows", "trust", "hp", "vults", "sanctum", "prison", "hask", "roland"], 0..4),
        ) {
            let ir = crate::build(EX2).unwrap();
            let story = Story::new(&ir).unwrap();
            let mut run = Run::new(&story, seed);
            run.start(None, None).unwrap();
            wander(&mut run, &first);
            let then = run.snapshot();
            wander(&mut run, &second);
            let now = run.snapshot();
            let categories: Vec<String> = pick.iter().map(|c| c.to_string()).collect();
            let mut merged = now.clone();
            run.merge(&mut merged, &then, &categories);
            let keys: std::collections::HashSet<&String> = now.vals.keys().chain(then.vals.keys()).collect();
            for key in keys {
                let named = categories.iter().any(|c| run.val_in(c, key));
                let want = if named { then.vals.get(key) } else { now.vals.get(key) };
                prop_assert_eq!(merged.vals.get(key), want, "{} ({:?})", key, categories);
            }
            if categories.iter().any(|c| c == "knows") {
                prop_assert_eq!(&merged.knows, &then.knows);
            } else {
                prop_assert_eq!(&merged.knows, &now.knows);
            }
        }

        /// Going back to a turn with nothing kept but the player's charges
        /// gives the world exactly as it was then.
        #[test]
        fn going_back_with_nothing_kept_restores_the_turn(
            seed in 1..500u64,
            first in proptest::collection::vec(0..20usize, 0..25),
            second in proptest::collection::vec(0..20usize, 0..25),
        ) {
            let ir = crate::build(EX2).unwrap();
            let mut story = Story::new(&ir).unwrap();
            story.rewind = None; // keeps nothing of its own
            let mut run = Run::new(&story, seed);
            run.start(None, None).unwrap();
            wander(&mut run, &first);
            let then = run.snapshot();
            wander(&mut run, &second);
            run.undo(then.clone()).unwrap();
            for (key, value) in &then.vals {
                let charge = key.ends_with(".rewinds") || key == "inquests";
                if !charge {
                    prop_assert_eq!(run.vals.get(key), Some(value), "{}", key);
                }
            }
            prop_assert_eq!(&run.holder, &then.holder);
            prop_assert_eq!(&run.knows, &then.knows);
            prop_assert_eq!(run.chronon, then.chronon);
        }
    }
}
