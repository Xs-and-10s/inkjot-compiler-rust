//! Cards, piles, and a character for each Linker.
//!
//! ```text
//! character guest: "a guest" ~ selectable ~ custom ~ one per linker
//! deck roles
//! ~ card mafia: members
//! ~ card doctor: 1
//! ~ card townsfolk: rest
//! pile hand ~ per guest ~ private ~ face down ~ shown as "your card"
//!
//! > [[Deal->dealt]] ~ then shuffle roles ~ then deal 1 from roles to each hand
//! set me.role: top of my hand
//! ```
//!
//! **One per Linker.** A character `~ one per linker` is a seat's worth of
//! characters: `guest_1`, `guest_2`, … one for each Linker seated, up to
//! the cohort's largest size. Each is its own character, with its own
//! state and name (`~ custom`: the Linker types it). The name stands for
//! all those seated: `count(guest)`, `all(guest, where: …)`, `~ at guest`.
//!
//! **Decks.** A deck is built from its `card`s the first time it is
//! shuffled: `card doctor: 1` is one, `card mafia: members` as many as that
//! says, `card townsfolk: rest` as many as make one card for each player
//! seated; `card locket, compass` is one of each. `shuffle` builds it if
//! need be, then shuffles what is in it.
//!
//! **Piles.** `pile hand ~ per guest` is one pile for each guest: `my
//! hand`, `target.keepsake`, and over a list, everyone's together
//! (`all(guest, …).loot`). `deal 1 from roles to each hand` deals from
//! the top of the deck, in seat order. `top of …` is the top card of a
//! pile or any list; `count(…)` counts one. `move locket to me.loot` takes
//! a card from wherever it is; `move each found to it.owner.keepsake`, one
//! at a time, `it` each. Cards are names, and may carry facts of their own
//! (`set taken.owner: victim`).
//!
//! `~ private`, `~ face down` and `~ face up` say who may see a pile; the
//! platform shows them. `{list hand of each guest}` shows everyone's.

use std::sync::Arc;

use super::{f, Entry, Local, Run, R, V};
use crate::json::Value;

impl Run {
    /// The pile `pile` of `owner`, as a list's name.
    fn pile_key(owner: &str, pile: &str) -> String {
        format!("{owner}.{pile}")
    }

    /// The cards in a list: a deck, or someone's pile.
    fn cards(&self, key: &str) -> Vec<String> {
        self.lists
            .get(key)
            .map(|l| l.iter().map(|e| e.id.clone()).collect())
            .unwrap_or_default()
    }

    fn put(&mut self, key: &str, card: &str) {
        self.lists.entry(key.to_string()).or_default().push(Entry {
            id: card.to_string(),
            label: None,
            mark: None,
            struck: false,
        });
    }

    /// `x.hand`: the cards in x's pile.
    pub(super) fn pile_of(&self, owner: &str, pile: &str) -> V {
        V::List(
            self.cards(&Self::pile_key(owner, pile))
                .into_iter()
                .map(V::Id)
                .collect(),
        )
    }

    /// How many players are seated: one, playing solo.
    fn seated_count(&self) -> usize {
        self.order.len().max(1)
    }

    /// `shuffle roles`: build the deck if it has never been, then shuffle.
    pub(super) fn shuffle(&mut self, deck: &str) -> R<()> {
        let built = format!("{deck}.built");
        if !self.vals.get(&built).is_some_and(V::truthy) {
            let spec = self
                .story
                .field(deck, "cards")
                .as_arr()
                .unwrap_or(&[])
                .to_vec();
            let mut counted = Vec::new();
            let mut rest = None;
            for c in &spec {
                let card = f(c, "card").as_str().unwrap_or("").to_string();
                match f(c, "count") {
                    Value::Str(r) if r == "rest" => rest = Some(card),
                    n => {
                        let n = self.eval(&n.clone())?.num().unwrap_or(0.0).max(0.0) as usize;
                        counted.push((card, n));
                    }
                }
            }
            let used: usize = counted.iter().map(|(_, n)| n).sum();
            if let Some(card) = rest {
                counted.push((card, self.seated_count().saturating_sub(used)));
            }
            self.lists.insert(deck.to_string(), Vec::new());
            for (card, n) in counted {
                for _ in 0..n {
                    self.put(deck, &card);
                }
            }
            self.vals.insert(built, V::Bool(true));
        }
        // Fisher–Yates, from the run's own dice.
        let mut cards = self.lists.remove(deck).unwrap_or_default();
        for i in (1..cards.len()).rev() {
            let j = self.roll(i as u64 + 1) as usize - 1;
            cards.swap(i, j);
        }
        self.lists.insert(deck.to_string(), cards);
        Ok(())
    }

