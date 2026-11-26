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
pub const VERSION: &str = "0.6";

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
    c.costs();
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
    /// Characters `~ one per linker`: `guest_2` names a seat of `guest`.
    seats: HashSet<String>,
    /// passage → the names it introduces: `let`s, input variables, and the
    /// `with {…}` parameters of Links that lead to it.
    scopes: HashMap<String, HashSet<String>>,
    /// `let`s in the config, visible everywhere.
    globals: HashSet<String>,
    /// The names in scope in the passage being checked right now.
    locals: HashSet<String>,
    stretch: BTreeMap<String, usize>,
    stats: Stats,
    /// The Inker's Link ids in the passage being checked: `~ id burn`.
    link_ids: HashSet<String>,
    /// The passage being checked, and its Links so far, for forks.
    here: String,
    links_here: Vec<LinkSeen>,
    /// Where the first fork is, if the Inklink has any.
    first_fork: Option<(usize, usize)>,
    /// `~ plants x` and `~ pays off x`: where each was first seen.
    plants: BTreeMap<String, (usize, usize)>,
    pays: BTreeMap<String, (usize, usize)>,
    /// Where the first cost is, and `default cost: vults` if the config has one.
    first_cost: Option<(usize, usize)>,
    default_cost: Option<(String, usize, usize)>,
    /// What is open in the body being checked, innermost last: an `[if]`
    /// chain, a `[quote …]` or a `[redacted until …]`, and where. `[end]`
    /// closes the innermost.
    blocks: Vec<(&'static str, usize, usize)>,
}

/// A Link seen in the passage being checked.
#[derive(Default)]
struct LinkSeen {
    line: usize,
    col: usize,
    target: String,
    inker_id: Option<String>,
    /// For a fork: where its `~ inquest` clause is, and the Link it names.
    fork: Option<(usize, usize)>,
    stands_in_for: Option<(String, usize, usize)>,
    pending: bool,
    rejoin: bool,
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

    /// A `where:` condition: an expression in which `it` is each candidate.
    fn where_expr(&mut self, text: &str, line: usize, col: usize) {
        let body = text.trim();
        let col = col + cols(&text[..text.len() - text.trim_start().len()]);
        self.stats.expressions += 1;
        match expr::parse(body) {
            Ok(e) => {
                for Problem { at, name, .. } in resolve::unresolved_where(&e, self) {
                    self.err(
                        line,
                        col + cols(&body[..at.min(body.len())]),
                        format!("`{name}` is not declared: it is not a builtin, a declared id, a tag, or a `let` in scope"),
                    );
                }
            }
            Err((off, msg)) => {
                let off = off.min(body.len());
                self.err(line, col + cols(&body[..off]), msg);
            }
        }
    }

