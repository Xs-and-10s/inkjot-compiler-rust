//! What an act asks before it can be done, and how often.
//!
//! ```text
//! act pick: cost 20 ~ requires lockpick
//! act together: cost 30 from each ~ requires pov.all here
//! act second_dawn: cost rest ~ min 20 ~ prayer ~ rewind ~ once
//! combo backward_knot: mend, hush, tug, mend ~ spell ~ rewind ~ once per turn
//! ```
//!
//! - `requires lockpick`: the actor holds that item, or an item tagged so.
//!   `requires pov.all here`: every pov is alive and in the actor's place;
//!   `requires pov.all alive`: every pov is alive, wherever they are.
//!   `requires full vults`: the actor has every vult.
//! - `cost 30 from each`: every pov pays, and every pov must be able to.
//! - `once`: each character may do it once a reading. `once per turn` (or
//!   `per chronon`, or the clock's own word): once a turn. `once per day`:
//!   once a day, where `clock: … ~ day 6` makes six turns a day.
//!
//! The uses of an act that rewinds belong to the player, like its charges:
//! going back in time does not untie the knot that took you there. Other
//! uses belong to the world, and a rewind takes them back.

use super::{f, strs, Run, R, V};
use crate::json::Value;

impl Run {
    /// May the actor do this act now, as far as `requires`, `once` and
    /// `cost … from each` go?
    pub(super) fn act_allowed(&mut self, link: &Value, actor: &str) -> R<bool> {
        let Some(act) = f(link, "act").as_str().map(str::to_string) else {
            return Ok(true);
        };
        if !self.requirements_met(&act, actor)? {
            return Ok(false);
        }
        if let Some(limit) = self.limit(&act) {
            let now = self.period(&limit, self.chronon)?;
            for used in self.uses(actor, &act) {
                if self.period(&limit, used)? == now {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// `requires …` on the act.
    fn requirements_met(&mut self, act: &str, actor: &str) -> R<bool> {
        let words = self.story.field(act, "requires").clone();
        let Some(words) = words.as_arr() else {
            return Ok(true);
        };
        let names: Vec<String> = strs(&Value::Arr(words.to_vec()));
        // `requires pov.all here`.
        let pov_all = words
            .first()
            .and_then(|w| w.get("get"))
            .and_then(Value::as_arr);
        // `requires pov.all here`, or `requires pov.all alive`.
        if pov_all.is_some_and(|p| p.len() == 2 && p[1].as_str() == Some("all")) {
            let here = words.get(1).and_then(Value::as_str) == Some("here");
            let place = self.location(actor);
            let everyone = self.story.povs.clone().iter().all(|p| {
                (!here || self.location(p) == place)
                    && !self.vals.get(&format!("{p}.dead")).is_some_and(V::truthy)
            });
            return Ok(everyone);
        }
        // `requires full vults`: as much of that resource as it holds full.
        if names.first().map(String::as_str) == Some("full") {
            let resource = names.get(1).cloned().unwrap_or_default();
            return Ok(self.is_full(actor, &resource));
        }
        // `requires lockpick`: hold it, or something tagged so.
        for need in names {
            let holds = self
                .holder
                .iter()
                .any(|(item, h)| h == actor && (*item == need || self.tagged(item, &need)));
            if !holds {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// `once` (for the whole reading) or `once per turn | day`.
    fn limit(&self, act: &str) -> Option<String> {
        if let Some(per) = self.story.field(act, "once_per").as_str() {
            return Some(per.to_string());
        }
        self.story
            .has_tag(act, "once")
            .then(|| String::from("reading"))
    }

    /// The period a turn falls in, for a limit: the turn itself, its day,
    /// or 0 for the whole reading.
    fn period(&self, limit: &str, turn: u32) -> R<u32> {
        Ok(match limit {
            "reading" => 0,
            "day" => {
                let day = self
                    .story
                    .defaults
                    .get("clock_day")
                    .and_then(Value::as_u64)
                    .ok_or("`once per day` needs days on the clock: `clock: … ~ day n`")?;
                turn / day.max(1) as u32
            }
            _ => turn,
        })
    }

    /// The turns on which the actor did the act.
    fn uses(&self, actor: &str, act: &str) -> Vec<u32> {
        let key = format!("{actor}.uses.{act}");
        let log = if self.story.has_tag(act, "rewind") {
            self.player_uses.get(&key)
        } else {
            self.vals.get(&key)
        };
        match log {
            Some(V::List(items)) => items.iter().filter_map(V::num).map(|n| n as u32).collect(),
            _ => Vec::new(),
        }
    }

    /// Note that the actor did the act this turn.
    pub(super) fn record_use(&mut self, link: &Value, actor: &str) {
        let Some(act) = f(link, "act").as_str() else {
            return;
        };
        if self.limit(act).is_none() {
            return;
        }
        let key = format!("{actor}.uses.{act}");
        let rewinds = self.story.has_tag(act, "rewind");
        let log = if rewinds {
            self.player_uses.entry(key).or_insert(V::List(Vec::new()))
        } else {
            self.vals.entry(key).or_insert(V::List(Vec::new()))
        };
        if let V::List(items) = log {
            items.push(V::Num(f64::from(self.chronon)));
        }
    }
}
