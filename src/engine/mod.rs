//! The engine: the interpreter as the Inklinks platform calls it.
//!
//! ```text
//! let inklink = Inklink::load(&ir_json)?;          // once per IR
//! let state = inklink.start(seed, &Start::default())?;
//! state.view();                                    // the Nib, its text, its Links
//! let next = state.take_link("sit_down")?;         // a new State; `state` is unchanged
//! let again = inklink.replay(seed, &Start::default(), next.actions(), None)?;
//! assert_eq!(again.view(), next.view());
//! ```
//!
//! Four promises, each under a property test in `tests/engine.rs`:
//!
//! - **States are values.** Nothing changes a [`State`]. Taking a Link
//!   clones the run, plays the Link on the clone, and returns it, so the
//!   platform may keep any state it has seen and branch from it.
//! - **A reading is its log.** A state is fully decided by the IR, the
//!   seed, the [`Start`] and the [`Action`]s taken, which it carries.
//!   [`Inklink::replay`] rebuilds it, or any state before it (`upto`).
//! - **Determinism within an engine version.** The same log plays the same
//!   way under the same [`ENGINE_VERSION`]. The interpreter keeps every map
//!   ordered and draws every roll from the seed; nothing reads the clock,
//!   the environment or a hasher. A change that could make an old log play
//!   differently bumps the version.
//! - **Errors are data.** Every call returns a [`Refused`] rather than
//!   panicking or aborting: a Link that is not offered, an answer that does
//!   not fit, an action that runs past the step limit, even a bug in the
//!   interpreter (a caught panic). A refused call changes nothing.
//!
//! Each state also carries the [`Event`]s the action that made it caused,
//! for the platform's log (see [`events`]).
//!
//! **A table.** A [`Start`] with `seats` seats a cohort, one Linker each
//! (`guest`, a character `~ one per linker`, takes the next of its seats:
//! `guest_1`, `guest_2`, …). Each seat has its own view
//! ([`State::view_of`]); a Linker acts through [`Action::As`], and the
//! state's [`State::view`] is then theirs. Real time ([`Action::Wait`]) is
//! the table's.

mod events;

pub use events::{catalogue_markdown, Event, EventType, CATALOGUE};

use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use crate::json::{self, Value};
use crate::run::{Ask, Run, Story};
pub use crate::run::{Edge, Node, Para, Pause, Quote, Span, StoryMap, BAR};

/// The engine's version. Logs recorded under one version replay the same
/// way under it; the platform records it with every reading.
pub const ENGINE_VERSION: u32 = 1;

/// The IR versions this engine reads.
pub const IR_VERSIONS: [&str; 1] = ["3"];

// ------------------------------------------------------------------ errors

/// Why the engine refused a call. Nothing was changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub kind: RefusedKind,
    /// A sentence for the log, or for the Inker.
    pub message: String,
    /// In a replay: which action was refused, counting from 0.
    pub at: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusedKind {
    /// The IR is not JSON, not an Inklink, or a version this engine does
    /// not read.
    BadIr,
    /// No Link with that id is offered now.
    NotOffered,
    /// Nothing is asking for an answer, or a vote.
    NotAsking,
    /// The answer or vote does not fit what is asked.
    BadAnswer,
    /// The action did more than one action may: see `run::MAX_ENTERED`
    /// and `run::MAX_STEPS`. Usually a `next:` loop in the story.
    StepLimit,
    /// The story uses something the interpreter does not run yet.
    Unsupported,
    /// The story went somewhere the interpreter could not follow: a
    /// passage that is not there, a value of the wrong kind.
    StoryError,
    /// A bug in the engine, caught. Please report it with the log.
    Panic,
}

impl RefusedKind {
    /// A stable name, for an Elixir atom or a log line.
    pub fn code(self) -> &'static str {
        match self {
            RefusedKind::BadIr => "bad_ir",
            RefusedKind::NotOffered => "not_offered",
            RefusedKind::NotAsking => "not_asking",
            RefusedKind::BadAnswer => "bad_answer",
            RefusedKind::StepLimit => "step_limit",
            RefusedKind::Unsupported => "unsupported",
            RefusedKind::StoryError => "story_error",
            RefusedKind::Panic => "panic",
        }
    }
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind.code(), self.message)
    }
}

