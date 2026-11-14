//! Typed fields for declarations and settings.
//!
//! A declaration's `~` clauses are short phrases: `in dining`, `held by wren`,
//! `shown as "minutes"`, `asleep during 2..=4`. This module turns each phrase
//! into a named JSON field, so a runtime reads `"held_by": "wren"` and never
//! has to know that the phrase was "held by".
//!
//! The vocabulary is one table, [`PATTERNS`]. A phrase that matches nothing
//! becomes a tag if it is one word (`evidence`, `playable`), and otherwise
//! goes to `other`, so nothing is ever silently dropped.

use std::collections::HashSet;

use super::{cost, effect, expression, quoted, token, word, words};
use crate::json::Value;
use crate::syntax::{is_ident, top_mask};

/// How the rest of a phrase, after its leading words, becomes a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// No value: the phrase itself means `true`. `ends turn`.
    Flag,
    /// One id: `in dining`.
    Id,
    /// A comma-separated list of ids: `knows debts`, `for npc`.
    Ids,
    /// One quoted string: `shown as "minutes"`.
    Text,
    /// An expression: `open when grate_up`, `asleep during 2..=4`.
    Expr,
    /// A single token, a number if it reads as one: `max 3`, `weight heavy`.
    Token,
    /// Each word lowered on its own: `requires pov.all here`.
    Words,
    /// An effect: `then set vell.cornered: yes`.
    Effect,
    /// A cost: `cost 2 when me.is(rook)`.
    Cost,
    /// `name: value`, collected into an object: `fact rusty: no`.
    Pair,
    /// `moves walk, dash`, `moves none`, `moves 2 per chronon`.
    Moves,
    /// `aka "a", "b"`, or an order: `aka shuffled`.
    Aka,
    /// `holds a, b: "label", c as mark`: list entries.
    Holds,
    /// `oubliette for roland, chapel for eldric`, or a single word.
    PerWho,
}

/// One phrase of the vocabulary: its leading words, the field it fills, and
/// how the rest of the phrase is read.
struct Pattern {
    words: &'static [&'static str],
    key: &'static str,
    shape: Shape,
}

const fn p(words: &'static [&'static str], key: &'static str, shape: Shape) -> Pattern {
    Pattern { words, key, shape }
}

