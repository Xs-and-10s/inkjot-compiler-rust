# Changelog

## Unreleased

### The engine

- `inkjot::engine`: the interpreter as the Inklinks platform calls it.
  `Inklink::load`, `meta`, `start`, `view`, `take_link`, `answer`, `act`
  and `replay`. States are values; a reading is its log; the same log reads
  the same way under one `ENGINE_VERSION`; every error is a `Refused`,
  panics included. See `docs/engine.md` (#23).
- A step limit: one action may enter 64 passages and evaluate 100,000
  expressions. A `next:` loop is now an error, not a stack overflow (#23).
- The interpreter keeps every map ordered, so nothing depends on a
  hasher (#23).
- An `Ask` carries its prompt (#23).

### Engine events

- Each state carries the events the action that made it caused:
  `nib_shown`, `ending_reached`, `reading_finished`, and from the
  interpreter `checkpoint_reached`, `died`, `rewound`, `echo_ended` and
  `turn_ended`. `engine::CATALOGUE` lists them with their keys (#24).
- `Inklink::replay_all` returns every state of a reading (#24).
- `[finale]` is an ending; `[epilogue]` a coda. Ex 2's endings are now
  `[finale]` (and `fallen` is not); Ex 3's `the_list` is
  `[epilogue finale]` (#24).
- `docs/platform/`: the platform's handoff (the event log, how it calls
  the engine, IR 3 requests, follow-ups) (#24).

### The NIF

- `Inkjot.Engine`: the engine inside the BEAM, as a Rustler NIF
  (`native/inkjot_nif`). `mix.exs` sits at the root and the Elixir code
  under `elixir/`, so the platform depends on this repository directly.
  States and Inklinks are resources; the calls that play the story run on
  dirty CPU schedulers; refusals are `{:error, {code, message, at}}`;
  events come back as maps with string keys. Also `build/1` (#25).
- History snapshots are shared behind `Arc`s, so a clone of a long run
  stays cheap: a Link 300 in to Ex 1 went from 25 ms to 0.5 ms (#25).
- The workspace: `inkjot` stays dependency-free; Rustler is only in
  `inkjot_nif` (#25).

### Licence

- The code is now `MIT OR Apache-2.0`. The example Inklinks are CC BY 4.0
  (`examples/LICENSE`) (#23).

## 0.5.0 (15 Nov 2026)

The first release. The compiler and interpreter cover Inkjot v0.5, as its
three example Inklinks use it, and the IR is frozen at version `"2"`.

### The compiler

- `inkjot check`: the Twee 3 container, statements, Links, bodies, the names
  of passages and declarations, and the names inside expressions, with
  positions on every diagnostic (#1, #3).
- `inkjot build`: the JSON IR, version 2, with declarations and settings as
  typed fields (#4, #5, #6). Documented in `docs/ir.md`.
- Comments `((…))` and `(((…)))`, doc comments, `~ choice`, kind filters,
  and no colon without a value (#10).
- Text takes double quotes; `'` marks thousands, as in `1'000'000` (#13).

### The interpreter

- `inkjot test` and `inkjot play`, `[test]` passages (#7).
- `let` and `get`, `on` handlers, NPC agendas (#8).
- Several Linkers at once: seats, claims and clashes, simultaneous moves (#9).
- Checkpoints, rewinds and echoes (#10).
- The jury (`{accuse}`, `{ballots}`), `{name input}`, `{choose input}`,
  the finale (#11, #12, #13). The dead drop what they carried (#13).
- Hints (#14, #15). Inquests are reserved: `~ inquest` gets a notice (#15).
- Sight, sound and sleep: NPCs see moves, noise wakes sleepers, a Nib's own
  `moves:`, echoes see NPCs where they were (#16).
- What an act asks first: `requires`, `cost … from each`, `once`, and
  `once per turn | day`, with `~ day n` on the clock (#17).

### Not yet

See "What it does not do yet" in `docs/run.md`: the v0.6 duel and `retell`,
Inquests, the rest of the input family, `span` and `quiet` on handlers, and
what belongs to the platform (talk between Linkers, the turn limit).

### Testing

Property tests with proptest throughout (#2 and after): the expression
printer round-trips, routes are shortest, every hint request buys one new
hint, cohorts play cleanly and deterministically, and random
playthroughs of all three examples fail only by naming what is not supported.