impl std::error::Error for Refused {}

fn refused(kind: RefusedKind, message: impl Into<String>) -> Refused {
    Refused {
        kind,
        message: message.into(),
        at: None,
    }
}

/// Run `work`, turning a panic into a [`Refused`].
fn guarded<T>(work: impl FnOnce() -> Result<T, Refused>) -> Result<T, Refused> {
    match catch_unwind(AssertUnwindSafe(work)) {
        Ok(result) => result,
        Err(payload) => {
            let why = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "the engine panicked".to_string());
            Err(refused(RefusedKind::Panic, why))
        }
    }
}

// ------------------------------------------------------------------ loading

/// What the platform shows about an Inklink before anyone reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Meta {
    pub title: String,
    pub ifid: String,
    /// The IR version: `"2"`.
    pub ir: String,
    /// The language version the source declared: `"0.6.0"`.
    pub inkjot: String,
    /// `story`, `story-game` or `game`.
    pub kind: String,
    /// `solo` or `cohort`.
    pub mode: String,
    /// The Nib a reading starts at.
    pub start: String,
    /// The `pov:` setting: who a Linker looks through, the default first.
    pub povs: Vec<String>,
    /// The characters marked `~ selectable`, for a lobby to offer.
    pub selectable: Vec<String>,
    /// The Inker's tags: `comedy`, `cozy`.
    pub tags: Vec<String>,
    /// The engine that read it: [`ENGINE_VERSION`].
    pub engine: u32,
    /// Inquests a reading has: the `inquests` resource, when the Inklink
    /// declares it and hides at least one fork. `None`: no Inquests.
    pub inquests: Option<u32>,
    /// `cohort: 5..=12`: how many may sit at a table, the most `None` for
    /// no limit. `None` for an Inklink with no `cohort:`.
    pub cohort: Option<(u32, Option<u32>)>,
}

/// A loaded IR. Cheap to clone: every state shares one.
#[derive(Clone, Debug)]
pub struct Inklink {
    story: Arc<Story>,
    meta: Arc<Meta>,
}

impl Inklink {
    /// Read an IR from its JSON text. Unknown fields are ignored, so a
    /// newer compiler's additions do not stop an older engine.
    pub fn load(ir: &str) -> Result<Inklink, Refused> {
        guarded(|| {
            let ir = json::parse(ir).map_err(|e| refused(RefusedKind::BadIr, e))?;
            Inklink::from_ir(&ir)
        })
    }

    /// Read an IR already parsed, as `inkjot::build` returns it.
    pub fn from_ir(ir: &Value) -> Result<Inklink, Refused> {
        guarded(|| {
            let text = |key: &str| ir.get(key).and_then(Value::as_str).unwrap_or("");
            let version = text("ir");
            if !IR_VERSIONS.contains(&version) {
                return Err(refused(
                    RefusedKind::BadIr,
                    format!("this is IR version `{version}`; this engine reads IR 3"),
                ));
            }
            let story = Story::new(ir).map_err(|e| refused(RefusedKind::BadIr, e))?;
            let meta = Meta {
                title: text("title").to_string(),
                ifid: text("ifid").to_string(),
                ir: version.to_string(),
                inkjot: text("inkjot").to_string(),
                kind: setting(ir, "kind").unwrap_or("story").to_string(),
                mode: setting(ir, "mode").unwrap_or("solo").to_string(),
                start: text("start").to_string(),
                povs: story.povs().to_vec(),
                selectable: story.selectable(),
                tags: config_tags(ir),
                engine: ENGINE_VERSION,
                inquests: inquests(ir).filter(|_| story.has_forks()),
                cohort: story.cohort_range(),
            };
            Ok(Inklink {
                story: Arc::new(story),
                meta: Arc::new(meta),
            })
        })
    }

    pub fn meta(&self) -> &Meta {
        &self.meta
    }

    /// The whole story as a graph: passages, and the Links, forks, `next:`s
    /// and ways between them.
    pub fn map(&self) -> StoryMap {
        self.story.map()
    }

    /// The passages whose forks may be asked for from `nib` (`~ inquest
    /// from nib`). Static: it depends on the story, not the reading.
    pub fn asked_from(&self, nib: &str) -> Vec<String> {
        self.story.asked_from(nib)
    }

