//! Knowledge bases, as far as milestone 21e builds them: a place to keep
//! facts. Inklog (milestone 22) adds rules, questions and the rest.
//!
//! ```text
//! knowledge findings ~ per character ~ learned
//! knowledge suspicions ~ per linker ~ manual
//! > [[Look into {target}->night:looked]] ~ then assert {{ target guilty }} in findings when target.team != town
//! ```
//!
//! A base `~ per character` is one for each character: `assert … in
//! findings` keeps the fact in the base of whoever does it (`me`), so the
//! Detective's findings are the Detective's alone. `~ per linker` is the
//! same, one per seat. A base with no `per` is everyone's, one for the
//! story. `learned` and `manual` say how facts come in (by the story, or
//! written by the Linker on the platform); this slice keeps what the story
//! asserts.
//!
//! A fact is kept with its terms worked out: `{{ target guilty }}` keeps
//! the guest `target` names, not the word `target`. Asserting a fact the
//! base already holds changes nothing. `me.findings` is the list of facts,
//! each a list of its terms (`[guest_2, guilty]`), in the order kept; they
//! are part of the world, so a rewind or a checkpoint puts them back.

use super::{f, Run, R, V};
use crate::json::Value;

impl Run {
    /// Whose `base` a fact asserted now goes into: the pov's, for a base
    /// per character or per linker; else the story's.
    fn base_key(&self, base: &str) -> String {
        match self.story.field(base, "per").as_str() {
            Some(_) => format!("{}.{base}", self.pov),
            None => base.to_string(),
        }
    }

    /// `assert {{ s verb o @ time }} in base`.
    pub(super) fn assert_fact(&mut self, fact: &Value, base: &str) -> R<()> {
        if self.story.kind(base) != Some("knowledge") {
            return Err(format!("`{base}` is not a knowledge base"));
        }
        let mut terms = Vec::new();
        for key in ["subject", "verb", "object"] {
            let term = f(fact, key);
            if *term == Value::Null {
                continue;
            }
            if term.get("var").is_some() {
                return Err("a fact with a variable needs Inklog's rules (milestone 22)".into());
            }
            terms.push(match key {
                "verb" => V::Id(term.as_str().unwrap_or("").to_string()),
                _ => match self.eval(term)? {
                    V::Null => V::Id("nothing".into()),
                    v => v,
                },
            });
        }
        if let Some(at) = f(fact, "at").as_str() {
            terms.push(V::Str(format!("@ {at}")));
        }
        let fact = V::List(terms);
        let key = self.base_key(base);
        let mut held = match self.vals.remove(&key) {
            Some(V::List(facts)) => facts,
            _ => Vec::new(),
        };
        if !held.contains(&fact) {
            held.push(fact);
        }
        self.vals.insert(key, V::List(held));
        Ok(())
    }
}
