//! The JSON IR: what `inkjot build` writes and a runtime reads.
//!
//! The rule that shapes it: **a runtime never parses Inkjot.** Every
//! expression is lowered to nested JSON, every Link is expanded with its
//! act's defaults, and every line of a body is tagged with what it is.
//!
//! `docs/ir.md` describes the format.

mod clauses;

use std::collections::{HashMap, HashSet};

use clauses::{per_who, Fields};

use crate::check::{check, Report};
use crate::expr::{self, Expr};
use crate::json::Value;
use crate::syntax::*;
use crate::twee::{self, Line, Passage};

/// Version of the IR format itself, bumped when its shape changes.
pub const IR_VERSION: &str = "3";

/// Compile Inkjot source to its IR. Source with errors is not compiled:
/// the report says what to fix.
pub fn build(src: &str) -> Result<Value, Report> {
    let report = check(src);
    if !report.ok() {
        return Err(report);
    }
    let passages = twee::parse(src, &mut Vec::new());
    Ok(Builder::new(&passages).inklink(&passages))
}

/// Lower one expression, as the IR has it: for tools and tests.
#[doc(hidden)]
pub fn lower_expression(text: &str) -> Value {
    expression(text)
}

/// Lower a source to IR without checking it first: for tools and tests
/// that look at a draft still being built. What a check would refuse may
/// lower to nonsense.
#[doc(hidden)]
pub fn lower_unchecked(src: &str) -> Value {
    let passages = twee::parse(src, &mut Vec::new());
    Builder::new(&passages).inklink(&passages)
}

/// What an act says about the Links that use it.
#[derive(Debug, Default, Clone)]
struct ActDef {
    cost: Vec<Value>,
    scope: Option<String>,
    capacity: Option<Value>,
    ends_turn: bool,
    check: Option<Value>,
    /// `~ choice proactive | reactive | inactive`.
    choice: Option<String>,
    /// `~ at item here where: …`: what it is done to, chosen by the Linker.
    at: Option<Value>,
    /// `~ when …`, and `~ learned from x` as `me.knows.x`: when it may be done.
    whens: Vec<Value>,
    /// `~ then …`: what it does, wherever it is done.
    effects: Vec<Value>,
}

struct Builder {
    acts: HashMap<String, ActDef>,
    /// Names of declared stats and resources: `wits`, `hp`.
    stats: HashSet<String>,
    default_clash: String,
}

fn num(n: impl ToString) -> Value {
    Value::Num(n.to_string())
}

fn strings<'a>(items: impl IntoIterator<Item = &'a str>) -> Value {
    Value::Arr(items.into_iter().map(Value::str).collect())
}

/// A token as a JSON value: a number if it reads as one, else a string.
fn token(word: &str) -> Value {
    // `1'000'000` is a million.
    let digits = word.replace('\'', "");
    if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) && !word.ends_with('\'') {
        num(digits.trim_start_matches('0').parse::<u64>().unwrap_or(0))
    } else {
        Value::str(word)
    }
}

/// One word of a clause as a JSON value: a number, a `{"str": …}` for a
/// quoted string, a lowered expression for anything that reads as one
/// (`self.deaths`, `one(living, in: here)`), and otherwise the word itself.
fn word(text: &str) -> Value {
    let text = text.trim_end_matches([':', ',']);
    if text.starts_with('"') {
        return Value::obj([("str", Value::str(text.trim_matches('"')))]);
    }
    match expr::parse(text) {
        Ok(e) => lower(&e),
        Err(_) => Value::str(text),
    }
}

