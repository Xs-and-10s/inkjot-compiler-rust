# Changelog

## Unreleased

### Milestone 22b: Ex 6 runs

- `^name` in a fact looks the name up; a bare word is always the word,
  and the checker notices one that is also in scope.
- `never … ~ on contradiction refuse | flag`, and `note {{ … }} in base`:
  the Linker writes, and a note that would break a refusing `never` is
  not kept. `base.contradicted`, `{contradictions ~ in base}`, `{known
  about: x ~ in base ~ with times}`, and the `{note who: kind, verb:
  kind ~ in base}` input (ask kind `"note"`).
- `for each P in list: effect`; facts as Link parameters; `x.did(act)` and
  a Link's parameters in NPC reactions; archetype members counted by
  `count(informant)`; `you.knows(it)` inside `count(…, where: …)`.
- A `play` of a place's passage with Links of its own goes there, as a
  scene; a test's `given:` arrives where an effect sent the pov.
- Checkpoints `~ on death` (anywhere) and `~ each hour`; one with no
  `resets` resets everything, the clock too; bases `~ per linker` come
  through a fall-back or a rewind as they are.
- `refill each turn`; `expect events: not died`.
- Ex 6 checks clean, passes its own tests, and every random evening ends.

### Milestone 22a: Inklog's core

- Facts in expressions (`{{ hale is ashe }} in notebook`, `?`, variables,
  `@ time`), asked of a base (the world when it says none) as the list
  that matches; quantifiers with a dot (`some P. …`, `no P. …`).
- `rule`, run to a fixed point in the bases it holds in, with joins,
  comparisons, other bases asked by name, and stratified `not` in closed
  bases; `~ known` and `~ learned from fact` for what each character
  knows; the `[rules]` margin.
- `world` and `knows` built in (`knows`: the meanings, `~ means`, of every
  fact learned); bases `~ for` someone, `~ starts with`, `~ reads`;
  `assert` with no `in` keeps a fact in the world; `retract`.
- A notice for `~ when` straight after `~ then` on a Link: that `when` is
  the Link's, not the effect's.
- `rule`, `never` and `retract` are no longer reserved.

### Milestone 21e: the table, through the engine; knowledge kept

- The engine seats a table: `Start.seats`, `Action::As { seat, action }`,
  `State::seats`, `State::view_of(seat)`, `Meta.cohort`; a table too
  small or too large for `cohort:` is refused, and so is an action no one
  does. In Elixir: `%{seats: [...]}`, `{:as, seat, action}`,
  `view(state, seat)`, `seats(state)`.
- `knowledge` bases and `assert {{ … }} in base`, a store only: Inklog's
  rules and questions are milestone 22's, on the same fact shape.
- Ex 7 checks clean, and plays whole, Detective included: its own tests,
  random games that all end, and (through the engine) a table whose log
  replays seat for seat. Milestone 21 is done.

### Milestone 21d: votes and seats

- `~ for guest where: …`; `~ consensus` and `~ consensus 2`, settled as
  the turn ends (`x.chosen_by`, `targets`); `cohort.all.chose`.
- `{ballot for: …, from: list, voters: …, ties: draw}` and `ballot.done`;
  the jury's `who:` as a list, `voters:`, `ties: draw`, an optional `how:`,
  `accusation.voters`, `accusation.drawn`, and `it.voted_with(…)`. A jury
  nobody may sit on decides at once, by lot (found by Ex 7's random games:
  a table where everyone pays a debt could not vote).
- Seats in a ring (`relation seat: each guest ~ ring`) and `spreads …
  ~ along seat`.
- Tables with named columns (`roll t where:`, `draw t row`, `t.column`);
  lists of text, picked their way by `one(list)`; `no`/`some`/`every guest
  where: …`; `first`, `last`, `possessive`; pronouns; `set x.shown_as: …`;
  `{text input}`; `{choose input}` from a list; `finale when:`.
- A `~ for cohort` passage's header runs once, for the one who took it;
  the others arrive with its `let`s, and statements about `me` run for
  each. Whole numbers divide into whole numbers, rounded down.
