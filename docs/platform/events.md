# The event log

> Copied from the platform's handoff of 20 Nov 2026. The platform's
> Inquest fixture was cut from an unpublished work; in this public copy its
> ids, labels and text are replaced, consistently, by a made-up river
> crossing, so they differ from the fixture in the platform repository.


**The spec for the interpreter's event log**, as built and tested by the
Inklinks platform (milestones 1 to 5). Everything a Linker does in a
reading is an event, appended to a log that only grows. Everything the
platform shows, and everything an Inker learns, is a projection of that
log. The Rust interpreter's own event log should be able to hold the same
events, under the same rules, so the two can be one log.

Code: `Inklinks.Events` (the store), `Inklinks.Events.Catalog` (every
type, its origin and its keys), `Inklinks.Reading` (what a reading records),
`Inklinks.Insights` (what an Inker reads back).

## 1. The model

- A **stream** is one reading: one Linker, one Inklink, from
  `reading_started` on. (A `reader` stream type is reserved for when
  accounts arrive: `reader_claimed` ties an anonymous reader id to an
  account.)
- Streams are **append-only**. Nothing is updated or deleted. A correction
  is a new event.
- The **record is the log**. A reading's state is rebuilt from it (§5); a
  projection (Continue, Inquests left, the Inker's reports) is a query over
  it (§8).

## 2. The envelope

Every event has these fields. `data` is the only part that varies by type.

| Field | Type | Meaning |
|---|---|---|
| `position` | bigint, generated | Global order across all streams. Nobody chooses it |
| `id` | UUIDv7 | The event's own id |
| `stream_type` | `reading` \| `reader` | What the stream is |
| `stream_id` | UUIDv7 | The reading |
| `stream_version` | integer ≥ 1 | 1, 2, 3… within the stream, no gaps |
| `type` | snake_case text | What happened (§4) |
| `type_version` | integer ≥ 1 | The shape of `data` for this type. All are 1 today; a change to a type's keys is a new version, read by an upcaster |
| `origin` | `linker` \| `engine` \| `platform` | Where it came from (§4) |
| `data` | JSON object | The event's facts. Keys are strings |
| `reader_id` | UUIDv7 | Who, copied from the stream for queries |
| `inklink_id` | text | Which Inklink, copied from the stream. **Opaque: never the title.** Today the IR file's name; once Inklinks are published, a UUIDv7 assigned once and never changed. Title, kind, Inker, series and adaptations are attributes and relations of the Inklink, so a rename or a story and a game with the same title change nothing here |
| `occurred_at` | timestamptz | When it happened, by the server's clock. An inferred event uses the inferred moment |
| `recorded_at` | timestamptz | When it was written |

As JSON (the first event of the example in §7):

```json
{
  "position": 339,
  "id": "01a1f592-f11b-7c49-9ff6-d054023b2172",
  "stream": "reading",
  "stream_id": "01a1f592-f0f2-7b02-abd0-7062add88893",
  "version": 1,
  "type": "reading_started",
  "type_version": 1,
  "origin": "linker",
  "reader_id": "01a1f592-f0d7-7c25-8112-31cf07363d0e",
  "inklink_id": "inquest-fixture",
  "occurred_at": "2026-11-19T20:50:10.331050Z",
  "recorded_at": "2026-11-19T20:50:10.331050Z",
  "data": {
    "seed": 42,
    "ir": "3",
    "ir_sha256": "e1a61d7b…",
    "engine": "Inklinks.Engine.Stub",
    "inquests": 3
  }
}
```

## 3. Guarantees

These hold in the platform's Postgres table, and the interpreter's log
should keep them:

1. **It only grows.** No update, no delete, no truncate. In Postgres a row
   trigger refuses `UPDATE` and `DELETE` and a statement trigger refuses
   `TRUNCATE`, for the table's owner too; the app's role should hold only
   `SELECT` and `INSERT`.
2. **Optimistic concurrency.** A writer appends after the version it last
   read. If the stream has moved on, the append fails with the stream's
   real version and writes nothing (even for an empty batch). In Postgres:
   `UNIQUE (stream_id, stream_version)`.
3. **All or nothing.** A batch of events is one transaction.
4. **Total order.** `position` orders everything; `stream_version` orders a
   stream. Projections catch up by position.
5. **Every event is in the catalogue** (§4), with exactly its keys, and
   stamped with its origin. Anything else is a bug, refused before writing.

## 4. The catalogue

Where an event comes from. The test is **could you delete it and get it
back by replaying?**

- **`linker`**: the Linker's own action: a choice, an answer, a look. No:
  only the Linker knows what they meant to do.
- **`engine`**: what the engine computed from the story, the seed and the
  choices so far: the Nib shown, an ending, the end of the reading. Yes,
  exactly; a property test checks it (§6).
- **`platform`**: what the platform decided by its own rules, or inferred
  from the clock: asking for a rating, turning an Inquest down, a fork
  declined, a reading abandoned. No: it changes if the rules change.

The column is `origin`, not `actor`: in cohort play, "actor" would read as
the character who acted.

The table below is generated from `Inklinks.Events.Catalog.markdown/0`; a
test fails if they differ.

| Type | Origin | `data` (`?`: optional) | When |
|---|---|---|---|
| `reading_started` | linker | `seed`, `ir`, `ir_sha256`, `engine`, `inquests` | A Linker begins a reading. |
| `returned_for_another_ending` | platform | `readings_finished`, `endings_seen` | The reading is by a Linker who has finished this Inklink before. |
| `nib_shown` | engine | `nib`, `passed`, `links`, `visit` | A Nib is put in front of the Linker. `visit` > 1 is a re-read. |
| `link_taken` | linker | `link`, `nib`, `label` | The Linker takes an offered Link. |
| `ending_reached` | engine | `ending` | A `finale` Nib is shown. |
| `reading_finished` | engine | `ending` | A Nib with no Links is shown: nothing left to choose. |
| `inquest_opened` | linker | `nib`, `remaining` | The Linker presses Inquest; the box opens. |
| `inquest_submitted` | linker | `nib`, `text` | They say what they would change here. |
| `inquest_skipped` | linker | `nib` | They skip the box (Skip, Escape, or nothing written). |
| `inquest_turned_down` | platform | `nib`, `reason` | Nothing to look at: noted, nothing spent. `reason`: `no_forks` or `none_left`. |
| `inquest_mode_entered` | platform | `nib`, `remaining` | Forks are hidden here, or asked for from here: Inquest mode. |
| `fork_looked` | linker | `nib`, `link`, `fork`, `label`, `at`?, `back_to`? | A fork is shown, the first time in this Inquest mode (one per fork in an opened group). |
| `fork_taken` | linker | `nib`, `link`, `fork`, `label`, `at`?, `back_to`? | The Linker takes a fork they were shown. The only event that spends an Inquest. |
| `fork_declined` | platform | `nib`, `link`, `fork`, `by`, `at`?, `back_to`? | A fork shown, left untaken when Inquest mode ends. `by`: `left`, `wanted_instead`, `took_link`, `took_another_fork`. |
| `wanted_instead_opened` | linker | `nib`, `link`, `at`?, `back_to`? | In Inquest mode, they choose an offered Link with no fork. |
| `wanted_instead_submitted` | linker | `nib`, `link`, `text`, `at`?, `back_to`? | They say what they wanted instead. |
| `wanted_instead_skipped` | linker | `nib`, `link`, `at`?, `back_to`? | They skip that. |
| `inquest_mode_left` | linker | `nib`, `reason` | Inquest mode ends without a fork taken. `reason`: `left`, `wanted_instead`, `took_link`. |
| `rating_asked` | platform | `ending`, `path` | A finale, not yet asked this reading, and not rated before by this Linker. |
| `change_wished` | linker | `nib`, `back_to`, `chose`, `text` | Under More…, a moment they would change (Add another, or Done). |
| `nothing_to_change` | linker | `ending` | They opened More… and left “I wouldn’t change a thing” selected. |
| `rating_given` | linker | `ending`, `ending_score`, `inklink_score`, `path_score`? | They answer the box. Scores 1 to 5, or null. |
| `rating_dismissed` | linker | `ending` | Not now, Escape, or an empty answer. |
| `reading_abandoned` | platform | `nib`, `idle_seconds` | Inferred: unfinished and idle. `occurred_at` is when they stopped. |
| `reading_resumed` | platform | `nib`, `after_seconds` | They act again in a reading inferred abandoned; written before their action. |

#### What a step writes

One Linker action is one append, in this order:

- **A Link:** `link_taken` (plus `fork_declined`s and `inquest_mode_left`
  first, if they were in Inquest mode), then `nib_shown`, then
  `ending_reached`, `rating_asked` and `reading_finished` as they apply.
- **A fork:** `fork_taken`, then `fork_declined` for the other forks
  shown, then `nib_shown` and the rest. Taking a fork moves on, which ends
  Inquest mode; no `inquest_mode_left` follows.
- **Back after a day away:** `reading_resumed` comes first, before the
  action.

#### Inquests

- **Inquests left is never stored.** It is `reading_started.inquests` less
  the stream's `fork_taken` count. Rewinds and going back give none back:
  they are the Linker's, not the story's.
- **Forks asked for from a later Nib** carry `at` (the earlier passage) and
  `back_to` (the `stream_version` of that visit's `nib_shown`). The record
  then reads: asked at `ch3`, back to `ch2`, took `swim_across` instead
  of `pay_the_ferryman`.
- **Where flow D stands** (box open, Inquest mode, asking what they
  wanted) is projected from these events since the last `nib_shown`, so a
  reload or a second tab lands in the same place.

#### Ratings, stopping, returning

- `rating_asked` is written with the finale's `nib_shown`, once per
  reading, and never for an ending this Linker has rated before (in any
  reading). A dismissed box does not stop the next reading asking.
- `reading_abandoned` is inferred by a sweep: no `reading_finished`, and
  nothing for a day (configurable). Its `occurred_at` is the last event's
  time, when the Linker actually stopped; `recorded_at` says when it was
  inferred. It is an inference, never a verdict: `reading_resumed` undoes it.

## 5. Replaying a reading

The engine is deterministic, so a reading's state is rebuilt from three
kinds of event, and nothing else changes it:

| Event | Replay |
|---|---|
| `reading_started` | `start(story, seed)` |
| `link_taken` | `take_link(story, state, link)` |
| `fork_taken` | `take_fork(story, state_at(back_to) or state, fork)` |

`state_at(v)` is the state when the `nib_shown` at version `v` was written:
going back starts from an earlier Nib. Everything else in the stream
(Inquests, ratings, inferences) is the platform's own projection.

Two things a replay must be able to trust, which the interpreter has to
guarantee: **the same story, seed and actions give the same states**, RNG
and shuffles included; and **the story is the one the reading began on**,
which `reading_started.ir_sha256` names. (The platform records it but does
not yet enforce it: it plays the current file.)

## 6. Invariants under test

Properties (StreamData) and scenarios that hold today:

- The log only ever grows, and no stored event changes, whatever mix of
  appends, stale appends, malformed batches and SQL
  `UPDATE`/`DELETE`/`TRUNCATE` is tried.
- A stream's versions run 1, 2, 3… with no gaps; a stale writer is refused
  and changes nothing. Of writers racing for one version on real
  connections, exactly one wins.
- A Linker can never take a Link the IR does not offer: every `link_taken`
  names a Link listed in the `nib_shown` just before it.
- An Inquest is spent exactly when a fork is taken: Inquests left falls by
  one on each `fork_taken` and at no other time, whatever sequence of
  actions (forged ones included, at the current Nib or earlier ones).
- Every `fork_taken` follows a `fork_looked` of that fork since the last
  `nib_shown`, and no fork ever appears in a `nib_shown`'s `links`.
- A reading rebuilt from its log is where the live one is: the same view,
  version, Inquest state, rating state and abandonment.
- Replaying a reading with the engine alone, from its seed and its
  `link_taken` and `fork_taken` events (going back included), writes
  exactly its `engine` events, in order. This is the determinism check to
  run against the Rust engine.

## 7. Example

A Linker reads the Inquest fixture: asks an Inquest at ch2, opens the
group (two forks), takes one, goes on to ch4, and rates it. The
real log, trimmed to `version`, `type`, `origin`, `occurred_at` and
`data`, one event per line:

```json
{"version": 1, "type": "reading_started", "origin": "linker", "occurred_at": "2026-11-19T20:50:10.331050Z", "data": {"engine": "Inklinks.Engine.Stub", "inquests": 3, "ir": "3", "ir_sha256": "e1a61d7b38d5…", "seed": 42}}
{"version": 2, "type": "nib_shown", "origin": "engine", "occurred_at": "2026-11-19T20:50:10.331050Z", "data": {"links": ["pay_the_ferryman", "wait_for_dawn"], "nib": "ch2", "passed": [], "visit": 1}}
{"version": 3, "type": "inquest_opened", "origin": "linker", "occurred_at": "2026-11-19T20:50:10.350234Z", "data": {"nib": "ch2", "remaining": 3}}
{"version": 4, "type": "inquest_submitted", "origin": "linker", "occurred_at": "2026-11-19T20:50:10.354400Z", "data": {"nib": "ch2", "text": "Don't pay him. Swim."}}
{"version": 5, "type": "inquest_mode_entered", "origin": "platform", "occurred_at": "2026-11-19T20:50:10.354400Z", "data": {"nib": "ch2", "remaining": 3}}
{"version": 6, "type": "fork_looked", "origin": "linker", "occurred_at": "2026-11-19T20:50:10.358334Z", "data": {"fork": "swim_across", "label": "Swim across in the dark", "link": "pay_the_ferryman", "nib": "ch2"}}
{"version": 7, "type": "fork_looked", "origin": "linker", "occurred_at": "2026-11-19T20:50:10.358334Z", "data": {"fork": "steal_the_boat", "label": "Steal the boat", "link": "pay_the_ferryman", "nib": "ch2"}}
{"version": 8, "type": "fork_taken", "origin": "linker", "occurred_at": "2026-11-19T20:50:10.362246Z", "data": {"fork": "swim_across", "label": "Swim across in the dark", "link": "pay_the_ferryman", "nib": "ch2"}}
{"version": 9, "type": "fork_declined", "origin": "platform", "occurred_at": "2026-11-19T20:50:10.362246Z", "data": {"by": "took_another_fork", "fork": "steal_the_boat", "link": "pay_the_ferryman", "nib": "ch2"}}
{"version": 10, "type": "nib_shown", "origin": "engine", "occurred_at": "2026-11-19T20:50:10.362246Z", "data": {"links": ["on_to_the_hills"], "nib": "ch2_alt", "passed": [], "visit": 1}}
{"version": 11, "type": "link_taken", "origin": "linker", "occurred_at": "2026-11-19T20:50:10.366350Z", "data": {"label": "Go on to the hills", "link": "on_to_the_hills", "nib": "ch2_alt"}}
{"version": 12, "type": "nib_shown", "origin": "engine", "occurred_at": "2026-11-19T20:50:10.366350Z", "data": {"links": ["keep_the_map"], "nib": "ch3", "passed": [], "visit": 1}}
{"version": 13, "type": "link_taken", "origin": "linker", "occurred_at": "2026-11-19T20:50:10.374957Z", "data": {"label": "Promise to burn the map", "link": "keep_the_map", "nib": "ch3"}}
{"version": 14, "type": "nib_shown", "origin": "engine", "occurred_at": "2026-11-19T20:50:10.374957Z", "data": {"links": [], "nib": "ch4", "passed": [], "visit": 1}}
{"version": 15, "type": "ending_reached", "origin": "engine", "occurred_at": "2026-11-19T20:50:10.374957Z", "data": {"ending": "ch4"}}
{"version": 16, "type": "rating_asked", "origin": "platform", "occurred_at": "2026-11-19T20:50:10.374957Z", "data": {"ending": "ch4", "path": true}}
{"version": 17, "type": "reading_finished", "origin": "engine", "occurred_at": "2026-11-19T20:50:10.374957Z", "data": {"ending": "ch4"}}
{"version": 18, "type": "rating_given", "origin": "linker", "occurred_at": "2026-11-19T20:50:10.378633Z", "data": {"ending": "ch4", "ending_score": 5, "inklink_score": 4, "path_score": 5}}
```

## 8. Reading the log

`Inklinks.Insights` answers the brief's questions with plain SQL over this
table, and its tests run them against real readings. That is the check that
the log holds enough.

| Question | From |
|---|---|
| Where readers stop | `reading_abandoned.nib` |
| Time per Nib | `nib_shown` to the next `linker` event in the stream (median) |
| Re-reads | `nib_shown.visit > 1`; readers with several `reading_started` |
| What happened to forks | `fork_looked`, `fork_taken`, `fork_declined` by `fork` |
| What readers asked for | `inquest_submitted.text` (and the event after it: turned down or Inquest mode), `wanted_instead_submitted.text`, `change_wished.text` |
| How endings land | `rating_given` by `ending` |
| Returns | `returned_for_another_ending`; readers with more than one distinct `ending_reached` |

## 9. For the interpreter's event log

What the Rust side needs to decide or build so the two logs are one:

1. **Use this envelope** (§2) and these guarantees (§3), with `origin`
   on every event. The engine's own events (below) are `engine`.
2. **The interpreter's events go in the same stream.** It already logs
   snapshots, checkpoints, rewinds, echoes and turn ends (run.md, "Time").
   Those become events in the reading's stream, `origin: engine`, alongside
   these: `checkpoint_taken`, `rewound` (with `to`), `turn_ended`, `died`,
   and in cohort play `move_locked`, `claim_settled`, `clashed`. They are
   not in this catalogue because the stub never produces them; the
   catalogue is where they would be added, with their keys.
3. **Replay needs either old states or a replay call.** Going back
   (§5) needs the state at an earlier Nib. See engine-calls.md: a
   `replay(story, seed, actions, upto)` call, or one state handle kept per
   Nib shown.
4. **Determinism across engine versions.** `reading_started` should record
   the engine's version, and a reading should replay on the engine it began
   on, or the engine must promise that a version never changes what a seed
   and a list of actions produce.
5. **`type_version` and upcasters.** When a type's keys change, bump its
   version and read old events through an upcaster, never by rewriting
   them (the log refuses that anyway).
6. **Ids are UUIDv7** (time-ordered), for events and readings alike.
7. **Cohort play** (not in this slice) adds a second axis: one world, many
   Linkers. Each Linker keeps their own reading stream for what they did,
   and the shared world gets its own stream (`stream_type: world`) for what
   the engine resolved for everyone. That needs deciding with the bridge.
   **Who holds a role** (the Storyteller, say) is the platform's to record,
   in the cohort's stream (`storyteller_chosen {user, by: volunteer | vote
   | random | inker}`); the IR says what the role may see and do, and never
   names who holds it.
8. **Open question: state changes.** Should the log record what a step
   changed (a character's trust, a resource)? Recommendation: not one event
   per effect (redundant with replay, many per game turn, and tied to
   variable names that change between revisions), but a compact diff on
   the step's own engine event, only for values the Inker marks as worth
   tracking:

   ```json
   {"type": "nib_shown", "origin": "engine", "data": {"nib": "ch3", "changes": {"leo.trust": [2, 3]}, "...": "…"}}
   ```

   Anything else an Inker asks about state is computed by replaying into
   a projection. How the Inker marks a value (`~ track`?) is Inkjot's to
   decide.
