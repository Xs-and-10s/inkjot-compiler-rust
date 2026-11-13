//! A small JSON value: enough to read `StoryData` and to write the IR.
//!
//! Objects keep their keys in insertion order, so the same source always
//! produces byte-for-byte the same output.

use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    /// A number, kept as the text it was written with.
    Num(String),
    Str(String),
    Arr(Vec<Value>),
    Obj(Vec<(String, Value)>),
}

impl Value {
    pub fn str(s: impl Into<String>) -> Value {
        Value::Str(s.into())
    }

    /// An object from `(key, value)` pairs, in the order given.
    pub fn obj<K: Into<String>>(pairs: impl IntoIterator<Item = (K, Value)>) -> Value {
        Value::Obj(pairs.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// The value under `key`, if this is an object that has it.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The value as a whole number, if it is one.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Num(n) => n.parse().ok(),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[Value]> {
        match self {
            Value::Arr(items) => Some(items),
            _ => None,
        }
    }

    /// Does this value hold no arrays or objects of its own?
    fn is_flat(&self) -> bool {
        match self {
            Value::Arr(items) => items
                .iter()
                .all(|v| !matches!(v, Value::Arr(_) | Value::Obj(_))),
            Value::Obj(pairs) => pairs
                .iter()
                .all(|(_, v)| !matches!(v, Value::Arr(_) | Value::Obj(_))),
            _ => true,
        }
    }

    fn write(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        let pad = "  ".repeat(indent + 1);
        let close = "  ".repeat(indent);
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Num(n) => write!(f, "{n}"),
            Value::Str(s) => write_string(f, s),
            Value::Arr(items) if items.is_empty() => write!(f, "[]"),
            Value::Obj(pairs) if pairs.is_empty() => write!(f, "{{}}"),
            // Small, flat containers go on one line; everything else is indented.
            Value::Arr(items) if self.is_flat() => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    item.write(f, indent)?;
                }
                write!(f, "]")
            }
            Value::Obj(pairs) if self.is_flat() && pairs.len() <= 3 => {
                write!(f, "{{")?;
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write_string(f, key)?;
                    write!(f, ": ")?;
                    value.write(f, indent)?;
                }
                write!(f, "}}")
            }
            Value::Arr(items) => {
                writeln!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    write!(f, "{pad}")?;
                    item.write(f, indent + 1)?;
                    writeln!(f, "{}", if i + 1 < items.len() { "," } else { "" })?;
                }
                write!(f, "{close}]")
            }
            Value::Obj(pairs) => {
                writeln!(f, "{{")?;
                for (i, (key, value)) in pairs.iter().enumerate() {
                    write!(f, "{pad}")?;
                    write_string(f, key)?;
                    write!(f, ": ")?;
                    value.write(f, indent + 1)?;
                    writeln!(f, "{}", if i + 1 < pairs.len() { "," } else { "" })?;
                }
                write!(f, "{close}}}")
            }
        }
    }
}

/// Pretty-printed JSON.
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write(f, 0)
    }
}

fn write_string(f: &mut fmt::Formatter<'_>, s: &str) -> fmt::Result {
    write!(f, "\"")?;
    for ch in s.chars() {
        match ch {
            '"' => write!(f, "\\\"")?,
            '\\' => write!(f, "\\\\")?,
            '\n' => write!(f, "\\n")?,
            '\r' => write!(f, "\\r")?,
            '\t' => write!(f, "\\t")?,
            c if u32::from(c) < 0x20 => write!(f, "\\u{:04x}", u32::from(c))?,
            c => write!(f, "{c}")?,
        }
    }
    write!(f, "\"")
}

/// Parse a JSON text.
pub fn parse(src: &str) -> Result<Value, String> {
    let mut p = Parser {
        chars: src.chars().collect(),
        pos: 0,
    };
    let value = p.value(0)?;
    p.skip_space();
    if p.pos != p.chars.len() {
        return Err("unexpected text after the end of the JSON value".into());
    }
    Ok(value)
}

