# The Inkjot IR

`inkjot build story.twee -o story.json` compiles an Inklink to one JSON file.
A runtime reads that file and never parses Inkjot itself.

This describes IR format version `"3"`, for Inkjot v0.6. It is being built
in milestone 18; what changed from version 2 is at the end. The three files
under `examples/ir/` are the compiler's real output for the three examples,
and a test keeps them current.

**Unknown fields are ignored.** A consumer ignores an object field it does
not know, and the compiler never changes the meaning of an existing field
without a new IR version. So a new optional field is an addition, not a new
version, and a runtime written for an earlier IR 3 keeps working.

## Top level

```json
{
  "ir": "3",
  "inkjot": "0.6.0",
  "title": "The Night Train",
  "ifid": "…",
  "start": "cast",
  "config": [ …statements… ],
  "acts": { "walk": { … }, "take": { … } },
  "passages": [ { … }, { … } ]
}
```

| Key | What it holds |
|---|---|
| `ir` | The version of this format |
| `inkjot` | The language version the source declared |
| `title`, `ifid`, `start` | From `StoryTitle` and `StoryData` |
| `config` | The statements of the `[config]` passage |
| `acts` | Every declared act and combo, with the fields Links inherit |
| `passages` | Every other passage, in source order |

## Passages

```json
{ "name": "ch2_alt", "title": "The Millpond", "tags": ["act"], "margin": false, "header": [ … ], "body": [ … ] }
```

`title` is present when the passage says `title: "…"` in its header: a name
for people, where the platform would otherwise show the passage's id. The
statement itself is not repeated in `header`. `header` holds statements.
`body` holds nodes. A margin passage (`[npc]`,
`[character]`, `[place]`, `[archetype]`) is all header.

A passage with a doc comment (a `(((…)))` block straight after its `::`
line) has a `doc` field after `tags`, for Inkwell. Other comments, `((…))`,
never reach the IR.

## Body nodes

Each node has a `type`.

**`prose`** and **`speech`**

```json
{ "type": "speech", "who": "leo", "direction": "whisper",
  "paragraphs": [ ["Joseph."] ] }
```

`paragraphs` has one entry per paragraph: the cue line, then each `~`
continuation. A paragraph is a list of parts. A part is a plain string, a
styled run, or an insert. Escapes are already resolved: `\{` arrives as `{`,
`\*` as `*`.

**Styled runs.** Emphasis is lowered here, so no runtime parses asterisks:

```json
["He opened his mouth to say ", {"style": ["italic"], "parts": ["I don’t remember"]}, "."]
```

`style` is `["italic"]`, `["bold"]` or `["bold", "italic"]`, for `*x*`,
`**x**` and `***x***`. `parts` may hold anything a paragraph holds, inserts
and other runs included. A run of asterisks pairs with the nearest open run
of the same length; one that pairs with nothing stays a literal `*`.

| Source | Part |
|---|---|

| Source | Part |
|---|---|
| `{present}` | `{"insert": "present"}` |
| `{embed nib: "x"}` | `{"insert": "embed", "nib": "x"}` |
| `{list the_list ~ one per line}` | `{"insert": "list", "list": "the_list", "options": ["one per line"]}` |
| `{name input for: me.name, max: 24}` | `{"insert": "name input", "args": {"for": {"get": ["me", "name"]}, "max": 24}}` |
| `{accuse who: suspect, how: scenario.methods, proof: proof_who and proof_how, revote: 1}` | `{"insert": "accuse", "args": {"who": "suspect", "how": {"get": […]}, "proof": {"and": ["proof_who", "proof_how"]}, "revote": 1}}` |

The jury's and the inputs' arguments are typed: a number is a number,
`'quoted'` text is text, and anything else is an expression. Other inserts
with arguments (`{reveal link: …}`, `{sound effect: …}`) keep them as text
for now.
| `{leo.aka}` | `{"value": {"get": ["leo", "aka"]}}` |
| `{all leo.aka}` | `{"value": {"get": ["leo", "aka"]}, "all": true}`: every name, not one picked |

**`modifier`**

