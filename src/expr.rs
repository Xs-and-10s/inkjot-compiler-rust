//! Inkjot expressions: what follows `when`, sits inside `[if …]`, or is a `let` value.
//!
//! Precedence, loosest first: `or`; `and` / `but`; `not`; comparisons and `in`;
//! ranges; `+ -`; `* / mod`; then member access, calls and indexing.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(String),
    /// Text with nothing to work out: `"a key"`.
    Str(String),
    /// Text with braces in it: `"Keeper of {here}"`.
    Text(Vec<TextPart>),
    /// `if cond then a else b`.
    If(Box<Expr>, Box<Expr>, Box<Expr>),
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
            Expr::If(..) => 0,
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
            Expr::Str(s) => write!(f, "\"{}\"", crate::text::escape(s)),
            Expr::Text(parts) => {
                write!(f, "\"")?;
                for part in parts {
                    match part {
                        TextPart::Lit(s) => write!(f, "{}", crate::text::escape(s))?,
                        TextPart::Expr(e) => write!(f, "{{{e}}}")?,
                    }
                }
                write!(f, "\"")
            }
            Expr::If(c, a, b) => {
                write!(f, "if ")?;
                c.fmt_at(f, 1)?;
                write!(f, " then ")?;
                a.fmt_at(f, 1)?;
                write!(f, " else ")?;
                b.fmt_at(f, 1)
            }
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
                } else if *op == "top" {
                    write!(f, "top of ")?;
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

/// A piece of interpolated text: words, or an expression in braces.
#[derive(Debug, Clone, PartialEq)]
pub enum TextPart {
    Lit(String),
    Expr(Expr),
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
            // `1'000'000`: a `'` between digits separates thousands.
            let digit_or_mark = |i: usize| {
                b[i].is_ascii_digit()
                    || (b[i] == b'\'' && i + 1 < b.len() && b[i + 1].is_ascii_digit())
            };
            while i < b.len() && digit_or_mark(i) {
                i += 1;
            }
            // A decimal point only if a digit follows, so `2..=4` stays a range.
            if i + 1 < b.len() && b[i] == b'.' && b[i + 1].is_ascii_digit() {
                i += 1;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            out.push((start, Tok::Num(src[start..i].replace('\'', ""))));
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
            // Braces in text may hold strings of their own, so the end is
            // found by the text rules, not the next quote.
            let Some(end) = crate::text::string_end(src, i) else {
                return Err((start, "this string is never closed".into()));
            };
            out.push((start, Tok::Str(src[start + 1..end].to_string())));
            i = end + 1;
        } else if let Some(op) = OPS.iter().find(|op| src[i..].starts_with(**op)) {
            i += op.len();
            out.push((start, Tok::Op(op)));
        } else if c == b'\'' {
            return Err((start, "text takes double quotes: \"like this\"".into()));
        } else if c == b'%' {
            return Err((
                start,
                "a remainder is written `mod`: `clock.now mod 4 == 0`".into(),
            ));
        } else {
            let ch = src[i..].chars().next().unwrap_or('?');
            return Err((start, format!("unexpected `{ch}` in an expression")));
        }
    }
    Ok(out)
}

