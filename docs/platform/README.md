# From the platform

> Copied from the platform's handoff of 20 Nov 2026. The platform's
> Inquest fixture was cut from an unpublished work; in this public copy its
> ids, labels and text are replaced, consistently, by a made-up river
> crossing, so they differ from the fixture in the platform repository.


The first slice of the Inklinks platform is done (milestones 1 to 5, PRs
#1 to #5). It runs Ex 1 to 3 and a hand-written Inquest fixture (IR "3")
on a stub engine behind `Inklinks.Engine`. These are the four documents to
take back, in the order to read them.

| File | What it settles |
|---|---|
| [`events.md`](events.md) | **The spec for the interpreter's event log**: envelope, guarantees, all 25 event types with origin and keys, replay rules, invariants under test, a real example log |
| [`engine-calls.md`](engine-calls.md) | **What the platform asks of an engine**, counted and timed; the Rust API it wants; the bridge recommendation |
| [`ir3-requests.md`](ir3-requests.md) | **Five requests for IR 3**, and one rule (ignore unknown fields), with the shapes the fixture already uses |
| [`compiler-followups.md`](compiler-followups.md) | Ten smaller issues found on the way, each with its workaround |

## The decisions that need the compiler's answer

1. **IR 3 §0–§2**: ignore unknown fields; the `inquest` field on forks
   (with `credit` for who wrote it and `pending` for a slot filled at
   runtime, which may grow a subtree under a named `brief`); stable Link
   ids. Without §1 and §2 an Inquest-enabled Inklink cannot ship.
2. **The bridge**: a Rustler NIF, with `replay(story, seed, actions, upto)`,
   immutable states, and `take_link` (not `take`, an Inkjot verb)
   (engine-calls.md, "The bridge").
3. **One log**: the interpreter's events join the reading's stream with
   `origin: engine` (events.md §9), and whether a step records what it
   changed (§9.8).
4. **Determinism across engine versions** (events.md §5, §9.4): the
   promise, or the engine version recorded and kept playable.
5. **`finale` vs `epilogue`** (follow-up 1).
6. **Roles never name people**: `pending.by: storyteller` means whoever
   holds the role; the platform chooses who (events.md §9.7).
7. **Show what the reader knows** (`shown_as` until `reveal`), and **`aka`
   policies** declared once (follow-ups 3, 10).

## How the platform checks itself against the spec

- `Inklinks.Events.Catalog` is the event table; a test fails if
  events.md's table and the code differ, or if the code writes a type the
  catalogue lacks.
- A property replays readings with the engine alone and checks it writes
  exactly their `engine` events: the determinism check for the Rust engine.
- The compiler's eight Ex 3 tests run against the stub
  (`test/inklinks/engine/stub_ex3_test.exs`); run them against the Rust
  engine when the bridge exists, and use the stub as an oracle in a
  property test.
