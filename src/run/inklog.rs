//! Inklog: facts, the bases that keep them, the rules that make more of
//! them, and the questions a story asks.
//!
//! ```text
//! knowledge notebook ~ per linker ~ manual ~ reads knows
//! fact telegram_text: "…" ~ means {{ wren met organist }}, {{ lark at room_nine @ new_years_eve.ten }}
//! rule wears: {{ N is P }} and {{ N at A @ W }} => {{ P at A @ W }} ~ known
//! ~ when {{ you hunting }} in dossier
//! objective names: "…" ~ done when count(cover, where: some P. {{ it is P }} in notebook) == 3
//! ```
//!
//! **Facts.** A fact is a subject, a verb, perhaps an object, perhaps a
//! time: `{{ hale is ashe }}`, `{{ lark at room_nine @ new_years_eve.ten }}`.
//! Its terms are atoms: a bare name is itself (`{{ sky rains }}`), unless
//! it is in scope (`target`, `you`, a `let`), when it is what that names.
//! A capitalised name is a variable (`P`), and `?` is anything.
//!
//! **Bases.** `world` (the truth, shared) and `knows` (what each character
//! knows: the meanings, `~ means`, of every fact they have learned) are
//! built in. A declared base is shared, or one each (`~ per character`, `~
//! per linker`), or someone's (`~ for keller`); `~ starts with` gives it
//! facts at the start, and `~ reads knows` lets it see what its owner
//! knows. A fact is asked `{{ … }} in base`; with no `in`, the world. The
//! answer is the list of facts that match, so it is true when there are
//! any, and `count(…)` counts them.
//!
//! **Rules** make facts from facts, in the bases they hold in: `~ in
//! base`, or, with no `in`, the world, and each character's `knows` too
//! once they know the rule (`~ known`: from the start; `~ learned from
//! fact`: once they know that fact). A rule's body joins facts (sharing
//! variables), compares (`A != B`), and may say `not {{ … }}` in a base
//! where what is not there is false: the world, or one `~ truth`. Rules
//! run to a fixed point, those that ask `not` after those whose facts they
//! ask about. Derived facts are worked out when asked, never kept, so a
//! retracted fact takes what followed from it with it.
//!
//! **Quantifiers** end with a dot: `some P. {{ it is P }} in notebook`
//! holds when some P makes the body true; `no P. …` when none does.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::{f, Local, Run, R, V};
use crate::json::Value;

/// A fact as kept: subject, verb, object (or nothing), time (or nothing).
type Fact = Vec<V>;

/// Variables bound so far.
type Bindings = BTreeMap<String, V>;

/// The binding that holds the facts a solution stands on. No variable
/// can be named this.
const USED: &str = "#used";

/// How deep bases may read each other before the interpreter stops.
const MAX_READS: usize = 8;

impl Run {
    // ------------------------------------------------------------ bases

    /// Where `base` keeps its facts for `owner`: one each, someone's, or
    /// the story's.
    fn base_key(&self, base: &str, owner: &str) -> String {
        if base == "knows" {
            return format!("{owner}.knows");
        }
        if let Some(who) = self.story.field(base, "for").as_str() {
            return format!("{who}.{base}");
        }
        match self.story.field(base, "per").as_str() {
            Some(_) => format!("{owner}.{base}"),
            None => base.to_string(),
        }
    }

    /// Whose a base's facts are, asked by the pov: the owner a `~ for`
    /// names, or the pov.
    fn owner_of(&self, base: &str) -> String {
        self.story
            .field(base, "for")
            .as_str()
            .map_or_else(|| self.pov.clone(), str::to_string)
    }