```json
{ "type": "modifier", "parts": [ {"name": "if", "when": { … }}, {"name": "append"} ] }
```

Modifiers stay in order, as in the source: each applies until the next one.
`after` and `beat` carry `"time"`; `reveal` carries `"mode"`.

Author's notes (`[note]`, `[todo]`, `[fix]`) and the text under them are
left out of the IR.

**`link`**

```json
{
  "type": "link",
  "id": "dining_table->dining_table:letter",
  "id_from": "target",
  "lib": "dining_table",
  "label": "Ask Mrs. Ardent about her letter",
  "target": "dining_table:letter",
  "kind": "act",
  "act": "ask",
  "arg": null,
  "cost": [ {"vults": 1}, {"vults": 2, "when": {"is": ["me", "rook"]}} ],
  "scope": "private",
  "capacity": null,
  "clash": "none",
  "check": null,
  "when": null,
  "hidden_until": {"knows": ["me", "two_hands"]},
  "for": null,
  "effects": [],
  "ends_turn": false,
  "overrides": ["cost"]
}
```

| Field | Meaning |
|---|---|
| `id` | The Inker's `~ id` when written; otherwise the passage, `->`, and the target (`ch2->ch3`), with `#2`, `#3` for a second and third Link to the same target. Never the Link's position, so reordering Links keeps their ids |
| `id_from` | `inker` or `target`: where the id came from |
| `label` | A list of parts, like a paragraph: styled runs and values included |
| `lib` | Present when the Link lives in a `[lib]` passage that cells embed |
| `kind` | `act` when the Link uses an act; `go` for a plain move; or `switch`, `pass`, `hint`, `release`. A fork keeps its own kind: being a fork is the `inquest` field |
| `act`, `arg` | The act or combo, and its argument (`take cufflink`) |
| `cost` | The act's costs, then the Link's. **The last entry whose `when` holds applies.** `vults` is a number or `"rest"`; `per` and `from` appear when written |
| `scope`, `capacity`, `check`, `ends_turn` | The Link's own value, else the act's |
| `clash` | The Link's own value; else the config's `default clash` for a shared Link; else `"none"` |
| `when` | The Link's conditions, joined with `and` when there are several |
| `for` | Who may take it, as a list, or `null` for anyone |
| `effects` | Its `then` clauses, as effects |
| `overrides` | Which inherited fields the Link set itself |

Other clauses appear as extra keys when used: `with` (an object of
parameters), `become`, `order`, `by`, `chance`, `requires`, `once`, `loud`,
`quiet`.

**Forks.** A fork is a Link with an `inquest` field. A Link without one is
an offered Link.

```json
{ "type": "link", "id": "swim", "label": ["Swim across in the dark"], "target": "bank", "kind": "go",
  "inquest": { "stands_in_for": "pay", "from": ["bank"], "credit": "inker" }, "…": "…" }
```

| Field | Meaning |
|---|---|
| `stands_in_for` | The id of the offered Link this fork stands in for, in the same passage. Always written out: `~ inquest for pay`, or by default the offered Link just above |
| `from` | Later passages it may also be asked for from: `~ inquest from bank` |
| `credit` | Who wrote it as it stands: `inker` (the default), `monkeypaw`, `ghostwriter` or `storyteller`, from `~ credit …`. Attribution only |
| `pending` | Only on a slot not written yet, filled at runtime: `{"by": "storyteller", "rejoin": ["bank"], "brief": "…"?}`, from `~ inquest by storyteller ~ rejoin bank ~ brief name`. `by` names a role, never a person |

Several forks may stand in for one offered Link; their order is the order
written. A fork is offered only while its offered Link is, and its own
conditions must hold too. `brief` names a `brief` declaration (its fields
come later; rule 0 lets them be added). The checker refuses forks in
`mode: cohort`.

**Analysis.** Tags that never change play ride in an `analysis` object:

