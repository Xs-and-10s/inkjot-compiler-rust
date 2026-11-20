//! The Inkjot engine as an Erlang NIF: `Inkjot.Native` on the Elixir side.
//!
//! A thin layer. Every call goes to `inkjot::engine`, which keeps the four
//! promises (states are values, a reading is its log, determinism within
//! an engine version, errors as data); this crate only turns Rust values
//! into terms and back.
//!
//! - An `Inklink` and a `State` cross as resources: the Elixir side holds
//!   a reference, never a copy, and the BEAM frees them when it drops it.
//! - Every call that plays the story (`start`, `act`, `take_link`,
//!   `replay`, `replay_all`) or reads a whole file (`build`, `load`) runs
//!   on a dirty CPU scheduler. A Link takes 0.2 to 0.6 ms on the examples,
//!   too close to the 1 ms a normal scheduler may be held, and it grows
//!   with a long game. The calls that only read a state (`view`, `events`,
//!   `meta`, `actions`) take microseconds and run on normal schedulers.
//! - A refusal is `{:error, {code, message, at}}`: `code` an atom
//!   (`:not_offered`), `at` the index of the refused action in a replay,
//!   else `nil`. A term of the wrong shape raises `ArgumentError`.

use inkjot::engine::{Action, Event, Inklink, Para, Refused, Start, State, View, ENGINE_VERSION};
use inkjot::json::Value;
use rustler::{Atom, Binary, Encoder, Env, NifResult, ResourceArc, Term};

mod atoms {
    rustler::atoms! {
        ok,
        error,
        take_link,
        answer,
        vote,
        nib,
        pov,
        hints,
    }
}

// ------------------------------------------------------------- resources

pub struct InklinkResource(Inklink);
pub struct StateResource(State);

#[rustler::resource_impl]
impl rustler::Resource for InklinkResource {}

#[rustler::resource_impl]
impl rustler::Resource for StateResource {}

type InklinkRef = ResourceArc<InklinkResource>;
type StateRef = ResourceArc<StateResource>;

// ------------------------------------------------------------- encoding

/// An Elixir map with atom keys: `%{nib: "hall", now: 0}`.
fn atom_map<'a>(env: Env<'a>, pairs: &[(&str, Term<'a>)]) -> Term<'a> {
    let keys: Vec<Term<'a>> = pairs
        .iter()
        .map(|(k, _)| Atom::from_str(env, k).expect("an atom").encode(env))
        .collect();
    let values: Vec<Term<'a>> = pairs.iter().map(|(_, v)| *v).collect();
    Term::map_from_term_arrays(env, &keys, &values).expect("distinct keys")
}

/// A JSON value as a term: objects become maps with string keys, as the
/// platform's event log stores them; numbers become integers when whole.
fn json_term<'a>(env: Env<'a>, v: &Value) -> Term<'a> {
    match v {
        Value::Null => rustler::types::atom::nil().encode(env),
        Value::Bool(b) => b.encode(env),
        Value::Num(n) => match n.parse::<i64>() {
            Ok(i) => i.encode(env),
            Err(_) => n.parse::<f64>().unwrap_or(f64::NAN).encode(env),
        },
        Value::Str(s) => s.encode(env),
        Value::Arr(items) => items
            .iter()
            .map(|i| json_term(env, i))
            .collect::<Vec<_>>()
            .encode(env),
        Value::Obj(pairs) => {
            let keys: Vec<Term<'a>> = pairs.iter().map(|(k, _)| k.encode(env)).collect();
            let values: Vec<Term<'a>> = pairs.iter().map(|(_, v)| json_term(env, v)).collect();
            Term::map_from_term_arrays(env, &keys, &values).expect("distinct keys")
        }
    }
}

fn refused_term<'a>(env: Env<'a>, r: &Refused) -> Term<'a> {
    let code = Atom::from_str(env, r.kind.code()).expect("an atom");
    (atoms::error(), (code, r.message.as_str(), r.at)).encode(env)
}

fn ok<'a>(env: Env<'a>, t: Term<'a>) -> Term<'a> {
    (atoms::ok(), t).encode(env)
}

/// A paragraph: `%{plain: "…", runs: [%{text: "…", style: ["italic"]}]}`.
fn para_term<'a>(env: Env<'a>, p: &Para) -> Term<'a> {
    atom_map(
        env,
        &[
            ("plain", p.plain().encode(env)),
            ("runs", runs_term(env, p)),
        ],
    )
}

/// A paragraph's runs: `[%{text: "…", style: ["italic"]}]`.
fn runs_term<'a>(env: Env<'a>, p: &Para) -> Term<'a> {
    let runs: Vec<Term<'a>> = p
        .runs
        .iter()
        .map(|r| {
            atom_map(
                env,
                &[("text", r.text.encode(env)), ("style", r.style.encode(env))],
            )
        })
        .collect();
    runs.encode(env)
}