    /// Begin a reading. The seed decides every roll and shuffle.
    pub fn start(&self, seed: u64, start: &Start) -> Result<State, Refused> {
        guarded(|| {
            let mut run = Run::new(Arc::clone(&self.story), seed);
            if !start.hints {
                run.turn_off_hints();
            }
            if let Some(pov) = &start.pov {
                if !self.story.povs().contains(pov) && !self.meta.selectable.contains(pov) {
                    return Err(refused(
                        RefusedKind::StoryError,
                        format!("`{pov}` is not someone a Linker may play"),
                    ));
                }
            }
            if !start.seats.is_empty() {
                return self.start_table(run, seed, start);
            }
            if let Err(why) = run.start(start.nib.as_deref(), start.pov.as_deref()) {
                return Err(classify(&run, why, RefusedKind::StoryError));
            }
            let view = see(&mut run).map_err(|why| classify(&run, why, RefusedKind::StoryError))?;
            let forks = forks_of(&mut run, &view)
                .map_err(|why| classify(&run, why, RefusedKind::StoryError))?;
            let mut ending = None;
            let events = events::events(&mut run, &view, &mut ending, Value::obj::<&str>([]));
            Ok(State {
                inklink: self.clone(),
                seed,
                start: start.clone(),
                actions: Vec::new(),
                run,
                view,
                events,
                ending,
                forks,
                views: BTreeMap::new(),
            })
        })
    }

    /// Seat a table: each seat must be someone a Linker may play, and the
    /// table a size the Inklink's `cohort:` allows.
    fn start_table(&self, mut run: Run, seed: u64, start: &Start) -> Result<State, Refused> {
        for seat in &start.seats {
            if !self.meta.selectable.contains(seat) && !self.story.povs().contains(seat) {
                return Err(refused(
                    RefusedKind::StoryError,
                    format!("`{seat}` is not someone a Linker may play"),
                ));
            }
        }
        let n = start.seats.len() as u32;
        if let Some((lo, hi)) = self.meta.cohort {
            if n < lo || hi.is_some_and(|hi| n > hi) {
                let most = hi.map_or(String::from(" or more"), |h| format!(" to {h}"));
                return Err(refused(
                    RefusedKind::StoryError,
                    format!("a table of {n}; this Inklink seats {lo}{most}"),
                ));
            }
        }
        let who: Vec<&str> = start.seats.iter().map(String::as_str).collect();
        if let Err(why) = run.start_cohort(start.nib.as_deref(), &who) {
            return Err(classify(&run, why, RefusedKind::StoryError));
        }
        let first = run.seated()[0].clone();
        let views = see_all(&mut run, &BTreeMap::new())
            .map_err(|why| classify(&run, why, RefusedKind::StoryError))?;
        run.sit(&first)
            .map_err(|why| classify(&run, why, RefusedKind::StoryError))?;
        let view = views[&first].clone();
        let mut ending = None;
        let events = events::events(&mut run, &view, &mut ending, Value::obj::<&str>([]));
        Ok(State {
            inklink: self.clone(),
            seed,
            start: start.clone(),
            actions: Vec::new(),
            run,
            view,
            events,
            ending,
            forks: BTreeMap::new(),
            views,
        })
    }

    /// Rebuild a reading from its log: the state after the first `upto`
    /// actions, or after all of them. A refused action is reported with
    /// its index in `at`.
    pub fn replay(
        &self,
        seed: u64,
        start: &Start,
        actions: &[Action],
        upto: Option<usize>,
    ) -> Result<State, Refused> {
        let upto = upto.unwrap_or(actions.len()).min(actions.len());
        let mut state = self.start(seed, start)?;
        for (i, action) in actions[..upto].iter().enumerate() {
            state = state
                .act(action)
                .map_err(|r| Refused { at: Some(i), ..r })?;
        }
        Ok(state)
    }

    /// Rebuild a reading from its log, keeping every state on the way:
    /// the start, then one per action. Going back needs earlier states,
    /// and their events, concatenated, are the reading's engine events.
    pub fn replay_all(
        &self,
        seed: u64,
        start: &Start,
        actions: &[Action],
    ) -> Result<Vec<State>, Refused> {
        let mut states = vec![self.start(seed, start)?];
        for (i, action) in actions.iter().enumerate() {
            let next = states
                .last()
                .expect("the start is always there")
                .act(action)
                .map_err(|r| Refused { at: Some(i), ..r })?;
            states.push(next);
        }
        Ok(states)
    }
}

