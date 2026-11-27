# Back to the compiler

> Copied from the platform's handoff of 20 Nov 2026. The platform's
> Inquest fixture was cut from an unpublished work; in this public copy its
> ids, labels and text are replaced, consistently, by a made-up river
> crossing, so they differ from the fixture in the platform repository.


Issues found while building the platform, to take back to the
inkjot-compiler-rust chat as one batch. None of them blocks the slice; each
says how the platform works around it meanwhile. IR 3 field requests have
their own file, `ir3-requests.md`.

## 1. `finale` and `epilogue`

Ex 3's last passage, `the_list`, is tagged `epilogue` and not `finale`, so
nothing marks it as an ending.

Proposed meaning:

- **`finale`** is an ending: what a Linker reaches, what the ratings box is
  about, and its passage name is the ending's id. An Inklink may have many.
- **`epilogue`** is a coda after a finale: one shared, or one per finale.
- The ratings box opens when a reading reaches a passage with no Links
  left, and is about the last `finale` the Linker passed. A passage tagged
  both is the ending and the end at once, which is well defined.

Also: `run.md` says the finale is the passage *no Link leads to* (Ex 1's
clock-out `terminus`), but Ex 4's endings are `[finale]` passages that
Links do lead to. Both should be allowed: a story reaches its endings by
Links; a clocked game by the clock.

**Meanwhile:** `priv/inklinks/ex3-the-fall-of-the-old-regime.json` has
`"tags": ["epilogue", "finale"]` on `the_list`, added by hand. Fix:
`:: the_list [epilogue finale]` in the Ex 3 source, then `UPDATE_IR=1`.

## 2. Emphasis

IR 2 passes `*italic*`, `**bold**` and `***both***` through as literal
asterisks, so a runtime has to parse Inkjot to render them. See
`ir3-requests.md` for the requested shape.

**Meanwhile:** the stub engine lowers asterisks itself, following the v0.6
rules (underscores literal, an unmatched `*` literal, `\*` an asterisk,
never across a paragraph).

## 3. `shown_as`: show what the reader knows (decided 19 Nov)

Ex 3 declares `npc widow: "Marge" ~ shown as "the Surviving Authority"`.
The interpreter's speech lines use the label, so she speaks as "Marge"
and `shown_as` is never shown, which gives away what `shown_as` hides.

Proposed rule, the same in speech and inline: **render what the reader
knows.**

- `@marge` (speech) and `{marge}` (inline) render `shown_as` until a
  `reveal marge` effect, then the name.
- `{marge.name}` and `{marge.shown_as}` render one or the other, always,
  for when the Inker wants that.
- Whether she has been revealed is state, so replay reproduces it.

**Meanwhile:** the stub matches the interpreter and uses the label.

## 4. The stub matches the interpreter, so test them together

The stub copies the interpreter's xorshift64 generator and its `aka`
shuffle, so the same seed names Leo the same way in both. When the bridge
exists, run the platform's Ex 3 scenarios (`test/inklinks/engine/`)
against the Rust engine as well: they are the compiler's own Ex 3 tests,
and the stub passes them all.

## 5. Inquests: the reserved `kind: "inquest"`

IR 2 reserves `kind: "inquest"` for Links. A fork can be a move or an act,
so being a fork should be a field (`"inquest": {"stands_in_for": …}`), not
a kind. See `ir3-requests.md` §1.

## 6. Ex 4 and the fixture

Flow D has a branch for an offered Link with no fork, chosen in Inquest
mode ("what did you want instead"). Every Inquest Nib in Ex 4 has exactly
one offered Link and its fork, so that branch never comes up in the real
text. The fixture adds to ch2, marked fixture-only:

- `wait_for_dawn` (to ch3): an offered Link with no fork.
- `steal_the_boat` (to a new passage): a second fork for
  `pay_the_ferryman`, which also sets `crossed_free`.

And `swim_across` is marked `from ch3`, so it can also be asked for from
ch3, which is not in the draft.

## 7. Vocabulary: offered Links (decided 18 Nov)

The Links a Nib shows are **offered Links**; a **fork** is hidden behind
one. "Canon" stays a story word, for the Inker's main line. The platform's
event data says `link` where it said `canon`. Worth using the same words in
run.md and the Inkjot docs.

## 8. What the engine needs to grow (from engine-calls.md)

For the bridge, in order:

- **`replay(story, seed, actions, upto)`**, returning one state per Nib
  shown: the platform rebuilds a reading on every request, and going back
  needs earlier states.
- **Immutable states** (`take_link` returns a new one and the old stays
  usable).
- **`take_link`, not `take`**: `take` is an Inkjot verb, and the pair
  `take_link` / `take_fork` matches `link_taken` / `fork_taken`.
- **A step limit**: a `next:` cycle or runaway loop returns an error, never
  hangs a scheduler.
- **Determinism across versions**: the same IR, seed and actions give the
  same views, or the engine version goes into `reading_started` and old
  readings replay on the version they began on.
- **`snapshot`/`restore`**, later, for long games.

