//! Just enough JSON to read `StoryData`: validates the text and returns the
//! top-level string fields.

use std::collections::HashMap;

pub fn top_level_strings(src: &str) -> Result<HashMap<String, String>, String> {
    let mut p = P {
        s: src.as_bytes(),
        i: 0,
    };
    p.ws();
    let mut out = HashMap::new();
    p.expect(b'{')?;
    p.ws();
    if p.peek() == Some(b'}') {
        p.i += 1;
    } else {
        loop {
            p.ws();
            let key = p.string()?;
            p.ws();
            p.expect(b':')?;
            p.ws();
            if p.peek() == Some(b'"') {
                let v = p.string()?;
                out.insert(key, v);
            } else {
                p.value()?;
            }
            p.ws();
            match p.next() {
                Some(b',') => continue,
                Some(b'}') => break,
                _ => return Err("expected `,` or `}`".into()),
            }
        }
    }
    p.ws();
    if p.i != p.s.len() {
        return Err("unexpected text after the closing `}`".into());
    }
    Ok(out)
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }
    fn next(&mut self) -> Option<u8> {
        let c = self.peek();
        self.i += 1;
        c
    }
    fn ws(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.i += 1;
        }
    }
    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.next() == Some(c) {
            Ok(())
        } else {
            Err(format!("expected `{}`", c as char))
        }
    }
    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = Vec::new();
        loop {
            match self.next() {
                None => return Err("unterminated string".into()),
                Some(b'"') => break,
                Some(b'\\') => match self.next() {
                    Some(b'n') => out.push(b'\n'),
                    Some(b't') => out.push(b'\t'),
                    Some(c @ (b'"' | b'\\' | b'/')) => out.push(c),
                    Some(b'u') => {
                        self.i += 4;
                        out.push(b'?');
                    }
                    _ => return Err("bad escape in string".into()),
                },
                Some(c) => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|_| "string is not valid UTF-8".to_string())
    }
    fn value(&mut self) -> Result<(), String> {
        self.ws();
        match self.peek() {
            Some(b'"') => self.string().map(|_| ()),
            Some(b'{') => {
                self.i += 1;
                self.ws();
                if self.peek() == Some(b'}') {
                    self.i += 1;
                    return Ok(());
                }
                loop {
                    self.ws();
                    self.string()?;
                    self.ws();
                    self.expect(b':')?;
                    self.value()?;
                    self.ws();
                    match self.next() {
                        Some(b',') => continue,
                        Some(b'}') => return Ok(()),
                        _ => return Err("expected `,` or `}`".into()),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                self.ws();
                if self.peek() == Some(b']') {
                    self.i += 1;
                    return Ok(());
                }
                loop {
                    self.value()?;
                    self.ws();
                    match self.next() {
                        Some(b',') => continue,
                        Some(b']') => return Ok(()),
                        _ => return Err("expected `,` or `]`".into()),
                    }
                }
            }
            Some(c) if c == b'-' || c.is_ascii_digit() => {
                while self.peek().is_some_and(|c| {
                    matches!(c, b'-' | b'+' | b'.' | b'e' | b'E') || c.is_ascii_digit()
                }) {
                    self.i += 1;
                }
                Ok(())
            }
            _ => {
                for word in ["true", "false", "null"] {
                    if self.s[self.i..].starts_with(word.as_bytes()) {
                        self.i += word.len();
                        return Ok(());
                    }
                }
                Err("expected a JSON value".into())
            }
        }
    }
}
