//! Encounters: an opponent's threats, round by round, and the counters
//! that answer them.
//!
//! ```text
//! pattern rhyme: flame, frost, blink, sunfall ~ repeat ~ one per round
//! ~ recall first when hask.saw_heart and holder != nobody
//! ~ falter instead of sunfall when holder != nobody
//!
//! threat flame: fire ~ at roland ~ answered by prevent, block, cover, absorb
//! ~ then wound target 1
//!
//! :: sanctum [encounter]
//! encounter hask ~ round by round ~ acts last ~ until hask.wards == 0
//! ```
//!
//! **Rounds are turns.** Entering an `[encounter]` passage begins the
//! encounter. The Linkers act as the story lets them, and when the turn
//! ends (by `~ pass ~ ends turn`, `turn ends when …`, or each action under
//! `clock: … ~ each action`), the opponent acts: its pattern's next threat
//! comes, the counters set up this round answer it, and the pov is shown
//! the encounter's `[resolve]` passage, where `threat.name`,
//! `threat.target` and `threat.outcome` say what came of it. Links to the
//! `[round]` passage and back into the encounter are not actions: reading
//! the next round costs no turn.
//!
//! **The pattern** gives one threat a round, in order (`~ repeat` starts it
//! again). `x first when …` comes before the next step, while it holds,
//! without using it up; `x instead of y when …` takes y's place. The
//! opponent's pattern is one declared in its own passage, or one whose
//! passage is named for it (`:: hask_duel`), or else the story's only one.
//!
//! **A threat** is `at` someone (a list, for `pov.all`), and `answered by`
//! the counters that can stop it. In this order:
//!
//! 1. It `fails when …`: outcome `fail`, and its `on fail:` effects.
//! 2. `prevent hask` this round: outcome `prevent`.
//! 3. `absorb with lumen` this round: the item takes it, outcome `absorb`,
//!    and `on absorb with lumen` fires.
//! 4. For each target: someone did `block for` them: if the blocker holds
//!    an item that `~ deflects` the threat's tag, `deflect`; one that
//!    `~ absorbs any`, `absorb`; otherwise the blocker takes it, `block`.
//!    Else the target is `cover`ed: the cover is used up, `cover`. Else it
//!    lands: `hit`, and the threat's `then` effects run with `target`.
//!
//! The outcome reported is `hit` if it landed on anyone, otherwise the
//! first target's. `skip hask this round` skips the opponent's turn
//! (outcome `skip`). Inside a threat's effects `actor` is the opponent, so
//! `take heart` is the opponent taking it.
//!
//! Counters last the round, except `cover`, which lasts until a threat
//! uses it (or `~ for N turns`). `pin hask` sets `hask.pinned` for the
//! round. `end encounter` ends it; so does leaving its passages.

use std::collections::{BTreeMap, BTreeSet};

use super::{f, strs, Event, Local, Run, R, V};
use crate::json::Value;

/// The encounter under way.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Encounter {
    pub opponent: String,
    /// The `[encounter]` passage. Its family is the encounter's: its
    /// sub-Nibs (`sanctum:round`), or its siblings (`warden:round` beside
    /// `warden:fight`).
    pub passage: String,
    /// The place the pov was in when it began: where they stand throughout,
    /// and where `->here` returns them once it is over.
    pub from: String,
    /// `until …` held, or `end encounter`: the round passages may still be
    /// read, and the next Link back to `here` leaves.
    pub over: bool,
    /// What the Linkers have set up this round.
    pub prevented: BTreeSet<String>,
    pub skipped: BTreeSet<String>,
    /// Who blocks for whom: `protected → blocker`.
    pub blocks: BTreeMap<String, String>,
    pub absorbing: Option<String>,
    /// The last threat, for the `[resolve]` passage: name, target, outcome.
    pub threat: Option<(String, String, String)>,
    /// A round has been resolved, and its `[resolve]` passage not yet shown.
    pub resolved: bool,
    /// What the threat's effects said (a death handler's lines), shown after
    /// the `[resolve]` passage tells what happened.
    pub after: Vec<super::prose::Para>,
}

impl Run {
    /// Entering an `[encounter]` passage begins it.
    pub(super) fn begin_encounter(&mut self, passage: &str) {
        let setting = self.story.passages.get(passage).and_then(|p| {
            p.header
                .iter()
                .find(|s| s.get("encounter").is_some())
                .cloned()
        });
        if let Some(s) = setting {
            let opponent = f(&s, "encounter").as_str().unwrap_or("").to_string();
            // Where the pov stands; or, when they stand nowhere yet (a test
            // that starts in the fight), where the opponent is.
            let mine = super::place_of(&self.location(&self.pov.clone())).to_string();
            let from = if self.story.kind(&mine) == Some("place") {
                mine
            } else {
                self.location(&opponent)
            };
            self.encounter = Some(Encounter {
                opponent: opponent.clone(),
                passage: passage.to_string(),
                from,
                ..Encounter::default()
            });
            self.opponent_stats(&opponent);
        }
    }

