//! A small interpreter over the IR: the first piece of an Inklinks runtime.
//!
//! It runs one Linker at a time, Link by Link: enter a passage, run its
//! header, render its body, offer its Links, take one. That is enough to play
//! a solo Inklink and to run `[test]` passages. Simultaneous turns, NPC
//! agendas and clashes are not here yet; anything the interpreter cannot do
//! is an error naming the feature, never a silent guess.

mod agenda;
mod cohort;
mod events;
mod inputs;
mod inquests;
pub mod test;
mod time;
mod value;

use events::Event;

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::json::Value;
pub use inputs::Ask;
pub use value::V;

/// An interpreter result. The error says what went wrong in plain words.
pub type R<T> = Result<T, String>;

/// A declaration, as the IR records it.
#[derive(Debug, Clone)]
struct Decl {
    kind: String,
    fields: Value,
    /// The passage it was declared in, for things declared in a place margin.
    home: Option<String>,
}

#[derive(Debug, Clone)]
struct Passage {
    tags: Vec<String>,
    header: Vec<Value>,
    body: Vec<Value>,
}

/// A compiled Inklink, indexed for running.
#[derive(Debug)]
pub struct Story {
    decls: HashMap<String, Decl>,
    passages: HashMap<String, Passage>,
    start: String,
    povs: Vec<String>,
    /// Where each pov starts, from `start: a for x, b for y`.
    start_for: HashMap<String, String>,
    /// Every way between two places: `(way, from, to)`. A two-way way is
    /// listed in both directions; a `route` adds one way per step.
    ways: Vec<(String, String, String)>,
    /// Every `on` handler, in source order, with the NPC or character whose
    /// margin it is in: that is who `self` means inside it.
    handlers: Vec<(Value, Option<String>)>,
    /// `default moves: walk, dash`, `default clash: random_winner`.
    defaults: HashMap<String, Value>,
    /// Every `checkpoint name ~ on death in …` declaration.
    checkpoints: Vec<Value>,
    /// The `rewind:` setting, if the Inklink has one.
    rewind: Option<Value>,
    /// Does any passage show `{ballots}`? Then the Inker frames the tally.
    shows_ballots: bool,
}

/// The field `key` of a JSON object, or `null`.
fn f<'a>(v: &'a Value, key: &str) -> &'a Value {
    v.get(key).unwrap_or(&Value::Null)
}

