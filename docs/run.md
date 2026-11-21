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
| `start: passage ~ as a, b` | Begin with several Linkers, one playing each |
| `answer: "Mara"` | Answer the first open input with text |
| `answer: gust, spark, hush` | …or with the things chosen |
| `vote: vell ~ how device ~ proof cufflink` | Cast a ballot for the jury, citing proof |
| `expect events: ending_reached, died` | The engine reported these since the last action (`docs/engine.md`, Events) |
| `expect fork: "Suppose …" ~ for "Say …"` | An Inquest here would offer this fork, standing in for that Link |
| `take fork: "Suppose …"` | Take a fork, as an Inquest would. The Inquest itself is the platform's to spend |

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
- **A state declared in a passage** belongs to that passage's place, or to
  the character or NPC whose margin it is: `orrery.ember`, `vell.cornered`.
- **In reach**: a `take` Link is offered while the item lies in this place, or
  is held by an NPC who is here (that is how Wren lifts the letter).
- **Costs** spend the resource the config names, `default cost: vults`;
  without one, acts cost nothing (`inkjot check` says so). The last cost
  entry whose condition holds applies; `rest` takes everything left. A Link
  the pov cannot afford is not offered.
- **Reserved resource names.** Three resources belong to the player, not
  the world, and the engine reads them by name: `rewinds` (charges to go
  back), `hints` and `inquests`. A rewind does not restore them. Every
  other name is the Inker's.
- **`[lib everywhere]`** passages add their Links to every passage.
- **Randomness** comes from the run's seed, so the same seed and the same
  choices always give the same story. A bare `d6` or `d20` rolls a die each
  time it is read, as in `table standoff: roll d6 + bonus`.

## Text

Prose, Link labels and every quoted string read the same way. Braces hold
an insert or an expression: `{present}`, `{coins}`, `"Keeper of {here}"`.
`\{` and `\}` are literal braces, `\"` a quote inside quoted text, `\\` a
backslash, and `\*` an asterisk that is not emphasis.

- **When it is worked out.** Prose and Link labels, each time they are
  shown. A `label`, `shown as`, a fact's text or a hint, each time it is
  shown. An effect's text (`set title: "Keeper of {here}"`, `say`), once,
  when the effect runs.
- **How values read.** An id reads as its name (below); a list as "a, b and
  c"; a number of a thousand or more with `'` marks (`1'000`); nothing as
  empty.
- **Inline choice:** `{if lied then "a liar" else "an honest girl"}`.
  Quotes nest inside braces.
- **Helpers:** `plural(n, "card")` is "card" when `n` is 1, else "cards"
  (`plural(n, "mouse", "mice")` for an irregular one); `capital(x)`
  capitalizes the first letter.
- **Emphasis** (`*italic*`, `**bold**`, `***both***`; a run of four or
  more asterisks is plain text) is kept as styles, in
  prose, Link labels, and quoted text that is read: a `say` line, a hint, a
  fact's text, a description, a list entry. A name (`label`, `shown as`) and
  a value (`set motto: "…"`) keep their asterisks. The terminal and `expect text:` see plain text; struck
  list entries read `~~like this~~`.

**Names, as the reader knows them.** `{marge}` and `@marge` show what she is
`shown as` until `reveal marge`, then her name. `{marge.name}` is always her
name and `{marge.shown_as}` always what she is shown as. Ex 3's widow is
"the Surviving Authority" to the cats from start to end.

**Pacing.** The interpreter does not wait; it says where to. `[after 2s]`
and `[beat 2s]` put a `pause` on the next paragraph shown: a wait after the
paragraph before it, so two `[after 2s]` in a row show their paragraphs two
and four seconds in. With `append` the wait falls inside the paragraph,
on the run appended (Ex 1: "He winds it once, twice— *(two seconds)* The
lights die."). `after` simply waits; `beat` is a dramatic pause, which a
runtime that reveals on tap may let the reader skip; a bare `[beat]` is a
beat of the runtime's own length (`ms: 0`). Times are `2s`, `1.5s` or
`400ms`. A pause waits for the next paragraph that is shown: across a
modifier line with nothing under it (`[after 2s]` then `[quote …]`), but
not past a block that is not shown, which takes its pause with it. `[verse]`
marks its paragraphs as lines of verse. The runtime does the waiting, lets
a reader skip ahead, and shows everything at once for a reader who asks for
reduced motion.

**Names at the start of a sentence** are capitalised: "{foreman} stands"
reads "The foreman stands", and so does a speech cue, `@foreman`. A
sentence starts at the start of a paragraph, or after `.`, `!`, `?` or `:`
and a space, with any opening quote marks between. `~ keep case` on a name
opts out (`npc poet: "ee cummings" ~ keep case`). A list's entries are
shown as written, being entries, not sentences.

**Speech cues** are styled by the config: `cue names: capital, bold` and
`cue directions: italic`, each a list of `capital`, `upper`, `bold` and
`italic`. Names are capitalised whatever the list says. A direction is
shown, in its parentheses: `@fenwick (to you)` reads "Mr. Fenwick (to
you): …".