    /// `stat wards: 3`, `stat wards: 2 when heart.cracked` in the
    /// opponent's passage: its values as the encounter begins, the last
    /// that holds winning.
    fn opponent_stats(&mut self, opp: &str) {
        let story = std::sync::Arc::clone(&self.story);
        for name in &story.order {
            if name != opp && !name.strip_prefix(opp).is_some_and(|r| r.starts_with('_')) {
                continue;
            }
            for s in &story.passages[name].header {
                if f(s, "declare").as_str() != Some("stat") {
                    continue;
                }
                let when = f(s, "when");
                if *when != Value::Null && !self.eval(when).is_ok_and(|v| v.truthy()) {
                    continue;
                }
                let v = match self.constant(f(s, "range")) {
                    V::Range { lo, .. } => V::Num(lo.unwrap_or(0.0)),
                    v => v,
                };
                let id = f(s, "id").as_str().unwrap_or("");
                self.vals.insert(format!("{opp}.{id}"), v);
            }
        }
    }

    /// The prefix the encounter's passages share: `sanctum:`, `warden:`.
    fn family(e: &Encounter) -> String {
        match e.passage.rsplit_once(':') {
            Some((parent, _)) => format!("{parent}:"),
            None => format!("{}:", e.passage),
        }
    }

    /// Is this passage one of the encounter's?
    pub(super) fn in_encounter(&self, passage: &str) -> bool {
        self.encounter
            .as_ref()
            .is_some_and(|e| passage == e.passage || passage.starts_with(&Self::family(e)))
    }

    /// Where the pov stands while in the encounter's passages: the place it
    /// began in, when those passages are not a place themselves.
    pub(super) fn encounter_place(&self, passage: &str) -> Option<String> {
        let e = self.encounter.as_ref()?;
        let own = self.story.kind(super::place_of(passage)) == Some("place");
        (!own && self.in_encounter(passage) && !e.from.is_empty()).then(|| e.from.clone())
    }

    /// `->here` from an encounter that is over: back to the place.
    pub(super) fn leave_encounter(&mut self) -> Option<String> {
        let e = self.encounter.as_ref()?;
        (e.over && self.in_encounter(self.here())).then(|| e.from.clone())
    }

    /// A Link that only moves between the encounter's passages to read the
    /// next round is not an action: to the `[round]` or `[encounter]`.
    pub(super) fn reads_round(&self, link: &Value) -> bool {
        let target = f(link, "target").as_str().unwrap_or("");
        self.in_encounter(target)
            && self
                .story
                .passages
                .get(target)
                .is_some_and(|p| p.tags.iter().any(|t| t == "round" || t == "encounter"))
    }

    /// The encounter's `[resolve]` passage.
    fn resolve_passage(&self) -> Option<String> {
        let e = self.encounter.as_ref()?;
        let prefix = Self::family(e);
        self.story
            .order
            .iter()
            .find(|n| {
                n.starts_with(&prefix)
                    && self.story.passages[*n].tags.iter().any(|t| t == "resolve")
            })
            .cloned()
    }

    /// After an action: a round resolved shows its `[resolve]` passage,
    /// unless the action already led there.
    pub(super) fn show_resolve(&mut self) -> R<bool> {
        let pending = self.encounter.as_ref().is_some_and(|e| e.resolved);
        if !pending {
            return Ok(false);
        }
        let after = match self.encounter.as_mut() {
            Some(e) => {
                e.resolved = false;
                std::mem::take(&mut e.after)
            }
            None => Vec::new(),
        };
        let entered = match self.resolve_passage() {
            Some(p) if p != self.here() => {
                self.enter(&p, BTreeMap::new())?;
                true
            }
            _ => false,
        };
        self.output.extend(after);
        Ok(entered)
    }

