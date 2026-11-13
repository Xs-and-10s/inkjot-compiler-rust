//! `inkjot check`: read an Inkjot source file and report what is wrong with it.
//!
//! Two passes. The first collects every passage name and every declared id
//! (places, items, clues, acts, …). The second walks each passage again and
//! validates statements, Links, speeches, modifiers, inserts and expressions
//! against that table.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::diag::{Diag, Severity};
use crate::resolve::{self, Lookup, Problem, Want};
use crate::syntax::*;
use crate::twee::{self, Line, Passage};
use crate::{expr, json};

/// The Inkjot version this compiler targets.
pub const VERSION: &str = "0.5";

#[derive(Debug, Default, Clone)]
pub struct Stats {
    pub passages: usize,
    pub declarations: usize,
    pub links: usize,
    pub speeches: usize,
    pub expressions: usize,
}

#[derive(Debug)]
pub struct Report {
    pub diags: Vec<Diag>,
    pub stats: Stats,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }
    pub fn ok(&self) -> bool {
        self.errors() == 0
    }
}

/// Check one Inkjot source text.
pub fn check(src: &str) -> Report {
    let mut c = Checker::default();
    let passages = twee::parse(src, &mut c.diags);
    c.stats.passages = passages.len();
    c.collect(&passages);
    for p in &passages {
        c.passage(p);
    }
    c.file_rules(&passages);
    c.stretch_notice();
    c.diags.sort_by_key(|d| (d.line, d.col, d.severity));
    Report {
        diags: c.diags,
        stats: c.stats,
    }
}

/// The candidate nearest to `name`, if one is a plausible misspelling of it.
fn closest(name: &str, candidates: Vec<String>) -> Option<String> {
    let limit = (name.chars().count() / 3).clamp(1, 2);
    candidates
        .into_iter()
        .map(|c| (edit_distance(name, &c), c))
        .filter(|(d, _)| *d <= limit)
        // Sort by distance, then by name, so the answer never depends on hash order.
        .min()
        .map(|(_, c)| c)
}

/// How many single-character edits turn `a` into `b` (Levenshtein distance).
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    // `row[j]` is the distance between the part of `a` seen so far and `b[..j]`.
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitute = diagonal + usize::from(ca != *cb);
            diagonal = row[j + 1];
            row[j + 1] = substitute.min(row[j] + 1).min(row[j + 1] + 1);
        }
    }
    row[b.len()]
}

// ---------------------------------------------------------------- the checker

#[derive(Default)]
struct Checker {
    diags: Vec<Diag>,
    passages: HashSet<String>,
    /// id → the kinds it is declared as (`clue` and `act` may share a name).
    decls: HashMap<String, HashSet<&'static str>>,
    /// Free words on declarations (`~ evidence`), plus the built-in ones.
    tags: HashSet<String>,
    /// passage → the names it introduces: `let`s, input variables, and the
    /// `with {…}` parameters of Links that lead to it.
    scopes: HashMap<String, HashSet<String>>,
    /// `let`s in the config, visible everywhere.
    globals: HashSet<String>,
    /// The names in scope in the passage being checked right now.
    locals: HashSet<String>,
    stretch: BTreeMap<String, usize>,
    stats: Stats,
}

impl Checker {
    fn err(&mut self, line: usize, col: usize, msg: impl Into<String>) {
        self.diags.push(Diag::error(line, col, msg));
    }

    fn has(&self, id: &str, kinds: &[&str]) -> bool {
        self.decls
            .get(id)
            .is_some_and(|ks| ks.iter().any(|k| kinds.contains(k)))
    }

    /// Require `id` to be declared as one of `kinds`.
    fn need(&mut self, id: &str, kinds: &[&str], what: &str, line: usize, col: usize) {
        let id = id.trim_matches(|c: char| c == ',' || c == ':');
        if !is_ident(id) || self.has(id, kinds) {
            return;
        }
        let also = match self.decls.get(id) {
            Some(ks) => {
                let mut ks: Vec<_> = ks.iter().copied().collect();
                ks.sort_unstable();
                format!(" (it is declared as: {})", ks.join(", "))
            }
            None => String::new(),
        };
        self.err(line, col, format!("`{id}` is not a declared {what}{also}"));
    }

    fn need_passage(&mut self, name: &str, what: &str, line: usize, col: usize) {
        if !self.passages.contains(name) {
            self.err(
                line,
                col,
                format!("{what} `{name}` is not a passage in this file"),
            );
        }
    }

