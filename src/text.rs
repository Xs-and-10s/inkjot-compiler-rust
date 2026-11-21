//! Text: what is written for the Linker to read, in prose and in quotes.
//!
//! ```text
//! The hall is quiet. {present} are here.                 prose: inserts
//! say leo "Who's hungry? Is it you, {jojo.aka}?"          quoted text: the same braces
//! "Nell kneels to {if lied then "a spirit" else "a monster"}."
//! A *thin*, **glassy** burn. Literal: \{ \} \* \" \\
//! ```
//!
//! Every piece of text uses the same braces: `{…}` holds an insert or an
//! expression, in prose and in every quoted string alike. `\{` and `\}` are
//! literal braces, `\"` a quote inside quoted text, `\\` a backslash, and
//! `\*` an asterisk that is not emphasis. Quotes nest inside braces, so a
//! braced expression may hold strings of its own. `{{` is Inklog's and is
//! refused in text.
//!
//! Emphasis (`*italic*`, `**bold**`, `***both***`) is lowered here too, for
//! prose and Link labels: a run of asterisks opens when text follows it and
//! closes when text comes before it, and pairs with the nearest open run of
//! the same length. An unmatched run stays literal. Emphasis never crosses a
//! paragraph, and underscores are always literal.

use crate::json::Value;

/// A piece of text: literal words, or the code inside a pair of braces.
#[derive(Debug, Clone, PartialEq)]
pub enum Piece {
    /// Literal text, unescaped, except that an escaped asterisk (`\*`) is
    /// [`STAR`], for emphasis to leave alone; [`plain`] turns it back.
    Lit(String),
    /// The inside of `{…}`, as written, and its byte offset in the text.
    Code(String, usize),
}

/// An asterisk that was escaped, `\*`: a character from Unicode's private
/// use area, so it can never be confused with one the Inker typed.
pub const STAR: char = '\u{E02A}';

/// Where a quoted string that opens at `open` (a `"`) closes: the index of
/// its closing quote. Escapes are skipped, and quotes inside braces belong
/// to the braced code.
pub fn string_end(s: &str, open: usize) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = open + 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'"' => return Some(i),
            b'{' => i = code_end(s, i)? + 1,
            _ => i += 1,
        }
    }
    None
}

/// Where the code that opens at `open` (a `{`) closes: the index of its `}`.
/// Strings inside the code are skipped whole.
fn code_end(s: &str, open: usize) -> Option<usize> {
    let b = s.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    while i < b.len() {
        match b[i] {
            b'"' => i = string_end(s, i)?,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Split text into literal pieces and braced code. An error is the byte
/// offset of the problem and what is wrong.
pub fn split(s: &str) -> Result<Vec<Piece>, (usize, String)> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut lit = String::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' if i + 1 < b.len() => {
                let next = s[i + 1..].chars().next().unwrap_or('\\');
                match next {
                    '{' | '}' | '"' | '\\' => lit.push(next),
                    // Marked for emphasis to leave alone.
                    '*' => lit.push(STAR),
                    other => {
                        lit.push('\\');
                        lit.push(other);
                    }
                }
                i += 1 + next.len_utf8();
            }
            b'{' => {
                if b.get(i + 1) == Some(&b'{') {
                    return Err((
                        i,
                        "`{{ … }}` is Inklog's, and is not text; write `\\{` for a brace".into(),
                    ));
                }
                let Some(close) = code_end(s, i) else {
                    return Err((
                        i,
                        "this `{` is never closed; write `\\{` for a brace".into(),
                    ));
                };
                if !lit.is_empty() {
                    out.push(Piece::Lit(std::mem::take(&mut lit)));
                }
                out.push(Piece::Code(s[i + 1..close].to_string(), i + 1));
                i = close + 1;
            }
            b'}' => {
                return Err((i, "this `}` closes nothing; write `\\}` for a brace".into()));
            }
            _ => {
                let ch = s[i..].chars().next().unwrap_or(' ');
                lit.push(ch);
                i += ch.len_utf8();
            }
        }
    }
    if !lit.is_empty() {
        out.push(Piece::Lit(lit));
    }
    Ok(out)
}

/// Literal text with emphasis escapes resolved: an escaped asterisk is an
/// asterisk again.
pub fn plain(lit: &str) -> String {
    lit.replace(STAR, "*")
}