    /// The end of a round: the opponent's threat.
    pub(super) fn resolve_round(&mut self) -> R<()> {
        let Some(e) = self.encounter.clone() else {
            return Ok(());
        };
        let opp = e.opponent.clone();
        let said = self.output.len();
        let result = if e.skipped.contains(&opp) {
            (String::new(), String::new(), "skip".to_string())
        } else {
            match self.pattern_next(&opp, true)? {
                Some(name) => self.threaten(&opp, &name, &e)?,
                None => (String::new(), String::new(), "skip".to_string()),
            }
        };
        // The round's counters are spent; a pin wears off.
        for who in self
            .story
            .ids_of("npc")
            .into_iter()
            .chain(self.story.ids_of("character"))
        {
            self.vals.remove(&format!("{who}.pinned"));
        }
        // `until hask.wards == 0 or pov.all.down`: over, once this round
        // has been told.
        let until = self
            .story
            .passages
            .get(&e.passage)
            .and_then(|p| p.header.iter().find(|s| s.get("encounter").is_some()))
            .map(|s| f(s, "until").clone())
            .unwrap_or(Value::Null);
        let over = until != Value::Null && self.eval(&until)?.truthy();
        let after = self.output.split_off(said.min(self.output.len()));
        if let Some(e) = self.encounter.as_mut() {
            e.after = after;
            e.prevented.clear();
            e.skipped.clear();
            e.blocks.clear();
            e.absorbing = None;
            e.threat = Some(result);
            e.resolved = true;
            e.over |= over;
        }
        Ok(())
    }

    /// One threat, at its targets, against this round's counters.
    fn threaten(&mut self, opp: &str, name: &str, e: &Encounter) -> R<(String, String, String)> {
        let story = std::sync::Arc::clone(&self.story);
        let answered = strs(story.field(name, "answered_by"));
        let answers = |k: &str| answered.iter().any(|a| a == k);
        let tags = strs(story.field(name, "tags"));
        let then = story.field(name, "then").as_arr().unwrap_or(&[]).to_vec();
        let at = story.field(name, "at").clone();
        let targets = if at == Value::Null {
            Vec::new()
        } else {
            self.targets(&[at], &self.pov.clone())?
                .into_iter()
                .filter(|t| !t.is_empty())
                .collect()
        };
        let first = targets.first().cloned().unwrap_or_default();
        let done = |outcome: &str| (name.to_string(), first.clone(), outcome.to_string());

        // Effects run with `actor` the opponent and `target` whom it reached.
        let run_effects = |run: &mut Run, effects: &[Value], target: &str| -> R<()> {
            let saved: Vec<(&str, Option<Local>)> = ["actor", "target"]
                .iter()
                .map(|k| (*k, run.locals.get(*k).cloned()))
                .collect();
            run.locals
                .insert("actor".into(), Local::Value(V::Id(opp.to_string())));
            let t = if target.is_empty() {
                V::Null
            } else {
                V::Id(target.to_string())
            };
            run.locals.insert("target".into(), Local::Value(t));
            let result = effects.iter().try_for_each(|eff| run.apply(eff, &[]));
            for (k, v) in saved {
                run.restore_local(k, v);
            }
            result
        };

        let fails = story.field(name, "fails_when").clone();
        if fails != Value::Null && self.eval(&fails)?.truthy() {
            let on_fail = story
                .field(name, "on_fail")
                .as_arr()
                .unwrap_or(&[])
                .to_vec();
            run_effects(self, &on_fail, &first)?;
            return Ok(done("fail"));
        }
        if e.prevented.contains(opp) && answers("prevent") {
            return Ok(done("prevent"));
        }
        if let Some(item) = e.absorbing.clone().filter(|_| answers("absorb")) {
            self.absorbed(&item)?;
            return Ok(done("absorb"));
        }
        if targets.is_empty() {
            run_effects(self, &then, "")?;
            return Ok(done("hit"));
        }
        let mut outcomes = Vec::new();
        for t in &targets {
            let outcome = match e.blocks.get(t).filter(|_| answers("block")) {
                Some(blocker) => {
                    let held: Vec<String> = self
                        .holder
                        .iter()
                        .filter(|(_, h)| *h == blocker)
                        .map(|(i, _)| i.clone())
                        .collect();
                    let deflects = held.iter().any(|i| {
                        strs(story.field(i, "deflects"))
                            .iter()
                            .any(|d| tags.contains(d) || d == "any")
                    });
                    let absorber = held.iter().find(|i| {
                        strs(story.field(i, "absorbs"))
                            .iter()
                            .any(|a| a == "any" || tags.contains(a))
                    });
                    if deflects {
                        "deflect"
                    } else if let Some(item) = absorber.cloned() {
                        self.absorbed(&item)?;
                        "absorb"
                    } else {
                        let blocker = blocker.clone();
                        run_effects(self, &then, &blocker)?;
                        "block"
                    }
                }
                None if self
                    .vals
                    .get(&format!("{t}.covered"))
                    .is_some_and(V::truthy)
                    && answers("cover") =>
                {
                    self.vals.insert(format!("{t}.covered"), V::Bool(false));
                    "cover"
                }
                None => {
                    run_effects(self, &then, t)?;
                    "hit"
                }
            };
            outcomes.push(outcome);
        }
        let outcome = if outcomes.contains(&"hit") {
            "hit"
        } else {
            outcomes[0]
        };
        Ok(done(outcome))
    }