/// `kind: story` and the like, from the config.
fn setting<'a>(ir: &'a Value, name: &str) -> Option<&'a str> {
    ir.get("config")?
        .as_arr()?
        .iter()
        .find(|s| s.get("setting").and_then(Value::as_str) == Some(name))?
        .get("value")?
        .as_str()
}

/// `resource inquests: 3`.
fn inquests(ir: &Value) -> Option<u32> {
    ir.get("config")?
        .as_arr()?
        .iter()
        .find(|s| {
            s.get("declare").and_then(Value::as_str) == Some("resource")
                && s.get("id").and_then(Value::as_str) == Some("inquests")
        })?
        .get("value")?
        .as_u64()
        .map(|n| n as u32)
}

fn config_tags(ir: &Value) -> Vec<String> {
    let tags = ir
        .get("config")
        .and_then(Value::as_arr)
        .and_then(|c| {
            c.iter()
                .find(|s| s.get("setting").and_then(Value::as_str) == Some("tags"))
        })
        .and_then(|s| s.get("tags"))
        .and_then(Value::as_arr)
        .unwrap_or(&[]);
    tags.iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

// ------------------------------------------------------------------ playing

/// Where and as whom a reading begins. The default is the story's own
/// start, through its default pov, with hints on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Start {
    /// A Nib to start at instead of the story's start.
    pub nib: Option<String>,
    /// A character to look through instead of the default.
    pub pov: Option<String>,
    /// The Linker wants hints offered (when the story has them).
    pub hints: bool,
    /// A table: one entry per Linker, the character each plays (`guest`
    /// for a character `~ one per linker`). Empty: one Linker, solo.
    pub seats: Vec<String>,
}

impl Default for Start {
    fn default() -> Start {
        Start {
            nib: None,
            pov: None,
            hints: true,
            seats: Vec::new(),
        }
    }
}

/// One thing a Linker does. A reading's log is a list of these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Take the Link with this id.
    TakeLink(String),
    /// Answer the open input: a name, or the words chosen.
    Answer(Vec<String>),
    /// Vote in the jury: a suspect, a method, and the proof cited.
    Vote {
        who: String,
        how: String,
        proof: Vec<String>,
    },
    /// Take a fork an Inquest showed. `from` is the passage the Linker asked
    /// from, when it is a later one (`~ inquest from`); the state taken from
    /// is then the earlier visit's.
    TakeFork { fork: String, from: Option<String> },
    /// Real time passes, in milliseconds, as the platform counts it: a
    /// Link timed `~ after 5m` is taken by itself when its time comes.
    Wait(u64),
    /// At a table: the Linker in `seat` does `action`.
    As { seat: String, action: Box<Action> },
}

/// A Link as the Linker sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub id: String,
    /// The label, with its styles: `Para::plain()` reads it as text.
    pub label: Para,
    /// A Link timed `~ after 5m` cannot be taken; it is taken by itself
    /// this many milliseconds from now, when the platform says that much
    /// time has passed (`Action::Wait`). `None` for every other Link.
    pub due_in: Option<u64>,
}

/// What the Linker sees now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    /// The Nib the pov is at.
    pub nib: String,
    /// Its tags: `finale`, `epilogue`, `act`.
    pub tags: Vec<String>,
    /// Who the Linker is looking through.
    pub pov: String,
    /// The clock's stop.
    pub now: u32,
    /// The paragraphs shown by the last action (or the start).
    pub text: Vec<Para>,
    /// The Links offered.
    pub links: Vec<Link>,
    /// The inputs open, and what they accept.
    pub asks: Vec<Ask>,
    /// Nothing is offered and nothing asked: the reading is over.
    pub over: bool,
}

/// A reading at one moment: a value, never changed.
#[derive(Clone)]
pub struct State {
    inklink: Inklink,
    seed: u64,
    start: Start,
    actions: Vec<Action>,
    run: Run,
    view: View,
    /// What the action that made this state caused.
    events: Vec<Event>,
    /// The last finale reached in this reading.
    ending: Option<String>,
    /// The forks behind each offered Link here, worked out with the view,
    /// so asking for them is a read. Never part of the view.
    forks: BTreeMap<String, Vec<Link>>,
    /// At a table, each seat's view; `view` is the last to act's.
    views: BTreeMap<String, View>,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("seed", &self.seed)
            .field("start", &self.start)
            .field("actions", &self.actions)
            .field("view", &self.view)
            .field("events", &self.events)
            .finish_non_exhaustive()
    }
}

