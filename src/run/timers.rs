//! Effects on the clock: `for N turns` and `after N turns`.
//!
//! ```text
//! :: spell:hush [act]
//! set me.hushed: yes ~ for 4 turns
//!
//! :: adit:burn [act]
//! set planks_out: yes ~ after 2 turns
//! ```
//!
//! A timing clause belongs to the effect before it. `after N turns` checks
//! the effect's `when` now, and does the effect when the clock has moved on
//! N turns; asked again while it waits, it still happens once, when first
//! asked. `for N turns` does the effect now, and when N turns have passed
//! puts back what it changed. Doing it again while it lasts makes it last
//! from then: the second hush does not end when the first would have.
//!
//! The waiting effects are part of the world, so a rewind or a checkpoint
//! puts them back as they were.

use super::time::{Due, Timer};
use super::{f, Local, Run, R, V};
use crate::json::Value;

impl Run {
    /// Hold back an effect `after N turns`. Its `when` already held.
    pub(super) fn delay(&mut self, e: &Value, turns: u32) {
        let effect = without(e, &["delay", "when"]);
        let locals = self
            .locals
            .iter()
            .filter_map(|(k, v)| match v {
                Local::Value(v) => Some((k.clone(), v.clone())),
                Local::Expr(_) => None,
            })
            .collect();
        let what = Due::Effect {
            effect,
            locals,
            pov: self.pov.clone(),
        };
        // The same effect already waiting is not set going twice: the
        // Gleaner's candle, blown out, is lit again three turns after, not
        // every turn after that as well.
        if !self.timers.iter().any(|t| t.what == what) {
            self.timers.push(Timer {
                due: self.chronon + turns,
                what,
            });
        }
    }

    /// Do an effect `for N turns`, and put back what it changed when they
    /// have passed.
    pub(super) fn last(&mut self, e: &Value, turns: u32, clauses: &[Value]) -> R<()> {
        let verb = f(e, "do").as_str().unwrap_or("");
        // `cover me ~ for 3 turns`: the cover wears off, if no threat used it.
        if verb == "cover" {
            let args = f(e, "args").as_arr().unwrap_or(&[]).to_vec();
            let who = self.targets(&args[..args.len().min(1)], &self.pov.clone())?;
            let keys: Vec<String> = who.iter().map(|w| format!("{w}.covered")).collect();
            self.apply(&without(e, &["lasts", "when"]), clauses)?;
            let due = self.chronon + turns;
            for key in keys {
                self.timers
                    .retain(|t| !matches!(&t.what, Due::Restore { key: k, .. } if *k == key));
                self.timers.push(Timer {
                    due,
                    what: Due::Restore {
                        key,
                        old: Some(V::Bool(false)),
                    },
                });
            }
            return Ok(());
        }
        let target = match verb {
            "set" => f(e, "target").clone(),
            "raise" | "lower" | "toggle" => f(e, "args")
                .as_arr()
                .and_then(|a| a.first())
                .cloned()
                .unwrap_or(Value::Null),
            _ => {
                return Err(format!(
                    "only `set`, `raise`, `lower` and `toggle` can last `for {turns} turns`, \
                     not `{verb}`"
                ))
            }
        };
        let keys = self.paths(&target)?;
        let before: Vec<(String, Option<V>)> = keys
            .into_iter()
            .map(|k| {
                let old = self.vals.get(&k).cloned();
                (k, old)
            })
            .collect();
        self.apply(&without(e, &["lasts", "when"]), clauses)?;
        let due = self.chronon + turns;
        for (key, old) in before {
            // Again while it lasts: it lasts from now, and puts back what
            // was there before the first.
            let waiting = self
                .timers
                .iter_mut()
                .find(|t| matches!(&t.what, Due::Restore { key: k, .. } if *k == key));
            match waiting {
                Some(t) => t.due = t.due.max(due),
                None => self.timers.push(Timer {
                    due,
                    what: Due::Restore { key, old },
                }),
            }
        }
        Ok(())
    }

    /// Everything due by now, in the order it was set going.
    pub(super) fn run_timers(&mut self) -> R<()> {
        let now = self.chronon;
        let (due, waiting): (Vec<Timer>, Vec<Timer>) = std::mem::take(&mut self.timers)
            .into_iter()
            .partition(|t| t.due <= now);
        self.timers = waiting;
        for timer in due {
            match timer.what {
                Due::Restore { key, old } => match old {
                    Some(v) => {
                        self.vals.insert(key, v);
                    }
                    None => {
                        self.vals.remove(&key);
                    }
                },
                Due::Effect {
                    effect,
                    locals,
                    pov,
                } => {
                    let saved = std::mem::take(&mut self.locals);
                    let saved_pov = std::mem::replace(&mut self.pov, pov);
                    self.locals = locals
                        .into_iter()
                        .map(|(k, v)| (k, Local::Value(v)))
                        .collect();
                    let result = self.apply(&effect, &[]);
                    self.locals = saved;
                    self.pov = saved_pov;
                    result?;
                }
            }
        }
        Ok(())
    }

    /// How many turns a timing field says: a number, or a name for one.
    pub(super) fn turns(&mut self, n: &Value) -> R<u32> {
        let v = self.eval(n)?;
        match v.num() {
            Some(n) if n >= 0.0 => Ok(n as u32),
            _ => Err(format!("a number of turns must be a number, not {v:?}")),
        }
    }
}

/// An effect without some of its fields.
fn without(e: &Value, keys: &[&str]) -> Value {
    match e {
        Value::Obj(pairs) => Value::Obj(
            pairs
                .iter()
                .filter(|(k, _)| !keys.contains(&k.as_str()))
                .cloned()
                .collect(),
        ),
        other => other.clone(),
    }
}