/// Text written back as source: braces, quotes and backslashes escaped.
pub fn escape(lit: &str) -> String {
    let mut out = String::new();
    for ch in lit.chars() {
        if matches!(ch, '{' | '}' | '"' | '\\') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

// ------------------------------------------------------------ emphasis

/// What emphasis sees: words, a run of asterisks, or a part it does not
/// look inside (an insert).
enum Tok {
    Text(String),
    Stars { n: usize, opens: bool, closes: bool },
    Part(Value),
}

/// Lower emphasis over a paragraph's parts (strings and inserts), giving
/// parts where styled runs are `{"style": ["italic"], "parts": […]}`.
pub fn emphasis(parts: Vec<Value>) -> Vec<Value> {
    let mut toks: Vec<Tok> = Vec::new();
    for part in parts {
        match part {
            Value::Str(s) => toks.extend(tokens(&s)),
            other => toks.push(Tok::Part(other)),
        }
    }
    // Which runs flank text: a run opens when something other than space
    // follows it, and closes when something other than space precedes it.
    let n = toks.len();
    for i in 0..n {
        let before = i.checked_sub(1).map(|j| solid_end(&toks[j]));
        let after = toks.get(i + 1).map(solid_start);
        if let Tok::Stars { opens, closes, .. } = &mut toks[i] {
            *opens = after.unwrap_or(false);
            *closes = before.unwrap_or(false);
        }
    }
    // Pair runs: a closing run takes the nearest open run of its length.
    let mut open: Vec<usize> = Vec::new();
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for i in 0..n {
        if let Tok::Stars {
            n: len,
            opens,
            closes,
        } = toks[i]
        {
            if closes {
                if let Some(at) = open.iter().rposition(|&j| stars(&toks[j]) == len) {
                    pairs.push((open[at], i));
                    open.truncate(at);
                    continue;
                }
            }
            if opens {
                open.push(i);
            }
        }
    }
    build(&toks, 0, n, &pairs)
}

fn stars(t: &Tok) -> usize {
    match t {
        Tok::Stars { n, .. } => *n,
        _ => 0,
    }
}

fn solid_start(t: &Tok) -> bool {
    match t {
        Tok::Text(s) => s.chars().next().is_some_and(|c| !c.is_whitespace()),
        Tok::Part(_) => true,
        Tok::Stars { .. } => false,
    }
}

fn solid_end(t: &Tok) -> bool {
    match t {
        Tok::Text(s) => s.chars().last().is_some_and(|c| !c.is_whitespace()),
        Tok::Part(_) => true,
        Tok::Stars { .. } => false,
    }
}

/// Split a string into words and runs of asterisks, with `\*` literal.
fn tokens(s: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut text = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            STAR => text.push('*'),
            '*' => {
                let mut n = 1;
                while chars.peek() == Some(&'*') {
                    chars.next();
                    n += 1;
                }
                if !text.is_empty() {
                    out.push(Tok::Text(std::mem::take(&mut text)));
                }
                out.push(Tok::Stars {
                    n,
                    opens: false,
                    closes: false,
                });
            }
            other => text.push(other),
        }
    }
    if !text.is_empty() {
        out.push(Tok::Text(text));
    }
    out
}

/// The parts of `toks[from..to]`, with each matched pair a styled run.
fn build(toks: &[Tok], from: usize, to: usize, pairs: &[(usize, usize)]) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    let mut i = from;
    while i < to {
        if let Some(&(_, close)) = pairs.iter().find(|(o, _)| *o == i) {
            let style = match stars(&toks[i]) {
                1 => vec!["italic"],
                2 => vec!["bold"],
                _ => vec!["bold", "italic"],
            };
            let inner = build(toks, i + 1, close, pairs);
            out.push(Value::obj([
                (
                    "style",
                    Value::Arr(style.into_iter().map(Value::str).collect()),
                ),
                ("parts", Value::Arr(inner)),
            ]));
            i = close + 1;
            continue;
        }
        match &toks[i] {
            Tok::Text(s) => push_str(&mut out, s),
            Tok::Stars { n, .. } => push_str(&mut out, &"*".repeat(*n)),
            Tok::Part(v) => out.push(v.clone()),
        }
        i += 1;
    }
    out
}