| Source | `analysis` |
|---|---|
| `~ beat turn` | `"beat": "turn"` (`opening`, `inciting`, `turn`, `midpoint`, `crisis`, `climax`, `resolution`) |
| `~ weighs mercy, obedience` | `"weighs": ["mercy", "obedience"]`: declared `theme`s |
| `~ arc ixe rises` | `"arc": {"who": "ixe", "way": "rises"}` (`rises`, `falls`, `holds`) |
| `~ stakes other` | `"stakes": "other"` (`self`, `other`, `world`) |
| `~ plants book`, `~ pays off book` | `"plants": ["book"]`, `"pays_off": ["book"]` |
| `~ predict 40%` | `"predict": 40` |
| `~ lasting` | `"lasting": true` |
| `~ tone dark`, `~ reveals debts` | `"tone": "dark"`, `"reveals": "debts"` |

`~ choice` stays its own field, as before.

## Statements

A statement is an object whose first key says what it is. Statements are
**sparse**: a field that was not written is absent, not `null`.

| First key | Source | Example |
|---|---|---|
| `declare` | `npc marsh: "Colonel Marsh" ~ …` | `{"declare": "npc", "id": "marsh", "label": "Colonel Marsh", …}` |
| `setting` | `cohort: 2..=5 ~ best 3..=4` | `{"setting": "cohort", "value": {range}, "best": {range}}` |
| `goal`, `reaction` | `goal flee: ~ toward platform ~ weight 6` | `{"goal": "flee", "toward": "platform", "weight": 6}` |
| `checkpoint` | `checkpoint cells: ~ on death in prison` | `{"checkpoint": "cells", "on_death_in": "prison", …}` |
| `hint` | `hint: "…" ~ wits 3..` | `{"hint": "…", "stats": {"wits": {range}}}` |
| `effect` | `learn secret` | the effect; `clauses` if it has any |
| `let` | `let ready: me.knows.secret` | `value` (an expression, or `{"roll": "d6"}`), `when` |
| `expect` | `expect always: …` | `always`, `else` |
| `on` | `on enter cellar when …` | `timing`, `what`, `by`, `when`, `then`, `once`, `at_most`, `tags` |
| `stretch` | a v0.6 statement | `source`: the text as written |

### What follows a declaration's colon

When there is no value, there is no colon: `archetype statue ~ lifeless`.

| Declares | Field |
|---|---|
| most things | `label`, from the quoted text |
| `state` | `value`, an expression (`no` is `false`), and `type`: `boolean`, `integer`, `decimal`, `text`, `id` (a name, or `nothing`), `range`, `list`, or `expression` |
| `fact` | `label`, the fact's text; `~ clue` and the other tags in `tags` |
| `stat` | `range` |
| `resource` | `value`, if it starts with a number |
| `fuse` | `start` |
| `way` | `from`, `to` |
| `combo` | `acts` |
| `relation` | `between` |
| `theme` | `label`: `theme mercy: "Who deserves saving"` |
| `brief` | `label`, and fields later |
| `table` | `roll`, an expression (`d6 + bonus`); dice are names like `d6` |
| `map` | `size`, `[columns, rows]` |

### Fields from `~` clauses

Each clause is one phrase, and each phrase fills one field. When a key
repeats, lists extend and objects merge.

