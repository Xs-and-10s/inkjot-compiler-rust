//! The shape of Inkjot text, shared by the checker and the IR builder:
//! the vocabulary, and how lines split into statements and clauses.

use crate::diag::Diag;
use crate::twee::{Line, Passage};

/// Passages whose whole content is statements, on both sides of `--`.
/// Passages whose every line is a statement. A `[test]` passage is a script
/// of statements too: `start:`, `given:`, `do:`, `expect:`.
pub(crate) const MARGIN_TAGS: &[&str] =
    &["config", "npc", "character", "place", "archetype", "test"];

/// Statements that declare an id.
pub(crate) const DECLS: &[&str] = &[
    "place",
    "spot",
    "way",
    "item",
    "clue",
    "fact",
    "fuse",
    "character",
    "npc",
    "voice",
    "act",
    "combo",
    "archetype",
    "list",
    "stat",
    "relation",
    "resource",
    "objective",
    "table",
    "scenario",
    "map",
];

/// Declarations that may appear only once per id.
pub(crate) const UNIQUE: &[&str] = &[
    "place",
    "way",
    "item",
    "clue",
    "character",
    "npc",
    "act",
    "combo",
    "list",
    "fuse",
    "table",
    "archetype",
];

/// Statements that configure something without declaring an id.
pub(crate) const SETTINGS: &[&str] = &[
    "kind",
    "tags",
    "ages",
    "mode",
    "cohort",
    "pov",
    "clock",
    "start",
    "default",
    "route",
    "rewind",
    "checkpoint",
    "expect",
    "let",
    "get",
    "starts",
    "ties",
    "goal",
    "reaction",
    "moves",
    "acts",
    "reveal",
    "dialogue",
    "next",
    "inquest",
    "observe",
    "timeout",
    "param",
    // `[test]` passages.
    "given",
    "do",
    "seed",
    "as",
];

/// Effects: what a header line or a `then` clause can do.
pub(crate) const EFFECTS: &[&str] = &[
    "learn", "teach", "gain", "lose", "give", "set", "raise", "lower", "toggle", "burn", "light",
    "wake", "move", "kill", "wound", "destroy", "detain", "add", "remove", "strike", "roll",
    "play", "say", "stop", "fix", "take", "dash", "feign", "nothing", "wait",
];

/// Deferred to v0.6: parsed far enough to skip, then reported in one notice.
pub(crate) const STRETCH: &[&str] = &[
    "encounter",
    "pattern",
    "spell",
    "retell",
    "shield",
    "interrupt",
    "pin",
    "skip",
    "guard",
    "absorb",
    "parry",
    "freeze",
];
pub(crate) const STRETCH_TAGS: &[&str] = &["encounter", "round", "resolve", "retell"];

/// Words that may start a Link clause, besides the name of a declared act or combo.
pub(crate) const LINK_CLAUSES: &[&str] = &[
    "cost", "when", "hidden", "for", "then", "ends", "private", "shared", "capacity", "clash",
    "check", "chance", "inquest", "order", "after", "become", "with", "by", "switch", "pass",
    "release", "requires", "once", "loud", "quiet", "choice",
];

/// What kind of choice an act or Link is: `~ choice proactive`.
pub(crate) const CHOICES: &[&str] = &["proactive", "reactive", "inactive"];

pub(crate) const EVENTS: &[&str] = &[
    "start", "enter", "leave", "cross", "take", "learn", "act", "turn", "death", "rewind", "out",
    "loud", "raise", "lower", "set",
];

/// Acts every Inklink has, whether or not its config tunes them.
pub(crate) const BUILTIN_ACTS: &[&str] = &["walk", "dash", "slip", "observe", "take"];

/// Tags the engine gives things itself.
pub(crate) const BUILTIN_TAGS: &[&str] = &[
    "linker",
    "living",
    "character",
    "npc",
    "item",
    "clue",
    "place",
];

/// Link targets that are not passages.
pub(crate) const BUILTIN_TARGETS: &[&str] = &["here", "rewind"];