/// The phrases Inkjot v0.5 uses on declarations and settings.
///
/// When several patterns match, the one with the most leading words wins, so
/// `watches here when …` beats `watches here`, and `refill on` beats `on`.
const PATTERNS: &[Pattern] = &[
    // Where things are and who has them.
    p(&["in"], "in", Shape::Id),
    p(&["at"], "at", Shape::Id),
    p(&["held", "by"], "held_by", Shape::Id),
    p(&["holds"], "holds", Shape::Holds),
    p(&["belongs", "to"], "belongs_to", Shape::Id),
    p(&["shared", "by"], "shared_by", Shape::Ids),
    p(&["knows"], "knows", Shape::Ids),
    p(&["for"], "for", Shape::Ids),
    p(&["one", "way"], "one_way", Shape::Flag),
    // What a thing is.
    p(&["like"], "like", Shape::Id),
    p(&["with"], "with", Shape::Pair),
    p(&["fact"], "facts", Shape::Pair),
    p(&["shown", "as"], "shown_as", Shape::Text),
    p(&["formally"], "formally", Shape::Text),
    p(&["aka"], "aka", Shape::Aka),
    p(&["else"], "else", Shape::Token),
    p(&["weight"], "weight", Shape::Token),
    p(&["of"], "of", Shape::Token),
    p(&["survives"], "survives", Shape::Token),
    // Conditions.
    p(&["when"], "when", Shape::Expr),
    p(&["hidden", "until"], "hidden_until", Shape::Expr),
    p(&["fixed", "until"], "fixed_until", Shape::Expr),
    p(&["offstage", "until"], "offstage_until", Shape::Expr),
    p(&["open", "when"], "open_when", Shape::Expr),
    p(&["done", "when"], "done_when", Shape::Expr),
    p(&["watches", "here"], "watches", Shape::Flag),
    p(&["watches", "here", "when"], "watches_when", Shape::Expr),
    p(&["then"], "then", Shape::Effect),
    // Acts.
    p(&["cost"], "cost", Shape::Cost),
    p(&["check"], "check", Shape::Expr),
    p(&["min"], "min", Shape::Token),
    p(&["steps"], "steps", Shape::Expr),
    p(&["capacity"], "capacity", Shape::Token),
    p(&["requires"], "requires", Shape::Words),
    p(&["ends", "turn"], "ends_turn", Shape::Flag),
    p(&["learned", "from", "clue"], "learned_from", Shape::Id),
    p(&["once", "per"], "once_per", Shape::Token),
    p(&["choice"], "choice", Shape::Token),
    // Actors.
    p(&["moves"], "moves", Shape::Moves),
    p(&["asleep", "during"], "asleep_during", Shape::Expr),
    p(&["wakes", "on"], "wakes_on", Shape::Token),
    p(&["stays", "awake"], "stays_awake", Shape::Token),
    p(&["toward"], "toward", Shape::Expr),
    p(&["via"], "via", Shape::Token),
    p(&["pace"], "pace", Shape::Token),
    // Resources.
    p(&["max"], "max", Shape::Token),
    p(&["per"], "per", Shape::Token),
    p(&["refill", "each"], "refill", Shape::Token),
    p(&["refill", "on"], "refill", Shape::Token),
    // Objectives and scenarios.
    p(&["part", "of"], "part_of", Shape::Id),
    p(&["after"], "after", Shape::Id),
    p(&["culprit"], "culprit", Shape::Id),
    p(&["method"], "method", Shape::Id),
    p(&["methods"], "methods", Shape::Ids),
    // The clock, the pov, and reveal.
    p(&["best"], "best", Shape::Expr),
    p(&["advance"], "advance", Shape::Token),
    p(&["turn", "limit"], "turn_limit", Shape::Token),
    p(&["turn", "ends", "when"], "turn_ends_when", Shape::Expr),
    p(&["switch"], "switch", Shape::Token),
    p(&["lead"], "lead", Shape::Id),
    p(
        &["order", "each", "other", "within"],
        "order_within",
        Shape::Token,
    ),
    p(&["on", "tap"], "on_tap", Shape::Flag),
    p(&["timed"], "timed", Shape::Token),
    p(&["beat"], "beat", Shape::Token),
    p(&["shows"], "shows", Shape::Token),
    // Rewinds and checkpoints.
    p(&["on", "death", "in"], "on_death_in", Shape::Id),
    p(&["keeps"], "keeps", Shape::Ids),
    p(&["resets"], "resets", Shape::Ids),
    p(&["to"], "to", Shape::PerWho),
    p(&["unseen", "by"], "unseen_by", Shape::Id),
];

/// Split a phrase into words at top-level spaces, so a quoted string or a
/// bracketed call stays whole: `aka "Le Le Le Leo", "Lapkins"` is three words.
fn split(text: &str) -> Vec<&str> {
    let mask = top_mask(text);
    let mut out = Vec::new();
    let mut start = None;
    for (i, ch) in text.char_indices() {
        let breaks = ch.is_whitespace() && mask[i];
        match (start, breaks) {
            (None, false) => start = Some(i),
            (Some(s), true) => {
                out.push(&text[s..i]);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push(&text[s..]);
    }
    out
}

/// The longest pattern whose words begin `phrase`.
fn find(phrase: &[&str]) -> Option<&'static Pattern> {
    PATTERNS
        .iter()
        .filter(|pat| {
            pat.words.len() <= phrase.len() && pat.words.iter().zip(phrase).all(|(a, b)| a == b)
        })
        .max_by_key(|pat| pat.words.len())
}

/// Comma-separated ids: `walk, dash` → `["walk", "dash"]`.
fn ids(text: &str) -> Value {
    Value::Arr(
        text.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(Value::str)
            .collect(),
    )
}

/// Comma-separated quoted strings: `"a", "b"` → `["a", "b"]`.
fn texts(text: &str) -> Value {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(s) = quoted(rest) {
        out.push(Value::str(s));
        let end = rest.find(s).map_or(rest.len(), |i| i + s.len() + 1);
        rest = &rest[end.min(rest.len())..];
    }
    Value::Arr(out)
}

/// The fields of one statement, collected in order.
///
/// Lists extend and objects merge when a key repeats, so `~ knows a` then
/// `~ knows b` gives `"knows": ["a", "b"]`.
///
/// `stats` names the Inklink's declared stats and resources, so that
/// `wits 4` or `hp 2` on a character lands in `"stats": {"wits": 4}`.
#[derive(Debug)]
pub(super) struct Fields<'a> {
    pairs: Vec<(String, Value)>,
    stats: &'a HashSet<String>,
}

