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
  choices always give the same story.

## What it does not do yet

It says so, by name, if a story needs one of these. It never guesses.

- Several Linkers at once: simultaneous turns, clashes, echoes.
- NPC agendas: `goal`, `reaction`, statues that move.
- `on` handlers, including the Host's voice in Ex 2.
- Inquests and rewinds.
- The jury (`{accuse}`), and inputs (`{name input}`, `{choose input}`).
- Anything marked v0.6: the duel, `retell`.
