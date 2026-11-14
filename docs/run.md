# Running a story

`inkjot play` and `inkjot test` run a story through a small interpreter over
the IR (`src/run/`). It is the first piece of an Inklinks runtime: it plays one
Linker at a time, Link by Link.

```
inkjot play examples/ex3-the-fall-of-the-old-regime.twee
inkjot test examples/ex2-the-ninefold-cell.twee examples/tests/ex2-the-ninefold-cell.tests.twee
```

Both read every file given as one story, as Twine does with a story split
across files. Errors name the file and line they really came from.

## Writing tests

A test is a passage tagged `[test]`. Keep tests in their own file beside the
story, so the story file stays exactly what a Linker plays:

```
:: the_chin_comes_back_in_the_last_scene [test]
start: arrival
do: "“Nothing. They’re nice. One of them scratched my chin.”"
expect: the_list.has(skittles)
expect text: "Scratched. Your. Chin."
do: "Run"
do: "The cat flap bangs"
do: "Go and meet the Round One"
do: "Press the big button"
do: "The coup begins"
expect text: "I told you it was good."
```

| Statement | Meaning |
|---|---|
| `seed: n` | Seed the dice and shuffles. First line only; the default is 1 |
| `start: passage` | Begin at a passage. Add `~ as who` to say whose eyes |
| `as: who` | Look through someone else from here on |
| `given: effect` | Change the state directly: `given: learn two_hands` |
| `do: "label"` | Take the available Link with this label |
| `do: hall#2` | Take a Link by its id |
| `expect: expression` | The expression holds now |
| `expect text: "…"` | The text shown since the last `do` contains this |
| `expect no text: "…"` | …does not contain this |
| `expect link: "…"` | A Link with this label is on offer |
| `expect no link: "…"` | …is not on offer |
| `expect at: passage` | The pov is at this passage |

`inkjot check` checks tests like any other passage, so a misspelled label's
passage, an undeclared name or a malformed effect is caught before anything
runs.

A failed `do` ends its test, because the rest of the script no longer
describes what is on screen; the failure lists the Links that were on offer.
A failed `expect` is reported and the test goes on, so one run shows every
broken expectation. The story's own `expect:` checks run too, and fail the
test that triggers them.

`[test]` passages are left out of the IR that `inkjot build` publishes.

## How the interpreter reads a story

These are the rules it follows where the spec leaves room. Each was settled by
running the three examples.

- **Arriving** at a passage runs its header (`let`, effects, `expect`,
  `next:`), shows its body, and offers its Links.
- **`let` and `get`.** A `let` is worked out on arrival and then fixed for
  that visit: `let before: me.vults` remembers what you had. A `get` is worked
  out each time it is read: `get aligned: orrery.ember == orrery.tide` is true
  the moment the planets line up, even later in the same passage. A `get` that
  reads another `get` reads it fresh too. A `let` that reads a `get` takes its
  value at that moment. `set` pins either one to a value; every `get` that
  reads it then sees the new value. `let stirs: roll d6` rolls once;
  `get die: roll d6` rolls on every read. `inkjot check` rejects a `get` that
  depends on itself.
- **A passage's `let`s run again** whenever its Links are worked out again:
  on returning to it after an act, and each time it is embedded.
- **A sub-Nib sees its parents' `let`s**: `orrery:crank` sees those of
  `orrery`, as `inkjot check` already assumes.
- **An `[act]` passage shows its result, then returns** the pov to where they
  were, with that passage's Links on offer again. If the act moved them
  (`move me to ossuary`), they arrive at the new place instead.
- **A `next:`** plays the passage and carries straight on to the next one.
- **Walking**: wherever a way leads from the pov's place and is open, a
  "Go to …" Link is offered at the `walk` act's cost. In a world of cells
  (`dining@1`), the walk arrives at the destination's cell for that time.
- **A cell is a time**: arriving at `dining@2` makes it stop 2.
- **A resource is each character's own** unless it is tagged `shared`.
- **A fact declared in a passage** belongs to that passage's place, or to the
  character or NPC whose margin it is: `orrery.ember`, `vell.cornered`.
- **In reach**: a `take` Link is offered while the item lies in this place, or
  is held by an NPC who is here (that is how Wren lifts the letter).
- **Costs**: the last cost entry whose condition holds applies; `rest` takes
  everything left. A Link the pov cannot afford is not offered.
