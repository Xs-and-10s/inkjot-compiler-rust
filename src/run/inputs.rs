//! The input family and the jury: where a Linker answers rather than picks
//! a Link.
//!
//! ```text
//! {name input for: me.name, max: 24}
//! {choose input for: mouth_words, from: cantrip, length: 3}
//! {accuse who: suspect, how: scenario.methods, proof: proof_who and proof_how, revote: 1}
//! ```
//!
//! An input in a passage's **body** stays open while the Linker is there:
//! they may answer it, and answer it again, between Links. An input in a
//! passage's **header** comes before what follows it, so arriving pauses
//! there until it is answered (Ex 2's mouth must hear the three cantrips
//! before it decides).
//!
//! **The jury** (`{accuse}`) is a secret ballot. Each Linker, alive or a
//! ghost, votes for a suspect and a method and may cite proof: items they
//! hold or clues they know. When everyone has voted the ballots are shown
//! together. The pairing with the most votes wins; a tie is a hung jury,
//! which votes again as often as `revote` allows. Then:
//!
//! | Lookup | Holds when |
//! |---|---|
//! | `accusation.done` | the jury has decided, or stayed hung |
//! | `accusation.hung` | no pairing won, even after the revotes |
//! | `accusation.correct` | the winning pairing is the scenario's culprit and method |
//! | `accusation.proven` | it is correct, and the proof cited by those who voted for it covers `proof:` |

use super::{f, Run, R, V};
use crate::json::Value;

/// Arriving at a passage, paused at a header input.
#[derive(Clone)]
pub(super) struct Paused {
    pub(super) passage: String,
    pub(super) from: String,
    /// The header statement to carry on from.
    pub(super) next_statement: usize,
    /// A `next:` already seen in the header.
    pub(super) next: Option<String>,
}

/// What an open input lets the Linker answer with, for a player or a test
/// that has to make something up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ask {
    /// `name input`, `choose input`, `accuse`.
    pub kind: String,
    /// What the Linker is asked, as the story words it.
    pub prompt: String,
    /// What may be chosen: the `from:` kind's things, or the suspects.
    pub options: Vec<String>,
    /// For the jury: the methods.
    pub methods: Vec<String>,
    /// For a `choose input`: how many to choose.
    pub length: Option<usize>,
    /// For the jury: has the Linker being served voted?
    pub voted: bool,
}

/// One Linker's ballot.
#[derive(Clone, Debug)]
pub(super) struct Ballot {
    who: String,
    how: String,
    proof: Vec<String>,
}

/// Is this statement or body part an input or the jury?
pub(super) fn is_input(v: &Value) -> bool {
    f(v, "insert")
        .as_str()
        .is_some_and(|i| i == "accuse" || i == "ballot" || i.ends_with(" input"))
}

impl Run {
    /// What the Linker being served is asked: one `(kind, prompt)` per open
    /// input, the first of which `answer` and `vote` fill.
    pub fn asking(&mut self) -> Vec<(String, String)> {
        let open = self.asking.clone();
        open.iter()
            .map(|i| {
                let kind = f(i, "insert").as_str().unwrap_or("").to_string();
                let prompt = self.prompt(i).unwrap_or_default();
                (kind, prompt)
            })
            .collect()
    }

    /// Each open input, and what it may be answered with.
    pub fn asks(&mut self) -> R<Vec<Ask>> {
        let open = self.asking.clone();
        let mut out = Vec::new();
        for input in open {
            let args = f(&input, "args").clone();
            let kind = f(&input, "insert").as_str().unwrap_or("").to_string();
            // A jury or ballot that has decided asks nothing more.
            let decided = |key: &str| self.vals.get(key).is_some_and(V::truthy);
            if (kind == "accuse" && decided("accusation.done"))
                || (kind == "ballot" && decided("ballot.done"))
            {
                continue;
            }
            let (options, methods) = match kind.as_str() {
                "choose input" | "ballot" => (self.options(&args)?, Vec::new()),
                "accuse" => (self.suspects(&args)?, self.methods(&args)?),
                _ => (Vec::new(), Vec::new()),
            };
            let prompt = self.prompt(&input)?;
            let voted = if kind == "ballot" {
                self.vals.contains_key(&format!("ballot.vote.{}", self.pov))
            } else {
                self.ballots.contains_key(&self.pov)
            };
            out.push(Ask {
                kind,
                prompt,
                options,
                methods,
                length: f(&args, "length").as_u64().map(|n| n as usize),
                voted,
            });
        }
        Ok(out)
    }

