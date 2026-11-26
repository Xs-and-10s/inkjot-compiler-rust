//! Votes at a table: Links chosen together (`~ consensus`), a `{ballot}`
//! for one of a list, and who sits next to whom.
//!
//! ```text
//! > ~ for guest where: it.alive and it.team == mafia ~ consensus
//! > [[Kill {target}->night:done]] ~ at guest where: it.alive ~ then set victim: target
//!
//! {ballot for: chosen_special, from: specials, voters: all(guest), ties: draw}
//! ```
//!
//! **Consensus.** The Links of a group marked `~ consensus` are a vote among
//! those the group is `for`. Taking one casts a vote for its target, and
//! the voter goes on where it leads; nothing it says to do is done yet.
//! When the turn ends, the votes cast count (all of them, or as many as
//! came before a deadline; none, and nothing happens): the target most
//! chosen wins, a tie going to the one chosen first. Then the effects of
//! the first vote placed on the winner are done, by that voter (`me`), with
//! `target` the winner. `x.chosen_by` lists who voted for x, first to last.
//! `~ consensus 2`: each voter chooses two, and `targets` is the two most
//! chosen, most first.
//!
//! **A ballot** asks each voter (`voters:`, everyone seated unless it says)
//! for one of a list. When all have answered, the most chosen is kept in
//! `for:`, and `ballot.done` holds; a tie is drawn by lot (`ties: draw`,
//! and `ballot.drawn`).
//!
//! **Seats.** `relation seat: each guest ~ ring` sits the table in a ring,
//! in the order they joined: `spreads 1 from heard to nearest(guest,
//! where: …) ~ along seat` goes round it, to the next seat that fits.

use super::{f, Local, Run, R, V};
use crate::json::Value;

/// One vote in a consensus.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Vote {
    pub voter: String,
    pub target: String,
    /// The Link voted with, whose effects the winner's first vote does.
    pub link: Value,
}

impl Run {
    /// The consensus a Link votes in: its passage and who its group is
    /// for, which every Link of a group shares. `None` for any other Link.
    pub(super) fn consensus_of(link: &Value) -> Option<String> {
        link.get("consensus")?;
        let id = f(link, "id").as_str().unwrap_or("");
        let passage = id.split("->").next().unwrap_or(id);
        Some(format!(
            "{passage}|{}|{}",
            f(link, "for"),
            f(link, "for_where")
        ))
    }

    /// How many each voter chooses: `~ consensus 2`.
    fn choices_each(link: &Value) -> usize {
        f(link, "consensus").as_u64().unwrap_or(1).max(1) as usize
    }

    /// What the Link is taken on: `{target}`.
    fn target_of(link: &Value) -> Option<String> {
        f(f(link, "with"), "target").as_str().map(str::to_string)
    }

    /// May the pov vote with this Link: not for the same target twice, nor
    /// more often than the consensus asks.
    pub(super) fn may_vote(&self, link: &Value) -> bool {
        let Some(key) = Self::consensus_of(link) else {
            return true;
        };
        let mine: Vec<&Vote> = self
            .votes
            .get(&key)
            .map(|v| v.iter().filter(|v| v.voter == self.pov).collect())
            .unwrap_or_default();
        let target = Self::target_of(link);
        mine.len() < Self::choices_each(link)
            && !mine.iter().any(|v| Some(&v.target) == target.as_ref())
    }

    /// Cast the pov's vote. Whether they have chosen all they may, and go
    /// on where the Link leads.
    pub(super) fn cast_vote(&mut self, link: &Value) -> bool {
        let Some(key) = Self::consensus_of(link) else {
            return true;
        };
        let target = Self::target_of(link).unwrap_or_default();
        let votes = self.votes.entry(key).or_default();
        votes.push(Vote {
            voter: self.pov.clone(),
            target,
            link: link.clone(),
        });
        let mine = votes.iter().filter(|v| v.voter == self.pov).count();
        mine >= Self::choices_each(link)
    }

    /// The turn is ending: every consensus counts the votes cast, and the
    /// winner's first vote is done.
    pub(super) fn settle_votes(&mut self) -> R<()> {
        let all = std::mem::take(&mut self.votes);
        for (_, votes) in all {
            let Some(first) = votes.first() else { continue };
            let n = Self::choices_each(&first.link);
            // Each target, with its voters, in the order first chosen.
            let mut tally: Vec<(String, Vec<String>)> = Vec::new();
            for v in &votes {
                match tally.iter_mut().find(|(t, _)| *t == v.target) {
                    Some((_, by)) => by.push(v.voter.clone()),
                    None => tally.push((v.target.clone(), vec![v.voter.clone()])),
                }
            }
            for (target, by) in &tally {
                let by = V::List(by.iter().cloned().map(V::Id).collect());
                self.vals.insert(format!("{target}.chosen_by"), by);
            }
            // Most chosen first; a stable sort keeps the first chosen first.
            let mut ranked = tally.clone();
            ranked.sort_by_key(|(_, by)| std::cmp::Reverse(by.len()));
            let winners: Vec<String> = ranked.iter().take(n).map(|(t, _)| t.clone()).collect();
            let winner = winners[0].clone();
            let deed = votes
                .iter()
                .find(|v| v.target == winner)
                .cloned()
                .expect("the winner was voted for");
            let saved_pov = std::mem::replace(&mut self.pov, deed.voter.clone());
            let saved = self.locals.get("targets").cloned();
            self.locals.insert(
                "targets".into(),
                Local::Value(V::List(winners.iter().cloned().map(V::Id).collect())),
            );
            let effects = f(&deed.link, "effects").as_arr().unwrap_or(&[]).to_vec();
            let done = self.with_link(&deed.link, |run| {
                for e in &effects {
                    run.apply(e, &[])?;
                }
                Ok(())
            });
            self.restore_local("targets", saved);
            self.pov = saved_pov;
            done?;
        }
        Ok(())
    }

    /// `cohort.all.chose`: everyone seated has done something this turn.
    pub(super) fn all_chose(&self) -> bool {
        let seated = if self.order.is_empty() {
            vec![self.pov.clone()]
        } else {
            self.order.clone()
        };
        seated.iter().all(|s| self.chose.contains(s))
    }

    /// `it.voted_with(cult)`: x's vote at the last jury was the same as
    /// someone else's of that team (or that person's).
    pub(super) fn voted_with(&mut self, who: &str, side: &V) -> R<bool> {
        let Some(mine) = self.vals.get(&format!("{who}.voted_for")).cloned() else {
            return Ok(false);
        };
        let side = side.name().unwrap_or("").to_string();
        for other in self.order.clone() {
            if other == who {
                continue;
            }
            let theirs = self.vals.get(&format!("{other}.voted_for"));
            let same_side = other == side
                || self
                    .vals
                    .get(&format!("{other}.team"))
                    .and_then(V::name)
                    .is_some_and(|t| t == side);
            if same_side && theirs == Some(&mine) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// `~ along seat`: is `relation` a ring the table sits in?
    pub(super) fn is_ring(&self, relation: &str) -> bool {
        self.story.kind(relation) == Some("relation") && self.story.has_tag(relation, "ring")
    }

    /// Steps round the ring from one seat to another, in the order they
    /// joined. `None` for anyone not seated.
    pub(super) fn seats_from(&self, from: &str, to: &str) -> Option<usize> {
        let n = self.order.len();
        let a = self.order.iter().position(|s| s == from)?;
        let b = self.order.iter().position(|s| s == to)?;
        Some((b + n - a) % n)
    }
}
