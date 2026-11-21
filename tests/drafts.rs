//! The draft examples are written ahead of the language, so `inkjot check`
//! refuses them. This file checks what can be checked already, so a draft
//! does not drift from the decisions made about v0.6:
//!
//! - an Inklog fact is two terms or three, with an optional `@ time`;
//! - every `@ monday.evening`, `clock.watch == evening` and `on watch begins
//!   night` names a real time, by the periods' arithmetic (`src/period.rs`),
//!   and the clock's range is written in those names;
//! - a knowledge base is asked with `{{ … }} in dossier`, never `~ in
//!   dossier`, which would cut a clause in two;
//! - a quantifier ends with a dot and a space: `some P. …`;
//! - every speech cue, Link target, learned fact and tested Link label
//!   exists;
//! - a comment that describes a passage sits under its header.
//!
//! The mutation properties at the end check that the lint itself catches
//! each kind of slip, wherever in the draft it is made.
//!
//! `INKJOT_DRAFT=path cargo test --test drafts -- --ignored` lints any other
//! draft, such as the Inklinks Library's.

use inkjot::period::Calendar;
use proptest::prelude::*;
use std::collections::HashSet;

fn draft(name: &str) -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    std::fs::read_to_string(format!("{dir}/examples/drafts/{name}")).unwrap()
}

/// Lines with author's comments blanked out (`(((…)))` blocks and inline
/// `((…))`), keeping line numbers.
fn without_comments(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block = false;
    for line in src.lines() {
        let t = line.trim();
        if in_block {
            if t.ends_with(")))") {
                in_block = false;
            }
            out.push(String::new());
            continue;
        }
        if t.starts_with("(((") {
            in_block = !t.ends_with(")))") || t == "(((";
            out.push(String::new());
            continue;
        }
        let mut kept = String::new();
        let mut rest = line;
        while let Some(i) = rest.find("((") {
            kept.push_str(&rest[..i]);
            match rest[i..].find("))") {
                Some(j) => rest = &rest[i + j + 2..],
                None => {
                    rest = "";
                }
            }
        }
        kept.push_str(rest);
        out.push(kept);
    }
    out
}

fn is_var(term: &str) -> bool {
    term == "?" || term.starts_with(|c: char| c.is_ascii_uppercase())
}

/// Everything between `{{` and `}}` on a line.
fn facts(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(i) = rest.find("{{") {
        match rest[i + 2..].find("}}") {
            Some(j) => {
                out.push(rest[i + 2..i + 2 + j].trim());
                rest = &rest[i + 2 + j + 2..];
            }
            None => break,
        }
    }
    out
}

/// Is byte `at` of `line` inside a single-brace insert, `{list x ~ …}`,
/// where `~` gives the insert its options?
fn inside_insert(line: &str, at: usize) -> bool {
    let before = line[..at].replace("{{", "").replace("}}", "");
    before.matches('{').count() > before.matches('}').count()
}

/// The words after `kw ` at the start of statements: declared ids.
fn declared(lines: &[String]) -> HashSet<String> {
    const KINDS: &[&str] = &[
        "place",
        "spot",
        "way",
        "item",
        "fact",
        "state",
        "theme",
        "brief",
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
        "knowledge",
        "rule",
        "period",
        "let",
        "get",
    ];
    let mut out = HashSet::new();
    for line in lines {
        let t = line.trim().trim_start_matches("~ ");
        let mut words = t.splitn(2, ' ');
        let (Some(kw), Some(rest)) = (words.next(), words.next()) else {
            continue;
        };
        if KINDS.contains(&kw) {
            let id: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            out.insert(id);
        }
    }
    out
}

/// Link labels: the text before `->` in `[[…->…]]`.
fn labels(lines: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for line in lines {
        let mut rest = line.as_str();
        while let Some(i) = rest.find("[[") {
            let Some(j) = rest[i..].find("]]") else { break };
            let inner = &rest[i + 2..i + j];
            if let Some((label, _)) = inner.rsplit_once("->") {
                out.push(label.to_string());
            }
            rest = &rest[i + j + 2..];
        }
    }
    out
}

/// Does a tested label match a Link label, where `{…}` in the Link's label
/// stands for anything?
fn label_matches(pattern: &str, said: &str) -> bool {
    let parts: Vec<&str> = pattern
        .split('{')
        .enumerate()
        .flat_map(|(i, p)| {
            if i == 0 {
                vec![p]
            } else {
                p.split_once('}')
                    .map_or(vec![p], |(_, after)| vec!["\u{0}", after])
            }
        })
        .collect();
    let mut at = 0;
    let mut wild = false;
    for (k, part) in parts.iter().enumerate() {
        if *part == "\u{0}" {
            wild = true;
            continue;
        }
        if part.is_empty() {
            continue;
        }
        match said[at..].find(part) {
            Some(i) if wild || i == 0 => {
                at += i + part.len();
                wild = false;
            }
            _ => return false,
        }
        if k == parts.len() - 1 && at != said.len() {
            return false;
        }
    }
    wild || at == said.len()
}