    /// Note an input met while showing a passage, and say what it asks.
    pub(super) fn meet_input(&mut self, input: &Value) -> R<String> {
        // A jury or a ballot met on a later turn is a new one.
        for (insert, key) in [("accuse", "accusation"), ("ballot", "ballot")] {
            if f(input, "insert").as_str() == Some(insert) {
                self.fresh_vote(key);
            }
        }
        // A jury nobody may sit on decides at once.
        if f(input, "insert").as_str() == Some("accuse")
            && !self.vals.get("accusation.done").is_some_and(V::truthy)
        {
            let args = f(input, "args").clone();
            if self.jury(&args)?.is_empty() {
                return self.decide_unvoted(&args, input);
            }
        }
        if !self.asking.contains(input) {
            self.asking.push(input.clone());
        }
        self.prompt(input)
    }

    /// The words that tell a Linker what an input wants.
    fn prompt(&mut self, input: &Value) -> R<String> {
        let args = f(input, "args").clone();
        Ok(match f(input, "insert").as_str().unwrap_or("") {
            "name input" => match f(&args, "max").as_u64() {
                Some(max) => format!("(Type a name, up to {max} letters.)"),
                None => String::from("(Type a name.)"),
            },
            "text input" => match f(&args, "max").as_u64() {
                Some(max) => format!("(Write it, up to {max} characters.)"),
                None => String::from("(Write it.)"),
            },
            "ballot" => {
                if self.vals.get("ballot.done").is_some_and(V::truthy) {
                    return Ok(String::new());
                }
                let options: Vec<String> = self
                    .options(&args)?
                    .iter()
                    .map(|o| self.option_label(&args, o))
                    .collect();
                let mine = if self.vals.contains_key(&format!("ballot.vote.{}", self.pov)) {
                    " Your vote is cast."
                } else {
                    ""
                };
                format!("(Vote for one of: {}.{mine})", or_list(&options))
            }
            "choose input" => {
                let options = self.options(&args)?;
                match f(&args, "length").as_u64() {
                    Some(n) => format!("(Choose {n}, in order, from: {}.)", options.join(", ")),
                    None => format!("(Choose from: {}.)", options.join(", ")),
                }
            }
            "accuse" => {
                if self.vals.get("accusation.done").is_some_and(V::truthy) {
                    return Ok(String::new());
                }
                let who: Vec<String> = self
                    .suspects(&args)?
                    .iter()
                    .map(|s| self.label(s))
                    .collect();
                let how = self.methods(&args)?;
                let mine = if self.ballots.contains_key(&self.pov) {
                    " Your ballot is cast."
                } else {
                    ""
                };
                if how.is_empty() {
                    format!("(Vote for one of: {}.{mine})", or_list(&who))
                } else {
                    format!(
                        "(Vote: who, of {}; how, of {}; and cite your proof.{mine})",
                        or_list(&who),
                        or_list(&how)
                    )
                }
            }
            other => {
                return Err(format!(
                    "`{{{other}}}` is not supported by the interpreter yet"
                ))
            }
        })
    }

    /// What a `{choose input}` chooses from: the entries of a list
    /// (`from: moderators`), or everything of a kind (`from: cantrip`).
    fn options(&self, args: &Value) -> R<Vec<String>> {
        let kind = f(args, "from").as_str().unwrap_or("");
        if self.story.kind(kind) == Some("list") {
            return Ok(self
                .lists
                .get(kind)
                .map(|l| l.iter().map(|e| e.id.clone()).collect())
                .unwrap_or_default());
        }
        let mut ids: Vec<String> = self
            .story
            .decls
            .keys()
            .filter(|id| self.is_kind(id, kind))
            .cloned()
            .collect();
        ids.sort();
        Ok(ids)
    }

    /// Who the jury may accuse: those of a kind (`who: suspect`), or a
    /// list worked out now (`who: all(guest, where: it.alive)`).
    fn suspects(&mut self, args: &Value) -> R<Vec<String>> {
        if let who @ Value::Obj(_) = f(args, "who") {
            return Ok(match self.eval(&who.clone())? {
                V::List(items) => items
                    .iter()
                    .filter_map(|i| i.name().map(str::to_string))
                    .collect(),
                one => one.name().map(str::to_string).into_iter().collect(),
            });
        }
        let kind = f(args, "who").as_str().unwrap_or("suspect");
        let mut ids: Vec<String> = self
            .story
            .decls
            .keys()
            .filter(|id| self.is_kind(id, kind))
            .cloned()
            .collect();
        ids.sort();
        Ok(ids)
    }