fn strs(v: &Value) -> Vec<String> {
    v.as_arr()
        .unwrap_or(&[])
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

impl Story {
    /// Index a compiled IR. Fails if the IR is not one `inkjot build` made.
    pub fn new(ir: &Value) -> R<Story> {
        let mut story = Story {
            decls: HashMap::new(),
            passages: HashMap::new(),
            start: f(ir, "start").as_str().unwrap_or("").to_string(),
            povs: Vec::new(),
            start_for: HashMap::new(),
            ways: Vec::new(),
            handlers: Vec::new(),
            defaults: HashMap::new(),
            checkpoints: Vec::new(),
            rewind: None,
            shows_ballots: false,
        };
        let config = ir
            .get("config")
            .and_then(Value::as_arr)
            .ok_or("this IR has no config")?;
        story.statements(config, None);
        for p in ir
            .get("passages")
            .and_then(Value::as_arr)
            .ok_or("this IR has no passages")?
        {
            let name = f(p, "name").as_str().unwrap_or("").to_string();
            let header = f(p, "header").as_arr().unwrap_or(&[]).to_vec();
            story.statements(&header, Some(&name));
            story.passages.insert(
                name,
                Passage {
                    tags: strs(f(p, "tags")),
                    header,
                    body: f(p, "body").as_arr().unwrap_or(&[]).to_vec(),
                },
            );
        }
        story.shows_ballots = ir.to_string().contains("\"insert\": \"ballots\"");
        Ok(story)
    }

    /// Record the declarations and the few settings the interpreter needs.
    fn statements(&mut self, stmts: &[Value], home: Option<&str>) {
        for s in stmts {
            if s.get("on").is_some() {
                self.handlers.push((s.clone(), home.map(str::to_string)));
            }
            // `checkpoint cells ~ on death in prison …` declares one;
            // `checkpoint threshold` in a passage reaches it.
            if s.get("checkpoint").is_some() && s.get("on_death_in").is_some() {
                self.checkpoints.push(s.clone());
            }
            if f(s, "declare").as_str() == Some("way") {
                let id = f(s, "id").as_str().unwrap_or("").to_string();
                let (from, to) = (
                    f(s, "from").as_str().unwrap_or(""),
                    f(s, "to").as_str().unwrap_or(""),
                );
                self.ways
                    .push((id.clone(), from.to_string(), to.to_string()));
                if f(s, "one_way") != &Value::Bool(true) {
                    self.ways.push((id, to.to_string(), from.to_string()));
                }
            }
            if f(s, "setting").as_str() == Some("route") {
                let stops = strs(f(s, "value"));
                for pair in stops.windows(2) {
                    self.ways
                        .push((String::new(), pair[0].clone(), pair[1].clone()));
                    self.ways
                        .push((String::new(), pair[1].clone(), pair[0].clone()));
                }
            }
            if let (Some(kind), Some(id)) = (f(s, "declare").as_str(), f(s, "id").as_str()) {
                let decl = Decl {
                    kind: kind.to_string(),
                    fields: s.clone(),
                    home: home.map(str::to_string),
                };
                self.decls.insert(id.to_string(), decl);
            }
            match f(s, "setting").as_str() {
                Some("pov") => self.povs = strs(f(s, "value")),
                Some("clock") => {
                    self.defaults.insert("clock".into(), f(s, "value").clone());
                    self.defaults
                        .insert("clock_shown_as".into(), f(s, "shown_as").clone());
                }
                Some("rewind") => self.rewind = Some(s.clone()),
                Some("default") => {
                    if let Some(of) = f(s, "of").as_str() {
                        self.defaults.insert(of.to_string(), f(s, "value").clone());
                    }
                }
                Some("start") => match f(s, "value") {
                    Value::Obj(pairs) => {
                        for (who, at) in pairs {
                            self.start_for
                                .insert(who.clone(), at.as_str().unwrap_or("").to_string());
                        }
                    }
                    Value::Str(at) => self.start = at.clone(),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn kind(&self, id: &str) -> Option<&str> {
        self.decls.get(id).map(|d| d.kind.as_str())
    }

    fn field(&self, id: &str, key: &str) -> &Value {
        self.decls
            .get(id)
            .map_or(&Value::Null, |d| f(&d.fields, key))
    }

    fn ids_of(&self, kind: &str) -> Vec<String> {
        let mut ids: Vec<String> = self
            .decls
            .iter()
            .filter(|(_, d)| d.kind == kind)
            .map(|(id, _)| id.clone())
            .collect();
        ids.sort();
        ids
    }

    fn has_tag(&self, id: &str, tag: &str) -> bool {
        strs(self.field(id, "tags")).iter().any(|t| t == tag)
    }
}

/// What an NPC saw at the end of one turn: the turn, and each person in
/// its place with what they held.
type Sighting = (u32, Vec<(String, Vec<String>)>);

/// A name bound in a passage.
#[derive(Debug, Clone)]
enum Local {
    /// A value fixed when it was bound: a `let`, a Link parameter, or
    /// anything `set` since.
    Value(V),
    /// A `get`: an expression, worked out again each time the name is read.
    /// So `get aligned: orrery.ember == orrery.tide` is true the moment the
    /// planets line up, even later in the same passage.
    Expr(Value),
}

/// One entry on a `list`.
#[derive(Debug, Clone)]
struct Entry {
    id: String,
    label: Option<String>,
    mark: Option<String>,
    struck: bool,
}

/// A story being played.
pub struct Run<'s> {
    story: &'s Story,
    /// Who the Linker is looking through. Empty before a lobby picks someone.
    pub pov: String,
    /// The passage each character is at.
    at: HashMap<String, String>,
    /// Where each NPC is.
    place: HashMap<String, String>,
    /// Every stored value, keyed by path: `grate_up`, `vell.cornered`, `roland.vults`.
    vals: HashMap<String, V>,
    knows: HashMap<String, BTreeSet<String>>,
    /// Who or where holds each item. An item with no entry is gone.
    holder: HashMap<String, String>,
    lists: HashMap<String, Vec<Entry>>,
    visits: HashMap<String, u32>,
    once: HashSet<String>,
    /// `let`s and Link parameters of the passage being run.
    locals: HashMap<String, Local>,
    /// The `let`s being worked out right now, to catch one that uses itself.
    evaluating: Vec<String>,
    /// The Links of the passage the pov is at, as last rendered.
    links: Vec<Value>,
    /// Text shown since the output was last taken.
    output: Vec<String>,
    chronon: u32,
    rng: u64,
    /// Per NPC: the order its `aka` names come out in, and how many are used.
    aka: HashMap<String, (Vec<usize>, usize)>,
    /// Failed `expect:` checks in the story's own passages.
    pub failures: Vec<String>,
    /// How many times each handler has fired, by its index in the story.
    handler_counts: Vec<u32>,
    /// How deeply handlers are nested right now.
    firing: usize,
    started: bool,
    /// While an NPC works out `toward`: the kind of way it keeps to (`via wide`).
    via: Option<String>,
    /// Several Linkers: the characters seated, in order. Empty for one.
    order: Vec<String>,
    /// The seated Linkers not being served right now.
    parked: HashMap<String, cohort::Seat>,
    /// Each Linker's move for this stop, once locked in.
    committed: HashMap<String, Value>,
    /// Shared acts waiting to be settled.
    claims: Vec<cohort::Claim>,
    /// While a claim is settled: who else reached for it (`{claimants}`)...
    claimants: String,
    /// ...and how it came out for this Linker: `(success, clashed)`.
    verdict: Option<(bool, bool)>,
    /// What each NPC saw at the end of each turn: who, holding what.
    sightings: HashMap<String, Vec<Sighting>>,
    /// The world at the start of every turn, oldest first.
    history: Vec<time::Snapshot>,
    /// Each checkpoint's snapshot: from the start, or from when it was reached.
    saved: HashMap<String, time::Snapshot>,
    /// The checkpoints reached so far, in order.
    reached: Vec<String>,
    /// Linkers re-living an earlier stop, and which.
    past: HashMap<String, u32>,
    /// Who dashed through each place, by the stop it was: `here.passed`.
    passed: HashMap<(u32, String), Vec<String>>,
    /// Characters who died during this Link, and where.
    dying: Vec<(String, String)>,
    /// While a rewind is announced: whose handlers do not see it.
    unseen_by: Vec<String>,
    /// The inputs open to the Linker being served: `{name input}`, `{accuse}`.
    asking: Vec<Value>,
    /// An arrival paused at a header input until it is answered.
    paused: Option<inputs::Paused>,
    /// The jury's ballots, by voter, until the jury decides.
    ballots: HashMap<String, inputs::Ballot>,
    /// The jury's last count: `(who, how, votes)`, most votes first.
    tally: Vec<(String, String, usize)>,
    /// Hints bought with Inquests: the player's, so a rewind keeps them.
    heard: inquests::Heard,
}

impl<'s> Run<'s> {
    /// A fresh run. The seed decides every roll and shuffle, so a run with
    /// the same seed and the same Links always goes the same way.
    pub fn new(story: &'s Story, seed: u64) -> Run<'s> {
        let mut run = Run {
            story,
            pov: story.povs.first().cloned().unwrap_or_default(),
            at: HashMap::new(),
            place: HashMap::new(),
            vals: HashMap::new(),
            knows: HashMap::new(),
            holder: HashMap::new(),
            lists: HashMap::new(),
            visits: HashMap::new(),
            once: HashSet::new(),
            locals: HashMap::new(),
            evaluating: Vec::new(),
            links: Vec::new(),
            output: Vec::new(),
            chronon: 0,
            rng: seed.max(1),
            aka: HashMap::new(),
            failures: Vec::new(),
            handler_counts: Vec::new(),
            firing: 0,
            started: false,
            via: None,
            order: Vec::new(),
            parked: HashMap::new(),
            committed: HashMap::new(),
            claims: Vec::new(),
            claimants: String::new(),
            verdict: None,
            sightings: HashMap::new(),
            history: Vec::new(),
            saved: HashMap::new(),
            reached: Vec::new(),
            past: HashMap::new(),
            passed: HashMap::new(),
            dying: Vec::new(),
            unseen_by: Vec::new(),
            asking: Vec::new(),
            paused: None,
            ballots: HashMap::new(),
            tally: Vec::new(),
            heard: Default::default(),
        };
        run.init();
        run.place_npcs();
        run.pick_scenario();
        run.start_the_log();
        run
    }

    /// Set every declared thing to its starting state.
    fn init(&mut self) {
        let story = self.story;
        let characters = story.ids_of("character");
        let mut ids: Vec<&String> = story.decls.keys().collect();
        ids.sort();
        for id in ids {
            let d = &story.decls[id];
            let field = |key: &str| f(&d.fields, key);
            match d.kind.as_str() {
                "character" | "npc" => {
                    for clue in strs(field("knows")) {
                        self.knows.entry(id.clone()).or_default().insert(clue);
                    }
                    if let Value::Obj(stats) = field("stats") {
                        for (stat, v) in stats {
                            self.vals.insert(format!("{id}.{stat}"), self.constant(v));
                        }
                    }
                    if let Some(place) = field("in").as_str() {
                        self.place.insert(id.clone(), place.to_string());
                    }
                }
                "item" => {
                    let home = field("in")
                        .as_str()
                        .or(field("held_by").as_str())
                        .or(d.home.as_deref());
                    if let Some(home) = home {
                        let home = home.to_string();
                        self.hold(id, &home);
                    }
                    // Facts come from the archetype first, then the item, then `with`.
                    let like = field("like").as_str().unwrap_or("");
                    for source in [story.field(like, "facts"), field("facts"), field("with")] {
                        if let Value::Obj(facts) = source {
                            for (name, v) in facts {
                                self.vals.insert(format!("{id}.{name}"), self.constant(v));
                            }
                        }
                    }
                }
                "fact" | "state" => {
                    let v = self.constant(field("value"));
                    // A fact declared in a passage belongs to its place, or to the
                    // character or NPC whose margin it is: `orrery.ember`, `vell.cornered`.
                    let owner = d.home.as_deref().map(place_of);
                    match field("belongs_to").as_str().or(owner) {
                        Some("character") => {
                            for c in &characters {
                                self.vals.insert(format!("{c}.{id}"), v.clone());
                            }
                        }
                        Some(owner) => {
                            self.vals.insert(format!("{owner}.{id}"), v);
                        }
                        None => {
                            self.vals.insert(id.clone(), v);
                        }
                    }
                }
                "resource" => {
                    let v = self.constant(field("value"));
                    // A resource is each character's own unless it is `shared`.
                    if !story.has_tag(id, "shared") {
                        for c in &characters {
                            self.vals.entry(format!("{c}.{id}")).or_insert(v.clone());
                        }
                    } else {
                        self.vals.insert(id.clone(), v);
                    }
                }
                "relation" => {
                    let lo = match self.constant(field("range")) {
                        V::Range { lo, .. } => V::Num(lo.unwrap_or(0.0)),
                        _ => V::Num(0.0),
                    };
                    self.vals.insert(id.clone(), lo);
                }
                "fuse" => {
                    self.vals
                        .insert(format!("{id}.left"), self.constant(field("start")));
                    let lit = !story.has_tag(id, "unlit");
                    self.vals.insert(format!("{id}.lit"), V::Bool(lit));
                }
                "list" => {
                    let entries = field("holds")
                        .as_arr()
                        .unwrap_or(&[])
                        .iter()
                        .map(entry)
                        .collect();
                    self.lists.insert(id.clone(), entries);
                }
                _ => {}
            }
        }
        for (who, at) in &story.start_for {
            self.at.insert(who.clone(), at.clone());
        }
    }

    /// A value from a declaration, which is a constant: `no`, `3`, `2..=4`.
    fn constant(&self, v: &Value) -> V {
        match v {
            Value::Null => V::Null,
            Value::Bool(b) => V::Bool(*b),
            Value::Num(n) => V::Num(n.parse().unwrap_or(0.0)),
            Value::Str(s) if s == "nothing" || s == "nobody" => V::Null,
            Value::Str(s) => V::Id(s.clone()),
            Value::Obj(_) if v.get("range").is_some() => {
                let bounds = f(v, "range").as_arr().unwrap_or(&[]);
                let bound = |i: usize| bounds.get(i).and_then(|b| self.constant(b).num());
                V::Range {
                    lo: bound(0),
                    hi: bound(1),
                    inclusive: f(v, "inclusive") == &Value::Bool(true),
                }
            }
            Value::Obj(_) if v.get("str").is_some() => {
                V::Str(f(v, "str").as_str().unwrap_or("").to_string())
            }
            // `methods device, poison, fall`.
            Value::Arr(items) => V::List(items.iter().map(|i| self.constant(i)).collect()),
            _ => V::Null,
        }
    }

    // ------------------------------------------------------------ playing

    /// Begin at `passage`, or at the story's start, looking through `who`.
    pub fn start(&mut self, passage: Option<&str>, who: Option<&str>) -> R<()> {
        if let Some(who) = who {
            self.pov = who.to_string();
        }
        let passage = match passage {
            Some(p) => p.to_string(),
            None => self.story.start.clone(),
        };
        if !self.started {
            self.started = true;
            let actor = self.pov.clone();
            self.fire(Event {
                on: "start",
                what: "",
                actor: &actor,
                before: false,
            })?;
        }
        self.enter(&passage, HashMap::new())?;
        // The log begins where play does.
        self.history = vec![self.snapshot()];
        Ok(())
    }

    /// The text shown since this was last called.
    pub fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.output)
    }

    /// The clock's last stop, if it has one: 5 for `clock: 0..=5`.
    fn last_stop(&self) -> Option<u32> {
        match self.constant(self.story.defaults.get("clock")?) {
            V::Range {
                hi: Some(hi),
                inclusive,
                ..
            } => Some(if inclusive { hi as u32 } else { hi as u32 - 1 }),
            _ => None,
        }
    }

    /// When the clock runs out, where everyone goes: the `[finale]` passage
    /// no Link leads to (Ex 1's `terminus`, not its `verdict`).
    fn finale(&self) -> Option<String> {
        let targets: std::collections::HashSet<&str> = self
            .story
            .passages
            .values()
            .flat_map(|p| p.body.iter().chain(p.header.iter()))
            .filter_map(|n| f(n, "target").as_str())
            .collect();
        let mut finales: Vec<&String> = self
            .story
            .passages
            .iter()
            .filter(|(name, p)| {
                p.tags.iter().any(|t| t == "finale") && !targets.contains(name.as_str())
            })
            .map(|(name, _)| name)
            .collect();
        finales.sort();
        finales.first().map(|f| f.to_string())
    }

    /// Is this the last stop, with a finale to go to when it ends?
    pub(super) fn finale_due(&self) -> bool {
        self.last_stop() == Some(self.when()) && self.finale().is_some()
    }

    /// The stop it is: `clock.now`.
    pub fn now(&self) -> u32 {
        self.chronon
    }

    /// Shared acts reached for and not yet settled.
    pub fn open_claims(&self) -> usize {
        self.claims.len()
    }

    /// Where the pov is.
    pub fn here(&self) -> &str {
        self.at.get(&self.pov).map_or("", String::as_str)
    }

    /// The Links the pov can take now, as `(id, label)`.
    pub fn choices(&mut self) -> R<Vec<(String, String)>> {
        let mut out = Vec::new();
        // A Linker who has locked in their move waits for the others; one
        // paused at an input answers it first.
        if self.waiting() || self.paused.is_some() {
            return Ok(out);
        }
        for link in self.links.clone() {
            if self.available(&link)? {
                let text = |key| f(&link, key).as_str().unwrap_or("").to_string();
                out.push((text("id"), text("label")));
            }
        }
        Ok(out)
    }

    /// Take the available Link with this id.
    pub fn choose(&mut self, id: &str) -> R<()> {
        let link = self
            .links
            .iter()
            .find(|l| f(l, "id").as_str() == Some(id))
            .cloned()
            .ok_or_else(|| format!("there is no Link `{id}` here"))?;
        if !self.available(&link)? {
            return Err(format!("the Link `{id}` is not available"));
        }
        self.take(&link)?;
        self.settle_deaths()
    }

    fn available(&mut self, link: &Value) -> R<bool> {
        // An Inquest is offered only when it would tell something.
        if f(link, "kind").as_str() == Some("inquest") && !self.may_inquire()? {
            return Ok(false);
        }
        // An echo may look and learn, but take nothing and reach for nothing.
        if self.in_past() && (self.is_claim(link) || f(link, "act").as_str() == Some("take")) {
            return Ok(false);
        }
        // Already reached for, and waiting to be settled.
        if self.claimed(link) {
            return Ok(false);
        }
        // Each character is played by one Linker.
        if let Some(who) = f(link, "become").as_str() {
            if who != self.pov && self.order.iter().any(|o| o == who) {
                return Ok(false);
            }
        }
        if let Some(audience) = link.get("for").and_then(Value::as_arr) {
            let pov = self.pov.clone();
            let fits = |w: &Value| match w.as_str() {
                Some("pov") => true,
                Some(w) => {
                    w == pov || self.story.has_tag(&pov, w) || self.story.kind(&pov) == Some(w)
                }
                None => false,
            };
            if !audience.iter().any(fits) {
                return Ok(false);
            }
        }
        for key in ["when", "hidden_until"] {
            let cond = f(link, key);
            if *cond != Value::Null && !self.eval(cond)?.truthy() {
                return Ok(false);
            }
        }
        // `take x` is offered only while x is within reach.
        if f(link, "act").as_str() == Some("take") {
            if let Some(item) = f(link, "arg").as_str() {
                if !self.in_reach(item) {
                    return Ok(false);
                }
            }
        }
        let actor = f(link, "order")
            .as_str()
            .map_or(self.pov.clone(), str::to_string);
        let have = self.vults(&actor);
        // `observe: cost rest ~ min 1`; `dash: requires full vults`.
        if let (Some(min), Some(have)) = (f(link, "min").as_u64(), have) {
            if have < min as f64 {
                return Ok(false);
            }
        }
        if f(link, "requires_full") == &Value::Bool(true) {
            let full = self.constant(self.story.field("vults", "value")).num();
            if have.is_some_and(|h| full.is_some_and(|full| h < full)) {
                return Ok(false);
            }
        }
        match self.cost(link)? {
            Some(cost) => Ok(self.vults(&actor).is_none_or(|have| have >= cost)),
            None => Ok(true),
        }
    }

    /// What a Link costs in vults: the last cost entry whose condition holds.
    /// `rest` costs whatever is left, which is always affordable.
    fn cost(&mut self, link: &Value) -> R<Option<f64>> {
        let mut cost = None;
        for entry in f(link, "cost").as_arr().unwrap_or(&[]) {
            let when = f(entry, "when");
            if *when == Value::Null || self.eval(when)?.truthy() {
                cost = Some(entry);
            }
        }
        Ok(cost.and_then(|c| match f(c, "vults") {
            Value::Num(n) => n.parse().ok(),
            _ => Some(0.0),
        }))
    }

    fn vults(&self, who: &str) -> Option<f64> {
        self.vals.get(&format!("{who}.vults")).and_then(V::num)
    }

    /// Is an item here: lying in this place, or held by an NPC who is?
    fn in_reach(&self, item: &str) -> bool {
        let here = self.here();
        let place = place_of(here);
        match self.holder.get(item) {
            Some(h) if h == here || h == place => true,
            Some(h) => self.story.kind(h) == Some("npc") && self.location(h) == place,
            None => false,
        }
    }

    /// Take a Link: a move or an ends-turn act in a world of stops is
    /// committed, a shared act with limited room is claimed, and anything
    /// else happens at once.
    fn take(&mut self, link: &Value) -> R<()> {
        if f(link, "kind").as_str() == Some("rewind") {
            return self.rewind_to(link);
        }
        // An echo moves through the past, one stop at a time.
        if self.in_past() && self.ends_turn(link) {
            return self.echo_moves(link);
        }
        if self.story.field("walk", "ends_turn") == &Value::Bool(true) && self.ends_turn(link) {
            if matches!(f(link, "kind").as_str(), Some("walk" | "stay")) {
                return self.commit(link, false);
            }
            // An act that ends the turn (`wind`) happens now; then the
            // Linker stays where they are.
            self.perform(link, false)?;
            let stay = Value::obj([("kind", Value::str("stay"))]);
            return self.commit(&stay, true);
        }
        if self.is_claim(link) {
            return self.claim(link);
        }
        self.perform(link, false)
    }

    /// Pay what a Link costs, from whoever acts.
    fn pay(&mut self, link: &Value) -> R<()> {
        if self.in_past() {
            return Ok(()); // the past is already paid for
        }
        let actor = f(link, "order")
            .as_str()
            .map_or(self.pov.clone(), str::to_string);
        if let Some(cost) = self.cost(link)? {
            if f(link, "cost")
                .as_arr()
                .and_then(|c| c.last())
                .is_some_and(|c| f(c, "vults").as_str() == Some("rest"))
            {
                self.vals.insert(format!("{actor}.vults"), V::Num(0.0));
            } else if let Some(have) = self.vults(&actor) {
                self.vals
                    .insert(format!("{actor}.vults"), V::Num(have - cost));
            }
        }
        Ok(())
    }

    /// Do what a Link does, now. `paid`: its cost was taken when it was claimed.
    fn perform(&mut self, link: &Value, paid: bool) -> R<()> {
        if !paid {
            self.pay(link)?;
        }
        let pov = self.pov.clone();
        // An order: the other character acts, with the pov's ideas.
        if let Some(actor) = f(link, "order").as_str() {
            self.pov = actor.to_string();
        }
        let actor = self.pov.clone();
        // A settled claim brings its verdict; it still has to succeed.
        let (success, clashed) = match self.verdict.take() {
            Some((won, clashed)) => (won && self.attempt(link)?, clashed),
            None => (self.attempt(link)?, false),
        };
        self.vals.insert("attempt.success".into(), V::Bool(success));
        self.vals.insert("attempt.clashed".into(), V::Bool(clashed));
        if success && f(link, "act").as_str() == Some("take") {
            if let Some(item) = f(link, "arg").as_str() {
                if self.fire(Event {
                    on: "take",
                    what: item,
                    actor: &actor,
                    before: true,
                })? {
                    self.pov = pov;
                    return self.relist();
                }
                self.hold(item, &actor);
                self.fire(Event {
                    on: "take",
                    what: item,
                    actor: &actor,
                    before: false,
                })?;
            }
        }
        // A loud act is heard: `force`, `dash`, or a Link marked `~ loud`.
        let act = f(link, "act").as_str().unwrap_or("");
        if self.story.has_tag(act, "loud") || f(link, "loud") == &Value::Bool(true) {
            self.fire(Event {
                on: "loud",
                what: act,
                actor: &actor,
                before: false,
            })?;
        }
        for effect in f(link, "effects").as_arr().unwrap_or(&[]).to_vec() {
            self.apply(&effect, &[])?;
        }
        self.pov = pov;
        let target = f(link, "target").as_str().unwrap_or("here").to_string();
        let params = match f(link, "with") {
            Value::Obj(pairs) => pairs
                .iter()
                .map(|(k, v)| (k.clone(), self.constant(v)))
                .collect(),
            _ => HashMap::new(),
        };
        match f(link, "kind").as_str() {
            Some("walk") => {
                let to = self
                    .arrival(&target)
                    .ok_or_else(|| format!("there is nowhere to arrive at `{target}`"))?;
                // Crossing a way: `before` handlers may stop it.
                let way = f(link, "way").as_str().unwrap_or("").to_string();
                if !way.is_empty() {
                    let actor = self.pov.clone();
                    if self.fire(Event {
                        on: "cross",
                        what: &way,
                        actor: &actor,
                        before: true,
                    })? {
                        return self.relist();
                    }
                    self.fire(Event {
                        on: "cross",
                        what: &way,
                        actor: &actor,
                        before: false,
                    })?;
                }
                if self.story.field("walk", "ends_turn") == &Value::Bool(true) {
                    self.end_turn()?;
                }
                return self.enter(&to, HashMap::new());
            }
            Some("switch") => {
                let other = self.story.povs.iter().find(|p| **p != self.pov).cloned();
                self.pov = other.ok_or("`switch` needs a second pov")?;
                let at = self.here().to_string();
                return self.enter(&at, HashMap::new());
            }
            Some("pass") => self.end_turn()?,
            Some("inquest") => self.inquire()?,
            _ => {}
        }
        if let Some(who) = f(link, "become").as_str() {
            // The seat now plays `who`.
            if let Some(seat) = self.order.iter_mut().find(|o| **o == self.pov) {
                *seat = who.to_string();
            }
            self.pov = who.to_string();
        }
        match target.as_str() {
            "here" => self.relist(),
            "rewind" => self.begin_rewind(),
            _ => self.enter(&target, params),
        }
    }

    /// Does the act succeed? A `check` must hold; a `chance` is rolled.
    fn attempt(&mut self, link: &Value) -> R<bool> {
        let act = f(link, "act").as_str().unwrap_or("");
        let check = match f(link, "check") {
            Value::Null => self.story.field(act, "check").clone(),
            c => c.clone(),
        };
        if check != Value::Null && !self.eval(&check)?.truthy() {
            return Ok(false);
        }
        // `chance might in 6`: roll a d6, succeed on the stat or under.
        if let Some(chance) = link.get("chance").and_then(Value::as_arr) {
            let stat = chance.first().and_then(Value::as_str).unwrap_or("");
            let sides = chance.get(2).and_then(Value::as_u64).unwrap_or(6);
            let need = self.name(stat)?.num().unwrap_or(0.0);
            return Ok(self.roll(sides) as f64 <= need);
        }
        Ok(true)
    }

    fn end_turn(&mut self) -> R<()> {
        let actor = self.pov.clone();
        self.fire(Event {
            on: "turn",
            what: "",
            actor: &actor,
            before: false,
        })?;
        // The clock stops at its last stop: `clock: 0..=5`.
        if self.last_stop().is_none_or(|last| self.chronon < last) {
            self.chronon += 1;
        }
        let story = self.story;
        for id in story.ids_of("resource") {
            if story.field(&id, "refill").as_str() == Some("chronon") {
                let full = self.constant(story.field(&id, "value"));
                for c in story.ids_of("character") {
                    self.vals.insert(format!("{c}.{id}"), full.clone());
                }
            }
        }
        // A fuse that was lit with `light` burns down by one each turn.
        for id in story.ids_of("fuse") {
            if self
                .vals
                .get(&format!("{id}.ticking"))
                .is_some_and(V::truthy)
            {
                self.burn(&id, 1.0)?;
            }
        }
        // Then everyone you are not playing takes their move, and the NPCs
        // note who they saw.
        self.agendas()?;
        self.look();
        self.history.push(self.snapshot());
        Ok(())
    }

    /// Arrive at a passage: run its header, show its body, offer its Links.
    fn enter(&mut self, passage: &str, params: HashMap<String, V>) -> R<()> {
        let story = self.story;
        let p = story
            .passages
            .get(passage)
            .ok_or_else(|| format!("there is no passage `{passage}`"))?;
        let from = self.here().to_string();
        self.at.insert(self.pov.clone(), passage.to_string());
        *self.visits.entry(passage.to_string()).or_default() += 1;
        let actor = self.pov.clone();
        if place_of(&from) != place_of(passage) {
            if !from.is_empty() {
                self.fire(Event {
                    on: "leave",
                    what: place_of(&from),
                    actor: &actor,
                    before: false,
                })?;
            }
            self.fire(Event {
                on: "enter",
                what: place_of(passage),
                actor: &actor,
                before: false,
            })?;
        }
        if p.tags.iter().any(|t| t == "act") {
            self.fire(Event {
                on: "act",
                what: passage,
                actor: &actor,
                before: false,
            })?;
        }
        // A cell, `dining@2`, is a place at a time: being there means it is stop 2.
        let cell_time = passage.rsplit_once('@').and_then(|(_, n)| n.parse().ok());
        if let Some(n) = cell_time.filter(|_| !self.in_past()) {
            self.chronon = n;
        }
        self.locals = params
            .into_iter()
            .map(|(k, v)| (k, Local::Value(v)))
            .collect();
        self.parent_lets(passage)?;
        self.arrive(passage, &from, 0, None)
    }

    /// Run a passage's header from statement `start`, then show it and
    /// offer its Links. A header input pauses the arrival there until it is
    /// answered (see `inputs.rs`).
    fn arrive(
        &mut self,
        passage: &str,
        from: &str,
        start: usize,
        mut next: Option<String>,
    ) -> R<()> {
        let story = self.story;
        let p = &story.passages[passage];
        let from = from.to_string();
        for (i, stmt) in p.header.iter().enumerate().skip(start) {
            if inputs::is_input(stmt) {
                self.asking.clear();
                let prompt = self.meet_input(stmt)?;
                self.output.push(prompt);
                self.links.clear();
                self.paused = Some(inputs::Paused {
                    passage: passage.to_string(),
                    from,
                    next_statement: i + 1,
                    next,
                });
                return Ok(());
            }
            if let Some(n) = self.statement(stmt)? {
                next = Some(n);
            }
        }
        self.asking.clear();
        let links = self.render(passage, false)?;
        if let Some(next) = next {
            return self.enter(&next, HashMap::new());
        }
        let now = self.here().to_string();
        if p.tags.iter().any(|t| t == "act") {
            if now == passage {
                // An act shows its result, and the pov is where they were.
                self.at.insert(self.pov.clone(), from.clone());
                self.relist()?;
            } else {
                // The act moved them: show where they are now.
                return self.enter(&now, HashMap::new());
            }
        } else {
            self.links = links;
            let more = self.standing_links()?;
            self.links.extend(more);
        }
        Ok(())
    }

    /// The passage a walk to `place` arrives at: the place's cell for the
    /// chronon it will be by then, or the place's own passage.
    fn arrival(&self, place: &str) -> Option<String> {
        let ends_turn = self.story.field("walk", "ends_turn") == &Value::Bool(true);
        self.arrival_at(place, self.when() + u32::from(ends_turn))
    }

    /// The passage for being at `place` at stop `when`: its cell, or the
    /// place's own passage.
    fn arrival_at(&self, place: &str, when: u32) -> Option<String> {
        let cell = format!("{place}@{when}");
        if self.story.passages.contains_key(&cell) {
            Some(cell)
        } else if self.story.field("walk", "ends_turn") == &Value::Bool(true) {
            // In a world of stops, you arrive at a time: a cell or nothing.
            None
        } else if self.story.passages.contains_key(place) {
            Some(place.to_string())
        } else {
            None
        }
    }

    /// Work out the Links on offer where the pov is, without showing anything:
    /// re-run the passage's `let`s, then render it quietly.
    fn relist(&mut self) -> R<()> {
        let at = self.here().to_string();
        self.asking.clear();
        self.locals.clear();
        self.parent_lets(&at)?;
        self.lets_of(&at)?;
        self.links = self.render(&at, true)?;
        let more = self.standing_links()?;
        self.links.extend(more);
        Ok(())
    }

    /// A sub-Nib sees the `let`s of the passages it belongs to:
    /// `orrery:crank` sees those of `orrery`. Outermost first.
    fn parent_lets(&mut self, passage: &str) -> R<()> {
        let mut parents = Vec::new();
        let mut name = passage;
        while let Some((parent, _)) = name.rsplit_once(':') {
            parents.push(parent.to_string());
            name = parent;
        }
        for parent in parents.iter().rev() {
            self.lets_of(parent)?;
        }
        Ok(())
    }

    /// Run only the `let`s of a passage's header.
    fn lets_of(&mut self, passage: &str) -> R<()> {
        let header = self
            .story
            .passages
            .get(passage)
            .map(|p| p.header.clone())
            .unwrap_or_default();
        for stmt in header.iter().filter(|s| s.get("let").is_some()) {
            self.statement(stmt)?;
        }
        Ok(())
    }

    /// Links offered everywhere: the Links of `[lib everywhere]` passages,
    /// and a walk along each open way out of the pov's place.
    fn standing_links(&mut self) -> R<Vec<Value>> {
        let story = self.story;
        let mut links = Vec::new();
        // In a lobby, before play begins, nothing stands everywhere.
        let in_lobby = story
            .passages
            .get(self.here())
            .is_some_and(|p| p.tags.iter().any(|t| t == "lobby"));
        if self.pov.is_empty() || in_lobby {
            return Ok(links);
        }
        let mut libs: Vec<&String> = story
            .passages
            .iter()
            .filter(|(_, p)| p.tags.iter().any(|t| t == "everywhere"))
            .map(|(n, _)| n)
            .collect();
        libs.sort();
        for lib in libs {
            links.extend(self.render(lib, true)?);
        }
        // In a world of stops, moving ends the turn: walk, dash, slip, stay.
        if story.field("walk", "ends_turn") == &Value::Bool(true) {
            links.extend(self.move_links()?);
            return Ok(links);
        }
        let here = place_of(self.here()).to_string();
        let walk_cost = story.field("walk", "cost").clone();
        for (way, from, to) in story.ways.clone() {
            if from != here {
                continue;
            }
            // A way `for npc` is closed to Linkers.
            if strs(story.field(&way, "for")).iter().any(|w| w == "npc") {
                continue;
            }
            if !way.is_empty() {
                if let Value::Obj(_) = story.field(&way, "open_when") {
                    let cond = story.field(&way, "open_when").clone();
                    if !self.eval(&cond)?.truthy() {
                        continue;
                    }
                } else if let Some(name) = story.field(&way, "open_when").as_str() {
                    if !self.name(name)?.truthy() {
                        continue;
                    }
                }
            }
            // No passage to arrive at (the last stop has passed): no walk.
            if self.arrival(&to).is_none() {
                continue;
            }
            let label = format!("Go to {}", self.label(&to));
            links.push(Value::obj([
                ("type", Value::str("link")),
                ("id", Value::str(format!("walk:{to}"))),
                ("way", Value::str(way.clone())),
                ("label", Value::str(label)),
                ("target", Value::str(to)),
                ("kind", Value::str("walk")),
                ("act", Value::str("walk")),
                ("cost", walk_cost.clone()),
            ]));
        }
        Ok(links)
    }

    /// Run one header statement. Returns the passage named by `next:`.
    fn statement(&mut self, s: &Value) -> R<Option<String>> {
        if let Some(name) = f(s, "let").as_str() {
            let when = f(s, "when");
            if *when == Value::Null || self.eval(when)?.truthy() {
                let value = f(s, "value");
                // `get` keeps the expression; `let` works it out now, once.
                let local = if f(s, "derived") == &Value::Bool(true) {
                    Local::Expr(value.clone())
                } else {
                    Local::Value(self.eval(value)?)
                };
                self.locals.insert(name.to_string(), local);
            }
        } else if let Some(effect) = s.get("effect") {
            let clauses = f(s, "clauses").as_arr().unwrap_or(&[]).to_vec();
            self.apply(effect, &clauses)?;
        } else if let (Some(name), None) = (f(s, "checkpoint").as_str(), s.get("on_death_in")) {
            self.reach_checkpoint(name);
        } else if f(s, "setting").as_str() == Some("next") {
            return Ok(f(s, "value").as_str().map(str::to_string));
        } else if let Some(check) = s.get("expect") {
            if !self.eval(check)?.truthy() {
                let why = f(s, "else").as_str().unwrap_or("an expect: check failed");
                self.failures.push(format!("{}: {why}", self.here()));
            }
        }
        Ok(None)
    }

    // ------------------------------------------------------------ rendering

    /// Show a passage's body and return its Links. With `quiet`, nothing is
    /// shown and nothing is used up: this only works out which Links are on
    /// offer now.
    fn render(&mut self, passage: &str, quiet: bool) -> R<Vec<Value>> {
        let story = self.story;
        let p = story
            .passages
            .get(passage)
            .ok_or_else(|| format!("there is no passage `{passage}`"))?;
        let first_visit = self.visits.get(passage).copied().unwrap_or(0) <= 1;
        let mut links = Vec::new();
        let mut show = true;
        let mut last_if = true;
        let mut append = false;
        for node in &p.body {
            match f(node, "type").as_str() {
                Some("modifier") => {
                    append = false;
                    for part in f(node, "parts").as_arr().unwrap_or(&[]) {
                        let when = f(part, "when");
                        match f(part, "name").as_str().unwrap_or("") {
                            "if" | "ifalways" => {
                                last_if = self.eval(when)?.truthy();
                                show = last_if;
                            }
                            "unless" | "ifnever" => {
                                last_if = !self.eval(when)?.truthy();
                                show = last_if;
                            }
                            "else" => show = !last_if,
                            "end" | "continue" | "cont" => {
                                show = true;
                                last_if = true;
                            }
                            "once" => show = show && first_visit,
                            "append" => append = true,
                            _ => {} // after, beat, verse, reveal: presentation only
                        }
                    }
                }
                Some("link") if show => links.push(node.clone()),
                Some("prose") | Some("speech") if show => {
                    let lead = match f(node, "who").as_str() {
                        Some(who) => {
                            let name = self.label(who);
                            match f(node, "direction").as_str() {
                                Some(d) => format!("{name} ({d}): "),
                                None => format!("{name}: "),
                            }
                        }
                        None => String::new(),
                    };
                    for (i, para) in f(node, "paragraphs")
                        .as_arr()
                        .unwrap_or(&[])
                        .iter()
                        .enumerate()
                    {
                        let (text, more) = self.paragraph(para, quiet)?;
                        links.extend(more);
                        let text = if i == 0 {
                            format!("{lead}{text}")
                        } else {
                            text
                        };
                        if quiet || text.trim().is_empty() {
                            continue;
                        }
                        match self.output.last_mut() {
                            Some(last) if append && i == 0 => {
                                last.push(' ');
                                last.push_str(&text);
                            }
                            _ => self.output.push(text),
                        }
                    }
                    append = false;
                }
                _ => {}
            }
        }
        Ok(links)
    }

    /// One paragraph as text, and the Links of any passage it embeds.
    fn paragraph(&mut self, para: &Value, quiet: bool) -> R<(String, Vec<Value>)> {
        let mut text = String::new();
        let mut links = Vec::new();
        for part in para.as_arr().unwrap_or(&[]) {
            if let Some(s) = part.as_str() {
                text.push_str(s);
                continue;
            }
            if let Some(v) = part.get("value") {
                let v = if quiet { V::Null } else { self.eval(v)? };
                text.push_str(&self.show(&v));
                continue;
            }
            match f(part, "insert").as_str().unwrap_or("") {
                "embed" => {
                    let nib = f(part, "nib").as_str().unwrap_or("").to_string();
                    // An embedded passage brings its own `let`s with it.
                    self.lets_of(&nib)?;
                    links.extend(self.render(&nib, quiet)?);
                }
                "present" => {
                    let here = place_of(self.here()).to_string();
                    let who: Vec<String> = self
                        .story
                        .ids_of("npc")
                        .into_iter()
                        .filter(|n| self.place.get(n).is_some_and(|p| *p == here))
                        .map(|n| self.label(&n))
                        .collect();
                    // The Linkers here, then the NPCs.
                    let mut linkers: Vec<String> = self
                        .linkers_here(&here)
                        .iter()
                        .map(|l| self.label(l))
                        .collect();
                    linkers.extend(who);
                    text.push_str(&linkers.join(", "));
                }
                "passers" => {
                    let key = (self.when(), place_of(self.here()).to_string());
                    let names: Vec<String> = self
                        .passed
                        .get(&key)
                        .cloned()
                        .unwrap_or_default()
                        .iter()
                        .map(|w| self.label(w))
                        .collect();
                    text.push_str(&names.join(" and "));
                }
                kind if kind == "accuse" || kind.ends_with(" input") => {
                    let prompt = self.meet_input(part)?;
                    if !quiet {
                        text.push_str(&prompt);
                    }
                }
                "ballots" => text.push_str(&self.ballots_text()),
                "claimants" => {
                    let names = if self.claimants.is_empty() {
                        String::from("Someone")
                    } else {
                        self.claimants.clone()
                    };
                    text.push_str(&names);
                }
                "list" => {
                    let name = f(part, "list").as_str().unwrap_or("");
                    let lines: Vec<String> = self
                        .lists
                        .get(name)
                        .map(|entries| {
                            entries
                                .iter()
                                .map(|e| {
                                    let mut label =
                                        e.label.clone().unwrap_or_else(|| self.label(&e.id));
                                    if let Some(mark) = &e.mark {
                                        label = format!("{label} ({mark})");
                                    }
                                    if e.struck {
                                        format!("~~{label}~~")
                                    } else {
                                        label
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    if !quiet {
                        self.output.extend(lines);
                    }
                }
                // Sounds, images, inputs and the like have no text of their own.
                _ => {}
            }
        }
        Ok((text, links))
    }

    /// How a value reads in the story: an id reads as its label.
    fn show(&self, v: &V) -> String {
        match v {
            V::Id(id) => self.label(id),
            other => other.to_string(),
        }
    }

    /// A thing's name as players see it.
    fn label(&self, id: &str) -> String {
        if let Some(name) = self.vals.get(&format!("{id}.name")) {
            return name.to_string();
        }
        match self.story.field(id, "label").as_str() {
            Some(label) => label.to_string(),
            None => id.to_string(),
        }
    }

    // ------------------------------------------------------------ effects

    /// Apply one effect. `clauses` are the `~` lines of a block form such as
    /// `add to charges:`.
    fn apply(&mut self, e: &Value, clauses: &[Value]) -> R<()> {
        if e.get("stretch").is_some() {
            return Ok(()); // a v0.6 effect, not run yet
        }
        if self.in_past() && !self.echo_may(e) {
            return Ok(()); // an echo changes nothing
        }
        let when = f(e, "when");
        if *when != Value::Null && !self.eval(when)?.truthy() {
            return Ok(());
        }
        if f(e, "once") == &Value::Bool(true) && !self.once.insert(e.to_string()) {
            return Ok(());
        }
        let args = f(e, "args").as_arr().unwrap_or(&[]).to_vec();
        let word = |i: usize| {
            args.get(i)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        let pov = self.pov.clone();
        match f(e, "do").as_str().unwrap_or("") {
            "learn" => {
                let clue = word(0);
                if self
                    .knows
                    .entry(pov.clone())
                    .or_default()
                    .insert(clue.clone())
                {
                    self.fire(Event {
                        on: "learn",
                        what: &clue,
                        actor: &pov,
                        before: false,
                    })?;
                }
            }
            "teach" => {
                let who = self.who(&args[0])?;
                self.knows.entry(who).or_default().insert(word(1));
            }
            "gain" => self.hold(&word(0), &pov),
            "give" => {
                let who = self.who(args.last().unwrap_or(&Value::Null))?;
                self.hold(&word(0), &who);
            }
            "lose" => {
                let key = format!("{pov}.{}", word(0));
                let left = match args.get(1) {
                    Some(Value::Str(s)) if s == "rest" => 0.0,
                    Some(n) => {
                        self.vals.get(&key).and_then(V::num).unwrap_or(0.0)
                            - self.eval(n)?.num().unwrap_or(0.0)
                    }
                    None => 0.0,
                };
                self.vals.insert(key, V::Num(left.max(0.0)));
            }
            "set" => {
                let v = self.eval(f(e, "value"))?;
                // Setting a local pins it: a `get` set to 2 is 2 from then on,
                // and every `get` that reads it sees 2.
                if let Some(name) = f(e, "target")
                    .as_str()
                    .filter(|n| self.locals.contains_key(*n))
                {
                    self.locals.insert(name.to_string(), Local::Value(v));
                    return Ok(());
                }
                let key = self.path(f(e, "target"))?;
                self.vals.insert(key.clone(), v);
                self.fire(Event {
                    on: "set",
                    what: &key,
                    actor: &pov,
                    before: false,
                })?;
            }
            "raise" | "lower" | "toggle" => {
                self.change(f(e, "do").as_str().unwrap_or(""), &args)?
            }
            "add" => self.add(&args, clauses)?,
            "remove" | "strike" => {
                let (item, list) = (word(0), word(2));
                let entries = self
                    .lists
                    .get_mut(&list)
                    .ok_or_else(|| format!("there is no list `{list}`"))?;
                if f(e, "do").as_str() == Some("remove") {
                    entries.retain(|en| en.id != item);
                } else {
                    for en in entries.iter_mut().filter(|en| en.id == item) {
                        en.struck = true;
                    }
                }
            }
            "move" => {
                let who = self.who(&args[0])?;
                let to = word(args.len() - 1);
                if self.story.kind(&who) == Some("character") {
                    self.at.insert(who, to);
                } else {
                    self.place.insert(who, to);
                }
            }
            "light" => {
                self.vals.insert(format!("{}.lit", word(0)), V::Bool(true));
                self.vals
                    .insert(format!("{}.ticking", word(0)), V::Bool(true));
            }
            "burn" => {
                let n = args
                    .get(1)
                    .map_or(Ok(V::Num(1.0)), |a| self.eval(a))?
                    .num()
                    .unwrap_or(1.0);
                self.burn(&word(0), n)?;
            }
            "destroy" => {
                self.holder.remove(&word(0));
                self.vals
                    .insert(format!("{}.destroyed", word(0)), V::Bool(true));
            }
            "kill" | "detain" | "wake" | "wound" => {
                // `wound 1` wounds the pov; `wound actor 1` someone else.
                let who = match args.first() {
                    Some(a) if !matches!(a, Value::Num(_)) => self.who(a)?,
                    _ => pov,
                };
                let fact = match f(e, "do").as_str() {
                    Some("kill") => "dead",
                    Some("detain") => "detained",
                    Some("wake") => "woken",
                    _ => "wounded",
                };
                let already = self
                    .vals
                    .get(&format!("{who}.{fact}"))
                    .is_some_and(V::truthy);
                self.vals.insert(format!("{who}.{fact}"), V::Bool(true));
                if fact == "dead" && !already {
                    if self.story.kind(&who) == Some("character") {
                        let place = self.location(&who);
                        self.dying.push((who.clone(), place));
                    }
                    self.fire(Event {
                        on: "death",
                        what: "",
                        actor: &who,
                        before: false,
                    })?;
                }
            }
            "play" => {
                self.render(&word(0), false)?;
            }
            // `roll standoff`: roll on a table.
            "roll" => self.roll_table(&word(0))?,
            // `fix actor`: it stays where it is from now on.
            "fix" => {
                let who = match args.first() {
                    Some(a) => self.who(a)?,
                    None => pov,
                };
                self.vals.insert(format!("{who}.fixed"), V::Bool(true));
            }
            // `feign sleep`: an NPC pretends; `x.asleep` believes it.
            "feign" => {
                let who = match self.locals.get("self") {
                    Some(Local::Value(V::Id(id))) => id.clone(),
                    _ => pov,
                };
                self.vals.insert(format!("{who}.feigning"), V::Bool(true));
            }
            "say" => {
                let who = f(e, "who").as_str().unwrap_or("");
                // `say self "…"`: self is the one whose margin the handler is in.
                let who = match self.name(who)? {
                    V::Id(id) => self.label(&id),
                    _ => self.label(who),
                };
                self.output
                    .push(format!("{who}: {}", f(e, "text").as_str().unwrap_or("")));
            }
            "stop" | "nothing" => {}
            other => {
                return Err(format!(
                    "the effect `{other}` is not supported by the interpreter yet"
                ))
            }
        }
        Ok(())
    }

    /// `raise trust 1`, `lower x 2`, `raise orrery.ember 1 mod 6`, `toggle x`.
    fn change(&mut self, verb: &str, args: &[Value]) -> R<()> {
        let key = self.path(args.first().unwrap_or(&Value::Null))?;
        let now = self.vals.get(&key).cloned().unwrap_or(V::Null);
        let v = if verb == "toggle" {
            V::Bool(!now.truthy())
        } else {
            let by = args
                .get(1)
                .map_or(Ok(V::Num(1.0)), |a| self.eval(a))?
                .num()
                .unwrap_or(1.0);
            let mut n = now.num().unwrap_or(0.0) + if verb == "raise" { by } else { -by };
            if args.get(2).and_then(Value::as_str) == Some("mod") {
                let m = args
                    .get(3)
                    .map_or(Ok(V::Null), |a| self.eval(a))?
                    .num()
                    .unwrap_or(1.0);
                n = n.rem_euclid(m);
            }
            // A relation stays inside its declared range; a resource under its max.
            let decl = key.rsplit('.').next().unwrap_or(&key);
            if let V::Range { lo, hi, .. } = self.constant(self.story.field(decl, "range")) {
                n = n.max(lo.unwrap_or(f64::MIN)).min(hi.unwrap_or(f64::MAX));
            }
            if let Some(max) = self.constant(self.story.field(decl, "max")).num() {
                n = n.min(max);
            }
            V::Num(n)
        };
        let changed = self.vals.get(&key) != Some(&v);
        self.vals.insert(key.clone(), v);
        if changed && verb != "toggle" {
            let actor = self.pov.clone();
            self.fire(Event {
                on: verb,
                what: &key,
                actor: &actor,
                before: false,
            })?;
        }
        Ok(())
    }

    /// `add naming: "…" to charges`, `add skittles to the_list as pending`,
    /// or the block form `add to charges:` with one entry per clause.
    fn add(&mut self, args: &[Value], clauses: &[Value]) -> R<()> {
        let to = args
            .iter()
            .position(|a| a.as_str() == Some("to"))
            .ok_or("`add` needs `to list`")?;
        let list = args
            .get(to + 1)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let mark = match args.iter().position(|a| a.as_str() == Some("as")) {
            Some(i) => args.get(i + 1).and_then(Value::as_str).map(str::to_string),
            None => None,
        };
        let mut entries = Vec::new();
        if to > 0 {
            entries.push(entry_from_words(&args[..to], mark.clone()));
        }
        for clause in clauses {
            entries.push(entry_from_words(
                f(clause, "words").as_arr().unwrap_or(&[]),
                None,
            ));
        }
        self.lists.entry(list).or_default().extend(entries);
        Ok(())
    }

    fn burn(&mut self, fuse: &str, by: f64) -> R<()> {
        let key = format!("{fuse}.left");
        let before = self.vals.get(&key).and_then(V::num).unwrap_or(0.0);
        let left = before - by;
        self.vals.insert(key, V::Num(left));
        if before > 0.0 && left <= 0.0 {
            let actor = self.pov.clone();
            self.fire(Event {
                on: "out",
                what: fuse,
                actor: &actor,
                before: false,
            })?;
            for effect in self
                .story
                .field(fuse, "then")
                .as_arr()
                .unwrap_or(&[])
                .to_vec()
            {
                self.apply(&effect, &[])?;
            }
        }
        Ok(())
    }

    /// `who` now holds `item`, wherever it was. Holding something teaches
    /// you it: a ghost can still cite what it once held.
    fn hold(&mut self, item: &str, who: &str) {
        self.holder.insert(item.to_string(), who.to_string());
        if matches!(self.story.kind(who), Some("character" | "npc")) {
            self.knows
                .entry(who.to_string())
                .or_default()
                .insert(item.to_string());
        }
    }

    /// The id an effect's argument names: `me` is the pov.
    fn who(&mut self, arg: &Value) -> R<String> {
        match self.eval(arg)? {
            V::Id(id) | V::Str(id) => Ok(id),
            other => Err(format!("expected someone or something, found `{other}`")),
        }
    }

    /// The storage key an expression names: `grate_up`, `me.grievance` →
    /// `skittles.grievance`, `vell.cornered`.
    fn path(&mut self, target: &Value) -> R<String> {
        match target {
            Value::Str(name) => Ok(match name.as_str() {
                "me" => self.pov.clone(),
                // A stat or resource of each character's own means the pov's:
                // `raise rewinds 1`.
                _ if matches!(self.story.kind(name), Some("stat" | "resource"))
                    && !self.story.has_tag(name, "shared") =>
                {
                    format!("{}.{name}", self.pov)
                }
                _ => name.clone(),
            }),
            _ => {
                let parts = target
                    .get("get")
                    .and_then(Value::as_arr)
                    .ok_or_else(|| format!("cannot set `{target}`"))?;
                let root = match parts.first() {
                    Some(Value::Str(root)) if root == "me" => self.pov.clone(),
                    Some(root) => self.who(root)?,
                    None => return Err("cannot set an empty path".into()),
                };
                let rest: Vec<&str> = parts[1..].iter().filter_map(Value::as_str).collect();
                Ok(format!("{root}.{}", rest.join(".")))
            }
        }
    }

    // ------------------------------------------------------------ expressions

    /// Evaluate a lowered expression.
    pub fn eval(&mut self, e: &Value) -> R<V> {
        let Value::Obj(pairs) = e else {
            return Ok(match e {
                Value::Str(name) => self.name(name)?,
                other => self.constant(other),
            });
        };
        let (op, x) = pairs.first().ok_or("an empty expression")?;
        let items = x.as_arr().unwrap_or(&[]);
        let named = |key: &str| f(f(e, "with"), key);
        Ok(match op.as_str() {
            "str" => V::Str(x.as_str().unwrap_or("").to_string()),
            // `roll d6`: under a `let`, rolled once; under a `get`, on every read.
            "roll" => {
                let sides = x
                    .as_str()
                    .unwrap_or("d6")
                    .trim_start_matches('d')
                    .parse()
                    .unwrap_or(6);
                V::Num(self.roll(sides) as f64)
            }
            "range" | "inclusive" => {
                let bound = |i: usize, run: &mut Self| match items.get(i) {
                    Some(Value::Null) | None => Ok(None),
                    Some(b) => run.eval(b).map(|v| v.num()),
                };
                let lo = bound(0, self)?;
                let hi = bound(1, self)?;
                V::Range {
                    lo,
                    hi,
                    inclusive: f(e, "inclusive") == &Value::Bool(true),
                }
            }
            "list" => V::List(items.iter().map(|i| self.eval(i)).collect::<R<_>>()?),
            "and" => V::Bool(self.eval(&items[0])?.truthy() && self.eval(&items[1])?.truthy()),
            "or" => V::Bool(self.eval(&items[0])?.truthy() || self.eval(&items[1])?.truthy()),
            "not" => V::Bool(!self.eval(x)?.truthy()),
            "neg" => V::Num(-self.eval(x)?.num().unwrap_or(0.0)),
            "==" | "!=" | "within" | "<" | ">" | "<=" | ">=" | "+" | "-" | "*" | "/" | "mod" => {
                let a = self.eval(&items[0])?;
                let b = self.eval(&items[1])?;
                binary(op, &a, &b)?
            }
            "get" => self.get(items)?,
            // `who.at(spot)`: is someone at a spot within their place?
            "at" => {
                let spot = items
                    .get(1)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let check = |run: &Self, who: &str| {
                    run.vals
                        .get(&format!("{who}.at"))
                        .and_then(V::name)
                        .is_some_and(|at| at == spot)
                };
                self.quantify(&items[0], check)?
            }
            "index" => match (self.eval(&items[0])?, self.eval(&items[1])?) {
                (V::List(list), V::Num(i)) => list.get(i as usize).cloned().unwrap_or(V::Null),
                _ => V::Null,
            },
            "knows" | "holds" => {
                let thing = items.get(1).and_then(Value::as_str).unwrap_or("");
                let check = |run: &Self, who: &str| {
                    if op == "knows" {
                        run.knows.get(who).is_some_and(|k| k.contains(thing))
                    } else {
                        run.holder.get(thing).is_some_and(|h| h == who)
                    }
                };
                self.quantify(&items[0], check)?
            }
            // `nobody.is(rook)` is simply false.
            "is" => match self.eval(&items[0])? {
                V::Id(who) | V::Str(who) => {
                    let what = self.eval(&items[1])?;
                    let what = what.name().unwrap_or("");
                    V::Bool(who == what || self.tagged(&who, what))
                }
                _ => V::Bool(false),
            },
            "has" => {
                let target = self.eval(&items[0])?;
                let target = target.name().unwrap_or("").to_string();
                if let Some(entries) = self.lists.get(&target) {
                    let wanted: Vec<String> = items[1..]
                        .iter()
                        .map(|i| i.as_str().unwrap_or("").to_string())
                        .collect();
                    V::Bool(wanted.iter().all(|w| entries.iter().any(|en| en.id == *w)))
                } else {
                    let thing = self.who(&items[1])?;
                    let place = place_of(&target).to_string();
                    // `here.has(knight)` is one thing; `here.has(statue)`,
                    // anything of that kind.
                    if matches!(self.story.kind(&thing), Some("character" | "npc" | "item")) {
                        V::Bool(self.location(&thing) == place)
                    } else {
                        let any = self
                            .story
                            .decls
                            .keys()
                            .any(|id| self.is_kind(id, &thing) && self.location(id) == place);
                        V::Bool(any)
                    }
                }
            }
            "in" => {
                let place = items
                    .get(1)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let check = |run: &Self, who: &str| run.inside(&run.location(who), &place);
                self.quantify(&items[0], check)?
            }
            "any" => {
                // `who.holds.any(tag)`: holds something with that tag.
                let who = match items[0].get("get").and_then(Value::as_arr) {
                    Some(path) => self.who(&path[0])?,
                    None => self.who(&items[0])?,
                };
                let tag = items.get(1).and_then(Value::as_str).unwrap_or("");
                V::Bool(
                    self.holder
                        .iter()
                        .any(|(item, h)| *h == who && self.story.has_tag(item, tag)),
                )
            }
            "where" => {
                let list = self.who(&items[0])?;
                let mark = items.get(1).and_then(Value::as_str).unwrap_or("");
                let entries = self.lists.get(&list).cloned().unwrap_or_default();
                V::List(
                    entries
                        .into_iter()
                        .filter(|en| en.mark.as_deref() == Some(mark))
                        .map(|en| V::Id(en.id))
                        .collect(),
                )
            }
            "count" | "one" => {
                let tag = items
                    .first()
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let place = match named("in") {
                    Value::Null => None,
                    p => Some(
                        self.eval(p)?
                            .name()
                            .map(|p| place_of(p).to_string())
                            .unwrap_or_default(),
                    ),
                };
                let found = self.things(&tag, place.as_deref());
                if op == "count" {
                    V::Num(found.len() as f64)
                } else {
                    // One of them, or nothing if there are none.
                    found.first().map_or(V::Null, |id| V::Id(id.clone()))
                }
            }
            "asleep" => {
                let who = self.who(&items[1])?;
                V::Bool(self.is_asleep(&who))
            }
            "nearest" => {
                let tag = items.first().and_then(Value::as_str).unwrap_or("");
                self.nearest(tag)?
            }
            "alone_with" => {
                let a = self.who(&items[0])?;
                match self.eval(&items[1])? {
                    V::Id(b) | V::Str(b) => V::Bool(self.alone_with(&a, &b)),
                    _ => V::Bool(false),
                }
            }
            "last_seen" => {
                let who = self.who(&items[0])?;
                let kind = items.get(1).and_then(Value::as_str);
                let tag = named("holding").as_str().unwrap_or("").to_string();
                self.last_seen(&who, kind, &tag)
            }
            // NPCs do not see acts yet: `self.saw(dash)` never holds.
            "saw" => V::Bool(false),
            other => return Err(format!("`{other}` is not supported by the interpreter yet")),
        })
    }

    /// Apply `check` to a subject, which may be `pov.any` or `pov.all`.
    fn quantify(&mut self, subject: &Value, check: impl Fn(&Self, &str) -> bool) -> R<V> {
        if let Some(path) = subject.get("get").and_then(Value::as_arr) {
            if path.first().and_then(Value::as_str) == Some("pov") {
                let povs = self.story.povs.clone();
                return Ok(V::Bool(match path.get(1).and_then(Value::as_str) {
                    Some("all") => povs.iter().all(|p| check(self, p)),
                    _ => povs.iter().any(|p| check(self, p)),
                }));
            }
        }
        let who = self.who(subject)?;
        Ok(V::Bool(check(self, &who)))
    }

    /// The value of a bare name.
    fn name(&mut self, n: &str) -> R<V> {
        match self.locals.get(n).cloned() {
            Some(Local::Value(v)) => return Ok(v),
            Some(Local::Expr(e)) => {
                if self.evaluating.iter().any(|x| x == n) {
                    return Err(format!("the let `{n}` is defined in terms of itself"));
                }
                self.evaluating.push(n.to_string());
                let v = self.eval(&e);
                self.evaluating.pop();
                return v;
            }
            None => {}
        }
        // `d6`, `d20`: a die, rolled each time it is read.
        if let Some(sides) = n.strip_prefix('d').and_then(|s| s.parse().ok()) {
            if self.story.kind(n).is_none() {
                return Ok(V::Num(self.roll(sides) as f64));
            }
        }
        Ok(self.global(n))
    }

    /// The value of a bare name that is not a local.
    fn global(&self, n: &str) -> V {
        match n {
            "me" => return V::Id(self.pov.clone()),
            "here" => return V::Id(self.here().to_string()),
            "nothing" | "nobody" => return V::Null,
            // Nothing is hushed unless a story says so.
            "hushed" => return self.vals.get("hushed").cloned().unwrap_or(V::Bool(false)),
            _ => {}
        }
        if let Some(v) = self.vals.get(n) {
            return v.clone();
        }
        // A stat or resource on its own means the pov's: `might >= 4`.
        if matches!(self.story.kind(n), Some("stat" | "resource")) {
            return self
                .vals
                .get(&format!("{}.{n}", self.pov))
                .cloned()
                .unwrap_or(V::Null);
        }
        V::Id(n.to_string())
    }

    /// `x.a.b`: walk a path of members.
    fn get(&mut self, path: &[Value]) -> R<V> {
        // `pov.all.here`, `pov.any.blessed`: the rest of the path, for each pov.
        if path.first().and_then(Value::as_str) == Some("pov") && path.len() > 2 {
            let all = path[1].as_str() == Some("all");
            let mut results = Vec::new();
            for p in self.story.povs.clone() {
                let mut each = vec![Value::str(p)];
                each.extend(path[2..].iter().cloned());
                results.push(self.get(&each)?.truthy());
            }
            return Ok(V::Bool(if all {
                results.iter().all(|r| *r)
            } else {
                results.iter().any(|r| *r)
            }));
        }
        let mut v = self.eval(path.first().ok_or("an empty path")?)?;
        for member in path[1..].iter().filter_map(Value::as_str) {
            v = self.member(&v, member)?;
        }
        Ok(v)
    }

    fn member(&mut self, v: &V, m: &str) -> R<V> {
        let id = match v {
            V::Id(id) => id.clone(),
            V::List(items) if m == "count" => return Ok(V::Num(items.len() as f64)),
            _ => return Ok(V::Null),
        };
        let story = self.story;
        Ok(match (id.as_str(), m) {
            ("clock", "now") => V::Num(f64::from(self.when())),
            ("pov", _) => {
                return Err(format!(
                    "`pov.{m}` needs a method after it, such as `pov.{m}.in(place)`"
                ))
            }
            // A `custom` character has no name until one is typed in.
            (_, "name") => match self.vals.get(&format!("{id}.name")) {
                Some(name) => name.clone(),
                None if story.has_tag(&id, "custom") => V::Str(String::new()),
                None => V::Str(self.label(&id)),
            },
            // `cohort.size`, `cohort.alive`: the Linkers playing, and those alive.
            ("cohort", "size") => V::Num(self.voters_count(false)),
            ("cohort", "alive") => V::Num(self.voters_count(true)),
            (_, "aka") => self.aka(&id),
            (_, "visits") => V::Num(f64::from(
                self.visits.get(self.here()).copied().unwrap_or(0),
            )),
            (_, "count") if self.lists.contains_key(&id) => V::Num(self.lists[&id].len() as f64),
            (_, "taken" | "secured") => V::Bool(
                self.holder
                    .get(&id)
                    .is_some_and(|h| matches!(story.kind(h), Some("character" | "npc"))),
            ),
            (_, "exists") => V::Bool(
                !self
                    .vals
                    .get(&format!("{id}.destroyed"))
                    .is_some_and(V::truthy),
            ),
            (_, "open") if story.kind(&id) == Some("way") => match story.field(&id, "open_when") {
                Value::Null => V::Bool(true),
                cond => {
                    let cond = cond.clone();
                    V::Bool(self.eval(&cond)?.truthy())
                }
            },
            (_, "alive") => V::Bool(!self.vals.get(&format!("{id}.dead")).is_some_and(V::truthy)),
            // `me.past`: re-living an earlier stop, as an echo.
            (_, "past") => V::Bool(self.past.contains_key(&id)),
            // `here.passed`: someone dashed through this stop.
            (_, "passed") => V::Bool(
                self.passed
                    .contains_key(&(self.when(), place_of(&id).to_string())),
            ),
            (_, "watched") => V::Bool(self.watched(&id)?),
            (_, "offstage") => V::Bool(self.offstage(&id)?),
            // `x.here`: is x where the pov is?
            (_, "here") => V::Bool(self.location(&id) == place_of(self.here())),
            // Not stored: a field of the thing's declaration, `knight.weight`.
            _ if !self.vals.contains_key(&format!("{id}.{m}"))
                && *story.field(&id, m) != Value::Null =>
            {
                self.constant(story.field(&id, m))
            }
            _ => self
                .vals
                .get(&format!("{id}.{m}"))
                .cloned()
                .unwrap_or(V::Null),
        })
    }

    /// The next of a character's other names, in its declared order.
    fn aka(&mut self, id: &str) -> V {
        let names = strs(self.story.field(id, "aka"));
        if names.is_empty() {
            return V::Str(self.label(id));
        }
        let order = self
            .story
            .field(id, "aka_order")
            .as_str()
            .unwrap_or("shuffled")
            .to_string();
        let n = names.len();
        let (bag, used) = self
            .aka
            .entry(id.to_string())
            .or_insert_with(|| (Vec::new(), 0))
            .clone();
        let (mut bag, mut used) = (bag, used);
        if bag.is_empty() || used == n {
            bag = (0..n).collect();
            match order.as_str() {
                "reversed" => bag.reverse(),
                "in order" => {}
                _ => {
                    // Fisher–Yates, from the run's seed.
                    for i in (1..n).rev() {
                        let j = self.roll(i as u64 + 1) as usize - 1;
                        bag.swap(i, j);
                    }
                }
            }
            used = 0;
        }
        let pick = if order == "random" {
            self.roll(n as u64) as usize - 1
        } else {
            bag[used]
        };
        self.aka.insert(id.to_string(), (bag, used + 1));
        V::Str(names[pick].clone())
    }

    /// Where something is: a character's place, an NPC's, or an item's holder's.
    fn location(&self, id: &str) -> String {
        if let Some(at) = self.at.get(id) {
            return place_of(at).to_string();
        }
        // A place, or a passage that is not someone's margin, is where it
        // is: `here.in(prison)`.
        let thing = matches!(self.story.kind(id), Some("character" | "npc" | "item"));
        if !thing && (self.story.passages.contains_key(id) || self.story.kind(id) == Some("place"))
        {
            return place_of(id).to_string();
        }
        if let Some(p) = self.place.get(id) {
            return p.clone();
        }
        // Held by someone or inside something: wherever that is. Otherwise
        // the holder is a place, or a passage in one.
        match self.holder.get(id) {
            Some(h)
                if h != id && matches!(self.story.kind(h), Some("character" | "npc" | "item")) =>
            {
                self.location(h)
            }
            Some(h) => place_of(h).to_string(),
            None => String::new(),
        }
    }

    /// Is `place` the same as `region`, or inside the map called `region`?
    fn inside(&self, place: &str, region: &str) -> bool {
        place == region
            || self
                .story
                .field(region, "rows")
                .as_arr()
                .is_some_and(|rows| {
                    rows.iter()
                        .flat_map(|r| r.as_arr().unwrap_or(&[]))
                        .any(|c| c.as_str() == Some(place))
                })
    }

    /// Everything with `tag`, in `place` if given.
    fn things(&self, tag: &str, place: Option<&str>) -> Vec<String> {
        let mut out: Vec<String> = self
            .story
            .decls
            .iter()
            .filter(|(id, d)| {
                let tagged = match tag {
                    "linker" => self.story.povs.contains(id) || self.at.contains_key(*id),
                    "living" => {
                        matches!(d.kind.as_str(), "character" | "npc")
                            && !self.vals.get(&format!("{id}.dead")).is_some_and(V::truthy)
                            && !self.tagged(id, "lifeless")
                    }
                    _ => d.kind == tag || self.tagged(id, tag),
                };
                tagged && place.is_none_or(|p| self.location(id) == p)
            })
            .map(|(id, _)| id.clone())
            .collect();
        out.sort();
        out
    }

    /// Has `id` the tag, itself or through what it is `like`?
    fn tagged(&self, id: &str, tag: &str) -> bool {
        let like = self.story.field(id, "like").as_str().unwrap_or("");
        self.story.has_tag(id, tag) || self.story.has_tag(like, tag)
    }

    /// A number from 1 to `sides`, from the run's seed (xorshift64).
    fn roll(&mut self, sides: u64) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        1 + self.rng % sides.max(1)
    }
}

/// The place a passage is in: `dining@1` and `dining_table:blueprint` are in
/// `dining` and `dining_table`; a room passage is its own place.
fn place_of(passage: &str) -> &str {
    passage.split(['@', ':']).next().unwrap_or(passage)
}

/// A list entry from a declaration's `holds`.
fn entry(v: &Value) -> Entry {
    Entry {
        id: f(v, "id").as_str().unwrap_or("").to_string(),
        label: f(v, "label").as_str().map(str::to_string),
        mark: f(v, "mark").as_str().map(str::to_string),
        struck: false,
    }
}

/// A list entry from an effect's words: `[id, {"str": label}?]`.
fn entry_from_words(words: &[Value], mark: Option<String>) -> Entry {
    let id = words
        .first()
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim_end_matches(':')
        .to_string();
    let label = words
        .get(1)
        .and_then(|w| w.get("str"))
        .and_then(Value::as_str)
        .map(str::to_string);
    Entry {
        id,
        label,
        mark,
        struck: false,
    }
}

fn binary(op: &str, a: &V, b: &V) -> R<V> {
    let nums = || match (a.num(), b.num()) {
        (Some(x), Some(y)) => Ok((x, y)),
        _ => Err(format!("`{op}` needs numbers, found `{a}` and `{b}`")),
    };
    Ok(match op {
        "==" => V::Bool(a.same(b)),
        "!=" => V::Bool(!a.same(b)),
        "within" => V::Bool(a.within(b)),
        "<" => V::Bool(nums()?.0 < nums()?.1),
        ">" => V::Bool(nums()?.0 > nums()?.1),
        "<=" => V::Bool(nums()?.0 <= nums()?.1),
        ">=" => V::Bool(nums()?.0 >= nums()?.1),
        "+" => V::Num(nums()?.0 + nums()?.1),
        "-" => V::Num(nums()?.0 - nums()?.1),
        "*" => V::Num(nums()?.0 * nums()?.1),
        "/" => V::Num(nums()?.0 / nums()?.1),
        _ => V::Num(nums()?.0.rem_euclid(nums()?.1)), // mod
    })
}
