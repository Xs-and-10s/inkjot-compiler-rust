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
| 17 | The NIF: `native/inkjot_nif` (Rustler) and `Inkjot.Engine` (`mix.exs` at the root, Elixir under `elixir/`), with StreamData properties; history snapshots shared behind `Arc`s (a Link 300 in to Ex 1: 25 ms to 0.5 ms); `build` through the NIF | #25 |
| 18a | v0.6 foundations: `state`, `fact`, `clue`; `inkjot migrate` (Ex 1 to 3 migrated); names for the pov; `[fix]`; reserved words; IR 3's core: stable Link ids, passage titles, typed state | #26 |
| 18b | Text: styled runs in prose and labels, braces in every string with escapes, `if … then … else`, `plural`, `capital`; names as the reader knows them (`shown_as` until `reveal`); `aka` policies and `{all x.aka}`; the engine's view carries styled paragraphs | #27 |
| 18c | Inquests: forks (`~ inquest`, `for`, `from`, `by role ~ rejoin`, `credit`) as the `inquest` field, checked; `theme`, `brief`; analysis tags; `~ track` and `changes` on `nib_shown`; the engine's `forks`, `any_forks`, `take_fork`, `asked_from` and `map()`, in the NIF too | #28 |
| 18d | Costs name their resource: `default cost: vults`, `{"amount": …}` in the IR, `requires full x` for any resource; `hushed` is no longer built in (Ex 2 declares it); the reserved resource names documented | #29 |
| — | Ex 6 to 9 drafted, reviewed and ready: *Seven Names* (the Library) with its public cut *Cloakroom*, *The Inquest*, *Mafia, by Lamplight*, *Saltmarrow*; pacing reaches the view | #30 to #38 |
| 19 | The page: `narration:`, `~ selectable` (with `inkjot migrate`), names capitalised at the start of a sentence (`~ keep case`), `cue names:` and `cue directions:`, `[if c; then e]` and `[unless c; then e]`, blocks that `[end]` closes in order, blockquotes, redaction three ways (the hidden words never reach the view), `expect events:`, `expect fork:`, `take fork:`. Ex 8 runs as written | #39 |
| 20a | Targets and filters: `where:` with `it`, `all(…)`, `~ at` (a Link per target, with `target`), acts that carry their conditions and effects to their Links, Link groups, effects on a set, the config's `let`s in every passage | #41 |
| 20b | Time and motion: `clock: … ~ each action`, `~ for N turns` and `~ after N turns` (kept in snapshots), handlers on a kind with `entered`, `left`, `crossed`, `moved`, `taken` and `by me`, `on move`, `~ follows me when …`, goals that keep to places (`~ where:`), move every so often (`~ every 2 turns`) or wander; a played passage runs its header | #42 |
| 20c | Making and the rest: `default takes`, counted items, recipes, tools that break, `gain` of a resource, effects on what an expression names, tables read as values, `nearest(…, where:)`, `toward`, `spreads`, an ending a `play` sends the pov to. Ex 9 plays to the encounter, and can be won without one | #43 |
| 20d | Encounters: `[encounter]`, `[round]` and `[resolve]` passages, rounds as turns, patterns, threats and the four counters, `deflects`/`absorbs` items, `next round`, `hp` that runs out. Ex 2's duel is rewritten in general words and tested; Ex 9 checks clean and its Warden can be fought. Milestone 20 done | #44 |
| 21a | Ex 7's Doppelgänger pass; `strike` and `struck` are `strikethrough`; the plan to v0.6.0 | #45 |
| 21b | Cards and seats: `one per linker`, decks, piles, `shuffle`, `deal`, `top of`, `my`, `move` to a pile, `~ for cohort`, `{present linker}`, `max`/`min`; Ex 7's role pile is `hand` and its killers' team `mafia` | #46 |
| 21c | Time: `period`s and named clocks (`clock.phase`, ordered names, clocks in names, `once per hour`), `on … begins` / `ends`, `wait until`, `turn ends when` with `passed` and `spent`, Links timed in real time (`~ after 5m`, `Action::Wait`, `due_in`); Ex 7's night has its deadline | #47 |
| 21d | Votes and seats: `~ for … where:`, `~ consensus`, `{ballot}`, the jury's `voters:` and `ties: draw`, seats in a ring, tables with named columns, lists of text, quantifiers, `first`/`last`/`possessive`, pronouns, settable `shown_as`, `{text input}`, `finale when:`. Ex 7 plays, but for its knowledge base | #48 |
| 21e | The engine seats a table (`Start.seats`, `Action::As`, `view_of`); knowledge bases and `assert … in`, a store only. Ex 7 plays whole. Milestone 21 done | #49 |
| 23a | Drafts of Ex 11 (*The Meridian Key*, a JRPG level and its boss) and Ex 13 (*Once Saved*, a debate on eternal security), linted, with what each asks of the milestone | #53 |
| 22c | Ex 5 brought up to date and running: meanings true in the world, `~ known` facts, `verb … ~ one value`, `observe`, a fact's terms, `relation … each`, `from:` a list, the clock running out into a finale anywhere; Inklog's closures remembered. Milestone 22 done | #52 |
| 22b | Ex 6 runs: `^` lookups in facts, `never` and `note` (refuse or flag), `{contradictions}`, `{known about}`, the `{note}` input, `for each`, facts as Link parameters, `x.did(act)`, scenes `play`ed by NPCs, checkpoints `on death` and `each hour` that keep a `per linker` notebook, `refill each turn` | #51 |
| 22a | Inklog's core: facts asked of bases, rules to a fixed point with stratified `not`, quantifiers with a dot, `world` and `knows`, `~ means`, `~ for`, `~ starts with`, `~ reads`, `retract`, `[rules]`; a notice for `~ then … ~ when` on a Link | #50 |