    fn methods(&mut self, args: &Value) -> R<Vec<String>> {
        let how = f(args, "how").clone();
        if how == Value::Null {
            return Ok(Vec::new());
        }
        Ok(match self.eval(&how)? {
            V::List(items) => items
                .iter()
                .filter_map(|v| v.name().map(str::to_string))
                .collect(),
            V::Id(one) | V::Str(one) => vec![one],
            _ => Vec::new(),
        })
    }

    /// Answer the first open input (not the jury): a name, or the words
    /// chosen.
    pub fn answer(&mut self, words: &[String]) -> R<()> {
        self.begin_action();
        let input = self
            .asking
            .iter()
            .find(|i| f(i, "insert").as_str() != Some("accuse"))
            .cloned()
            .ok_or("nothing here is asking for an answer")?;
        let args = f(&input, "args").clone();
        if f(&input, "insert").as_str() == Some("ballot") {
            return self.ballot_vote(&args, words);
        }
        let value = match f(&input, "insert").as_str().unwrap_or("") {
            "name input" => {
                let name = words.join(" ");
                let name = name.trim();
                if name.is_empty() {
                    return Err("a name cannot be empty".into());
                }
                if let Some(max) = f(&args, "max").as_u64() {
                    if name.chars().count() > max as usize {
                        return Err(format!("a name here is at most {max} letters"));
                    }
                }
                V::Str(name.to_string())
            }
            // `{text input for: report, max: 600}`: anything, within the limit.
            "text input" => {
                let text = words.join(" ");
                if let Some(max) = f(&args, "max").as_u64() {
                    if text.chars().count() > max as usize {
                        return Err(format!("this is at most {max} characters"));
                    }
                }
                V::Str(text)
            }
            "choose input" => {
                let options = self.options(&args)?;
                if let Some(bad) = words.iter().find(|w| !options.contains(w)) {
                    return Err(format!(
                        "`{bad}` is not one of the choices: {}",
                        options.join(", ")
                    ));
                }
                if let Some(n) = f(&args, "length").as_u64() {
                    if words.len() != n as usize {
                        return Err(format!("choose exactly {n}"));
                    }
                }
                // `length: 1` into a single value: `moderator`, not a list.
                if f(&args, "length").as_u64() == Some(1) {
                    V::Id(words[0].clone())
                } else {
                    V::List(words.iter().map(|w| V::Id(w.clone())).collect())
                }
            }
            other => {
                return Err(format!(
                    "`{{{other}}}` is not supported by the interpreter yet"
                ))
            }
        };
        // Keep the answer where `for:` says: `me.name`, `mouth_words`.
        let key = self.path(f(&args, "for"))?;
        match self.locals.get(&key) {
            Some(_) => {
                self.locals.insert(key, super::Local::Value(value));
            }
            None => {
                self.vals.insert(key, value);
            }
        }
        // A header input lets the arrival carry on; a body input just
        // changes what the passage offers.
        match self.paused.take() {
            Some(paused) => {
                self.asking.clear();
                self.resume_arrival(paused)
            }
            None => self.relist(),
        }
    }

    /// Cast the served Linker's ballot. When everyone has, the jury decides.
    pub fn vote(&mut self, who: &str, how: &str, proof: &[String]) -> R<()> {
        self.begin_action();
        let jury = self
            .asking
            .iter()
            .find(|i| f(i, "insert").as_str() == Some("accuse"))
            .cloned()
            .ok_or("there is no jury here")?;
        if self.vals.get("accusation.done").is_some_and(V::truthy) {
            return Err("the jury has already decided".into());
        }
        let args = f(&jury, "args").clone();
        let suspects = self.suspects(&args)?;
        // `voters: all(guest, where: it.alive)`: only they vote.
        let voters = self.jury(&args)?;
        if !voters.contains(&self.pov) {
            return Err(format!("`{}` has no vote here", self.pov));
        }
        if !suspects.iter().any(|s| s == who) {
            return Err(format!("`{who}` is not a suspect: {}", suspects.join(", ")));
        }
        let methods = self.methods(&args)?;
        if !methods.is_empty() && !methods.iter().any(|m| m == how) {
            return Err(format!(
                "`{how}` is not one of the methods: {}",
                methods.join(", ")
            ));
        }
        // You can only cite what you hold or know.
        let me = self.pov.clone();
        for item in proof {
            let held = self.holder.get(item).is_some_and(|h| *h == me);
            let known = self.knows.get(&me).is_some_and(|k| k.contains(item));
            if !held && !known {
                return Err(format!(
                    "you can cite only what you hold or know, and not `{item}`"
                ));
            }
        }
        // Remembered for `it.voted_with(…)`, until the next jury.
        self.vals
            .insert(format!("{me}.voted_for"), V::Id(who.to_string()));
        self.ballots.insert(
            me,
            Ballot {
                who: who.to_string(),
                how: how.to_string(),
                proof: proof.to_vec(),
            },
        );
        if voters.iter().all(|v| self.ballots.contains_key(v)) {
            self.tally(&args)
        } else {
            self.output
                .push("(Your ballot is cast. Waiting for the others.)".into());
            self.relist()
        }
    }

