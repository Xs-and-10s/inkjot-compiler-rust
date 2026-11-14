//! NPC agendas: what the people (and statues) you are not playing do.
//!
//! ```text
//! :: vell [npc]
//! starts: lounge@0
//!
//! goal flee:
//! ~ toward platform
//! ~ weight 6
//! ~ pace dash
//! ~ when self.cornered
//!
//! reaction surrender:
//! ~ when self.leaving and count(linker, in: here) >= 2
//! ~ then detain vell
//! ```
//!
//! At the end of every turn, each NPC with an agenda takes one move:
//!
//! 1. Its margin `let`s are worked out, with `self` as the NPC and `here`
//!    as its place.
//! 2. It picks a goal: the heaviest whose `when` holds. Equal weights go
//!    to the one written first (`ties: first`, the default).
//! 3. It plans its route toward the goal, so that `self.heading` and
//!    `self.leaving` mean something.
//! 4. Its reactions are checked, in order. The first that holds runs its
//!    `then`, and that is the NPC's whole turn.
//! 5. Otherwise it walks: one step a turn, two at `pace dash`, or its own
//!    `moves N per chronon`. Arriving runs the goal's `then`.
//!
//! An `[archetype]` passage is an agenda shared by every NPC `like` it:
//! the three statues of Ex 2 all follow `:: statue [archetype]`.

use std::collections::{HashMap, VecDeque};

use super::{f, place_of, strs, Event, Local, Run, R, V};
use crate::json::Value;

/// Who is finding a way: NPCs and Linkers may use different ways.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Walker {
    Npc,
    Linker,
}

/// An NPC's agenda: its own margin, then its archetype's.
struct Agenda {
    statements: Vec<Value>,
}

impl Agenda {
    fn goals(&self) -> impl Iterator<Item = &Value> {
        self.statements.iter().filter(|s| s.get("goal").is_some())
    }

    fn reactions(&self) -> impl Iterator<Item = &Value> {
        self.statements
            .iter()
            .filter(|s| s.get("reaction").is_some())
    }

    /// A setting such as `starts:` or `moves:`.
    fn setting(&self, name: &str) -> Option<&Value> {
        self.statements
            .iter()
            .find(|s| f(s, "setting").as_str() == Some(name))
    }
}