fn view_term<'a>(env: Env<'a>, v: &View) -> Term<'a> {
    let links: Vec<Term<'a>> = v
        .links
        .iter()
        .map(|l| {
            atom_map(
                env,
                &[
                    ("id", l.id.encode(env)),
                    ("label", l.label.plain().encode(env)),
                    ("runs", runs_term(env, &l.label)),
                ],
            )
        })
        .collect();
    let asks: Vec<Term<'a>> = v
        .asks
        .iter()
        .map(|a| {
            atom_map(
                env,
                &[
                    ("kind", a.kind.encode(env)),
                    ("prompt", a.prompt.encode(env)),
                    ("options", a.options.encode(env)),
                    ("methods", a.methods.encode(env)),
                    ("length", a.length.encode(env)),
                    ("voted", a.voted.encode(env)),
                ],
            )
        })
        .collect();
    atom_map(
        env,
        &[
            ("nib", v.nib.encode(env)),
            ("tags", v.tags.encode(env)),
            ("pov", v.pov.encode(env)),
            ("now", v.now.encode(env)),
            (
                "text",
                v.text
                    .iter()
                    .map(|p| para_term(env, p))
                    .collect::<Vec<_>>()
                    .encode(env),
            ),
            ("links", links.encode(env)),
            ("asks", asks.encode(env)),
            ("over", v.over.encode(env)),
        ],
    )
}

fn event_term<'a>(env: Env<'a>, e: &Event) -> Term<'a> {
    json_term(env, &e.to_json())
}

fn action_term<'a>(env: Env<'a>, a: &Action) -> Term<'a> {
    match a {
        Action::TakeLink(id) => (atoms::take_link(), id).encode(env),
        Action::Answer(words) => (atoms::answer(), words).encode(env),
        Action::Vote { who, how, proof } => (atoms::vote(), who, how, proof).encode(env),
    }
}

// ------------------------------------------------------------- decoding

/// `%{nib: "dining@1", pov: "wren", hints: false}`; every key optional.
fn decode_start(t: Term) -> NifResult<Start> {
    let mut start = Start::default();
    let get = |key: Atom| t.map_get(key).ok();
    if let Some(nib) = get(atoms::nib()) {
        start.nib = nib.decode()?;
    }
    if let Some(pov) = get(atoms::pov()) {
        start.pov = pov.decode()?;
    }
    if let Some(hints) = get(atoms::hints()) {
        start.hints = hints.decode()?;
    }
    Ok(start)
}

/// `{:take_link, id}`, `{:answer, [words]}`, `{:vote, who, how, [proof]}`.
fn decode_action(t: Term) -> NifResult<Action> {
    let parts = rustler::types::tuple::get_tuple(t)?;
    let tag: Atom = parts.first().ok_or(rustler::Error::BadArg)?.decode()?;
    match (parts.len(), tag) {
        (2, a) if a == atoms::take_link() => Ok(Action::TakeLink(parts[1].decode()?)),
        (2, a) if a == atoms::answer() => Ok(Action::Answer(parts[1].decode()?)),
        (4, a) if a == atoms::vote() => Ok(Action::Vote {
            who: parts[1].decode()?,
            how: parts[2].decode()?,
            proof: parts[3].decode()?,
        }),
        _ => Err(rustler::Error::BadArg),
    }
}

fn decode_actions(t: Term) -> NifResult<Vec<Action>> {
    let items: Vec<Term> = t.decode()?;
    items.into_iter().map(decode_action).collect()
}

// ------------------------------------------------------------- the calls

/// The engine's version: logs recorded under one replay the same under it.
#[rustler::nif]
fn engine_version() -> u32 {
    ENGINE_VERSION
}

/// Compile Inkjot source to IR JSON: `{:ok, json}`, or `{:error, [message]}`
/// with each problem as `inkjot check` would print it.
#[rustler::nif(schedule = "DirtyCpu")]
fn build<'a>(env: Env<'a>, source: String) -> Term<'a> {
    match inkjot::build(&source) {
        Ok(ir) => ok(env, ir.to_string().encode(env)),
        Err(report) => {
            let messages: Vec<String> = report
                .diags
                .iter()
                .filter(|d| d.severity == inkjot::Severity::Error)
                .map(|d| inkjot::render(d, "source", &source))
                .collect();
            (atoms::error(), messages).encode(env)
        }
    }
}

