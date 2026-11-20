//! `inkjot migrate`: rewrite an Inkjot v0.5 source for v0.6.
//!
//! The changes are mechanical, so a program makes them:
//!
//! | v0.5 | v0.6 |
//! |---|---|
//! | `"format-version": "0.5.0"` | `"format-version": "0.6.0"` |
//! | `fact cornered: no` | `state cornered: no` |
//! | `~ fact rusty: no` (on an item) | `~ state rusty: no` |
//! | `clue debts: "…"` | `fact debts: "…" ~ clue` |
//! | `~ learned from clue lodestone` | `~ learned from lodestone` |
//! | `[fixme]` | `[fix]` |
//!
//! Only statements are rewritten, found the way the checker finds them, so
//! prose that happens to start with "fact" is left alone. What cannot be
//! rewritten safely, such as an id that is now a reserved word, is reported
//! as a note for the Inker instead.

use crate::syntax::{gather, DECLS, RESERVED};
use crate::twee;

/// The rewritten source, and notes on what needs a person.
pub struct Migrated {
    pub source: String,
    pub notes: Vec<String>,
}

pub fn migrate(src: &str) -> Migrated {
    let mut lines: Vec<String> = src.lines().map(str::to_string).collect();
    let mut notes = Vec::new();
    let mut scratch = Vec::new();
    let passages = twee::parse(src, &mut scratch);
    // A file that already says v0.6 is left alone: `fact` means something
    // else there.
    let already = passages.iter().filter(|p| p.name == "StoryData").any(|p| {
        p.lines
            .iter()
            .any(|l| l.text.contains("\"format-version\"") && !l.text.contains("\"0.5"))
    });
    if already {
        notes.push("this file is not Inkjot v0.5; nothing to migrate".to_string());
        return Migrated {
            source: src.to_string(),
            notes,
        };
    }
    for p in &passages {
        if p.name == "StoryData" {
            for l in &p.lines {
                let raw = &mut lines[l.no - 1];
                if raw.contains("\"format-version\"") {
                    *raw = replace_version(raw);
                }
            }
            continue;
        }
        let (stmts, body) = gather(p, &mut scratch);
        for s in &stmts {
            let head = &s.clauses[0];
            let raw = &mut lines[head.line - 1];
            match s.kw.as_str() {
                "fact" => *raw = replace_word_at(raw, head.col, "fact", "state"),
                "clue" => {
                    *raw = replace_word_at(raw, head.col, "clue", "fact");
                    raw.push_str(" ~ clue");
                }
                _ => {}
            }
            if DECLS.contains(&s.kw.as_str()) || s.kw == "clue" {
                let rest = head.text[s.kw.len()..].trim_start();
                let name = rest.split([':', ' ', '~']).next().unwrap_or("");
                if RESERVED.contains(&name) {
                    notes.push(format!(
                        "line {}: `{name}` is a reserved word in v0.6; rename it by hand",
                        head.line
                    ));
                }
            }
            for c in &s.clauses[1..] {
                let raw = &mut lines[c.line - 1];
                if c.text.starts_with("fact ") {
                    *raw = replace_word_at(raw, c.col, "fact", "state");
                }
                if c.text.starts_with("learned from clue ") {
                    *raw = raw.replacen("learned from clue ", "learned from ", 1);
                }
            }
        }
        for l in body {
            let t = l.text.trim();
            if t.starts_with('[') && !t.starts_with("[[") && t.contains("fixme") {
                lines[l.no - 1] = lines[l.no - 1].replace("fixme", "fix");
            }
        }
    }
    let mut source = lines.join("\n");
    if src.ends_with('\n') {
        source.push('\n');
    }
    Migrated { source, notes }
}

/// `"format-version": "0.5.0"` becomes `"0.6.0"`.
fn replace_version(line: &str) -> String {
    let Some(i) = line.find("\"0.5") else {
        return line.to_string();
    };
    let end = line[i + 1..]
        .find('"')
        .map_or(line.len(), |j| i + 1 + j + 1);
    format!("{}\"0.6.0\"{}", &line[..i], &line[end..])
}

/// Replace `old` with `new` where the statement or clause starts: `col` is
/// the 1-based column of its first character, perhaps after a `~`.
fn replace_word_at(line: &str, col: usize, old: &str, new: &str) -> String {
    let start: usize = line
        .char_indices()
        .nth(col - 1)
        .map_or(line.len(), |(i, _)| i);
    match line[start..].find(old) {
        Some(j) => {
            let at = start + j;
            format!("{}{new}{}", &line[..at], &line[at + old.len()..])
        }
        None => line.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V05: &str = r#":: StoryTitle
Old

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.5.0", "start": "hall"}

:: Inklink [config]
kind: story
mode: solo
pov: ann
character ann: "Ann" ~ playable
clue debts: "He owed." ~ motive
fact cornered: no
item lever: "a lever"
~ fact rusty: no
act pull: cost 1 ~ learned from clue debts

:: hall
fact here_only: 1
--
[fixme] the fact of the matter
fact is, this line is prose.
> [[Stay->hall]]
"#;

    #[test]
    fn a_v05_source_becomes_v06() {
        let m = migrate(V05);
        let s = &m.source;
        assert!(s.contains(r#""format-version": "0.6.0""#), "{s}");
        assert!(
            s.contains("fact debts: \"He owed.\" ~ motive ~ clue"),
            "{s}"
        );
        assert!(s.contains("state cornered: no"), "{s}");
        assert!(s.contains("~ state rusty: no"), "{s}");
        assert!(s.contains("~ learned from debts"), "{s}");
        assert!(s.contains("state here_only: 1"), "{s}");
        assert!(s.contains("[fix] the fact of the matter"), "{s}");
        assert!(s.contains("fact is, this line is prose."), "{s}");
        assert!(m.notes.is_empty());
        let report = crate::check(s);
        assert!(report.ok(), "{:#?}", report.diags);
    }

    #[test]
    fn a_reserved_id_is_noted_not_renamed() {
        let src = V05.replace("character ann", "character you");
        let m = migrate(&src);
        assert!(m
            .notes
            .iter()
            .any(|n| n.contains("`you` is a reserved word")));
    }

    #[test]
    fn migrating_twice_changes_nothing_more() {
        let once = migrate(V05).source;
        let twice = migrate(&once);
        assert_eq!(twice.source, once);
        assert_eq!(twice.notes.len(), 1);
    }
}
