//! Inkjot expressions: what follows `when`, sits inside `[if …]`, or is a `let` value.
//!
//! Precedence, loosest first: `or`; `and` / `but`; `not`; comparisons and `in`;
//! ranges; `+ -`; `* / mod`; then member access, calls and indexing.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(String),
    Str(String),
    /// A bare name, and where it starts in the source text.
    Name(String, At),
    Member(Box<Expr>, String),
    Call(Box<Expr>, Vec<Arg>),
    Index(Box<Expr>, Box<Expr>),
    List(Vec<Expr>),
    Unary(&'static str, Box<Expr>),
    Binary(&'static str, Box<Expr>, Box<Expr>),
    Range {
        lo: Option<Box<Expr>>,
        hi: Option<Box<Expr>>,
        op: &'static str,
    },
}

/// A byte offset into the source text.
///
/// Two `At`s always compare equal. That keeps positions out of the meaning of
/// an expression: `a and b` parsed from two different places is the same tree,
/// and a printed-then-reparsed expression equals the original.
#[derive(Debug, Clone, Copy, Default)]
pub struct At(pub usize);

impl PartialEq for At {
    fn eq(&self, _: &At) -> bool {
        true
    }
}

impl Expr {
    /// A bare name with no position, for building trees by hand.
    pub fn name(name: impl Into<String>) -> Expr {
        Expr::Name(name.into(), At::default())
    }

    /// How tightly this expression binds; a higher number binds tighter.
    /// The printer uses it to decide where brackets are needed.
    fn precedence(&self) -> u8 {
        match self {
            Expr::Binary(op, ..) => match *op {
                "or" => 1,
                "and" => 2,
                "==" | "!=" | ">=" | "<=" | ">" | "<" | "in" => 4,
                "+" | "-" => 6,
                _ => 7, // * / mod
            },
            Expr::Unary("not", _) => 3,
            Expr::Range { .. } => 5,
            Expr::Unary(..) => 8, // unary minus
            _ => 9,               // names, literals, calls, members, indexing, lists
        }
    }

    /// Write `self`, in brackets if it binds more loosely than `min`.
    fn fmt_at(&self, f: &mut fmt::Formatter<'_>, min: u8) -> fmt::Result {
        if self.precedence() < min {
            write!(f, "({self})")
        } else {
            write!(f, "{self}")
        }
    }
}

/// Print an expression as Inkjot source, with only the brackets it needs.
///
/// The contract, checked by a property test: parsing the printed text gives
/// back an equal tree.
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mine = self.precedence();
        match self {
            Expr::Num(n) => write!(f, "{n}"),
            Expr::Str(s) => write!(f, "\"{s}\""),
            Expr::Name(n, _) => write!(f, "{n}"),
            Expr::Member(target, member) => {
                target.fmt_at(f, 9)?;
                write!(f, ".{member}")
            }
            Expr::Call(target, args) => {
                target.fmt_at(f, 9)?;
                write!(f, "(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if let Some(key) = &arg.key {
                        write!(f, "{key}: ")?;
                    }
                    write!(f, "{}", arg.value)?;
                }
                write!(f, ")")
            }
            Expr::Index(target, index) => {
                target.fmt_at(f, 9)?;
                write!(f, "[{index}]")
            }
            Expr::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            Expr::Unary(op, operand) => {
                if *op == "not" {
                    write!(f, "not ")?;
                } else {
                    write!(f, "{op}")?;
                }
                operand.fmt_at(f, mine)
            }
            // Binary operators group to the left: `a - b - c` is `(a - b) - c`.
            // So the left side may bind as loosely as this operator, but the
            // right side must bind tighter or it needs brackets.
            Expr::Binary(op, lhs, rhs) => {
                lhs.fmt_at(f, mine)?;
                write!(f, " {op} ")?;
                rhs.fmt_at(f, mine + 1)
            }
            Expr::Range { lo, hi, op } => {
                if let Some(lo) = lo {
                    lo.fmt_at(f, mine + 1)?;
                }
                write!(f, "{op}")?;
                if let Some(hi) = hi {
                    hi.fmt_at(f, mine + 1)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    pub key: Option<String>,
    pub value: Expr,
}

/// A parse failure: byte offset into the source text, and what went wrong.
pub type ParseError = (usize, String);

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(String),
    Str(String),
    Ident(String),
    Op(&'static str),
}

const OPS: &[&str] = &[
    "..=", "..<", "..", "==", "!=", ">=", "<=", ">", "<", "+", "-", "*", "/", "(", ")", "[", "]",
    ",", ".", ":",
];

fn lex(src: &str) -> Result<Vec<(usize, Tok)>, ParseError> {
    let b = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        if c.is_ascii_digit() {
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            // A decimal point only if a digit follows, so `2..=4` stays a range.
            if i + 1 < b.len() && b[i] == b'.' && b[i + 1].is_ascii_digit() {
                i += 1;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            out.push((start, Tok::Num(src[start..i].to_string())));
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            // Cell names: `lounge@0`.
            if i + 1 < b.len() && b[i] == b'@' && b[i + 1].is_ascii_alphanumeric() {
                i += 1;
                while i < b.len() && b[i].is_ascii_alphanumeric() {
                    i += 1;
                }
            }
            out.push((start, Tok::Ident(src[start..i].to_string())));
        } else if c == b'"' {
            i += 1;
            while i < b.len() && b[i] != b'"' {
                i += 1;
            }
            if i >= b.len() {
                return Err((start, "this string is never closed".into()));
            }
            out.push((start, Tok::Str(src[start + 1..i].to_string())));
            i += 1;
        } else if let Some(op) = OPS.iter().find(|op| src[i..].starts_with(**op)) {
            i += op.len();
            out.push((start, Tok::Op(op)));
        } else {
            let ch = src[i..].chars().next().unwrap_or('?');
            return Err((start, format!("unexpected `{ch}` in an expression")));
        }
    }
    Ok(out)
}

pub fn parse(src: &str) -> Result<Expr, ParseError> {
    let toks = lex(src)?;
    if toks.is_empty() {
        return Err((0, "expected an expression".into()));
    }
    let mut p = Parser {
        toks,
        pos: 0,
        len: src.len(),
        depth: 0,
    };
    let e = p.or()?;
    if let Some((at, tok)) = p.toks.get(p.pos) {
        return Err((*at, format!("unexpected {} here", describe(tok))));
    }
    Ok(e)
}

fn describe(t: &Tok) -> String {
    match t {
        Tok::Num(n) => format!("`{n}`"),
        Tok::Str(_) => "a string".into(),
        Tok::Ident(s) => format!("`{s}`"),
        Tok::Op(o) => format!("`{o}`"),
    }
}

/// A range with an upper bound must say whether the bound is included.
const BARE_RANGE: &str = "write `..=` to include the upper bound or `..<` to exclude it; a bare `..` is only for an open range, `a..`";

const WORD_OPS: &[&str] = &["and", "or", "but", "not", "in", "mod"];

/// How deeply brackets and prefix operators may nest. Each level is a real
/// function call here, so without a limit ten thousand `(` would overflow
/// the stack, and a stack overflow cannot be caught and reported.
const MAX_DEPTH: usize = 64;

struct Parser {
    toks: Vec<(usize, Tok)>,
    pos: usize,
    len: usize,
    depth: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.1)
    }
    fn at(&self) -> usize {
        self.toks.get(self.pos).map_or(self.len, |t| t.0)
    }
    fn word(&self, w: &str) -> bool {
        matches!(self.peek(), Some(Tok::Ident(s)) if s == w)
    }
    fn op(&self, o: &str) -> bool {
        matches!(self.peek(), Some(Tok::Op(s)) if *s == o)
    }
    fn eat_op(&mut self, o: &str) -> bool {
        if self.op(o) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn expect_op(&mut self, o: &str, why: &str) -> Result<(), ParseError> {
        if self.eat_op(o) {
            Ok(())
        } else {
            Err((self.at(), format!("expected `{o}` {why}")))
        }
    }

    /// Run `f` one nesting level deeper, refusing to go past `MAX_DEPTH`.
    fn nested(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<Expr, ParseError>,
    ) -> Result<Expr, ParseError> {
        if self.depth >= MAX_DEPTH {
            return Err((self.at(), "this expression is nested too deeply".into()));
        }
        self.depth += 1;
        let result = f(self);
        self.depth -= 1;
        result
    }

    /// Every bracket, call argument and index re-enters here, so this is
    /// where nesting is counted.
    fn or(&mut self) -> Result<Expr, ParseError> {
        self.nested(|p| {
            let mut lhs = p.and()?;
            while p.word("or") {
                p.pos += 1;
                lhs = Expr::Binary("or", Box::new(lhs), Box::new(p.and()?));
            }
            Ok(lhs)
        })
    }

    fn and(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.not()?;
        // `but` is an exact synonym for `and`.
        while self.word("and") || self.word("but") {
            self.pos += 1;
            lhs = Expr::Binary("and", Box::new(lhs), Box::new(self.not()?));
        }
        Ok(lhs)
    }

    fn not(&mut self) -> Result<Expr, ParseError> {
        if self.word("not") {
            self.pos += 1;
            return self.nested(|p| Ok(Expr::Unary("not", Box::new(p.not()?))));
        }
        self.cmp()
    }

    fn cmp(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.range()?;
        loop {
            let op: &'static str = match self.peek() {
                Some(Tok::Op(o)) if ["==", "!=", ">=", "<=", ">", "<"].contains(o) => o,
                Some(Tok::Ident(s)) if s == "in" => "in",
                _ => break,
            };
            self.pos += 1;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(self.range()?));
        }
        Ok(lhs)
    }

    fn range_op(&self) -> Option<&'static str> {
        match self.peek() {
            Some(Tok::Op(o)) if ["..=", "..<", ".."].contains(o) => Some(o),
            _ => None,
        }
    }

    fn starts_operand(&self) -> bool {
        match self.peek() {
            Some(Tok::Num(_)) | Some(Tok::Str(_)) => true,
            Some(Tok::Ident(s)) => !WORD_OPS.contains(&s.as_str()),
            Some(Tok::Op(o)) => *o == "(" || *o == "[" || *o == "-",
            None => false,
        }
    }

    fn range(&mut self) -> Result<Expr, ParseError> {
        if let Some(op) = self.range_op() {
            let at = self.at();
            self.pos += 1;
            let hi = self.add()?;
            if op == ".." {
                return Err((at, BARE_RANGE.into()));
            }
            return Ok(Expr::Range {
                lo: None,
                hi: Some(Box::new(hi)),
                op,
            });
        }
        let lo = self.add()?;
        if let Some(op) = self.range_op() {
            let at = self.at();
            self.pos += 1;
            let hi = if self.starts_operand() {
                Some(Box::new(self.add()?))
            } else {
                None
            };
            if hi.is_none() && op != ".." {
                return Err((
                    self.at(),
                    format!("`{op}` needs an upper bound; an open range is written `a..`"),
                ));
            }
            if hi.is_some() && op == ".." {
                return Err((at, BARE_RANGE.into()));
            }
            return Ok(Expr::Range {
                lo: Some(Box::new(lo)),
                hi,
                op,
            });
        }
        Ok(lo)
    }

    fn add(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.mul()?;
        loop {
            let op: &'static str = if self.op("+") {
                "+"
            } else if self.op("-") {
                "-"
            } else {
                break;
            };
            self.pos += 1;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(self.mul()?));
        }
        Ok(lhs)
    }

    fn mul(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.unary()?;
        loop {
            let op: &'static str = if self.op("*") {
                "*"
            } else if self.op("/") {
                "/"
            } else if self.word("mod") {
                "mod"
            } else {
                break;
            };
            self.pos += 1;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(self.unary()?));
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.eat_op("-") {
            return self.nested(|p| Ok(Expr::Unary("-", Box::new(p.unary()?))));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, ParseError> {
        let mut e = self.primary()?;
        loop {
            if self.eat_op(".") {
                match self.peek().cloned() {
                    // After a dot any word is a member, keywords included: `here.in(x)`.
                    Some(Tok::Ident(s)) => {
                        self.pos += 1;
                        e = Expr::Member(Box::new(e), s);
                    }
                    _ => return Err((self.at(), "expected a name after `.`".into())),
                }
            } else if self.eat_op("(") {
                let mut args = Vec::new();
                if !self.eat_op(")") {
                    loop {
                        args.push(self.arg()?);
                        if self.eat_op(",") {
                            continue;
                        }
                        self.expect_op(")", "to close the call")?;
                        break;
                    }
                }
                e = Expr::Call(Box::new(e), args);
            } else if self.eat_op("[") {
                let idx = self.or()?;
                self.expect_op("]", "to close the index")?;
                e = Expr::Index(Box::new(e), Box::new(idx));
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn arg(&mut self) -> Result<Arg, ParseError> {
        // `key: value`, where the key may be any word (`in: here`).
        if let (Some((_, Tok::Ident(k))), Some((_, Tok::Op(":")))) =
            (self.toks.get(self.pos), self.toks.get(self.pos + 1))
        {
            let key = k.clone();
            self.pos += 2;
            return Ok(Arg {
                key: Some(key),
                value: self.or()?,
            });
        }
        Ok(Arg {
            key: None,
            value: self.or()?,
        })
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        let at = self.at();
        match self.peek().cloned() {
            Some(Tok::Num(n)) => {
                self.pos += 1;
                Ok(Expr::Num(n))
            }
            Some(Tok::Str(s)) => {
                self.pos += 1;
                Ok(Expr::Str(s))
            }
            Some(Tok::Ident(s)) => {
                if WORD_OPS.contains(&s.as_str()) {
                    return Err((at, format!("`{s}` needs something before it")));
                }
                self.pos += 1;
                Ok(Expr::Name(s, At(at)))
            }
            Some(Tok::Op("(")) => {
                self.pos += 1;
                let e = self.or()?;
                self.expect_op(")", "to close the bracket")?;
                Ok(e)
            }
            Some(Tok::Op("[")) => {
                self.pos += 1;
                let mut items = Vec::new();
                if !self.eat_op("]") {
                    loop {
                        items.push(self.or()?);
                        if self.eat_op(",") {
                            continue;
                        }
                        self.expect_op("]", "to close the list")?;
                        break;
                    }
                }
                Ok(Expr::List(items))
            }
            Some(t) => Err((at, format!("unexpected {} here", describe(&t)))),
            None => Err((at, "the expression stops too soon".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_shapes_the_examples_use() {
        for src in [
            "me.is(rook)",
            "clock.now in 2..=4",
            "count(linker, in: here) >= 2",
            "not (glyph.broken or glyph.frozen)",
            "mouth_words == [gust, spark, hush]",
            "me.name != \"\"",
            "hask.grounded and me.holds.any(sword) and not eldric.frozen",
            "player.has(magic) but not door.open",
            "hours[orrery.ember]",
            "self.saw(dash, toward: self.heading)",
            "3..",
            "..=2",
            "orrery.ember + 1 mod 6",
        ] {
            assert!(parse(src).is_ok(), "{src}: {:?}", parse(src));
        }
    }

    #[test]
    fn but_means_and() {
        assert_eq!(parse("a but b"), parse("a and b"));
    }

    #[test]
    fn rejects_broken_expressions() {
        for src in [
            "",
            "me.is(rook",
            "a ==",
            "and b",
            "a b",
            "x..=",
            "a.(b)",
            "1..5",
            "..5",
        ] {
            assert!(parse(src).is_err(), "{src} should not parse");
        }
    }
}

#[cfg(test)]
mod depth_tests {
    use super::parse;

    #[test]
    fn deep_nesting_is_an_error_not_a_crash() {
        for src in [
            "(".repeat(10_000),
            "not ".repeat(10_000),
            "-".repeat(10_000),
            "[".repeat(10_000),
        ] {
            assert!(parse(&src).is_err());
        }
    }

    #[test]
    fn ordinary_nesting_is_fine() {
        let src = format!("{}x{}", "(".repeat(20), ")".repeat(20));
        assert!(parse(&src).is_ok());
    }
}