/// The inside of a quoted string, `offset` bytes into the source: plain
/// text, or text with expressions in braces.
fn text(raw: &str, offset: usize) -> Result<Expr, ParseError> {
    use crate::text::{plain, split, Piece};
    let pieces = split(raw).map_err(|(at, why)| (offset + at, why))?;
    if pieces.iter().all(|p| matches!(p, Piece::Lit(_))) {
        let s: String = pieces
            .iter()
            .map(|p| match p {
                Piece::Lit(l) => plain(l),
                Piece::Code(..) => String::new(),
            })
            .collect();
        return Ok(Expr::Str(s));
    }
    let mut parts = Vec::new();
    for p in pieces {
        match p {
            Piece::Lit(l) => parts.push(TextPart::Lit(plain(&l))),
            Piece::Code(code, at) => {
                let e = parse(&code).map_err(|(i, why)| (offset + at + i, why))?;
                parts.push(TextPart::Expr(e));
            }
        }
    }
    Ok(Expr::Text(parts))
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

    fn expect_word(&mut self, w: &str) -> Result<(), ParseError> {
        match self.peek() {
            Some(Tok::Ident(s)) if s == w => {
                self.pos += 1;
                Ok(())
            }
            _ => Err((self.at(), format!("expected `{w}` in `if … then … else …`"))),
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
        if let Some(q) = self.quantifier() {
            return self.nested(|p| p.quantified(q));
        }
        self.cmp()
    }

    /// `no guest where: …`, `some guest where: …`, `every guest where: …`:
    /// the quantifier, if one starts here.
    fn quantifier(&self) -> Option<&'static str> {
        let q = match self.peek() {
            Some(Tok::Ident(s)) if s == "no" => "no",
            Some(Tok::Ident(s)) if s == "some" => "some",
            Some(Tok::Ident(s)) if s == "every" => "every",
            _ => return None,
        };
        let kind = matches!(self.toks.get(self.pos + 1), Some((_, Tok::Ident(_))));
        let where_ =
            matches!(self.toks.get(self.pos + 2), Some((_, Tok::Ident(w))) if w == "where");
        let colon = matches!(self.toks.get(self.pos + 3), Some((_, Tok::Op(":"))));
        (kind && where_ && colon).then_some(q)
    }

    /// A quantifier over a kind, as the count it stands for: `no guest
    /// where: c` is `count(guest, where: c) == 0`; `some`, `> 0`; `every`,
    /// none where it does not hold. The condition runs to the end.
    fn quantified(&mut self, q: &'static str) -> Result<Expr, ParseError> {
        let at = self.toks[self.pos + 1].0;
        let Some((_, Tok::Ident(kind))) = self.toks.get(self.pos + 1).cloned() else {
            unreachable!("checked by quantifier()")
        };
        self.pos += 4;
        let cond = self.or()?;
        let cond = if q == "every" {
            Expr::Unary("not", Box::new(cond))
        } else {
            cond
        };
        let count = Expr::Call(
            Box::new(Expr::name("count")),
            vec![
                Arg {
                    key: None,
                    value: Expr::Name(kind, At(at)),
                },
                Arg {
                    key: Some("where".into()),
                    value: cond,
                },
            ],
        );
        let (op, n) = if q == "some" { (">", "0") } else { ("==", "0") };
        Ok(Expr::Binary(
            op,
            Box::new(count),
            Box::new(Expr::Num(n.into())),
        ))
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
            Some(Tok::Ident(s)) => !WORD_OPS.contains(&s.as_str()) && s != "then" && s != "else",
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
        // `top of my hand`: the first card of a pile, or of any list.
        if matches!(self.peek(), Some(Tok::Ident(s)) if s == "top")
            && matches!(self.toks.get(self.pos + 1), Some((_, Tok::Ident(s))) if s == "of")
        {
            self.pos += 2;
            return self.nested(|p| Ok(Expr::Unary("top", Box::new(p.unary()?))));
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
                        // `i.know.x`, `i.hold.key`, `i.am(rook)`: the first
                        // person's verbs are the same as `knows`, `holds`, `is`.
                        let s = match s.as_str() {
                            "know" => "knows".to_string(),
                            "hold" => "holds".to_string(),
                            "am" | "are" => "is".to_string(),
                            _ => s,
                        };
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
                text(&s, at + 1)
            }
            Some(Tok::Ident(s)) if s == "if" => {
                self.pos += 1;
                self.nested(|p| {
                    let cond = p.or()?;
                    p.expect_word("then")?;
                    let yes = p.or()?;
                    p.expect_word("else")?;
                    let no = p.or()?;
                    Ok(Expr::If(Box::new(cond), Box::new(yes), Box::new(no)))
                })
            }
            Some(Tok::Ident(s)) => {
                if WORD_OPS.contains(&s.as_str()) {
                    return Err((at, format!("`{s}` needs something before it")));
                }
                self.pos += 1;
                // `my hand` is `me.hand`.
                if s == "my" {
                    if let Some(Tok::Ident(m)) = self.peek().cloned() {
                        if !WORD_OPS.contains(&m.as_str()) {
                            self.pos += 1;
                            let me = Expr::Name("me".to_string(), At(at));
                            return Ok(Expr::Member(Box::new(me), m));
                        }
                    }
                }
                // `i`, `you`, `myself` and `yourself` are the pov, as `me` is.
                let s = if crate::syntax::POV_NAMES.contains(&s.as_str()) {
                    "me".to_string()
                } else {
                    s
                };
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