impl State {
    /// What the Linker sees: alone, the one Linker; at a table, the one who
    /// acted last (at the start, the first seat).
    pub fn view(&self) -> &View {
        &self.view
    }

    /// The seats at a table, in the order they joined: `guest_1`, … Empty
    /// for a solo reading.
    pub fn seats(&self) -> &[String] {
        self.run.seated()
    }

    /// What the Linker in `seat` sees. `None` for a seat not at the table.
    pub fn view_of(&self, seat: &str) -> Option<&View> {
        self.views.get(seat)
    }

    /// Do `action` as the Linker in `seat`.
    pub fn act_as(&self, seat: &str, action: &Action) -> Result<State, Refused> {
        self.act(&Action::As {
            seat: seat.to_string(),
            action: Box::new(action.clone()),
        })
    }

    /// The engine's events from the action that made this state (or from
    /// the start), in order.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// The last finale this reading reached, if any.
    pub fn ending(&self) -> Option<&str> {
        self.ending.as_deref()
    }

    /// The log: every action taken since the start, in order.
    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn started(&self) -> &Start {
        &self.start
    }

    pub fn inklink(&self) -> &Inklink {
        &self.inklink
    }

    /// Let real time pass: `ms` milliseconds.
    pub fn wait(&self, ms: u64) -> Result<State, Refused> {
        self.act(&Action::Wait(ms))
    }

    /// Take an offered Link.
    pub fn take_link(&self, id: &str) -> Result<State, Refused> {
        self.act(&Action::TakeLink(id.to_string()))
    }

    /// Answer the open input.
    pub fn answer(&self, words: &[String]) -> Result<State, Refused> {
        self.act(&Action::Answer(words.to_vec()))
    }

    /// The forks that stand in for the offered Link `link`, in the order
    /// the Inker wrote them. `from`: the passage the Linker is asking from,
    /// when it is not this state's Nib. Forks are never in the view: they
    /// cross only when asked for, one Link's at a time.
    pub fn forks(&self, link: &str, from: Option<&str>) -> Result<Vec<Link>, Refused> {
        guarded(|| {
            let from = from.filter(|f| *f != self.view.nib);
            if from.is_none() {
                return Ok(self.forks.get(link).cloned().unwrap_or_default());
            }
            let mut run = self.run.clone();
            let found = run
                .forks(link, from)
                .map_err(|why| classify(&run, why, RefusedKind::StoryError))?;
            Ok(found
                .into_iter()
                .map(|(id, label)| Link {
                    id,
                    label,
                    due_in: None,
                })
                .collect())
        })
    }

    /// May any fork be taken here, asking from `from` (or from here)?
    pub fn any_forks(&self, from: Option<&str>) -> Result<bool, Refused> {
        let mut any = false;
        for link in &self.view.links {
            any = any || !self.forks(&link.id, from)?.is_empty();
        }
        Ok(any)
    }

    /// Take a fork an Inquest showed.
    pub fn take_fork(&self, fork: &str, from: Option<&str>) -> Result<State, Refused> {
        self.act(&Action::TakeFork {
            fork: fork.to_string(),
            from: from.map(str::to_string),
        })
    }