    /// An item took a threat: `on absorb with lumen`.
    fn absorbed(&mut self, item: &str) -> R<()> {
        let pov = self.pov.clone();
        self.fire(Event {
            on: "absorb",
            what: item,
            actor: &pov,
            before: false,
        })?;
        Ok(())
    }

    /// The pattern an opponent follows: declared in its passage, or a
    /// passage named for it, or the story's only one.
    fn pattern_of(&self, opp: &str) -> Option<Value> {
        let mut all = Vec::new();
        for name in &self.story.order {
            let p = &self.story.passages[name];
            for s in p.header.iter().filter(|s| s.get("pattern").is_some()) {
                all.push((name.clone(), s.clone()));
            }
        }
        let own = all
            .iter()
            .find(|(n, _)| n == opp || n.strip_prefix(opp).is_some_and(|r| r.starts_with('_')));
        match own {
            Some((_, s)) => Some(s.clone()),
            None if all.len() == 1 => Some(all[0].1.clone()),
            None => None,
        }
    }

    /// Is `name` the name of a pattern? `hask.rhyme`.
    pub(super) fn is_pattern(&self, name: &str) -> bool {
        self.story.passages.values().any(|p| {
            p.header
                .iter()
                .any(|s| f(s, "pattern").as_str() == Some(name))
        })
    }

    /// The opponent's next threat: `hask.rhyme.next`. `advance` uses it up.
    pub(super) fn pattern_next(&mut self, opp: &str, advance: bool) -> R<Option<String>> {
        let Some(p) = self.pattern_of(opp) else {
            return Ok(None);
        };
        let name = f(&p, "pattern").as_str().unwrap_or("").to_string();
        for rule in f(&p, "first").as_arr().unwrap_or(&[]).to_vec() {
            let when = f(&rule, "when");
            if *when == Value::Null || self.eval(when)?.truthy() {
                return Ok(f(&rule, "threat").as_str().map(str::to_string));
            }
        }
        let steps = strs(f(&p, "steps"));
        if steps.is_empty() {
            return Ok(None);
        }
        let key = format!("{opp}.{name}.at");
        let mut at = self.vals.get(&key).and_then(V::num).unwrap_or(0.0) as usize;
        if at >= steps.len() {
            if f(&p, "repeat") != &Value::Bool(true) {
                return Ok(None);
            }
            at %= steps.len();
        }
        let mut next = steps[at].clone();
        for rule in f(&p, "instead").as_arr().unwrap_or(&[]).to_vec() {
            if f(&rule, "of").as_str() != Some(next.as_str()) {
                continue;
            }
            let when = f(&rule, "when");
            if *when == Value::Null || self.eval(when)?.truthy() {
                next = f(&rule, "threat").as_str().unwrap_or(&next).to_string();
            }
        }
        if advance {
            self.vals.insert(key, V::Num((at + 1) as f64));
        }
        Ok(Some(next))
    }

    /// `threat.name`, `threat.target`, `threat.outcome`.
    pub(super) fn threat_member(&self, m: &str) -> V {
        let Some((name, target, outcome)) = self.encounter.as_ref().and_then(|e| e.threat.clone())
        else {
            return V::Null;
        };
        let id = |s: String| if s.is_empty() { V::Null } else { V::Id(s) };
        match m {
            "name" => id(name),
            "target" => id(target),
            "outcome" => id(outcome),
            _ => V::Null,
        }
    }

    /// The counters, as effects.
    pub(super) fn counter(&mut self, verb: &str, args: &[Value]) -> R<()> {
        let word = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
        match verb {
            "prevent" | "skip" => {
                let who = self.who(&word(0))?;
                if let Some(e) = self.encounter.as_mut() {
                    if verb == "prevent" {
                        e.prevented.insert(who);
                    } else {
                        e.skipped.insert(who);
                    }
                }
            }
            "block" => {
                let who = self.who(&word(1))?;
                let blocker = match self.locals.get("actor") {
                    Some(Local::Value(V::Id(a))) => a.clone(),
                    _ => self.pov.clone(),
                };
                if let Some(e) = self.encounter.as_mut() {
                    e.blocks.insert(who, blocker);
                }
            }
            "absorb" => {
                let item = self.who(&word(1))?;
                if let Some(e) = self.encounter.as_mut() {
                    e.absorbing = Some(item);
                }
            }
            "cover" => {
                for who in self.targets(&args[..args.len().min(1)], &self.pov.clone())? {
                    self.vals.insert(format!("{who}.covered"), V::Bool(true));
                }
            }
            "pin" => {
                let who = self.who(&word(0))?;
                self.vals.insert(format!("{who}.pinned"), V::Bool(true));
            }
            "end" => {
                if let Some(e) = self.encounter.as_mut() {
                    e.over = true;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
