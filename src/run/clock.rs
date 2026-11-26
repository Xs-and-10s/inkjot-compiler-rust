//! The clock's names: periods, `wait until`, and when a turn ends.
//!
//! ```text
//! period phase: night 1, day 1
//! on phase begins day
//! ~ then say lamplighter "Morning."
//!
//! > [[Night falls->night]] ~ for cohort ~ then wait until night
//!
//! clock: 0.. ~ shown as "turn"
//! ~ turn ends when pov.all.spent or pov.all.passed
//! ```
//!
//! **Periods.** The clock counts turns; a `period` names stretches of them
//! (see `src/period.rs` for the arithmetic). `clock.phase` is the name it
//! is now, `night`, and names of one period compare in its order:
//! `clock.hour >= ten`. When a turn ends, each name that ends with it fires
//! `on hour ends eleven`, innermost first; then the clock moves on, and each
//! name that begins fires `on phase begins night`, outermost first, then
//! `on turn begins`. The first turn is the story's start, not a beginning:
//! `on start` is for that.
//!
//! **`wait until night`** ends turns, each as usual, until a night begins.
//! If one begins on this turn, there is nothing to wait for. A clock with a
//! last stop stops there, whatever is still to come.
//!
//! **`turn ends when`.** With it, a turn ends when its condition holds after
//! an action, not on each `pass`: a `pass` says the pov is done
//! (`me.passed`), and `x.spent` says x has none left of what Links cost
//! (`default cost: vults`). Ex 2's two heroes both pass, or both run out,
//! and only then does the Host's turn come.

use super::events::Event;
use super::{f, Run, R};
use crate::json::Value;
use std::sync::Arc;

impl Run {
    /// The names that end with this turn, innermost first: `on hour ends
    /// eleven`. Nothing ends if the clock is at its last stop.
    pub(super) fn periods_end(&mut self) -> R<()> {
        if self.last_stop().is_some_and(|last| self.chronon >= last) {
            return Ok(());
        }
        let story = Arc::clone(&self.story);
        let now = story.calendar.at(self.chronon);
        let next = story.calendar.begins(self.chronon + 1);
        for (period, _) in next.iter().rev() {
            if let Some((_, name)) = now.iter().find(|(p, _)| p == period) {
                self.fire_period("ends", period, name)?;
            }
        }
        Ok(())
    }

    /// The names that begin with the turn just begun, outermost first, then
    /// the turn itself: `on phase begins night`, `on turn begins`.
    pub(super) fn periods_begin(&mut self) -> R<()> {
        let story = Arc::clone(&self.story);
        for (period, name) in story.calendar.begins(self.chronon) {
            self.fire_period("begins", &period, &name)?;
        }
        let actor = self.pov.clone();
        self.fire(Event {
            on: "begins",
            what: "turn",
            actor: &actor,
            before: false,
        })?;
        Ok(())
    }

    fn fire_period(&mut self, on: &str, period: &str, name: &str) -> R<()> {
        let actor = self.pov.clone();
        let what = format!("{period}.{name}");
        self.fire(Event {
            on,
            what: &what,
            actor: &actor,
            before: false,
        })?;
        Ok(())
    }

    /// `wait until night`: end turns until a night begins.
    pub(super) fn wait_until(&mut self, time: &str) -> R<()> {
        let story = Arc::clone(&self.story);
        let Some(due) = story.calendar.next_start(time, self.chronon) else {
            return Err(format!(
                "`wait until {time}`: `{time}` is not a time of this Inklink's periods"
            ));
        };
        while self.chronon < due {
            let before = self.chronon;
            self.end_turn()?;
            self.waited = true;
            // The clock has stopped at its last stop.
            if self.chronon == before {
                break;
            }
        }
        Ok(())
    }

    /// `clock: … ~ turn ends when …`, if the Inklink has one.
    pub(super) fn turn_ends_when(&self) -> Option<Value> {
        self.story
            .defaults
            .get("clock_turn_ends_when")
            .filter(|c| **c != Value::Null)
            .cloned()
    }

    /// Does the turn's condition hold now?
    pub(super) fn turn_is_over(&mut self) -> R<bool> {
        match self.turn_ends_when() {
            Some(cond) => Ok(self.eval(&cond)?.truthy()),
            None => Ok(false),
        }
    }

    /// `x.spent`: x has none left of what Links cost by default.
    pub(super) fn spent(&self, who: &str) -> bool {
        self.purse(who).is_some_and(|n| n <= 0.0)
    }
}

// ------------------------------------------------------------ real time

/// `400ms`, `30s`, `5m`, `2h`: a real time, in milliseconds.
pub(crate) fn millis(text: &str) -> Option<u64> {
    let text = text.trim();
    let digits = text.find(|c: char| !c.is_ascii_digit())?;
    let n: u64 = text[..digits].parse().ok()?;
    let per = match &text[digits..] {
        "ms" => 1,
        "s" => 1_000,
        "m" => 60_000,
        "h" => 3_600_000,
        _ => return None,
    };
    Some(n * per)
}

impl Run {
    /// `> [[To the vote->vote]] ~ after 5m`: how long the Link waits, if it
    /// is timed in real time.
    pub(super) fn timed(link: &Value) -> Option<u64> {
        f(link, "after").as_str().and_then(millis)
    }

    /// How long until a timed Link is taken by itself: its time, less what
    /// has passed since it was first offered this turn.
    pub fn due_in(&self, link: &Value) -> Option<u64> {
        let after = Self::timed(link)?;
        let id = f(link, "id").as_str().unwrap_or("");
        let since = self.offered_at.get(id).map_or(0, |at| self.ms - at);
        Some(after.saturating_sub(since))
    }

    /// `due_in` for the offered Link with this id.
    pub fn due_in_id(&self, id: &str) -> Option<u64> {
        let link = self
            .links
            .iter()
            .find(|l| f(l, "id").as_str() == Some(id))?;
        self.due_in(link)
    }

    /// Real time passes: `ms` milliseconds, as the platform counts them.
    /// Every timed Link whose time has come is taken, by whoever it is
    /// offered to; one `~ for cohort`, once for the whole table.
    pub fn wait(&mut self, ms: u64) -> R<()> {
        self.begin_action();
        let served = self.pov.clone();
        let seats = if self.order.is_empty() {
            vec![served.clone()]
        } else {
            self.order.clone()
        };
        // What is on offer now has been on offer since now, at the latest.
        for who in &seats {
            self.seat(who)?;
            for link in self.links.clone() {
                if Self::timed(&link).is_some()
                    && self.with_link(&link, |run| run.available(&link))?
                {
                    let id = f(&link, "id").as_str().unwrap_or("").to_string();
                    self.offered_at.entry(id).or_insert(self.ms);
                }
            }
        }
        self.ms += ms;
        for who in &seats {
            self.seat(who)?;
            let due = self
                .links
                .clone()
                .into_iter()
                .find(|l| self.due_in(l) == Some(0));
            if let Some(link) = due {
                if self.with_link(&link, |run| run.available(&link))? {
                    let id = f(&link, "id").as_str().unwrap_or("").to_string();
                    self.offered_at.remove(&id);
                    self.take(&link)?;
                    self.settle_deaths()?;
                }
            }
        }
        self.seat(&served)?;
        self.enter_finale()
    }

    /// Serve this Linker: sit at their seat at a table, or, alone, be them.
    fn seat(&mut self, who: &str) -> R<()> {
        if self.order.is_empty() {
            Ok(())
        } else {
            self.sit(who)
        }
    }
}
