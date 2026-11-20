# How the platform calls the engine

> Copied from the platform's handoff of 20 Nov 2026. The platform's
> Inquest fixture was cut from an unpublished work; in this public copy its
> ids, labels and text are replaced, consistently, by a made-up river
> crossing, so they differ from the fixture in the platform repository.


The bridge to the Rust interpreter is undecided: a Rustler NIF (which must
not block the BEAM's schedulers) or a port or sidecar service. WASM in the
browser is ruled out, because it would put the forks in the client. These
notes record what the platform actually asks of an engine, how often, with
what, and what it needs back, and end with a recommendation (§ "The
bridge").

The contract is `Inklinks.Engine` (`lib/inklinks/engine.ex`). Every call
goes through `Inklinks.Engine.call/2`, which wraps it in a telemetry span
(`[:inklinks, :engine, callback]`), so the counts below are measured, not
guessed. Status: final for this slice. Measured on the stub with Ex 3
(milestone 2); Inquest calls counted again on the Inquest fixture in
milestone 5, through the telemetry events.

## The callbacks

| Call | In | Out |
|---|---|---|
| `load/1` | the IR file's bytes | a story handle, or `{:unsupported, reasons}` |
| `meta/1` | story | title, kind, mode, Inquest budget |
| `start/2` | story, seed | state |
| `view/2` | story, state | the Nib's Ink by passage, its offered Links, ending, done |
| `take_link/3` | story, state, Link id | state, or `:not_offered` |
| `asked_from/2` | story, Nib | passages whose forks may be asked for from this Nib (static, from `inquest.from`) |
| `forks?/3` | story, state, Nib asked from | yes or no: may a fork be taken here, asking from that Nib? |
| `forks/4` | story, state, offered Link id, Nib asked from | the forks that stand in for it, in the Inker's order (ids and labels) |
| `take_fork/3` | story, state, fork id | state, or `:not_offered` |

`forks?/3` and `forks/4` take the Nib the Linker is asking **from**. Given
the current state, that is the Nib's own forks. Given the state of an
earlier visit, only the forks whose `from` names the current Nib.

## When the platform calls, per request

A reading's state is not kept between requests. Each request rebuilds it
from the log (`Inklinks.Reading.load/2`): `start` with the recorded seed,
then `take_link` for each `link_taken`.

| Request | Engine calls | |
|---|---|---|
| First listing of an Inklink, or after its file changes | `load` ×1 | cached in ETS by file, mtime and size |
| Start a reading | `start` ×1, `view` ×1, `meta` ×1 | |
| Open a reading (`GET /readings/:id`), *n* Links in | `start` ×1, `take_link` ×*n*, `view` ×1 | the rebuild |
| Take a Link, *n* Links in | `start` ×1, `take_link` ×*n* (rebuild), then `take_link` ×1 and `view` ×3 | the rebuild runs in `authorize/2` before the event |

Inquests add, on top of the rebuild every request pays (`start` ×1, then
`take_link` or `take_fork` ×*n*). Counted on the Inquest fixture (milestone 5), Inquest
asked at ch2:

| Step | Engine calls |
|---|---|
| Start a reading (first ever: the story is loaded) | `load` ×1, `meta` ×1, `start` ×1, `view` ×1 |
| Press Inquest | `view` ×1 |
| Answer or skip the box | `asked_from` ×1, `forks?` ×1 per place (the Nib, and each earlier visit), `view` ×1 |
| Open a Link (hover, focus or first tap), a group of two forks | `asked_from` ×1, `forks` ×1, `view` ×2: **one Link's forks cross, for one look** |
| Take a fork | `forks` ×1, `take_fork` ×1 (from the earlier visit's state when going back), `view` ×3 |
| Take a Link | `take_link` ×1, `view` ×2 |
| Rebuild after a fork and a Link | `start` ×1, `take_fork` ×1, `take_link` ×1 |

The `view`s are the platform reading the state it already holds (the Nib's
Links to check a choice, the Links to list): in the stub they cost 2 µs,
and in Rust they must stay pure reads of a stored view, never a re-render.

A look is the only call made on hover, so it is the one that must be fast:
a reader moving the mouse down a list of Links makes one request per Link.
The stub answers `forks` in microseconds.

**Going back needs old states.** A fork asked for from a later Nib is taken
from the state of an earlier visit. The platform keeps every `nib_shown`'s
state while it replays a reading (`Reading.nibs`), so the stub needs nothing
more. For the Rust engine across a bridge, that means either holding one
state handle per Nib shown (cheap as NIF resources; a memory question for a
sidecar), or a `replay(story, seed, actions, upto: version)` call that
rebuilds an earlier state in one trip. This is the strongest argument yet
for a replay or snapshot call.

So **one Link costs *n* + 1 `take_link`s**, and a whole reading of *n* Links
costs about *n*²/2. For a story that is nothing; for a game with thousands
of moves it is the first thing to fix.

## Measured (stub, Ex 3, dev machine)

| What | Time |
|---|---|
| `load` (82 KB of IR: JSON decode and checks) | 3.3 ms, once |
| `start` | 0.15 ms (5 ms on the very first call: code loading) |
| `take_link` | 0.12–0.19 ms |
| `view` | 2 µs (the stub renders on arrival and keeps the view) |
| Rebuilding a finished reading (9 Links), engine only | ≈ 1.1 ms |
| The same, with the event query, in dev with debug logging | ≈ 12 ms |
| State size (`:erlang.external_size`) | 2.4 KB |
| `forks?/3` (the Inquest fixture) | 0.7 µs |
| `forks/4`, a group of two (the Inquest fixture) | 6 µs |
| Rebuilding an Inquest-fixture reading two steps in, with the event query and the story cache, dev | ≈ 17 ms (engine under 1 ms; the rest not profiled) |

## What a Rust engine must provide

The calls above, plus two that the platform will want as soon as the
engine is real. A sketch of the crate's surface, which the bridge then
wraps (names are the interpreter's to choose):

```rust
pub struct Story(/* parsed IR, immutable, Send + Sync */);
pub struct State(/* one world: values, lists, aka, visits, rng, view */);

impl Story {
    pub fn load(ir_json: &[u8]) -> Result<Story, Vec<Unsupported>>;
    pub fn meta(&self) -> Meta;                       // title, kind, mode, inquests
    pub fn start(&self, seed: u64) -> State;
    pub fn take_link(&self, s: &State, link: &str) -> Result<State, NotOffered>;
    pub fn asked_from(&self, nib: &str) -> &[String]; // static, from inquest.from
    pub fn forks_any(&self, s: &State, from: &str) -> bool;
    pub fn forks(&self, s: &State, link: &str, from: &str) -> Vec<Fork>;
    pub fn take_fork(&self, s: &State, fork: &str) -> Result<State, NotOffered>;

    // New for the bridge:
    pub fn replay(&self, seed: u64, actions: &[Action], upto: Option<usize>)
        -> Result<Vec<State>, Replay>;                // one State per Nib shown
    pub fn snapshot(&self, s: &State) -> Vec<u8>;     // versioned, stable bytes
    pub fn restore(&self, bytes: &[u8]) -> Result<State, Snapshot>;
}

impl State {
    pub fn view(&self) -> &View;                      // stored on arrival
}

pub enum Action { Link(String), Fork { fork: String, back_to: Option<usize> } }
```

- **`take_link`, not `take`.** `take` is an Inkjot verb (an act like
  `take book`), so the call that takes a Link says so, and pairs with
  `take_fork` as `link_taken` pairs with `fork_taken`.
- **Immutable states.** `take_link` returns a new `State` and leaves the old one
  usable: going back takes a fork from an earlier state, and a stale
  request must not see a later one. Structural sharing (`im`, or `Arc`
  around the big parts) keeps that cheap.
- **`replay`** turns the per-request rebuild into one call, returns the
  per-Nib states that going back needs (§5 of events.md), and fixes the
  *n*² cost of rebuilding on every request.
- **`snapshot`/`restore`** let the platform keep a snapshot beside the log
  (every *k* events, or at each `nib_shown`) and replay only what follows.
  Only needed once readings get long (games, cohort play); a story never
  will.
- **Errors are data**: `NotOffered`, `Unsupported` with reasons the
  library can show ("Not playable yet: …"), never a panic across the
  boundary.

## The bridge: a NIF, with a sidecar kept possible

**Recommendation: Rustler NIF.** The numbers decide it:

- Every call but `load` is microseconds (the stub, in Elixir, is 0.1–0.2 ms
  per `take_link` and µs for the rest; Rust will be faster). That is far under
  the 1 ms a NIF may hold a normal scheduler, so `take_link`, `view`, `forks?`,
  `forks` and `take_fork` run as plain NIFs.
- `load` (parse and check a whole IR file, 3 ms for Ex 3), and `replay` of
  a long reading, run on **dirty CPU schedulers**
  (`#[rustler::nif(schedule = "DirtyCpu")]`).
- `Story` and `State` cross as **resources** (`ResourceArc`): a pointer,
  freed by the BEAM's GC. The story is passed on every call and costs
  nothing; holding one `State` per Nib shown for going back is just
  memory, about what the Elixir stub already holds.
- Views cross as maps built once per arrival (`NifMap`/`NifStruct`), the
  shapes in `Inklinks.Engine`'s types, so `Inklinks.Web.Ink` renders them
  unchanged.
- The `Inklinks.Engine` behaviour stays the seam: `Inklinks.Engine.Rust`
  implements it, the stub stays for tests and as an oracle (a property
  test runs both on the same seed and actions and compares views, which is
  how the Rust engine proves it plays Ex 1–4 the same).

**Why not a sidecar first.** A sidecar (a port, or a service over a socket)
buys crash isolation: a panic or a runaway loop in the interpreter cannot
take the node down. It costs a round trip per call, serialization of every
view, a story id the sidecar must resolve, and state that lives in another
process (so going back needs `replay` or held handles there). With a
rebuild being 1 + *n* calls, it is only reasonable once `replay` exists.

**The risks of a NIF, and their answers:**

| Risk | Answer |
|---|---|
| A panic kills the VM | Rustler catches panics and raises; keep `unsafe` out of the interpreter |
| An infinite loop in a story (a `next:` cycle, a runaway `[once]`) | A step limit in the interpreter, returned as an error, never a hang |
| A slow call blocks a scheduler | Dirty schedulers for `load` and `replay`; a test that times every other call under 1 ms on the largest example |
| Memory from held states | States are resources; a reading's are dropped with its request |

If isolation turns out to matter (untrusted IR uploaded by Inkers is the
likely trigger), the same crate goes behind a port with `replay` as its
main call, and the platform's seam does not change.

## What the counts say

- **Calls are small and frequent, not large and rare.** A rebuild is
  1 + *n* calls in a row with the same story handle. Over a NIF that is
  cheap (each call is microseconds, well under the 1 ms a NIF may hold a
  scheduler, so no dirty scheduler is needed for `take_link` and `view`; `load`
  parses a whole IR file and should run on a dirty CPU scheduler). Over a
  sidecar it is 1 + *n* round trips per request, which argues for one of:
  - a **`replay(story, seed, [link ids])`** call that rebuilds in one trip;
  - **`snapshot(state)` / `restore(story, bytes)`**, so the platform can
    store a snapshot beside the log and replay only what came after it;
  - keeping each open reading's state **in a process** (a GenServer per
    reading, or the sidecar holding it), so a Link is one `take_link`.
  Any of these also fixes the *n*² above.
- **Handles must be cheap to pass.** The story is passed on every call. As
  a NIF resource that is a pointer; across a sidecar it must be an id the
  sidecar resolves, never the IR again.
- **Determinism is load-bearing.** The log holds only the seed and the
  Links; everything else is recomputed. The Rust engine's RNG, shuffles
  and rendering must be stable across engine versions for a reading to
  replay, or `reading_started` must record the engine version and the
  platform must keep that version able to play it. `reading_started`
  already records `ir_sha256`, so a reading is replayed against the IR it
  began on, not a revised one (not yet enforced: the platform plays the
  current file).
- **The stub renders on arrival.** Reading `aka` rolls dice, so rendering
  twice would change the story. The Rust interpreter also renders once,
  on arrival; `view` must stay a pure read.

## Decisions so far

- **IR crosses as JSON bytes.** A Rust engine parses it with serde; nothing
  is converted term by term.
- **Handles are opaque.** The platform never looks inside a story or a
  state.
- **The log is the record, not the state.** See above.
- **Forks cross only when asked.** `view/2` returns offered Links only;
  `forks/4` returns one Link's forks, for one look. `forks?/3` says only
  yes or no. A fork's id reaches the page only in the patch that answers
  its look, and the platform refuses to take a fork the Linker has not
  been shown.
- **The platform owns Inquests.** The budget comes from `meta/1`; what is
  left is counted from the log. The engine never spends.