- Fixed: a value `~ of guest` was counted as one of the guests; a story's
  own `alive` was ignored; `until` inside quoted text cut a `set` in two;
  `expect at:` refused a sub-Nib; tests may sit at and vote for seats, and
  `given:` lists what is offered again.
- Ex 7 now checks clean but for the Detective's knowledge base (milestone
  22's), and plays: `tests/ex7_play.rs` runs its own tests with those two
  lines set aside, and 32 random games, every one of which ends.

### Milestone 21c: time

- Periods: `period phase: night 1, day 1`, `period season: spring, summer
  ~ 3 each`, `period day: monday, tuesday ~ of watch`. `clock.phase` reads
  the name it is now, names of one period compare in its order
  (`clock.hour >= ten`), the clock's range may be written in them
  (`clock: new_years_eve.eight ..= new_years_eve.midnight`), and `once per
  hour` limits by one. `period` is no longer reserved.
- `on turn begins`; `on phase begins night`, `on hour ends eleven`, or any
  of a period's names (`on watch begins`). The start is not a beginning.
- `wait until night` (or `tuesday.dusk`) ends turns until it begins; a
  test may say `given: wait until …`.
- `clock: … ~ turn ends when pov.all.spent or pov.all.passed`: a `pass`
  says the pov is done (`me.passed`), `x.spent` says x has none left of
  what Links cost, and the turn ends when the condition holds. Ex 2's
  tests now pass with both heroes; an engine test likewise.
- Real time: a Link `~ after 5m` is taken by itself when its time has
  passed. The engine has `Action::Wait(ms)` (`{:wait, ms}` in Elixir) and
  a Link's `due_in`; tests say `do: wait 5m`. Ex 7's night has its
  deadline this way (Dawn, after two minutes), and its two `on phase
  begins` handlers, which would have shown the night and the morning
  twice, are gone.

### Milestone 21b: cards and seats

- `character guest ~ one per linker`: a seat each, `guest_1` to the
  cohort's largest; `count(guest)` counts those seated; seating a Linker
  as `guest` takes the next seat. Characters now take their `~ state`.
- Decks (`deck roles ~ card mafia: members ~ card townsfolk: rest`),
  piles (`pile hand ~ per guest ~ private ~ face down`), `shuffle`, `deal
  1 from roles to each hand`, `top of …`, `my hand`, piles as members and
  over lists, `move card to x.pile`, `move each … to it.…`, `count(…)` of
  any list, deck or declared list, `{list hand of each guest}`.
- `~ for cohort`: one Linker takes it for the table, and everyone goes.
- `{present linker}`; `max(…)`, `min(…)`.
- Ex 7: the pile `role` is `hand` (beside the state `role`), and the
  killers' card and team are `mafia`, since a bare `syndicate` reads the
  syndicate drawn. Its cards check and deal; the rest is 21c and 21d.

### Milestone 21a: the Doppelgänger pass, and `strikethrough`

- Ex 7's Doppelgänger Clan, as agreed on 25 Nov 2026: each vote carries
  its voter's method (`me.method`), and the one who does the deed does it
  their way; one face each (`me.swapped`), a stopped replacement not
  counting; a replacement leaves no body, and the morning says the old
  name is missing; the engine's plain lines ("Gone in the night", "Missing
  in the night") are shown whoever tells the morning; new measures
  `disguised_lynched`, `replaced_doctor`, `replaced_detective`; the
  night's deadline noted for `~ consensus`. The platform's side is in
  `docs/platform/compiler-followups.md`, 11.
- `strike x on list` is now `strikethrough x on list`, and the style
  `struck` is `strikethrough` (a change the platform sees in a run's
  `style`). `inkjot migrate` rewrites `strike`; `strike` as a statement or
  effect is an error that says so. Ex 3's IR fixture changed with it.
- The plan, decided 26 Nov 2026, in `docs/status.md`: milestone 23,
  encounters generalized (Ex 11 and Ex 13); 24, closing v0.6; Ex 10, Ex
  12 and groups in v0.7.

### Milestone 20d: encounters

- `[encounter]` passages with `encounter x ~ round by round ~ acts last ~
  until …`; `[round]` and `[resolve]` passages. Rounds are turns: when one
  ends, the opponent's threat comes and the `[resolve]` passage is shown.