/// Every problem in a draft, as `line N: what`.
fn lint(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let raw: Vec<&str> = src.lines().collect();
    let lines = without_comments(src);
    let cal = match Calendar::parse(lines.iter().map(String::as_str)) {
        Ok(c) => c,
        Err(e) => {
            out.push(format!("periods: {e}"));
            Calendar::default()
        }
    };
    let ids = declared(&lines);
    let passages: HashSet<String> = raw
        .iter()
        .filter_map(|l| l.strip_prefix("::"))
        .map(|h| {
            h.trim()
                .split(['[', '{'])
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        })
        .collect();
    let bases: Vec<String> = lines
        .iter()
        .filter_map(|l| l.trim().strip_prefix("knowledge "))
        .map(|r| r.split_whitespace().next().unwrap_or("").to_string())
        .collect();
    let period_names = |p: &str| -> Option<Vec<String>> { cal.get(p).map(|p| p.names.clone()) };
    let all_labels = labels(&lines);

    // The clock's range, in period names.
    for (i, line) in lines.iter().enumerate() {
        if let Some(range) = line.trim().strip_prefix("clock: ") {
            let range = range.split('~').next().unwrap_or("").trim();
            if range.contains("..=") && range.contains('.') && cal.clock(range).is_none() {
                out.push(format!(
                    "line {}: the clock `{range}` names no span of the periods",
                    i + 1
                ));
            }
        }
    }

    for (i, line) in lines.iter().enumerate() {
        let no = i + 1;
        let t = line.trim();

        // Facts: two or three terms, then an optional `@ time`.
        for fact in facts(line) {
            let (body, time) = match fact.split_once('@') {
                Some((b, w)) => (b.trim(), Some(w.trim())),
                None => (fact, None),
            };
            let n = body.split_whitespace().count();
            if !(2..=3).contains(&n) {
                out.push(format!(
                    "line {no}: `{{{{ {fact} }}}}` has {n} terms; a fact is `subject verb` or `subject verb object`, a verb one word"
                ));
            }
            if let Some(w) = time {
                if w.split_whitespace().count() != 1 {
                    out.push(format!("line {no}: `@ {w}`: a fact takes one time"));
                }
                let named = w
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '.' || c == '_')
                    && !w.starts_with("clock.");
                if named && !is_var(w) && cal.span(w).is_none() {
                    out.push(format!("line {no}: `@ {w}` is not a time the periods name"));
                }
            }
        }

        // Asking a base: `{{ … }} in dossier`, never `~ in dossier` in a clause.
        let is_rule = ["rule ", "never "].iter().any(|k| t.starts_with(k));
        let is_decl = t.starts_with("knowledge ");
        if !is_rule && !is_decl {
            for b in &bases {
                let needle = format!("~ in {b}");
                let in_clause = line
                    .match_indices(&needle)
                    .any(|(k, _)| !inside_insert(line, k));
                if in_clause {
                    out.push(format!(
                        "line {no}: `~ in {b}` starts a new clause; ask the base with `{{{{ … }}}} in {b}`"
                    ));
                }
            }
        }

        // Quantifiers end with a dot and a space.
        for q in ["some ", "all ", "every ", "no "] {
            let mut rest = line.as_str();
            while let Some(k) = rest.find(q) {
                let before_ok = k == 0 || !rest.as_bytes()[k - 1].is_ascii_alphanumeric();
                let after = &rest[k + q.len()..];
                let vars: String = after
                    .chars()
                    .take_while(|c| c.is_ascii_uppercase() || *c == ',' || *c == ' ')
                    .collect();
                let next = after[vars.len()..].chars().next();
                if before_ok && vars.trim().starts_with(|c: char| c.is_ascii_uppercase()) {
                    match next {
                        Some(':') => out.push(format!(
                            "line {no}: a quantifier ends with a dot: `{}{}.`",
                            q,
                            vars.trim()
                        )),
                        Some('.') if !after[vars.len() + 1..].starts_with(' ') => out.push(
                            format!("line {no}: a quantifier's dot is followed by a space"),
                        ),
                        _ => {}
                    }
                }
                rest = &rest[k + q.len()..];
            }
        }

        // Speech cues name someone declared.
        if let Some(cue) = t.strip_prefix('@') {
            let who: String = cue
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !who.is_empty() && !ids.contains(&who) && !["you", "me", "i"].contains(&who.as_str())
            {
                out.push(format!(
                    "line {no}: `@{who}` speaks, but nobody is declared `{who}`"
                ));
            }
        }

        // Link targets are passages.
        let mut rest = line.as_str();
        while let Some(k) = rest.find("->") {
            let after = &rest[k + 2..];
            if let Some(end) = after.find("]]") {
                let target = after[..end].trim();
                if !passages.contains(target) && target != "here" {
                    out.push(format!(
                        "line {no}: the Link goes to `{target}`, which is not a passage"
                    ));
                }
            }
            rest = after;
        }

        // Learned facts are declared (Inklog facts are learned as written).
        if let Some(what) = t.strip_prefix("learn ") {
            let id: String = what
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !id.is_empty() && !ids.contains(&id) {
                out.push(format!(
                    "line {no}: `learn {id}`, but no fact is declared `{id}`"
                ));
            }
        }

        // Period names: `clock.watch == evening`, `clock.day in [tuesday]`,
        // `on watch begins night`.
        for p in &cal.periods {
            let names = period_names(&p.name).unwrap_or_default();
            let mut check = |name: &str| {
                if !names.iter().any(|n| n == name) {
                    out.push(format!("line {no}: `{name}` is not a {}", p.name));
                }
            };
            let eq = format!("clock.{} == ", p.name);
            let neq = format!("clock.{} != ", p.name);
            for pat in [&eq, &neq] {
                let mut rest = line.as_str();
                while let Some(k) = rest.find(pat.as_str()) {
                    let name: String = rest[k + pat.len()..]
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    check(&name);
                    rest = &rest[k + pat.len()..];
                }
            }
            let within = format!("clock.{} in [", p.name);
            let mut rest = line.as_str();
            while let Some(k) = rest.find(within.as_str()) {
                let list = &rest[k + within.len()..];
                let list = &list[..list.find(']').unwrap_or(list.len())];
                for name in list.split(',') {
                    check(name.trim());
                }
                rest = &rest[k + within.len()..];
            }
            if let Some(name) = t.strip_prefix(&format!("on {} begins ", p.name)) {
                check(name.trim());
            }
        }

        // Tested Links exist.
        if let Some(label) = t.strip_prefix("do: \"").and_then(|l| l.strip_suffix('"')) {
            if !all_labels.iter().any(|l| label_matches(l, label)) {
                out.push(format!(
                    "line {no}: the test takes \"{label}\", and no Link says that"
                ));
            }
        }
    }

    // Every declared state is read somewhere: a condition, an insert, an
    // effect. One nothing reads is a thread the Inker dropped.
    let text = lines.join("\n");
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim().trim_start_matches("~ ");
        let Some(rest) = t.strip_prefix("state ") else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        // A read is any mention that is not the declaration or a write
        // (`set x:`, `raise x`, `lower x`, with or without an owner).
        let reads = text
            .match_indices(name.as_str())
            .filter(|(k, _)| {
                let b = text.as_bytes();
                let before = *k == 0 || !(b[k - 1].is_ascii_alphanumeric() || b[k - 1] == b'_');
                let end = k + name.len();
                let after = end >= b.len() || !(b[end].is_ascii_alphanumeric() || b[end] == b'_');
                let line_start = text[..*k].rfind('\n').map_or(0, |n| n + 1);
                let lead = text[line_start..*k].trim_start().trim_start_matches("~ ");
                let lead = lead.trim_start_matches("then ");
                let owner = lead.rsplit(' ').next().unwrap_or("");
                let verb = lead.trim_end_matches(owner).trim_end();
                let write = ["set", "raise", "lower", "state"].contains(&verb)
                    && (owner.is_empty() || owner.ends_with('.'));
                before && after && !write
            })
            .count();
        if reads == 0 {
            out.push(format!(
                "line {}: `state {name}` is declared and never read",
                i + 1
            ));
        }
    }

    // A comment just above a header belongs to the passage before it.
    for (i, w) in raw.windows(2).enumerate() {
        if w[0].trim() == ")))" && w[1].starts_with("::") {
            out.push(format!(
                "line {}: this comment ends right above `{}`, so it is in the passage before; put it under the header",
                i + 1,
                w[1].trim()
            ));
        }
    }
    out
}