/// Read an IR from its JSON text.
#[rustler::nif(schedule = "DirtyCpu")]
fn load<'a>(env: Env<'a>, ir: Binary) -> Term<'a> {
    let text = match std::str::from_utf8(ir.as_slice()) {
        Ok(text) => text,
        Err(_) => {
            let r = Refused {
                kind: inkjot::engine::RefusedKind::BadIr,
                message: "the IR is not UTF-8".into(),
                at: None,
            };
            return refused_term(env, &r);
        }
    };
    match Inklink::load(text) {
        Ok(inklink) => ok(env, ResourceArc::new(InklinkResource(inklink)).encode(env)),
        Err(r) => refused_term(env, &r),
    }
}

#[rustler::nif]
fn meta<'a>(env: Env<'a>, inklink: InklinkRef) -> Term<'a> {
    let m = inklink.0.meta();
    atom_map(
        env,
        &[
            ("title", m.title.encode(env)),
            ("ifid", m.ifid.encode(env)),
            ("ir", m.ir.encode(env)),
            ("inkjot", m.inkjot.encode(env)),
            ("kind", m.kind.encode(env)),
            ("mode", m.mode.encode(env)),
            ("start", m.start.encode(env)),
            ("povs", m.povs.encode(env)),
            ("playable", m.playable.encode(env)),
            ("tags", m.tags.encode(env)),
            ("engine", m.engine.encode(env)),
        ],
    )
}

#[rustler::nif(schedule = "DirtyCpu")]
fn start<'a>(env: Env<'a>, inklink: InklinkRef, seed: u64, from: Term<'a>) -> NifResult<Term<'a>> {
    let start = decode_start(from)?;
    Ok(state_result(env, inklink.0.start(seed, &start)))
}

fn state_result<'a>(env: Env<'a>, result: Result<State, Refused>) -> Term<'a> {
    match result {
        Ok(state) => ok(env, ResourceArc::new(StateResource(state)).encode(env)),
        Err(r) => refused_term(env, &r),
    }
}

#[rustler::nif]
fn view<'a>(env: Env<'a>, state: StateRef) -> Term<'a> {
    view_term(env, state.0.view())
}

/// The engine's events from the action that made this state.
#[rustler::nif]
fn events<'a>(env: Env<'a>, state: StateRef) -> Term<'a> {
    let events: Vec<Term<'a>> = state
        .0
        .events()
        .iter()
        .map(|e| event_term(env, e))
        .collect();
    events.encode(env)
}

/// The log: every action since the start.
#[rustler::nif]
fn actions<'a>(env: Env<'a>, state: StateRef) -> Term<'a> {
    let actions: Vec<Term<'a>> = state
        .0
        .actions()
        .iter()
        .map(|a| action_term(env, a))
        .collect();
    actions.encode(env)
}

#[rustler::nif]
fn seed(state: StateRef) -> u64 {
    state.0.seed()
}

/// The last finale this reading reached, or `nil`.
#[rustler::nif]
fn ending(state: StateRef) -> Option<String> {
    state.0.ending().map(str::to_string)
}

#[rustler::nif(schedule = "DirtyCpu")]
fn act<'a>(env: Env<'a>, state: StateRef, action: Term<'a>) -> NifResult<Term<'a>> {
    let action = decode_action(action)?;
    Ok(state_result(env, state.0.act(&action)))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn take_link<'a>(env: Env<'a>, state: StateRef, id: String) -> Term<'a> {
    state_result(env, state.0.take_link(&id))
}

/// Rebuild a reading from its log: the state after the first `upto`
/// actions (`nil`: all of them).
#[rustler::nif(schedule = "DirtyCpu")]
fn replay<'a>(
    env: Env<'a>,
    inklink: InklinkRef,
    seed: u64,
    start: Term<'a>,
    actions: Term<'a>,
    upto: Option<usize>,
) -> NifResult<Term<'a>> {
    let start = decode_start(start)?;
    let actions = decode_actions(actions)?;
    Ok(state_result(
        env,
        inklink.0.replay(seed, &start, &actions, upto),
    ))
}

/// Every state of a reading: the start, then one per action.
#[rustler::nif(schedule = "DirtyCpu")]
fn replay_all<'a>(
    env: Env<'a>,
    inklink: InklinkRef,
    seed: u64,
    start: Term<'a>,
    actions: Term<'a>,
) -> NifResult<Term<'a>> {
    let start = decode_start(start)?;
    let actions = decode_actions(actions)?;
    Ok(match inklink.0.replay_all(seed, &start, &actions) {
        Ok(states) => {
            let states: Vec<Term<'a>> = states
                .into_iter()
                .map(|s| ResourceArc::new(StateResource(s)).encode(env))
                .collect();
            ok(env, states.encode(env))
        }
        Err(r) => refused_term(env, &r),
    })
}

rustler::init!("Elixir.Inkjot.Native");