/// Append text, joining it to a string just before.
fn push_str(out: &mut Vec<Value>, s: &str) {
    if let Some(Value::Str(last)) = out.last_mut() {
        last.push_str(s);
    } else {
        out.push(Value::str(s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json;
    use proptest::prelude::*;

    fn em(s: &str) -> String {
        Value::Arr(emphasis(vec![Value::str(s)])).to_string()
    }

    fn j(s: &str) -> String {
        json::parse(s).unwrap().to_string()
    }

    #[test]
    fn asterisks_become_styles() {
        assert_eq!(
            em("a *b* c"),
            j(r#"["a ", {"style": ["italic"], "parts": ["b"]}, " c"]"#)
        );
        assert_eq!(em("**b**"), j(r#"[{"style": ["bold"], "parts": ["b"]}]"#));
        assert_eq!(
            em("***b***"),
            j(r#"[{"style": ["bold", "italic"], "parts": ["b"]}]"#)
        );
        assert_eq!(
            em("**a *b* c**"),
            j(
                r#"[{"style": ["bold"], "parts": ["a ", {"style": ["italic"], "parts": ["b"]}, " c"]}]"#
            )
        );
    }

    #[test]
    fn what_does_not_pair_is_literal() {
        assert_eq!(em("3 * 4"), j(r#"["3 * 4"]"#));
        assert_eq!(em("*open"), j(r#"["*open"]"#));
        let escaped: String = split(r"\*not\*")
            .unwrap()
            .into_iter()
            .map(|p| match p {
                Piece::Lit(l) => l,
                Piece::Code(..) => String::new(),
            })
            .collect();
        assert_eq!(em(&escaped), j(r#"["*not*"]"#));
        assert_eq!(em("_under_"), j(r#"["_under_"]"#));
        assert_eq!(em("**a*"), j(r#"["**a*"]"#));
    }

    #[test]
    fn emphasis_wraps_an_insert() {
        let insert = json::parse(r#"{"value": {"get": ["leo", "aka"]}}"#).unwrap();
        let out = emphasis(vec![Value::str("Is it you, *"), insert, Value::str("*?")]);
        assert_eq!(
            Value::Arr(out).to_string(),
            j(
                r#"["Is it you, ", {"style": ["italic"], "parts": [{"value": {"get": ["leo", "aka"]}}]}, "?"]"#
            )
        );
    }

    #[test]
    fn braces_are_code_and_escapes_are_literal() {
        assert_eq!(
            split(r#"Keeper of {here}, \{not\} "q" \\"#).unwrap(),
            vec![
                Piece::Lit("Keeper of ".into()),
                Piece::Code("here".into(), 11),
                Piece::Lit(r#", {not} "q" \"#.into()),
            ]
        );
        let nested = r#"to {if lied then "a spirit {x}" else "a monster"}."#;
        let pieces = split(nested).unwrap();
        assert_eq!(pieces.len(), 3);
        assert!(matches!(&pieces[1], Piece::Code(c, _) if c.starts_with("if lied")));
    }

    #[test]
    fn bad_braces_are_reported() {
        assert!(split("a { b").unwrap_err().1.contains("never closed"));
        assert!(split("a } b").unwrap_err().1.contains("closes nothing"));
        assert!(split("{{ a b }}").unwrap_err().1.contains("Inklog"));
    }

    #[test]
    fn a_string_ends_after_its_braces() {
        let s = r#""a {if x then "b" else "c"} d" ~ tail"#;
        assert_eq!(string_end(s, 0), Some(s.find(" ~").unwrap() - 1));
        assert_eq!(string_end(r#""a \" b" c"#, 0), Some(7));
    }

    proptest! {
        /// Escaping any text and splitting it back gives the text, whole.
        #[test]
        fn escape_then_split_gives_the_text(s in "[a-z {}\"\\\\*]{0,24}") {
            let back = split(&escape(&s)).unwrap();
            let lit: String = back
                .iter()
                .map(|p| match p {
                    Piece::Lit(l) => plain(l),
                    Piece::Code(..) => panic!("code in escaped text"),
                })
                .collect();
            prop_assert_eq!(lit, s);
        }

        /// Emphasis never loses or invents a character.
        #[test]
        fn emphasis_keeps_every_character(s in "[a-z *]{0,24}") {
            fn flat(v: &Value, out: &mut String) {
                match v {
                    Value::Str(s) => out.push_str(s),
                    Value::Arr(items) => items.iter().for_each(|i| flat(i, out)),
                    Value::Obj(_) => {
                        let style = v.get("style").and_then(Value::as_arr).unwrap_or(&[]);
                        let n = match style {
                            [one] if one.as_str() == Some("bold") => 2,
                            [_] => 1,
                            _ => 3,
                        };
                        let stars = "*".repeat(n);
                        out.push_str(&stars);
                        flat(v.get("parts").unwrap(), out);
                        out.push_str(&stars);
                    }
                    _ => {}
                }
            }
            let mut out = String::new();
            flat(&Value::Arr(emphasis(vec![Value::str(&s)])), &mut out);
            prop_assert_eq!(out, s);
        }
    }
}