The recommendation is a Rustler NIF (calls are microseconds; `load` and
`replay` on dirty schedulers; states as resources), with the stub kept as
an oracle in a property test against the Rust engine. The platform already
has the test to point at it: replaying a reading with the engine alone
must write exactly the reading's `engine` events.

## 9. Engine events in the reading's log (from events.md)

The interpreter's own events (checkpoints, rewinds, turn ends, deaths, and
in cohort play locks, claims and clashes) should join the reading's stream
with `origin: engine`, under the envelope and guarantees in events.md §2–3.
Whether a step records the state it changed is an open question there
(§9.8; recommended: a diff on the step's event, for tracked values only).
Adding one means adding it to `Inklinks.Events.Catalog` with its keys.

## 10. `aka`: a policy declared once, a few forms at the call site (decided 19 Nov)

Today `{leo.aka}` renders one alias: the interpreter shuffles with the
seeded RNG on arrival, and the stub copies it. Proposed, so a list can be
shown whole or one item at a time without an expression language in the
prose:

- **The policy is declared once, with the data:**
  `~ aka "Leo", "Leo Leo Beans" ~ pick cycle`. Policies: `shuffle` (today's
  behaviour, the default), `cycle`, `random` (every time), `first`.
- **The call site has a few named forms:** `{leo.aka}` picks one by that
  policy; `{all leo.aka}` renders the whole list, through the list
  rendering `the_list` already has (items struck, highlighted, added).
- **Redacted items, too** (for a spy story-game): rendered as a bar, and
  the text never reaches the page, like a hidden fork.
- **No filters** (`where starts_with …`) until an example needs one.
- **Replay keeps working:** every random pick uses the seeded RNG, and a
  cycle's position lives in state.

**Meanwhile:** the stub keeps today's `shuffle`.

## 11. Ex 7's Doppelgänger, on the platform (decided 25 Nov)

The compiler's side is in the draft (milestone 21a). The platform's:

- Presence, typing indicators and read receipts follow the face a seat
  shows, not the account behind it, so a swap is not given away by who is
  online.
- Night results are shown in one batch, at the morning.
- An irreversible night choice (a kill, a replacement) asks for
  confirmation.
- The replacer never sees the victim's private messages, history or
  credentials: a swap is of fictional characters, never of people.
- An asynchronous day is settled at its deadline (or when every ballot is
  locked), not the moment someone reaches a majority; a short extension
  when the lead changes near the deadline is allowed.
- The engine's plain lines ("Gone in the night: …", "Missing in the night:
  …") are shown whoever tells the morning.


## 12. Time in real time (milestone 21c, 26 Nov)

- A Link timed `~ after 5m` comes in the view with `due_in` (milliseconds
  left) and cannot be taken (`not_offered`). Show it with a countdown.
  When the time runs out, send `{:wait, ms}` (any amount; the engine
  counts what it is told): the Link is taken by itself, for whoever it is
  offered to, once for the table if it is `~ for cohort`. A `Wait` is
  logged like any other action, so a replay takes it where the reading
  did.
- Its time runs from when it was first offered this turn and starts again
  when the turn ends. Ex 7's day (`To the vote`, five minutes) and its
  night's deadline (`Dawn`, two minutes) are such Links.
- The engine is still solo; the table's own `Wait` comes with 21e's
  seating API.
- The view's `now` is still the turn. Names of periods (`clock.phase`) are
  the story's to show, in its text.

## 13. Votes at a table (milestone 21d, 26 Nov)

- A `{ballot}` comes in `asks` with `kind: "ballot"`, `options` (the
  list's entries) and `voted`; it is answered with `Answer([option])`.
  A decided ballot or jury is no longer asked.
- The jury at a table: `who:` may be a list of seats, `how:` may be
  missing (then the ask's `methods` is empty, and a vote's `how` is `""`),
  and only `voters:` may vote; others are refused (`bad_answer`). Ties may
  be drawn by lot.
- Consensus Links (`~ consensus`): taking one is a vote; its effects
  happen when the turn ends. Night results therefore land together, at
  the morning, as §11 asks.
- A `~ for cohort` Link moves the whole table; the platform shows each seat
  its own view of the passage. The engine is still solo (21e).

## 14. A table through the engine (milestone 21e, 27 Nov)

- `start(inklink, seed, %{seats: List.duplicate("guest", n)})` seats a
  table; `meta.cohort` (`{5, 12}`) says how many it may seat, and a table
  outside it is refused (`story_error`). Seats are `guest_1` … in the
  order given.
- Every action is `{:as, seat, action}`, except `{:wait, ms}`, the table's.
  `view(state, seat)` is each Linker's view; `view(state)` the actor's.
  Send each Linker their own view after every action.
- The engine's events are the actor's; other seats' moves show in their
  views. Persist the log as before: it holds the `{:as, …}` tuples.
- Ex 7 needs a lobby that seats 5 to 12, a name input per seat, and (for
  the Storyteller) a seat for `storyteller`.