- **`[lib everywhere]`** passages add their Links to every passage.
- **Randomness** comes from the run's seed, so the same seed and the same
  choices always give the same story. A bare `d6` or `d20` rolls a die each
  time it is read, as in `table standoff: roll d6 + bonus`.

## `on` handlers

The interpreter fires these events. Every handler for an event runs, in the
order written. Inside one, `actor` is who did it and `self` is the character
or NPC whose margin holds the handler (so `say self "…"` is the Host).

| Event | When | `what` |
|---|---|---|
| `start` | the run begins | — |
| `enter`, `leave` | anyone (a pov or an NPC) changes place | the place |
| `act` | a pov arrives at an `[act]` passage | the passage |
| `cross` | a pov walks along a way | the way |
| `take` | a pov takes an item | the item |
| `learn` | a pov learns a clue they did not know | the clue |
| `raise`, `lower`, `set` | a value changes | the value, `cleverness` or `glyph.broken` |
| `turn` | a turn ends | — |
| `death` | anyone dies | — (`actor` is who died) |
| `loud` | a pov does a `loud` act, or takes a Link marked `~ loud` | the act |
| `out` | a fuse burns down | the fuse |

A `before` handler (`on before cross glyph_door`) runs first and may cancel
a walk or a take with `~ then stop`. `once` and `at most N` count the times a
handler actually ran. Handlers that set each other off more than eight deep
are stopped with an error, since that is almost always a loop.

## NPC agendas

At the end of each turn, after the `turn` handlers, every NPC with an agenda
(its own `[npc]` margin, and the `[archetype]` passage of what it is `like`)
takes one move. Dead, detained and `fix`ed NPCs sit it out.

1. Its margin `let`s are worked out, with `self` as the NPC and `here` as its
   place.
2. **It picks a goal**: the heaviest whose `when` holds and whose `toward`
   names somewhere. A goal toward nobody (no one holds the evidence) is passed
   over. Equal weights go to the one written first.
3. **It plans a route**: the shortest along the ways it may use. NPCs know the
   hidden ways but must respect `one way` and `open when`; `via wide` keeps to
   ways tagged `wide`. Toward a person means toward where they are.
4. **Reactions come first.** The first reaction whose `when` holds runs its
   `then`, and that is the NPC's whole turn. `self.heading` is the next place
   on its route, and `self.leaving` means it is heading for an `offstage`
   place.
5. **Otherwise it walks**: one place a turn, two at `pace dash`, or what its
   `moves N per chronon` says (`moves none`: never). Each step fires `leave`
   and `enter`. On arriving it runs the goal's `then`.

The words agendas lean on:

- `self.watched`: the pov is in the NPC's place, or something there watches,
  as a mirror does: `archetype mirror: ~ watches here when self.open`. In solo
  play, the character you are not looking through is unwatched.