    /// `deal 1 from roles to each hand`: from the top of the deck, in seat
    /// order, to every pile of that name.
    pub(super) fn deal(&mut self, n: usize, deck: &str, pile: &str) -> R<()> {
        let owners = self.pile_owners(pile);
        for owner in owners {
            for _ in 0..n {
                let mut cards = self.lists.remove(deck).unwrap_or_default();
                if cards.is_empty() {
                    self.lists.insert(deck.to_string(), cards);
                    return Ok(()); // the deck has run out
                }
                let top = cards.remove(0);
                self.lists.insert(deck.to_string(), cards);
                self.put(&Self::pile_key(&owner, pile), &top.id);
            }
        }
        Ok(())
    }

    /// Who has a pile of this name: everyone it is `per`, seated first.
    fn pile_owners(&self, pile: &str) -> Vec<String> {
        let per = self
            .story
            .field(pile, "per")
            .as_str()
            .unwrap_or("")
            .to_string();
        let mut owners: Vec<String> = self
            .order
            .iter()
            .filter(|w| {
                self.story.of(w) == Some(per.as_str()) || self.story.kind(w) == Some(per.as_str())
            })
            .cloned()
            .collect();
        if owners.is_empty() {
            owners = self.things(&per, None);
        }
        owners
    }

    /// `move locket to first_voter.loot`: a card, from wherever it is, to
    /// a pile. `None` if `to` does not name a pile.
    pub(super) fn move_card(&mut self, card: &str, to: &Value) -> R<Option<()>> {
        let Some(path) = to.get("get").and_then(Value::as_arr) else {
            return Ok(None);
        };
        let Some(pile) = path.last().and_then(Value::as_str) else {
            return Ok(None);
        };
        if self.story.kind(pile) != Some("pile") {
            return Ok(None);
        }
        let owner = if path.len() == 2 {
            self.who(&path[0])?
        } else {
            self.who(&Value::obj([(
                "get",
                Value::Arr(path[..path.len() - 1].to_vec()),
            )]))?
        };
        for entries in self.lists.values_mut() {
            if let Some(i) = entries.iter().position(|e| e.id == card) {
                entries.remove(i);
                break;
            }
        }
        self.put(&Self::pile_key(&owner, pile), card);
        Ok(Some(()))
    }

    /// `move each found to it.owner.keepsake`: each of a list in turn.
    pub(super) fn move_each(&mut self, list: &Value, to: &Value) -> R<()> {
        let items = match self.eval(list)? {
            V::List(items) => items,
            V::Null => Vec::new(),
            one => vec![one],
        };
        let saved = self.locals.get("it").cloned();
        for item in items {
            let Some(card) = item.name().map(str::to_string) else {
                continue;
            };
            self.locals
                .insert("it".into(), Local::Value(V::Id(card.clone())));
            let moved = self.move_card(&card, to);
            if let Err(e) = moved {
                self.restore_local("it", saved);
                return Err(e);
            }
        }
        self.restore_local("it", saved);
        Ok(())
    }

    /// `{list hand of each guest}`: everyone's pile, one line each.
    pub(super) fn piles_text(&mut self, pile: &str, kind: &str) -> Vec<String> {
        let owners: Vec<String> = if self.story.kind(pile) == Some("pile") {
            let seated = self.pile_owners(pile);
            seated
                .into_iter()
                .filter(|o| self.story.of(o) == Some(kind) || self.story.kind(o) == Some(kind))
                .collect()
        } else {
            Vec::new()
        };
        owners
            .into_iter()
            .map(|o| {
                let cards: Vec<String> = self.cards(&Self::pile_key(&o, pile));
                let shown = self.show(&V::List(cards.into_iter().map(V::Id).collect()));
                format!("{}: {shown}", self.label(&o))
            })
            .collect()
    }

    /// Is this a character the seated Linkers each have one of, rather than
    /// a seat: `guest`, not `guest#2`?
    pub(super) fn is_per_linker(&self, id: &str) -> bool {
        self.story.field(id, "one_per_linker") == &Value::Bool(true) && self.story.of(id).is_none()
    }

    /// The seat a Linker sits in for a character `~ one per linker`: the
    /// next of them not yet taken. Anyone else is themselves.
    pub(super) fn seat_for(&self, who: &str, taken: &[String]) -> String {
        if !self.is_per_linker(who) {
            return who.to_string();
        }
        let story = Arc::clone(&self.story);
        let mut seats: Vec<&String> = story
            .decls
            .keys()
            .filter(|k| story.of(k) == Some(who))
            .collect();
        seats.sort_by_key(|k| k.rsplit('_').next().and_then(|n| n.parse::<u32>().ok()));
        seats
            .into_iter()
            .find(|k| !taken.contains(k))
            .cloned()
            .unwrap_or_else(|| who.to_string())
    }

    /// `max(1, players / 4)`, `min(…)`: the largest or smallest number.
    pub(super) fn extreme(&mut self, items: &[Value], most: bool) -> R<V> {
        let mut best: Option<f64> = None;
        for item in items {
            if let Some(n) = self.eval(item)?.num() {
                best = Some(match best {
                    Some(b) if most => b.max(n),
                    Some(b) => b.min(n),
                    None => n,
                });
            }
        }
        Ok(best.map_or(V::Null, V::Num))
    }
}
