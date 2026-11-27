//! Name resolution: does every name in an expression refer to something?
//!
//! The parser only checks shape. This pass walks the tree and reports names
//! that are not builtins, declared ids, tags, or `let`s in scope.

use crate::expr::Expr;

/// What the resolver needs to know about the world. The checker implements
/// this; a test can implement it with a couple of hard-coded sets.
pub trait Lookup {
    /// Is `name` usable as a value here: a builtin, a declared id, a tag, or a local?
    fn is_known(&self, name: &str) -> bool;
    /// Is `name` declared as this kind of thing (`"clue"`, `"item"`)?
    fn is_kind(&self, name: &str, kind: &str) -> bool;
}

/// What a name was expected to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Want {
    /// Any known name.
    Value,
    /// A function that can be called: `count(…)`.
    Function,
    /// A declared thing of one kind: the `x` in `me.knows.x` must be a clue.
    Kind(&'static str),
}

/// A name that does not resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// Byte offset of the expression's nearest named root.
    pub at: usize,
    pub name: String,
    pub want: Want,
}

/// Functions that may be called by bare name.
pub const FUNCTIONS: &[&str] = &[
    "count",
    "one",
    "all",
    "alone_with",
    "nearest",
    // `max(1, players / 4)`.
    "max",
    "min",
    // `first(victim.chosen_by)`, `last(targets)`: the ends of a list.
    "first",
    "last",
    // Helpers for text.
    "plural",
    "capital",
    // `possessive(victim)`: "Ada’s".
    "possessive",
];

/// Names every Inklink can use without declaring them.
pub const BUILTINS: &[&str] = &[
    "me",
    "here",
    "self",
    "actor",
    "attempt",
    "clock",
    "cohort",
    "pov",
    "scenario",
    "accusation",
    // `ballot.done`: the table's last `{ballot}`.
    "ballot",
    // What a Link with `~ at` was taken on, in its effects and its passage.
    "target",
    // What an `on` handler's event happened to: `on enter shaft when entered.burnt`.
    "entered",
    "left",
    "crossed",
    "moved",
    "taken",
    // In a `[resolve]` passage: `threat.name`, `threat.target`, `threat.outcome`.
    "threat",
    "yes",
    "no",
    "nobody",
    "nothing",
    // v0.6 stretch: the duel.
    "cast",
    "absorbed",
    "hinted",
];

/// Every bare name an expression reads, in order. `a.b(c)` reads `a` and `c`.
pub fn names(e: &Expr) -> Vec<String> {
    let mut out = Vec::new();
    collect(e, &mut out);
    out
}

fn collect(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Num(_) | Expr::Str(_) => {}
        Expr::Text(parts) => parts.iter().for_each(|p| {
            if let crate::expr::TextPart::Expr(e) = p {
                collect(e, out)
            }
        }),
        Expr::If(c, a, b) => [c, a, b].into_iter().for_each(|e| collect(e, out)),
        Expr::Name(name, _) => out.push(name.clone()),
        Expr::Member(target, _) => collect(target, out),
        Expr::Call(target, args) => {
            collect(target, out);
            args.iter().for_each(|a| collect(&a.value, out));
        }
        Expr::Index(a, b) | Expr::Binary(_, a, b) => {
            collect(a, out);
            collect(b, out);
        }
        Expr::List(items) => items.iter().for_each(|i| collect(i, out)),
        Expr::Unary(_, a) => collect(a, out),
        Expr::Range { lo, hi, .. } => [lo, hi].into_iter().flatten().for_each(|b| collect(b, out)),
        Expr::Var(_) | Expr::Any => {}
        Expr::Fact {
            subject, object, ..
        } => {
            for t in std::iter::once(subject).chain(object) {
                collect(t, out);
            }
        }
        Expr::Quant(_, _, body) => collect(body, out),
    }
}

