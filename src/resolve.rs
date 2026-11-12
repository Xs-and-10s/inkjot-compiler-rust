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
pub const FUNCTIONS: &[&str] = &["count", "one", "alone_with", "nearest"];

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
    "yes",
    "no",
    "nobody",
    "nothing",
    "hushed",
    // v0.6 stretch: the duel.
    "cast",
    "absorbed",
    "inquest",
];

/// Every name in `e` that does not resolve.
pub fn unresolved(e: &Expr, lookup: &impl Lookup) -> Vec<Problem> {
    let mut out = Vec::new();
    walk(e, lookup, &mut out);
    out
}

fn walk(e: &Expr, lookup: &impl Lookup, out: &mut Vec<Problem>) {
    match e {
        Expr::Num(_) | Expr::Str(_) => {}
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
            // `who.knows.x` wants a clue; `who.holds.x` wants an item.
            // `who.holds.any(tag)` is a call, not an item named `any`.
            if let Expr::Member(_, verb) = target.as_ref() {
                let kind = match verb.as_str() {
                    "knows" => Some("clue"),
                    "holds" if member != "any" => Some("item"),
                    _ => None,
                };
                if let Some(kind) = kind {
                    if !lookup.is_kind(member, kind) {
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
                walk(&arg.value, lookup, out);
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
            matches!((name, kind), ("secret", "clue") | ("key", "item"))
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