    /// Do one thing, on a copy. `self` is never changed.
    pub fn act(&self, action: &Action) -> Result<State, Refused> {
        guarded(|| {
            let table = !self.views.is_empty();
            // At a table, everything but real time is someone's to do.
            let (seat, inner) = match action {
                Action::As { seat, action } => (Some(seat.as_str()), action.as_ref()),
                other => (None, other),
            };
            let view = match seat {
                _ if matches!(inner, Action::As { .. }) => {
                    return Err(refused(
                        RefusedKind::BadAnswer,
                        "`As` holds one action, not another `As`",
                    ))
                }
                Some(s) if !table => {
                    return Err(refused(
                        RefusedKind::NotOffered,
                        format!("`{s}`: this reading is not at a table"),
                    ))
                }
                Some(s) => self.views.get(s).ok_or_else(|| {
                    refused(RefusedKind::NotOffered, format!("nobody sits at `{s}`"))
                })?,
                None if table && !matches!(inner, Action::Wait(_)) => {
                    return Err(refused(
                        RefusedKind::NotOffered,
                        "at a table, say who acts: `As { seat, action }`",
                    ))
                }
                None => &self.view,
            };
            Self::check(view, inner, |link, from| self.forks(link, from))?;
            let mut run = self.run.clone();
            if let Some(s) = seat {
                run.sit(s)
                    .map_err(|why| classify(&run, why, RefusedKind::StoryError))?;
            }
            let (done, otherwise) = match inner {
                Action::TakeLink(id) => (run.choose(id), RefusedKind::StoryError),
                Action::Answer(words) => (run.answer(words), RefusedKind::BadAnswer),
                Action::Vote { who, how, proof } => {
                    (run.vote(who, how, proof), RefusedKind::BadAnswer)
                }
                Action::TakeFork { fork, from } => {
                    let from = from.as_deref().filter(|f| *f != self.view.nib);
                    (run.choose_fork(fork, from), RefusedKind::StoryError)
                }
                Action::Wait(ms) => (run.wait(*ms), RefusedKind::StoryError),
                Action::As { .. } => unreachable!("refused above"),
            };
            if let Err(why) = done {
                return Err(classify(&run, why, otherwise));
            }
            let (view, views, forks) = if table {
                let views = see_all(&mut run, &self.views)
                    .map_err(|why| classify(&run, why, RefusedKind::StoryError))?;
                let actor = seat.unwrap_or(&self.view.pov).to_string();
                run.sit(&actor)
                    .map_err(|why| classify(&run, why, RefusedKind::StoryError))?;
                (views[&actor].clone(), views, BTreeMap::new())
            } else {
                let view =
                    see(&mut run).map_err(|why| classify(&run, why, RefusedKind::StoryError))?;
                let forks = forks_of(&mut run, &view)
                    .map_err(|why| classify(&run, why, RefusedKind::StoryError))?;
                (view, BTreeMap::new(), forks)
            };
            let mut ending = self.ending.clone();
            let changes = changes(&self.run.tracked(), &run.tracked());
            let events = events::events(&mut run, &view, &mut ending, changes);
            let mut actions = self.actions.clone();
            actions.push(action.clone());
            Ok(State {
                inklink: self.inklink.clone(),
                seed: self.seed,
                start: self.start.clone(),
                actions,
                run,
                view,
                events,
                ending,
                forks,
                views,
            })
        })
    }

    /// Refuse, before playing anything, what the view shows cannot be done.
    fn check(
        view: &View,
        action: &Action,
        forks: impl Fn(&str, Option<&str>) -> Result<Vec<Link>, Refused>,
    ) -> Result<(), Refused> {
        let asks = |jury: bool| view.asks.iter().any(|a| (a.kind == "accuse") == jury);
        match action {
            Action::TakeLink(id) if !view.links.iter().any(|l| l.id == *id) => Err(refused(
                RefusedKind::NotOffered,
                format!("no Link `{id}` is offered here"),
            )),
            Action::TakeLink(id)
                if view.links.iter().any(|l| l.id == *id && l.due_in.is_some()) =>
            {
                Err(refused(
                    RefusedKind::NotOffered,
                    format!("the Link `{id}` is taken by itself, when its time comes"),
                ))
            }
            Action::Answer(_) if !asks(false) => Err(refused(
                RefusedKind::NotAsking,
                "nothing here is asking for an answer",
            )),
            Action::Vote { .. } if !asks(true) => {
                Err(refused(RefusedKind::NotAsking, "no jury is sitting here"))
            }
            Action::TakeFork { fork, from } => {
                let mut shown = false;
                for link in &view.links {
                    shown = shown
                        || forks(&link.id, from.as_deref())?
                            .iter()
                            .any(|f| f.id == *fork);
                }
                if shown {
                    Ok(())
                } else {
                    Err(refused(
                        RefusedKind::NotOffered,
                        format!("no fork `{fork}` can be taken here"),
                    ))
                }
            }
            _ => Ok(()),
        }
    }
}

