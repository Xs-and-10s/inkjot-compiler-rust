//! Inkjot expressions: what follows `when`, sits inside `[if …]`, or is a `let` value.
//!
//! Precedence, loosest first: `or`; `and` / `but`; `not`; comparisons and `in`;
//! ranges; `+ -`; `* / mod`; then member access, calls and indexing.

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(String),
    Str(String),
    Name(String),
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

const WORD_OPS: &[&str] = &["and", "or", "but", "not", "in", "mod"];

struct Parser {
    toks: Vec<(usize, Tok)>,
    pos: usize,
    len: usize,
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

    fn or(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.and()?;
        while self.word("or") {
            self.pos += 1;
            lhs = Expr::Binary("or", Box::new(lhs), Box::new(self.and()?));
        }
        Ok(lhs)
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
            return Ok(Expr::Unary("not", Box::new(self.not()?)));
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
            self.pos += 1;
            let hi = self.add()?;
            return Ok(Expr::Range {
                lo: None,
                hi: Some(Box::new(hi)),
                op,
            });
        }
        let lo = self.add()?;
        if let Some(op) = self.range_op() {
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
            return Ok(Expr::Unary("-", Box::new(self.unary()?)));
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
                Ok(Expr::Name(s))
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
        for src in ["", "me.is(rook", "a ==", "and b", "a b", "x..=", "a.(b)"] {
            assert!(parse(src).is_err(), "{src} should not parse");
        }
    }
}
