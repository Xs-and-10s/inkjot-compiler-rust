//! Inquests: hints a player buys.
//!
//! ```text
//! resource inquests: 3 ~ shared
//!
//! :: oubliette [room]
//! inquest: "The grate is too heavy for anything you can do from down here." ~ wits ..=2
//! inquest: "Look past the grate. On the far door above, something only needs to slide." ~ wits 3..
//!
//! :: party_bar [lib everywhere]
//! > [[Spend an Inquest->here]] ~ inquest ~ when inquests > 0
//! ```
//!
//! A Link with `~ inquest` spends one Inquest and tells the pov the next
//! hint they have not heard. The hints come from the Nib the pov is on,
//! then the Nibs it belongs to (`ossuary` for `ossuary:bolt`), then its
//! place. A hint applies when the pov's stats are in its ranges and its
//! `when` holds; listed vaguest first, they make a ladder, one rung per
//! Inquest.
//!
//! The Link is offered only when it would tell something: an Inquest is
//! left, and a hint here applies that has not been heard. So it never costs
//! an Inquest for nothing. An echo spends nothing, so it cannot inquire.
//!
//! Inquests and the hints they bought belong to the player, not the
//! characters: a rewind gives neither back.

use std::collections::HashSet;

use super::{f, place_of, Run, R, V};
use crate::json::Value;

/// A hint heard: by which player, from which Nib, which line of its header.
pub(super) type Heard = HashSet<(String, String, usize)>;

impl Run<'_> {
    /// Whose hints these are. In solo play the one player switches between
    /// characters, so the hints are theirs; in a cohort, each Linker's own.
    fn player(&self) -> String {
        if self.order.len() > 1 {
            self.pov.clone()
        } else {
            String::new()
        }
    }

    /// Where hints may come from, nearest first.
    fn hint_nibs(&self) -> Vec<String> {
        let mut nibs = vec![self.here().to_string()];
        let mut name = self.here();
        while let Some((parent, _)) = name.rsplit_once(':') {
            nibs.push(parent.to_string());
            name = parent;
        }
        let place = place_of(self.here()).to_string();
        if !nibs.contains(&place) {
            nibs.push(place);
        }
        nibs
    }

    /// The next hint the pov would hear: the first unheard one that applies.
    fn next_hint(&mut self) -> R<Option<(String, usize, String)>> {
        let player = self.player();
        for nib in self.hint_nibs() {
            let header = match self.story.passages.get(&nib) {
                Some(p) => p.header.clone(),
                None => continue,
            };
            for (i, stmt) in header.iter().enumerate() {
                let Some(text) = stmt.get("inquest").and_then(Value::as_str) else {
                    continue;
                };
                if self.heard.contains(&(player.clone(), nib.clone(), i)) {
                    continue;
                }
                if self.hint_applies(stmt)? {
                    return Ok(Some((nib, i, text.to_string())));
                }
            }
        }
        Ok(None)
    }

    /// `~ wits 3..`: each stat of the pov in its range; `~ when …` holds.
    fn hint_applies(&mut self, stmt: &Value) -> R<bool> {
        if let Some(Value::Obj(stats)) = stmt.get("stats") {
            for (stat, range) in stats {
                let have = self.name(stat)?;
                if !have.within(&self.constant(range)) {
                    return Ok(false);
                }
            }
        }
        let when = f(stmt, "when");
        Ok(*when == Value::Null || self.eval(when)?.truthy())
    }

    /// The Inquests the pov may spend: the shared count, or their own.
    fn inquests_left(&mut self) -> R<f64> {
        let key = self.path(&Value::str("inquests"))?;
        Ok(self.vals.get(&key).and_then(V::num).unwrap_or(0.0))
    }

    /// Would an Inquest tell the pov anything here?
    pub(super) fn may_inquire(&mut self) -> R<bool> {
        if self.in_past() || self.inquests_left()? < 1.0 {
            return Ok(false);
        }
        Ok(self.next_hint()?.is_some())
    }

    /// Spend an Inquest and hear the next hint.
    pub(super) fn inquire(&mut self) -> R<()> {
        let Some((nib, i, text)) = self.next_hint()? else {
            return Err("there is no hint here to spend an Inquest on".into());
        };
        let key = self.path(&Value::str("inquests"))?;
        let left = self.inquests_left()?;
        self.vals.insert(key, V::Num(left - 1.0));
        self.heard.insert((self.player(), nib, i));
        self.output.push(text);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::run::{Run, Story, V};
    use proptest::prelude::*;

    const EX2: &str = include_str!("../../examples/ex2-the-ninefold-cell.twee");

    fn left(run: &Run) -> f64 {
        run.vals.get("inquests").and_then(V::num).unwrap_or(-1.0)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// Wandering Ex 2 and spending Inquests whenever offered: each one
        /// spent is exactly one hint heard, new each time, and the count
        /// never goes below zero or comes back, whatever rewinds and deaths
        /// happen on the way.
        #[test]
        fn every_inquest_buys_exactly_one_new_hint(
            seed in 1..500u64,
            picks in proptest::collection::vec((any::<bool>(), 0..20usize), 0..80),
        ) {
            let ir = crate::build(EX2).unwrap();
            let story = Story::new(&ir).unwrap();
            let mut run = Run::new(&story, seed);
            run.start(None, None).unwrap();
            let start = left(&run);
            let mut low = start;
            for (inquire, pick) in picks {
                let Ok(choices) = run.choices() else { break };
                if choices.is_empty() {
                    break;
                }
                let spend = choices.iter().find(|(_, label)| label == "Spend an Inquest");
                let (id, _) = match spend {
                    Some(c) if inquire => c.clone(),
                    _ => choices[pick % choices.len()].clone(),
                };
                let before = (left(&run), run.heard.len());
                run.take_output();
                if run.choose(&id).is_err() {
                    break;
                }
                let after = (left(&run), run.heard.len());
                if spend.is_some_and(|(s, _)| *s == id) {
                    prop_assert_eq!(after.0, before.0 - 1.0);
                    prop_assert_eq!(after.1, before.1 + 1);
                    prop_assert!(!run.take_output().is_empty());
                } else {
                    prop_assert_eq!(after.1, before.1);
                }
                prop_assert!(left(&run) >= 0.0);
                prop_assert!(left(&run) <= low, "an Inquest came back");
                low = left(&run);
                prop_assert_eq!(run.heard.len() as f64, start - low);
            }
        }
    }
}