| Phrase | Field |
|---|---|
| `"…"` | `label`, or `description` if there is already a label. Quoted text is a plain string, or, when it has braces, `{"text": ["Keeper of ", {"value": "here"}]}`, worked out each time it is shown. Text that is read (a fact's text, a description, a list entry, a hint, a `say` line) also takes emphasis, so its parts may be styled runs; a name (`label`, `shown as`) and a value (`set x: "…"`) never do |
| one word: `evidence`, `playable`, `loud`, `track` | added to `tags`. `track` marks a value whose changes the engine reports |
| `in X`, `at X` | `in`, `at` |
| `held by X`, `belongs to X` | `held_by`, `belongs_to` |
| `holds a, b: "label", c as mark` | `holds`: `[{"id", "label"?, "mark"?}]` |
| `knows a`, `for a, b`, `shared by a` | `knows`, `for`, `shared_by` (lists) |
| `like X`, `with k: v`, `state k: v` | `like`, `with` (object), `state` (object) |
| `shown as "…"`, `formally "…"` | `shown_as`, `formally` |
| `aka "a", "b"` / `pick cycle` | `aka` (list) / `aka_pick`: `shuffled` (the default), `cycle`, `random` or `first` |
| `else absent`, `weight heavy`, `of 6`, `survives death` | `else`, `weight`, `of`, `survives` |
| `when E`, `open when E`, `done when E` | `when`, `open_when`, `done_when` |
| `hidden until E`, `fixed until E`, `offstage until E` | `hidden_until`, `fixed_until`, `offstage_until` |
| `watches here` / `watches here when E` | `watches: true` / `watches_when` |
| `then EFFECT` | `then` (list of effects) |
| `cost …`, `check E`, `min n`, `steps R`, `capacity n` | `cost` (list), `check`, `min`, `steps`, `capacity` |
| `requires …` | `requires` (lowered words) |
| `ends turn`, `one way`, `on tap` | `ends_turn`, `one_way`, `on_tap`: `true` |
| `learned from X`, `once per turn` / `once per day` | `learned_from`, `once_per`: `"turn"` / `"day"` |
| `choice proactive` (`reactive`, `inactive`) | `choice` |
| `back 1..=2` (on `rewind: echo`) | `back` |
| `moves a, b` / `moves none` / `moves 2 per chronon` | `moves` (list; `[]` for none) / `speed` |
| `asleep during R`, `wakes on loud`, `stays awake n` | `asleep_during`, `wakes_on`, `stays_awake` |
| `toward E`, `via X`, `pace X` | `toward`, `via`, `pace` |
| `max n`, `per character`, `refill each chronon` / `refill on checkpoint` | `max`, `per`, `refill`: `"chronon"` / `"checkpoint"` |
| `part of X`, `after X` | `part_of`, `after` |
| `culprit X`, `method X`, `methods a, b` | `culprit`, `method`, `methods` |
| `best R`, `advance x`, `turn limit 24h`, `turn ends when E`, `day n` | `best`, `advance`, `turn_limit`, `turn_ends_when`, `day` |
| `switch freely`, `lead X`, `order each other within earshot` | `switch`, `lead`, `order_within` |
| `timed 40ms`, `beat 1s`, `shows visited` | `timed`, `beat`, `shows` |
| `on death in X`, `keeps a, b`, `resets a, b` | `on_death_in`, `keeps`, `resets` |
| `to X` / `to a for x, b for y` | `to`: a string / an object keyed by who |
| `unseen by X` | `unseen_by` |
| a stat or resource and a value: `wits 4`, `hp 2` | `stats`: `{"wits": 4, "hp": 2}` |
| a range: `0..=3` | `range` |
| a table row: `1..=2: then kill blocker` | `rows`: `[{"range", "then"}]` |
| a map row: `a | b | c` | `rows`: `[["a", "b", "c"]]` |
| anything else | `other`: `[{"text", "words"}]` |

The vocabulary lives in one table, `PATTERNS` in `src/ir/clauses.rs`. When
several phrases match, the longest wins.

### Effects

```json
{ "do": "burn", "args": ["nerve", 1], "when": {"not": {"get": ["vell", "shoes_seen"]}} }
{ "do": "set", "target": {"get": ["vell", "cornered"]}, "value": true, "when": null }
{ "do": "say", "who": "hask", "text": "Again! From the top!", "when": null }
```

`args` are the effect's words in order. A number is a number, a quoted string
is `{"str": "…"}`, and anything that reads as an expression is lowered
(`self.deaths` becomes `{"get": ["self", "deaths"]}`). Joining words such as
`to`, `on` and `as` stay as strings. `once` becomes `"once": true`.

### Clauses

The `~` clauses of a declaration or setting:

| Shape | Source |
|---|---|
| `{"then": effect}` | `~ then set vell.cornered: yes` |
| `{"label": "…"}` | `~ "a snap of heat and light"` |
| `{"clause": "hidden", "until": expr}` | `~ hidden until chute_found` |
| `{"row": range, "then": effect}` | `~ 1..=2: then kill blocker` |
| `{"clause": "in dining", "words": ["in", "dining"]}` | everything else |

## Expressions

| Source | JSON |
|---|---|
| `me`, and `i`, `you`, `myself`, `yourself` | `"me"` |
| `yes`, `no` | `true`, `false` |
| `3`, `1.5` | `3`, `1.5` |
| `"text"` | `{"str": "text"}` |
| `"Keeper of {here}"` | `{"text": ["Keeper of ", {"value": "here"}]}` |
| `if lied then "a" else "b"` | `{"if": ["lied", {"str": "a"}, {"str": "b"}]}` |
| `plural(n, "card")`, `capital(x)` | `{"plural": ["n", {"str": "card"}]}`, `{"capital": ["x"]}` |
| `door.open` | `{"get": ["door", "open"]}` |
| `me.knows.secret`, `i.know.secret` | `{"knows": ["me", "secret"]}` |
| `me.holds.key`, `i.hold.key` | `{"holds": ["me", "key"]}` |
| `me.is(rook)`, `i.am(rook)` | `{"is": ["me", "rook"]}` |
| `count(linker, in: here)` | `{"count": ["linker"], "with": {"in": "here"}}` |
| `a and b`, `a or b` | `{"and": [a, b]}`, `{"or": [a, b]}` |
| `not a`, `-a` | `{"not": a}`, `{"neg": a}` |
| `a == b` (also `!= >= <= > <`) | `{"==": [a, b]}` |
| `a + b` (also `- * / mod`) | `{"+": [a, b]}` |
| `x in 2..=4` | `{"within": ["x", {"range": [2, 4], "inclusive": true}]}` |
| `3..` | `{"range": [3, null], "inclusive": false}` |
| `[a, b]` | `{"list": ["a", "b"]}` |
| `hours[i]` | `{"index": ["hours", "i"]}` |

A method call puts its receiver first: `here.has(vell)` is
`{"has": ["here", "vell"]}`. The `in` operator is spelled `within` so it can
never be confused with a method named `in`. `but` is lowered as `and`.

## What version 3 does not lower yet

Only one thing reaches `other`: the Ex 2 duel's spell vocabulary (`thaws ice`,
`absorbs spells`, `shields target from the next spell`, and four more), which
belongs to Inkjot v0.6. A test lists these seven phrases and fails if any other
phrase is left untyped.

