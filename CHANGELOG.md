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
