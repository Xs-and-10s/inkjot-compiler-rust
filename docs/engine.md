# The engine

`inkjot::engine` is the interpreter as a program that plays Inklinks calls
it: the Inklinks platform, through a Rustler NIF (milestone 17), or a test.
It sits on top of `inkjot::run` and adds four promises, each under a
property test in `tests/engine.rs`.

| Promise | What it means | Property |
|---|---|---|
| **States are values** | Nothing changes a `State`. Acting clones the run, plays on the clone and returns it. Keep any state you have seen; branch from it. | `states_are_values` |
| **A reading is its log** | A state is decided by the IR, the seed, the `Start` and the `Action`s, which it carries. `replay` rebuilds it, or any state before it. | `replay_equals_playing_step_by_step` |
| **Determinism within an engine version** | The same log reads the same way under the same `ENGINE_VERSION`. | `the_same_log_reads_the_same_way` |
| **Errors are data** | Every call returns `Result<_, Refused>`. A refused call changes nothing. Panics are caught. | `what_is_not_offered_is_refused`, `a_panic_is_refused_not_raised` |

This is a solo engine. Cohort play, the event log and Inquest forks come in
later milestones (see the end).

## Calls

```rust
use inkjot::engine::{Action, Inklink, Start};

let inklink = Inklink::load(&ir_json)?;                 // once per IR
let meta = inklink.meta();                              // title, kind, povs…
let state = inklink.start(seed, &Start::default())?;    // a reading begins
let view = state.view();                                // what the Linker sees
let state = state.take_link(&view.links[0].id)?;        // a new State
let state = state.answer(&["Ash".to_string()])?;       // an open input
let again = inklink.replay(seed, &Start::default(), state.actions(), None)?;
assert_eq!(again.view(), state.view());
```