    /// A jury with no voters: the tally of nothing, drawn among everyone.
    fn decide_unvoted(&mut self, args: &Value, input: &Value) -> R<String> {
        if !self.asking.contains(input) {
            self.asking.push(input.clone());
        }
        let voters = Vec::new();
        self.tally_with(args, &voters)?;
        self.prompt(input)
    }

    /// Who sits on this jury or votes in this ballot: `voters:` worked out
    /// now, or every Linker.
    fn jury(&mut self, args: &Value) -> R<Vec<String>> {
        match f(args, "voters") {
            Value::Null => Ok(self.voters()),
            v => Ok(match self.eval(&v.clone())? {
                V::List(items) => items
                    .iter()
                    .filter_map(|i| i.name().map(str::to_string))
                    .collect(),
                one => one.name().map(str::to_string).into_iter().collect(),
            }),
        }
    }

    /// A new jury or ballot, met on a later turn than the last: forget the
    /// last one's votes and verdict.
    fn fresh_vote(&mut self, key: &str) {
        let turn = V::Num(f64::from(self.chronon));
        if self.vals.get(&format!("{key}.turn")) == Some(&turn) {
            return;
        }
        let prefix = format!("{key}.");
        self.vals.retain(|k, _| !k.starts_with(&prefix));
        if key == "accusation" {
            self.ballots.clear();
            self.vals.retain(|k, _| !k.ends_with(".voted_for"));
        }
        self.vals.insert(format!("{key}.turn"), turn);
    }

    /// The text a ballot shows for one of its options: the list's label.
    fn option_label(&self, args: &Value, option: &str) -> String {
        let list = f(args, "from").as_str().unwrap_or("");
        self.lists
            .get(list)
            .and_then(|l| l.iter().find(|e| e.id == option))
            .and_then(|e| e.label.as_ref())
            .and_then(|l| self.text_now(l))
            .unwrap_or_else(|| self.label(option))
    }

    /// `{ballot …}`: the pov votes for one option. When every voter has,
    /// the most chosen is kept in `for:`.
    fn ballot_vote(&mut self, args: &Value, words: &[String]) -> R<()> {
        if self.vals.get("ballot.done").is_some_and(V::truthy) {
            return Err("the ballot is over".into());
        }
        let voters = self.jury(args)?;
        if !voters.contains(&self.pov) {
            return Err(format!("`{}` has no vote in this ballot", self.pov));
        }
        let options = self.options(args)?;
        let choice = words.join(" ");
        if !options.contains(&choice) {
            return Err(format!("`{choice}` is not one of: {}", options.join(", ")));
        }
        self.vals
            .insert(format!("ballot.vote.{}", self.pov), V::Id(choice));
        if !voters
            .iter()
            .all(|v| self.vals.contains_key(&format!("ballot.vote.{v}")))
        {
            self.output
                .push("(Your vote is cast. Waiting for the others.)".into());
            return self.relist();
        }
        // Count, in the order first voted for (seating order).
        let mut counts: Vec<(String, usize)> = Vec::new();
        for v in &voters {
            if let Some(c) = self.vals.get(&format!("ballot.vote.{v}")).and_then(V::name) {
                match counts.iter_mut().find(|(o, _)| o == c) {
                    Some((_, n)) => *n += 1,
                    None => counts.push((c.to_string(), 1)),
                }
            }
        }
        let top = counts.iter().map(|(_, n)| *n).max().unwrap_or(0);
        let leaders: Vec<String> = counts
            .iter()
            .filter(|(_, n)| *n == top)
            .map(|(o, _)| o.clone())
            .collect();
        let drawn = leaders.len() > 1;
        let winner = if drawn && f(args, "ties").as_str() != Some("hung") {
            let i = self.roll(leaders.len() as u64) as usize - 1;
            Some(leaders[i].clone())
        } else if drawn {
            None
        } else {
            Some(leaders[0].clone())
        };
        self.vals.insert("ballot.done".into(), V::Bool(true));
        self.vals
            .insert("ballot.drawn".into(), V::Bool(drawn && winner.is_some()));
        self.vals
            .insert("ballot.hung".into(), V::Bool(winner.is_none()));
        if let Some(w) = winner {
            let key = self.path(f(args, "for"))?;
            self.vals.insert(key, V::Id(w.clone()));
            self.vals.insert("ballot.winner".into(), V::Id(w));
        }
        let lines = Vec::new();
        self.tell_jury(&voters, lines)
    }