impl<'a> Fields<'a> {
    pub(super) fn new(stats: &'a HashSet<String>) -> Fields<'a> {
        Fields {
            pairs: Vec::new(),
            stats,
        }
    }

    /// Set `key`, or extend it if it is already a list or an object.
    pub(super) fn add(&mut self, key: &str, value: Value) {
        let Some(slot) = self.pairs.iter_mut().find(|(k, _)| k == key) else {
            self.pairs.push((key.to_string(), value));
            return;
        };
        match (&mut slot.1, value) {
            (Value::Arr(have), Value::Arr(more)) => have.extend(more),
            (Value::Obj(have), Value::Obj(more)) => have.extend(more),
            (Value::Arr(have), one) => have.push(one),
            (have, one) => *have = Value::Arr(vec![have.clone(), one]),
        }
    }

    /// Read one phrase, the text of a `~` clause, into fields.
    pub(super) fn phrase(&mut self, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        if text.starts_with('"') {
            // The first quoted phrase is a label; a later one is a description.
            let key = if self.pairs.iter().any(|(k, _)| k == "label") {
                "description"
            } else {
                "label"
            };
            self.add(key, Value::str(quoted(text).unwrap_or(text)));
            return;
        }
        // A map row: `orrery | gallery | gate`.
        if text.contains('|') {
            let row = text.split('|').map(|c| Value::str(c.trim())).collect();
            self.add("rows", Value::Arr(vec![Value::Arr(row)]));
            return;
        }
        // A table row, `1..=2: then teach blocker the_attack`, or a bare
        // range, `0..=3`.
        if text.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
            match text.split_once(": ") {
                Some((range, row)) => {
                    let row = row.trim();
                    let then = row.strip_prefix("then ").map_or(Value::Null, effect);
                    let entry = Value::obj([("range", expression(range)), ("then", then)]);
                    self.add("rows", Value::Arr(vec![entry]));
                }
                None => self.add("range", expression(text)),
            }
            return;
        }
        let parts = split(text);
        // A stat or resource with its value: `wits 4`, `hp 2`, `wits ..=2`.
        if let [name, value] = parts.as_slice() {
            if self.stats.contains(*name) {
                self.add(
                    "stats",
                    Value::Obj(vec![(name.to_string(), expression(value))]),
                );
                return;
            }
        }
        let Some(pat) = find(&parts) else {
            if is_ident(text) {
                self.add("tags", Value::Arr(vec![Value::str(text)]));
            } else {
                let other = Value::obj([
                    ("text", Value::str(text)),
                    ("words", Value::Arr(words(text))),
                ]);
                self.add("other", Value::Arr(vec![other]));
            }
            return;
        };
        // The text after the pattern's leading words.
        let lead: usize = pat.words.iter().map(|w| w.len() + 1).sum();
        let rest = text.get(lead.min(text.len())..).unwrap_or("").trim();
        let value = match pat.shape {
            Shape::Flag => Value::Bool(true),
            Shape::Id => Value::str(rest),
            Shape::Ids => ids(rest),
            Shape::Text => Value::str(quoted(rest).unwrap_or(rest)),
            Shape::Expr => expression(rest),
            Shape::Token => token(rest),
            Shape::Words => Value::Arr(words(rest)),
            Shape::Effect => Value::Arr(vec![effect(rest)]),
            Shape::Cost => Value::Arr(vec![cost(rest)]),
            Shape::Pair => {
                let (name, value) = rest.split_once(':').unwrap_or((rest, "yes"));
                Value::Obj(vec![(name.trim().to_string(), word(value.trim()))])
            }
            Shape::Moves => match rest.split_whitespace().collect::<Vec<_>>().as_slice() {
                ["none"] => Value::Arr(Vec::new()),
                [n, "per", "chronon"] => {
                    self.add("speed", token(n));
                    return;
                }
                _ => ids(rest),
            },
            Shape::Aka => {
                if rest.starts_with('"') {
                    texts(rest)
                } else {
                    self.add("aka_order", Value::str(rest));
                    return;
                }
            }
            Shape::Holds => {
                let mut entries = Vec::new();
                for item in rest.split(',') {
                    let item = item.trim();
                    let (item, mark) = match item.split_once(" as ") {
                        Some((i, m)) => (i.trim(), Some(m.trim())),
                        None => (item, None),
                    };
                    let (id, label) = match item.split_once(':') {
                        Some((i, l)) => (i.trim(), quoted(l)),
                        None => (item, None),
                    };
                    let mut entry = vec![("id".to_string(), Value::str(id))];
                    if let Some(label) = label {
                        entry.push(("label".to_string(), Value::str(label)));
                    }
                    if let Some(mark) = mark {
                        entry.push(("mark".to_string(), Value::str(mark)));
                    }
                    entries.push(Value::Obj(entry));
                }
                Value::Arr(entries)
            }
            Shape::PerWho => per_who(rest),
        };
        self.add(pat.key, value);
    }

