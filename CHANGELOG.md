# Changelog

## Unreleased

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