    /// How many Linkers are playing, or how many of them are alive.
    pub(super) fn voters_count(&self, alive: bool) -> f64 {
        self.voters()
            .iter()
            .filter(|v| !alive || !self.vals.get(&format!("{v}.dead")).is_some_and(V::truthy))
            .count() as f64
    }

    /// Everyone on the jury: every Linker, the dead too (as ghosts).
    fn voters(&self) -> Vec<String> {
        if self.order.is_empty() {
            vec![self.pov.clone()]
        } else {
            self.order.clone()
        }
    }

    /// Count the ballots, show them to everyone, and decide or revote.
    fn tally(&mut self, args: &Value) -> R<()> {
        let voters = self.jury(args)?;
        self.tally_with(args, &voters)
    }

    fn tally_with(&mut self, args: &Value, voters: &[String]) -> R<()> {
        let voters = voters.to_vec();
        self.vals.insert(
            "accusation.voters".into(),
            V::List(voters.iter().cloned().map(V::Id).collect()),
        );
        let ballots: Vec<Ballot> = voters
            .iter()
            .filter_map(|v| self.ballots.get(v).cloned())
            .collect();
        // Count each pairing, in the order first voted for.
        let mut counts: Vec<((String, String), usize)> = Vec::new();
        for b in &ballots {
            let pair = (b.who.clone(), b.how.clone());
            match counts.iter_mut().find(|(p, _)| *p == pair) {
                Some((_, n)) => *n += 1,
                None => counts.push((pair, 1)),
            }
        }
        // Nobody may vote (Ex 7: every voter paid a debt): every suspect
        // ties, with nothing.
        let unvoted: Vec<(String, String)> = if voters.is_empty() {
            self.suspects(args)?
                .into_iter()
                .map(|s| (s, String::new()))
                .collect()
        } else {
            Vec::new()
        };
        let top = counts.iter().map(|(_, n)| *n).max().unwrap_or(0);
        let leaders: Vec<&(String, String)> = if voters.is_empty() {
            unvoted.iter().collect()
        } else {
            counts
                .iter()
                .filter(|(_, n)| *n == top)
                .map(|(p, _)| p)
                .collect()
        };
        // Kept for `{ballots}`: most votes first, ties in seating order.
        let mut sorted = counts.clone();
        sorted.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        self.tally = sorted
            .into_iter()
            .map(|((who, how), n)| (who, how, n))
            .collect();
        // An Inklink that shows `{ballots}` itself frames the tally; one that
        // does not gets a plain line from the interpreter.
        let framed = self.story.shows_ballots;
        let mut lines = Vec::new();
        if !framed {
            lines.push(format!(
                "The ballots are revealed: {}.",
                self.ballots_text()
            ));
        }

        let revotes = self
            .vals
            .get("accusation.revotes")
            .and_then(V::num)
            .unwrap_or_else(|| f(args, "revote").as_u64().unwrap_or(0) as f64);
        if leaders.len() != 1 && revotes > 0.0 && !voters.is_empty() {
            self.vals
                .insert("accusation.revotes".into(), V::Num(revotes - 1.0));
            self.ballots.clear();
            lines.push("(The jury is hung. Vote again.)".into());
            return self.tell_jury(&voters, lines);
        }
        self.vals.insert("accusation.done".into(), V::Bool(true));
        // `ties: draw`: by lot, among the tied, once the revotes are spent.
        let mut leaders = leaders;
        let drawn = leaders.len() > 1 && f(args, "ties").as_str() == Some("draw");
        if drawn {
            let i = self.roll(leaders.len() as u64) as usize - 1;
            leaders = vec![leaders[i]];
        }
        self.vals.insert("accusation.drawn".into(), V::Bool(drawn));
        if leaders.len() != 1 {
            self.vals.insert("accusation.hung".into(), V::Bool(true));
            self.vals
                .insert("accusation.correct".into(), V::Bool(false));
            self.vals.insert("accusation.proven".into(), V::Bool(false));
            if !framed {
                lines.push("The jury is hung.".into());
            }
            return self.tell_jury(&voters, lines);
        }
        let (who, how) = leaders[0].clone();
        let scenario = self
            .vals
            .get("scenario")
            .and_then(|s| s.name().map(str::to_string));
        let field = |run: &Self, key: &str| {
            scenario
                .as_deref()
                .and_then(|s| run.story.field(s, key).as_str().map(str::to_string))
        };
        let correct = field(self, "culprit").as_deref() == Some(who.as_str())
            && field(self, "method").as_deref().is_none_or(|m| m == how);
        // The proof laid down by those who voted for the winning pairing.
        let cited: Vec<String> = ballots
            .iter()
            .filter(|b| b.who == who && b.how == how)
            .flat_map(|b| b.proof.clone())
            .collect();
        let proven = correct && self.covers(f(args, "proof"), &cited);
        self.vals.insert("accusation.hung".into(), V::Bool(false));
        self.vals
            .insert("accusation.who".into(), V::Id(who.clone()));
        self.vals
            .insert("accusation.how".into(), V::Id(how.clone()));
        self.vals
            .insert("accusation.correct".into(), V::Bool(correct));
        self.vals
            .insert("accusation.proven".into(), V::Bool(proven));
        if !framed {
            lines.push(if how.is_empty() {
                format!("The jury accuses {}.", self.label(&who))
            } else {
                format!("The jury accuses {}, by {how}.", self.label(&who))
            });
        }
        self.tell_jury(&voters, lines)
    }