#[test]
fn the_public_cut_of_ex6_lints_clean() {
    assert_eq!(lint(&draft("ex6-cloakroom.twee")), Vec::<String>::new());
}

#[test]
fn ex8_lints_clean() {
    assert_eq!(lint(&draft("ex8-the-inquest.twee")), Vec::<String>::new());
}

#[test]
#[ignore = "lints the draft named by INKJOT_DRAFT"]
fn a_draft_from_elsewhere() {
    let path = std::env::var("INKJOT_DRAFT").expect("INKJOT_DRAFT names a .twee file");
    let src = std::fs::read_to_string(&path).unwrap();
    let problems = lint(&src);
    assert!(problems.is_empty(), "{path}:\n{}", problems.join("\n"));
}

#[test]
fn the_lint_reads_labels_with_targets() {
    assert!(label_matches(
        "Name {target} as CORMORANT",
        "Name Julian Ashe as CORMORANT"
    ));
    assert!(label_matches(
        "Frostglass {target}",
        "Frostglass the east mirror"
    ));
    assert!(!label_matches(
        "Frostglass {target}",
        "Rime Shield on Eldric"
    ));
    assert!(label_matches("Begin", "Begin"));
    assert!(!label_matches("Begin", "Begin again"));
}

/// The positions in `src` where `needle` occurs.
fn sites(src: &str, needle: &str) -> Vec<usize> {
    src.match_indices(needle).map(|(i, _)| i).collect()
}