- `nearest(living)`: the nearest living thing the NPC can reach (by its
  goal's `via`), other than itself and its own kind.
- `living`: characters and NPCs not dead and not tagged `lifeless`. A tag on
  an archetype counts for everything like it: `archetype statue: ~ lifeless`.
- `alone_with(a, b)`: the two share a place with no other living thing.
- `x.last_seen(holding: evidence)`: a character holding a thing with that tag,
  preferring one in `x`'s place. Sightings are not tracked yet, so this is who
  holds it now.
- `x.offstage`: in a place marked `offstage`, or `offstage until` a passage
  not yet seen.
- `roll standoff`: roll a table and run every row the result falls in.
- `feign sleep`: `x.asleep` holds while the NPC pretends.

## Several Linkers at once

In a world of stops (where `walk` ends the turn, as in Ex 1) several
Linkers can play together, one per character. `start: lounge@0 ~ as ada,
rook` in a test, or `inkjot play --as ada,rook story.twee` at one keyboard.
Within a stop each acts in any order:

- **Private acts** (`examine`, `ask`) happen at once. Only the actor sees
  the result.
- **Shared acts with a capacity** (`take`, `force`) are *claims*. A claim is
  settled as soon as no one else in the place could still reach for the same
  thing (everyone else there has claimed it too, or locked in their move).
  If more reach than there is room for, they *clash*: `attempt.clashed`
  holds for all of them, and the Link's `clash` rule decides who succeeds.
  `random_winner` (the default) picks as many winners as there is room;
  `random` flips a coin for each; `all` and `none` mean what they say.
  `{claimants}` names the others who reached. Others in the place see the
  winner take it.
- **Moves end the turn**: `Go to`, `Dash to` and `Slip to` (as far as the
  act's `steps` and the character's `moves` allow), `Stay`, `Observe`, or
  any act marked `ends turn` (which happens first, then the Linker stays).
  A Linker who has moved waits; their Links are gone until the stop
  resolves.

When everyone has moved, the stop resolves in the order the paper playtest
used:

1. Open claims are settled.
2. Everyone moves at the same moment. Who ends the stop where sees who came
   and went: a walk is seen leaving and arriving; a loud move (a dash) also
   by anyone in a place it passes through; a quiet one (a slip) only where
   it arrives.
3. The turn ends: `turn` handlers, the clock (which stops at its last stop),
   refills, fuses, then the NPCs' agendas. NPCs then note who is in their
   place and what they hold; `x.last_seen(holding: tag)` reads these
   sightings, so Vell hunts whoever she last saw holding evidence.
4. Each Linker arrives at their cell for the new stop.

A run with one Linker follows the same rules; its claims settle at once,
since no one else could reach.

## Time: checkpoints and rewinds

The interpreter keeps a log, as the paper Conductor did: a snapshot of the
whole world at the start of every turn, and one for each checkpoint (taken
at the start, and again whenever a passage says `checkpoint name`).

**A death** is settled after the Link that caused it:

1. If a checkpoint covers the place (`checkpoint cells ~ on death in
   prison`), the world falls back. What it `resets` comes from the
   checkpoint's snapshot; what it `keeps` stays, and keeps win over resets.
   The dead come back, resources that `refill on checkpoint` refill, and
   everyone goes where `to` says.
2. Otherwise, a Linker with a charge left (`me.rewinds > 0`) is pulled back:
   the charge is spent and they re-live the stop they died in, as an echo.
3. Otherwise they stay dead.

**A Link to `rewind`** spends a charge and offers where to go back to:

- `rewind: undo` (Ex 2): the start of any turn in the log, or any checkpoint
  reached. Everything goes back except what the rewind `keeps` and what the
  player owns rather than the characters (charges and Inquests). Handlers of
  whoever the rewind is `unseen by` do not hear the `rewind` event. The
  dice go back too, so the same rolls come again.
- `rewind: echo` (Ex 1): back as many stops as `back 1..=2` allows. Only
  this Linker goes back; everyone else stays in the present.

**An echo** re-lives an earlier stop in their old cell: `me.past` holds and
`clock.now` is that stop. They may look, ask and learn, but take nothing,
reach for nothing, change nothing and spend nothing: only `learn`, `say` and
`play` happen. Each move takes them one stop forward, and the move that
reaches the present brings them back. The others wait for them, since they
have not moved yet this stop.

What a checkpoint resets, or a rewind keeps, is a list of categories:

| Category | Means |
|---|---|
| `world` | every fact, everyone's included (`eldric.chained`); NPCs' places; items lying anywhere; lists. Not resources |
| `held` | which character holds which item |
| `knows` | what everyone knows |
| a fact, stat, resource or relation (`trust`, `hp`) | its value, for everyone |
| a place or map (`sanctum`, `prison`) | the facts of the places in it, and the items lying there |
| a character or NPC (`hask`) | every fact about them |

`here.passed` holds, and `{passers}` names them, when someone dashed
through this place on the way to the stop it is.

## What it does not do yet

It says so, by name, if a story needs one of these. It never guesses.

- Several Linkers in a world without stops (Ex 2 and Ex 3 are solo).
- Talking between Linkers, the `turn limit` timeout, and a Nib's own
  `moves:` (Ex 1's mail car allows only walking).
- Sleepers woken by a loud move (`~ wakes on loud`).
- NPCs seeing acts: `self.saw(dash)` never holds yet, so Vell never gives
  chase.
- `span` and `quiet` on handlers (reserved).
- Inquests.
- An echo sees the NPCs where they are now, not where they were then.
- `once per day` and `needs two to hold it` on a rewind spell.
- The jury (`{accuse}`), and inputs (`{name input}`, `{choose input}`).
- Anything marked v0.6: the duel, `retell`.
