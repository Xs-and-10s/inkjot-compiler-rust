//! The Twee 3 container: split a `.twee` file into passages, and take out
//! the comments.
//!
//! Comments are for the Inker and never reach the player:
//!
//! ```text
//! :: some_act [act]
//! (((
//! A block comment straight after the `::` line is the passage's doc
//! comment: Inkwell shows it, and the IR keeps it as `doc`.
//! )))
//! ((A line comment: the whole line.))
//! @jojo (growling) "Put him on the list." ~ ((alt: "Welcome to the list."))
//! ```
//!
//! `((` starts a comment only at the start of a line or straight after a
//! `~`, so `not ((a or b) and c)` and parentheses in prose are left alone.
//! A `~ ((` comment runs to the end of its line.

use crate::diag::Diag;

#[derive(Debug, Clone)]
pub struct Line {
    /// 1-based line number in the file.
    pub no: usize,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Passage {
    pub name: String,
    pub tags: Vec<String>,
    /// Line of the `::` header.
    pub line: usize,
    /// 1-based character column where the name starts on that line.
    pub col: usize,
    pub lines: Vec<Line>,
    /// A `(((…)))` block straight after the `::` line.
    pub doc: Option<String>,
}

impl Passage {
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

pub fn parse(src: &str, diags: &mut Vec<Diag>) -> Vec<Passage> {
    let mut passages: Vec<Passage> = Vec::new();
    for (i, raw) in src.lines().enumerate() {
        let no = i + 1;
        let text = raw.trim_end_matches('\r');
        if let Some(rest) = text.strip_prefix("::") {
            let (name, tags) = parse_header(rest, no, diags);
            // Point at the name itself; with no name, at the end of the line.
            let lead = rest.len() - rest.trim_start().len();
            let col = (3 + rest[..lead].chars().count()).min(text.chars().count() + 1);
            passages.push(Passage {
                name,
                tags,
                line: no,
                col,
                lines: Vec::new(),
                doc: None,
            });
        } else if let Some(p) = passages.last_mut() {
            p.lines.push(Line {
                no,
                text: text.to_string(),
            });
        }
        // Text before the first passage is ignored, as Twee 3 specifies.
    }
    for p in &mut passages {
        strip_comments(p, diags);
        while p.lines.last().is_some_and(|l| l.text.trim().is_empty()) {
            p.lines.pop();
        }
    }
    passages
}

/// Take the comments out of a passage's lines; keep a doc comment.
fn strip_comments(p: &mut Passage, diags: &mut Vec<Diag>) {
    let mut kept = Vec::new();
    // While inside a `(((` block: where it began, its text, and whether it
    // came before anything else in the passage.
    let mut block: Option<(usize, Vec<String>, bool)> = None;
    for line in std::mem::take(&mut p.lines) {
        let t = line.text.trim();
        if let Some((_, text, _)) = block.as_mut() {
            match t.strip_suffix(")))") {
                Some(last) => {
                    text.push(last.to_string());
                    let (_, text, first) = block.take().unwrap_or_default();
                    if first {
                        p.doc = Some(text.join("\n").trim().to_string());
                    }
                }
                None => text.push(line.text.clone()),
            }
            continue;
        }
        if let Some(open) = t.strip_prefix("(((") {
            let first = kept.iter().all(|l: &Line| l.text.trim().is_empty());
            match open.strip_suffix(")))") {
                Some(one) if first => p.doc = Some(one.trim().to_string()),
                Some(_) => {}
                None => block = Some((line.no, vec![open.to_string()], first)),
            }
            continue;
        }
        if t.starts_with("((") && t.ends_with("))") {
            continue;
        }
        kept.push(Line {
            no: line.no,
            text: cut_trailing_comment(&line.text).to_string(),
        });
    }
    if let Some((no, _, _)) = block {
        diags.push(Diag::error(
            no,
            1,
            "this `(((` comment is never closed with `)))`",
        ));
    }
    p.lines = kept;
}

/// A line without its `~ ((…))` comment, if it ends with one.
fn cut_trailing_comment(text: &str) -> &str {
    let mask = crate::syntax::top_mask(text);
    for (i, ch) in text.char_indices() {
        if ch != '~' || !mask[i] {
            continue;
        }
        let after = text[i + 1..].trim_start();
        if after.starts_with("((") && text.trim_end().ends_with("))") {
            return text[..i].trim_end();
        }
    }
    text
}

fn parse_header(rest: &str, no: usize, diags: &mut Vec<Diag>) -> (String, Vec<String>) {
    // `:: name [tag tag] {"position":"…"}`; the metadata block is accepted and ignored.
    let cut = rest.find(['[', '{']).unwrap_or(rest.len());
    let name = rest[..cut].trim().to_string();
    if name.is_empty() {
        diags.push(Diag::error(no, 1, "a passage needs a name after `::`"));
    }
    let mut tags = Vec::new();
    let after = &rest[cut..];
    if let Some(inner) = after.strip_prefix('[') {
        match inner.find(']') {
            Some(end) => {
                tags = inner[..end]
                    .split_whitespace()
                    .map(str::to_string)
                    .collect()
            }
            None => diags.push(Diag::error(
                no,
                3 + rest[..cut].chars().count(),
                "the tag list is missing its closing `]`",
            )),
        }
    }
    (name, tags)
}
