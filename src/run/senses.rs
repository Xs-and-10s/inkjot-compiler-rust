//! What NPCs see and hear: moves, noise, and sleep.
//!
//! ```text
//! npc marsh: "Colonel Marsh" ~ witness ~ in sleeper
//! ~ asleep during 2..=4
//! ~ wakes on loud
//! ~ stays awake 1
//!
//! reaction chase
//! ~ when self.saw(dash, toward: self.heading)
//! ~ then dash toward self.heading
//! ```
//!
//! **Seeing a move.** Moves are simultaneous, so an NPC sees a Linker's move
//! from where it stood when the move was made. A walk is seen in the place
//! left and the place reached; a loud move (a dash) also in every place it
//! passed; a quiet move (a slip) only in the place reached, and only by an
//! awake witness. A sleeper sees nothing. `self.saw(dash, toward: p)` holds
//! when the NPC saw a dash this turn that reached or passed `p`, or ended
//! nearer to it than it began.
//!
//! **Noise.** A loud move is heard in every place it left, passed and
//! reached. A loud act (`force`) is heard in its place and the places next
//! to it. A move along a loud way (`way vestibule … ~ loud`) is loud. Noise
//! wakes anyone who `wakes on loud`: woken at one stop, they stay awake
//! through `stays awake n` more stops, then sleep again if it is still
//! their hour. Without `stays awake`, they stay awake for good.

use super::agenda::Walker;
use super::{f, place_of, strs, Run, R, V};
use crate::json::Value;

/// A Linker's move, as one NPC saw it.
#[derive(Clone, Debug)]
pub(super) struct SeenMove {
    pub(super) act: String,
    pub(super) from: String,
    pub(super) passed: Vec<String>,
    pub(super) to: String,
}