    /// Parse an expression and resolve the names in it.
    fn expr(&mut self, text: &str, line: usize, col: usize) {
        let lead = text.len() - text.trim_start().len();
        let body = text.trim();
        let col = col + cols(&text[..lead]);
        self.stats.expressions += 1;
        match expr::parse(body) {
            Ok(e) => self.resolve(&e, body, line, col),
            Err((off, msg)) => {
                let off = off.min(body.len());
                self.err(line, col + cols(&body[..off]), msg);
            }
        }
    }

    /// Report every name in `e` that refers to nothing.
    fn resolve(&mut self, e: &expr::Expr, src: &str, line: usize, col: usize) {
        for Problem { at, name, want } in resolve::unresolved(e, self) {
            let (what, candidates) = match want {
                Want::Value => (
                    "is not declared: it is not a builtin, a declared id, a tag, or a `let` in scope".to_string(),
                    self.decls.keys().chain(&self.locals).chain(&self.globals).cloned().collect(),
                ),
                Want::Function => (
                    format!("is not a function; the functions are: {}", resolve::FUNCTIONS.join(", ")),
                    resolve::FUNCTIONS.iter().map(|f| f.to_string()).collect(),
                ),
                Want::Kind(kind) => (
                    format!("is not a declared {kind}"),
                    self.decls.iter().filter(|(_, ks)| ks.contains(kind)).map(|(n, _)| n.clone()).collect(),
                ),
            };
            let hint = match closest(&name, candidates) {
                Some(near) => format!("; did you mean `{near}`?"),
                None => String::new(),
            };
            let at = at.min(src.len());
            self.err(
                line,
                col + cols(&src[..at]),
                format!("`{name}` {what}{hint}"),
            );
        }
    }

    // ------------------------------------------------------------ pass one