    /// `~ at item here where: not it.lit`: a kind, `here`, and a condition.
    fn target_clause(&mut self, c: &Clause) {
        let rest = c.text["at".len()..].trim_start();
        let rest_col = c.col + cols(&c.text[..c.text.len() - rest.len()]);
        let (head, condition) = match rest.split_once("where:") {
            Some((h, w)) => (h, Some((w, rest_col + cols(h) + 6))),
            None => (rest, None),
        };
        let mut words = head.split_whitespace();
        let kind = words.next().unwrap_or("");
        if !self.is_kind_word(kind) {
            self.err(
                c.line,
                rest_col,
                format!("`~ at` names a kind of thing, such as `item` or `place`, then `here` or `where:`; `{kind}` is not one"),
            );
        }
        for w in words {
            if w != "here" {
                self.err(
                    c.line,
                    rest_col,
                    format!("`~ at {kind}` may be followed by `here` and `where: …`, not `{w}`"),
                );
            }
        }
        if let Some((w, col)) = condition {
            self.where_expr(w, c.line, col);
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

    /// A `get` is worked out each time it is read, so one that depends on
    /// itself, directly or through other `get`s, would never finish.
    fn get_cycles(&mut self, stmts: &[Stmt]) {
        let mut deps: HashMap<String, (Vec<String>, usize, usize)> = HashMap::new();
        for s in stmts.iter().filter(|s| s.kw == "get") {
            let head = &s.clauses[0];
            let rest = head.text[3..].trim_start();
            let Some((name, value)) = rest.split_once(':') else {
                continue;
            };
            if let Ok(e) = expr::parse(before_when(value.trim())) {
                deps.insert(
                    name.trim().to_string(),
                    (resolve::names(&e), head.line, head.col),
                );
            }
        }
        let mut names: Vec<&String> = deps.keys().collect();
        names.sort();
        for start in names {
            // Depth-first from `start`, looking for a way back to it.
            let mut stack = vec![(start.clone(), vec![start.clone()])];
            let mut seen = HashSet::new();
            while let Some((at, path)) = stack.pop() {
                for next in &deps[&at].0 {
                    if next == start {
                        let (_, line, col) = deps[start];
                        let chain = format!("{} → {start}", path.join(" → "));
                        self.err(
                            line,
                            col,
                            format!("`get {start}` depends on itself: {chain}"),
                        );
                        return;
                    }
                    if deps.contains_key(next) && seen.insert(next.clone()) {
                        let mut longer = path.clone();
                        longer.push(next.clone());
                        stack.push((next.clone(), longer));
                    }
                }
            }
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
                // A name that stands for several (`~ one per linker`, `~ count
                // 4`) is a kind of its own: `~ at guest`.
                if s.clauses[1..]
                    .iter()
                    .any(|c| c.text == "one per linker" || c.text.starts_with("count "))
                {
                    self.tags.insert(name.to_string());
                }
                if s.clauses[1..].iter().any(|c| c.text == "one per linker") {
                    self.seats.insert(name.to_string());
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
        for s in stmts.iter().filter(|s| s.kw == "let" || s.kw == "get") {
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
        for t in &p.tags {
            if STRETCH_TAGS.contains(&t.as_str()) {
                *self.stretch.entry(format!("[{t}]")).or_default() += 1;
            }
        }
        self.locals = self.scope_of(&p.name);
        self.link_ids.clear();
        self.here = p.name.clone();
        self.links_here.clear();
        let mut diags = std::mem::take(&mut self.diags);
        let (stmts, body) = gather(p, &mut diags);
        self.diags = diags;
        for s in &stmts {
            self.stmt(s);
        }
        self.body(body);
        self.forks_here();
        self.get_cycles(&stmts);
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
            Some(v) if v.starts_with("0.5") => self.err(
                p.line,
                1,
                format!(
                    "this file targets Inkjot {v}; this compiler reads {VERSION}.x. \
                     `inkjot migrate` rewrites a v0.5 file for v0.6"
                ),
            ),
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
        // `once per turn | day`: a day needs the clock to say how long it is.
        let clock = stmts.iter().find(|s| s.kw == "clock");
        let has_day =
            clock.is_some_and(|s| s.clauses[1..].iter().any(|c| c.text.starts_with("day ")));
        let word = clock
            .and_then(|s| {
                s.clauses[1..]
                    .iter()
                    .find_map(|c| c.text.strip_prefix("shown as "))
            })
            .map(|w| w.trim_matches('"').to_string());
        for p in passages {
            let mut scratch = Vec::new();
            let (stmts, _) = gather(p, &mut scratch);
            for st in &stmts {
                for c in &st.clauses[1..] {
                    let Some(period) = c.text.strip_prefix("once per ") else {
                        continue;
                    };
                    let period = period.trim();
                    let col = c.col + cols("once per ");
                    match period {
                        "turn" | "chronon" => {}
                        p if word.as_deref() == Some(p) => {}
                        "day" if has_day => {}
                        "day" => self.err(
                            c.line,
                            col,
                            "`once per day` needs days on the clock: `clock: … ~ day n`",
                        ),
                        _ => self.err(
                            c.line,
                            col,
                            "`once per` takes `turn`, `day`, or the clock's own word",
                        ),
                    }
                }
            }
        }
        if let Some((line, col)) = self.first_fork {
            if value("mode").is_some_and(|(m, ..)| m == "cohort") {
                self.err(
                    line,
                    col,
                    "Inquests are for `mode: solo`; a cohort cannot take forks",
                );
            }
            if !self.has("inquests", &["resource"]) {
                self.diags.push(Diag::notice(
                    line,
                    col,
                    "Linkers can ask for forks only when the Inklink declares `resource inquests: n`",
                ));
            }
        }
        let plants = std::mem::take(&mut self.plants);
        let pays = std::mem::take(&mut self.pays);
        for (name, (line, col)) in &plants {
            if !pays.contains_key(name) {
                self.diags.push(Diag::notice(
                    *line,
                    *col,
                    format!("`{name}` is planted but never paid off"),
                ));
            }
        }
        for (name, (line, col)) in &pays {
            if !plants.contains_key(name) {
                self.diags.push(Diag::notice(
                    *line,
                    *col,
                    format!("`{name}` is paid off but never planted"),
                ));
            }
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

    fn cost_seen(&mut self, line: usize, col: usize) {
        if self.first_cost.is_none() {
            self.first_cost = Some((line, col));
        }
    }

    /// Costs spend the resource `default cost` names. Without it nothing is
    /// spent, which is allowed but almost never meant.
    fn costs(&mut self) {
        match (self.default_cost.clone(), self.first_cost) {
            (Some((name, line, col)), _) => {
                if !self.has(&name, &["resource", "stat"]) {
                    self.err(
                        line,
                        col,
                        format!("`default cost` names the resource acts spend; `{name}` is not a declared resource"),
                    );
                }
            }
            (None, Some((line, col))) => self.diags.push(Diag::notice(
                line,
                col,
                "this costs nothing until the config says what acts spend: `default cost: vults`",
            )),
            (None, None) => {}
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
        if let Some((_, now)) = RETIRED.iter().find(|(w, _)| *w == kw) {
            return self.err(head.line, head.col, now.to_string());
        }
        if RESERVED_LATER.contains(&kw) {
            return self.err(
                head.line,
                head.col,
                format!("`{kw}` is reserved for a part of Inkjot v0.6 that is not built yet"),
            );
        }
        let rest = head.text[kw.len()..].trim_start();
        let rest_col = head.col + cols(&head.text[..head.text.len() - rest.len()]);

        if kw == "told" {
            return self.err(
                head.line,
                head.col,
                "`told:` is now `narration:` (`narration: second person`); `inkjot migrate` rewrites it",
            );
        }
        // A test takes a fork by its label: `take fork: "Suppose …"`.
        if kw == "take" && rest.starts_with("fork:") {
            let value = rest["fork:".len()..].trim();
            if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
                self.err(
                    head.line,
                    rest_col,
                    "`take fork:` takes the fork's label in quotes",
                );
            }
            return;
        }
        if kw == "on" || kw == "upon" {
            self.on(rest, head.line, rest_col);
        } else if let Some(kind) = DECLS.iter().copied().find(|k| *k == kw) {
            self.decl(kind, s, rest, rest_col);
        } else if is_effect(kw, rest) {
            self.effect(&head.text, head.line, head.col);
        } else if SETTINGS.contains(&kw) {
            self.setting(kw, s, rest, rest_col);
        } else if self.has(kw, &["resource", "stat"]) {
            // `hp 3 when …`: a conditional value for a declared resource or stat.
        } else {
            return self.err(head.line, head.col, format!("unknown statement `{kw}`"));
        }

        for (i, c) in s.clauses.iter().enumerate() {
            // `act walk: cost 2` or `~ cost 2 when …`.
            let text = if i == 0 {
                c.text.split_once(": ").map_or("", |(_, v)| v)
            } else {
                c.text.as_str()
            };
            if text.starts_with("cost ") {
                self.cost_seen(c.line, c.col);
            }
            // `spreads … ~ then set it.heat: 1`: `it` is each place reached.
            let it = i > 0 && kw == "spreads" && self.locals.insert("it".to_string());
            self.clause_common(c, i > 0);
            if it {
                self.locals.remove("it");
            }
            if i > 0 && matches!(kw, "item" | "npc" | "character") {
                self.actor_clause(c);
            }
        }
    }

    /// `~ for 3 turns`, `~ after 2 turns when lit`: a number of turns, or a
    /// name that holds one.
    fn timing(&mut self, c: &Clause) {
        let words: Vec<&str> = before_when(&c.text).split_whitespace().collect();
        let timed = matches!(words.first(), Some(&("for" | "after")))
            && matches!(words.last(), Some(&("turn" | "turns")));
        if !timed {
            return;
        }
        match words.as_slice() {
            [_, n, _] if n.parse::<u32>().is_ok() => {}
            [lead, n, _] => self.expr(n, c.line, c.col + cols(lead) + 1),
            _ => self.err(
                c.line,
                c.col,
                "a time is `~ for 3 turns` or `~ after 3 turns`, after the effect it belongs to",
            ),
        }
    }

    /// Checks shared by every clause: `when`, `until`, `then`, `check`, `like`, table rows.
    fn clause_common(&mut self, c: &Clause, is_tail: bool) {
        if let Some(i) = after_word(&c.text, "when") {
            self.expr(&c.text[i..], c.line, c.col + cols(&c.text[..i]));
        }
        for lead in [
            "hidden until ",
            "fixed until ",
            "offstage until ",
            "redacted until ",
        ] {
            if let Some(e) = c.text.strip_prefix(lead) {
                self.expr(e, c.line, c.col + cols(lead));
            }
        }
        if let Some(stat) = c.text.strip_prefix("check ") {
            self.need(first_word(stat), &["stat"], "stat", c.line, c.col + 6);
        }
        self.timing(c);
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
                } else if row.starts_with('"') {
                    // A table read as a value: `0..=19: "a lamp carrier"`.
                    self.strings(row, c.line, col);
                } else if row != "nothing" {
                    self.expr(row, c.line, col);
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
            ["knows", fact] => self.need(fact, &["fact"], "fact", c.line, c.col + 6),
            ["moves", ..] => self.moves(&c.text[5..], c.line, c.col + 5),
            // `follows me when self.with_me`: only the pov is followed.
            ["follows", who, ..] if *who != "me" => self.err(
                c.line,
                c.col + 8,
                "an NPC follows the pov: `~ follows me`, or `~ follows me when …`",
            ),
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
        if RESERVED.contains(&name) {
            return self.err(
                head.line,
                rest_col,
                format!("`{name}` is a reserved word and cannot name a {kind}"),
            );
        }
        let after = rest[name.len()..].trim_start();
        // When there is no value, there is no colon: `archetype statue ~ lifeless`.
        let value = match after.strip_prefix(':') {
            Some(value) => value,
            None if after.is_empty() => "",
            None => {
                return self.err(
                    head.line,
                    rest_col + cols(name),
                    format!("expected `:` after the name `{name}`"),
                )
            }
        };
        let value_col = rest_col + cols(&rest[..rest.len() - value.len()]);
        // Quoted text in the value and in clauses that hold text, not
        // conditions: labels, `shown as`, `formally`, list entries.
        if value.contains('"') {
            self.strings(value, head.line, value_col);
        }
        for c in &s.clauses[1..] {
            let text = c.text.as_str();
            if text == "playable" {
                self.err(
                    c.line,
                    c.col,
                    "`~ playable` is now `~ selectable`: every character can be played, and this one is offered at the start; `inkjot migrate` rewrites it",
                );
            }
            // `~ pick cycle`: how `{x.aka}` picks from the names.
            if let Some(how) = text.strip_prefix("pick ") {
                if !AKA_PICKS.contains(&how.trim()) {
                    self.err(
                        c.line,
                        c.col + 5,
                        "`pick` is one of: shuffled, cycle, random, first",
                    );
                }
            }
            if let Some(how) = text.strip_prefix("aka ").filter(|h| !h.starts_with('"')) {
                self.err(
                    c.line,
                    c.col,
                    format!(
                        "how `aka` picks is its own clause in v0.6: `~ pick {}`",
                        how.trim()
                    ),
                );
            }
            if text.starts_with('"')
                || text.starts_with("shown as ")
                || text.starts_with("formally ")
                || text.starts_with("holds ")
            {
                self.strings(text, c.line, c.col);
            }
        }
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
            "act" | "combo" => {
                for c in &s.clauses[1..] {
                    if let Some(v) = c.text.strip_prefix("choice ") {
                        self.choice(v.trim(), c.line, c.col + 7);
                    }
                    if c.text.starts_with("at ") {
                        self.target_clause(c);
                    }
                }
                if kind == "combo" {
                    for act in value.split(',') {
                        self.need(act.trim(), &["act", "combo"], "act", head.line, value_col);
                    }
                }
            }
            // `deck roles ~ card mafia: members ~ card townsfolk: rest`.
            "deck" => {
                for c in &s.clauses[1..] {
                    if let Some((_, n)) =
                        c.text.strip_prefix("card ").and_then(|t| t.split_once(':'))
                    {
                        if n.trim() != "rest" {
                            self.expr(n.trim(), c.line, c.col);
                        }
                    }
                }
            }
            // `pile hand ~ per guest`: whose it is.
            "pile" => {
                for c in &s.clauses[1..] {
                    if let Some(who) = c.text.strip_prefix("per ") {
                        let who = who.trim();
                        if !self.is_kind_word(who) && !self.has(who, &["character", "npc"]) {
                            self.err(
                                c.line,
                                c.col + 4,
                                format!("a pile is `~ per` a kind of person, such as a character `~ one per linker`; `{who}` is not one"),
                            );
                        }
                    }
                }
            }
            // `threat flame: fire ~ at roland ~ answered by prevent, block`.
            "threat" => {
                let first = Clause {
                    text: value.to_string(),
                    line: head.line,
                    col: value_col,
                };
                for c in std::iter::once(&first).chain(&s.clauses[1..]) {
                    if let Some(who) = c.text.strip_prefix("at ") {
                        self.expr(who, c.line, c.col + 3);
                    }
                    if let Some(kinds) = c.text.strip_prefix("answered by ") {
                        for k in kinds.split(',').map(str::trim) {
                            if !COUNTERS.contains(&k) {
                                self.err(
                                    c.line,
                                    c.col + 12,
                                    format!(
                                        "`{k}` is not a counter; counters are: {}",
                                        COUNTERS.join(", ")
                                    ),
                                );
                            }
                        }
                    }
                    if let Some(e) = c.text.strip_prefix("on fail:") {
                        self.effect(e.trim(), c.line, c.col + 9);
                    }
                }
            }
            _ => {}
        }
    }

    fn one_of(&mut self, value: &str, allowed: &[&str], what: &str, line: usize, col: usize) {
        if !allowed.contains(&value) {
            self.err(
                line,
                col,
                format!("`{what}` is one of: {}", allowed.join(", ")),
            );
        }
    }

    /// `~ inquest`, `~ inquest for burn`, `~ inquest from ch3, ch4`,
    /// `~ inquest by monkeypaw`: this Link is a fork.
    fn inquest_clause(&mut self, c: &Clause) {
        let rest = c.text["inquest".len()..].trim();
        let rest_col = c.col + cols(&c.text[..c.text.len() - rest.len()]);
        self.first_fork.get_or_insert((c.line, c.col));
        if let Some(l) = self.links_here.last_mut() {
            l.fork = Some((c.line, c.col));
        }
        let (how, list) = rest.split_once(' ').unwrap_or((rest, ""));
        match how {
            "" => {}
            "for" if is_ident(list.trim()) => {
                if let Some(l) = self.links_here.last_mut() {
                    l.stands_in_for = Some((list.trim().to_string(), c.line, rest_col + 4));
                }
            }
            "from" if !list.trim().is_empty() => {
                for name in list.split(',') {
                    self.need_passage(name.trim(), "the passage a fork may be asked from", c.line, rest_col + 5);
                }
            }
            "by" => {
                if !ROLES.contains(&list.trim()) || list.trim() == "inker" {
                    self.err(
                        c.line,
                        rest_col + 3,
                        "a pending fork is written by a role: monkeypaw, ghostwriter or storyteller",
                    );
                }
                if let Some(l) = self.links_here.last_mut() {
                    l.pending = true;
                }
            }
            _ => self.err(
                c.line,
                c.col,
                "a fork is `~ inquest`, `~ inquest for id`, `~ inquest from passage` or `~ inquest by role`",
            ),
        }
    }

    /// After a passage: every fork stands in for an offered Link here, and
    /// a pending fork says where it rejoins.
    fn forks_here(&mut self) {
        let links = std::mem::take(&mut self.links_here);
        let mut derived: HashMap<String, usize> = HashMap::new();
        let ids: Vec<String> = links
            .iter()
            .map(|l| match &l.inker_id {
                Some(id) => id.clone(),
                None => {
                    let base = format!("{}->{}", self.here, l.target);
                    let n = derived.entry(base.clone()).or_default();
                    *n += 1;
                    if *n == 1 {
                        base
                    } else {
                        format!("{base}#{n}")
                    }
                }
            })
            .collect();
        let has_forks = links.iter().any(|l| l.fork.is_some());
        for (i, l) in links.iter().enumerate() {
            let Some((line, col)) = l.fork else {
                continue;
            };
            match &l.stands_in_for {
                Some((name, line, col)) => {
                    let offered = links
                        .iter()
                        .zip(&ids)
                        .any(|(o, id)| o.fork.is_none() && id == name);
                    if !offered {
                        self.err(
                            *line,
                            *col,
                            format!("`{name}` is not an offered Link in this passage"),
                        );
                    }
                }
                None => {
                    if !links[..i].iter().any(|o| o.fork.is_none()) {
                        self.err(
                            line,
                            col,
                            "a fork stands in for an offered Link: put it below one, or name it with `~ inquest for id`",
                        );
                    }
                }
            }
            if l.pending && !l.rejoin {
                self.err(
                    line,
                    col,
                    "a pending fork says where it rejoins: `~ rejoin ch5`",
                );
            }
        }
        if has_forks {
            for l in links.iter().filter(|l| l.inker_id.is_none()) {
                self.diags.push(Diag::notice(
                    l.line,
                    l.col,
                    "give this Link an id (`~ id name`), so the Inker's data still points at it after a revision",
                ));
            }
        }
    }

    /// `~ choice proactive`: one of the three kinds of choice.
    fn choice(&mut self, value: &str, line: usize, col: usize) {
        if !CHOICES.contains(&value) {
            self.err(
                line,
                col,
                "`choice` is one of: proactive, reactive, inactive",
            );
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
            "hint" => self.strings(value, line, vcol),
            // A passage's own title, for the Linker: `title: "The Millpond"`.
            "title" => {
                if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
                    self.err(
                        line,
                        vcol,
                        "a title is quoted text: `title: \"The Millpond\"`",
                    );
                }
            }
            // `pattern rhyme: flame, frost ~ repeat ~ recall first when …`.
            "pattern" => {
                let steps = rest.split_once(':').map_or("", |(_, v)| v);
                for step in steps.split(',').map(str::trim).filter(|t| !t.is_empty()) {
                    self.need(step, &["threat"], "threat", line, vcol);
                }
                for c in &s.clauses[1..] {
                    let words: Vec<&str> = before_when(&c.text).split_whitespace().collect();
                    match words.as_slice() {
                        ["repeat"] | ["one", "per", "round"] => {}
                        [threat, "first"] => self.need(threat, &["threat"], "threat", c.line, c.col),
                        [threat, "instead", "of", of] => {
                            self.need(threat, &["threat"], "threat", c.line, c.col);
                            self.need(of, &["threat"], "threat", c.line, c.col);
                        }
                        _ => self.err(
                            c.line,
                            c.col,
                            "a pattern's clause is `repeat`, `one per round`, `x first when …` or `x instead of y when …`",
                        ),
                    }
                }
            }
            // `encounter hask ~ round by round ~ acts last ~ until …`.
            "encounter" => {
                self.need(
                    first_word(rest),
                    &["npc", "character"],
                    "NPC",
                    line,
                    rest_col,
                );
                for c in &s.clauses[1..] {
                    if let Some(e) = c.text.strip_prefix("until ") {
                        self.expr(e, c.line, c.col + 6);
                    }
                }
            }
            // `recipe stretcher: stave, stave, sacking ~ using hammer ~ make`.
            "recipe" => {
                let (result, parts) = rest.split_once(':').unwrap_or((rest, ""));
                self.need(result.trim(), &["item"], "item", line, rest_col);
                for part in parts.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                    self.need(part, &["item"], "item", line, vcol);
                }
                if parts.trim().is_empty() {
                    self.err(
                        line,
                        rest_col,
                        "a recipe names what it takes: `recipe pawl: horseshoe`",
                    );
                }
                for c in &s.clauses[1..] {
                    if let Some(tools) = c.text.strip_prefix("using ") {
                        for tool in tools.split(',').map(str::trim) {
                            self.need(tool, &["item"], "item", c.line, c.col + 6);
                        }
                    }
                }
            }
            // `spreads 1 from ablaze to nearest(place, where: …)`.
            "spreads" => {
                let tail = rest.split_once(' ').map_or("", |(_, t)| t).trim();
                let (from, to) = match tail.strip_prefix("from ") {
                    Some(t) => match t.find(" to ") {
                        Some(i) => (Some(&t[..i]), &t[i + 4..]),
                        None => (Some(t), ""),
                    },
                    None => (None, tail.strip_prefix("to ").unwrap_or(tail)),
                };
                if let Some(from) = from {
                    self.expr(from, line, rest_col);
                }
                if to.trim().is_empty() {
                    self.err(line, rest_col, "`spreads` says where to: `spreads 1 from fires to nearest(place, where: …)`");
                } else {
                    self.expr(to, line, rest_col);
                }
            }
            // `measure nights: nights`: reported when the reading ends.
            "measure" => {
                let name = first_word(rest);
                if !is_ident(name) || colon.is_none() || value.is_empty() {
                    self.err(
                        line,
                        rest_col,
                        "a measure is written `measure name: expression`",
                    );
                } else if value
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .any(|w| POV_NAMES.contains(&w))
                {
                    self.err(
                        line,
                        vcol,
                        "a measure is about the whole reading, not one Linker: it cannot say `me` or `you`",
                    );
                } else {
                    self.expr(value, line, vcol);
                }
            }
            "narration" => {
                if !NARRATIONS.contains(&value) {
                    self.err(
                        line,
                        vcol,
                        "`narration` is one of: first person, second person, third person",
                    );
                }
                for c in &s.clauses[1..] {
                    if c.text != "omniscient" {
                        self.err(
                            c.line,
                            c.col,
                            "after `narration:` only `~ omniscient` may follow",
                        );
                    }
                }
            }
            "cue" => {
                let of = rest.split(':').next().unwrap_or("").trim();
                if !["names", "directions"].contains(&of) {
                    self.err(
                        line,
                        rest_col,
                        "`cue` is written `cue names: capital, bold` or `cue directions: italic`",
                    );
                }
                for style in value.split(',').map(str::trim) {
                    if !CUE_STYLES.contains(&style) {
                        self.err(
                            line,
                            vcol,
                            format!(
                                "`{style}` is not a cue style; they are: {}",
                                CUE_STYLES.join(", ")
                            ),
                        );
                    }
                }
            }
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
            // `undo` puts everything back; `echo` sends one Linker back to re-live.
            "rewind" => {
                if !["undo", "echo"].contains(&value) {
                    self.err(line, vcol, "`rewind` is `undo` or `echo`");
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
                if rest.starts_with("cost") {
                    self.default_cost = Some((value.to_string(), line, vcol));
                }
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
            // `vote: vell ~ how device ~ proof cufflink`: a ballot in a test.
            "vote" => self.need(
                first_word(value),
                &["character", "npc"],
                "suspect",
                line,
                vcol,
            ),
            "seed" => {
                if value.parse::<u64>().is_err() {
                    self.err(line, vcol, "`seed:` takes a whole number");
                }
            }
            // `expect events: ending_reached, died`: in a test, what the
            // engine reported since the last action.
            "expect" if rest.starts_with("events") => {
                for kind in value.split(',').map(str::trim) {
                    if !crate::engine::CATALOGUE.iter().any(|t| t.name == kind) {
                        let names: Vec<&str> =
                            crate::engine::CATALOGUE.iter().map(|t| t.name).collect();
                        self.err(
                            line,
                            vcol,
                            format!("`{kind}` is not an event; they are: {}", names.join(", ")),
                        );
                    }
                }
            }
            // `expect fork: "Suppose …" ~ for "Say …"`.
            "expect" if rest.starts_with("fork") => {
                if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
                    self.err(
                        line,
                        vcol,
                        "`expect fork:` takes the fork's label in quotes",
                    );
                }
                let for_ok = s.clauses[1..].iter().all(|c| {
                    c.text
                        .strip_prefix("for ")
                        .is_some_and(|l| l.trim().starts_with('"') && l.trim().ends_with('"'))
                });
                if !for_ok || s.clauses.len() > 2 {
                    self.err(
                        line,
                        vcol,
                        "`expect fork:` may say which Link it stands in for: `~ for \"label\"`",
                    );
                }
            }
            "expect" => {
                if value.is_empty() {
                    self.err(line, rest_col, "`expect:` needs an expression to check");
                } else {
                    self.expr(value, line, vcol);
                }
            }
            "let" | "get" => {
                let v = before_when(value);
                if v.is_empty() {
                    self.err(
                        line,
                        rest_col,
                        format!("`{kw}` is written `{kw} name: value`"),
                    );
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
        // `on turn ends`: the turn has nothing it happens to.
        if arg == "by" || (event == "turn" && arg == "ends") {
            return;
        }
        // A kind or an archetype names every thing of it: `on enter place`,
        // `on enter shaft`.
        let kinds: &[&str] = match event {
            "cross" => &["way"],
            "enter" | "leave" => &["place"],
            "take" => &["item"],
            "move" => &["npc", "character"],
            _ => &[],
        };
        if kinds.contains(&arg) || self.has(arg, &["archetype"]) {
            return;
        }
        match event {
            "act" => self.need_passage(arg, "the act", line, arg_col),
            "cross" | "enter" | "leave" | "take" => self.need(arg, kinds, kinds[0], line, arg_col),
            "move" => self.need(arg, kinds, "person", line, arg_col),
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
        if let Some((_, now)) = RETIRED.iter().find(|(w, _)| *w == verb) {
            return self.err(line, col, now.to_string());
        }
        if !EFFECTS.contains(&verb) {
            let shown = if verb.is_empty() { text } else { verb };
            return self.err(line, col, format!("`{shown}` is not an effect"));
        }
        let core = before_when(text);
        self.strings(core, line, col);
        let words: Vec<&str> = core.split_whitespace().collect();
        let word_col = |n: usize| col + cols(&core[..core.find(words[n]).unwrap_or(0)]);
        let arg = |n: usize| words.get(n).copied().unwrap_or("");
        match verb {
            "learn" => self.need(arg(1), &["fact"], "fact", line, col + 6),
            "teach" => {
                if words.len() < 3 {
                    self.err(line, col, "`teach` is written `teach who fact`");
                } else {
                    self.need(arg(2), &["fact"], "fact", line, word_col(2));
                }
            }
            // Counters: `prevent hask`, `block for roland`, `cover me`,
            // `absorb with lumen`; and `pin hask`, `skip hask this round`.
            "prevent" | "cover" | "pin" | "skip" => {
                self.who_arg(arg(1), line, col + cols(verb) + 1)
            }
            "block" => {
                if arg(1) != "for" {
                    self.err(line, col, "`block` is written `block for who`");
                } else {
                    self.who_arg(arg(2), line, col);
                }
            }
            "absorb" => {
                if arg(1) != "with" {
                    self.err(line, col, "`absorb` is written `absorb with item`");
                } else {
                    self.need(arg(2), &["item"], "item", line, col);
                }
            }
            // Cards: `shuffle roles`, `deal 1 from roles to each hand`.
            "shuffle" => self.need(arg(1), &["deck"], "deck", line, col + 8),
            "deal" => match words.as_slice() {
                [_, n, "from", deck, "to", "each", pile] => {
                    self.expr(n, line, col + 5);
                    self.need(deck, &["deck"], "deck", line, word_col(3));
                    self.need(pile, &["pile"], "pile", line, word_col(6));
                }
                _ => self.err(
                    line,
                    col,
                    "`deal` is written `deal 1 from deck to each pile`",
                ),
            },
            "end" if arg(1) != "encounter" => {
                self.err(line, col, "`end` is written `end encounter`");
            }
            // `gain salt 2`, `gain tally`, `give one(item, …) to gleaner`,
            // `give target to me`, `destroy all(item, …)`, `lose stave 2`.
            "gain" | "give" | "lose" | "destroy" => {
                let thing = core[verb.len()..].trim();
                let thing = match verb {
                    "give" => thing.rfind(" to ").map_or(thing, |i| thing[..i].trim()),
                    "gain" | "lose" => thing.split_whitespace().next().unwrap_or(""),
                    _ => thing,
                };
                let kinds: &[&str] = match verb {
                    "gain" | "lose" => &["item", "resource", "stat"],
                    _ => &["item"],
                };
                if is_ident(thing) && !self.is_known_local(thing) {
                    self.need(thing, kinds, kinds[0], line, col + cols(verb) + 1);
                } else if !is_ident(thing) && !thing.is_empty() {
                    self.expr(thing, line, col + cols(verb) + 1);
                }
            }
            "reveal" => self.need(
                arg(1),
                &["npc", "character", "item", "place", "fact"],
                "thing",
                line,
                col + 7,
            ),
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
            "add" | "strikethrough" | "remove" => {
                let marker = match verb {
                    "add" => "to",
                    "strikethrough" => "on",
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
        self.blocks.clear();
        for l in lines {
            let t = l.text.trim();
            let col = 1 + cols(&l.text[..l.text.len() - l.text.trim_start().len()]);
            if t.is_empty() {
                last = Last::Nothing;
            } else if t.starts_with('>') && !t.contains("[[") {
                // A Link group: its clauses go to every Link below it.
                let rest = t.trim_start_matches('>').trim_start();
                if !rest.starts_with('~') {
                    self.err(
                        l.no,
                        col,
                        "a Link line is written `> [[label->target]]`; a Link group is `> ~ clause`",
                    );
                }
                for c in split_clauses(&l.text, l.no, 1) {
                    if c.text != ">" {
                        self.link_clause(&c);
                    }
                }
                last = Last::Fork;
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
        for (what, line, col) in std::mem::take(&mut self.blocks) {
            // A flat `[if]` chain needs no `[end]`; a block does.
            if what == "if" {
                continue;
            }
            self.err(
                line,
                col,
                format!("this `[{what}]` is never closed: end it with `[end]`"),
            );
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
        } else {
            self.inserts(label, l.no, cols(&text[..open]) + 3);
        }
        let target = target.trim();
        let tcol = cols(&text[..target_at]) + 1;
        if target.is_empty() {
            self.err(l.no, tcol, "this Link has no target after `->`");
        } else if !BUILTIN_TARGETS.contains(&target) {
            self.need_passage(target, "the Link target", l.no, tcol);
        }
        self.links_here.push(LinkSeen {
            line: l.no,
            col,
            target: target.to_string(),
            ..LinkSeen::default()
        });
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
            "at" => self.target_clause(c),
            "uses" => {
                for tool in c.text[word.len()..].split(',').map(str::trim) {
                    self.need(tool, &["item"], "item", c.line, arg_col);
                }
            }
            "cost" => self.cost_seen(c.line, c.col),
            "choice" => self.choice(arg, c.line, arg_col),
            "inquest" => self.inquest_clause(c),
            "rejoin" => {
                for name in c.text[word.len()..].split(',') {
                    self.need_passage(name.trim(), "the passage a fork rejoins", c.line, arg_col);
                }
                if let Some(l) = self.links_here.last_mut() {
                    l.rejoin = true;
                }
            }
            "brief" => self.need(arg, &["brief"], "brief", c.line, arg_col),
            "credit" => {
                if !ROLES.contains(&arg) {
                    self.err(
                        c.line,
                        arg_col,
                        "`credit` is one of: inker, monkeypaw, ghostwriter, storyteller",
                    );
                }
            }
            "beat" => self.one_of(arg, BEATS, "beat", c.line, arg_col),
            "stakes" => self.one_of(arg, STAKES, "stakes", c.line, arg_col),
            "tone" => self.one_of(arg, TONES, "tone", c.line, arg_col),
            "weighs" => {
                for theme in c.text[word.len()..].split(',') {
                    self.need(theme.trim(), &["theme"], "theme", c.line, arg_col);
                }
            }
            "arc" => {
                let words: Vec<&str> = c.text.split_whitespace().collect();
                match words.as_slice() {
                    [_, who, way] => {
                        self.need(who, &["character", "npc"], "character", c.line, arg_col);
                        if !ARCS.contains(way) {
                            self.err(c.line, c.col, "an arc is `~ arc who rises | falls | holds`");
                        }
                    }
                    _ => self.err(c.line, c.col, "an arc is `~ arc who rises | falls | holds`"),
                }
            }
            "plants" | "pays" => {
                let list = match (word, c.text[word.len()..].trim_start().strip_prefix("off ")) {
                    ("pays", Some(list)) => list,
                    ("pays", None) => {
                        return self.err(c.line, c.col, "a payoff is `~ pays off name`");
                    }
                    _ => &c.text[word.len()..],
                };
                for name in list.split(',') {
                    let name = name.trim().to_string();
                    let seen = if word == "plants" {
                        &mut self.plants
                    } else {
                        &mut self.pays
                    };
                    seen.entry(name).or_insert((c.line, c.col));
                }
            }
            "predict" => {
                let n = arg.strip_suffix('%').and_then(|n| n.parse::<u32>().ok());
                if !n.is_some_and(|n| n <= 100) || c.text.split_whitespace().count() != 2 {
                    self.err(
                        c.line,
                        arg_col,
                        "a prediction is a share of Linkers: `~ predict 40%`",
                    );
                }
            }
            "reveals" => self.need(arg, &["fact"], "fact", c.line, arg_col),
            "id" => {
                if let Some(l) = self.links_here.last_mut() {
                    l.inker_id = Some(arg.to_string());
                }
                if !is_ident(arg) || c.text.split_whitespace().count() != 2 {
                    self.err(c.line, c.col, "a Link id is one name: `~ id burn_the_book`");
                } else if !self.link_ids.insert(arg.to_string()) {
                    self.err(
                        c.line,
                        arg_col,
                        format!("two Links in this passage have the id `{arg}`"),
                    );
                }
            }
            "take" => self.need(arg, &["item"], "item", c.line, arg_col),
            "become" | "order" => self.need(arg, &["character"], "character", c.line, arg_col),
            "hint" if !self.has("hints", &["resource"]) => self.err(
                c.line,
                c.col,
                "`~ hint` spends a hint request: declare `resource hints`",
            ),
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
        if !POV_NAMES.contains(&who) {
            self.need(
                who,
                &["character", "npc", "voice"],
                "character, NPC or voice",
                line,
                col,
            );
        }
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
        match crate::text::split(text) {
            Err((at, why)) => self.err(line, col + cols(&text[..at]), why),
            Ok(pieces) => {
                for p in pieces {
                    if let crate::text::Piece::Code(code, at) = p {
                        self.insert(&code, line, col + cols(&text[..at]));
                    }
                }
            }
        }
    }

    /// Every quoted string in `text`: its braces hold expressions, checked
    /// like any other.
    fn strings(&mut self, text: &str, line: usize, col: usize) {
        let mut from = 0;
        while let Some(i) = text[from..].find('"') {
            let open = from + i;
            let Some(close) = crate::text::string_end(text, open) else {
                return self.err(
                    line,
                    col + cols(&text[..open]),
                    "this string is never closed",
                );
            };
            let inner = &text[open + 1..close];
            let inner_col = col + cols(&text[..open + 1]);
            match crate::text::split(inner) {
                Err((at, why)) => self.err(line, inner_col + cols(&inner[..at]), why),
                Ok(pieces) => {
                    for p in pieces {
                        if let crate::text::Piece::Code(code, at) = p {
                            self.expr(&code, line, inner_col + cols(&inner[..at]));
                        }
                    }
                }
            }
            from = close + 1;
        }
    }

    fn insert(&mut self, content: &str, line: usize, col: usize) {
        let c = content.trim();
        if let Some(at) = single_quote(c) {
            return self.err(
                line,
                col + cols(&c[..at]),
                "text takes double quotes: {embed nib: \"name\"}",
            );
        }
        let mut words = c.split_whitespace();
        let first = words.next().unwrap_or("");
        let second = words.next().unwrap_or("");
        match first {
            "present" | "echoes" | "claimants" | "passers" | "ballots" if second.is_empty() => {}
            "retold" if second.is_empty() => {
                *self.stretch.entry("{retold}".into()).or_default() += 1
            }
            "embed" if second == "nib:" => {
                let name = c.split('"').nth(1).unwrap_or("");
                if name.is_empty() {
                    self.err(
                        line,
                        col,
                        "`embed nib:` needs a passage name in quotes: {embed nib: \"name\"}",
                    );
                } else {
                    self.need_passage(name, "the embedded passage", line, col);
                }
            }
            "embed" if second == "image:" || second == "video:" => {}
            "reveal" if second == "link:" => {}
            "sound" if second == "effect:" => {}
            "ambient" if second == "sound:" => {}
            "accuse" => self.accuse(&c[first.len()..], line, col),
            // `{redacted "TEN" until telegram.dry}`.
            "redacted" => {
                let text = c["redacted".len()..].trim_start();
                let close = text
                    .starts_with('"')
                    .then(|| crate::text::string_end(text, 0))
                    .flatten();
                match close {
                    Some(end) => {
                        self.strings(&text[..=end], line, col);
                        match text[end + 1..].trim().strip_prefix("until ") {
                            Some(cond) => self.expr(cond.trim(), line, col),
                            None => self.err(
                                line,
                                col,
                                "`{redacted \"…\"}` says until when: `{redacted \"TEN\" until telegram.dry}`",
                            ),
                        }
                    }
                    None => self.err(
                        line,
                        col,
                        "`{redacted …}` takes quoted text: `{redacted \"TEN\" until telegram.dry}`",
                    ),
                }
            }
            // `{list burning_near}`: a declared list, or a `let` that holds one.
            "list" if self.locals.contains(second) || self.globals.contains(second) => {}
            // `{list hand of each guest}`: a pile, for everyone of a kind.
            "list" if content.contains(" of each ") => {
                self.need(second, &["pile"], "pile", line, col + 5);
            }
            "present" if second == "linker" => {}
            "list" => self.need(second, &["list"], "list", line, col + 5),
            "all" if !second.is_empty() => {
                let rest = &content[content.find("all").unwrap_or(0) + 3..];
                self.expression_in(
                    rest,
                    line,
                    col + cols(&content[..content.len() - rest.len()]),
                );
            }
            _ if second == "input" => self.input(first, c, line, col),
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

    /// `{accuse who: suspect, how: scenario.methods, proof: proof_who and
    /// proof_how, revote: 1}`: the jury.
    fn accuse(&mut self, args: &str, line: usize, col: usize) {
        let mut seen = Vec::new();
        for part in crate::ir::split_args(args) {
            let Some((key, value)) = part.split_once(':') else {
                self.err(
                    line,
                    col,
                    format!("`{part}` in `{{accuse}}` needs a name: `who: suspect`"),
                );
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            seen.push(key.to_string());
            match key {
                "who" => {
                    if !self.is_kind_word(value) {
                        self.err(line, col, format!("`who: {value}` should name a kind of suspect: a tag such as `suspect`, or `npc`"));
                    }
                }
                "how" => self.expression_in(value, line, col),
                // Tags a conviction must cite: `proof_who and proof_how`.
                "proof" => {
                    if let Err((_, why)) = expr::parse(value) {
                        self.err(line, col, format!("`proof: {value}` does not read ({why})"));
                    }
                }
                "revote" => {
                    if value.parse::<u32>().is_err() {
                        self.err(line, col, "`revote` is a number of revotes: `revote: 1`");
                    }
                }
                other => self.err(
                    line,
                    col,
                    format!("`{{accuse}}` takes who, how, proof and revote, not `{other}`"),
                ),
            }
        }
        if !seen.iter().any(|k| k == "who") {
            self.err(
                line,
                col,
                "`{accuse}` needs `who:`, the kind of thing the jury may accuse",
            );
        }
    }

    /// `{name input for: me.name, max: 24}`, `{choose input for: words, from:
    /// cantrip, length: 3}`.
    fn input(&mut self, kind: &str, c: &str, line: usize, col: usize) {
        if !INPUTS.contains(&kind) {
            return self.err(
                line,
                col,
                format!(
                    "`{kind} input` is not an input; the inputs are: {}",
                    INPUTS.join(", ")
                ),
            );
        }
        let args = c.split_once("input").map_or("", |(_, r)| r);
        let mut has_for = false;
        for part in crate::ir::split_args(args) {
            let Some((key, value)) = part.split_once(':') else {
                self.err(line, col, format!("`{part}` needs a name: `for: me.name`"));
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            match key {
                "for" => has_for = true,
                "from" => {
                    if !self.is_kind_word(value) {
                        self.err(
                            line,
                            col,
                            format!("`from: {value}` should name a kind, a tag or an archetype"),
                        );
                    }
                }
                "max" | "min" | "length" if value.parse::<u32>().is_err() => {
                    self.err(line, col, format!("`{key}` is a number"));
                }
                _ => {}
            }
        }
        if !has_for && !matches!(kind, "act" | "note") {
            self.err(
                line,
                col,
                format!("`{kind} input` needs `for:`, the name its answer is kept in"),
            );
        }
    }

    /// Is `word` a kind of thing: a declared kind, a tag on something, or
    /// an archetype?
    fn is_kind_word(&self, word: &str) -> bool {
        is_ident(word)
            && (DECLS.contains(&word)
                || ["person", "linker", "living"].contains(&word)
                || self.has(word, &["archetype"])
                || self.tags.contains(word))
    }

    fn expression_in(&mut self, text: &str, line: usize, col: usize) {
        self.stats.expressions += 1;
        match expr::parse(text) {
            Ok(e) => self.resolve(&e, text, line, col),
            Err((_, why)) => self.err(line, col, format!("`{text}` does not read ({why})")),
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
                    if self.blocks.last().is_none_or(|b| b.0 != "if") {
                        self.blocks.push(("if", line, pcol));
                    }
                    if rest.is_empty() {
                        self.err(line, pcol, format!("`[{word}]` needs a condition"));
                    } else {
                        self.expr(rest, line, pcol + cols(word) + 1);
                    }
                }
                // `[if c; then e]`: e happens when the block is shown.
                "then" => {
                    if rest.is_empty() {
                        self.err(line, pcol, "`then` needs an effect: `[if c; then play x]`");
                    } else {
                        self.effect(rest, line, pcol + 5);
                    }
                }
                // `[quote letter ~ by "A. K."]` … `[end]`.
                "quote" => {
                    let mut clauses = rest.splitn(2, '~');
                    let kind = clauses.next().unwrap_or("").trim();
                    if !QUOTES.contains(&kind) {
                        self.err(
                            line,
                            pcol,
                            format!("`[quote …]` is one of: {}", QUOTES.join(", ")),
                        );
                    }
                    if let Some(by) = clauses.next().map(str::trim) {
                        match by.strip_prefix("by ").map(str::trim) {
                            Some(t) if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') => {
                                self.strings(t, line, pcol)
                            }
                            _ => self.err(
                                line,
                                pcol,
                                "a quote's author is quoted: `[quote letter ~ by \"A. K.\"]`",
                            ),
                        }
                    }
                    self.blocks.push(("quote", line, pcol));
                }
                // `[redacted until record_closed]` … `[end]`.
                "redacted" => {
                    match rest.strip_prefix("until ").map(str::trim) {
                        Some(c) if !c.is_empty() => self.expr(c, line, pcol + 15),
                        _ => self.err(
                            line,
                            pcol,
                            "`[redacted]` says until when: `[redacted until record_closed]`",
                        ),
                    }
                    self.blocks.push(("redacted", line, pcol));
                }
                "else" => {
                    if !*if_seen {
                        self.err(line, pcol, "`[else]` has no `[if …]` before it");
                    }
                }
                "end" | "continue" | "cont" => {
                    self.blocks.pop();
                    *if_seen = self.blocks.iter().any(|b| b.0 == "if");
                }
                "append" | "once" | "note" | "todo" | "fix" | "verse" => {}
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

impl Checker {
    /// Whom a counter names: a person, or a name for one (`target`, `me`).
    fn who_arg(&mut self, who: &str, line: usize, col: usize) {
        if who.is_empty() {
            return self.err(line, col, "say whom: `cover me`, `prevent hask`");
        }
        if is_ident(who) && !self.is_known_local(who) && !crate::syntax::POV_NAMES.contains(&who) {
            self.need(who, &["character", "npc"], "character or NPC", line, col);
        } else if !is_ident(who) {
            self.expr(who, line, col);
        }
    }

    /// A builtin (`target`, `it`) or a `let` in scope: not a declared id.
    fn is_known_local(&self, name: &str) -> bool {
        resolve::BUILTINS.contains(&name) || self.locals.contains(name) || name == "it"
    }
}

impl Lookup for Checker {
    fn is_known(&self, name: &str) -> bool {
        resolve::BUILTINS.contains(&name)
            || self.decls.contains_key(name)
            || name
                .rsplit_once('_')
                .is_some_and(|(base, n)| n.parse::<u32>().is_ok() && self.seats.contains(base))
            || self.tags.contains(name)
            || self.locals.contains(name)
            || self.globals.contains(name)
            || self.passages.contains(name)
    }

    fn is_kind(&self, name: &str, kind: &str) -> bool {
        self.has(name, &[kind])
    }
}

/// Where `text` has a `'` used as a quote: outside double quotes, and not
/// a thousands mark between digits (`1'000`).
fn single_quote(text: &str) -> Option<usize> {
    let b = text.as_bytes();
    let mut in_text = false;
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'"' => in_text = !in_text,
            b'\'' if !in_text => {
                let digits = i > 0
                    && b[i - 1].is_ascii_digit()
                    && b.get(i + 1).is_some_and(u8::is_ascii_digit);
                if !digits {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}
