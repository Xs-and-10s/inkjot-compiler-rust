# The engine

`inkjot::engine` is the interpreter as a program that plays Inklinks calls
it: the Inklinks platform, through the NIF (`Inkjot.Engine`, below), or a
test.
It sits on top of `inkjot::run` and adds four promises, each under a
property test in `tests/engine.rs`.

| Promise | What it means | Property |
|---|---|---|
| **States are values** | Nothing changes a `State`. Acting clones the run, plays on the clone and returns it. Keep any state you have seen; branch from it. | `states_are_values` |
| **A reading is its log** | A state is decided by the IR, the seed, the `Start` and the `Action`s, which it carries. `replay` rebuilds it, or any state before it. | `replay_equals_playing_step_by_step` |
| **Determinism within an engine version** | The same log reads the same way under the same `ENGINE_VERSION`. | `the_same_log_reads_the_same_way` |
| **Replay reports the same events** | Replaying a log with the engine alone reports exactly the reading's engine events, in order: the platform's own check against its log. | `replay_reports_exactly_the_readings_events` |
| **Errors are data** | Every call returns `Result<_, Refused>`. A refused call changes nothing. Panics are caught. | `what_is_not_offered_is_refused`, `a_panic_is_refused_not_raised` |

Every state also carries the **events** the action that made it caused,
for the platform's log (below).