    fn collect(&mut self, passages: &[Passage]) {
        for act in BUILTIN_ACTS {
            self.decls.entry(act.to_string()).or_default().insert("act");
        }
        for tag in BUILTIN_TAGS {
            self.tags.insert(tag.to_string());
        }
        for p in passages {
            // A nameless passage is already reported by the Twee reader.
            if !self.passages.insert(p.name.clone()) && !p.name.is_empty() {
                self.err(
                    p.line,
                    p.col,
                    format!("there are two passages named `{}`", p.name),
                );
            }
            if p.has_tag("archetype") {
                self.decls
                    .entry(p.name.clone())
                    .or_default()
                    .insert("archetype");
            }
        }
        let mut seen: HashSet<(String, &'static str)> = HashSet::new();
        for p in passages {
            let mut scratch = Vec::new();
            let (stmts, _) = gather(p, &mut scratch);
            self.collect_scope(p, &stmts);
            for s in &stmts {
                self.collect_entries(s);
            }
            for s in stmts {
                let Some(kind) = DECLS.iter().copied().find(|k| *k == s.kw) else {
                    continue;
                };
                // A one-word clause on a declaration is a tag: `~ evidence`.
                for c in &s.clauses[1..] {
                    if is_ident(&c.text) {
                        self.tags.insert(c.text.clone());
                    }
                }
                let head = &s.clauses[0];
                let name = first_word(head.text[s.kw.len()..].trim_start());
                if !is_ident(name) {
                    continue; // reported in pass two
                }
                self.stats.declarations += 1;
                self.decls.entry(name.to_string()).or_default().insert(kind);
                if UNIQUE.contains(&kind) && !seen.insert((name.to_string(), kind)) {
                    self.err(
                        head.line,
                        head.col,
                        format!("{kind} `{name}` is declared twice"),
                    );
                }
            }
        }
    }

    /// A list entry is named where it is added: `~ holds vacuum: "the vacuum"`,
    /// `add button: "the button" to the_list`, or a clause of `add to charges:`.
    fn collect_entries(&mut self, s: &Stmt) {
        let mut names: Vec<String> = Vec::new();
        let entry = |text: &str| {
            let text = text.trim();
            let end = text.find([':', ' ']).unwrap_or(text.len());
            text[..end].to_string()
        };
        match s.kw.as_str() {
            "list" => {
                for c in &s.clauses[1..] {
                    if let Some(items) = c.text.strip_prefix("holds ") {
                        names.extend(items.split(',').map(entry));
                    }
                }
            }
            "add" => {
                let rest = s.clauses[0].text[3..].trim_start();
                if !rest.starts_with("to ") {
                    names.push(entry(rest));
                }
                names.extend(s.clauses[1..].iter().map(|c| entry(&c.text)));
            }
            _ => {}
        }
        for name in names.into_iter().filter(|n| is_ident(n)) {
            self.decls.entry(name).or_default().insert("entry");
        }
    }

    /// Record the names a passage introduces, and the parameters its Links
    /// hand to the passages they lead to.
    fn collect_scope(&mut self, p: &Passage, stmts: &[Stmt]) {
        let mut names: HashSet<String> = HashSet::new();
        for s in stmts.iter().filter(|s| s.kw == "let") {
            names.insert(first_word(s.clauses[0].text[3..].trim_start()).to_string());
        }
        let mut target: Option<String> = None;
        for l in &p.lines {
            let t = l.text.trim();
            // `{choose input for: mouth_words, …}` introduces `mouth_words`.
            if let Some(i) = t.find(" input for: ") {
                names.insert(first_word(&t[i + 12..]).to_string());
            }
            if t.starts_with('>') {
                target = link_target(t).map(str::to_string);
            } else if !t.starts_with('~') {
                target = None;
            }
            // `~ with {target: roland}` gives the Link's target a `target`.
            if let (Some(target), Some(i)) = (&target, t.find("with {")) {
                let params = t[i + 6..].split('}').next().unwrap_or("");
                let scope = self.scopes.entry(target.clone()).or_default();
                for param in params.split(',') {
                    scope.insert(first_word(param.trim()).to_string());
                }
            }
        }
        if p.has_tag("config") {
            self.globals.extend(names);
        } else {
            self.scopes.entry(p.name.clone()).or_default().extend(names);
        }
    }

    /// The names in scope in `passage`: its own, and those of every passage
    /// it is a sub-Nib of (`orrery:crank` sees the `let`s of `orrery`).
    fn scope_of(&self, passage: &str) -> HashSet<String> {
        let mut names = HashSet::new();
        let mut name = passage;
        loop {
            if let Some(scope) = self.scopes.get(name) {
                names.extend(scope.iter().cloned());
            }
            match name.rsplit_once(':') {
                Some((parent, _)) => name = parent,
                None => return names,
            }
        }
    }

    // ------------------------------------------------------------ pass two

    fn passage(&mut self, p: &Passage) {
        match p.name.as_str() {
            "StoryTitle" => {
                if p.lines.iter().all(|l| l.text.trim().is_empty()) {
                    self.err(
                        p.line,
                        1,
                        "StoryTitle is empty; put the Inklink's title on the next line",
                    );
                }
                return;
            }
            "StoryData" => return self.story_data(p),
            _ => {}
        }
        if let Some((parent, _)) = p.name.rsplit_once(':') {
            if !self.passages.contains(parent) {
                self.err(
                    p.line,
                    p.col,
                    format!(
                        "`{}` is a sub-Nib of `{parent}`, but there is no passage `{parent}`",
                        p.name
                    ),
                );
            }
        }
        for t in &p.tags {
            if STRETCH_TAGS.contains(&t.as_str()) {
                *self.stretch.entry(format!("[{t}]")).or_default() += 1;
            }
        }
        self.locals = self.scope_of(&p.name);
        let mut diags = std::mem::take(&mut self.diags);
        let (stmts, body) = gather(p, &mut diags);
        self.diags = diags;
        for s in &stmts {
            self.stmt(s);
        }
        self.body(body);
    }

    fn story_data(&mut self, p: &Passage) {
        let text: String = p
            .lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let fields = match json::top_level_strings(&text) {
            Ok(f) => f,
            Err(msg) => return self.err(p.line, 1, format!("StoryData is not valid JSON: {msg}")),
        };
        match fields.get("format").map(String::as_str) {
            Some("Inkjot") => {}
            Some(other) => self.err(
                p.line,
                1,
                format!("StoryData's format is \"{other}\"; this compiler reads \"Inkjot\""),
            ),
            None => self.err(p.line, 1, "StoryData needs \"format\": \"Inkjot\""),
        }
        match fields.get("format-version") {
            Some(v) if v.starts_with(VERSION) => {}
            Some(v) => self.err(
                p.line,
                1,
                format!("this file targets Inkjot {v}; this compiler reads {VERSION}.x"),
            ),
            None => self.err(
                p.line,
                1,
                format!("StoryData needs \"format-version\": \"{VERSION}.0\""),
            ),
        }
        match fields.get("start") {
            Some(start) => {
                self.need_passage(&start.clone(), "StoryData's start passage", p.line, 1)
            }
            None => self.err(p.line, 1, "StoryData needs a \"start\" passage"),
        }
    }

    fn file_rules(&mut self, passages: &[Passage]) {
        if !self.passages.contains("StoryData") {
            self.err(0, 0, "there is no `:: StoryData` passage");
        }
        let Some(config) = passages.iter().find(|p| p.has_tag("config")) else {
            return self.err(0, 0, "there is no `[config]` passage");
        };
        let mut scratch = Vec::new();
        let (stmts, _) = gather(config, &mut scratch);
        let value = |kw: &str| {
            stmts.iter().find(|s| s.kw == kw).map(|s| {
                let head = &s.clauses[0];
                (
                    head.text[kw.len()..]
                        .trim_start_matches(':')
                        .trim()
                        .to_string(),
                    head.line,
                    head.col,
                )
            })
        };
        if value("kind").is_none() {
            self.err(
                config.line,
                1,
                "the config needs `kind: story | game | story-game`",
            );
        }
        let cohort = value("cohort");
        match (value("mode"), cohort) {
            (Some((m, ..)), Some((_, line, col))) if m == "solo" => {
                self.err(
                    line,
                    col,
                    "`mode: solo` already means one Linker; remove `cohort`",
                );
            }
            (Some((m, line, col)), None) if m == "cohort" => {
                self.err(
                    line,
                    col,
                    "`mode: cohort` needs a `cohort:` line saying how many Linkers",
                );
            }
            _ => {}
        }
    }

    fn stretch_notice(&mut self) {
        if self.stretch.is_empty() {
            return;
        }
        let total: usize = self.stretch.values().sum();
        let parts: Vec<String> = self
            .stretch
            .iter()
            .map(|(k, n)| format!("{k} ×{n}"))
            .collect();
        self.diags.push(Diag::notice(
            0,
            0,
            format!(
                "{total} uses of v0.6 stretch features were parsed and skipped: {}",
                parts.join(", ")
            ),
        ));
    }

    // ------------------------------------------------------------ statements

    fn stmt(&mut self, s: &Stmt) {
        let head = s.clauses[0].clone();
        let kw = s.kw.as_str();
        match kw {
            "{" => return self.inserts(&head.text, head.line, head.col),
            "[" => {
                let mut if_seen = true;
                return self.modifier(&head.text, head.line, head.col, &mut if_seen);
            }
            "" => {
                return self.err(
                    head.line,
                    head.col,
                    "expected a statement here (a keyword such as `item`, `set` or `on`)",
                )
            }
            _ => {}
        }
        if STRETCH.contains(&kw) {
            *self.stretch.entry(kw.to_string()).or_default() += 1;
            return;
        }
        let rest = head.text[kw.len()..].trim_start();
        let rest_col = head.col + cols(&head.text[..head.text.len() - rest.len()]);

        if kw == "on" || kw == "upon" {
            self.on(rest, head.line, rest_col);
        } else if let Some(kind) = DECLS.iter().copied().find(|k| *k == kw) {
            self.decl(kind, s, rest, rest_col);
        } else if EFFECTS.contains(&kw) {
            self.effect(&head.text, head.line, head.col);
        } else if SETTINGS.contains(&kw) {
            self.setting(kw, s, rest, rest_col);
        } else if self.has(kw, &["resource", "stat"]) {
            // `hp 3 when …`: a conditional value for a declared resource or stat.
        } else {
            return self.err(head.line, head.col, format!("unknown statement `{kw}`"));
        }

        for (i, c) in s.clauses.iter().enumerate() {
            self.clause_common(c, i > 0);
            if i > 0 && matches!(kw, "item" | "npc" | "character") {
                self.actor_clause(c);
            }
        }
    }

    /// Checks shared by every clause: `when`, `until`, `then`, `check`, `like`, table rows.
    fn clause_common(&mut self, c: &Clause, is_tail: bool) {
        if let Some(i) = after_word(&c.text, "when") {
            self.expr(&c.text[i..], c.line, c.col + cols(&c.text[..i]));
        }
        for lead in ["hidden until ", "fixed until ", "offstage until "] {
            if let Some(e) = c.text.strip_prefix(lead) {
                self.expr(e, c.line, c.col + cols(lead));
            }
        }
        if let Some(stat) = c.text.strip_prefix("check ") {
            self.need(first_word(stat), &["stat"], "stat", c.line, c.col + 6);
        }
        if !is_tail {
            return;
        }
        if let Some(e) = c.text.strip_prefix("then ") {
            self.effect(e, c.line, c.col + 5);
        } else if let Some(a) = c.text.strip_prefix("like ") {
            self.need(
                first_word(a),
                &["archetype"],
                "archetype",
                c.line,
                c.col + 5,
            );
        } else if c
            .text
            .starts_with(|ch: char| ch.is_ascii_digit() || ch == '.')
        {
            // A table row: `1..=2: then teach blocker the_attack`.
            if let Some(i) = c.text.find(": ") {
                let row = c.text[i + 2..].trim_start();
                let col = c.col + cols(&c.text[..c.text.len() - row.len()]);
                if let Some(e) = row.strip_prefix("then ") {
                    self.effect(e, c.line, col + 5);
                } else if row != "nothing" {
                    self.err(
                        c.line,
                        col,
                        "a table row is `range: then effect` or `range: nothing`",
                    );
                }
            }
        }
    }

    /// Clauses on things that sit somewhere: `in place`, `held by who`, `knows clue`, `moves …`.
    fn actor_clause(&mut self, c: &Clause) {
        let words: Vec<&str> = c.text.split_whitespace().collect();
        match words.as_slice() {
            ["in", place] => self.need(place, &["place"], "place", c.line, c.col + 3),
            ["held", "by", who] => self.need(
                who,
                &["character", "npc"],
                "character or NPC",
                c.line,
                c.col + 8,
            ),
            ["knows", clue] => self.need(clue, &["clue"], "clue", c.line, c.col + 6),
            ["moves", ..] => self.moves(&c.text[5..], c.line, c.col + 5),
            _ => {}
        }
    }

    fn moves(&mut self, list: &str, line: usize, col: usize) {
        let list = list.trim_start_matches(':');
        if list
            .trim_start()
            .starts_with(|ch: char| ch.is_ascii_digit())
        {
            return; // `moves: 1 per chronon`
        }
        for name in list.split(',') {
            let name = name.trim();
            if name != "none" {
                self.need(name, &["act", "combo"], "act", line, col);
            }
        }
    }

    fn decl(&mut self, kind: &'static str, s: &Stmt, rest: &str, rest_col: usize) {
        let head = &s.clauses[0];
        let name = first_word(rest);
        if !is_ident(name) {
            return self.err(
                head.line,
                rest_col,
                format!("`{kind}` needs a name: `{kind} name: …`"),
            );
        }
        let after = rest[name.len()..].trim_start();
        let Some(value) = after.strip_prefix(':') else {
            return self.err(
                head.line,
                rest_col + cols(name),
                format!("expected `:` after the name `{name}`"),
            );
        };
        let value_col = rest_col + cols(&rest[..rest.len() - value.len()]);
        match kind {
            "way" => self.places(value, " - ", head.line, value_col),
            // A map names a region and lays its places out in rows: `~ a | b | c`.
            "map" => {
                for c in &s.clauses[1..] {
                    if c.text.contains('|') {
                        self.places(&c.text, "|", c.line, c.col);
                    }
                }
            }
            "combo" => {
                for act in value.split(',') {
                    self.need(act.trim(), &["act", "combo"], "act", head.line, value_col);
                }
            }
            _ => {}
        }
    }

    fn places(&mut self, list: &str, sep: &str, line: usize, col: usize) {
        for place in list.split(sep) {
            self.need(place.trim(), &["place"], "place", line, col);
        }
    }

    fn setting(&mut self, kw: &str, s: &Stmt, rest: &str, rest_col: usize) {
        let head = &s.clauses[0];
        let line = head.line;
        // The value after the first top-level colon, if the setting has one.
        let mask = top_mask(rest);
        let colon = rest
            .char_indices()
            .find(|(i, ch)| *ch == ':' && mask[*i])
            .map(|(i, _)| i);
        let (value, vcol) = match colon {
            Some(i) => {
                let v = rest[i + 1..].trim_start();
                (v, rest_col + cols(&rest[..rest.len() - v.len()]))
            }
            None => ("", rest_col),
        };
        match kw {
            "kind" => {
                if !["story", "game", "story-game"].contains(&value) {
                    self.err(line, vcol, "`kind` is one of: story, game, story-game");
                }
            }
            "mode" => {
                if !["solo", "cohort"].contains(&value) {
                    self.err(line, vcol, "`mode` is `solo` or `cohort`");
                }
            }
            "pov" => {
                for who in value.split(',') {
                    let who = who.trim();
                    if is_ident(who) {
                        self.need(who, &["character"], "character", line, vcol);
                    }
                }
            }
            "start" => {
                for part in value.split(',') {
                    if let Some(cell) = part.split_whitespace().next() {
                        self.need_passage(cell, "the start cell", line, vcol);
                    }
                }
            }
            "starts" | "next" => {
                let target = value.split_whitespace().next().unwrap_or("");
                if target.is_empty() {
                    self.err(line, vcol, format!("`{kw}:` needs a passage name"));
                } else {
                    self.need_passage(
                        target,
                        if kw == "next" {
                            "the next passage"
                        } else {
                            "the starting cell"
                        },
                        line,
                        vcol,
                    );
                }
            }
            "route" => self.places(value, " - ", line, vcol),
            "default" => {
                if let Some(list) = rest.strip_prefix("moves") {
                    self.moves(list, line, rest_col + 5);
                } else if rest.starts_with("clash")
                    && !["all", "none", "random", "random_winner"].contains(&value)
                {
                    self.err(
                        line,
                        vcol,
                        "`default clash` is one of: all, none, random, random_winner",
                    );
                }
            }
            "moves" => self.moves(rest, line, rest_col),
            "given" => self.effect(value, line, vcol),
            "do" => {
                if value.starts_with('"') {
                    if value.len() < 2 || !value.ends_with('"') {
                        self.err(line, vcol, "a Link label in `do:` needs its closing quote");
                    }
                } else if let Some((passage, n)) = value.split_once('#') {
                    if n.parse::<usize>().is_err() {
                        self.err(
                            line,
                            vcol,
                            "`do:` takes a Link's label in quotes, or its id such as `hall#2`",
                        );
                    } else {
                        self.need_passage(passage, "the Link's passage", line, vcol);
                    }
                } else {
                    self.err(
                        line,
                        vcol,
                        "`do:` takes a Link's label in quotes, or its id such as `hall#2`",
                    );
                }
            }
            "as" => self.need(value, &["character"], "character", line, vcol),
            "seed" => {
                if value.parse::<u64>().is_err() {
                    self.err(line, vcol, "`seed:` takes a whole number");
                }
            }
            "expect" => {
                if value.is_empty() {
                    self.err(line, rest_col, "`expect:` needs an expression to check");
                } else {
                    self.expr(value, line, vcol);
                }
            }
            "let" => {
                let v = before_when(value);
                if v.is_empty() {
                    self.err(line, rest_col, "`let` is written `let name: value`");
                } else if !v.starts_with("roll ") {
                    self.expr(v, line, vcol);
                }
            }
            _ => {}
        }
    }

    /// `on [before|after] event [what] [by who] [when expr]`
    fn on(&mut self, rest: &str, line: usize, col: usize) {
        let head = before_when(rest);
        let mut words = head.split_whitespace();
        let mut event = words.next().unwrap_or("");
        if event == "before" || event == "after" {
            event = words.next().unwrap_or("");
        }
        if !EVENTS.contains(&event) {
            let shown = if event.is_empty() {
                "nothing".to_string()
            } else {
                format!("`{event}`")
            };
            return self.err(
                line,
                col,
                format!("{shown} is not an event; events are: {}", EVENTS.join(", ")),
            );
        }
        let Some(arg) = words.next() else { return };
        let arg_col = col + cols(&rest[..rest.find(arg).unwrap_or(0)]);
        match event {
            "act" => self.need_passage(arg, "the act", line, arg_col),
            "cross" => self.need(arg, &["way"], "way", line, arg_col),
            "enter" | "leave" => self.need(arg, &["place"], "place", line, arg_col),
            "take" => self.need(arg, &["item"], "item", line, arg_col),
            _ => {}
        }
    }

    /// One effect, as written on a header line or after `then`.
    fn effect(&mut self, text: &str, line: usize, col: usize) {
        let verb = first_word(text);
        if STRETCH.contains(&verb) {
            *self.stretch.entry(verb.to_string()).or_default() += 1;
            return;
        }
        if !EFFECTS.contains(&verb) {
            let shown = if verb.is_empty() { text } else { verb };
            return self.err(line, col, format!("`{shown}` is not an effect"));
        }
        let core = before_when(text);
        let words: Vec<&str> = core.split_whitespace().collect();
        let word_col = |n: usize| col + cols(&core[..core.find(words[n]).unwrap_or(0)]);
        let arg = |n: usize| words.get(n).copied().unwrap_or("");
        match verb {
            "learn" => self.need(arg(1), &["clue"], "clue", line, col + 6),
            "teach" => {
                if words.len() < 3 {
                    self.err(line, col, "`teach` is written `teach who clue`");
                } else {
                    self.need(arg(2), &["clue"], "clue", line, word_col(2));
                }
            }
            "gain" | "give" => self.need(arg(1), &["item"], "item", line, col + 5),
            "burn" | "light" => self.need(arg(1), &["fuse"], "fuse", line, col + cols(verb) + 1),
            "roll" => self.need(arg(1), &["table"], "table", line, col + 5),
            "wake" => self.need(arg(1), &["npc", "character"], "NPC", line, col + 5),
            "play" => match arg(1) {
                "" => self.err(line, col, "`play` needs a passage name"),
                name => self.need_passage(name, "the passage", line, word_col(1)),
            },
            "set" => {
                if !core.contains(':') {
                    self.err(line, col, "`set` is written `set name: value`");
                }
            }
            "add" | "strike" | "remove" => {
                let marker = match verb {
                    "add" => "to",
                    "strike" => "on",
                    _ => "from",
                };
                match words.iter().rposition(|w| *w == marker) {
                    Some(i) if i + 1 < words.len() => {
                        self.need(words[i + 1], &["list"], "list", line, word_col(i + 1));
                    }
                    _ => self.err(
                        line,
                        col,
                        format!("`{verb}` is written `{verb} item {marker} list`"),
                    ),
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ bodies

    fn body(&mut self, lines: &[Line]) {
        #[derive(PartialEq)]
        enum Last {
            Nothing,
            Fork,
            Speech,
        }
        let mut last = Last::Nothing;
        let mut if_seen = false;
        for l in lines {
            let t = l.text.trim();
            let col = 1 + cols(&l.text[..l.text.len() - l.text.trim_start().len()]);
            if t.is_empty() {
                last = Last::Nothing;
            } else if t.starts_with('>') {
                self.fork(l, col);
                last = Last::Fork;
            } else if let Some(more) = t.strip_prefix('~') {
                match last {
                    Last::Fork => {
                        for c in split_clauses(&l.text, l.no, 1) {
                            self.link_clause(&c);
                        }
                    }
                    Last::Speech => self.inserts(more, l.no, col + 1),
                    Last::Nothing => self.err(
                        l.no,
                        col,
                        "`~` continues a Link or a speech, but there is nothing on the line before to continue",
                    ),
                }
            } else if t.starts_with('[') && !t.starts_with("[[") && t.ends_with(']') {
                self.modifier(t, l.no, col, &mut if_seen);
                last = Last::Nothing;
            } else if let Some(cue) = t.strip_prefix('@') {
                self.speech(cue, l.no, col + 1);
                last = Last::Speech;
            } else {
                self.inserts(t, l.no, col);
                last = Last::Speech;
            }
        }
    }

    fn fork(&mut self, l: &Line, col: usize) {
        let text = &l.text;
        let Some(open) = text.find("[[") else {
            return self.err(l.no, col, "a Link line is written `> [[label->target]]`");
        };
        if !text[text.find('>').map_or(0, |i| i + 1)..open]
            .trim()
            .is_empty()
        {
            return self.err(l.no, col, "a Link line is written `> [[label->target]]`");
        }
        let Some(close) = text[open..].find("]]").map(|i| open + i) else {
            return self.err(
                l.no,
                cols(&text[..open]) + 1,
                "this Link is missing its closing `]]`",
            );
        };
        let inner = &text[open + 2..close];
        let (label, target, target_at) = if let Some(i) = inner.rfind("->") {
            (&inner[..i], &inner[i + 2..], open + 2 + i + 2)
        } else if let Some(i) = inner.rfind('|') {
            (&inner[..i], &inner[i + 1..], open + 2 + i + 1)
        } else {
            (inner, inner, open + 2)
        };
        self.stats.links += 1;
        if label.trim().is_empty() {
            self.err(l.no, cols(&text[..open]) + 3, "this Link has no label");
        }
        let target = target.trim();
        let tcol = cols(&text[..target_at]) + 1;
        if target.is_empty() {
            self.err(l.no, tcol, "this Link has no target after `->`");
        } else if !BUILTIN_TARGETS.contains(&target) {
            self.need_passage(target, "the Link target", l.no, tcol);
        }
        let rest = &text[close + 2..];
        if rest.trim().is_empty() {
            return;
        }
        if !rest.trim_start().starts_with('~') {
            return self.err(
                l.no,
                cols(&text[..close + 2]) + 1 + cols(&rest[..rest.len() - rest.trim_start().len()]),
                "after `]]` only `~` clauses may follow",
            );
        }
        for c in split_clauses(rest, l.no, cols(&text[..close + 2]) + 1) {
            self.link_clause(&c);
        }
    }

    fn link_clause(&mut self, c: &Clause) {
        let word = first_word(&c.text);
        if !LINK_CLAUSES.contains(&word) && !self.has(word, &["act", "combo"]) {
            let shown = if word.is_empty() {
                c.text.as_str()
            } else {
                word
            };
            self.err(
                c.line,
                c.col,
                format!("`{shown}` is neither a Link clause nor a declared act"),
            );
            return;
        }
        let arg = c.text[word.len()..].split_whitespace().next().unwrap_or("");
        let arg_col = c.col + cols(word) + 1;
        match word {
            "take" => self.need(arg, &["item"], "item", c.line, arg_col),
            "become" | "order" => self.need(arg, &["character"], "character", c.line, arg_col),
            _ => {}
        }
        self.clause_common(c, true);
    }

    fn speech(&mut self, cue: &str, line: usize, col: usize) {
        self.stats.speeches += 1;
        let who = first_word(cue);
        if who.is_empty() {
            return self.err(line, col, "`@` needs a speaker: `@leo Hello.`");
        }
        self.need(
            who,
            &["character", "npc", "voice"],
            "character, NPC or voice",
            line,
            col,
        );
        let mut rest = cue[who.len()..].trim_start();
        if rest.starts_with('(') {
            match rest.find(')') {
                Some(i) => rest = &rest[i + 1..],
                None => {
                    return self.err(
                        line,
                        col + cols(who) + 1,
                        "this stage direction is missing its closing `)`",
                    );
                }
            }
        }
        let rest_col = col + cols(&cue[..cue.len() - rest.len()]);
        self.inserts(rest, line, rest_col);
    }

    /// Every `{…}` in a line of prose.
    fn inserts(&mut self, text: &str, line: usize, col: usize) {
        let mut from = 0;
        while let Some(i) = text[from..].find('{') {
            let open = from + i;
            let Some(close) = text[open..].find('}').map(|j| open + j) else {
                return self.err(line, col + cols(&text[..open]), "this `{` is never closed");
            };
            self.insert(&text[open + 1..close], line, col + cols(&text[..open]) + 1);
            from = close + 1;
        }
    }

    fn insert(&mut self, content: &str, line: usize, col: usize) {
        let c = content.trim();
        let mut words = c.split_whitespace();
        let first = words.next().unwrap_or("");
        let second = words.next().unwrap_or("");
        match first {
            "present" | "echoes" | "claimants" | "passers" if second.is_empty() => {}
            "retold" if second.is_empty() => {
                *self.stretch.entry("{retold}".into()).or_default() += 1
            }
            "embed" if second == "nib:" => {
                let name = c.split('\'').nth(1).unwrap_or("");
                if name.is_empty() {
                    self.err(
                        line,
                        col,
                        "`embed nib:` needs a passage name in quotes: {embed nib: 'name'}",
                    );
                } else {
                    self.need_passage(name, "the embedded passage", line, col);
                }
            }
            "embed" if second == "image:" || second == "video:" => {}
            "reveal" if second == "link:" => {}
            "sound" if second == "effect:" => {}
            "ambient" if second == "sound:" => {}
            "accuse" => {}
            "list" => self.need(second, &["list"], "list", line, col + 5),
            _ if second == "input" => {}
            _ => {
                self.stats.expressions += 1;
                let col = col + cols(&content[..content.len() - content.trim_start().len()]);
                match expr::parse(c) {
                    Ok(e) => self.resolve(&e, c, line, col),
                    Err((_, why)) => self.err(
                        line,
                        col,
                        format!("`{{{c}}}` is not a known insert, and it does not read as an expression ({why})"),
                    ),
                }
            }
        }
    }

    /// A `[modifier]` line; several may be combined with `;`.
    fn modifier(&mut self, text: &str, line: usize, col: usize, if_seen: &mut bool) {
        // `strip_prefix`/`strip_suffix` cannot panic the way slicing by index can.
        let Some(inner) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) else {
            return self.err(line, col, "this modifier is missing its closing `]`");
        };
        let mut at = 0;
        for part in inner.split(';') {
            let lead = part.len() - part.trim_start().len();
            let pcol = col + 1 + cols(&inner[..at + lead]);
            at += part.len() + 1;
            let part = part.trim();
            let word = first_word(part);
            let rest = part[word.len()..].trim();
            match word {
                "if" | "unless" | "ifalways" | "ifnever" => {
                    *if_seen = true;
                    if rest.is_empty() {
                        self.err(line, pcol, format!("`[{word}]` needs a condition"));
                    } else {
                        self.expr(rest, line, pcol + cols(word) + 1);
                    }
                }
                "else" => {
                    if !*if_seen {
                        self.err(line, pcol, "`[else]` has no `[if …]` before it");
                    }
                }
                "end" | "continue" | "cont" => *if_seen = false,
                "append" | "once" | "note" | "todo" | "fixme" | "verse" => {}
                "after" => {
                    if !is_duration(rest) {
                        self.err(line, pcol, "`[after]` needs a time such as `2s` or `400ms`");
                    }
                }
                "beat" => {
                    if !rest.is_empty() && !is_duration(rest) {
                        self.err(line, pcol, "`[beat]` takes a time such as `2s`, or nothing");
                    }
                }
                "reveal" => {
                    if !["all", "by speech", "by word", "by letter"].contains(&rest) {
                        self.err(
                            line,
                            pcol,
                            "`[reveal …]` is one of: all, by speech, by word, by letter",
                        );
                    }
                }
                _ => self.err(line, pcol, format!("unknown modifier `[{part}]`")),
            }
        }
    }
}

impl Lookup for Checker {
    fn is_known(&self, name: &str) -> bool {
        resolve::BUILTINS.contains(&name)
            || self.decls.contains_key(name)
            || self.tags.contains(name)
            || self.locals.contains(name)
            || self.globals.contains(name)
            || self.passages.contains(name)
    }

    fn is_kind(&self, name: &str, kind: &str) -> bool {
        self.has(name, &[kind])
    }
}
