//! Hints: help a player asks for.
//!
//! ```text
//! resource hints: 3 ~ shared
//!
//! :: oubliette [room]
//! hint: "The grate is too heavy for anything you can do from down here." ~ wits ..=2
//! hint: "Look past the grate. On the far door above, something only needs to slide." ~ wits 3..
//!
//! :: party_bar [lib everywhere]
//! > [[Ask for a hint->here]] ~ hint ~ when hints > 0
//! ```
//!
//! A Link with `~ hint` spends one hint request and tells the pov the next
//! hint they have not heard. The hints come from the Nib the pov is on,
//! then the Nibs it belongs to (`ossuary` for `ossuary:bolt`), then its
//! place. A hint applies when the pov's stats are in its ranges and its
//! `when` holds; listed vaguest first, they make a ladder, one rung per
//! request.
//!
//! The Link is offered only when it would tell something: a request is
//! left, and a hint here applies that has not been heard. So it never costs
//! a request for nothing. An echo spends nothing, so it cannot ask. A
//! Linker who turned hints off is never offered one.
//!
//! Requests and the hints they bought belong to the player, not the
//! characters: a rewind gives neither back.
//!
//! Hints are not Inquests. An Inquest asks the story to diverge from its
//! canon where the Inker hid a fork (`~ inquest`); it is reserved, not run.

use std::collections::BTreeSet;

use super::{f, place_of, Run, R, V};
use crate::json::Value;

/// A hint heard: by which player, from which Nib, which line of its header.
pub(super) type Heard = BTreeSet<(String, String, usize)>;

impl Run {
    /// The Linker turns hints off for this Inklink: no hint is offered.
    pub fn turn_off_hints(&mut self) {
        self.no_hints = true;
    }

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
    fn next_hint(&mut self) -> R<Option<(String, usize, Value)>> {
        let player = self.player();
        for nib in self.hint_nibs() {
            let header = match self.story.passages.get(&nib) {
                Some(p) => p.header.clone(),
                None => continue,
            };
            for (i, stmt) in header.iter().enumerate() {
                let Some(hint) = stmt.get("hint") else {
                    continue;
                };
                if self.heard.contains(&(player.clone(), nib.clone(), i)) {
                    continue;
                }
                if self.hint_applies(stmt)? {
                    return Ok(Some((nib, i, hint.clone())));
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

    /// The hint requests the pov may spend: the shared count, or their own.
    fn hints_left(&mut self) -> R<f64> {
        let key = self.path(&Value::str("hints"))?;
        Ok(self.vals.get(&key).and_then(V::num).unwrap_or(0.0))
    }

    /// Would asking for a hint tell the pov anything here?
    pub(super) fn may_hint(&mut self) -> R<bool> {
        if self.no_hints || self.in_past() || self.hints_left()? < 1.0 {
            return Ok(false);
        }
        Ok(self.next_hint()?.is_some())
    }

    /// Spend a request and hear the next hint.
    pub(super) fn give_hint(&mut self) -> R<()> {
        let Some((nib, i, text)) = self.next_hint()? else {
            return Err("there is no hint here to ask for".into());
        };
        // `me.asked_hint`, until the round ends.
        self.hinted.insert(self.pov.clone());
        let key = self.path(&Value::str("hints"))?;
        let left = self.hints_left()?;
        self.vals.insert(key, V::Num(left - 1.0));
        self.heard.insert((self.player(), nib, i));
        let text = self.text_para(&text)?;
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
        run.vals.get("hints").and_then(V::num).unwrap_or(-1.0)
    }

    /// A Linker who turned hints off is never offered one, even where one
    /// applies and requests are left.
    #[test]
    fn a_linker_can_turn_hints_off() {
        let ir = crate::build(EX2).unwrap();
        let story = std::sync::Arc::new(Story::new(&ir).unwrap());
        let offered = |off: bool| {
            let mut run = Run::new(story.clone(), 1);
            if off {
                run.turn_off_hints();
            }
            run.start(Some("oubliette"), Some("roland")).unwrap();
            let choices = run.choices().unwrap();
            choices.iter().any(|(_, label)| label == "Ask for a hint")
        };
        assert!(offered(false));
        assert!(!offered(true));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// Wandering Ex 2 and asking for hints whenever offered: each request
        /// spent is exactly one hint heard, new each time, and the count
        /// never goes below zero or comes back, whatever rewinds and deaths
        /// happen on the way.
        #[test]
        fn every_request_buys_exactly_one_new_hint(
            seed in 1..500u64,
            picks in proptest::collection::vec((any::<bool>(), 0..20usize), 0..80),
        ) {
            let ir = crate::build(EX2).unwrap();
            let story = std::sync::Arc::new(Story::new(&ir).unwrap());
            let mut run = Run::new(story.clone(), seed);
            run.start(None, None).unwrap();
            let start = left(&run);
            let mut low = start;
            for (inquire, pick) in picks {
                let Ok(choices) = run.choices() else { break };
                if choices.is_empty() {
                    break;
                }
                let spend = choices.iter().find(|(_, label)| label == "Ask for a hint");
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
                prop_assert!(left(&run) <= low, "a hint request came back");
                low = left(&run);
                prop_assert_eq!(run.heard.len() as f64, start - low);
            }
        }
    }
}