This is a solo engine. Cohort play and Inquest forks come in later
milestones (see the end).

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
| `Inklink::load(json)` | `Inklink` | Parses the IR text. Unknown fields are ignored. Reads IR `"3"`. |
| `Inklink::from_ir(&value)` | `Inklink` | The same, from an IR already parsed (`inkjot::build`'s result). |
| `inklink.meta()` | `&Meta` | No reading needed. |
| `inklink.start(seed, &start)` | `State` | `Start { nib, pov, hints }`; the default is the story's own start and pov, hints on. |
| `state.view()` | `&View` | Worked out once, when the state was made. |
| `state.take_link(id)` | `State` | Only a Link in `view.links`. |
| `state.answer(words)` | `State` | Only while `view.asks` has an input that is not the jury. |
| `state.act(&action)` | `State` | Any `Action`; the three above are shorthands. |
| `inklink.replay(seed, &start, &actions, upto)` | `State` | The state after the first `upto` actions, or all. A refused action has its index in `Refused.at`. |
| `inklink.replay_all(seed, &start, &actions)` | `Vec<State>` | Every state on the way: the start, then one per action. For going back, and for checking a log's engine events. |
| `state.events()` | `&[Event]` | What the action that made this state caused (below). |
| `state.ending()` | `Option<&str>` | The last finale this reading reached. |
| `state.actions()`, `state.seed()`, `state.started()` | the log | What to persist. |

An `Inklink` is cheap to clone (two `Arc`s); every state made from it shares
one indexed story.

## Types

**`Meta`**: `title`, `ifid`, `ir` (`"3"`), `inkjot` (the language version the source declared),
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
| `text` | The paragraphs the last action (or the start) showed, in order: `Para`s, each a list of `Span { text, style }` with `style` from `italic`, `bold`, `struck`. `Para::plain()` reads one as text. |
| `links` | The Links offered: `{ id, label }`, the label a `Para`. |
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

## Events

The platform keeps one log per reading
([`platform/events.md`](platform/events.md)). Some events are the Linker's
(`link_taken`), some the platform's (`rating_asked`), and some the
engine's: whatever follows from the story, the seed and the actions, so a
replay reports it again, exactly. `state.events()` holds those, for the
action that made the state. The platform wraps each in its envelope (ids,
stream, version, times); `event.to_json()` gives the part the engine
knows: `{"type", "type_version": 1, "origin": "engine", "data"}`.

The catalogue (`engine::CATALOGUE`; a test fails if this table differs):

| Type | `data` | When |
|---|---|---|
| `nib_shown` | `nib`, `passed`, `links`, `visit` | After every action, and at the start: the Nib in front of the Linker, the passages the action went through on the way, the ids of the Links offered, and how many times the Nib has been arrived at. |
| `ending_reached` | `ending` | The action arrived at a `[finale]` Nib. Its name is the ending's id. |
| `reading_finished` | `ending` | Nothing is offered and nothing asked. `ending` is the last finale reached, or null. |
| `checkpoint_reached` | `checkpoint` | A passage says `checkpoint name`: the world is saved there. |
| `died` | `who`, `at` | A character died during the action, at that place. |
| `rewound` | `who`, `stop`, `checkpoint`, `echo`, `cause` | Someone went back: to the start of `stop`, or to `checkpoint` (else null); as an echo re-living it, or with the world going back; `cause` is `rewind` (a Link) or `death`. |
| `echo_ended` | `who` | An echo reached the present again. |
| `turn_ended` | `stop` | The clock's stop `stop` ended. |

**Order.** An action's events come in the order things happened: what the
interpreter met while playing it (a death, the fall back that caught it, a
checkpoint, a turn's end), then `nib_shown`, then `ending_reached`, then
`reading_finished`. The platform puts the Linker's `link_taken` before
them, and its own `rating_asked` between `ending_reached` and
`reading_finished`.

**`nib_shown`** comes once per action, answers included: a new view is a
Nib put in front of the Linker again. `passed` lists the passages the
action entered before the one shown (an act, a `next:` hop, a checkpoint's
fall back), oldest first. `visit` counts arrivals: it does not go up when
an act shows its result and leaves the pov where it was.

**Finales and endings.** A `[finale]` passage is an ending, and its name is
the ending's id. An Inklink may have many, reached by Links (Ex 2, Ex 3) or
by the clock (Ex 1's `terminus`). `[epilogue]` is a coda after one, and a
passage may be both (Ex 3's `the_list`). `ending_reached` is reported on
arriving at a finale; `reading_finished` names the last finale reached,
which the ratings box is about.

Epilogues are ordinary passages, joined by Links: each finale may lead to
its own, several may share one, and a finale may lead to a run of them
(how the hero's story ends, then that the villain lived, then a hint of
the next book). The ending stays the last `[finale]` passed, so tag only
the true ending `[finale]`, and the epilogues after it `[epilogue]`.

**Not yet.** Whether a step reports what it changed (events.md §9.8) waits
for a way to mark values worth tracking (`~ track`?) in v0.6. Cohort play
will add locks, claims and clashes, in a stream of the shared world.

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

`take_link` clones the run and plays on the clone. The run keeps a
snapshot of the world at the start of every turn (for rewinds and echoes);
the snapshots sit behind `Arc`s, so a clone copies pointers to them, not
the snapshots. Measured through the NIF (release, a small cloud machine),
on the examples:

| Call | Ex 1 (109 KB IR) | Ex 2 (239 KB) | Ex 3 (71 KB) |
|---|---|---|---|
| `load` | 2.6 ms | 5.1 ms | 1.3 ms |
| `start` | 0.25 ms | 0.17 ms | 0.04 ms |
| `take_link`, 30 Links in | 0.22 ms | 0.20 ms | |
| `take_link`, 400 Links in | 0.51 ms | 0.25 ms | |
| `view` | 13 µs | 3 µs | 2 µs |
| `replay` of 30 actions | 8.2 ms | 6.7 ms | 0.4 ms (8) |

Before the snapshots were shared, a Link 300 in to Ex 1 took 25 ms. What
still grows is what NPCs have seen (Ex 1's sightings, one entry per person
per turn), copied into each new snapshot. Sharing it the same way is the
next step if a long game needs it.

## The NIF

`Inkjot.Engine` (in `elixir/`, with `mix.exs` at the root of this
repository) wraps `native/inkjot_nif`, a Rustler crate. The `inkjot` crate
itself stays dependency-free: Rustler is only in `inkjot_nif`.

```elixir
# The platform's mix.exs
{:inkjot, github: "Xs-and-10s/inkjot-compiler-rust", branch: "main"}  # or a tag, once there is one
```

It needs Elixir 1.15 or later and a Rust toolchain (1.91 or later) where it
is compiled: Rustler builds the crate in release mode with `mix compile`.

```elixir
{:ok, inklink} = Inkjot.Engine.load(File.read!("ex3.json"))
{:ok, state} = Inkjot.Engine.start(inklink, 42, %{})
%{nib: nib, links: links, text: text} = Inkjot.Engine.view(state)
# text: [%{plain: "…", runs: [%{text: "…", style: ["italic"]}]}]
# links: [%{id: "…", label: "plain label", runs: [%{text, style}]}]
{:ok, next} = Inkjot.Engine.take_link(state, hd(links).id)
[%{"type" => "nib_shown", "origin" => "engine", "data" => data} | _] = Inkjot.Engine.events(next)
{:ok, again} = Inkjot.Engine.replay(inklink, 42, %{}, Inkjot.Engine.actions(next))
```

| Call | Returns | Scheduler |
|---|---|---|
| `build(source)` | `{:ok, ir_json}` or `{:error, [message]}` | dirty CPU |
| `load(ir_json)` | `{:ok, inklink}` | dirty CPU |
| `meta(inklink)` | a map with atom keys | normal |
| `start(inklink, seed, start \\ %{})` | `{:ok, state}` | dirty CPU |
| `view(state)` | a map with atom keys | normal |
| `events(state)` | a list of maps with string keys, as the platform's log stores them | normal |
| `actions(state)`, `seed(state)`, `ending(state)` | the log | normal |
| `take_link(state, id)`, `act(state, action)` | `{:ok, state}` | dirty CPU |
| `replay(inklink, seed, start, actions, upto \\ nil)` | `{:ok, state}` | dirty CPU |
| `replay_all(inklink, seed, start, actions)` | `{:ok, [state]}` | dirty CPU |
| `engine_version()` | `1` | normal |

- **Handles.** An `inklink` and a `state` are resources: references the
  Elixir side holds, never copies. The BEAM frees them when nothing holds
  them. Keep one state per Nib shown, if going back needs it.
- **Terms.** `start` is `%{nib: _, pov: _, hints: _}`, every key optional.
  An action is `{:take_link, id}`, `{:answer, [word]}` or
  `{:vote, who, how, [proof]}`. A refusal is
  `{:error, {code, message, at}}`. A term of the wrong shape raises
  `ArgumentError`.
- **Schedulers.** Every call that plays the story or reads a whole file runs
  on a dirty CPU scheduler. A Link takes 0.2 to 0.6 ms (above): under the
  1 ms a NIF may hold a normal scheduler, but too close, and it grows with
  a long game. A dirty call costs a few microseconds more. The calls that
  only read a state take microseconds and run on normal schedulers.
- **Stack.** The step limit keeps the engine's stack small in a release
  build, so a `next:` loop is refused on a scheduler thread too.
  `Inkjot.Native` always builds in release mode.
- **Tests** (`mix test`, `elixir/test/`): StreamData properties over
  random readings of Ex 1 to 3 (replay gives the same views and events;
  acting never changes a state; a Link not offered is refused; every event
  is in the catalogue), a `next:` loop refused with the VM alive, and a
  Link under a millisecond deep into Ex 1.

The platform's stub engine can serve as an oracle: run its Ex 3 scenarios
against `Inkjot.Engine`, and compare the two engines' views over random
readings.

## Not yet

- **Cohort play**: seats, shared acts and stops, through the engine.
- **Inquests** (IR 3, milestone 18): `asked_from`, `forks`, `take_fork`.
- **The story map**: the whole story (passages, Links, forks) as a graph,
  for the platform's path tree and the Inker's path analytics: an engine
  call, `map()`, built with IR 3.
- **`snapshot`/`restore`**, once readings get long enough to need them.
- **Precompiled NIFs** (`rustler_precompiled`), so the platform's build
  needs no Rust toolchain.
