//! Values while a story runs.
//!
//! The IR is JSON, but a running story needs a little more than JSON: it has
//! to tell the *name* of a thing (`rook`, an id) apart from a piece of text
//! (`"Rook"`), and it compares ranges. So the interpreter works in its own
//! small value type, [`V`].

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum V {
    /// `nothing`, `nobody`, or a value that was never set.
    Null,
    Bool(bool),
    Num(f64),
    /// Text: a quoted string in the source, or a label.
    Str(String),
    /// The id of something declared, or a bare word used as a value: `rook`, `box`.
    Id(String),
    List(Vec<V>),
    /// A range: low and high bounds, either of which may be open.
    Range {
        lo: Option<f64>,
        hi: Option<f64>,
        inclusive: bool,
    },
}

impl V {
    /// Inkjot's idea of true: anything but `no`, `nothing`, `0`, `""` and `[]`.
    pub fn truthy(&self) -> bool {
        match self {
            V::Null => false,
            V::Bool(b) => *b,
            V::Num(n) => *n != 0.0,
            V::Str(s) => !s.is_empty(),
            V::Id(_) => true,
            V::List(items) => !items.is_empty(),
            V::Range { .. } => true,
        }
    }

    pub fn num(&self) -> Option<f64> {
        match self {
            V::Num(n) => Some(*n),
            V::Bool(b) => Some(f64::from(u8::from(*b))),
            _ => None,
        }
    }

    /// The id or text inside, for values that are names.
    pub fn name(&self) -> Option<&str> {
        match self {
            V::Id(s) | V::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Equality as Inkjot means it: an id equals text with the same letters,
    /// so `me.grievance == box` holds after `set me.grievance: box`.
    pub fn same(&self, other: &V) -> bool {
        match (self, other) {
            (V::Id(a) | V::Str(a), V::Id(b) | V::Str(b)) => a == b,
            (V::Num(a), V::Num(b)) => a == b,
            (V::Bool(a), V::Bool(b)) => a == b,
            (V::Null, V::Null) => true,
            (V::List(a), V::List(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.same(y))
            }
            (a, b) => a == b,
        }
    }

    /// Is `self` inside the range or list `within`?
    pub fn within(&self, within: &V) -> bool {
        match within {
            V::List(items) => items.iter().any(|i| i.same(self)),
            V::Range { lo, hi, inclusive } => match self.num() {
                Some(n) => {
                    lo.is_none_or(|lo| n >= lo)
                        && hi.is_none_or(|hi| if *inclusive { n <= hi } else { n < hi })
                }
                None => false,
            },
            _ => false,
        }
    }
}

/// How a value reads in running text.
impl fmt::Display for V {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            V::Null => Ok(()),
            V::Bool(true) => write!(f, "yes"),
            V::Bool(false) => write!(f, "no"),
            // Whole numbers print without a decimal point.
            V::Num(n) if n.fract() == 0.0 && n.abs() < 1e15 => write!(f, "{}", *n as i64),
            V::Num(n) => write!(f, "{n}"),
            V::Str(s) | V::Id(s) => write!(f, "{s}"),
            V::List(items) => {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                Ok(())
            }
            V::Range { lo, hi, inclusive } => {
                let bound = |b: &Option<f64>| b.map(|n| V::Num(n).to_string()).unwrap_or_default();
                write!(
                    f,
                    "{}{}{}",
                    bound(lo),
                    if *inclusive { "..=" } else { "..<" },
                    bound(hi)
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_text_compare_by_letters() {
        assert!(V::Id("box".into()).same(&V::Str("box".into())));
        assert!(!V::Id("box".into()).same(&V::Id("bag".into())));
    }

    #[test]
    fn ranges_respect_their_bounds() {
        let r = V::Range {
            lo: Some(2.0),
            hi: Some(4.0),
            inclusive: true,
        };
        assert!(V::Num(4.0).within(&r));
        let r = V::Range {
            lo: Some(2.0),
            hi: Some(4.0),
            inclusive: false,
        };
        assert!(!V::Num(4.0).within(&r));
        assert!(V::Num(100.0).within(&V::Range {
            lo: Some(3.0),
            hi: None,
            inclusive: false
        }));
    }

    #[test]
    fn whole_numbers_print_plainly() {
        assert_eq!(V::Num(3.0).to_string(), "3");
        assert_eq!(V::Num(1.5).to_string(), "1.5");
    }
}
