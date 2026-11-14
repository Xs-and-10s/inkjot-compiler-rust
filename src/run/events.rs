//! `on` handlers: what happens when something happens.
//!
//! ```text
//! on cross glyph_door when not (glyph.broken or glyph.frozen)
//! ~ then wound actor 1
//! ~ then play chapel:glyph_fires
//! ```
//!
//! The interpreter calls [`Run::fire`] at each event: a walk across a way, a
//! take, a fact raised, a turn ending. Every handler for that event runs in
//! the order it was written. A `before` handler may cancel the thing with
//! `~ then stop`.

use super::{f, Local, Run, R, V};
use crate::json::Value;

/// How deeply handlers may set off other handlers before the interpreter
/// stops, rather than loop forever.
const MAX_DEPTH: usize = 8;

/// One event: what happened, to what, and who did it.
pub(super) struct Event<'a> {
    pub on: &'a str,
    /// The thing it happened to: a place, a way, an item, a passage, a fact.
    pub what: &'a str,
    pub actor: &'a str,
    pub before: bool,
}

impl Run<'_> {
    /// Run every handler for `event`. Returns `true` if one said `stop`.
    pub(super) fn fire(&mut self, event: Event) -> R<bool> {
        if self.firing >= MAX_DEPTH {
            return Err(format!(
                "`on {}` handlers set each other off more than {MAX_DEPTH} deep; check for a loop",
                event.on
            ));
        }
        let story = self.story;
        let mut stopped = false;
        for (i, (handler, owner)) in story.handlers.iter().enumerate() {
            if f(handler, "on").as_str() != Some(event.on) {
                continue;
            }
            let before = f(handler, "timing").as_str() == Some("before");
            if before != event.before {
                continue;
            }
            // `what`: the handler's first word after the event must name the thing.
            if let Some(what) = f(handler, "what")
                .as_arr()
                .and_then(|w| w.first())
                .and_then(Value::as_str)
            {
                let last = event.what.rsplit('.').next().unwrap_or(event.what);
                if what != event.what && what != last {
                    continue;
                }
            }
            // A rewind `unseen by hask` is not seen by hask's handlers.
            if owner.as_ref().is_some_and(|o| self.unseen_by.contains(o)) {
                continue;
            }
            if let Some(by) = f(handler, "by").as_str() {
                let a = event.actor;
                let fits = a == by
                    || story.has_tag(a, by)
                    || story.kind(a) == Some(by)
                    || story.field(a, "like").as_str() == Some(by);
                if !fits {
                    continue;
                }
            }
            let fired = self.handler_counts.get(i).copied().unwrap_or(0);
            if f(handler, "once") == &Value::Bool(true) && fired >= 1 {
                continue;
            }
            if let Some(most) = f(handler, "at_most").as_u64() {
                if u64::from(fired) >= most {
                    continue;
                }
            }
            // Inside a handler, `actor` is who did it and `self` is the owner.
            let saved = self.locals.clone();
            self.locals
                .insert("actor".into(), Local::Value(V::Id(event.actor.to_string())));
            if let Some(owner) = owner {
                self.locals
                    .insert("self".into(), Local::Value(V::Id(owner.clone())));
            }
            self.firing += 1;
            let result = self.run_handler(handler);
            self.firing -= 1;
            self.locals = saved;
            let result = result.map_err(|e| format!("{e} (in an `on {}` handler)", event.on))?;
            match result {
                None => continue, // its `when` did not hold
                Some(stop) => {
                    if i >= self.handler_counts.len() {
                        self.handler_counts.resize(i + 1, 0);
                    }
                    self.handler_counts[i] += 1;
                    stopped |= stop;
                }
            }
        }
        Ok(stopped)
    }

    /// `None` if the handler's condition did not hold; otherwise whether it
    /// said `stop`.
    fn run_handler(&mut self, handler: &Value) -> R<Option<bool>> {
        let when = f(handler, "when");
        if *when != Value::Null && !self.eval(when)?.truthy() {
            return Ok(None);
        }
        let mut stop = false;
        for effect in f(handler, "then").as_arr().unwrap_or(&[]).to_vec() {
            if f(&effect, "do").as_str() == Some("stop") {
                stop = true;
            }
            self.apply(&effect, &[])?;
        }
        Ok(Some(stop))
    }
}
