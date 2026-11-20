# `define`: an Inker's own words (proposal)

Proposed 20 Nov 2026. **Not decided.** This note records the design, one
example written both ways, and how the two compare.

## The question

Can an Inker add words that belong to one Inklink (a `spell`, a `cast`) on
top of the words every Inklink shares (an `act`, `take`), without the
engine learning them? Milestone 18d took the last two such words (`vults`,
`hushed`) out of the engine. `define` would let them back into the source
safely: the compiler turns them into general words, and the engine only
ever sees those.

## The design

A defined word is a new kind of declaration built on a general one. It is
written as a passage, like an `[archetype]`:

```
:: spell [define act]
verb: cast
field power: number
field polarity: harm, help ~ default harm
field delay: duration ~ default 0s
field hops: number ~ default 0
field falloff: number ~ default 0
--
act self: threat ~ at enemy here ~ ends turn
~ spreads self.hops to nearest enemy ~ every 0.3s
~ then after self.delay: wound target self.power - self.falloff * hop ~ when self.polarity == harm
~ then after self.delay: raise target.hp self.power ~ when self.polarity == help
```

How to read it:

- `[define act]`: `spell` is a new word, and a spell is an act. Everything
  an act can do, a spell can do: costs, `once`, `requires`, Links.
- `verb: cast`: the word the platform shows for taking one ("Roland casts
  chain lightning"). The engine's action is still taking a Link.
- `field …`: what each spell says about itself, with a type (`number`,
  `duration`, a list of words, a kind, text) and a default. A field with no
  default must be given.
- Below `--`: one act, written the way any act is written, named `self`.
  Every spell gets these clauses. The fields are read as `self.power`, the
  same `self` an NPC's margin uses. Nothing is pasted in as text.

Then:

```
spell lightning_bolt: "a single bolt" ~ cost 20 ~ power 6 ~ delay 0.5s
spell chain_lightning: "a bolt that leaps to the next foe, and the next" ~ cost 35
~ like lightning_bolt ~ hops 2 ~ falloff 2
```

`like lightning_bolt` gives chain lightning the bolt's fields. Chain
lightning "shoots a lightning bolt" because it is one, plus two hops.

**What the compiler does.** A spell is lowered to an act: its clauses
merged with the `[define]` passage's, its fields kept as values on it, and
`"defined_as": "spell"` added so the platform can group spells in a
spellbook or show the verb. The engine never meets the word `spell`.

**Rules the checker keeps:**

- A defined word builds on a general kind only (`act`, `item`, `npc`,
  `place`, `state`…), not on another defined word.
- It may not reuse a declaration, effect, setting or reserved word.
- `define` names things. It cannot add an effect, a condition or a new way
  to run: those need the engine, so they are general words or nothing.
- Errors name both words: "`chain_lightning` (a spell, which is an act):
  `hops` must be a number".
- `inkjot expand` prints the source with every defined word replaced by
  what it means, for teaching, for review, and for generated Inquests.

## Chain lightning in general words only

```
act lightning_bolt: cost 20 ~ threat ~ storm ~ at enemy here ~ ends turn
~ "a single bolt"
~ then after 0.5s: wound target 6 - 2 * hop

combo chain_lightning: lightning_bolt ~ cost 35
~ "a bolt that leaps to the next foe, and the next"
~ spreads 2 to nearest enemy ~ every 0.3s
```

`hop` is 0 for a bolt cast alone, so the bolt is written once and the
chain is a combo of it. A combo of one act is still a combo.

## What both versions need

`define` does not make anything possible that was not possible already.
It names things that general words already say. So both versions need the
same new general words, which belong to the encounter milestone:

| Word | Meaning |
|---|---|
| `threat` (a tag) | Something incoming that counters may answer: a ward can absorb it, a mirror deflect it |
| `~ at <kind filter>` | The act has a target, chosen by the Linker from what fits: `target` in its effects |
| `~ then after <duration>: …` | The effect lands later, in the encounter's time |
| `~ spreads N to nearest <filter> ~ every <duration>` | After landing, the act lands again on the nearest one that fits, from the last one hit, never the same one twice, up to N more times. `hop` counts: 0, 1, 2. It stops early when nothing fits. Each hop is its own threat, so a ward absorbs its own hop; `~ stops when blocked` ends the chain there. Paid for once |

`spreads` is general on purpose. A fire spreading from room to room, a
plague, or a rumour among the gnomes of Ex 7 uses the same word.

## Comparison

| | `define` | General words |
|---|---|---|
| Two spells | fourteen lines | six lines |
| Twelve spells (Ex 2 has eleven) | one shared passage, then one line each | every spell repeats `threat ~ at … ~ ends turn` |
| Changing every spell at once (say, all can be interrupted) | one line in the passage | every spell, by hand |
| Reads like the fiction | yes: `spell`, `cast`, `power` | partly: tags (`storm`) and descriptions carry the fiction |
| A newcomer reading the source | must find the `[define]` passage first | knows everything already |
| What the checker can catch | typed fields: `polarity harmful` is an error; a spell with no power is an error | only what any act can get wrong |
| The platform | groups by `defined_as`, shows the verb | groups by tag, at best |
| Generated Inquests | the generator needs the Inklink's words as well as Inkjot's (`inkjot expand` helps) | nothing extra |
| The engine | unchanged | unchanged |
| The compiler | lowering and merging, field types, two-word errors | nothing new |
| The risk | it grows into macros: templates, text pasted in, words that hide control flow | the source grows long when many things share a shape |

The sketch from 20 Nov, piece by piece:

- `name ?(1, fixed, final)`: the id already does this.
- `polarity one(positive|negative)`: a field, `harm, help`.
- `effect …` and `enact = cast <<name>> at ?target => after <<delay>>
  trigger <<effect>>`: ordinary `~ then` clauses on `act self`, reading
  `self.delay`. A second way to write effects, with text pasted into
  `<<…>>`, would give errors that point at the pasted result, not at what
  the Inker wrote.
- `cost … in vults`: acts already cost, and `default cost: vults` (18d)
  says in what.
- `requires <<target>>`: `requires` already means "holds something", so a
  target is `~ at`.
- `lasts`: a field of type duration, used as `~ until self.lasts` on a
  spell that leaves something behind, such as a ward.

## Recommendation

Build it in three steps, and stop when a step is not needed:

1. **With the encounter milestone:** the general words above (`threat`,
   `at`, `after`, `spreads`, `hop`), and `like` for acts (`act x: ~ like
   spell`), using an `[archetype]` passage. That alone gives most of what
   `define` gives: shared clauses, one place to change them.
2. **Fields** on those archetypes, typed and checked, read as `self.x`.
3. **The word and the verb**: `spell x:` in place of `act x: ~ like
   spell`, `verb: cast`, `defined_as` in the IR, `inkjot expand`.

Test each step by writing Ex 2's duel both ways, as above. If step 1 makes
the duel short and clear, steps 2 and 3 can wait for v0.7.