/// The Inklog variables an expression uses, in order, once each.
pub fn vars(e: &Expr) -> Vec<String> {
    fn go(e: &Expr, out: &mut Vec<String>) {
        match e {
            Expr::Var(v) if !out.contains(v) => out.push(v.clone()),
            Expr::Fact {
                subject,
                object,
                at,
                ..
            } => {
                for t in std::iter::once(subject).chain(object).chain(at) {
                    go(t, out);
                }
            }
            Expr::Binary(_, a, b) | Expr::Index(a, b) => {
                go(a, out);
                go(b, out);
            }
            Expr::Unary(_, a) | Expr::Member(a, _) => go(a, out),
            Expr::Quant(_, _, body) => go(body, out),
            Expr::Call(t, args) => {
                go(t, out);
                args.iter().for_each(|a| go(&a.value, out));
            }
            Expr::List(items) => items.iter().for_each(|i| go(i, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    go(e, &mut out);
    out
}

/// Every name in `e` that does not resolve.
pub fn unresolved(e: &Expr, lookup: &impl Lookup) -> Vec<Problem> {
    let mut out = Vec::new();
    walk(e, lookup, &mut out);
    out
}

/// Inside a `where:` filter, `it` is each candidate in turn.
struct WithIt<'a>(&'a dyn Lookup);

impl Lookup for WithIt<'_> {
    fn is_known(&self, name: &str) -> bool {
        name == "it" || self.0.is_known(name)
    }
    fn is_kind(&self, name: &str, kind: &str) -> bool {
        self.0.is_kind(name, kind)
    }
}

/// Every name in a `where:` condition that does not resolve, `it` known.
pub fn unresolved_where(e: &Expr, lookup: &impl Lookup) -> Vec<Problem> {
    let mut out = Vec::new();
    walk(e, &WithIt(lookup), &mut out);
    out
}

fn walk(e: &Expr, lookup: &dyn Lookup, out: &mut Vec<Problem>) {
    match e {
        Expr::Num(_) | Expr::Str(_) => {}
        Expr::Text(parts) => parts.iter().for_each(|p| {
            if let crate::expr::TextPart::Expr(e) = p {
                walk(e, lookup, out)
            }
        }),
        Expr::If(c, a, b) => [c, a, b].into_iter().for_each(|e| walk(e, lookup, out)),
        Expr::Name(name, at) => {
            if !lookup.is_known(name) {
                out.push(Problem {
                    at: at.0,
                    name: name.clone(),
                    want: Want::Value,
                });
            }
        }
        Expr::Member(target, member) => {
            // `who.knows.x` wants a clue, or an item (holding something
            // teaches you it); `who.holds.x` wants an item.
            // `who.holds.any(tag)` is a call, not an item named `any`.
            if let Expr::Member(_, verb) = target.as_ref() {
                let kind = match verb.as_str() {
                    "knows" => Some("fact"),
                    "holds" if member != "any" => Some("item"),
                    _ => None,
                };
                if let Some(kind) = kind {
                    let known_item = kind == "fact" && lookup.is_kind(member, "item");
                    if !lookup.is_kind(member, kind) && !known_item {
                        out.push(Problem {
                            at: root(e),
                            name: member.clone(),
                            want: Want::Kind(kind),
                        });
                    }
                }
            }
            walk(target, lookup, out);
        }
        Expr::Call(target, args) => {
            match target.as_ref() {
                Expr::Name(name, at) => {
                    if !FUNCTIONS.contains(&name.as_str()) {
                        out.push(Problem {
                            at: at.0,
                            name: name.clone(),
                            want: Want::Function,
                        });
                    }
                }
                other => walk(other, lookup, out),
            }
            for arg in args {
                if arg.key.as_deref() == Some("where") {
                    walk(&arg.value, &WithIt(lookup), out);
                } else {
                    walk(&arg.value, lookup, out);
                }
            }
        }
        Expr::Index(target, index) => {
            walk(target, lookup, out);
            walk(index, lookup, out);
        }
        Expr::List(items) => items.iter().for_each(|item| walk(item, lookup, out)),
        Expr::Unary(_, operand) => walk(operand, lookup, out),
        Expr::Binary(op, lhs, rhs) => {
            walk(lhs, lookup, out);
            // `me.treat == hold`: the right side of a comparison may be a bare
            // word that is simply a value, declared nowhere. That is allowed
            // unless the left side is a literal, where it would mean nothing.
            // `{{ … }} in dossier`: a knowledge base.
            if *op == "in" && matches!(lhs.as_ref(), Expr::Fact { .. }) {
                walk(lhs, lookup, out);
                if let Expr::Name(base, at) = rhs.as_ref() {
                    if !lookup.is_kind(base, "knowledge") {
                        out.push(Problem {
                            at: at.0,
                            name: base.clone(),
                            want: Want::Kind("knowledge"),
                        });
                    }
                }
                return;
            }
            let compares = matches!(*op, "==" | "!=" | "in");
            let lhs_is_literal = matches!(lhs.as_ref(), Expr::Num(_) | Expr::Str(_));
            if !(compares && !lhs_is_literal && is_symbolic(rhs)) {
                walk(rhs, lookup, out);
            }
        }
        Expr::Range { lo, hi, .. } => {
            for bound in [lo, hi].into_iter().flatten() {
                walk(bound, lookup, out);
            }
        }
        Expr::Var(_) | Expr::Any => {}
        // A fact's terms are atoms: a bare name is itself (`{{ sky rains }}`)
        // unless it is in scope (`target`, `you`). Only what is worked out
        // (`target.owner`) must resolve.
        Expr::Fact {
            subject, object, ..
        } => {
            for t in std::iter::once(subject).chain(object) {
                if !matches!(t.as_ref(), Expr::Name(..) | Expr::Var(_) | Expr::Any) {
                    walk(t, lookup, out);
                }
            }
        }
        Expr::Quant(_, _, body) => walk(body, lookup, out),
    }
}

/// A bare word, or a list of bare words: `hold`, `[flame, frost]`.
fn is_symbolic(e: &Expr) -> bool {
    match e {
        Expr::Name(..) => true,
        Expr::List(items) => !items.is_empty() && items.iter().all(|i| matches!(i, Expr::Name(..))),
        _ => false,
    }
}

/// Where the leftmost name of a member/call/index chain starts.
fn root(e: &Expr) -> usize {
    match e {
        Expr::Name(_, at) => at.0,
        Expr::Member(target, _) | Expr::Call(target, _) | Expr::Index(target, _) => root(target),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::parse;

    /// A tiny world: two known names, one clue, one item.
    struct World;
    impl Lookup for World {
        fn is_known(&self, name: &str) -> bool {
            BUILTINS.contains(&name) || ["door", "secret", "key", "evidence"].contains(&name)
        }
        fn is_kind(&self, name: &str, kind: &str) -> bool {
            matches!((name, kind), ("secret", "fact") | ("key", "item"))
        }
    }

    fn names(src: &str) -> Vec<String> {
        unresolved(&parse(src).unwrap(), &World)
            .into_iter()
            .map(|p| p.name)
            .collect()
    }

    #[test]
    fn known_names_resolve() {
        for src in [
            "door.open and not me.knows.secret",
            "me.holds.key or me.holds.any(evidence)",
            "count(evidence, in: here) >= 1",
            "me.treat == hold",
            "next_spell in [flame, frost] or door.open",
        ] {
            let expected: Vec<String> = if src.starts_with("next_spell") {
                vec!["next_spell".into()]
            } else {
                vec![]
            };
            assert_eq!(names(src), expected, "{src}");
        }
    }

    #[test]
    fn unknown_names_are_reported() {
        assert_eq!(names("dor.open"), ["dor"]);
        assert_eq!(names("me.knows.secrt"), ["secrt"]);
        assert_eq!(names("me.holds.kee"), ["kee"]);
        assert_eq!(names("cont(evidence)"), ["cont"]);
        assert_eq!(names("count(evidnce)"), ["evidnce"]);
        assert_eq!(names("3 == hold"), ["hold"]);
    }

    #[test]
    fn the_position_is_the_name() {
        let src = "door.open and dor.open";
        let p = &unresolved(&parse(src).unwrap(), &World)[0];
        assert_eq!(&src[p.at..p.at + 3], "dor");
    }
}
