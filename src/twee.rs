//! The Twee 3 container: split a `.twee` file into passages.

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
        while p.lines.last().is_some_and(|l| l.text.trim().is_empty()) {
            p.lines.pop();
        }
    }
    passages
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