impl Run<'_> {
    /// The agenda of `npc`, or `None` if it has none.
    fn agenda(&self, npc: &str) -> Option<Agenda> {
        let story = self.story;
        let mut statements = Vec::new();
        let own = story
            .passages
            .get(npc)
            .filter(|p| p.tags.iter().any(|t| t == "npc"));
        let like = story.field(npc, "like").as_str().unwrap_or("");
        let shared = story
            .passages
            .get(like)
            .filter(|p| p.tags.iter().any(|t| t == "archetype"));
        for p in [own, shared].into_iter().flatten() {
            statements.extend(p.header.iter().cloned());
        }
        if statements.is_empty() {
            None
        } else {
            Some(Agenda { statements })
        }
    }

    /// Put each NPC where its agenda's `starts:` says, `starts: lounge@0`.
    pub(super) fn place_npcs(&mut self) {
        for npc in self.story.ids_of("npc") {
            let start = self.agenda(&npc).and_then(|a| {
                a.setting("starts")
                    .and_then(|s| f(s, "value").as_str())
                    .map(str::to_string)
            });
            if let Some(start) = start {
                self.place.insert(npc, place_of(&start).to_string());
            }
        }
    }

    /// Every NPC takes its move. Called at the end of each turn.
    pub(super) fn agendas(&mut self) -> R<()> {
        for npc in self.story.ids_of("npc") {
            let Some(agenda) = self.agenda(&npc) else {
                continue;
            };
            let out = ["dead", "detained", "fixed"].iter().any(|fact| {
                self.vals
                    .get(&format!("{npc}.{fact}"))
                    .is_some_and(V::truthy)
            });
            if out {
                continue;
            }
            // The NPC's own names, for the length of its move.
            let saved = std::mem::take(&mut self.locals);
            let result = self.move_npc(&npc, &agenda);
            self.locals = saved;
            result.map_err(|e| format!("{e} (in the agenda of `{npc}`)"))?;
        }
        Ok(())
    }

    fn move_npc(&mut self, npc: &str, agenda: &Agenda) -> R<()> {
        let here = self.location(npc);
        self.locals
            .insert("self".into(), Local::Value(V::Id(npc.to_string())));
        self.locals
            .insert("here".into(), Local::Value(V::Id(here.clone())));
        for s in agenda.statements.iter().filter(|s| s.get("let").is_some()) {
            self.statement(s)?;
        }

        // 2. The heaviest goal that holds and has somewhere to go; the first
        //    of equals. A goal `toward` nobody (no one holds the evidence
        //    yet) is passed over.
        let mut chosen: Option<(&Value, String)> = None;
        for goal in agenda.goals() {
            let when = f(goal, "when");
            if *when != Value::Null && !self.eval(when)?.truthy() {
                continue;
            }
            let Some(target) = self.goal_place(goal)? else {
                continue;
            };
            let weight = |g: &Value| f(g, "weight").as_u64().unwrap_or(0);
            if chosen
                .as_ref()
                .is_none_or(|(c, _)| weight(goal) > weight(c))
            {
                chosen = Some((goal, target));
            }
        }

        // 3. Plan the route, so reactions can ask where the NPC is going.
        let (goal, target) = match chosen {
            Some((goal, target)) => (Some(goal), target),
            None => (None, String::new()),
        };
        let route = match goal {
            Some(goal) => {
                let via = f(goal, "via").as_str();
                self.route(&here, &target, via, Walker::Npc)?
                    .unwrap_or_default()
            }
            None => Vec::new(),
        };
        let heading = route.first().cloned().unwrap_or_default();
        let leaving = !heading.is_empty() && self.story.has_tag(&target, "offstage");
        self.vals.insert(format!("{npc}.heading"), V::Id(heading));
        self.vals.insert(format!("{npc}.leaving"), V::Bool(leaving));

        // 4. A reaction that holds is the NPC's whole turn.
        for reaction in agenda.reactions() {
            let when = f(reaction, "when");
            if *when != Value::Null && !self.eval(when)?.truthy() {
                continue;
            }
            for effect in f(reaction, "then").as_arr().unwrap_or(&[]).to_vec() {
                self.apply(&effect, &[])?;
            }
            return Ok(());
        }

        // 5. Walk, then do what the goal came for.
        let Some(goal) = goal else {
            return Ok(());
        };
        let steps = if f(goal, "pace").as_str() == Some("dash") {
            2
        } else {
            self.speed(npc, agenda)
        };
        for place in route.iter().take(steps) {
            self.step(npc, place)?;
        }
        if !target.is_empty() && self.location(npc) == target {
            self.locals
                .insert("here".into(), Local::Value(V::Id(target)));
            for effect in f(goal, "then").as_arr().unwrap_or(&[]).to_vec() {
                self.apply(&effect, &[])?;
            }
        }
        Ok(())
    }

    /// The place a goal heads for: `toward platform`, or toward wherever
    /// a person or thing is. `None` for a goal toward nobody.
    fn goal_place(&mut self, goal: &Value) -> R<Option<String>> {
        self.via = f(goal, "via").as_str().map(str::to_string);
        let toward = self.eval(f(goal, "toward"));
        self.via = None;
        let Some(name) = toward?.name().map(str::to_string) else {
            return Ok(None);
        };
        Ok(Some(match self.story.kind(&name) {
            Some("character" | "npc" | "item") => self.location(&name),
            _ => place_of(&name).to_string(),
        }))
    }

    /// How many places an NPC may move a turn: `moves 2 per chronon`,
    /// `moves none`, or one.
    fn speed(&self, npc: &str, agenda: &Agenda) -> usize {
        if let Some(n) = self.story.field(npc, "speed").as_u64() {
            return n as usize;
        }
        if let Some(Value::Arr(acts)) = self
            .story
            .decls
            .get(npc)
            .and_then(|d| d.fields.get("moves"))
        {
            if acts.is_empty() {
                return 0; // `moves none`
            }
        }
        agenda
            .setting("moves")
            .and_then(|s| f(s, "speed").as_u64())
            .map_or(1, |n| n as usize)
    }

    /// Move an NPC one place, and tell any handlers.
    fn step(&mut self, npc: &str, to: &str) -> R<()> {
        let from = self.location(npc);
        self.place.insert(npc.to_string(), to.to_string());
        self.fire(Event {
            on: "leave",
            what: &from,
            actor: npc,
            before: false,
        })?;
        self.fire(Event {
            on: "enter",
            what: to,
            actor: npc,
            before: false,
        })?;
        Ok(())
    }

    /// The places passed through from `from` to `to`, not counting `from`:
    /// a shortest route along the ways `who` may use. `via wide` keeps to
    /// ways tagged `wide`. `None` if there is no route.
    pub(super) fn route(
        &mut self,
        from: &str,
        to: &str,
        via: Option<&str>,
        who: Walker,
    ) -> R<Option<Vec<String>>> {
        let (_, parent) = self.search(from, via, who)?;
        if from != to && !parent.contains_key(to) {
            return Ok(None);
        }
        // Follow the breadcrumbs back from `to`.
        let mut path = Vec::new();
        let mut at = to.to_string();
        while at != from {
            path.push(at.clone());
            at = parent[&at].clone();
        }
        path.reverse();
        Ok(Some(path))
    }

    /// How many steps each place is from `from`, along the ways `who` may use.
    pub(super) fn distances(
        &mut self,
        from: &str,
        via: Option<&str>,
        who: Walker,
    ) -> R<HashMap<String, usize>> {
        Ok(self.search(from, via, who)?.0)
    }

    /// Breadth-first search from `from`: each reachable place's distance,
    /// and the place it was first reached from.
    fn search(
        &mut self,
        from: &str,
        via: Option<&str>,
        who: Walker,
    ) -> R<(HashMap<String, usize>, HashMap<String, String>)> {
        let mut seen = HashMap::from([(from.to_string(), 0)]);
        let mut parent = HashMap::new();
        let mut queue = VecDeque::from([from.to_string()]);
        while let Some(at) = queue.pop_front() {
            for (way, a, b) in self.story.ways.clone() {
                if a != at || seen.contains_key(&b) || !self.may_use(&way, via, who)? {
                    continue;
                }
                seen.insert(b.clone(), seen[&at] + 1);
                parent.insert(b.clone(), at.clone());
                queue.push_back(b);
            }
        }
        Ok((seen, parent))
    }

    /// May `who` use this way now? It must be open, and tagged `via` if a
    /// goal asks for that. NPCs know the hidden ways; Linkers must find
    /// them, and may not use a way `for npc`.
    fn may_use(&mut self, way: &str, via: Option<&str>, who: Walker) -> R<bool> {
        if let Some(via) = via {
            if !self.story.has_tag(way, via) {
                return Ok(false);
            }
        }
        if way.is_empty() {
            return Ok(true); // a step of a `route`
        }
        if who == Walker::Linker {
            if strs(self.story.field(way, "for"))
                .iter()
                .any(|w| w == "npc")
            {
                return Ok(false);
            }
            let hidden = self.story.field(way, "hidden_until").clone();
            if hidden != Value::Null && !self.eval(&hidden)?.truthy() {
                return Ok(false);
            }
        }
        Ok(match self.story.field(way, "open_when").clone() {
            Value::Null => true,
            Value::Str(name) => self.name(&name)?.truthy(),
            cond => self.eval(&cond)?.truthy(),
        })
    }

    /// `self.watched`: the pov is standing in the NPC's place, looking, or
    /// something there `watches here`, as a mirror does while it is open:
    /// `archetype mirror: ~ watches here when self.open`.
    pub(super) fn watched(&mut self, npc: &str) -> R<bool> {
        let place = self.location(npc);
        if place_of(self.here()) == place {
            return Ok(true);
        }
        for item in self.story.ids_of("item") {
            if self.location(&item) != place {
                continue;
            }
            let like = self.story.field(&item, "like").as_str().unwrap_or("");
            let rule = match self.story.field(&item, "watches_when") {
                Value::Null => self.story.field(like, "watches_when").clone(),
                own => own.clone(),
            };
            if rule == Value::Null {
                continue;
            }
            // Inside the rule, `self` is the watcher.
            let saved = self
                .locals
                .insert("self".into(), Local::Value(V::Id(item.clone())));
            let watching = self.eval(&rule).map(|v| v.truthy());
            match saved {
                Some(old) => self.locals.insert("self".into(), old),
                None => self.locals.remove("self"),
            };
            if watching? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// `nearest(living)`: the nearest thing with the tag that the NPC asking
    /// can reach, other than itself and its own kind.
    pub(super) fn nearest(&mut self, tag: &str) -> R<V> {
        let me = match self.locals.get("self") {
            Some(Local::Value(V::Id(id))) => id.clone(),
            _ => self.pov.clone(),
        };
        let like = self.story.field(&me, "like").clone();
        let via = self.via.clone();
        let distances = self.distances(&self.location(&me), via.as_deref(), Walker::Npc)?;
        let mut best: Option<(usize, String)> = None;
        for id in self.things(tag, None) {
            if id == me || (like != Value::Null && *self.story.field(&id, "like") == like) {
                continue;
            }
            if let Some(&d) = distances.get(&self.location(&id)) {
                if best.as_ref().is_none_or(|(b, _)| d < *b) {
                    best = Some((d, id));
                }
            }
        }
        Ok(best.map_or(V::Null, |(_, id)| V::Id(id)))
    }

    /// `alone_with(a, b)`: in the same place, with no one else living there.
    /// `alone_with(a, b)`: a and b are the only Linkers or awake
    /// witnesses in the place. A sleeping Colonel sees nothing, and Mrs.
    /// Ardent, who is no witness, does not count.
    pub(super) fn alone_with(&self, a: &str, b: &str) -> bool {
        let place = self.location(a);
        if place.is_empty() || self.location(b) != place {
            return false;
        }
        self.things("living", Some(&place)).iter().all(|id| {
            let counts =
                self.is_kind(id, "linker") || (self.tagged(id, "witness") && !self.is_asleep(id));
            id == a || id == b || !counts
        })
    }

    /// Asleep now: during the hours `asleep during` says, or pretending,
    /// and not woken.
    pub(super) fn is_asleep(&self, who: &str) -> bool {
        let during = self.constant(self.story.field(who, "asleep_during"));
        let flag = |f: &str| self.vals.get(&format!("{who}.{f}")).is_some_and(V::truthy);
        (V::Num(f64::from(self.chronon)).within(&during) || flag("feigning")) && !flag("woken")
    }

    /// `x.last_seen(character, holding: evidence)`: whoever of that kind
    /// `x` most recently saw holding a thing with the tag. With no kind,
    /// anyone. NPCs look at the end of each turn (see [`Run::look`]), so
    /// this is someone who ended a stop in their place.
    pub(super) fn last_seen(&self, who: &str, kind: Option<&str>, tag: &str) -> V {
        let log = self.sightings.get(who).map(Vec::as_slice).unwrap_or(&[]);
        for (_, seen) in log.iter().rev() {
            for (person, held) in seen {
                let fits = kind.is_none_or(|k| self.is_kind(person, k));
                if fits && held.iter().any(|item| self.tagged(item, tag)) {
                    return V::Id(person.clone());
                }
            }
        }
        V::Null
    }

    /// Is `id` of this kind? A declared kind (`character`, `npc`, `item`),
    /// a tag (`suspect`), `person` (a character or an NPC) or `linker`
    /// (a character someone is playing).
    pub(super) fn is_kind(&self, id: &str, kind: &str) -> bool {
        match kind {
            "person" => matches!(self.story.kind(id), Some("character" | "npc")),
            "linker" => self.story.povs.iter().any(|p| p == id) || self.at.contains_key(id),
            _ => {
                self.story.kind(id) == Some(kind)
                    || self.tagged(id, kind)
                    || self.story.field(id, "like").as_str() == Some(kind)
            }
        }
    }

    /// `roll standoff`: roll the table's dice and run every row that matches.
    pub(super) fn roll_table(&mut self, table: &str) -> R<()> {
        let dice = self.story.field(table, "roll").clone();
        let rolled = self.eval(&dice)?;
        let rows = self
            .story
            .field(table, "rows")
            .as_arr()
            .unwrap_or(&[])
            .to_vec();
        for row in rows {
            let range = self.constant(f(&row, "range"));
            if rolled.within(&range) && *f(&row, "then") != Value::Null {
                self.apply(f(&row, "then"), &[])?;
            }
        }
        Ok(())
    }

    /// `x.offstage`: in a place marked `offstage`, or `offstage until`
    /// something that has not happened yet.
    pub(super) fn offstage(&mut self, id: &str) -> R<bool> {
        // `offstage until testament`: until that passage has been seen.
        let until = self.story.field(id, "offstage_until").clone();
        let arrived = match &until {
            Value::Null => true,
            Value::Str(p) if self.story.passages.contains_key(p) => self.visits.contains_key(p),
            cond => self.eval(cond)?.truthy(),
        };
        if !arrived {
            return Ok(true);
        }
        let place = self.location(id);
        Ok(self.story.has_tag(&place, "offstage")
            || strs(self.story.field(id, "tags"))
                .iter()
                .any(|t| t == "offstage"))
    }
}

#[cfg(test)]
mod tests {
    use super::Walker;
    use crate::run::{Run, Story};
    use proptest::prelude::*;
    use std::collections::{HashMap, VecDeque};

    /// A story that is nothing but `n` places and the given ways.
    fn map(n: usize, ways: &[(usize, usize, bool)]) -> String {
        let mut src = String::from(
            ":: StoryTitle\nMap\n\n:: StoryData\n{\"ifid\": \"x\", \"format\": \"Inkjot\", \"format-version\": \"0.5.0\", \"start\": \"p0\"}\n\n:: Inklink [config]\nkind: story\nmode: solo\npov: ann\ncharacter ann: \"Ann\" ~ playable\n",
        );
        for i in 0..n {
            src.push_str(&format!("place p{i}: \"p{i}\"\n"));
        }
        for (k, (a, b, one_way)) in ways.iter().enumerate() {
            let tail = if *one_way { " ~ one way" } else { "" };
            src.push_str(&format!("way w{k}: p{a} - p{b}{tail}\n"));
        }
        src.push_str("\n:: p0\nHere.\n");
        src
    }

    /// The reference answer: breadth-first search over the ways as given.
    fn shortest(n: usize, ways: &[(usize, usize, bool)], from: usize) -> Vec<Option<usize>> {
        let mut dist = vec![None; n];
        dist[from] = Some(0);
        let mut queue = VecDeque::from([from]);
        while let Some(at) = queue.pop_front() {
            for &(a, b, one_way) in ways {
                for (x, y) in [(a, b), (b, a)] {
                    if (x, y) == (b, a) && one_way {
                        continue;
                    }
                    if x == at && dist[y].is_none() {
                        dist[y] = Some(dist[at].unwrap() + 1);
                        queue.push_back(y);
                    }
                }
            }
        }
        dist
    }

    fn graphs() -> impl Strategy<Value = (usize, Vec<(usize, usize, bool)>)> {
        (2..8usize).prop_flat_map(|n| {
            let way = (0..n, 0..n, any::<bool>()).prop_filter("no loops", |(a, b, _)| a != b);
            (Just(n), proptest::collection::vec(way, 0..12))
        })
    }

    proptest! {
        /// A route exists exactly when the place is reachable; it is as
        /// short as the shortest path; and every step follows a way, in a
        /// direction the way allows.
        #[test]
        fn routes_are_shortest_and_follow_the_ways((n, ways) in graphs(), to in 0..8usize) {
            let to = to % n;
            let ir = crate::build(&map(n, &ways)).map_err(|r| TestCaseError::fail(format!("{:?}", r.diags)))?;
            let story = Story::new(&ir).unwrap();
            let mut run = Run::new(&story, 1);
            let want = shortest(n, &ways, 0);
            let route = run.route("p0", &format!("p{to}"), None, Walker::Npc).unwrap();
            match (route, want[to]) {
                (None, None) => {}
                (Some(path), Some(d)) => {
                    prop_assert_eq!(path.len(), d);
                    let mut at = String::from("p0");
                    for next in path {
                        let step_ok = story.ways.iter().any(|(_, a, b)| *a == at && *b == next);
                        prop_assert!(step_ok, "no way from {} to {}", at, next);
                        at = next;
                    }
                }
                (got, want) => prop_assert!(false, "route {:?}, but the shortest path is {:?}", got, want),
            }
            // Distances agree with the reference everywhere.
            let found: HashMap<String, usize> = run.distances("p0", None, Walker::Npc).unwrap();
            for (i, d) in want.iter().enumerate() {
                prop_assert_eq!(found.get(&format!("p{i}")).copied(), *d);
            }
        }
    }
}