- `pattern name: a, b, c ~ repeat ~ one per round`, with `x first when …`
  and `x instead of y when …`; `npc.pattern.next`.
- `threat x: tag ~ at who ~ answered by prevent, block, cover, absorb ~
  fails when … ~ on fail: … ~ then …`; `threat.name`, `threat.target`,
  `threat.outcome`.
- Counters: `prevent x`, `block for x`, `cover x` (until used, or `~ for N
  turns`), `absorb with item`; items that `~ deflects fire` or `~ absorbs
  any`; `on absorb with item`. And `pin x`, `skip x this round`, `end
  encounter`.
- `set x: yes next round`; `wound x 1` costs `hp` when there is one, and
  none left is death; `x.down`, `x.up`, `me.asked_hint`; `take x` by
  whoever acts.
- A death that sends the pov to an ending stands.
- `holds.any(sword)` finds things `like sword`.
- `spell`, `shield`, `interrupt`, `guard`, `parry` and `freeze` as
  statements are errors that say what replaces each.
- **Ex 2's duel is in general words** (from `ex2-duel-general.twee`), with
  nine tests; `get holder` moved to the config, where the duel's pattern
  can see it. Its IR fixture is regenerated.
- **Ex 9 checks clean and runs whole**: its Warden can be fought, warded
  and run from (three new `[test]` passages, run by `cargo test`), and
  random play goes through the fight.

### Milestone 20c: making and the rest

- `default takes: take, drop`: a Link to take each item lying here and to
  drop each held, beside the walks. `drop` is a built-in act and event.
- Counted items: `item stave: … ~ plural "staves" ~ count 4` is four items
  under one name; `lose stave 2`, `gain stave 2`, `me.holds.stave`; lists
  show "two staves".
- Recipes: `recipe stretcher: stave, stave, sacking ~ using hammer ~ make
  ~ "…"`, offered as a Link when the parts are held and the tools at hand;
  making fires `make`.
- Tools: `~ uses pick` on a Link, `~ breaks 1 in 6` on the tool.
- `gain` of a resource (`gain salt 2`), `lose` of an item; `give`,
  `destroy` and `gain` take an expression (`give one(item, …) to x`,
  `destroy all(item, …)`, `give target to me`).
- Tables read as values: `table rank: score ~ 0..=19: "a lamp carrier"`,
  then `{rank}`. A row of one number (`100: …`) now matches it.
- `nearest(kind, where: …)` along the map, measured from who is moved, a
  spread's source, the NPC, or the pov; `x.toward(y)`, the next place on
  the way; `spreads N from SET to nearest(…) ~ along way ~ within D ~ then
  …`; `move x to nearest(…)`; `~ along way when …` on an effect.
- `count`/`one`/`all` take `in:` a list of places, and count a `let`'s
  list; `{list x}` shows a `let`'s list.
- A `play` of an ending sends the pov there once the action is done, with
  no turn after it.
- `state heat: 0 ~ belongs to place` gives every place one; places take
  their archetype's state and `~ with` values, as items do.
- Setting through a member: `set gleaner.place.heat: 1`.
- Two ways from here to one place each say which: "Go to the Incline (the
  cage)".
- In an act kept in a lib, `here` is the pov's place. Links offered
  everywhere, walks, takes and recipes are not offered in an ending, or in
  a prologue that is not a place.
- A margin's `state salt` beside a config's `resource salt` is the NPC's
  own (`warden.salt`), not a second declaration of `salt`.
- A sub-Nib no longer needs its parent passage: `spell:kindle` is a name.
- The config's `let`s are in scope when the Links are offered again, as on
  arrival.
- Entering or playing an encounter's passage says it is not built yet.
- `inkjot::ir::lower_unchecked`, for tests of drafts still being built.
- Ex 9 plays: at random (`tests/ex9_play.rs`, stopping only at the
  encounter) and won without a fight, by a stretcher made in the timber
  yard.

### Milestone 20b: time and motion

- `clock: 0.. ~ each action`: every Link taken is a turn, ended after the
  act is shown; hints, switching pov, going back and endings are not.