## Next

The plan agreed on 20 Nov 2026, after the platform's first slice:

1. **Milestone 15, the engine API.** Done.
2. **Milestone 16, engine events.** Done.
3. **Milestone 17, the Rustler NIF.** Done. The platform's side:
   `Inklinks.Engine.Rust` over `Inkjot.Engine`, its Ex 3 scenarios run
   against it, and the stub kept as an oracle (follow-up 4).
4. **Milestone 18, IR 3 and the first part of v0.6**, in three pull
   requests (see "Inkjot v0.6" below for what is in each): #26, #27, #28,
   and a fourth, #29, that takes the last Inklink words out of the engine.
5. **Ex 6 to 9**, pitched and chosen on 20 Nov 2026 (below), drafted in
   the order 6, 8, 7, 9. Done: all four ready (21 Nov).
6. **The rest of v0.6**, agreed 21 Nov 2026, the examples as acceptance
   tests: a milestone is done when its example runs as written.
   - 19, **the page**: done when Ex 8 runs with no rewrites. Done (#39).
   - 20, **the world**: targets and `where:`, Link groups, Links offered
     everywhere, handlers on a kind, `on move`, durations and delays, NPCs
     that follow or pursue, `spreads`, threats and counters, recipes,
     `~ breaks` and `~ uses`, counted items, `default takes`, dice, tables
     read as values. Done when Ex 9 and the Ex 2 duel in general words run.
     In four parts: 20a, targets and filters (#41); 20b, time and motion
     (#42); 20c, making and the rest (#43); 20d, encounters (#44). Done.
   - 21, **the table**: decks and piles, periods and named clocks,
     `{ballot}`, `voters:`, `ties: draw`, `~ consensus`, seating, settable
     `shown_as`, `finale when:`, text tables, `spreads … ~ along seat`.
     Done when Ex 7 runs. Its first step is the Doppelgänger pass agreed
     on 25 Nov 2026:
     1. The method follows the doer: each vote carries its own (kill or
        replace), and the first to vote for the chosen victim does the
        deed their way.
     2. One successful replacement per member; one the Doctor stops does
        not use it up. "Replace" is offered only to those who have not.
     3. A replacement leaves no body: the morning says the replacer's old
        name is missing and nobody died. The lynch reveal stays.
     4. The night has a deadline: votes cast by then count; none, no kill.
        Part of `~ consensus`.
     And, from the designer's notes: the engine's plain lines (who died,
     who is missing) always shown, whatever the Storyteller or MonkeyPaw
     tells; measures for a face-wearer voted out and for the role of the
     one replaced; for the platform briefing, presence and typing follow
     the face, not the account, night results in one batch, irreversible
     night choices confirmed, the replacer never sees the victim's private
     messages, and an asynchronous day settled at its deadline.
     21a (#45) did the Doppelgänger pass, and renamed the list effect
     `strike` (and the style `struck`) to `strikethrough`, decided 26 Nov
     2026. Then: 21b (#46), cards and seats (`one per linker`, decks,
     piles, `~ for cohort`); 21c, time (periods and named clocks, night and
     day, `turn ends when`, the night's deadline as a Link timed in real
     time; the votes it counts are 21d's); 21d, votes and seats
     (`{ballot}`, `voters:`, `ties: draw`, `~ consensus`, seating and
     `along seat`, settable `shown_as`, `finale when:`, text tables, the
     quantifiers Ex 7 uses, `first`, `last`, `possessive`); 21e, the
     engine's cohort API, so the platform can seat a table, and a
     store-only slice of knowledge bases (`knowledge`, `assert … in`),
     decided 27 Nov 2026, so Ex 7 plays whole. Milestone 21 is done (#49).
   - 22, **Inklog**: rules, `{{ }}`, knowledge bases (built on 21e's
     store and its fact shape), `assert` and
     `retract`, quantifiers, facts at a time, `never`, `{note input}`. Done
     when Ex 6's public cut runs and Ex 5 is brought up to date.
   - 23, **encounters, generalized** (decided 26 Nov 2026): `encounter`
     stays and is documented as any exchange in rounds, not only a fight;
     `~ as fight | negotiation | seduction | discovery | other`, settable
     mid-scene, a label for conditions, the platform and the Inker's
     reports, not separate rules; `unleash x` (from the world) and `unleash
     x ~ by y` (from someone), a threat outside a round; the outcome `hit`
     renamed `landed`; an opponent's moves chosen by condition or weight,
     not only a fixed pattern; and what Ex 11 asks for (several opponents,
     pattern phases, resistances and weaknesses, cooldowns, one command per
     ally a round). Drafts first, as with Ex 6 to 9. Done when Ex 11 and
     Ex 13 run. In three parts: 23a, the two drafts (#53); 23b, the
     general words (`~ as`, `unleash`, `landed`, moves by weight, counters
     as tags, `answer`) until Ex 13 runs; 23c, what Ex 11 adds (several
     opponents, commands, kinds of harm, cooldowns, phases) until it runs.
   - 24, **close v0.6**: `retell`, `define` step 2 if it still earns its
     place, the IR frozen, release v0.6.0. **Then the platform**, with
     v0.7 alongside.
   - To v0.7: engine timers for effect chains that wait on the clock;
     `define` steps 1 and 3; facets and `~ conflict`; Ex 10 and Ex 12;
     groups (below).

**Groups** (decided 26 Nov 2026, for v0.7, with Ex 12): one declaration
for any body of people, named or faceless or both: an army, a party, the
CIA, a visiting team, a religion and its sects. `group fleet: "the Fifth
Fleet" ~ size 3000 ~ members vance, okafor ~ part of navy`: `size` counts
everyone, the named among them; `part of` nests (a sect in a religion, a
command in an army); `x.in_group(fleet)`, `count(person, where:
it.in_group(cia))`; a group may be a threat's target, have goals like an
NPC, stand in a relation (`trust` with a faction), and lose `size` in a
battle without anyone being named. Ex 7's `team` is a group in all but
name. The word is `group`: `faction` and `side` assume a quarrel, `party`
and `company` have other meanings, `cohort` is taken by the Linkers, and
`host` by Ex 2's lich.

## Ex 10 to 13 (decided 26 Nov 2026)

| Ex | What | When | Lives | Tests most |
|---|---|---|---|---|
| 10 | A period social `story-game` after *Pride and Prejudice* (public domain), a small section of it played | v0.7 | here, unless decided otherwise | encounters `~ as seduction` and `~ as negotiation`, reputation as a value, gossip that spreads among acquaintances, several people in one scene |
| 11 | A JRPG-style fantasy and science-fiction `game`, one level and its boss: a party of four rebel scouts of different classes, hunted by an empire, getting something in and out. One leader, commands to the other three; the leader may be switched | v0.6, milestone 23 | here, an original setting | several allies and opponents, one command each a round, classes as archetypes, resistances, pattern phases, cooldowns |
| 12 | A military-science-fiction `story`: a commander wins before the battle, by reading the enemy and baiting a chain of small mistakes, in the vein of Gordon R. Dickson's *Tactics of Mistake* without borrowing from it | v0.7 | to be decided | groups, NPC goals at army scale, Inklog on the enemy's intent |
| 13 | A debate `game`: an amicable argument with a Protestant relative on eternal security ("once saved, always saved"), from traditional Catholic teaching (Trent's decree on justification; Aquinas; Cornelius à Lapide), with logic, scripture and analogy. Win (defend the position), convince, both, tie or lose. No Inquests, no rewinds; hints as recollections of what he said before. His side argued at its strongest, so that a win means something | v0.6, milestone 23 | here (decided: an acceptance test, not a Library piece) | a verbal encounter: objections as threats, answers as counters, conviction and understanding as values, outcome tiers |

## Ex 6 to 9

| Ex | What | Lives | Tests most |
|---|---|---|---|
| 6 | *Seven Names*, spy-noir `story-game`, solo | the Library (decided 21 Nov); its public cut, *Cloakroom* (`examples/drafts/ex6-cloakroom.twee`), here | knowledge bases, integrity constraints, redaction, blockquotes, pronouns and `possessive()`, `reveal` and `aka`, uneven periods, `expect events:` |
| 7 | *Mafia*, `game`, 5 to 12 Linkers | here; the Library later, under a Creative Commons licence, if the themes come off | decks and private piles, night and day, `{present linker}`, per-Linker knowledge, `where:` (its showcase), `\|` in filters, quantifiers, `finale when:`, tables |
| 8 | *The Inquest*, `story`, solo, short | here; a longer literary version may go to the Library | forks (written and pending), analysis tags, `theme`, `brief`, `[if …; then …]`, emphasis, `~ track`, presentation hints |
| 9 | *Saltmarrow*, Zork-like `game`, solo | here | the encounter words, `spreads`, recipes, `~ breaks`, dice and tables, plural irregulars, and `define`'s next test (a spellbook) |

**Ex 6, *Seven Names*.** A neutral port, 1952. A cipher clerk, a dead
head of station, an agent with seven cover names. Suspense comes from the
hunter becoming the hunted: the antagonist has a knowledge base of his
own, fed by informants and by reading the clerk's notebook, and acts on
it. Drafted 20 Nov, in review (private for now; where it lives is Mark's
call). Later, a Library version with the antagonist chosen at random each
reading, for replays.

**Ex 7, *Mafia*.** The engine moderates by default. Each game draws a
syndicate from a shuffled table: a type (historical, mythical, absurd or
horror-tinged), a setting to match, its own titles for the roles, how it
kills, and a table of flourishes for each death so the theme builds as
bodies fall. Special syndicates change the rules: the Doppelgänger Clan
(a killer may take the victim's form, so the morning reports the wrong
death), and three more drafted with it. Three ways to moderate: the
engine's tables alone; tables with generated flourishes (MonkeyPaw, the
table's line as the fallback); or a human Storyteller who sees everything
and may rewrite each morning's report. Whispers is an optional variant.
Syndicates name no real living people or real present-day gangs, and use
original names rather than those from films and songs.

**Ex 7, *Mafia, by Lamplight*.** Drafted 21 Nov
(`examples/drafts/ex7-mafia.twee`); ready (#37). The save stops every
special's night choice; the Debt Collectors' Doctor is the Old Landlord
and the Cult's is the One Who Left. `tests/ex7_rules.rs` plays its rules:
every game ends, and both sides can win with every syndicate at 5 to 12
players. What it asks of v0.6 beyond the plan is in the drafts README.

**Ex 8, *The Inquest*.** Drafted 21 Nov (`examples/drafts/ex8-the-inquest.twee`),
in review. Runs on today's engine bar five unbuilt constructs
(`tests/ex8.rs`). Found while writing it: an `[act]` passage cannot offer
Links of its own (the testimony's steps are plain Nibs); speech labels
keep a lowercase name at the start of a line ("the foreman: …"), which
the engine should capitalise (proposed below); `[if c; then e]` confirmed.
A fen village, 1893: the lock-keeper drowned the
night the sluice was opened. The frame is in the second person, to the
coroner's clerk; each testimony is in the first person, as the witness,
some reliable and some not. Dialogue is screenplay-style speech cues.
Possible v0.6 addition it may ask for: telling the engine a passage's
person (`told: first person`), so `@you` and `{me}` read "I" in a
testimony and "you" in the frame.

**Ex 9, *Saltmarrow*.** Spells cost salt, carried as crystals. A dozen or
more spells, with power, duration and range, some composing into greater
ones. Fire needs a flame near dry timber (the lamp, a spell, the thief's
candle, firedamp); about a quarter of the map is timbered. A burnt shaft
can be crossed a few times, with warnings, then falls in. No dead ends:
every collapse leaves another way, by magic or by engineering, checked by
random playthroughs. The guardian can be fought, sneaked past, bargained
with or outrun.

**Ex 9, *Saltmarrow*.** Drafted 21 Nov
(`examples/drafts/ex9-saltmarrow.twee`); ready (#38). Fifteen places, four
timbered; nineteen spells, five of them greater workings; the Gleaner; the
Brinewarden, who can also be left the shard. `tests/ex9_map.rs` plays it
at random and searches for a way out from wherever play stops: no dead
ends, with the pick or without. `define`'s second test is in
`docs/define.md`: step 2 (typed fields on archetypes) earns its place;
step 3 stays optional.

**`where:`** is used heavily where the rules, not the Inker, pick which of
many things: targets in a social game, an NPC choosing loot, a fire or a
rumour choosing where to go next, counts for a win. Ex 7 is its showcase,
then Ex 9 and Ex 6; Ex 8 hardly needs it.

**Built in milestone 18 but in no example yet**, to be covered by Ex 6
to 9: forks and Inquests (Ex 4 was the only one), `~ track`, `reveal`,
`{all x.aka}` and `~ pick`, `plural`, `capital`, `if … then … else …` in
braces, emphasis, `theme`, `brief`, analysis tags.

**Ex 5** predates the 20 Nov decisions: its seasons are a clock setting
(now `period season`), its relation names the gardener (now `you`), and
`world only` / `almanac only` become `~ in world` / `~ in almanac` with
`knowledge almanac`.

## Inkjot v0.6

The whole list, so nothing proposed falls through the cracks. Decided on
13, 15, 16 and 20 Nov 2026 unless marked. Everything that breaks a v0.5
source lands in milestone 18, behind `inkjot migrate`, so there is one
breaking change.

### Milestone 18a: foundations (breaking)

- `format-version: 0.6.0`.
- **`state`, `fact`, `clue`.** `state` is a mutable value (v0.5's `fact`):
  `state wound_watch: no ~ belongs to character`, read as
  `me.wound_watch`. `fact` is knowledge (v0.5's `clue`): `fact debts: "…"`,
  learned with `learn debts`, tested with `me.knows.debts`. `clue` becomes
  a tag on a fact (`~ clue`); `proof_who`, `proof_how`, `motive` and
  `clears` stay tags, and `evidence` stays a tag on items.
- **What a character knows** is the built-in knowledge base `knows`
  (below): `learn` adds a fact to it, `me.knows.x` asks it. The syntax does
  not change; the meaning is defined so Inklog extends it.
- `[fixme]` is now `[fix]`.
- **Names for the pov:** `i`, `me`, `you`, `myself`, `yourself` all mean
  the pov, everywhere, NPC margins included; the IR always says `me`. House
  style: `i` as subject, `me` as object. `self` is the NPC or character
  whose margin it is, with no other spelling: `it` is the candidate in a
  `where:` filter, so it cannot also mean `self`.
- **Reserved now, built later**, so ids written today never collide:
  `knowledge`, `rule`, `never`, `assert`, `retract`, `period`, `recipe`,
  `between`, `{{` and `}}`.
- `inkjot migrate` rewrites a v0.5 source mechanically.

### Milestone 18a: IR 3, the core

- §0: a consumer ignores fields it does not know.
- §2: Link ids that survive revisions: the Inker's `~ id`, else derived from
  passage and target (`ch2->ch3`), with a suffix only for a clash.
- §4: `state` declarations carry their type.
- §5: passage titles, `title: "…"` in the header.

### Milestone 18b: text

- §3: emphasis (`*italic*`, `**bold**`, `***both***`; underscores literal,
  `\*` an asterisk, never across a paragraph or in code) lowered to styled
  runs; Link labels become lists of parts.
- **String interpolation**, one form: braces work in every quoted string
  as in prose (`"Keeper of {here}"`), with `\{` and `\}` for literal braces.
  Inline choice is an expression, `{if c then "a" else "b"}`; square
  modifiers stay line-level. Helpers: `plural(n, "card")`, `capital(x)`,
  `list(x)`. In an effect, worked out once, as it runs; in a label, `shown
  as` or fact text, each time it is shown.
- `shown_as` until `reveal`: `{marge}` and `@marge` show what the reader
  knows; `{marge.name}` and `{marge.shown_as}` always one or the other.
- `aka` policies, declared once: `~ pick shuffled | cycle | random | first`;
  `{x.aka}` picks one, `{all x.aka}` shows the whole list.

### Milestone 18c: analysis, tracking, relations, Inquests

- `theme` declarations and the analysis tags on Links (`choice`, `beat`,
  `weighs`, `arc`, `stakes`, `plants` / `pays off`, `predict`, `lasting`,
  `tone`, `reveals`). They never change play.
- `~ track` marks values whose changes the engine reports on its events
  (`platform/events.md` §9.8).
- §1, Inquests: `~ inquest`, `~ inquest for <id>`, `~ inquest from
  <passage>`, `~ inquest by <role> ~ rejoin …`; the `inquest` field
  (`stands_in_for`, `from`, `credit`, `pending`); `brief` declarations as a
  shape only; `kind: "inquest"` retired. The checker: solo only, the
  offered Link in the same passage, `from` names passages, a warning for
  forks or offered Links without ids. Generated forks must pass the
  checker. Roles never name people.
- The engine: `asked_from`, `forks`, `take_fork`, and `map()`, the story
  as a graph.

### Milestone 18d: the engine speaks no Inklink's words

- `default cost: vults` names the resource acts spend; a cost in the IR is
  `{"amount": 2}`. Without a `default cost` nothing is spent, and `inkjot
  check` notes the first cost. `inkjot migrate` adds `default cost: vults`
  to a v0.5 source that declares `resource vults`.
- `requires full x` reads the resource it names.
- `hushed` is no longer a built-in name: Ex 2 declares `state hushed: no`.
- `rewinds`, `hints` and `inquests` are the reserved resource names, the
  player's rather than the world's (documented in `run.md`).

### Later in v0.6

- **Inklog:** `rule`, `{{ }}` lexed as its own tokens, `~ means` on facts,
  stratified negation, `assert`, `retract`, `assert … until`, `verb … ~ one
  value`, `me.knows {{ … }}`, `{known about: x}`, the `[rules]` margin.
- **Knowledge bases**, declared with `knowledge`, each with an owner
  (`shared`, `per character`, `per linker`) and a way facts get in
  (`truth`: closed world, `not` allowed; `learned`: open world, no `not`;
  `manual`: the Linker writes them). Built in: `knowledge world ~ truth`
  and `knowledge knows ~ per character ~ learned`. A rule says where it
  holds (`~ in world`, `~ in knows ~ learned from x`), replacing Ex 5's
  `~ world only` and `~ almanac only`.
- **Integrity constraints** for Ex 6: `never … ~ in notebook`, `!=` and
  comparisons in rule bodies, `~ on contradiction refuse | flag`, and a
  `{note input}` for facts the Linker writes.
- Quantifiers `some`, `every`, `no`.
- `on turn begins`, `on turn ends` (v0.5's `on turn` a synonym). Built
  in 21c.
- **Periods:** `period season: spring, summer, autumn, winter ~ 3 each`,
  `period year ~ 4 seasons each`, uneven lengths (`dawn 1, day 4`);
  `clock.season`, `on season begins summer`. Built in 21c (a year is
  `period year: one ~ of season`; `~ 4 seasons each` was not kept).
- Kind filters: `where:` with `it` for each candidate; `|` for
  alternatives; `{present linker}`.
- Relations among a kind: `relation trust: you, each gnome ~ 0..=3`,
  naming both ends: `raise trust between me and self 1`, read as `trust
  between me and pip` or `me.trust(pip)`. (v0.5's relations are one pair,
  so nothing breaks waiting for this.)
- The dice language and random tables (one `table`, no `matrix`).
- Decks, piles, melds, card effects, one Link per thing with `~ for each`.
- `[if …; then …]`, in place of automatic forks.
- Making things: `recipe lockpick: hairpin, spring ~ using pliers ~ make`.
  Tools do not break by default; `~ breaks 1 in 6` is an opt-in, and a
  tool that can break must never be the only way forward.
- `finale when:` for endings that depend on play.
- `expect events:` in tests.
- **Redaction**, for Ex 6: `~ redacted until …` on list items, `aka`
  entries, paragraphs, Link labels and documents (below). Redacted text is
  shown as a bar and never reaches the page; the IR carries its length
  only.

Inklog and time, from Ex 6 (21 Nov 2026):

- **Decided.** A fact is two terms (`{{ you curious }}`) or three (`{{ hale
  is ashe }}`); a verb is one word (`asked_about`). A fact may carry a
  time after `@`: `{{ hale at miramar @ monday.evening }}`. The time is
  when it was true (valid time), in the periods' names; when a fact
  reached a knowledge base is kept by the engine on its own (`{known
  about: x ~ in dossier ~ with times}`). `@` already means "at a time" in
  cells (`dining@2`).
- **Decided.** Quantifiers end with a dot: `some P. you.notebook {{ it is
  P }}`; several of one kind with commas, kinds in a row with dots: `some
  P, Q. all R. …`. The dot must be followed by a space, which tells it from
  `P.name`.
- **Decided** (21 Nov). Periods made of periods: `period day: monday, …
  saturday ~ of watch`; the clock's range in their names; inline
  redaction with braces. (Arithmetic in `src/period.rs`, with property
  tests; the checker and interpreter use it when periods are built.)
- **Found** (21 Nov, reviewing Ex 6). A base is asked with `in`, no tilde:
  `{{ you must_go }} in dossier`, `assert {{ … }} in dossier`, `{{ ? is
  ashe }} in notebook`. Written `~ in dossier`, the tilde starts a new
  clause and cuts the condition in two. `~ in dossier` stays right where
  it is a clause: on a `rule` or a `never`. `you.notebook {{ … }}` is
  dropped for the same form. `tests/drafts.rs` enforces it in drafts.
- **Was proposed** (now decided above). A period made of another names it: `period day: monday, …
  saturday ~ of watch` (one round of watches each), so nothing is counted
  or pluralised; a number still means turns (`~ 3 each`). The clock's range
  in those names: `clock: monday.morning ..= saturday.night`. Reading:
  `clock.day`, `clock.watch`, and the turn within each.
- **Proposed.** Inline redaction as an insert, `{redacted "the Dentist"
  until you.clearance >= 2}`; `[redacted until …]` stays for paragraphs;
  `~ redacted until …` on a declared thing hides its name everywhere.
- **Proposed.** A state that holds an id may say of what kind: `state
  named: nobody ~ of suspect`.
- **Fixed** (#33): `person`, built in as a character or an NPC, now counts
  in `count`, `one` and `all`, as it already did in goals; documented.
- From the draft's README, still to place: a manual base that reads
  another (`~ reads knows`); a base owned by an NPC (`~ for ashe ~ starts
  with …`); asking a base by name (`{{ … }} ~ in dossier`,
  `you.notebook {{ … }}`); `for each P in …: effect`; the notebook's
  inserts (`{note …}`, `{contradictions}`); per-Linker knowledge surviving
  a fall-back; a checkpoint that shows a passage first; `finale when:` in a
  passage header; pronouns the Linker chooses.

From Ex 8's review (21 Nov 2026):

- **Decided.** `[if c; then e]`: the block shows when `c` holds, and `e`
  happens when it is shown. Its mirror, `[unless c; then e]` (Mark
  proposed `do`; `then` is recommended, since `then` introduces effects
  everywhere else). Effects in a passage happen top to bottom when it is
  shown, so a block may unlock the next ("effect chains"). `[after 2s]`
  only paces the showing; a chain that waits on the clock needs engine
  timers, a v0.7 question.
- **Decided** (21 Nov). `told:` becomes `narration: first person | second person |
  third person`, the craft term; `person:` would clash with the built-in
  kind. It decides only the words the engine writes (`@you`, `{me}`,
  pronoun inserts, retellings) and the person a generated fork must be
  written in; prose is written by hand in whatever person it is. Close,
  distant and omniscient are stances, not grammar: an optional clause
  (`~ omniscient`) for analysis and briefs, ignored by play.
- **Decided** (21 Nov). `~ playable` becomes `~ selectable`: every `character` can
  be played (that is what makes it not an `npc`); the clause means "offered
  to choose at the start". Ex 8's witnesses are characters reached by
  `become`, so only the clerk is selectable. Renamed with `inkjot migrate`.
- **Decided** (21 Nov). Names at the start of a line or sentence are capitalised
  by the engine, everywhere, not only in speech labels ("The foreman: …");
  `~ keep case` on a name opts out. Styling the parts of a cue with a list
  of transforms applied in order: `cue names: capital, bold`, `cue
  directions: italic`. (Mark's pipeline, `~ capitalized(_) ~> bold(_)`,
  written as a plain list; the full pipeline waits for a second use.)
- **Done** (#36). `[after 2s]`, `beat` and `verse` reach the engine's
  view: each paragraph carries its `pause` and `verse`, and a run appended
  after a wait carries its own (`run.md`, Pacing). The `reveal:` setting's
  mode is still the runtime's to read from the IR.
- **As designed.** A speech direction, `@fenwick (to you)`, is shown to
  the reader, as in a play script; an author's note in a cue is written
  `((…))` and never shown.

Text, decided 20 Nov 2026:

- **`plural()`** keeps its rules and gains a built-in table of about sixty
  common irregulars and invariants (`child`, `mouse`, `sheep`, `series`…),
  the third argument as an override (`plural(n, "goose", "geese")`), and
  `~ plural "geese"` on a declared thing so `{plural(n, goose)}` needs no
  override. Documented: pass the singular.
- **`possessive()` and pronouns**, together: `~ pronouns she` (or `he`,
  `they`, `it`, or a set: `~ pronouns xe/xem/xyr`), read as `{x.they}`,
  `{x.them}`, `{x.their}`, with `capital()` for the start of a sentence;
  `possessive(x)` gives "Leo's", "the Widow's", "the twins'".
- **Blockquotes**: `[quote sign]` … `[end]`, with kinds `sign`, `letter`,
  `poster`, `edict` and an optional `~ by "…"`. An IR node of its own, so
  the platform styles each kind. Pairs with redaction: a redacted line in a
  letter.
- **Presentation hints** on an insert: `{list stocks ~ marquee}`, `{all
  leo.aka ~ rotate 2s}`. The engine passes the hint through and changes
  nothing else; the text is complete without it. The platform renders it
  with a pause control (WCAG 2.2.2, anything that moves for more than five
  seconds) and, under `prefers-reduced-motion`, shows the whole list at
  once. The intent: the reader lives what the character does, remembering
  a list one item at a time, or a mind that will not stop cycling (OCD).

Inquests:

- **A pending fork's one home** (with MonkeyPaw): the placeholder passage
  says everything, `pending: by monkeypaw ~ rejoin ch5 ~ brief tragic ~
  timeout 1m`; its body is the fallback a Linker reads if nothing is
  generated in time; the fork itself just says `~ inquest`.

**The encounter milestone**, Ex 2's duel rewritten in general words (the
analysis of 20 Nov 2026):

- `spell` becomes `threat`: anything incoming that counters can answer.
- Counters come in four kinds, `prevents`, `blocks`, `covers`, `absorbs`;
  effects `block`, `prevent`, `cover`, `deflect with`, `absorb with`.
  `parries flame` is `~ deflects flame`; `absorbs spells` is `~ absorbs
  any`.
- Materials are tags plus rules, not verbs: `thaws ice` and `cracks glass`
  go. `shields target…` and `interrupts flame, frost` are covered by the
  counter kinds. `needs two to hold it` is stale.
- `freeze` goes; `set x.frozen: yes` says it.
- `cast.spell` and `cast.outcome` are `threat.name` and `threat.outcome`;
  the `absorbed` built-in is `threat.outcome == absorb`; `hask casts last`
  is `~ acts last`; `through mirror` is `~ requires mirror`; `hinted` is
  `me.asked_hint`.
- Kept: `encounter`, `pattern`, `pin`, `skip`, the `round` tag.
- Targeting and spreading, for any act or Link: `~ at enemy here` gives
  `target`, one Link per target, `{target}` in the label; `~ then after
  0.5s: …`; `~ spreads 2 to nearest enemy ~ every 0.3s`, with `hop`
  counting the hops, for chain lightning, a fire moving room to room, a
  rumour among gnomes.
- `~ answered by prevent, block, cover, absorb` on a threat;
  `threat.target`; `on absorb with lumen`.
- **Link groups**: a `>` line with clauses and no Link applies them to the
  Links below it, up to a blank line (`> ~ for eldric ~ when not
  eldric.frozen`).
- The draft: `examples/drafts/ex2-duel-general.twee`, now Ex 2's own duel
  (milestone 20d, #44). It also fixes two
  bugs in the duel (`sanctum:shatter` naming no mirror; the cover line
  always saying Roland).

**`define`** (proposed 20 Nov 2026; `docs/define.md`): an Inker's own
declaration word, built on a general one, with typed fields and the verb
the platform shows, lowered by the compiler so the engine never sees it.
Planned in three steps: `like` for acts, typed fields, then the word.
**Tested on Ex 2's duel the same day** (`examples/drafts/ex2-duel-*`):
every step made the duel longer, and what it needed was general words and
Link groups. So `define` waits: Ex 9, written in general words first, is
its next test (a spellbook with many spells); if step 3 comes back, it
builds on an archetype rather than a kind.

### Not in v0.6

- v0.7: `~ conflict` on scenes; facets.
- Dropped: automatic forks; `~ canon`; `refute`; `any` as a quantifier.
- (The duel moved into v0.6 on 20 Nov 2026: the encounter milestone,
  above.)

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
16; 4 (the stub as an oracle) is the platform's, now that milestone 17 gives
it the NIF; 2, 3, 5, 7 and 10
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
mix test                                     # the NIF (Elixir 1.15+)
for ex in examples/*.twee; do                # the stories' own tests
  inkjot test "$ex" "examples/tests/$(basename "$ex" .twee).tests.twee"
done
PROPTEST_CASES=5000 cargo test --test properties
UPDATE_IR=1 cargo test --test build          # after an intended IR change
```

Work happens on a branch per milestone, pushed early and often, with a pull
request into `main`.