fn mutated(src: &str, needle: &str, pick: usize, with: &str) -> Option<String> {
    let at = sites(src, needle);
    let i = *at.get(pick % at.len().max(1))?;
    let mut s = src.to_string();
    s.replace_range(i..i + needle.len(), with);
    Some(s)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Two more terms in any fact (so four or more) are caught.
    #[test]
    fn a_fourth_term_anywhere_is_caught(pick in 0usize..1000) {
        let src = draft("ex6-cloakroom.twee");
        let s = mutated(&src, " }}", pick, " one more }}").unwrap();
        // A fact in an author's note is not read, so is not checked.
        prop_assume!(without_comments(&s).iter().any(|l| l.contains(" one more }}")));
        let problems = lint(&s);
        prop_assert!(
            problems.iter().any(|p| p.contains("terms") || p.contains("one time")),
            "{:?}",
            problems
        );
    }

    /// A time the periods don't name is caught, wherever a time is written.
    #[test]
    fn a_wrong_time_anywhere_is_caught(pick in 0usize..1000) {
        let src = draft("ex6-cloakroom.twee");
        let s = mutated(&src, "@ new_years_eve.", pick, "@ new_years_day.").unwrap();
        let problems = lint(&s);
        prop_assert!(problems.iter().any(|p| p.contains("not a time")), "{:?}", problems);
    }

    /// Asking a base with a tilde is caught, at any `in dossier`.
    #[test]
    fn a_tilde_before_a_base_is_caught(pick in 0usize..1000) {
        let src = draft("ex6-cloakroom.twee");
        let s = mutated(&src, "}} in dossier", pick, "}} ~ in dossier").unwrap();
        let problems = lint(&s);
        prop_assert!(problems.iter().any(|p| p.contains("starts a new clause")), "{:?}", problems);
    }

    /// A Link to a passage that doesn't exist is caught.
    #[test]
    fn a_missing_target_is_caught(pick in 0usize..1000) {
        let src = draft("ex6-cloakroom.twee");
        let s = mutated(&src, "->", pick, "->nowhere_").unwrap();
        let problems = lint(&s);
        prop_assert!(problems.iter().any(|p| p.contains("not a passage")), "{:?}", problems);
    }

    /// An hour that isn't one is caught, in any comparison or event.
    #[test]
    fn a_wrong_hour_is_caught(pick in 0usize..1000) {
        let src = draft("ex6-cloakroom.twee");
        let s = mutated(&src, "clock.hour == ", pick, "clock.hour == twelve_").unwrap();
        let problems = lint(&s);
        prop_assert!(problems.iter().any(|p| p.contains("is not a hour")), "{:?}", problems);
    }
}

#[test]
fn a_colon_after_a_quantifier_is_caught() {
    let src = draft("ex6-cloakroom.twee").replace("some P. {{", "some P: {{");
    assert!(lint(&src).iter().any(|p| p.contains("ends with a dot")));
}

#[test]
fn a_comment_above_a_header_is_caught() {
    let src = draft("ex6-cloakroom.twee").replace(
        ":: dossier_rules [rules]\n(((\nHis rules, in the order he finds you.\n)))",
        "(((\nHis rules, in the order he finds you.\n)))\n:: dossier_rules [rules]",
    );
    assert!(lint(&src).iter().any(|p| p.contains("under the header")));
}

#[test]
fn a_state_nothing_reads_is_caught() {
    let src = draft("ex8-the-inquest.twee").replace(
        "state struck: no",
        "state struck: no\nstate dropped_thread: no",
    );
    let src = src.replace(
        ":: garrard:dined\nset struck: yes",
        ":: garrard:dined\nset struck: yes\nset dropped_thread: yes",
    );
    let problems = lint(&src);
    assert!(
        problems
            .iter()
            .any(|p| p.contains("`state dropped_thread` is declared and never read")),
        "{problems:?}"
    );
}