| Call | Returns | Notes |
|---|---|---|
| `Inklink::load(json)` | `Inklink` | Parses the IR text. Unknown fields are ignored. Reads IR `"2"`. |
| `Inklink::from_ir(&value)` | `Inklink` | The same, from an IR already parsed (`inkjot::build`'s result). |
| `inklink.meta()` | `&Meta` | No reading needed. |
| `inklink.start(seed, &start)` | `State` | `Start { nib, pov, hints }`; the default is the story's own start and pov, hints on. |
| `state.view()` | `&View` | Worked out once, when the state was made. |
| `state.take_link(id)` | `State` | Only a Link in `view.links`. |
| `state.answer(words)` | `State` | Only while `view.asks` has an input that is not the jury. |
| `state.act(&action)` | `State` | Any `Action`; the three above are shorthands. |
| `inklink.replay(seed, &start, &actions, upto)` | `State` | The state after the first `upto` actions, or all. A refused action has its index in `Refused.at`. |
| `state.actions()`, `state.seed()`, `state.started()` | the log | What to persist. |

An `Inklink` is cheap to clone (two `Arc`s); every state made from it shares
one indexed story.

## Types

**`Meta`**: `title`, `ifid`, `ir` (`"2"`), `inkjot` (the compiler version),
`kind` (`story`, `story-game`, `game`), `mode` (`solo`, `cohort`), `start`,
`povs` (the `pov:` setting, default first), `playable` (characters marked
`~ playable`, for a lobby), `tags` (the Inker's), `engine`.

**`Start`**: `nib` (a Nib to start at), `pov` (a character from `povs` or
`playable`), `hints` (offer hints; a Linker may turn them off).

**`Action`**: `TakeLink(id)`, `Answer(words)`, `Vote { who, how, proof }`.

**`View`**:

| Field | |
|---|---|
| `nib` | The Nib the pov is at. |
| `tags` | Its tags: `finale`, `ending`, `epilogue`, `act`. |
| `pov` | Who the Linker looks through. |
| `now` | The clock's stop. |
| `text` | The paragraphs the last action (or the start) showed, in order. Plain strings in IR 2; styled runs come with IR 3. |
| `links` | The Links offered: `{ id, label }`. |
| `asks` | The open inputs: `{ kind, prompt, options, methods, length, voted }`. |
| `over` | Nothing offered, nothing asked: the reading has ended. |

**`Refused`**: `{ kind, message, at }`. `kind.code()` gives a stable name
for an Elixir atom:

| Code | When |
|---|---|
| `bad_ir` | Not JSON, not an Inklink, or an IR version this engine does not read |
| `not_offered` | No Link with that id is in the view |
| `not_asking` | An answer or vote with nothing asking for one |
| `bad_answer` | The answer or vote does not fit what is asked |
| `step_limit` | The action did more than one action may (below) |
| `unsupported` | The story uses something the interpreter does not run yet |
| `story_error` | The story went somewhere the interpreter could not follow: a missing passage, a pov nobody may play |
| `panic` | A bug in the engine, caught. Report it with the log. |

## The log, and what to persist

For each reading, persist: `ENGINE_VERSION`, which IR (the platform's own
id or hash for it), the seed, the `Start`, and the actions. That is enough
to rebuild every state of the reading. States themselves need not be
stored; keep the latest in memory for speed.

**Versions.** Under the same `ENGINE_VERSION`, a log replays exactly. The
version goes up whenever a change could make an old log read differently:
a different roll, a different order, a Link offered or not. A log from an
older version may still replay, but nothing promises the same text, and a
replay may be refused partway (`at` says where). The platform's own event
log keeps what the Linker actually saw, so nothing is lost either way.

**Why it is deterministic.** Every map in the interpreter is a `BTreeMap`
or `BTreeSet`, so nothing depends on a hasher's order. Every roll and
shuffle comes from the seed. Nothing reads the clock or the environment.
And a view is worked out exactly once per action, the same way whether the
action was taken live or replayed, so anything that looking costs (a `when`
that rolls) is spent the same number of times.

## The step limit

One action may enter at most `run::MAX_ENTERED` (64) passages and evaluate
at most `run::MAX_STEPS` (100,000) expressions. Past either, it is refused
with `step_limit`, and the state before it is untouched. The most any
action in Ex 1 to 3 does, over thousands of random readings, is 3 passages
and about 100 expressions.

The passage limit matters most. Two passages whose `next:` lead to each
other recurse, and a stack overflow ends the process: no Rust code can
catch it, and inside a NIF it would take down the BEAM. With the limit, a
`next:` loop needs under 64 KB of stack in a release build (about 256 KB in
a debug build). `a_next_loop_hits_the_step_limit` runs it on a 512 KB
thread.

## Costs

`take_link` clones the run. The run keeps a snapshot of the world at the
start of every turn (for rewinds and echoes), so a clone costs more as a
reading goes on. That is fine for stories of the length of Ex 1 to 3;
milestone 17 measures it through the NIF, and sharing snapshots behind
`Arc`s is the fix if it is needed.

## For the NIF (milestone 17)

- A two-crate workspace: `inkjot`, which stays dependency-free, and
  `inkjot_nif`, which depends on it and on Rustler.
- `Inklink` and `State` as `ResourceArc`s; the Elixir side holds handles,
  never copies.
- `load` and `replay` on dirty CPU schedulers: they can take longer than
  a millisecond. `take_link` and `answer` usually do not, but the step limit
  is the bound, not the usual case; measure, and move them too if needed.
- Build the NIF in release mode. Dirty scheduler stacks are small (set by
  `+sssdcpu`); the step limit keeps the engine's stack use bounded, but a
  debug build uses several times more.
- `Refused` becomes `{:error, {code, message, at}}`.

## Not yet

- **Events** (milestone 16): each action returns the events it caused, in
  the shape the platform's event log needs.
- **Cohort play**: seats, shared acts and stops, through the engine.
- **Inquests** (IR 3, milestone 18): `asked_from`, `forks`, `take_fork`.
