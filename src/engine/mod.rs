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
//! This is a solo engine. Cohort play (seats, shared acts) comes later.

mod events;

pub use events::{catalogue_markdown, Event, EventType, CATALOGUE};

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use crate::json::{self, Value};
use crate::run::{Ask, Run, Story};

/// The engine's version. Logs recorded under one version replay the same
/// way under it; the platform records it with every reading.
pub const ENGINE_VERSION: u32 = 1;

/// The IR versions this engine reads.
pub const IR_VERSIONS: [&str; 1] = ["2"];

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
    /// The compiler that made the IR: `"0.5.0"`.
    pub inkjot: String,
    /// `story`, `story-game` or `game`.
    pub kind: String,
    /// `solo` or `cohort`.
    pub mode: String,
    /// The Nib a reading starts at.
    pub start: String,
    /// The `pov:` setting: who a Linker looks through, the default first.
    pub povs: Vec<String>,
    /// The characters marked `~ playable`, for a lobby to offer.
    pub playable: Vec<String>,
    /// The Inker's tags: `comedy`, `cozy`.
    pub tags: Vec<String>,
    /// The engine that read it: [`ENGINE_VERSION`].
    pub engine: u32,
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
                    format!("this is IR version `{version}`; this engine reads IR 2"),
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
                playable: story.playable(),
                tags: config_tags(ir),
                engine: ENGINE_VERSION,
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

    /// Begin a reading. The seed decides every roll and shuffle.
    pub fn start(&self, seed: u64, start: &Start) -> Result<State, Refused> {
        guarded(|| {
            let mut run = Run::new(Arc::clone(&self.story), seed);
            if !start.hints {
                run.turn_off_hints();
            }
            if let Some(pov) = &start.pov {
                if !self.story.povs().contains(pov) && !self.meta.playable.contains(pov) {
                    return Err(refused(
                        RefusedKind::StoryError,
                        format!("`{pov}` is not someone a Linker may play"),
                    ));
                }
            }
            if let Err(why) = run.start(start.nib.as_deref(), start.pov.as_deref()) {
                return Err(classify(&run, why, RefusedKind::StoryError));
            }
            let view = see(&mut run).map_err(|why| classify(&run, why, RefusedKind::StoryError))?;
            let mut ending = None;
            let events = events::events(&mut run, &view, &mut ending);
            Ok(State {
                inklink: self.clone(),
                seed,
                start: start.clone(),
                actions: Vec::new(),
                run,
                view,
                events,
                ending,
            })
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
}

impl Default for Start {
    fn default() -> Start {
        Start {
            nib: None,
            pov: None,
            hints: true,
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
}

/// A Link as the Linker sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub id: String,
    pub label: String,
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
    pub text: Vec<String>,
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
    pub fn view(&self) -> &View {
        &self.view
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

    /// Take an offered Link.
    pub fn take_link(&self, id: &str) -> Result<State, Refused> {
        self.act(&Action::TakeLink(id.to_string()))
    }

    /// Answer the open input.
    pub fn answer(&self, words: &[String]) -> Result<State, Refused> {
        self.act(&Action::Answer(words.to_vec()))
    }

    /// Do one thing, on a copy. `self` is never changed.
    pub fn act(&self, action: &Action) -> Result<State, Refused> {
        guarded(|| {
            self.check(action)?;
            let mut run = self.run.clone();
            let (done, otherwise) = match action {
                Action::TakeLink(id) => (run.choose(id), RefusedKind::StoryError),
                Action::Answer(words) => (run.answer(words), RefusedKind::BadAnswer),
                Action::Vote { who, how, proof } => {
                    (run.vote(who, how, proof), RefusedKind::BadAnswer)
                }
            };
            if let Err(why) = done {
                return Err(classify(&run, why, otherwise));
            }
            let view = see(&mut run).map_err(|why| classify(&run, why, RefusedKind::StoryError))?;
            let mut ending = self.ending.clone();
            let events = events::events(&mut run, &view, &mut ending);
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
            })
        })
    }

    /// Refuse, before playing anything, what the view shows cannot be done.
    fn check(&self, action: &Action) -> Result<(), Refused> {
        let asks = |jury: bool| self.view.asks.iter().any(|a| (a.kind == "accuse") == jury);
        match action {
            Action::TakeLink(id) if !self.view.links.iter().any(|l| l.id == *id) => Err(refused(
                RefusedKind::NotOffered,
                format!("no Link `{id}` is offered here"),
            )),
            Action::Answer(_) if !asks(false) => Err(refused(
                RefusedKind::NotAsking,
                "nothing here is asking for an answer",
            )),
            Action::Vote { .. } if !asks(true) => {
                Err(refused(RefusedKind::NotAsking, "no jury is sitting here"))
            }
            _ => Ok(()),
        }
    }
}

/// Look at the run after an action: the text it showed, and what is
/// offered now. Every path to a state goes through here once per action,
/// so a replay looks exactly as often as the reading did.
fn see(run: &mut Run) -> Result<View, String> {
    let text = run.take_output();
    let links: Vec<Link> = run
        .choices()?
        .into_iter()
        .map(|(id, label)| Link { id, label })
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
