# Changelog

## Unreleased

### Engine

- Pacing reaches the view: each paragraph carries `pause` (`[after 2s]`,
  `[beat 2s]`, a wait after the paragraph before) and `verse`, and a run
  appended after a wait carries its own; the same through the NIF. Times
  may be decimal (`1.5s`) (#36).

### Ex 8

- *The Inquest*, drafted (`examples/drafts/ex8-the-inquest.twee`): a
  short `story` for Inquests. `tests/ex8.rs` runs it on today's engine,
  with a property that every reading reaches one verdict.

### Ex 6

- *Cloakroom*, the public cut of Ex 6: one evening, three names, the full
  draft's machinery (`examples/drafts/ex6-cloakroom.twee`). The full Ex 6
  lives in the Inklinks Library.
- `src/period.rs`: periods' arithmetic (names to turns and back, `~ of`,
  clocks written in period names), with property tests.
- `tests/drafts.rs`: a lint for draft examples, with mutation properties;
  `INKJOT_DRAFT=… cargo test --test drafts -- --ignored` lints any draft.

### Drafts

- Ex 2's duel written four ways, to test the `define` proposal: in general
  words, and with each of its three steps (`examples/drafts/ex2-duel-*`).
  The findings, in `docs/define.md`: the general words win; `define` waits
  for an Inklink with a spellbook. Link groups and `~ at` on Links join the
  encounter milestone's plan.
- Ex 2: the Host's humming in the Gallery teaches the rhyme's frost line,
  which nothing taught before, so "You know his rhyme, every line" can be
  shown.

### Inkjot v0.6, milestone 18d

- Costs name their resource: `default cost: vults` in the config, and a
  cost in the IR is `{"amount": …}`. Without a `default cost` nothing is
  spent, and `inkjot check` says so; a `default cost` that is not a
  declared resource is an error. `inkjot migrate` adds `default cost:
  vults` where a v0.5 source declares `resource vults` (#29).
- `requires full x` reads the resource it names (#29).
- `hushed` is no longer built in; Ex 2 declares `state hushed: no` (#29).
- `rewinds`, `hints` and `inquests` are documented as the reserved,
  player-owned resource names (#29).
- `docs/define.md`: a proposal for Inker-defined words, not decided (#29).

### Inkjot v0.6, milestone 18c

- Inquests: a fork is a Link with `~ inquest` (`for id`, `from passage`,
  `by role ~ rejoin passage ~ brief name`, `~ credit role`), lowered to an
  `inquest` field; `kind: "inquest"` is retired. The checker refuses forks
  in a cohort, a fork with no offered Link to stand in for, and a pending
  fork with nowhere to rejoin; it notes Links without ids beside forks (#28).
- `theme` and `brief` declarations; analysis tags on Links (`beat`,
  `weighs`, `arc`, `stakes`, `plants`, `pays off`, `predict`, `lasting`,
  `tone`, `reveals`), checked, in an `analysis` object (#28).
- `~ track` on a value: `nib_shown` reports its `changes` (#28).
- The engine: `forks`, `any_forks`, `take_fork`, `asked_from`, `map()`,
  `Meta.inquests`, `Action::TakeFork`; the same through the NIF (#28).

### Inkjot v0.6, milestone 18b

- Emphasis (`*italic*`, `**bold**`, `***both***`) is lowered to styled runs
  in prose, Link labels and quoted text that is read (`say`, hints, fact
  text, descriptions, list entries); labels are lists of parts (#27).
- Braces work in every quoted string, as in prose, with `\{`, `\}`, `\"`,
  `\\` and `\*` as escapes; `if … then … else …`, `plural()` and
  `capital()`; lists read "a, b and c" and big numbers take `'` marks (#27).
- Names as the reader knows them: `shown as` until `reveal x`, then the
  name; `{x.name}` and `{x.shown_as}` (#27).
- `~ pick shuffled | cycle | random | first` for `aka`, and `{all x.aka}`
  (#27).
- The engine's view carries styled paragraphs and labels; the NIF sends
  `%{plain, runs}` (#27).

### Inkjot v0.6, milestone 18a

- `format-version: 0.6.0`. `inkjot migrate` rewrites a v0.5 source, and the
  checker points a v0.5 file at it (#26).
- `state` is a mutable value (v0.5's `fact`), with a type in the IR; `fact`
  is knowledge (v0.5's `clue`); `clue` is a tag on a fact (#26).
- `i`, `you`, `myself` and `yourself` mean the pov, and `i.know.x`,
  `i.hold.key`, `i.am(rook)` read as `me.knows.x`, `me.holds.key`,
  `me.is(rook)`; the IR always says `me` (#26).
- `[fixme]` is `[fix]` (#26).
- Reserved words: the pov names, `self`, `it`, `here`, and the words of v0.6
  features still to come (#26).
- IR version 3: unknown fields are ignored; Link ids come from the Inker's
  `~ id` or from passage and target, never from position; passages may
  have a `title` (#26).

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