- `~ for N turns` and `~ after N turns [when …]` after an effect, on a
  header line, a Link, an act or a handler. A lasting effect puts back what
  it changed, and lasts from the latest time it was done; a waiting effect
  happens once. Both are kept in the world's snapshots, so an undo puts
  them back too.
- Handlers on a kind, tag or archetype (`on enter place`, `on enter
  shaft`), with `entered`, `left`, `crossed`, `moved` and `taken` for what
  it happened to, and `by me` for what the pov did. `on turn ends` reads as
  `on turn`.
- A `move` event, after `leave` and `enter`, for the pov and NPCs.
- `npc x ~ follows me when …`: the NPC moves with the pov, as they move.
- Goals: `~ every N turns`, `~ where: …` (the places an NPC may go
  through), and `~ wanders` in place of `~ toward`.
- A played passage (`then play fire:turn`) runs its header before it is
  shown.
- `x.place`, where a person or thing is.
- An act is done where the pov stands: an act kept in a `[lib]` passage no
  longer fires `leave` and `enter` for the lib.
- A remainder is `mod`; `%` is now an error that says so (Ex 7 and Ex 9
  wrote `%`).
- Ex 9's NPCs are goals now, its handlers `by me`, and its undo Links go to
  `rewind`.

### Milestone 20a: targets and filters

- `where:` with `it` in kind filters, and `all(…)`: `count(guest, where:
  it.alive and it != me)`. `it` is known only inside `where:`.
- Tags read as yes (`it.timbered`), and `x.next_to(y)`, `x.like(shaft)`,
  `x.held_by(who)`.
- `~ at item here where: …` on a Link or an act: one Link per target, with
  `target` in its label, conditions, effects and the passage it leads to.
- An act's `~ at`, `~ when`, `~ learned from` and `~ then` go to every Link
  that names it; a combo needs its acts known too.
- Link groups: `> ~ clause` lends its clauses to the Links below it.
- Effects on a set: `set due.alive: no` with `due` a list.
- The config's `let`s and `get`s are in scope in every passage (they were
  checked as such, but never loaded).
- A colon no longer starts a sentence for capitalising; a speaker's line
  does (a fix to milestone 19).
- Ex 9's spellbook is a `[lib everywhere]` passage, the built way to offer
  Links everywhere, so no new word was needed.

### Measures

