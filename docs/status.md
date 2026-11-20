# Status

Where the compiler stands, and what comes next. Update this file in the same
commit as the work it describes, so an interrupted session can pick up from
here.

## Done

| Milestone | What | Pull request |
|---|---|---|
| 1 | `inkjot check`: the container, statements, Links, bodies, names of passages and declared ids | #1 |
| — | Property tests with proptest; three bugs found and fixed | #2 |
| 2a | Names inside expressions resolved; the expression printer and its round-trip property | #3 |
| 2b | `inkjot build` and the JSON IR, version 1 | #4 |
| 3 | IR version 2: declarations and settings as typed fields | #5, landed by #6 |
| 4 | An interpreter over the IR; `[test]` passages; `inkjot test` and `inkjot play` | #7 |
| 5 | `get` beside `let`; `on` handlers; NPC agendas | #8 |
| 6 | Several Linkers at once: seats, claims and clashes, simultaneous moves; more property tests | #9 |
| 7 | Checkpoints, rewinds and echoes; comments, `~ choice`, no colon without a value, kind filters | #10 |
| 8 | The jury (`{accuse}`), `{name input}`, `{choose input}`; the finale | #11 |
| 9 | `{choice input}` dropped; `alone_with` as specified; `survives death` | #12 |
| 10 | Text takes double quotes; `'` as a thousands mark; `{ballots}`; the dead drop what they carried | #13 |
| 11 | Hints, first built under the name Inquests | #14 |
| 12 | Inquests renamed to hints; `~ inquest` reserved for true Inquests; hints can be turned off | #15 |
| 13 | Sight, sound and sleep: NPCs see moves (`self.saw`), noise wakes sleepers, a Nib's own `moves:`, echoes see NPCs where they were; `kill pov.all` and `kill each in` | #16 |
| 14 | v0.5 closed: `requires`, `cost … from each`, `once` and `once per turn \| day` on acts; IR version 2 frozen; release v0.5.0 | #17 |
| — | Ex 5 *Thistledown* and the rummy sketch drafted for v0.6; Ex 4 withdrawn | #18 to #22 |
| 15 | The engine API (`src/engine.rs`, `docs/engine.md`): states as values, replay from a log, errors as data, a step limit, `ENGINE_VERSION`; ordered maps throughout; code dual-licensed MIT OR Apache-2.0, examples CC BY 4.0 | #23 |
| 16 | Engine events: `nib_shown`, `ending_reached`, `reading_finished`, and the interpreter's checkpoints, deaths, rewinds, echoes and turns; `replay_all`; `[finale]` is an ending (Ex 2 and Ex 3 retagged); the platform's handoff in `docs/platform/` | #24 |

## Next

The plan agreed on 20 Nov 2026, after the platform's first slice:

1. **Milestone 15, the engine API.** Done.
2. **Milestone 16, engine events.** Done.
3. **Milestone 17, the Rustler NIF.** A workspace of two crates: `inkjot`
   (no dependencies) and `inkjot_nif` (Rustler). States as `ResourceArc`s;
   `load` and `replay` on dirty schedulers. See `docs/engine.md`.
4. **Milestone 18, IR 3 and the first part of v0.6.** IR 3: unknown fields
   ignored; an `inquest` field on forks (`stands_in_for`, `from`, `credit`,
   `pending`), retiring `kind: "inquest"`; stable ids; styled text runs and
   labels as parts; `state` with its type; passage titles (`title: "…"`).
   Then `finale` (many, reached by a Link or the clock) and `epilogue`;
   `shown_as` until `reveal`; `aka` policies (`shuffled`, `cycle`, `random`,
   `first`); emphasis.
5. **Ex 6 to 9**, ideas first, then drafts:
   - Ex 6, a spy-noir `story-game` that leans on Inklog: two knowledge
     bases per character, and integrity constraints for facts that rule
     each other out.
   - Ex 7, Mafia, the card game.
   - Ex 8, a `story` written for Inquests, in place of Ex 4.
   - Ex 9, a modern Zork-like `game`.
6. **The rest of v0.6**: Inklog (stratified negation; the world and what a
   character knows), `some`/`every`/`no`, `on turn begins`/`on turn ends`,
   `where:`, `you` reserved, analysis tags, dice, `[if …; then …]`, decks
   and piles, and perhaps `{expr}` in all text.

Smaller things noted on the way:

- `inkjot check` could warn about a `next:` cycle. The engine's step limit
  stops one at run time.
- **The story map.** The platform will want the whole story (passages,
  Links, forks) as a graph: a path tree a Linker may look at after a
  reading, and later path analytics for the Inker. Either an engine call or
  a documented part of the IR. Decided (20 Nov): an engine call
  (`map()`), so it can add edges the IR only implies (`next:`, acts that
  move, the ways of a map) without the platform learning the IR's
  insides. Built with IR 3, when forks exist.
- **Whether a step reports what it changed** (`platform/events.md` §9.8):
  waits for a way to mark tracked values in v0.6.

The platform's handoff of 20 Nov 2026 is in `docs/platform/`. Of its
follow-ups (`platform/compiler-followups.md`): 1 (`finale` and `epilogue`)
was settled in milestone 16; 8 and 9 (the engine and its events) in 15 and
16; 4 (the stub as an oracle) belongs to milestone 17; 2, 3, 5, 7 and 10
(emphasis, `shown_as`, Inquest forks, "offered Links", `aka` policies) to
milestone 18.

**Where a new example lives.** Works meant for the Inklinks Library are
kept out of this repository. When a new Ex N is drafted, ask Mark where it
goes: here, the platform's repository, or the private Library, with a
recommendation (a strong test case leans public; a promising work leans
Library; a public rewrite can serve both).

## How to resume

```
cargo test                                   # everything should pass
for ex in examples/*.twee; do                # the stories' own tests
  inkjot test "$ex" "examples/tests/$(basename "$ex" .twee).tests.twee"
done
PROPTEST_CASES=5000 cargo test --test properties
UPDATE_IR=1 cargo test --test build          # after an intended IR change
```

Work happens on a branch per milestone, pushed early and often, with a pull
request into `main`.