    pub(super) fn into_pairs(self) -> Vec<(String, Value)> {
        self.pairs
    }
}

/// `oubliette for roland, chapel for eldric` → an object keyed by who;
/// a single word → that word; anything else → the text.
pub(super) fn per_who(text: &str) -> Value {
    if text.contains(" for ") {
        let pairs = text
            .split(',')
            .filter_map(|part| part.split_once(" for "))
            .map(|(what, who)| (who.trim().to_string(), Value::str(what.trim())))
            .collect();
        return Value::Obj(pairs);
    }
    Value::str(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fields one phrase produces.
    fn read(phrase: &str) -> Value {
        let stats = HashSet::from(["wits".to_string()]);
        let mut f = Fields::new(&stats);
        f.phrase(phrase);
        Value::Obj(f.into_pairs())
    }

    /// JSON text as a value, to compare with what `read` gives.
    fn json(text: &str) -> Value {
        crate::json::parse(text).unwrap()
    }

    #[test]
    fn common_phrases_become_named_fields() {
        assert_eq!(read("held by wren"), json(r#"{"held_by": "wren"}"#));
        assert_eq!(
            read("shown as \"minutes\""),
            json(r#"{"shown_as": "minutes"}"#)
        );
        assert_eq!(
            read("asleep during 2..=4"),
            json(r#"{"asleep_during": {"range": [2, 4], "inclusive": true}}"#)
        );
        assert_eq!(read("moves none"), json(r#"{"moves": []}"#));
        assert_eq!(read("moves 2 per chronon"), json(r#"{"speed": 2}"#));
        assert_eq!(read("knows debts"), json(r#"{"knows": ["debts"]}"#));
        assert_eq!(
            read("fact rusty: no"),
            json(r#"{"facts": {"rusty": false}}"#)
        );
        assert_eq!(
            read("to oubliette for roland, chapel for eldric"),
            json(r#"{"to": {"roland": "oubliette", "eldric": "chapel"}}"#)
        );
    }

    #[test]
    fn the_longest_pattern_wins() {
        assert_eq!(read("watches here"), json(r#"{"watches": true}"#));
        assert_eq!(
            read("watches here when self.open"),
            json(r#"{"watches_when": {"get": ["self", "open"]}}"#)
        );
    }

    #[test]
    fn one_word_is_a_tag_and_anything_else_is_kept() {
        assert_eq!(read("evidence"), json(r#"{"tags": ["evidence"]}"#));
        let other = read("thaws ice");
        let kept = other.get("other").and_then(Value::as_arr).unwrap();
        assert_eq!(
            kept[0].get("text").and_then(Value::as_str),
            Some("thaws ice")
        );
    }

    #[test]
    fn repeated_keys_accumulate() {
        let stats = HashSet::new();
        let mut f = Fields::new(&stats);
        for phrase in ["knows a", "evidence", "knows b", "motive"] {
            f.phrase(phrase);
        }
        let expected = json(r#"{"knows": ["a", "b"], "tags": ["evidence", "motive"]}"#);
        assert_eq!(Value::Obj(f.into_pairs()), expected);
    }

    #[test]
    fn stats_and_ranges() {
        assert_eq!(read("wits 4"), json(r#"{"stats": {"wits": 4}}"#));
        assert_eq!(
            read("0..=3"),
            json(r#"{"range": {"range": [0, 3], "inclusive": true}}"#)
        );
    }

    #[test]
    fn aka_takes_names_or_an_order() {
        assert_eq!(
            read("aka \"Le Le Le Leo\", \"Lapkins\""),
            json(r#"{"aka": ["Le Le Le Leo", "Lapkins"]}"#)
        );
        assert_eq!(read("aka shuffled"), json(r#"{"aka_order": "shuffled"}"#));
    }
}