/// Every seat's view after an action, in seating order. A seat the action
/// showed nothing new to, still at the same Nib, keeps the text it was
/// reading.
fn see_all(
    run: &mut Run,
    before: &BTreeMap<String, View>,
) -> Result<BTreeMap<String, View>, String> {
    let mut views = BTreeMap::new();
    for seat in run.seated().to_vec() {
        run.sit(&seat)?;
        let mut view = see(run)?;
        if let Some(old) = before.get(&seat) {
            if view.text.is_empty() && old.nib == view.nib {
                view.text = old.text.clone();
            }
        }
        views.insert(seat, view);
    }
    Ok(views)
}

/// Look at the run after an action: the text it showed, and what is
/// offered now. Every path to a state goes through here once per action,
/// so a replay looks exactly as often as the reading did.
fn see(run: &mut Run) -> Result<View, String> {
    let text = run.take_text();
    let links: Vec<Link> = run
        .offers()?
        .into_iter()
        .map(|(id, label)| {
            let due_in = run.due_in_id(&id);
            Link { id, label, due_in }
        })
        .collect();
    let asks = run.asks()?;
    let nib = run.here().to_string();
    let over = links.is_empty() && asks.is_empty() && !run.waiting();
    Ok(View {
        tags: run.story().passage_tags(&nib).to_vec(),
        nib,
        pov: run.pov.clone(),
        now: run.now(),
        text,
        links,
        asks,
        over,
    })
}

/// How the tracked values changed: `{"leo.trust": [2, 3]}`, a value that
/// is new reading `null` before.
fn changes(
    before: &BTreeMap<String, crate::run::V>,
    after: &BTreeMap<String, crate::run::V>,
) -> Value {
    let mut out = Vec::new();
    for (key, now) in after {
        let then = before.get(key);
        if then != Some(now) {
            let then = then.map_or(Value::Null, json_of);
            out.push((key.clone(), Value::Arr(vec![then, json_of(now)])));
        }
    }
    for (key, then) in before {
        if !after.contains_key(key) {
            out.push((key.clone(), Value::Arr(vec![json_of(then), Value::Null])));
        }
    }
    Value::Obj(out)
}

/// A value as JSON: a number is a number, an id or text a string.
pub(super) fn json_of(v: &crate::run::V) -> Value {
    use crate::run::V;
    match v {
        V::Null => Value::Null,
        V::Bool(b) => Value::Bool(*b),
        V::Num(n) if n.fract() == 0.0 && n.abs() < 1e15 => Value::Num((*n as i64).to_string()),
        V::Num(n) => Value::Num(n.to_string()),
        V::Str(s) | V::Id(s) => Value::str(s),
        V::List(items) => Value::Arr(items.iter().map(json_of).collect()),
        other => Value::str(other.to_string()),
    }
}

/// The forks behind each offered Link in `view`, asked from here.
fn forks_of(run: &mut Run, view: &View) -> Result<BTreeMap<String, Vec<Link>>, String> {
    let mut out = BTreeMap::new();
    for link in &view.links {
        let found: Vec<Link> = run
            .forks(&link.id, None)?
            .into_iter()
            .map(|(id, label)| Link {
                id,
                label,
                due_in: None,
            })
            .collect();
        if !found.is_empty() {
            out.insert(link.id.clone(), found);
        }
    }
    Ok(out)
}

/// Name an interpreter error: the step limit, an unsupported feature, or
/// `otherwise`.
fn classify(run: &Run, why: String, otherwise: RefusedKind) -> Refused {
    let kind = if run.work().over_limit() {
        RefusedKind::StepLimit
    } else if why.contains("not supported by the interpreter yet") {
        RefusedKind::Unsupported
    } else {
        otherwise
    };
    refused(kind, why)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A panic inside the engine comes back as a refusal, not an unwind
    /// into the caller (which, across a NIF, would take down the VM).
    #[test]
    fn a_panic_is_refused_not_raised() {
        let r = guarded(|| -> Result<(), Refused> { panic!("a bug") }).unwrap_err();
        assert_eq!((r.kind, r.message.as_str()), (RefusedKind::Panic, "a bug"));
        let r = guarded(|| -> Result<(), Refused> { panic!("{} bugs", 2) }).unwrap_err();
        assert_eq!(r.message, "2 bugs");
    }
}