Insert arguments for inputs, `accuse`, `reveal` and sounds are still text in
`args`, and v0.6 statements appear as `stretch` with their source.

## Changes from version 2

Inkjot v0.6, milestone 18:

- Unknown fields are ignored (above).
- `fact` (a value) is now `state`, with a `type`; `fact` is now knowledge
  (v0.5's `clue`), with a `label`; `clue` is a tag. An item's `~ fact k: v`
  is `~ state k: v`, in a `state` field.
- Link ids come from the Inker's `~ id` or from passage and target, with
  `id_from`; never from position.
- Passages may have a `title`.
- `[fixme]` is `[fix]`.
- Emphasis is lowered to styled runs; Link labels are lists of parts.
- Quoted text with braces is `{"text": [parts]}`, in fields and in
  expressions; `if … then … else …`, `plural` and `capital` are new.
- `aka_order` is `aka_pick`; `{all x.aka}` carries `"all": true`.
- A fork is a Link with an `inquest` field (`stands_in_for`, `from`,
  `credit`, `pending`); `kind: "inquest"` is retired. Analysis tags ride
  in `analysis`. `theme` and `brief` are declarations; `pending:` is a
  reserved header statement.
- `reveal x` is an effect (`{"do": "reveal", "args": ["x"]}`); the
  `reveal:` setting is still the text's pacing.
- `i`, `you`, `myself`, `yourself` are lowered as `me`; `know`, `hold`, `am`,
  `are` as `knows`, `holds`, `is`.

## Changes from version 1

- Declarations and settings are typed fields instead of `clauses` and `words`.
- Statements are sparse: absent fields are left out.
- Goals, reactions, checkpoints and hints have their own first key.
- Indexing (`hours[i]`) is `{"index": …}`, so `at` is free for the method
  `who.at(spot)`.
- `[test]` passages are left out.