/// Split `text` into words at top-level spaces, so a quoted string or a
/// bracketed call stays in one piece.
fn words(text: &str) -> Vec<Value> {
    let mask = top_mask(text);
    let mut out = Vec::new();
    let mut start = None;
    for (i, ch) in text.char_indices() {
        let breaks = ch.is_whitespace() && mask[i];
        match (start, breaks) {
            (None, false) => start = Some(i),
            (Some(s), true) => {
                out.push(word(&text[s..i]));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push(word(&text[s..]));
    }
    out
}

/// Split `text` into what comes before a top-level ` when ` and the
/// condition after it, lowered.
fn split_when(text: &str) -> (&str, Option<Value>) {
    match after_word(text, "when") {
        Some(i) => (before_when(text), Some(expression(&text[i..]))),
        None => (text, None),
    }
}

/// `for 3 turns`, `after 2 turns when lit`: how long an effect lasts, or
/// how long until it happens. `("lasts" | "delay", N, when)`.
pub(super) fn timing(text: &str) -> Option<(&'static str, Value, Option<Value>)> {
    let (head, when) = split_when(text.trim());
    let words: Vec<&str> = head.split_whitespace().collect();
    let key = match words.as_slice() {
        ["for", _, "turn" | "turns"] => "lasts",
        ["after", _, "turn" | "turns"] => "delay",
        _ => return None,
    };
    Some((key, token(words[1]), when))
}

/// Put a timing clause on the effect before it: `~ then set me.hushed: yes
/// ~ for 4 turns`. A `when` on the clause joins the effect's own. `false`
/// if `text` is not a timing clause.
pub(super) fn time_effect(effect: Option<&mut Value>, text: &str) -> bool {
    let (Some(Value::Obj(pairs)), Some((key, n, when))) = (effect, timing(text)) else {
        return false;
    };
    pairs.push((key.to_string(), n));
    if let Some(when) = when {
        match pairs.iter_mut().find(|(k, _)| k == "when") {
            Some((_, have)) if *have != Value::Null => {
                *have = Value::obj([("and", Value::Arr(vec![have.clone(), when]))]);
            }
            Some((_, have)) => *have = when,
            None => pairs.push(("when".to_string(), when)),
        }
    }
    true
}

/// `~ along way when …` after an effect: how `nearest` measures (along the
/// map's ways, the only measure there is), and the effect's condition.
fn along(effect: &mut Value, text: &str) -> bool {
    let (head, when) = split_when(text.trim());
    let Some(how) = head.strip_prefix("along ") else {
        return false;
    };
    if let Value::Obj(pairs) = effect {
        pairs.push(("along".to_string(), Value::str(how.trim())));
        if let Some(when) = when {
            match pairs.iter_mut().find(|(k, _)| k == "when") {
                Some((_, have)) if *have != Value::Null => {
                    *have = Value::obj([("and", Value::Arr(vec![have.clone(), when]))]);
                }
                Some((_, have)) => *have = when,
                None => pairs.push(("when".to_string(), when)),
            }
        }
    }
    true
}

/// `pattern rhyme: flame, frost, blink ~ repeat ~ one per round ~ recall
/// first when … ~ falter instead of sunfall when …`.
fn pattern(s: &Stmt, rest: &str) -> Value {
    let (name, steps) = rest.split_once(':').unwrap_or((rest, ""));
    let mut repeat = false;
    let mut first = Vec::new();
    let mut instead = Vec::new();
    for c in &s.clauses[1..] {
        let (head, when) = split_when(c.text.trim());
        let words: Vec<&str> = head.split_whitespace().collect();
        let when = when.unwrap_or(Value::Null);
        match words.as_slice() {
            ["repeat"] => repeat = true,
            [threat, "first"] => first.push(Value::obj([
                ("threat", Value::str(*threat)),
                ("when", when),
            ])),
            [threat, "instead", "of", of] => instead.push(Value::obj([
                ("threat", Value::str(*threat)),
                ("of", Value::str(*of)),
                ("when", when),
            ])),
            _ => {} // `one per round`: a pattern always gives one a round
        }
    }
    Value::obj([
        ("pattern", Value::str(name.trim())),
        (
            "steps",
            strings(steps.split(',').map(str::trim).filter(|t| !t.is_empty())),
        ),
        ("repeat", Value::Bool(repeat)),
        ("first", Value::Arr(first)),
        ("instead", Value::Arr(instead)),
    ])
}

/// `spreads 1 from ablaze to nearest(place, where: …) ~ along way ~ within 1
/// ~ then set it.heat: 1`.
fn spreads(s: &Stmt, rest: &str) -> Value {
    let words: Vec<&str> = rest.splitn(2, ' ').collect();
    let n = token(words.first().copied().unwrap_or("1"));
    let tail = words.get(1).copied().unwrap_or("").trim();
    let (from, to) = match tail.strip_prefix("from ") {
        Some(t) => match after_word(t, "to") {
            Some(i) => (Some(t[..i].trim().trim_end_matches(" to").trim()), &t[i..]),
            None => (Some(t), ""),
        },
        None => (None, tail.strip_prefix("to ").unwrap_or(tail)),
    };
    let mut pairs = vec![
        ("spreads".to_string(), n),
        ("from".to_string(), from.map_or(Value::Null, expression)),
        ("to".to_string(), expression(to.trim())),
    ];
    let mut then = Vec::new();
    let mut when = Value::Null;
    for c in &s.clauses[1..] {
        let text = c.text.trim();
        if let Some(e) = text.strip_prefix("then ") {
            then.push(effect(e));
        } else if let Some(how) = text.strip_prefix("along ") {
            pairs.push(("along".to_string(), Value::str(how.trim())));
        } else if let Some(d) = text.strip_prefix("within ") {
            pairs.push(("within".to_string(), token(d.trim())));
        } else if let Some(e) = text.strip_prefix("when ") {
            when = expression(e);
        }
    }
    pairs.push(("then".to_string(), Value::Arr(then)));
    pairs.push(("when".to_string(), when));
    Value::Obj(pairs)
}

/// The text inside the first pair of double quotes, if any, as written:
/// braces may hold quotes of their own, and escapes are left in.
fn quoted(text: &str) -> Option<&str> {
    let start = text.find('"')?;
    let end = crate::text::string_end(text, start)?;
    Some(&text[start + 1..end])
}

/// Quoted text as the IR keeps it: a plain string, or, when it has braces,
/// `{"text": ["Keeper of ", {"value": "here"}]}`, worked out when used.
pub(crate) fn text_value(inner: &str) -> Value {
    use crate::text::{plain, split, Piece};
    let Ok(pieces) = split(inner) else {
        return Value::str(inner);
    };
    if pieces.iter().all(|p| matches!(p, Piece::Lit(_))) {
        let s: String = pieces
            .iter()
            .map(|p| match p {
                Piece::Lit(l) => plain(l),
                Piece::Code(..) => String::new(),
            })
            .collect();
        return Value::str(s);
    }
    let parts = pieces
        .into_iter()
        .map(|p| match p {
            Piece::Lit(l) => Value::str(plain(&l)),
            Piece::Code(code, _) => Value::obj([("value", expression(&code))]),
        })
        .collect();
    Value::obj([("text", Value::Arr(parts))])
}

/// Quoted text, if there is any, as [`text_value`] keeps it.
fn quoted_text(text: &str) -> Option<Value> {
    quoted(text).map(text_value)
}

/// Quoted text meant to be read, with emphasis lowered as in prose: a
/// `say` line, a hint, a fact's text, a list entry, a description. A plain
/// string when it has no braces and no emphasis; else `{"text": [parts]}`,
/// whose parts may be styled runs. Names (`label`, `shown as`) and values
/// (`set x: "…"`) never take emphasis.
pub(crate) fn styled_value(inner: &str) -> Value {
    use crate::text::{emphasis, plain, split, Piece};
    let Ok(pieces) = split(inner) else {
        return Value::str(inner);
    };
    let raw: Vec<Value> = pieces
        .into_iter()
        .map(|p| match p {
            Piece::Lit(l) => Value::str(l),
            Piece::Code(code, _) => Value::obj([("value", expression(&code))]),
        })
        .collect();
    let parts = emphasis(raw);
    match parts.as_slice() {
        [] => Value::str(""),
        [Value::Str(s)] => Value::str(plain(s)),
        _ => Value::obj([("text", Value::Arr(parts))]),
    }
}

/// Quoted text meant to be read, as [`styled_value`] keeps it.
fn quoted_styled(text: &str) -> Option<Value> {
    quoted(text).map(styled_value)
}

// ---------------------------------------------------------------- expressions

/// Parse and lower an expression. `check` has already accepted the source,
/// so a failure here means this module sliced the text differently; it is
/// recorded in the IR instead of panicking.
pub(crate) fn expression(text: &str) -> Value {
    match expr::parse(text.trim()) {
        Ok(e) => lower(&e),
        Err((_, why)) => Value::obj([
            ("unparsed", Value::str(text.trim())),
            ("why", Value::str(why)),
        ]),
    }
}

/// What a `state` may hold, from its starting value: `boolean`, `integer`,
/// `decimal`, `text`, `id` (a name, or `nothing`), `range`, `list`, or
/// `expression` for anything worked out.
fn type_of(value: &Value) -> &'static str {
    match value {
        Value::Bool(_) => "boolean",
        Value::Num(n) if n.contains('.') => "decimal",
        Value::Num(_) => "integer",
        Value::Str(_) => "id",
        Value::Obj(pairs) => match pairs.first().map(|(k, _)| k.as_str()) {
            Some("str") => "text",
            Some("range") => "range",
            Some("list") => "list",
            Some("neg") if matches!(&pairs[0].1, Value::Num(_)) => "integer",
            _ => "expression",
        },
        _ => "expression",
    }
}

/// Lower an expression tree to JSON.
///
/// - a name is a string: `"me"`; `yes` and `no` are `true` and `false`
/// - a number is a number; a string literal is `{"str": "…"}`
/// - an operator is a one-key object: `{"and": [a, b]}`, `{"not": a}`
/// - a member path is `{"get": ["door", "open"]}`
/// - `who.knows.x` and `who.holds.x` are `{"knows": [who, "x"]}`
/// - a method call puts its target first: `me.is(rook)` is `{"is": ["me", "rook"]}`
/// - named arguments ride in a second key: `{"count": ["linker"], "with": {"in": "here"}}`
/// - the `in` operator is `"within"`, so it cannot be mistaken for a method named `in`
pub fn lower(e: &Expr) -> Value {
    let one = |key: &str, items: Vec<Value>| Value::obj([(key, Value::Arr(items))]);
    match e {
        Expr::Num(n) => {
            let (whole, frac) = n.split_once('.').unwrap_or((n, ""));
            let whole = whole.trim_start_matches('0');
            let whole = if whole.is_empty() { "0" } else { whole };
            Value::Num(if frac.is_empty() {
                whole.to_string()
            } else {
                format!("{whole}.{frac}")
            })
        }
        Expr::Str(s) => Value::obj([("str", Value::str(s))]),
        // Text with braces: `{"text": ["Keeper of ", {"value": "here"}]}`.
        Expr::Text(parts) => Value::obj([(
            "text",
            Value::Arr(
                parts
                    .iter()
                    .map(|p| match p {
                        crate::expr::TextPart::Lit(s) => Value::str(s),
                        crate::expr::TextPart::Expr(e) => Value::obj([("value", lower(e))]),
                    })
                    .collect(),
            ),
        )]),
        Expr::If(c, a, b) => one("if", vec![lower(c), lower(a), lower(b)]),
        // `yes` and `no` are Inkjot's booleans.
        Expr::Name(n, _) if n == "yes" => Value::Bool(true),
        Expr::Name(n, _) if n == "no" => Value::Bool(false),
        Expr::Name(n, _) => Value::str(n),
        Expr::Member(target, member) => {
            if let Expr::Member(who, verb) = target.as_ref() {
                if verb == "knows" || (verb == "holds" && member != "any") {
                    return one(verb, vec![lower(who), Value::str(member)]);
                }
            }
            // Flatten `a.b.c` into one path.
            match lower(target) {
                Value::Obj(mut pairs) if pairs.len() == 1 && pairs[0].0 == "get" => {
                    if let Value::Arr(path) = &mut pairs[0].1 {
                        path.push(Value::str(member));
                    }
                    Value::Obj(pairs)
                }
                root => one("get", vec![root, Value::str(member)]),
            }
        }
        Expr::Call(target, args) => {
            let (name, mut items) = match target.as_ref() {
                Expr::Name(function, _) => (function.clone(), Vec::new()),
                Expr::Member(receiver, method) => (method.clone(), vec![lower(receiver)]),
                other => ("call".to_string(), vec![lower(other)]),
            };
            let mut named = Vec::new();
            for arg in args {
                match &arg.key {
                    Some(key) => named.push((key.clone(), lower(&arg.value))),
                    None => items.push(lower(&arg.value)),
                }
            }
            let mut pairs = vec![(name, Value::Arr(items))];
            if !named.is_empty() {
                pairs.push(("with".to_string(), Value::Obj(named)));
            }
            Value::Obj(pairs)
        }
        // `index`, not `at`: `me.at(lectern)` is a method named `at`.
        Expr::Index(target, index) => one("index", vec![lower(target), lower(index)]),
        Expr::List(items) => one("list", items.iter().map(lower).collect()),
        Expr::Unary(op, operand) => {
            Value::obj([(if *op == "-" { "neg" } else { *op }, lower(operand))])
        }
        // `{{ … }} in dossier`: a question for that base.
        Expr::Binary("in", lhs, rhs) if matches!(lhs.as_ref(), Expr::Fact { .. }) => {
            let mut q = lower(lhs);
            if let (Value::Obj(pairs), Expr::Name(base, _)) = (&mut q, rhs.as_ref()) {
                pairs.push(("in".into(), Value::str(base)));
            }
            q
        }
        Expr::Binary(op, lhs, rhs) => one(
            if *op == "in" { "within" } else { op },
            vec![lower(lhs), lower(rhs)],
        ),
        Expr::Var(v) => Value::obj([("var", Value::str(v))]),
        Expr::Any => Value::obj([("any", Value::Bool(true))]),
        Expr::Fact {
            subject,
            verb,
            object,
            at,
        } => {
            // A time is a path of period names, or a variable.
            let at = match at.as_deref() {
                None => Value::Null,
                Some(Expr::Var(v)) => Value::obj([("var", Value::str(v))]),
                Some(e) => Value::Str(e.to_string()),
            };
            Value::obj([(
                "fact",
                Value::obj([
                    ("subject", lower(subject)),
                    ("verb", Value::str(verb)),
                    ("object", object.as_deref().map_or(Value::Null, lower)),
                    ("at", at),
                ]),
            )])
        }
        Expr::Quant(q, vars, body) => Value::obj([
            ("quant", Value::str(*q)),
            ("vars", Value::Arr(vars.iter().map(Value::str).collect())),
            ("body", lower(body)),
        ]),
        Expr::Range { lo, hi, op } => {
            let bound = |b: &Option<Box<Expr>>| b.as_deref().map_or(Value::Null, lower);
            Value::obj([
                ("range", Value::Arr(vec![bound(lo), bound(hi)])),
                ("inclusive", Value::Bool(*op == "..=")),
            ])
        }
    }
}

// ---------------------------------------------------------------- the builder

impl Builder {
    fn new(passages: &[Passage]) -> Builder {
        let mut b = Builder {
            acts: HashMap::new(),
            stats: HashSet::new(),
            default_clash: "none".to_string(),
        };
        for p in passages {
            let (stmts, _) = gather(p, &mut Vec::new());
            for s in &stmts {
                let head = &s.clauses[0].text;
                match s.kw.as_str() {
                    "stat" | "resource" => {
                        let name = first_word(head[s.kw.len()..].trim_start());
                        b.stats.insert(name.to_string());
                    }
                    "act" => {
                        let name = first_word(head[3..].trim_start()).to_string();
                        let def = b.act_def(s);
                        b.acts.insert(name, def);
                    }
                    // A combo costs the sum of its acts, unless it says otherwise.
                    "combo" => {
                        let (name, list) = head[5..].split_once(':').unwrap_or((head, ""));
                        let total: u64 = list
                            .split(',')
                            .filter_map(|act| b.acts.get(act.trim()))
                            .filter_map(|def| def.cost.first()?.get("amount")?.as_u64())
                            .sum();
                        let mut def = b.act_def(s);
                        if def.cost.is_empty() {
                            def.cost.push(Value::obj([("amount", num(total))]));
                        }
                        // A combo needs each of its acts known as well as itself.
                        for act in list.split(',') {
                            if let Some(part) = b.acts.get(act.trim()) {
                                for w in &part.whens {
                                    if !def.whens.contains(w) {
                                        def.whens.push(w.clone());
                                    }
                                }
                            }
                        }
                        b.acts.insert(name.trim().to_string(), def);
                    }
                    "default" if head.starts_with("default clash:") => {
                        b.default_clash = head["default clash:".len()..].trim().to_string();
                    }
                    _ => {}
                }
            }
        }
        b
    }

    /// The fields of an `act` declaration that Links inherit.
    fn act_def(&self, s: &Stmt) -> ActDef {
        let mut def = ActDef::default();
        let head = &s.clauses[0].text;
        let first = head.split_once(':').map_or("", |(_, v)| v.trim());
        for clause in std::iter::once(first).chain(s.clauses[1..].iter().map(|c| c.text.as_str())) {
            if time_effect(def.effects.last_mut(), clause) {
                continue;
            }
            let word = first_word(clause);
            let rest = clause[word.len()..].trim();
            match word {
                "cost" => def.cost.push(cost(rest)),
                "private" | "shared" => def.scope = Some(word.to_string()),
                "capacity" => def.capacity = Some(token(rest)),
                "ends" => def.ends_turn = true,
                "check" => def.check = Some(expression(rest)),
                "choice" => def.choice = Some(rest.to_string()),
                "at" => def.at = Some(target_filter(rest)),
                "when" => def.whens.push(expression(rest)),
                "learned" => {
                    let fact = rest.strip_prefix("from").unwrap_or(rest).trim();
                    def.whens.push(expression(&format!("me.knows.{fact}")));
                }
                "then" => def.effects.push(effect(rest)),
                _ => {}
            }
        }
        def
    }

    fn inklink(&self, passages: &[Passage]) -> Value {
        let mut meta: Vec<(String, Value)> = vec![
            ("ir".into(), Value::str(IR_VERSION)),
            ("inkjot".into(), Value::Null),
            ("title".into(), Value::Null),
        ];
        let mut config = Vec::new();
        let mut out = Vec::new();
        for p in passages {
            let text = || {
                p.lines
                    .iter()
                    .map(|l| l.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            match p.name.as_str() {
                "StoryTitle" => meta[2].1 = Value::str(text().trim()),
                "StoryData" => {
                    if let Ok(Value::Obj(pairs)) = crate::json::parse(&text()) {
                        for (key, value) in pairs {
                            match key.as_str() {
                                "format-version" => meta[1].1 = value,
                                "format" => {}
                                _ => meta.push((key, value)),
                            }
                        }
                    }
                }
                // Tests are for the Inker; `inkjot test` reads them from source.
                _ if p.has_tag("test") => {}
                _ if p.has_tag("config") => {
                    let (stmts, _) = gather(p, &mut Vec::new());
                    config.extend(stmts.iter().map(|s| self.statement(s)));
                }
                _ => out.push(self.passage(p)),
            }
        }
        let mut acts: Vec<(&String, &ActDef)> = self.acts.iter().collect();
        acts.sort_by_key(|(name, _)| name.as_str());
        let acts = acts.into_iter().map(|(name, def)| {
            (
                name.clone(),
                Value::obj([
                    ("cost", Value::Arr(def.cost.clone())),
                    (
                        "scope",
                        def.scope.as_deref().map_or(Value::Null, Value::str),
                    ),
                    ("capacity", def.capacity.clone().unwrap_or(Value::Null)),
                    ("ends_turn", Value::Bool(def.ends_turn)),
                    ("check", def.check.clone().unwrap_or(Value::Null)),
                    (
                        "choice",
                        def.choice.as_deref().map_or(Value::Null, Value::str),
                    ),
                ]),
            )
        });
        meta.push(("config".into(), Value::Arr(config)));
        meta.push(("acts".into(), Value::Obj(acts.collect())));
        meta.push(("passages".into(), Value::Arr(out)));
        Value::Obj(meta)
    }

    fn passage(&self, p: &Passage) -> Value {
        let (mut stmts, body) = gather(p, &mut Vec::new());
        let margin = p.tags.iter().any(|t| MARGIN_TAGS.contains(&t.as_str()));
        // `title: "The Millpond"` is the passage's, not a header statement.
        let title = stmts.iter().position(|s| s.kw == "title").map(|i| {
            let s = stmts.remove(i);
            let text = s.clauses[0].text["title".len()..]
                .trim_start_matches(':')
                .trim();
            Value::str(quoted(text).unwrap_or(text))
        });
        // `narration: first person` in a passage tells that passage only.
        let narration = stmts
            .iter()
            .position(|s| s.kw == "narration")
            .map(|i| narration(&stmts.remove(i)));
        let mut pairs = vec![
            ("name".to_string(), Value::str(&p.name)),
            (
                "tags".to_string(),
                strings(p.tags.iter().map(String::as_str)),
            ),
            ("margin".to_string(), Value::Bool(margin)),
            (
                "header".to_string(),
                Value::Arr(stmts.iter().map(|s| self.statement(s)).collect()),
            ),
            ("body".to_string(), Value::Arr(self.body(p, body))),
        ];
        // A `(((…)))` doc comment, for Inkwell. Only passages that have one.
        if let Some(doc) = &p.doc {
            pairs.insert(2, ("doc".to_string(), Value::str(doc)));
        }
        if let Some(title) = title {
            pairs.insert(1, ("title".to_string(), title));
        }
        if let Some(narration) = narration {
            let at = pairs.iter().position(|(k, _)| k == "margin").unwrap_or(1);
            pairs.insert(at, ("narration".to_string(), narration));
        }
        Value::Obj(pairs)
    }

    // ------------------------------------------------------------ statements

    fn statement(&self, s: &Stmt) -> Value {
        let head = s.clauses[0].text.as_str();
        let kw = s.kw.as_str();
        let rest = head.get(kw.len()..).unwrap_or("").trim_start();
        let source = || {
            s.clauses
                .iter()
                .map(|c| c.text.as_str())
                .collect::<Vec<_>>()
                .join(" ~ ")
        };
        match kw {
            "{" => insert(head.trim_start_matches('{').trim_end_matches('}')),
            "[" => modifier(head),
            _ if STRETCH.contains(&kw) => Value::obj([
                ("stretch", Value::str(kw)),
                ("source", Value::str(source())),
            ]),
            "on" | "upon" => handler(rest, &s.clauses[1..]),
            // `let` is worked out on arrival; `get` each time it is read.
            "let" | "get" => {
                let (name, value) = rest.split_once(':').unwrap_or((rest, ""));
                let (value, when) = split_when(value.trim());
                let value = match value.strip_prefix("roll ") {
                    Some(dice) => Value::obj([("roll", Value::str(dice.trim()))]),
                    None => expression(value),
                };
                let mut pairs = vec![
                    ("let".to_string(), Value::str(name.trim())),
                    ("value".to_string(), value),
                    ("when".to_string(), when.unwrap_or(Value::Null)),
                ];
                if kw == "get" {
                    pairs.push(("derived".to_string(), Value::Bool(true)));
                }
                Value::Obj(pairs)
            }
            "expect" => {
                let (scope, value) = rest.split_once(':').unwrap_or(("", rest));
                let otherwise = s.clauses[1..]
                    .iter()
                    .find_map(|c| c.text.strip_prefix("else "))
                    .and_then(quoted);
                Value::obj([
                    ("expect", expression(value)),
                    ("always", Value::Bool(scope.trim() == "always")),
                    ("else", otherwise.map_or(Value::Null, Value::str)),
                ])
            }
            _ if DECLS.contains(&kw) => self.declaration(s, kw, rest),
            _ if is_effect(kw, rest) => {
                // `set planks_out: yes ~ after 2 turns`: the timing is the effect's.
                let mut done = effect(head);
                let mut rest = Vec::new();
                for c in &s.clauses[1..] {
                    if !time_effect(Some(&mut done), &c.text) && !along(&mut done, &c.text) {
                        rest.push(clause(&c.text));
                    }
                }
                let mut pairs = vec![("effect".to_string(), done)];
                if !rest.is_empty() {
                    pairs.push(("clauses".to_string(), Value::Arr(rest)));
                }
                Value::Obj(pairs)
            }
            // Settings, and conditional values such as `hp 3 when …`.
            _ => self.setting(s, kw, rest),
        }
    }

    /// `item cufflink: "a scorched cufflink" ~ evidence ~ in dining`
    fn declaration(&self, s: &Stmt, kw: &str, rest: &str) -> Value {
        if kw == "period" {
            return period(s);
        }
        let (id, value) = rest.split_once(':').unwrap_or((rest, ""));
        let (value, when) = split_when(value.trim());
        let mut f = Fields::new(&self.stats);
        f.add("declare", Value::str(kw));
        f.add("id", Value::str(id.trim()));
        // What follows the colon depends on what is being declared.
        match kw {
            "state" => {
                let value = expression(value);
                f.add("type", Value::str(type_of(&value)));
                f.add("value", value);
            }
            "stat" => f.add("range", expression(value)),
            "fuse" => f.add("start", token(value)),
            "way" => {
                let (from, to) = value.split_once(" - ").unwrap_or((value, ""));
                f.add("from", Value::str(from.trim()));
                f.add("to", Value::str(to.trim()));
            }
            "combo" => f.add("acts", strings(value.split(',').map(str::trim))),
            "relation" => f.add("between", strings(value.split(',').map(str::trim))),
            // `table syndicate: shuffled`: rows drawn, not rolled for.
            "table" if matches!(value.trim(), "shuffled" | "random" | "cycle") => {
                f.add("pick", Value::str(value.trim()))
            }
            "table" => f.add(
                "roll",
                expression(value.strip_prefix("roll ").unwrap_or(value)),
            ),
            "map" => {
                let size: Vec<Value> = value.split(" by ").map(|n| token(n.trim())).collect();
                f.add("size", Value::Arr(size));
            }
            "resource" => {
                // `60`, or `per character` with no starting value.
                let (amount, more) = value.split_once(' ').unwrap_or((value, ""));
                if amount.chars().all(|c| c.is_ascii_digit()) && !amount.is_empty() {
                    f.add("value", token(amount));
                    f.phrase(more);
                } else {
                    f.phrase(value);
                }
            }
            _ => f.phrase(value),
        }
        if let Some(when) = when {
            f.add("when", when);
        }
        for c in &s.clauses[1..] {
            f.phrase(&c.text);
        }
        let mut pairs = f.into_pairs();
        // `threat sunfall: at pov.all`: whom it is at, worked out when it comes.
        if kw == "threat" {
            for (k, v) in pairs.iter_mut() {
                if k == "at" {
                    if let Value::Str(who) = v {
                        *v = expression(&who.clone());
                    }
                }
            }
        }
        // A fact's text and a description are read, so they take emphasis;
        // a name does not.
        let texts: Vec<&str> = std::iter::once(value)
            .chain(s.clauses[1..].iter().map(|c| c.text.as_str()))
            .filter(|t| t.starts_with('"'))
            .filter_map(quoted)
            .collect();
        for (key, i) in [("label", 0), ("description", 1)] {
            if key == "label" && kw != "fact" {
                continue;
            }
            if let (Some(slot), Some(text)) =
                (pairs.iter_mut().find(|(k, _)| k == key), texts.get(i))
            {
                slot.1 = styled_value(text);
            }
        }
        Value::Obj(pairs)
    }

    /// `cohort: 2..=5 ~ best 3..=4`, `goal flee: ~ toward platform`, …
    fn setting(&self, s: &Stmt, kw: &str, rest: &str) -> Value {
        let (value, when) = split_when(rest.trim_start_matches(':').trim());
        let mut f = Fields::new(&self.stats);
        match kw {
            // Named blocks: the name, then fields from the clauses.
            "goal" | "reaction" | "checkpoint" => {
                f.add(kw, Value::str(value.trim_end_matches(':').trim()));
            }
            "hint" => f.add("hint", quoted_styled(value).unwrap_or(Value::str(value))),
            // `recipe stretcher: stave, stave, sacking ~ using hammer ~ make ~ "…"`.
            "recipe" => {
                let (result, parts) = rest.split_once(':').unwrap_or((rest, ""));
                f.add("recipe", Value::str(result.trim()));
                f.add(
                    "parts",
                    strings(parts.split(',').map(str::trim).filter(|p| !p.is_empty())),
                );
                let mut using = Vec::new();
                for c in &s.clauses[1..] {
                    let text = c.text.trim();
                    if let Some(tools) = text.strip_prefix("using ") {
                        using.extend(tools.split(',').map(str::trim).filter(|t| !t.is_empty()));
                    } else if text.starts_with('"') {
                        f.add("label", quoted_text(text).unwrap_or(Value::str(text)));
                    } else if is_ident(text) {
                        f.add("verb", Value::str(text));
                    }
                }
                f.add("using", strings(using));
                return Value::Obj(f.into_pairs());
            }
            // `spreads 1 from ablaze to nearest(place, where: …) ~ along way
            // ~ within 1 ~ then set it.heat: 1`.
            "spreads" => return spreads(s, rest),
            // `pattern rhyme: flame, frost ~ repeat ~ one per round ~ recall
            // first when … ~ falter instead of sunfall when …`.
            "pattern" => return pattern(s, rest),
            // `encounter hask ~ round by round ~ acts last ~ until …`.
            "encounter" => {
                f.add("encounter", Value::str(value.trim()));
                for c in &s.clauses[1..] {
                    f.phrase(&c.text);
                }
                return Value::Obj(f.into_pairs());
            }
            // `measure nights: nights` → `{"measure": "nights", "value": …}`.
            // `rule wears: {{ N is P }} and {{ N at A @ W }} => {{ P at A @ W }}
            // ~ known`.
            "rule" => return rule(s, rest),
            // `never {{ N is A }} and {{ N is B }} and A != B ~ in notebook ~
            // on contradiction refuse ~ "A name is worn by one person."`
            "never" => return never(s, rest),
            // `finale when: no guest where: …`: the game ends here when it holds.
            "finale" => {
                let cond = rest.trim().strip_prefix("when:").unwrap_or(rest).trim();
                f.add("finale_when", expression(cond));
                return Value::Obj(f.into_pairs());
            }
            "measure" => {
                let (name, expr) = rest.split_once(':').unwrap_or((rest, ""));
                f.add("measure", Value::str(name.trim()));
                f.add("value", expression(expr.trim()));
                return Value::Obj(f.into_pairs());
            }
            "narration" => {
                f.add("setting", Value::str("narration"));
                f.add("value", narration(s));
                return Value::Obj(f.into_pairs());
            }
            // `cue names: capital, bold` → `{"setting": "cue", "of": "names", "value": […]}`.
            "cue" => {
                let (of, list) = rest.split_once(':').unwrap_or((rest, ""));
                f.add("setting", Value::str("cue"));
                f.add("of", Value::str(of.trim()));
                f.add(
                    "value",
                    strings(list.split(',').map(str::trim).filter(|w| !w.is_empty())),
                );
                return Value::Obj(f.into_pairs());
            }
            "tags" => {
                let all =
                    std::iter::once(value).chain(s.clauses[1..].iter().map(|c| c.text.as_str()));
                f.add("setting", Value::str("tags"));
                f.add("tags", strings(all.filter(|t| !t.is_empty())));
                return Value::Obj(f.into_pairs());
            }
            _ => {
                f.add("setting", Value::str(kw));
                let typed = match kw {
                    "ages" | "cohort" | "clock" if value == "none" => Value::Null,
                    "ages" | "cohort" | "clock" => expression(value),
                    "pov" => strings(value.split(',').map(str::trim)),
                    "start" => per_who(value),
                    "route" => strings(value.split(" - ").map(str::trim)),
                    "default" => {
                        let (what, list) = value.split_once(':').unwrap_or((value, ""));
                        f.add("of", Value::str(what.trim()));
                        match what.trim() {
                            "moves" => strings(list.split(',').map(str::trim)),
                            _ => Value::str(list.trim()),
                        }
                    }
                    "moves" => {
                        f.phrase(&format!("moves {value}"));
                        Value::Null
                    }
                    // A conditional value for a stat or resource: `hp 3 when …`.
                    _ if self.stats.contains(kw) => token(value),
                    _ => Value::str(value),
                };
                if typed != Value::Null {
                    f.add("value", typed);
                }
            }
        }
        if let Some(when) = when {
            f.add("when", when);
        }
        for c in &s.clauses[1..] {
            f.phrase(&c.text);
        }
        Value::Obj(f.into_pairs())
    }

    // ------------------------------------------------------------ bodies

    fn body(&self, p: &Passage, lines: &[Line]) -> Vec<Value> {
        let mut out: Vec<Value> = Vec::new();
        // Derived Link ids seen so far: `ch2->ch3` twice is `ch2->ch3#2`.
        let mut derived: HashMap<String, usize> = HashMap::new();
        // The offered Link a fork stands in for, unless it names one.
        let mut last_offered: Option<String> = None;
        let mut i = 0;
        // An author's note runs from `[note]` to the next modifier. It is
        // for the Inker, so it never reaches the IR.
        let mut in_note = false;
        // A Link group: a `>` line with clauses and no Link lends its
        // clauses to every Link below it, up to the next blank line.
        let mut group: Vec<String> = Vec::new();
        while i < lines.len() {
            let t = lines[i].text.trim();
            i += 1;
            // A line and the `~` lines that continue it.
            let mut more: Vec<&str> = Vec::new();
            if !t.is_empty() && !(t.starts_with('[') && !t.starts_with("[[")) {
                while i < lines.len() && lines[i].text.trim_start().starts_with('~') {
                    more.push(lines[i].text.trim());
                    i += 1;
                }
            }
            if t.is_empty() {
                group.clear();
                continue;
            } else if t.starts_with('>') && !t.contains("[[") {
                group = std::iter::once(t.trim_start_matches('>').trim().to_string())
                    .chain(more.iter().map(|m| m.to_string()))
                    .collect();
                continue;
            } else if t.starts_with('>') {
                let mut lent: Vec<&str> = group.iter().map(String::as_str).collect();
                lent.extend(more.iter().copied());
                let more = lent;
                let mut link = self.link(p, t, &more, &mut derived);
                let id = link.get("id").and_then(Value::as_str).map(str::to_string);
                match &mut link {
                    Value::Obj(pairs) if pairs.iter().any(|(k, _)| k == "inquest") => {
                        if let Some((_, Value::Obj(fields))) =
                            pairs.iter_mut().find(|(k, _)| k == "inquest")
                        {
                            if !fields.iter().any(|(k, _)| k == "stands_in_for") {
                                let offered = last_offered.clone().map_or(Value::Null, Value::str);
                                fields.insert(0, ("stands_in_for".into(), offered));
                            }
                        }
                    }
                    _ => last_offered = id,
                }
                out.push(link);
            } else if t.starts_with('[') && !t.starts_with("[[") && t.ends_with(']') {
                in_note = t[1..]
                    .split([';', ']'])
                    .any(|part| matches!(part.trim(), "note" | "todo" | "fix"));
                if !in_note {
                    out.push(modifier(t));
                }
            } else if in_note {
                continue;
            } else {
                let paragraphs = |first: &str| {
                    let rest = more.iter().map(|m| m.trim_start_matches('~').trim());
                    Value::Arr(std::iter::once(first).chain(rest).map(parts).collect())
                };
                match t.strip_prefix('@') {
                    Some(cue) => {
                        let who = first_word(cue);
                        let mut text = cue[who.len()..].trim_start();
                        let mut direction = Value::Null;
                        if let Some(inner) = text.strip_prefix('(') {
                            if let Some((d, after)) = inner.split_once(')') {
                                direction = Value::str(d.trim());
                                text = after.trim_start();
                            }
                        }
                        // `@you`, `@i`: the pov, which the IR always calls `me`.
                        let who = if POV_NAMES.contains(&who) { "me" } else { who };
                        out.push(Value::obj([
                            ("type", Value::str("speech")),
                            ("who", Value::str(who)),
                            ("direction", direction),
                            ("paragraphs", paragraphs(text)),
                        ]));
                    }
                    None => out.push(Value::obj([
                        ("type", Value::str("prose")),
                        ("paragraphs", paragraphs(t)),
                    ])),
                }
            }
        }
        out
    }

    /// One Link, expanded with the defaults of the act it uses.
    fn link(
        &self,
        p: &Passage,
        line: &str,
        more: &[&str],
        derived: &mut HashMap<String, usize>,
    ) -> Value {
        let open = line.find("[[").map_or(0, |i| i + 2);
        let close = line.find("]]").unwrap_or(line.len());
        let inner = line.get(open..close).unwrap_or("");
        let (label, target) = match (inner.rfind("->"), inner.rfind('|')) {
            (Some(i), _) => (&inner[..i], &inner[i + 2..]),
            (None, Some(i)) => (&inner[..i], &inner[i + 1..]),
            (None, None) => (inner, inner),
        };
        let mut clauses = split_clauses(line.get(close + 2..).unwrap_or(""), 0, 0);
        for m in more {
            clauses.extend(split_clauses(m, 0, 0));
        }

        let mut act: Option<(String, ActDef)> = None;
        let mut arg = Value::Null;
        let mut costs = Vec::new();
        let (mut scope, mut capacity, mut clash, mut ends_turn, mut check_) =
            (None, None, None, None, None);
        let mut choice: Option<String> = None;
        let (mut whens, mut hidden, mut audience, mut effects) =
            (Vec::new(), Value::Null, Value::Null, Vec::new());
        let mut kind = "go";
        let mut extra: Vec<(String, Value)> = Vec::new();
        let mut overrides: Vec<&str> = Vec::new();
        let mut inker_id: Option<String> = None;
        // A fork: `~ inquest …`, and what a pending one needs.
        let mut fork: Option<Vec<(String, Value)>> = None;
        let mut pending: Vec<(String, Value)> = Vec::new();
        let mut credit: Option<String> = None;
        // Analysis tags, for the Inker's reports.
        let mut analysis: Vec<(String, Value)> = Vec::new();
        let list = |rest: &str| strings(rest.split(',').map(str::trim));
        let mut at: Option<Value> = None;

        for c in &clauses {
            let text = c.text.as_str();
            if time_effect(effects.last_mut(), text) {
                continue;
            }
            let word = first_word(text);
            let rest = text[word.len()..].trim();
            match word {
                "cost" => {
                    costs.push(cost(rest));
                    overrides.push("cost");
                }
                "when" => whens.push(expression(rest)),
                "hidden" => hidden = expression(rest.strip_prefix("until").unwrap_or(rest)),
                // `~ for guest where: it.alive`: who of a kind may take it.
                "for" if rest.contains(" where:") => {
                    let (kinds, cond) = rest.split_once(" where:").unwrap_or((rest, ""));
                    audience = strings(kinds.split(',').map(str::trim));
                    extra.push(("for_where".into(), expression(cond.trim())));
                }
                "for" => audience = strings(rest.split(',').map(str::trim)),
                // `~ consensus`, `~ consensus 2`: a vote among those who may
                // take it; each chooses that many.
                "consensus" => extra.push((
                    "consensus".into(),
                    if rest.trim().is_empty() {
                        num(1)
                    } else {
                        token(rest.trim())
                    },
                )),
                "then" => effects.push(effect(rest)),
                "ends" => ends_turn = Some(true),
                "private" | "shared" => scope = Some(word.to_string()),
                "capacity" => capacity = Some(token(rest)),
                "clash" => clash = Some(rest.to_string()),
                "check" => check_ = Some(expression(rest)),
                "choice" => choice = Some(rest.to_string()),
                "with" => extra.push(("with".into(), parameters(rest))),
                // `~ at item here where: …`: one Link for each that fits.
                "at" => at = Some(target_filter(rest)),
                "id" => inker_id = Some(rest.to_string()),
                "switch" | "pass" | "hint" | "release" => kind = word,
                "inquest" => {
                    let fields = fork.get_or_insert_with(Vec::new);
                    let (how, value) = rest.split_once(' ').unwrap_or((rest, ""));
                    match how {
                        "for" => fields.push(("stands_in_for".into(), Value::str(value.trim()))),
                        "from" => fields.push(("from".into(), list(value))),
                        "by" => pending.push(("by".into(), Value::str(value.trim()))),
                        _ => {}
                    }
                }
                "rejoin" => pending.push(("rejoin".into(), list(rest))),
                "brief" => pending.push(("brief".into(), Value::str(rest))),
                "credit" => credit = Some(rest.to_string()),
                "beat" | "stakes" | "tone" | "reveals" => {
                    analysis.push((word.into(), Value::str(rest)))
                }
                "weighs" => analysis.push(("weighs".into(), list(rest))),
                "plants" => analysis.push(("plants".into(), list(rest))),
                "pays" => analysis.push((
                    "pays_off".into(),
                    list(rest.strip_prefix("off").unwrap_or(rest).trim()),
                )),
                "arc" => {
                    let (who, way) = rest.split_once(' ').unwrap_or((rest, ""));
                    analysis.push((
                        "arc".into(),
                        Value::obj([("who", Value::str(who)), ("way", Value::str(way.trim()))]),
                    ));
                }
                "predict" => analysis.push(("predict".into(), token(rest.trim_end_matches('%')))),
                "lasting" => analysis.push(("lasting".into(), Value::Bool(true))),
                // `~ uses pick`: a tool, which may break.
                "uses" => extra.push(("uses".into(), list(rest))),
                "become" | "order" | "by" | "chance" | "after" | "requires" => {
                    let mut values = words(rest);
                    let value = if values.len() == 1 {
                        values.remove(0)
                    } else {
                        Value::Arr(values)
                    };
                    extra.push((word.into(), value));
                }
                "once" | "loud" | "quiet" => extra.push((word.into(), Value::Bool(true))),
                _ => {
                    // The name of an act or combo, with an optional argument: `take cufflink`.
                    let def = self.acts.get(word).cloned().unwrap_or_default();
                    act = Some((word.to_string(), def));
                    kind = "act";
                    let (argument, when) = split_when(rest);
                    if !argument.is_empty() {
                        arg = Value::str(argument);
                    }
                    whens.extend(when);
                }
            }
        }

        let def = act.as_ref().map(|(_, d)| d.clone()).unwrap_or_default();
        // An act's conditions and effects hold wherever it is done.
        whens.splice(0..0, def.whens.iter().cloned());
        effects.splice(0..0, def.effects.iter().cloned());
        if let Some(at) = at.or(def.at.clone()) {
            extra.push(("at".into(), at));
        }
        // An act's costs come first; a Link's follow. The last one whose
        // condition holds is the one that applies.
        let mut all_costs = def.cost.clone();
        all_costs.extend(costs);
        for (field, set) in [
            ("scope", scope.is_some()),
            ("capacity", capacity.is_some()),
            ("clash", clash.is_some()),
            ("ends_turn", ends_turn.is_some()),
            ("check", check_.is_some()),
            ("choice", choice.is_some()),
        ] {
            if set {
                overrides.push(field);
            }
        }
        overrides.dedup();
        let scope = scope.or(def.scope).unwrap_or_else(|| "private".to_string());
        let clash = clash.unwrap_or_else(|| {
            if scope == "shared" {
                self.default_clash.clone()
            } else {
                "none".to_string()
            }
        });
        let when = match whens.len() {
            0 => Value::Null,
            1 => whens.remove(0),
            _ => Value::obj([("and", Value::Arr(whens))]),
        };

        // The Inker's id, else one from the passage and the target, which
        // survives reordering; a second Link to the same target gets `#2`.
        let (id, id_from) = match inker_id {
            Some(id) => (id, "inker"),
            None => {
                let base = format!("{}->{}", p.name, target.trim());
                let n = derived.entry(base.clone()).or_default();
                *n += 1;
                let id = if *n == 1 { base } else { format!("{base}#{n}") };
                (id, "target")
            }
        };
        let mut pairs: Vec<(String, Value)> = vec![
            ("type".into(), Value::str("link")),
            ("id".into(), Value::str(&id)),
            ("id_from".into(), Value::str(id_from)),
            ("label".into(), parts(label.trim())),
            ("target".into(), Value::str(target.trim())),
            ("kind".into(), Value::str(kind)),
            (
                "act".into(),
                act.as_ref()
                    .map_or(Value::Null, |(name, _)| Value::str(name)),
            ),
            ("arg".into(), arg),
            ("cost".into(), Value::Arr(all_costs)),
            ("scope".into(), Value::str(scope)),
            (
                "capacity".into(),
                capacity.or(def.capacity).unwrap_or(Value::Null),
            ),
            ("clash".into(), Value::str(clash)),
            ("check".into(), check_.or(def.check).unwrap_or(Value::Null)),
            // A Link's own choice, else its act's; passing is inactive.
            (
                "choice".into(),
                choice
                    .or(def.choice)
                    .or((kind == "pass").then(|| "inactive".to_string()))
                    .map_or(Value::Null, Value::str),
            ),
            ("when".into(), when),
            ("hidden_until".into(), hidden),
            ("for".into(), audience),
            ("effects".into(), Value::Arr(effects)),
            (
                "ends_turn".into(),
                Value::Bool(ends_turn.unwrap_or(def.ends_turn)),
            ),
            ("overrides".into(), strings(overrides)),
        ];
        if p.has_tag("lib") {
            pairs.insert(3, ("lib".into(), Value::str(&p.name)));
        }
        pairs.extend(extra);
        // A fork: who wrote it, where else it may be asked from, and, if it
        // is a slot not written yet, who fills it and where it rejoins.
        if let Some(mut fields) = fork {
            let writer = credit.unwrap_or_else(|| "inker".to_string());
            fields.push(("credit".into(), Value::str(writer)));
            if !pending.is_empty() {
                fields.push(("pending".into(), Value::Obj(pending)));
            }
            pairs.push(("inquest".into(), Value::Obj(fields)));
        }
        if !analysis.is_empty() {
            pairs.push(("analysis".into(), Value::Obj(analysis)));
        }
        Value::Obj(pairs)
    }
}

// ---------------------------------------------------------------- the pieces

/// `who: suspect, how: scenario.methods, revote: 1` as an object: a number
/// is a number, `'quoted'` text is text, and anything else an expression.
pub(crate) fn insert_args(text: &str) -> Value {
    Value::Obj(
        split_args(text)
            .into_iter()
            .filter_map(|part| {
                let (key, value) = part.split_once(':')?;
                let value = value.trim();
                let number = value.replace('\'', "");
                let lowered = match value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
                    Some(text) => Value::str(text),
                    None if number.parse::<f64>().is_ok() => Value::Num(number),
                    None => expression(value),
                };
                Some((key.trim().to_string(), lowered))
            })
            .collect(),
    )
}

/// Split at the commas that are outside brackets and quotes.
pub(crate) fn split_args(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let (mut depth, mut quote, mut cur) = (0i32, None::<char>, String::new());
    for ch in text.chars() {
        match ch {
            // Text is in double quotes; `'` is only an apostrophe or a
            // thousands mark.
            '"' if quote == Some(ch) => quote = None,
            '"' if quote.is_none() => quote = Some(ch),
            '(' | '[' | '{' if quote.is_none() => depth += 1,
            ')' | ']' | '}' if quote.is_none() => depth -= 1,
            ',' if quote.is_none() && depth == 0 => {
                parts.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(ch);
    }
    if !cur.trim().is_empty() {
        parts.push(cur);
    }
    parts.into_iter().map(|p| p.trim().to_string()).collect()
}

/// `2`, `rest`, `1 per step`, `30 from roland`, `0 when me.is(ada)`.
fn cost(text: &str) -> Value {
    let (core, when) = split_when(text);
    let words: Vec<&str> = core.split_whitespace().collect();
    let mut pairs = vec![(
        "amount".to_string(),
        words.first().map_or(Value::Null, |w| token(w)),
    )];
    for pair in words[words.len().min(1)..].chunks(2) {
        if let [key @ ("per" | "from"), value] = pair {
            pairs.push((key.to_string(), Value::str(*value)));
        }
    }
    if let Some(when) = when {
        pairs.push(("when".to_string(), when));
    }
    Value::Obj(pairs)
}

/// `{target: roland, max: 3}` as an object of tokens.
fn parameters(text: &str) -> Value {
    let inner = text.trim().trim_start_matches('{').trim_end_matches('}');
    Value::Obj(
        inner
            .split(',')
            .filter_map(|pair| pair.split_once(':'))
            .map(|(key, value)| (key.trim().to_string(), token(value.trim())))
            .collect(),
    )
}

/// One effect: `{"do": verb, "args": […], "when": …}`.
///
/// `set` and `say` get named fields, because their arguments are not plain tokens.
pub(crate) fn effect(text: &str) -> Value {
    // `set hask.grounded: yes next round`: a round is a turn.
    if let Some(now) = text.trim().strip_suffix(" next round") {
        let mut e = effect(now);
        if let Value::Obj(pairs) = &mut e {
            pairs.push(("delay".to_string(), Value::Num("1".to_string())));
        }
        return e;
    }
    let (core, when) = split_when(text.trim());
    let verb = first_word(core);
    if STRETCH.contains(&verb) {
        return Value::obj([
            ("stretch", Value::str(verb)),
            ("source", Value::str(text.trim())),
        ]);
    }
    let rest = core[verb.len()..].trim();
    let mut pairs: Vec<(String, Value)> = vec![("do".into(), Value::str(verb))];
    match verb {
        "set" => {
            let (target, value) = rest.split_once(':').unwrap_or((rest, ""));
            // `set x: y until …`, but not an "until" in quoted text.
            let value = value.trim();
            let at = value
                .match_indices(" until ")
                .map(|(i, _)| i)
                .find(|i| value[..*i].matches('"').count() % 2 == 0);
            let (value, until) = match at {
                Some(i) => (&value[..i], &value[i + " until ".len()..]),
                None => (value, ""),
            };
            pairs.push(("target".into(), expression(target)));
            pairs.push(("value".into(), expression(value)));
            if !until.is_empty() {
                pairs.push(("until".into(), Value::str(until.trim())));
            }
        }
        // `roll syndicate where: it.special == chosen_special`: only rows
        // the condition holds for.
        "roll" if rest.contains(" where:") => {
            let (table, cond) = rest.split_once(" where:").unwrap_or((rest, ""));
            pairs.push(("args".into(), Value::Arr(vec![Value::str(table.trim())])));
            pairs.push(("where".into(), expression(cond.trim())));
        }
        // `assert {{ target guilty }} in findings`, `retract {{ sky rains }}`;
        // with no `in`, the world.
        "assert" | "retract" => {
            let (fact, base) = match rest.rsplit_once(" in ") {
                Some((f, b)) if !b.contains("}}") => (f, b.trim()),
                _ => (rest, "world"),
            };
            pairs.push(("fact".into(), fact_value(fact)));
            pairs.push(("in".into(), Value::str(base)));
        }
        // `draw syndicate vampires`: a table's row, chosen (in tests).
        "draw" => {
            let mut w = rest.split_whitespace();
            pairs.push(("table".into(), Value::str(w.next().unwrap_or(""))));
            pairs.push(("row".into(), Value::str(w.next().unwrap_or(""))));
        }
        // `wait until night`, `wait until new_years_eve.nine`.
        "wait" if rest.starts_with("until ") => {
            pairs.push(("until".into(), Value::str(rest["until ".len()..].trim())));
        }
        "say" => {
            pairs.push(("who".into(), Value::str(first_word(rest))));
            pairs.push(("text".into(), quoted_styled(rest).unwrap_or(Value::Null)));
        }
        _ => {
            let mut args = words(rest);
            let before = args.len();
            args.retain(|a| a.as_str() != Some("once"));
            let once = args.len() < before;
            pairs.push(("args".into(), Value::Arr(args)));
            if once {
                pairs.push(("once".into(), Value::Bool(true)));
            }
        }
    }
    pairs.push(("when".into(), when.unwrap_or(Value::Null)));
    Value::Obj(pairs)
}

/// `rule name: body => head ~ in base ~ known ~ learned from fact`.
fn rule(s: &Stmt, rest: &str) -> Value {
    let (name, def) = rest.split_once(':').unwrap_or((rest, ""));
    let (body, head) = def.split_once("=>").unwrap_or((def, ""));
    let mut pairs = vec![
        ("rule".to_string(), Value::str(name.trim())),
        ("body".to_string(), expression(body.trim())),
        ("head".to_string(), fact_value(head.trim())),
    ];
    let mut base = Value::Null;
    let mut known = false;
    let mut learned = Value::Null;
    for c in &s.clauses[1..] {
        let t = c.text.trim();
        if let Some(b) = t.strip_prefix("in ") {
            base = Value::str(b.trim());
        } else if t == "known" {
            known = true;
        } else if let Some(f) = t.strip_prefix("learned from ") {
            learned = Value::str(f.trim());
        }
    }
    pairs.push(("in".into(), base));
    pairs.push(("known".into(), Value::Bool(known)));
    pairs.push(("learned_from".into(), learned));
    Value::Obj(pairs)
}

/// `never body ~ in base ~ on contradiction refuse | flag ~ "why"`.
fn never(s: &Stmt, rest: &str) -> Value {
    let mut base = Value::Null;
    let mut on = Value::str("refuse");
    let mut label = Value::Null;
    for c in &s.clauses[1..] {
        let t = c.text.trim();
        if let Some(b) = t.strip_prefix("in ") {
            base = Value::str(b.trim());
        } else if let Some(how) = t.strip_prefix("on contradiction ") {
            on = Value::str(how.trim());
        } else if t.starts_with('"') {
            label = quoted_styled(t).unwrap_or(Value::str(t));
        }
    }
    Value::obj([
        ("never", expression(rest.trim())),
        ("in", base),
        ("on_contradiction", on),
        ("label", label),
    ])
}

/// `{{ hale is ashe @ monday.evening }}` → `{"subject": "hale", "verb":
/// "is", "object": "ashe", "at": "monday.evening"}`. The subject and object
/// are expressions, worked out when the fact is kept (`{{ target guilty }}`
/// keeps whom `target` names); a capitalised term, `P`, is an Inklog
/// variable, `{"var": "P"}`. The verb is a word. A fact has two terms or
/// three; `object` and `at` are null when it has none.
pub(crate) fn fact_value(text: &str) -> Value {
    match expr::parse(text.trim()) {
        Ok(e @ Expr::Fact { .. }) => {
            let lowered = lower(&e);
            f_get(&lowered, "fact")
        }
        // Not a fact as written (the checker says so): a value that holds
        // one, `assert story in dossier`.
        Ok(e) => Value::obj([("value", lower(&e))]),
        Err((_, why)) => Value::obj([
            ("unparsed", Value::str(text.trim())),
            ("why", Value::str(why)),
        ]),
    }
}

fn f_get(v: &Value, key: &str) -> Value {
    v.get(key).cloned().unwrap_or(Value::Null)
}

/// A `~` clause on a declaration or setting.
fn clause(text: &str) -> Value {
    if let Some(e) = text.strip_prefix("then ") {
        return Value::obj([("then", effect(e))]);
    }
    if text.starts_with('"') {
        return Value::obj([("label", quoted_text(text).unwrap_or(Value::Null))]);
    }
    for lead in ["hidden", "fixed", "offstage"] {
        if let Some(e) = text
            .strip_prefix(lead)
            .and_then(|t| t.strip_prefix(" until "))
        {
            return Value::obj([("clause", Value::str(lead)), ("until", expression(e))]);
        }
    }
    // A table row: `1..=2: then teach blocker the_attack`.
    if text.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
        if let Some((range, row)) = text.split_once(": ") {
            let then = row.trim().strip_prefix("then ").map_or(Value::Null, effect);
            return Value::obj([("row", expression(range)), ("then", then)]);
        }
    }
    // Anything else: the clause as written, and as words a runtime can match
    // on without parsing.
    let (core, when) = split_when(text);
    let mut pairs = vec![
        ("clause".to_string(), Value::str(core)),
        ("words".to_string(), Value::Arr(words(core))),
    ];
    if let Some(when) = when {
        pairs.push(("when".to_string(), when));
    }
    Value::Obj(pairs)
}

/// `period phase: night 1, day 1`, `period season: spring, summer ~ 3
/// each`, `period day: monday, tuesday ~ of watch`: its names, and how
/// long each lasts (`lengths`, one each; `each`, the same for all; `of`,
/// one round of another period). A period the checker refused is lowered
/// as far as it reads.
fn period(s: &Stmt) -> Value {
    let text = s
        .clauses
        .iter()
        .map(|c| c.text.as_str())
        .collect::<Vec<_>>()
        .join(" ~ ");
    let mut pairs = vec![("declare".to_string(), Value::str("period"))];
    match crate::period::Period::parse(&text) {
        Ok(p) => {
            pairs.push(("id".into(), Value::str(&p.name)));
            pairs.push(("names".into(), strings(p.names.iter().map(String::as_str))));
            match p.each {
                crate::period::Each::Own(lens) => pairs.push((
                    "lengths".into(),
                    Value::Arr(lens.into_iter().map(num).collect()),
                )),
                crate::period::Each::Turns(n) => pairs.push(("each".into(), num(n))),
                crate::period::Each::Of(unit) => pairs.push(("of".into(), Value::str(unit))),
            }
        }
        Err(_) => {
            let rest = text.trim_start_matches("period").trim();
            let id = rest.split(':').next().unwrap_or("").trim();
            pairs.push(("id".into(), Value::str(id)));
        }
    }
    Value::Obj(pairs)
}

/// `on [before|after] event [what] [by who] [when …]` and its clauses.
fn handler(rest: &str, clauses: &[Clause]) -> Value {
    let (head, when) = split_when(rest);
    let mut words: Vec<&str> = head.split_whitespace().collect();
    let timing = match words.first() {
        Some(&"before") => words.remove(0),
        Some(&"after") => {
            words.remove(0);
            "after"
        }
        _ => "after",
    };
    let mut event = if words.is_empty() {
        ""
    } else {
        words.remove(0)
    };
    // `on turn ends`: the turn has nothing it happens to.
    if event == "turn" && words.first() == Some(&"ends") {
        words.remove(0);
    }
    // `on turn begins`, `on phase begins night`, `on hour ends eleven`:
    // the event is the beginning or end; the period is said apart.
    let mut period = None;
    if matches!(words.first(), Some(&("begins" | "ends"))) {
        period = Some(event);
        event = words.remove(0);
    }
    // `on absorb with lumen`: what took it.
    if event == "absorb" && words.first() == Some(&"with") {
        words.remove(0);
    }
    let by = words.iter().position(|w| *w == "by").map(|i| {
        let who = words.get(i + 1).copied().unwrap_or("");
        words.truncate(i);
        who
    });
    let mut then = Vec::new();
    let mut once = false;
    let mut at_most = Value::Null;
    let mut tags = Vec::new();
    for c in clauses {
        let text = c.text.as_str();
        if time_effect(then.last_mut(), text) {
            continue;
        }
        if let Some(e) = text.strip_prefix("then ") {
            then.push(effect(e));
        } else if text == "once" {
            once = true;
        } else if let Some(n) = text.strip_prefix("at most ") {
            at_most = token(first_word(n));
        } else {
            tags.push(text);
        }
    }
    let mut pairs = vec![
        ("on", Value::str(event)),
        ("timing", Value::str(timing)),
        ("what", strings(words)),
        ("by", by.map_or(Value::Null, Value::str)),
        ("when", when.unwrap_or(Value::Null)),
        ("then", Value::Arr(then)),
        ("once", Value::Bool(once)),
        ("at_most", at_most),
        ("tags", strings(tags)),
    ];
    if let Some(period) = period {
        pairs.insert(1, ("period", Value::str(period)));
    }
    Value::obj(pairs)
}

/// `item here where: not it.lit` → `{"kind": "item", "here": true, "where": …}`.
pub(crate) fn target_filter(text: &str) -> Value {
    let (head, condition) = match text.split_once("where:") {
        Some((h, c)) => (h.trim(), Some(c.trim())),
        None => (text.trim(), None),
    };
    let mut words = head.split_whitespace();
    let kind = words.next().unwrap_or("");
    let here = words.any(|w| w == "here");
    let mut pairs = vec![
        ("kind".to_string(), Value::str(kind)),
        ("here".to_string(), Value::Bool(here)),
    ];
    if let Some(c) = condition {
        pairs.push(("where".to_string(), expression(c)));
    }
    Value::Obj(pairs)
}

/// `narration: second person ~ omniscient` → `{"person": "second", "omniscient": true}`.
fn narration(s: &Stmt) -> Value {
    let head = s.clauses[0].text["narration".len()..]
        .trim_start_matches(':')
        .trim();
    let person = head.split_whitespace().next().unwrap_or("");
    Value::obj([
        ("person", Value::str(person)),
        (
            "omniscient",
            Value::Bool(s.clauses[1..].iter().any(|c| c.text == "omniscient")),
        ),
    ])
}

/// A `[modifier]` line: `{"type": "modifier", "parts": [{"name": "if", "when": …}, …]}`.
fn modifier(text: &str) -> Value {
    // Strip exactly one bracket from each end: `[if x == [a, b]]` ends in two.
    let text = text.trim();
    let inner = text
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .unwrap_or(text);
    let parts = inner.split(';').map(|part| {
        let part = part.trim();
        let name = first_word(part);
        let rest = part[name.len()..].trim();
        let mut pairs = vec![("name".to_string(), Value::str(name))];
        match name {
            "if" | "unless" | "ifalways" | "ifnever" => {
                pairs.push(("when".into(), expression(rest)))
            }
            "after" | "beat" if !rest.is_empty() => pairs.push(("time".into(), Value::str(rest))),
            "reveal" => pairs.push(("mode".into(), Value::str(rest))),
            // `[if c; then e]`: the effect, done when the block is shown.
            "then" => pairs.push(("effect".into(), effect(rest))),
            // `[quote letter ~ by "A. K."]`.
            "quote" => {
                let (kind, by) = rest.split_once('~').unwrap_or((rest, ""));
                pairs.push(("kind".into(), Value::str(kind.trim())));
                let by = by.trim().strip_prefix("by ").map(str::trim);
                pairs.push((
                    "by".into(),
                    by.and_then(quoted_styled).unwrap_or(Value::Null),
                ));
            }
            // `[redacted until c]`: hidden as a bar while c does not hold.
            "redacted" => pairs.push((
                "until".into(),
                expression(rest.strip_prefix("until").unwrap_or(rest).trim()),
            )),
            _ => {}
        }
        Value::Obj(pairs)
    });
    Value::obj([
        ("type", Value::str("modifier")),
        ("parts", Value::Arr(parts.collect())),
    ])
}

/// A line of prose as parts: plain strings, and one object per `{insert}`.
fn parts(text: &str) -> Value {
    use crate::text::{emphasis, split, Piece};
    let pieces = split(text).unwrap_or_else(|_| vec![Piece::Lit(text.to_string())]);
    let raw: Vec<Value> = pieces
        .into_iter()
        .map(|p| match p {
            Piece::Lit(l) => Value::str(l),
            Piece::Code(code, _) => insert(&code),
        })
        .collect();
    Value::Arr(emphasis(raw))
}

/// One `{insert}`: a named insert, or `{"value": expression}`.
fn insert(content: &str) -> Value {
    let c = content.trim();
    let mut words = c.split_whitespace();
    let first = words.next().unwrap_or("");
    let second = words.next().unwrap_or("");
    let named = |name: &str, rest: &str| {
        let mut pairs = vec![("insert".to_string(), Value::str(name))];
        if !rest.trim().is_empty() {
            pairs.push(("args".to_string(), Value::str(rest.trim())));
        }
        Value::Obj(pairs)
    };
    match first {
        "present" | "echoes" | "claimants" | "passers" | "ballots" | "retold"
            if second.is_empty() =>
        {
            named(first, "")
        }
        // `{present linker}`: the name of whoever is reading.
        "present" if second == "linker" => named("present linker", ""),
        "embed" if second == "nib:" => Value::obj([
            ("insert", Value::str("embed")),
            ("nib", Value::str(c.split('"').nth(1).unwrap_or(""))),
        ]),
        "list" => {
            let options = c.split('~').skip(1).map(str::trim);
            let head = c.split('~').next().unwrap_or("");
            let mut pairs = vec![
                ("insert".to_string(), Value::str("list")),
                ("list".to_string(), Value::str(second)),
                ("options".to_string(), strings(options)),
            ];
            // `{list hand of each guest}`: a pile, everyone's.
            if let Some(kind) = head.split_once(" of each ").map(|(_, k)| k.trim()) {
                pairs.push(("of_each".to_string(), Value::str(kind)));
            }
            Value::Obj(pairs)
        }
        // `{reveal link: "Look closer", text: "…"}` → `{"link": …, "text": …}`.
        "embed" | "reveal" | "sound" | "ambient" if second.ends_with(':') => Value::obj([
            (
                "insert",
                Value::str(format!("{first} {}", second.trim_end_matches(':'))),
            ),
            ("args", insert_args(&c[first.len()..])),
        ]),
        // `{redacted "TEN" until telegram.dry}`: text shown as a bar until c.
        "redacted" => {
            let text = c["redacted".len()..].trim_start();
            let end = crate::text::string_end(text, 0).unwrap_or(text.len().saturating_sub(1));
            let inner = text.get(1..end).unwrap_or("");
            let until = text[end + 1..].trim().trim_start_matches("until").trim();
            Value::obj([
                ("insert", Value::str("redacted")),
                ("text", parts(inner)),
                ("until", expression(until)),
            ])
        }
        // The jury, a ballot, and the inputs take typed arguments.
        "accuse" | "ballot" => Value::obj([
            ("insert", Value::str(first)),
            ("args", insert_args(&c[first.len()..])),
        ]),
        _ if second == "input" => Value::obj([
            ("insert", Value::str(format!("{first} input"))),
            (
                "args",
                insert_args(c.split_once("input").map_or("", |(_, r)| r)),
            ),
        ]),
        // `{all leo.aka}`: every name, not one picked.
        "all" if !second.is_empty() => Value::obj([
            ("value", expression(&c[first.len()..])),
            ("all", Value::Bool(true)),
        ]),
        _ => Value::obj([("value", expression(c))]),
    }
}