    /// The facts kept in a base (not those rules make).
    fn kept(&self, key: &str) -> Vec<Fact> {
        match self.vals.get(key) {
            Some(V::List(facts)) => facts
                .iter()
                .filter_map(|f| match f {
                    V::List(terms) => Some(terms.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// `~ starts with {{ wren is keller }}`: every declared base's first
    /// facts, at the start of a run.
    pub(super) fn start_bases(&mut self) -> R<()> {
        let story = Arc::clone(&self.story);
        for base in story.ids_of("knowledge") {
            let facts = story
                .field(&base, "starts_with")
                .as_arr()
                .unwrap_or(&[])
                .to_vec();
            for fact in facts {
                self.assert_fact(&fact, &base)?;
            }
        }
        Ok(())
    }

    /// `assert {{ s verb o @ time }} in base`, or a value that holds a fact
    /// (`assert story in dossier`). A fact the base holds is not kept twice.
    pub(super) fn assert_fact(&mut self, fact: &Value, base: &str) -> R<()> {
        let fact = self.fact_of(fact)?;
        let owner = self.owner_of(base);
        let key = self.base_key(base, &owner);
        let mut held = self.kept(&key);
        if !held.iter().any(|h| same_fact(h, &fact)) {
            held.push(self.stamped(fact));
        }
        self.vals
            .insert(key, V::List(held.into_iter().map(V::List).collect()));
        Ok(())
    }

    /// A fact as kept: and the turn it reached the base, after its terms.
    fn stamped(&self, mut fact: Fact) -> Fact {
        fact.truncate(4);
        fact.push(V::Num(f64::from(self.chronon)));
        fact
    }

    /// `retract {{ sky rains }}`: the facts that match go; what rules made
    /// of them goes with them.
    pub(super) fn retract_fact(&mut self, fact: &Value, base: &str) -> R<()> {
        let owner = self.owner_of(base);
        let key = self.base_key(base, &owner);
        let pattern = match fact.get("value") {
            Some(v) => {
                let v = self.eval(v)?;
                fact_value_of(&v).ok_or("`retract` needs a fact")?
            }
            None => fact.clone(),
        };
        let held = self.kept(&key);
        let mut left = Vec::new();
        for h in held {
            if self.match_fact(&pattern, &h, &Bindings::new())?.is_none() {
                left.push(h);
            }
        }
        self.vals
            .insert(key, V::List(left.into_iter().map(V::List).collect()));
        Ok(())
    }

    /// A fact worked out to keep: its terms valued, no variables.
    fn fact_of(&mut self, fact: &Value) -> R<Fact> {
        if let Some(v) = fact.get("value") {
            let v = self.eval(v)?;
            return match v {
                V::List(terms) if terms.len() == 4 => Ok(terms),
                other => Err(format!("`{other}` is not a fact to keep")),
            };
        }
        let mut terms = Vec::new();
        for key in ["subject", "verb", "object"] {
            let t = f(fact, key);
            terms.push(match key {
                "verb" => V::Id(t.as_str().unwrap_or("").to_string()),
                _ if *t == Value::Null => V::Null,
                _ if t.get("any").is_some() => {
                    return Err("a fact kept names things; it has no `?`".into())
                }
                // A variable `for each` has bound: what it is now.
                _ => match self.term(t, &Bindings::new())? {
                    V::Null if t.get("var").is_some() => {
                        return Err("a fact kept names things; its variables must be bound".into())
                    }
                    v => v,
                },
            });
        }
        terms.push(match f(fact, "at") {
            Value::Str(at) => V::Str(at.clone()),
            _ => V::Null,
        });
        Ok(terms)
    }

    /// A term's value: a variable's binding, an atom, or what a name in
    /// scope names.
    fn term(&mut self, t: &Value, b: &Bindings) -> R<V> {
        if let Some(v) = t.get("var").and_then(Value::as_str) {
            // Bound here, or by an enclosing `for each P` or quantifier.
            return Ok(match b.get(v) {
                Some(bound) => bound.clone(),
                None => match self.locals.get(v) {
                    Some(Local::Value(bound)) => bound.clone(),
                    _ => V::Null,
                },
            });
        }
        match t {
            // A bare word is itself, but for the words that always mean
            // someone: `you`, `here`, `self`, `it`.
            Value::Str(name) => match name.as_str() {
                "me" | "here" | "self" | "it" => self.eval(t),
                "nothing" | "nobody" => Ok(V::Null),
                _ => Ok(V::Id(name.clone())),
            },
            // `^thug`: what it names.
            other => match other.get("lookup") {
                Some(e) => self.eval(&e.clone()),
                None => self.eval(other),
            },
        }
    }

    // ---------------------------------------------------------- asking

    /// `{{ ? is ashe }} in notebook`: the facts that match, as a list.
    pub(super) fn ask_fact(&mut self, q: &Value) -> R<V> {
        let base = f(q, "in").as_str().unwrap_or("world").to_string();
        let pattern = f(q, "fact").clone();
        let owner = self.owner_of(&base);
        let facts = self.closure(&base, &owner, 0)?;
        let mut out = Vec::new();
        let b = self.bound_vars();
        for fact in facts {
            if self.match_fact(&pattern, &fact, &b)?.is_some() {
                out.push(V::List(fact));
            }
        }
        Ok(V::List(out))
    }

    /// Variables bound by an enclosing quantifier, kept as locals.
    fn bound_vars(&self) -> Bindings {
        self.locals
            .iter()
            .filter(|(k, _)| crate::expr::is_var(k))
            .filter_map(|(k, v)| match v {
                Local::Value(v) => Some((k.clone(), v.clone())),
                Local::Expr(_) => None,
            })
            .collect()
    }

    /// `some P. body`, `no P. body`: whether a value of the variables makes
    /// the body true (or none does).
    pub(super) fn quantified(&mut self, q: &Value) -> R<V> {
        let body = f(q, "body").clone();
        let found = !self.solve(&body, &self.bound_vars(), None)?.is_empty();
        Ok(V::Bool(match f(q, "quant").as_str() {
            Some("no") => !found,
            _ => found,
        }))
    }

    /// Does `fact` fit `pattern`, given `b`? The bindings it adds.
    fn match_fact(&mut self, pattern: &Value, fact: &Fact, b: &Bindings) -> R<Option<Bindings>> {
        let mut b = b.clone();
        for (i, key) in ["subject", "verb", "object", "at"].iter().enumerate() {
            let t = f(pattern, key);
            let have = &fact[i];
            match *key {
                "verb" => {
                    if have.name() != t.as_str() {
                        return Ok(None);
                    }
                }
                // No object asked: only a fact with none fits. No time asked:
                // any time fits.
                "object" if *t == Value::Null => {
                    if *have != V::Null {
                        return Ok(None);
                    }
                }
                "at" if *t == Value::Null => {}
                "at" if t.as_str().is_some() => {
                    if have.to_string() != t.as_str().unwrap_or("") {
                        return Ok(None);
                    }
                }
                _ if t.get("any").is_some() => {}
                _ => match t.get("var").and_then(Value::as_str) {
                    Some(var) => match b.get(var) {
                        Some(bound) if !bound.same(have) => return Ok(None),
                        Some(_) => {}
                        None if *have == V::Null => return Ok(None),
                        None => {
                            b.insert(var.to_string(), have.clone());
                        }
                    },
                    None => {
                        let want = self.term(t, &b)?;
                        if !want.same(have) {
                            return Ok(None);
                        }
                    }
                },
            }
        }
        Ok(Some(b))
    }

    /// Every way of binding the body's variables that makes it true.
    /// `here`: the facts an unqualified fact is asked of (a rule's own
    /// base, as it grows); `None` for the world, as for any question.
    fn solve(&mut self, e: &Value, b: &Bindings, here: Option<&[Fact]>) -> R<Vec<Bindings>> {
        let Value::Obj(pairs) = e else {
            return Ok(if self.with_vars(b, |run| run.eval(e))?.truthy() {
                vec![b.clone()]
            } else {
                Vec::new()
            });
        };
        let op = pairs.first().map(|(k, _)| k.as_str()).unwrap_or("");
        match op {
            "fact" => {
                let facts = match (f(e, "in").as_str(), here) {
                    (None, Some(facts)) => facts.to_vec(),
                    (base, _) => {
                        let base = base.unwrap_or("world").to_string();
                        let owner = self.owner_of(&base);
                        self.closure(&base, &owner, 0)?
                    }
                };
                let pattern = f(e, "fact").clone();
                let mut out = Vec::new();
                for fact in &facts {
                    if let Some(mut nb) = self.match_fact(&pattern, fact, b)? {
                        // Which facts a solution stands on, for a `never`.
                        let mut used = match nb.remove(USED) {
                            Some(V::List(u)) => u,
                            _ => Vec::new(),
                        };
                        used.push(V::List(fact.clone()));
                        nb.insert(USED.to_string(), V::List(used));
                        out.push(nb);
                    }
                }
                Ok(out)
            }
            "and" => {
                let items = f(e, "and").as_arr().unwrap_or(&[]).to_vec();
                let mut all = vec![b.clone()];
                for item in &items {
                    let mut next = Vec::new();
                    for bb in &all {
                        next.extend(self.solve(item, bb, here)?);
                    }
                    all = next;
                    if all.is_empty() {
                        break;
                    }
                }
                Ok(all)
            }
            "or" => {
                let items = f(e, "or").as_arr().unwrap_or(&[]).to_vec();
                let mut out = Vec::new();
                for item in &items {
                    out.extend(self.solve(item, b, here)?);
                }
                Ok(out)
            }
            "not" => {
                let inner = f(e, "not").clone();
                Ok(if self.solve(&inner, b, here)?.is_empty() {
                    vec![b.clone()]
                } else {
                    Vec::new()
                })
            }
            "quant" => {
                let body = f(e, "body").clone();
                let found = !self.solve(&body, b, here)?.is_empty();
                let holds = if f(e, "quant").as_str() == Some("no") {
                    !found
                } else {
                    found
                };
                Ok(if holds { vec![b.clone()] } else { Vec::new() })
            }
            // Anything else is worked out with the variables bound.
            _ => Ok(if self.with_vars(b, |run| run.eval(e))?.truthy() {
                vec![b.clone()]
            } else {
                Vec::new()
            }),
        }
    }

    /// Work something out with the variables bound as names in scope.
    fn with_vars<T>(&mut self, b: &Bindings, work: impl FnOnce(&mut Self) -> R<T>) -> R<T> {
        let saved: Vec<(String, Option<Local>)> = b
            .keys()
            .map(|k| (k.clone(), self.locals.get(k).cloned()))
            .collect();
        for (k, v) in b.iter().filter(|(k, _)| *k != USED) {
            self.locals.insert(k.clone(), Local::Value(v.clone()));
        }
        let result = work(self);
        for (k, old) in saved {
            self.restore_local(&k, old);
        }
        result
    }

    // ----------------------------------------------------------- rules

    /// Everything true in a base for its owner: what it keeps, what it
    /// reads, and what the rules that hold there make of them.
    fn closure(&mut self, base: &str, owner: &str, depth: usize) -> R<Vec<Fact>> {
        if depth > MAX_READS {
            return Err(format!(
                "knowledge bases read each other more than {MAX_READS} deep, through `{base}`"
            ));
        }
        let mut facts = self.kept(&self.base_key(base, owner));
        // What a character knows: what every fact they have learned means.
        if base == "knows" {
            let known: Vec<String> = self
                .knows
                .get(owner)
                .map(|k| k.iter().cloned().collect())
                .unwrap_or_default();
            for id in known {
                let means = self
                    .story
                    .field(&id, "means")
                    .as_arr()
                    .unwrap_or(&[])
                    .to_vec();
                for m in means {
                    let fact = self.fact_of(&m)?;
                    if !facts.iter().any(|h| same_fact(h, &fact)) {
                        facts.push(fact);
                    }
                }
            }
        }
        // `~ reads knows`: and what its owner's other bases hold.
        for read in super::strs(self.story.field(base, "reads")) {
            for fact in self.closure(&read, owner, depth + 1)? {
                if !facts.iter().any(|h| same_fact(h, &fact)) {
                    facts.push(fact);
                }
            }
        }
        let rules = self.rules_in(base, owner);
        for stratum in strata(&rules)? {
            loop {
                let mut grew = false;
                for rule in &stratum {
                    let body = f(rule, "body").clone();
                    let head = f(rule, "head").clone();
                    let found = self.solve(&body, &Bindings::new(), Some(&facts))?;
                    for b in found {
                        let fact = self.head_fact(&head, &b)?;
                        if !facts.iter().any(|h| same_fact(h, &fact)) {
                            facts.push(fact);
                            grew = true;
                        }
                    }
                }
                if !grew {
                    break;
                }
            }
        }
        Ok(facts)
    }

    /// The rules that hold in `base` for `owner`.
    fn rules_in(&self, base: &str, owner: &str) -> Vec<Value> {
        let knows_it = |rule: &Value| {
            f(rule, "known") == &Value::Bool(true)
                || f(rule, "learned_from")
                    .as_str()
                    .is_some_and(|x| self.knows.get(owner).is_some_and(|k| k.contains(x)))
        };
        // A base that reads what its owner knows runs the rules they know.
        let reads_knows = super::strs(self.story.field(base, "reads"))
            .iter()
            .any(|r| r == "knows");
        self.story
            .rules
            .iter()
            .filter(|rule| match f(rule, "in").as_str() {
                // A rule for a base everyone has one of holds once known.
                Some(b) if b == base => {
                    let gated = f(rule, "known") == &Value::Bool(true)
                        || f(rule, "learned_from") != &Value::Null;
                    !gated || base == "world" || knows_it(rule)
                }
                Some(b) => b == "knows" && reads_knows && knows_it(rule),
                None if base == "world" => true,
                None => (base == "knows" || reads_knows) && knows_it(rule),
            })
            .cloned()
            .collect()
    }

    /// The fact a rule makes, its variables bound.
    fn head_fact(&mut self, head: &Value, b: &Bindings) -> R<Fact> {
        let mut terms = Vec::new();
        for key in ["subject", "verb", "object"] {
            let t = f(head, key);
            terms.push(match key {
                "verb" => V::Id(t.as_str().unwrap_or("").to_string()),
                _ if *t == Value::Null => V::Null,
                _ => self.term(t, b)?,
            });
        }
        terms.push(match f(head, "at") {
            Value::Str(at) => V::Str(at.clone()),
            v if v.get("var").is_some() => {
                let var = f(v, "var").as_str().unwrap_or("");
                b.get(var).cloned().unwrap_or(V::Null)
            }
            _ => V::Null,
        });
        Ok(terms)
    }
}

impl Run {
    // ------------------------------------------------- notes and nevers

    /// `note {{ lark is orme }} in notebook`: the Linker writes a fact. A
    /// `never … ~ on contradiction refuse` it would break keeps it out, and
    /// says why; one that `flag`s lets it stand, and the base is
    /// `contradicted` until a note goes.
    pub(super) fn note_fact(&mut self, fact: &Value, base: &str) -> R<()> {
        let new = self.fact_of(fact)?;
        let owner = self.owner_of(base);
        let key = self.base_key(base, &owner);
        let held = self.kept(&key);
        if held.iter().any(|h| same_fact(h, &new)) {
            return Ok(());
        }
        let before = self.broken(base, "refuse")?;
        let mut with = held.clone();
        with.push(self.stamped(new.clone()));
        self.vals.insert(
            key.clone(),
            V::List(with.into_iter().map(V::List).collect()),
        );
        // Refused: it breaks a `never` that held before it was written.
        for (label, facts) in self.broken(base, "refuse")? {
            let new_break = !before.iter().any(|(l, f)| {
                *l == label
                    && f.len() == facts.len()
                    && f.iter().all(|x| facts.iter().any(|y| same_fact(x, y)))
            });
            if new_break {
                self.vals
                    .insert(key, V::List(held.into_iter().map(V::List).collect()));
                let why = if label.is_empty() {
                    String::from(
                        "(You can’t write that: it can’t be true with what you have written.)",
                    )
                } else {
                    format!("(You can’t write that. {label})")
                };
                self.output.push(why.as_str().into());
                return Ok(());
            }
        }
        Ok(())
    }

    /// Every `never` of a base, of this kind (`refuse` or `flag`), that
    /// what it holds breaks: its text, and the facts written there that
    /// break it.
    fn broken(&mut self, base: &str, how: &str) -> R<Vec<(String, Vec<Fact>)>> {
        let story = Arc::clone(&self.story);
        let owner = self.owner_of(base);
        let facts = self.closure(base, &owner, 0)?;
        let kept = self.kept(&self.base_key(base, &owner));
        let mut out: Vec<(String, Vec<Fact>)> = Vec::new();
        for never in story.nevers.iter().filter(|n| {
            f(n, "in").as_str() == Some(base) && f(n, "on_contradiction").as_str() == Some(how)
        }) {
            let label = match f(never, "label") {
                Value::Null => String::new(),
                text => self.text_of(text)?,
            };
            for b in self.solve(f(never, "never"), &Bindings::new(), Some(&facts))? {
                let mut used: Vec<Fact> = match b.get(USED) {
                    Some(V::List(u)) => u
                        .iter()
                        .filter_map(|f| match f {
                            V::List(t) => Some(t.clone()),
                            _ => None,
                        })
                        .filter(|f| kept.iter().any(|k| same_fact(k, f)))
                        .collect(),
                    _ => Vec::new(),
                };
                used.dedup_by(|a, b| same_fact(a, b));
                let seen = out.iter().any(|(l, u)| {
                    *l == label
                        && u.len() == used.len()
                        && u.iter().all(|x| used.iter().any(|y| same_fact(x, y)))
                });
                if !seen {
                    out.push((label.clone(), used));
                }
            }
        }
        Ok(out)
    }

    /// `notebook.contradicted`: a `never ~ on contradiction flag` is broken.
    pub(super) fn contradicted(&mut self, base: &str) -> R<bool> {
        Ok(!self.broken(base, "flag")?.is_empty())
    }

    /// `{contradictions ~ in notebook}`: one line each: why, and the notes.
    pub(super) fn contradiction_lines(&mut self, base: &str) -> R<Vec<String>> {
        let mut lines = Vec::new();
        for (label, facts) in self.broken(base, "flag")? {
            let notes: Vec<String> = facts.iter().map(|f| self.fact_text(f)).collect();
            lines.push(if label.is_empty() {
                notes.join("; ")
            } else {
                format!("{label} {}", notes.join("; "))
            });
        }
        Ok(lines)
    }

    /// `{known about: covers ~ in notebook ~ with times}`: every fact the
    /// base holds about any of them, a line each; with times, when each
    /// reached it.
    pub(super) fn known_about(&mut self, about: &Value, base: &str, times: bool) -> R<Vec<String>> {
        let who: Vec<String> = match self.eval(about)? {
            V::Id(list) if self.lists.contains_key(&list) => {
                self.lists[&list].iter().map(|e| e.id.clone()).collect()
            }
            V::List(items) => items
                .iter()
                .filter_map(|i| i.name().map(str::to_string))
                .collect(),
            one => one.name().map(str::to_string).into_iter().collect(),
        };
        let owner = self.owner_of(base);
        let facts = self.closure(base, &owner, 0)?;
        let mut lines = Vec::new();
        for fact in facts {
            let names = |t: &V| t.name().is_some_and(|n| who.iter().any(|w| w == n));
            if !(names(&fact[0]) || names(&fact[2])) {
                continue;
            }
            let text = self.fact_text(&fact);
            match (times, fact.get(4).and_then(V::num)) {
                (true, Some(turn)) => {
                    let when = self.turn_name(turn as u32);
                    lines.push(format!("{when}: {text}"));
                }
                _ => lines.push(text),
            }
        }
        Ok(lines)
    }

    /// A turn by its innermost period's name (`ten`), or its number.
    fn turn_name(&self, turn: u32) -> String {
        match self.story.calendar.at(turn).last() {
            Some((_, name)) => name.clone(),
            None => format!("turn {turn}"),
        }
    }

    /// A fact as a sentence: "The cloakroom attendant seen at office."
    pub(super) fn fact_text(&self, fact: &Fact) -> String {
        let term = |t: &V| match t {
            V::Id(id) if self.story.kind(id).is_some() => self.label(id),
            V::Id(id) => id.replace('_', " "),
            other => self.show(other),
        };
        let mut words = vec![term(&fact[0]), term(&fact[1])];
        if fact[2] != V::Null {
            words.push(term(&fact[2]));
        }
        if let V::Str(at) = &fact[3] {
            words.push(format!("({})", at.rsplit('.').next().unwrap_or(at)));
        }
        let text = words.join(" ");
        let mut chars = text.chars();
        match chars.next() {
            Some(c) => format!("{}{}.", c.to_uppercase(), chars.as_str()),
            None => text,
        }
    }

    /// `for each P in all(person, in: here): …`: the effect once for each,
    /// `P` bound to it.
    pub(super) fn for_each(&mut self, e: &Value) -> R<()> {
        let var = f(e, "var").as_str().unwrap_or("").to_string();
        let items = match self.eval(f(e, "in"))? {
            V::List(items) => items,
            V::Null => Vec::new(),
            one => vec![one],
        };
        let then = f(e, "then").clone();
        let saved = self.locals.get(&var).cloned();
        for item in items {
            self.locals.insert(var.clone(), Local::Value(item));
            if let Err(err) = self.apply(&then, &[]) {
                self.restore_local(&var, saved);
                return Err(err);
            }
        }
        self.restore_local(&var, saved);
        Ok(())
    }

    /// A Link's parameter: a fact (`{story: {{ you leaving @ … }}}`) is
    /// kept as one, its terms worked out now.
    pub(super) fn param_value(&mut self, v: &Value) -> R<V> {
        match v.get("fact") {
            Some(fact) => Ok(V::List(self.fact_of(&fact.clone())?)),
            None => Ok(self.constant(v)),
        }
    }
}

/// Two facts the same: term for term (not when each was kept).
fn same_fact(a: &Fact, b: &Fact) -> bool {
    a.len() >= 4 && b.len() >= 4 && a.iter().zip(b).take(4).all(|(x, y)| x.same(y))
}

/// A fact held as a value (`story`, a Link's parameter), as a pattern.
fn fact_value_of(v: &V) -> Option<Value> {
    let V::List(terms) = v else { return None };
    let term = |t: &V| match t {
        V::Null => Value::Null,
        other => Value::str(other.to_string()),
    };
    Some(Value::obj([
        ("subject", term(terms.first()?)),
        ("verb", term(terms.get(1)?)),
        ("object", term(terms.get(2)?)),
        ("at", term(terms.get(3)?)),
    ]))
}

/// The verbs a body asks about: those asked plainly, and those under `not`.
fn verbs(e: &Value, negated: bool, out: &mut Vec<(String, bool)>) {
    match e {
        Value::Obj(pairs) => {
            if let Some(v) = e
                .get("fact")
                .and_then(|fact| fact.get("verb"))
                .and_then(Value::as_str)
            {
                out.push((v.to_string(), negated));
                return;
            }
            for (k, v) in pairs {
                verbs(v, negated || k == "not", out);
            }
        }
        Value::Arr(items) => items.iter().for_each(|i| verbs(i, negated, out)),
        _ => {}
    }
}

/// The rules in layers: a rule that asks `not {{ x v … }}` comes after
/// every rule that makes `v`. A loop through `not` has no layering.
fn strata(rules: &[Value]) -> R<Vec<Vec<Value>>> {
    let heads: Vec<String> = rules
        .iter()
        .map(|r| f(f(r, "head"), "verb").as_str().unwrap_or("").to_string())
        .collect();
    let asks: Vec<Vec<(String, bool)>> = rules
        .iter()
        .map(|r| {
            let mut out = Vec::new();
            verbs(f(r, "body"), false, &mut out);
            out
        })
        .collect();
    let mut level = vec![0usize; rules.len()];
    for _ in 0..=rules.len() {
        let mut changed = false;
        for i in 0..rules.len() {
            for (verb, negated) in &asks[i] {
                for j in 0..rules.len() {
                    if heads[j] == *verb {
                        let need = level[j] + usize::from(*negated);
                        if level[i] < need {
                            level[i] = need;
                            changed = true;
                        }
                    }
                }
            }
        }
        if !changed {
            let top = level.iter().copied().max().unwrap_or(0);
            return Ok((0..=top)
                .map(|l| {
                    rules
                        .iter()
                        .zip(&level)
                        .filter(|(_, lv)| **lv == l)
                        .map(|(r, _)| r.clone())
                        .collect()
                })
                .collect());
        }
    }
    Err("rules ask `not` of each other in a loop, so none can go first".into())
}
