//! Nearness on the map: `nearest(…, where:)`, `x.toward(y)`, `spreads`.
//!
//! ```text
//! let out: nearest(place, where: it.way_out) ~ along way
//! let step: here.toward(out)
//! spreads 1 from ablaze to nearest(place, where: it.timbered and it.heat == 0)
//! ~ along way ~ within 1 ~ then set it.heat: 1
//! move thwaite to nearest(place, where: it.heat == 0) ~ along way when …
//! ```
//!
//! Distance is counted in ways, along the map, open or not: fire and air
//! do not wait for a door to be mended, and Dowse tastes the way out even
//! when it is shut. A one-way way counts in its own direction only.
//!
//! `nearest` measures from someone: the thing being moved (`move thwaite
//! to nearest(…)`), each place a `spreads` starts from, the NPC whose
//! agenda it is (`self`), or else the pov. Its own place counts, at no
//! distance; ties go to the first by name. `~ within N` keeps to places N
//! ways away or fewer.

use std::collections::{BTreeMap, VecDeque};

use super::{f, place_of, Local, Run, R, V};
use crate::json::Value;

impl Run {
    /// How many ways each place is from `from`, along the map.
    fn map_distances(&self, from: &str) -> (BTreeMap<String, usize>, BTreeMap<String, String>) {
        let mut seen = BTreeMap::from([(from.to_string(), 0)]);
        let mut parent = BTreeMap::new();
        let mut queue = VecDeque::from([from.to_string()]);
        while let Some(at) = queue.pop_front() {
            for (_, a, b) in &self.story.ways {
                if *a != at || seen.contains_key(b) {
                    continue;
                }
                seen.insert(b.clone(), seen[&at] + 1);
                parent.insert(b.clone(), at.clone());
                queue.push_back(b.clone());
            }
        }
        (seen, parent)
    }

    /// Where `nearest` measures from: the anchor a statement set, the NPC
    /// whose move it is, or the pov.
    fn origin(&self) -> String {
        let who = match (&self.near, self.locals.get("self")) {
            (Some(near), _) => near.clone(),
            (None, Some(Local::Value(V::Id(npc)))) => npc.clone(),
            _ => self.pov.clone(),
        };
        self.place_id(&who)
    }

    /// The things of `kind` that pass `cond`, nearest first, from the
    /// origin, no further than `within` ways. Places out of reach are left
    /// out.
    pub(super) fn nearest_where(
        &mut self,
        kind: &str,
        cond: &Value,
        within: Option<usize>,
    ) -> R<Vec<String>> {
        let from = self.origin();
        let (dist, _) = self.map_distances(&from);
        let mut found = self.things(kind, None);
        if *cond != Value::Null {
            found = self.where_filter(found, cond)?;
        }
        let mut near: Vec<(usize, String)> = found
            .into_iter()
            .filter_map(|id| {
                let d = *dist.get(&self.place_id(&id))?;
                within.is_none_or(|w| d <= w).then_some((d, id))
            })
            .collect();
        near.sort();
        Ok(near.into_iter().map(|(_, id)| id).collect())
    }

    /// `here.toward(out)`: the place next to `a` on a shortest way to `b`.
    pub(super) fn toward(&self, a: &str, b: &str) -> V {
        let (from, to) = (self.place_id(a), self.place_id(b));
        if from == to {
            return V::Null;
        }
        let (_, parent) = self.map_distances(&from);
        let mut at = to;
        loop {
            match parent.get(&at) {
                None => return V::Null,
                Some(p) if *p == from => return V::Id(at),
                Some(p) => at = p.clone(),
            }
        }
    }

    /// `spreads N from SET to nearest(…) ~ within D ~ then …`: from each of
    /// the set, to the N nearest that fit, each `it` in the effects.
    pub(super) fn spread(&mut self, s: &Value) -> R<()> {
        let when = f(s, "when");
        if *when != Value::Null && !self.eval(when)?.truthy() {
            return Ok(());
        }
        let ring = f(s, "along").as_str().filter(|h| self.is_ring(h)).is_some();
        if let Some(how) = f(s, "along").as_str().filter(|h| *h != "way" && !ring) {
            return Err(format!(
                "`spreads … ~ along {how}`: `along way`, or a relation the table sits in, \
                 `relation seat: each guest ~ ring`"
            ));
        }
        let n = f(s, "spreads").as_u64().unwrap_or(1) as usize;
        let within = f(s, "within").as_u64().map(|w| w as usize);
        let sources = match f(s, "from") {
            Value::Null => vec![self.pov.clone()],
            from => self.items_named(&from.clone())?,
        };
        let to = f(s, "to").clone();
        let (kind, cond) = nearest_call(&to)
            .ok_or("`spreads` reaches `nearest(kind, where: …)`: say which things it may reach")?;
        let then = f(s, "then").as_arr().unwrap_or(&[]).to_vec();
        let mut reached = Vec::new();
        for source in sources {
            // Round the table: the seats after the source's, in order.
            if ring {
                let mut found = self.things(&kind, None);
                if cond != Value::Null {
                    found = self.where_filter(found, &cond)?;
                }
                let mut round: Vec<(usize, String)> = found
                    .into_iter()
                    .filter_map(|id| Some((self.seats_from(&source, &id)?, id)))
                    .filter(|(d, _)| *d > 0 && within.is_none_or(|w| *d <= w))
                    .collect();
                round.sort();
                for (_, id) in round.into_iter().take(n) {
                    if !reached.contains(&id) {
                        reached.push(id);
                    }
                }
                continue;
            }
            self.near = Some(source);
            let near = self.nearest_where(&kind, &cond, within);
            self.near = None;
            for id in near?.into_iter().take(n) {
                if !reached.contains(&id) {
                    reached.push(id);
                }
            }
        }
        let saved = self.locals.get("it").cloned();
        for id in reached {
            self.locals.insert("it".into(), Local::Value(V::Id(id)));
            for effect in &then {
                if let Err(e) = self.apply(effect, &[]) {
                    self.restore_local("it", saved);
                    return Err(e);
                }
            }
        }
        self.restore_local("it", saved);
        Ok(())
    }

    /// `move thwaite to nearest(…)`: the place to move to, measured from
    /// who is moving.
    pub(super) fn move_target(&mut self, who: &str, to: &Value) -> R<Option<String>> {
        if let Some(name) = to.as_str() {
            if !self.locals.contains_key(name) {
                return Ok(Some(name.to_string()));
            }
        }
        self.near = Some(who.to_string());
        let v = self.eval(to);
        self.near = None;
        Ok(v?.name().map(|p| place_of(p).to_string()))
    }
}

/// `nearest(place, where: …)` as lowered: its kind and its condition.
pub(super) fn nearest_call(e: &Value) -> Option<(String, Value)> {
    let kind = e.get("nearest")?.as_arr()?.first()?.as_str()?.to_string();
    Some((kind, f(f(e, "with"), "where").clone()))
}