**`narration:`** says whose words the engine writes for the pov: `@you`
(and `@me`, `@i`, or the pov's own name in a cue) and `{me}`. `narration:
second person` reads "You: …" and "you"; `first person` reads "I: …", and
`{me}` as "I" at the start of a sentence and "me" elsewhere; `third person`,
or nothing said, reads the pov's name. Prose is written by hand in whatever
person it is. A passage may say its own (`narration: first person` in its
header, for a witness's testimony), and its sub-Nibs (`martha:tom_out`) are
told as it is. `~ omniscient` may follow, for analysis; play ignores it.

**`[if c; then e]`** shows its block when c holds, and does e when the
block is shown; `[unless c; then e]` is its mirror. Effects in a passage
happen top to bottom as it is shown, so one block may unlock the next:

```
[if not rang; then set rang: yes]
The bell rings.
[if rang; then raise counted 1]
Someone counts.
```

Nothing is done while the interpreter only works out which Links are on
offer.

**Blocks.** An `[if]` chain is flat: another `[if]` in the same chain starts
it over, and `[else]` is the chain's. `[quote …]` and `[redacted until …]`
open blocks. `[end]` closes whatever opened last, the chain or the block,
so an `[if]` inside a quote ends before the quote does, and a quote inside
an `[if]` before the `[if]` does (Ex 8's struck letter). Nothing inside a
chain or block that is not shown is shown. A block left open is an error.

**Blockquotes:** `[quote letter ~ by "E. Garrard"]` … `[end]`, of kind
`sign`, `letter`, `poster` or `edict`. Each paragraph inside carries its
quote (`kind`, `by`, and `first` on the one that begins it), so a runtime
draws the letter or the edict as one block.

**Redaction**, three ways:

- `[redacted until record_closed]` … `[end]`: the paragraphs inside, while
  `record_closed` does not hold. A speaker's name is kept; what they said
  is not.
- `{redacted "TEN" until telegram.dry}`: a few words in a line.
- `npc organist: "the Organist" ~ redacted until telegram.dry`: the name,
  wherever it is shown (`{organist}`, `@organist`, a list entry).
  `{organist.name}` is still the name: asking for it is asking.

Redacted text never reaches the page. In its place is a run of `█`, one
for each character hidden, styled `redacted`, which a runtime draws as a
bar. Only the length is given away. When the condition holds, the text is
shown as written.

**`aka`.** `{leo.aka}` picks one of his names by his `~ pick` policy:
`shuffled` (the default: each name once, in a seeded order, then again),
`cycle` (in the order written), `random` (any, each time) or `first`.
`{all leo.aka}` shows every name.

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
| `learn` | a pov learns a fact they did not know | the fact |
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
   `then`, and that is the NPC's whole turn. `self.heading` is the place its
   goal leads to (while there is a way there), and `self.leaving` means that
   place is `offstage`.
5. **Otherwise it walks**: one place a turn, two at `pace dash`, or what its
   `moves N per chronon` says (`moves none`: never). Each step fires `leave`
   and `enter`. On arriving it runs the goal's `then`.

The words agendas lean on:

- `self.watched`: the pov is in the NPC's place, or something there watches,
  as a mirror does: `archetype mirror: ~ watches here when self.open`. In solo
  play, the character you are not looking through is unwatched.
- `nearest(living)`: the nearest living thing the NPC can reach (by its
  goal's `via`), other than itself and its own kind.
- `person`: any character or NPC, living or not. Built in, like `linker`
  and `living`; an Inklink does not declare it.
- `living`: characters and NPCs not dead and not tagged `lifeless`. A tag on
  an archetype counts for everything like it: `archetype statue: ~ lifeless`.
- `alone_with(a, b)`: the two share a place, and no other Linker or awake
  witness is there. Sleepers and NPCs who are not witnesses do not count.
- `x.last_seen(character, holding: evidence)`: whoever of that kind `x`
  last saw holding a thing with that tag, from the sightings NPCs take at the
  end of each turn.
- `x.offstage`: in a place marked `offstage`, or `offstage until` a passage
  not yet seen.
- `roll standoff`: roll a table and run every row the result falls in.
- `feign sleep`: `x.asleep` holds while the NPC pretends. A pretender still
  sees everything.
- `self.saw(dash, toward: self.heading)`: this turn, the NPC saw a dash that
  reached or passed that place, or ended nearer to it. See below.
- `dash toward p`, in a reaction: the NPC runs up to two places toward `p`,
  loudly. Vell's `chase` races a Linker to where she is going.
- `kill pov.all` kills every pov; `kill each in oubliette` everyone there.

## What an act asks first

An act's own clauses are checked before its Link is offered:

- `requires lockpick`: the actor holds that item, or one tagged so.
  `requires pov.all here`: every pov is alive and in the actor's place.
  `requires pov.all alive`: every pov is alive, wherever they are.
  `requires full vults`: the actor has as much of that resource as it
  holds when full.
- `cost 30 from each`: every pov pays, so every pov must be able to.
- `once`: each character may do it once a reading. `once per turn` (or
  `per chronon`, or the clock's own word): once a turn. `once per day`:
  once a day, where `clock: … ~ day 6` makes six turns a day; `inkjot
  check` refuses `once per day` on a clock without days.

The uses of an act that rewinds belong to the player, like its charges: going
back does not untie the knot that took you there. Other uses belong to the
world, so a rewind takes them back.

## Sight, sound and sleep

**Seeing a move.** Moves are simultaneous, so an NPC sees a Linker's move
from where it stood when the move was made, before any NPC moves:

| Move | Seen by NPCs in |
|---|---|
| a walk | the place left and the place reached |
| a loud move (a dash, or along a loud way) | every place left, passed and reached |
| a quiet move (a slip) | the place reached, and only by an awake `witness` |

A sleeper sees nothing; a pretender (`feign sleep`) sees it all.

**Noise.** A loud move is heard everywhere it went. A loud act (`force`, or
a Link marked `~ loud`) is heard in its place and the places next to it,
once its own passage has been shown. An NPC's dash, or its step along a
loud way (Vell leaving through the vestibule), is loud too.

**Sleep.** `asleep during 2..=4` is the NPC's hours. Noise wakes anyone who
`wakes on loud`, and the Linkers there read that they wake. Woken at one
stop, they stay awake through `stays awake n` more, then sleep again if it
is still their hour: the Colonel, woken at stop 2, is up at stop 3 and
asleep at stop 4. Without `stays awake`, and after a `wake` effect, they
stay up for good.

**A Nib's own `moves:`** cuts down how a Linker may leave that cell: `moves:
walk` in the mail car leaves Wren only her walk. The moves on offer are the
character's (or the `default moves:`) that the Nib also allows.

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
  act's `steps`, the character's `moves` and the Nib's `moves:` allow),
  `Stay`, `Observe`, or
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

## Inputs and the jury

An input asks the Linker for an answer rather than a pick. In a passage's
**body** it stays open while they are there: they may answer it, and answer
again, between Links (Ex 1's lobby: the Guest types a name, then boards). In a
**header** it comes before what follows, so arriving pauses there, with no
Links, until it is answered (Ex 2's mouth hears three cantrips, then decides).

| Input | Answer | Kept as |
|---|---|---|
| `{name input for: me.name, max: 24}` | text, not empty, at most `max` letters | text; `{me.name}` shows it, and a `custom` character's name is empty until then |
| `{choose input for: words, from: cantrip, length: 3}` | that many of the things of that kind, in order | a list: `words == [gust, spark, hush]` |

The rest of the family (`pick`, `rank`, `number`, `secret`, …) is checked
but not run yet.

**The jury** (`{accuse who: suspect, how: scenario.methods, proof:
proof_who and proof_how, revote: 1}`) is a secret ballot. Every Linker votes,
the dead too, as ghosts: a suspect (anything tagged as `who:` says), a
method (one of `how:`), and any proof they can cite, which is only what they
hold or know. When all have voted, the ballots are shown to everyone
together. The pairing with the most votes wins; a tie is a hung jury, which
votes again as often as `revote:` allows and then stays hung. Then:

| Lookup | Holds when |
|---|---|
| `accusation.done` | the jury has decided, or stayed hung |
| `accusation.hung` | no pairing won |
| `accusation.correct` | the winner is the scenario's culprit and method |
| `accusation.proven` | it is correct, and the proof cited by those who voted for it covers `proof:` (a tag needs something cited that has it; `and`, `or` and `not` combine) |
| `accusation.who`, `accusation.how` | the winning pairing |

**`{ballots}`** shows the last count, most votes first, ties in seating
order: "2 for Miss Edith Vell, by device; 1 for Mr. Crane, by poison". An
Inklink that shows `{ballots}` anywhere frames the tally itself, and the
interpreter then adds nothing but the prompt to vote again after a tie. One
that does not gets plain lines: the ballots, and whom the jury accuses.

**The dead hold nothing.** A Linker who dies for good drops what they carried
where they fell, for anyone to find, unless the item `survives death`.
Holding something teaches you it (`me.knows.cufflink`), so a ghost can still
cite it to the jury.

`scenario` is the scenario in play: the only one, or, with several, one the
dice pick at the start, so the culprit can change from run to run.
`cohort.size` and `cohort.alive` count the Linkers.

**The finale.** When the clock runs out, the last stop offers `Stay`, and when
everyone has moved, they all go to the `[finale]` passage no Link leads to
(Ex 1's `terminus`, not its `verdict`).

A `[finale]` is an ending, and its name is the ending's id. An Inklink may
have many, reached by Links (Ex 2's three, Ex 1's `verdict`) or by the clock
(Ex 1's `terminus`). An `[epilogue]` is a coda after one: each finale may lead
to its own, or to a run of them, and the ending stays the finale. A
passage may be both (Ex 3's `the_list`).

## Time: checkpoints and rewinds

The interpreter keeps a log, as the paper Conductor did: a snapshot of the
whole world at the start of every turn, and one for each checkpoint (taken
at the start, and again whenever a passage says `checkpoint name`).

A state marked `survives death` (`state deaths: 0 ~ survives death`) is
never reset by a checkpoint.

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
  player owns rather than the characters (charges and hints). Handlers of
  whoever the rewind is `unseen by` do not hear the `rewind` event. The
  dice go back too, so the same rolls come again.
- `rewind: echo` (Ex 1): back as many stops as `back 1..=2` allows. Only
  this Linker goes back; everyone else stays in the present.

**An echo** re-lives an earlier stop in their old cell: `me.past` holds and
`clock.now` is that stop. They may look, ask and learn, but take nothing,
reach for nothing, change nothing and spend nothing: only `learn`, `say` and
`play` happen. Each move takes them one stop forward, and the move that
reaches the present brings them back. The others wait for them, since they
have not moved yet this stop. An echo sees the NPCs where they stood at
that stop: Vell is in the dining car again, though she has since moved on.

What a checkpoint resets, or a rewind keeps, is a list of categories:

| Category | Means |
|---|---|
| `world` | every state, everyone's included (`eldric.chained`); NPCs' places; items lying anywhere; lists. Not resources |
| `held` | which character holds which item |
| `knows` | what everyone knows |
| a state, stat, resource or relation (`trust`, `hp`) | its value, for everyone |
| a place or map (`sanctum`, `prison`) | the states of the places in it, and the items lying there |
| a character or NPC (`hask`) | every state about them |

`here.passed` holds, and `{passers}` names them, when someone dashed
through this place on the way to the stop it is.

## Hints

A hint is help the player asks for. A Nib lists its hints in its header,
vaguest first; a Link with `~ hint` spends one hint request and tells the
next hint the pov has not heard:

```
resource hints: 3 ~ shared

:: oubliette [room]
hint: "The grate is too heavy for anything you can do from down here." ~ wits ..=2
hint: "Look past the grate. On the far door above, something only needs to slide." ~ wits 3..
--

:: party_bar [lib everywhere]
> [[Ask for a hint->here]] ~ hint ~ when hints > 0
```

- **Which hints.** Those of the Nib the pov is on, then of the Nibs it
  belongs to (`ossuary` for `ossuary:bolt`), then of its place. A hint
  applies when each stat it names is in its range for the pov (`~ wits 3..`)
  and its `~ when` holds. So Roland and Eldric hear different hints in the
  same room, and a list of hints is a ladder: one rung per request.
- **Never for nothing.** The Link is offered only when a request is left
  and a hint here applies that has not been heard. An echo spends nothing,
  so it cannot ask.
- **Off, if the Linker wants.** A Linker may turn hints off for an Inklink
  (`inkjot play --no-hints`); then no hint Link is offered.
- **The player's.** Requests and the hints they bought belong to the player,
  not the characters: a rewind or a checkpoint gives neither back. In solo
  play a hint is heard once, whichever character asks; in a cohort, each
  Linker hears their own.
- **Whose requests.** `~ shared` makes one pool; otherwise each character
  has their own and the pov spends from theirs. `inkjot check` refuses a
  `~ hint` Link in a story with no `resource hints`.

**Hints are not Inquests.** An Inquest is a Linker asking the story to
diverge from its canon at a point where the Inker hid a fork (`~ inquest`
on a Link). `inkjot check` accepts `~ inquest` with a notice; the
interpreter does not run it yet.

## What it does not do yet

It says so, by name, if a story needs one of these. It never guesses.

- Several Linkers in a world without stops (Ex 2 and Ex 3 are solo).
- Talking between Linkers, and the `turn limit` timeout: both belong to the
  platform, which runs in real time.
- `span` and `quiet` on handlers (reserved).
- `plus 2s when hinted` on an encounter (encounters are v0.6).
- The rest of the input family: `pick`, `rank`, `allocate`, `number`,
  `target`, `secret`, `roll`, `note`, `act`.
- Anything marked v0.6: the duel, `retell`.
