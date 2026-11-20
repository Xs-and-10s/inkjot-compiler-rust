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

## Next

The plan agreed on 20 Nov 2026, after the platform's first slice:

1. **Milestone 15, the engine API.** Done.
2. **Milestone 16, engine events.** Done.
3. **Milestone 17, the Rustler NIF.** Done. The platform's side:
   `Inklinks.Engine.Rust` over `Inkjot.Engine`, its Ex 3 scenarios run
   against it, and the stub kept as an oracle (follow-up 4).
4. **Milestone 18, IR 3 and the first part of v0.6**, in three pull
   requests (see "Inkjot v0.6" below for what is in each).
5. **Ex 6 to 9**, ideas first, then drafts:
   - Ex 6, a spy-noir `story-game` that leans on Inklog: two knowledge
     bases per character, and integrity constraints for facts that rule
     each other out.
   - Ex 7, Mafia, the card game.
   - Ex 8, a `story` written for Inquests, in place of Ex 4.
   - Ex 9, a modern Zork-like `game`.
6. **The rest of v0.6** (below).

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
- `on turn begins`, `on turn ends` (v0.5's `on turn` a synonym).
- **Periods:** `period season: spring, summer, autumn, winter ~ 3 each`,
  `period year ~ 4 seasons each`, uneven lengths (`dawn 1, day 4`);
  `clock.season`, `on season begins summer`.
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
  entries, paragraphs and Link labels. Redacted text is shown as a bar and
  never reaches the page; the IR carries its length only.

### Not in v0.6

- v0.7: `~ conflict` on scenes; facets.
- Dropped: automatic forks; `~ canon`; `refute`; `any` as a quantifier.
- **The duel** (Ex 2's spell vocabulary: `thaws ice`, `absorbs spells` and
  five more) is not planned; it stays untyped in the IR and the
  interpreter refuses it as not supported.

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