- `measure name: expression` in the config: values the engine reports in
  a new `measured` event when a reading reaches an ending, for tuning
  from real readings. A measure is about the whole reading, so it cannot
  say `me`. Ex 7 declares the measures its tuning needs (who won, at what
  size, against which special, in how many nights, how well the town read
  the table, what each special's rule did), and the drafts README says
  which question each answers.

### Milestone 19: the page

- `narration: first person | second person | third person` (`~ omniscient`
  may follow) in the config or a passage, for the words the engine writes
  in prose: `{me}` reads "you", or "I" and "me". A speech cue always names
  its speaker, the pov too (changed after review, 21 Nov 2026: "Martha
  Rudd: …", not "I: …"). A sub-Nib is told as its passage is. `told:` is
  renamed `narration:`.
- `~ playable` is now `~ selectable`; the engine's `Meta` and the NIF's
  `meta/1` say `selectable`. `inkjot migrate` rewrites both renames, in a
  v0.6 file too; Ex 1 to 3 and the drafts are migrated.
- A name starting a sentence is capitalised, in prose and in speech cues;
  `~ keep case` opts out. List entries are shown as written.
- `cue names: capital, upper, bold, italic` and `cue directions: …` style
  speech cues.
- `[if c; then e]` and `[unless c; then e]`: the effect happens when the
  block is shown, top to bottom, so one block may unlock the next.
- `[end]` closes whatever opened last, an `[if]` chain or a block.
- Blockquotes, `[quote sign | letter | poster | edict ~ by "…"]` … `[end]`:
  each paragraph carries its `quote` (`kind`, `by`, `first`), in the view
  and through the NIF.
- Redaction: `[redacted until c]` … `[end]`, `{redacted "…" until c}` and
  `~ redacted until c` on a name. The hidden words never reach the view: a
  run of `█` as long as what it hides, styled `redacted`.
- Tests: `expect events:`, `expect fork: "…" ~ for "…"`, `take fork: "…"`.
- Ex 8 runs as written (`tests/ex8.rs` no longer rewrites it). Running it
  found a wrong expectation in the draft: taking a fork does not spend an
  Inquest in the engine; the platform spends it.
- `tests/page.rs`: each of the above, and properties: redaction shows only
  a length, capitalising changes only a first letter, and blocks nest as
  written, against a model of the rules.

### Engine

- Pacing reaches the view: each paragraph carries `pause` (`[after 2s]`,
  `[beat 2s]`, a wait after the paragraph before) and `verse`, and a run
  appended after a wait carries its own; the same through the NIF. Times
  may be decimal (`1.5s`) (#36).

### Ex 7

- *Mafia, by Lamplight*, drafted (`examples/drafts/ex7-mafia.twee`): ten
  syndicates, four that change the rules, three ways to moderate, and
  `where:` in nearly every Link. Linted with the other drafts.
- Who does the deed for the syndicate: the first member who placed their
  vote on the target chosen (`first(victim.chosen_by)`), so the
  Doppelgänger who takes the face and the thief who carries the keepsake
  are decided by the vote, and it pays to vote quickly. A stolen keepsake
  goes into its thief's own loot; the haul is all the Crew's loot, so the
  win count is unchanged, and the epilogue shows whose luggage held what.
- A thief voted out is searched, and what they stole goes back to its
  owners. The Crew now needs a third of the table's keepsakes plus one
  (`players / 3 + 1`, was `members + 2`), tuned so that, in random play,
  the town wins the Heist about as often as against a plain syndicate at
  every size from 5 to 12 (`tests/ex7_rules.rs`).
- The Cult and the Collectors, retuned the same way. The Cult: the One
  Who Left can never be taken again (and so sits up only for others), and
  once there are two Acolytes they go out on odd nights only; a notice
  tells the Acolytes, and the newly taken learn it. The Collectors mark two
  names a night. In random play the town now wins within about 12 points
  of its rate against a plain syndicate at every size, the Cult at five
  players excepted (a lone Acolyte; it leans to the town). A property
  checks every special against the plain game.
- Last bug pass: the Doppelgänger's swap now happens before the morning
  report; debts fall due at nightfall, a full day after marking; the
  syndicate's size counts players only, after the deal; teammates are
  found by team (so the cult sees the cult); a saved guest keeps their
  keepsake; a tied vote is drawn, so every game ends. `tests/ex7_rules.rs`
  plays the rules out for every syndicate at 5 to 12 players: every game
  ends, and both sides can win everywhere.
- The save now stops every special's night choice, which lands in the
  morning rather than on its Link: the Debt Collectors' Doctor is the Old
  Landlord, who takes someone in, and the Cult's is the One Who Left, who
  sits up with someone through the night.

### Ex 9

- *Saltmarrow*, drafted (`examples/drafts/ex9-saltmarrow.twee`): a
  Zork-like `game` in general words, with nineteen spells paid for in
  salt, fire that spreads through timber, shafts that fall after three
  burnt crossings, the Gleaner, and the Brinewarden. Linted with the other
  drafts. `tests/ex9_map.rs` plays it at random and checks there are no
  dead ends, with the pick or without, and that the spellbook's numbers
  agree with what the spells do.
- Last bug pass: Thwaite must be beside you to count as brought out (a
  lighten can wear off on the way; she waits where it did, and drags
  herself clear of fire); a treasure scores once; the Gleaner's candle
  never cools a blaze; the Warden's hall reads right when it has gone
  after you, and a heap of salt cannot be fought or paid; the sluice wears
  the Warden down only in its hall; the planks score whichever way they
  go; text fixed for a dry stair, a dark drift, a fixed jack roll and a
  shored shaft. New properties: every spell has its Link in the book, the
  ranks cover every score once, and the best game scores exactly 100.
- `define`, tested on its spellbook (`ex9-spellbook-define.twee`,
  `docs/define.md`): typed fields earn their place; the word itself stays
  optional.

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