#[derive(Debug, Clone)]
pub(crate) struct Clause {
    pub(crate) text: String,
    pub(crate) line: usize,
    /// 1-based character column of the clause's first character.
    pub(crate) col: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct Stmt {
    /// The leading word, or `{` / `[` for an insert or modifier line in a header.
    pub(crate) kw: String,
    /// `clauses[0]` is the head; the rest follow `~`.
    pub(crate) clauses: Vec<Clause>,
}

pub(crate) fn cols(s: &str) -> usize {
    s.chars().count()
}

/// For each byte of `s`: is it outside every string and bracket?
pub(crate) fn top_mask(s: &str) -> Vec<bool> {
    let mut mask = vec![false; s.len()];
    let mut in_str = false;
    let mut depth = 0usize;
    for (i, ch) in s.char_indices() {
        match ch {
            '"' => in_str = !in_str,
            '(' | '[' | '{' if !in_str => depth += 1,
            ')' | ']' | '}' if !in_str => depth = depth.saturating_sub(1),
            _ => mask[i] = !in_str && depth == 0,
        }
    }
    mask
}

/// Split a line into clauses at every top-level `~`.
pub(crate) fn split_clauses(text: &str, line: usize, base_col: usize) -> Vec<Clause> {
    let mask = top_mask(text);
    let mut out = Vec::new();
    let mut start = 0;
    let mut push = |a: usize, b: usize| {
        let seg = &text[a..b];
        let trimmed = seg.trim();
        if !trimmed.is_empty() {
            let lead = seg.len() - seg.trim_start().len();
            out.push(Clause {
                text: trimmed.to_string(),
                line,
                col: base_col + cols(&text[..a + lead]),
            });
        }
    };
    for (i, ch) in text.char_indices() {
        if ch == '~' && mask[i] {
            push(start, i);
            start = i + 1;
        }
    }
    push(start, text.len());
    out
}

/// Byte offset just past a top-level ` word ` (or a leading `word `), if any.
pub(crate) fn after_word(s: &str, word: &str) -> Option<usize> {
    let lead = format!("{word} ");
    if s.starts_with(&lead) {
        return Some(lead.len());
    }
    let needle = format!(" {word} ");
    let mask = top_mask(s);
    let mut from = 0;
    while let Some(i) = s[from..].find(&needle) {
        let at = from + i;
        if mask[at] {
            return Some(at + needle.len());
        }
        from = at + 1;
    }
    None
}

/// The text before a top-level ` when `.
pub(crate) fn before_when(s: &str) -> &str {
    match after_word(s, "when") {
        Some(i) if i > 5 => s[..i - 5].trim_end(),
        Some(_) => "",
        None => s,
    }
}

pub(crate) fn first_word(s: &str) -> &str {
    let end = s
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(s.len());
    &s[..end]
}

pub(crate) fn is_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !s.starts_with(|c: char| c.is_ascii_digit())
}

pub(crate) fn is_duration(s: &str) -> bool {
    let digits = s.trim_end_matches("ms").trim_end_matches('s');
    digits.len() < s.len() && !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

/// The target of a Link line: the `b` in `> [[a->b]]`.
pub(crate) fn link_target(line: &str) -> Option<&str> {
    let open = line.find("[[")?;
    let inner = &line[open + 2..open + line[open..].find("]]")?];
    let target = match (inner.rfind("->"), inner.rfind('|')) {
        (Some(i), _) => &inner[i + 2..],
        (None, Some(i)) => &inner[i + 1..],
        (None, None) => inner,
    };
    Some(target.trim())
}

/// Split a passage into its statements and its body lines.
///
/// In a margin passage every line is a statement. Elsewhere the lines before
/// `--` are statements and the rest is body; with no `--`, it is all body.
pub(crate) fn gather<'a>(p: &'a Passage, diags: &mut Vec<Diag>) -> (Vec<Stmt>, &'a [Line]) {
    let margin = p.tags.iter().any(|t| MARGIN_TAGS.contains(&t.as_str()));
    let sep = p.lines.iter().position(|l| l.text.trim() == "--");
    let (header, body): (&[Line], &[Line]) = if margin {
        (&p.lines, &[])
    } else if let Some(i) = sep {
        (&p.lines[..i], &p.lines[i + 1..])
    } else {
        (&[], &p.lines)
    };
    let mut stmts: Vec<Stmt> = Vec::new();
    let mut open = false;
    for l in header {
        let t = l.text.trim();
        let col = 1 + cols(&l.text[..l.text.len() - l.text.trim_start().len()]);
        if t.is_empty() || t == "--" {
            open = false;
        } else if t.starts_with('~') {
            match stmts.last_mut() {
                Some(s) if open => s.clauses.extend(split_clauses(&l.text, l.no, 1)),
                _ => diags.push(Diag::error(
                    l.no,
                    col,
                    "`~` continues a statement, but there is none on the line before",
                )),
            }
        } else if t.starts_with('{') || t.starts_with('[') {
            let kw = t[..1].to_string();
            stmts.push(Stmt {
                kw,
                clauses: vec![Clause {
                    text: t.to_string(),
                    line: l.no,
                    col,
                }],
            });
            open = false;
        } else {
            let clauses = split_clauses(&l.text, l.no, 1);
            let kw = clauses.first().map(|c| {
                let end = c
                    .text
                    .find(|ch: char| !(ch.is_ascii_alphabetic() || ch == '_'))
                    .unwrap_or(c.text.len());
                c.text[..end].to_string()
            });
            if let Some(kw) = kw {
                stmts.push(Stmt { kw, clauses });
                open = true;
            }
        }
    }
    (stmts, body)
}