/// The top-level string fields of a JSON object, as `StoryData` needs them.
pub fn top_level_strings(src: &str) -> Result<HashMap<String, String>, String> {
    match parse(src)? {
        Value::Obj(pairs) => Ok(pairs
            .into_iter()
            .filter_map(|(k, v)| match v {
                Value::Str(s) => Some((k, s)),
                _ => None,
            })
            .collect()),
        _ => Err("expected a JSON object: `{ … }`".into()),
    }
}

/// How deeply values may nest. Without a limit, a file of ten thousand `[`
/// would overflow the stack.
const MAX_DEPTH: usize = 64;

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek();
        self.pos += 1;
        c
    }

    fn skip_space(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, want: char) -> Result<(), String> {
        match self.next() {
            Some(c) if c == want => Ok(()),
            _ => Err(format!("expected `{want}`")),
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err("the JSON is nested too deeply".into());
        }
        self.skip_space();
        match self.peek() {
            Some('"') => self.string().map(Value::Str),
            Some('{') => {
                self.pos += 1;
                let mut pairs = Vec::new();
                self.skip_space();
                if self.peek() == Some('}') {
                    self.pos += 1;
                    return Ok(Value::Obj(pairs));
                }
                loop {
                    self.skip_space();
                    let key = self.string()?;
                    self.skip_space();
                    self.expect(':')?;
                    pairs.push((key, self.value(depth + 1)?));
                    self.skip_space();
                    match self.next() {
                        Some(',') => continue,
                        Some('}') => return Ok(Value::Obj(pairs)),
                        _ => return Err("expected `,` or `}`".into()),
                    }
                }
            }
            Some('[') => {
                self.pos += 1;
                let mut items = Vec::new();
                self.skip_space();
                if self.peek() == Some(']') {
                    self.pos += 1;
                    return Ok(Value::Arr(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    self.skip_space();
                    match self.next() {
                        Some(',') => continue,
                        Some(']') => return Ok(Value::Arr(items)),
                        _ => return Err("expected `,` or `]`".into()),
                    }
                }
            }
            Some(c) if c == '-' || c.is_ascii_digit() => {
                let start = self.pos;
                while self
                    .peek()
                    .is_some_and(|c| matches!(c, '-' | '+' | '.' | 'e' | 'E') || c.is_ascii_digit())
                {
                    self.pos += 1;
                }
                Ok(Value::Num(self.chars[start..self.pos].iter().collect()))
            }
            _ => {
                for (word, value) in [
                    ("true", Value::Bool(true)),
                    ("false", Value::Bool(false)),
                    ("null", Value::Null),
                ] {
                    let end = self.pos + word.len();
                    if self
                        .chars
                        .get(self.pos..end)
                        .is_some_and(|cs| cs.iter().copied().eq(word.chars()))
                    {
                        self.pos = end;
                        return Ok(value);
                    }
                }
                Err("expected a JSON value".into())
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.next() {
                None => return Err("unterminated string".into()),
                Some('"') => return Ok(out),
                Some('\\') => match self.next() {
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some(c @ ('"' | '\\' | '/')) => out.push(c),
                    Some('u') => {
                        let hex: String = (0..4).filter_map(|_| self.next()).collect();
                        let code = u32::from_str_radix(&hex, 16)
                            .map_err(|_| "bad \\u escape in string")?;
                        out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                    }
                    _ => return Err("bad escape in string".into()),
                },
                Some(c) => out.push(c),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_and_reads_back() {
        let v = Value::obj([
            ("name", Value::str("a \"quoted\" line\nwith a tab\t")),
            ("n", Value::Num("12".into())),
            ("flags", Value::Arr(vec![Value::Bool(true), Value::Null])),
            (
                "nested",
                Value::obj([(
                    "list",
                    Value::Arr(vec![Value::obj([("k", Value::str("v"))])]),
                )]),
            ),
        ]);
        assert_eq!(parse(&v.to_string()), Ok(v));
    }

    #[test]
    fn rejects_bad_json() {
        for src in ["", "{", "{\"a\" 1}", "[1,]", "nul", "\"open", "{} extra"] {
            assert!(parse(src).is_err(), "{src} should not parse");
        }
    }

    #[test]
    fn deep_nesting_is_an_error_not_a_crash() {
        assert!(parse(&"[".repeat(10_000)).is_err());
    }
}