    /// `{ballots}`: the last count, most votes first. "2 for Miss Edith
    /// Vell, by device; 1 for Mr. Crane, by poison".
    pub(super) fn ballots_text(&self) -> String {
        self.tally
            .iter()
            .map(|(who, how, n)| {
                if how.is_empty() {
                    format!("{n} for {}", self.label(who))
                } else {
                    format!("{n} for {}, by {how}", self.label(who))
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Does the proof cited cover what `proof:` asks? A tag needs something
    /// cited that has it; `and`, `or` and `not` combine.
    fn covers(&self, need: &Value, cited: &[String]) -> bool {
        match need {
            Value::Null => true,
            Value::Str(tag) => cited.iter().any(|c| self.tagged(c, tag)),
            Value::Obj(_) => {
                let parts = |key| f(need, key).as_arr().unwrap_or(&[]).to_vec();
                if need.get("and").is_some() {
                    parts("and").iter().all(|p| self.covers(p, cited))
                } else if need.get("or").is_some() {
                    parts("or").iter().any(|p| self.covers(p, cited))
                } else if let Some(inner) = need.get("not") {
                    !self.covers(inner, cited)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Show the jury's lines to every voter, and let the passage update.
    fn tell_jury(&mut self, voters: &[String], lines: Vec<String>) -> R<()> {
        let served = self.pov.clone();
        for v in voters {
            self.sit(v)?;
            self.output.extend(lines.iter().map(|l| l.as_str().into()));
            self.relist()?;
        }
        self.sit(&served)
    }

    /// Carry on arriving after a header input was answered.
    pub(super) fn resume_arrival(&mut self, p: Paused) -> R<()> {
        self.arrive(&p.passage, &p.from, p.next_statement, p.next)
    }

    /// `scenario`: the scenario in play. With several, one is picked by
    /// the run's dice, so the culprit changes from run to run.
    pub(super) fn pick_scenario(&mut self) {
        let all = self.story.ids_of("scenario");
        if all.is_empty() {
            return;
        }
        let i = if all.len() == 1 {
            0
        } else {
            self.roll(all.len() as u64) as usize - 1
        };
        self.vals.insert("scenario".into(), V::Id(all[i].clone()));
    }
}

/// `a, b or c`.
fn or_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}