impl Run<'_> {
    /// Truly asleep: in the hours `asleep during` says, and not woken.
    /// Feigning is not sleeping.
    pub(super) fn sleeping(&self, who: &str) -> bool {
        let during = self.constant(self.story.field(who, "asleep_during"));
        V::Num(f64::from(self.chronon)).within(&during) && !self.woken(who)
    }

    /// Woken: by `wake`, for good, or by noise, until `awake_until`.
    pub(super) fn woken(&self, who: &str) -> bool {
        let flag = self
            .vals
            .get(&format!("{who}.woken"))
            .is_some_and(V::truthy);
        let until = self
            .vals
            .get(&format!("{who}.awake_until"))
            .and_then(V::num);
        flag || until.is_some_and(|n| f64::from(self.chronon) <= n)
    }

    /// Is this move loud: a loud act, or along a loud way?
    fn loud_move(&self, act: &str, path: &[String]) -> bool {
        if self.story.has_tag(act, "loud") {
            return true;
        }
        path.windows(2).any(|pair| {
            self.story.ways.iter().any(|(way, a, b)| {
                *a == pair[0] && *b == pair[1] && self.story.has_tag(way, "loud")
            })
        })
    }

    /// The places next to `place`, along any way, either direction.
    fn adjacent(&self, place: &str) -> Vec<String> {
        let mut out = Vec::new();
        for (_, a, b) in &self.story.ways {
            let other = if a == place {
                b
            } else if b == place {
                a
            } else {
                continue;
            };
            if !out.contains(other) {
                out.push(other.clone());
            }
        }
        out
    }

    /// Noise in these places wakes whoever `wakes on loud` there. Did
    /// anyone wake?
    pub(super) fn hear(&mut self, places: &[String]) -> bool {
        let mut woke = false;
        let sleepers: Vec<String> = self
            .story
            .ids_of("npc")
            .into_iter()
            .chain(self.story.ids_of("character"))
            .filter(|who| self.story.field(who, "wakes_on").as_str() == Some("loud"))
            .collect();
        for who in sleepers {
            let at = self.location(&who);
            if !places.contains(&at) || !self.sleeping(&who) {
                continue;
            }
            match self.story.field(&who, "stays_awake").as_u64() {
                Some(n) => {
                    let until = f64::from(self.chronon) + n as f64;
                    self.vals
                        .insert(format!("{who}.awake_until"), V::Num(until));
                }
                None => {
                    self.vals.insert(format!("{who}.woken"), V::Bool(true));
                }
            }
            woke = true;
            let line = format!("{} wakes with a start.", self.label(&who));
            for linker in self.linkers() {
                if self.location(&linker) == at {
                    self.tell(&linker, line.clone());
                }
            }
        }
        woke
    }

    /// A loud act (`force`) is heard here and next door.
    pub(super) fn loud_act(&mut self, place: &str) -> bool {
        let mut places = vec![place.to_string()];
        places.extend(self.adjacent(place));
        self.hear(&places)
    }

    /// A Linker's move: heard if it is loud, and seen by the NPCs placed
    /// to see it. Called while every NPC still stands where it stood.
    pub(super) fn witness_move(&mut self, link: &Value, from: &str, to: &str) {
        if from == to {
            return;
        }
        let act = f(link, "act").as_str().unwrap_or("walk").to_string();
        let passed = strs(f(link, "via"));
        let mut path = vec![from.to_string()];
        path.extend(passed.iter().cloned());
        path.push(to.to_string());
        let loud = self.loud_move(&act, &path);
        if loud {
            self.hear(&path);
        }
        let quiet = self.story.has_tag(&act, "quiet");
        for npc in self.story.ids_of("npc") {
            let at = self.location(&npc);
            if at.is_empty() || self.sleeping(&npc) {
                continue;
            }
            let sees = if quiet {
                at == to && self.tagged(&npc, "witness")
            } else if loud {
                path.contains(&at)
            } else {
                at == from || at == to
            };
            if sees {
                self.seen.entry(npc).or_default().push(SeenMove {
                    act: act.clone(),
                    from: from.to_string(),
                    passed: passed.clone(),
                    to: to.to_string(),
                });
            }
        }
    }

    /// `self.saw(dash, toward: p)`: this turn, the NPC saw a move of that
    /// pace that reached or passed `p`, or ended nearer to it.
    pub(super) fn saw(&mut self, npc: &str, pace: &str, toward: Option<&str>) -> R<bool> {
        let moves = self.seen.get(npc).cloned().unwrap_or_default();
        for m in moves.iter().filter(|m| m.act == pace) {
            let Some(p) = toward else {
                return Ok(true);
            };
            if m.to == p || m.passed.iter().any(|x| x == p) {
                return Ok(true);
            }
            let start = self.distances(&m.from, None, Walker::Npc)?.get(p).copied();
            let end = self.distances(&m.to, None, Walker::Npc)?.get(p).copied();
            if let (Some(start), Some(end)) = (start, end) {
                if end < start {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// `dash toward p`, in an NPC's reaction: up to two places along its
    /// route, loudly.
    pub(super) fn npc_dash(&mut self, npc: &str, toward: &str) -> R<()> {
        let here = self.location(npc);
        let route = self
            .route(&here, toward, None, Walker::Npc)?
            .unwrap_or_default();
        let mut path = vec![here];
        for place in route.into_iter().take(2) {
            path.push(place.clone());
            self.step(npc, &place)?;
        }
        if path.len() > 1 {
            self.hear(&path);
        }
        Ok(())
    }

    /// An NPC's step is loud along a loud way, or at a dash.
    pub(super) fn npc_noise(&mut self, from: &str, to: &str, dashing: bool) {
        let path = [from.to_string(), to.to_string()];
        if dashing || self.loud_move("", &path) {
            self.hear(&path);
        }
    }

    /// Where a Nib's own `moves:` lets a Linker move by: the character's
    /// moves, cut down to the Nib's (`moves: walk` in the mail car).
    pub(super) fn nib_moves(&self, passage: &str, own: Vec<String>) -> Vec<String> {
        let setting = self.story.passages.get(passage).and_then(|p| {
            p.header
                .iter()
                .find(|s| f(s, "setting").as_str() == Some("moves"))
                .cloned()
        });
        match setting.map(|s| strs(f(&s, "moves"))) {
            Some(allowed) if !allowed.is_empty() => {
                own.into_iter().filter(|m| allowed.contains(m)).collect()
            }
            _ => own,
        }
    }

    /// Where an NPC stood at an earlier stop, for an echo re-living it.
    pub(super) fn place_then(&self, npc: &str, stop: u32) -> Option<String> {
        self.history
            .iter()
            .rev()
            .find(|s| s.chronon == stop)
            .and_then(|s| s.place_of(npc))
            .map(|p| place_of(&p).to_string())
    }
}
