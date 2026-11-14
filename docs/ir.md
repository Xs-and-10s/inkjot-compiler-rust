# The Inkjot IR

`inkjot build story.twee -o story.json` compiles an Inklink to one JSON file.
A runtime reads that file and never parses Inkjot itself.

This describes IR format version `"2"`. The three files under `examples/ir/`
are the compiler's real output for the three examples, and a test keeps them
current.

## Top level

```json
{
  "ir": "2",
  "inkjot": "0.5.0",
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
{ "name": "hall:look", "tags": ["act"], "margin": false, "header": [ … ], "body": [ … ] }
```

`header` holds statements. `body` holds nodes. A margin passage (`[npc]`,
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
continuation. A paragraph is a list of parts. A part is a plain string or an
insert:

| Source | Part |
|---|---|
| `{present}` | `{"insert": "present"}` |
| `{embed nib: 'x'}` | `{"insert": "embed", "nib": "x"}` |
| `{list the_list ~ one per line}` | `{"insert": "list", "list": "the_list", "options": ["one per line"]}` |
| `{name input for: me.name}` | `{"insert": "name input", "args": "…"}` |
| `{leo.aka}` | `{"value": {"get": ["leo", "aka"]}}` |

**`modifier`**

```json
{ "type": "modifier", "parts": [ {"name": "if", "when": { … }}, {"name": "append"} ] }
```

Modifiers stay in order, as in the source: each applies until the next one.
`after` and `beat` carry `"time"`; `reveal` carries `"mode"`.

Author's notes (`[note]`, `[todo]`, `[fixme]`) and the text under them are
left out of the IR.

**`link`**

```json
{
  "type": "link",
  "id": "dining_table#4",
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
| `id` | The passage name, `#`, and the Link's position in it, counting from 1 |
| `lib` | Present when the Link lives in a `[lib]` passage that cells embed |
| `kind` | `act` when the Link uses an act; `go` for a plain move; or `switch`, `pass`, `inquest`, `release` |
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

## Statements

A statement is an object whose first key says what it is. Statements are
**sparse**: a field that was not written is absent, not `null`.

| First key | Source | Example |
|---|---|---|
| `declare` | `npc marsh: "Colonel Marsh" ~ …` | `{"declare": "npc", "id": "marsh", "label": "Colonel Marsh", …}` |
| `setting` | `cohort: 2..=5 ~ best 3..=4` | `{"setting": "cohort", "value": {range}, "best": {range}}` |
| `goal`, `reaction` | `goal flee: ~ toward platform ~ weight 6` | `{"goal": "flee", "toward": "platform", "weight": 6}` |
| `checkpoint` | `checkpoint cells: ~ on death in prison` | `{"checkpoint": "cells", "on_death_in": "prison", …}` |
| `inquest` | `inquest: "…" ~ wits 3..` | `{"inquest": "…", "stats": {"wits": {range}}}` |
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
| `fact` | `value`, an expression (`no` is `false`) |
| `stat` | `range` |
| `resource` | `value`, if it starts with a number |
| `fuse` | `start` |
| `way` | `from`, `to` |
| `combo` | `acts` |
| `relation` | `between` |
| `table` | `roll`, an expression (`d6 + bonus`); dice are names like `d6` |
| `map` | `size`, `[columns, rows]` |

### Fields from `~` clauses

Each clause is one phrase, and each phrase fills one field. When a key
repeats, lists extend and objects merge.

| Phrase | Field |
|---|---|
| `"…"` | `label`, or `description` if there is already a label |
| one word: `evidence`, `playable`, `loud` | added to `tags` |
| `in X`, `at X` | `in`, `at` |
| `held by X`, `belongs to X` | `held_by`, `belongs_to` |
| `holds a, b: "label", c as mark` | `holds`: `[{"id", "label"?, "mark"?}]` |
| `knows a`, `for a, b`, `shared by a` | `knows`, `for`, `shared_by` (lists) |
| `like X`, `with k: v`, `fact k: v` | `like`, `with` (object), `facts` (object) |
| `shown as "…"`, `formally "…"` | `shown_as`, `formally` |
| `aka "a", "b"` / `aka shuffled` | `aka` (list) / `aka_order` |
| `else absent`, `weight heavy`, `of 6`, `survives death` | `else`, `weight`, `of`, `survives` |
| `when E`, `open when E`, `done when E` | `when`, `open_when`, `done_when` |
| `hidden until E`, `fixed until E`, `offstage until E` | `hidden_until`, `fixed_until`, `offstage_until` |
| `watches here` / `watches here when E` | `watches: true` / `watches_when` |
| `then EFFECT` | `then` (list of effects) |
| `cost …`, `check E`, `min n`, `steps R`, `capacity n` | `cost` (list), `check`, `min`, `steps`, `capacity` |
| `requires …` | `requires` (lowered words) |
| `ends turn`, `one way`, `on tap` | `ends_turn`, `one_way`, `on_tap`: `true` |
| `learned from clue X`, `once per day` | `learned_from`, `once_per` |
| `choice proactive` (`reactive`, `inactive`) | `choice` |
| `back 1..=2` (on `rewind: echo`) | `back` |
| `moves a, b` / `moves none` / `moves 2 per chronon` | `moves` (list; `[]` for none) / `speed` |
| `asleep during R`, `wakes on loud`, `stays awake n` | `asleep_during`, `wakes_on`, `stays_awake` |
| `toward E`, `via X`, `pace X` | `toward`, `via`, `pace` |
| `max n`, `per character`, `refill each chronon` / `refill on checkpoint` | `max`, `per`, `refill`: `"chronon"` / `"checkpoint"` |
| `part of X`, `after X` | `part_of`, `after` |
| `culprit X`, `method X`, `methods a, b` | `culprit`, `method`, `methods` |
| `best R`, `advance x`, `turn limit 24h`, `turn ends when E` | `best`, `advance`, `turn_limit`, `turn_ends_when` |
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
| `me` | `"me"` |
| `yes`, `no` | `true`, `false` |
| `3`, `1.5` | `3`, `1.5` |
| `"text"` | `{"str": "text"}` |
| `door.open` | `{"get": ["door", "open"]}` |
| `me.knows.secret` | `{"knows": ["me", "secret"]}` |
| `me.holds.key` | `{"holds": ["me", "key"]}` |
| `me.is(rook)` | `{"is": ["me", "rook"]}` |
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

## What version 2 does not lower

Only one thing reaches `other`: the Ex 2 duel's spell vocabulary (`thaws ice`,
`absorbs spells`, `shields target from the next spell`, and four more), which
belongs to Inkjot v0.6. A test lists these seven phrases and fails if any other
phrase is left untyped.

Insert arguments for inputs, `accuse`, `reveal` and sounds are still text in
`args`, and v0.6 statements appear as `stretch` with their source.

## Changes from version 1

- Declarations and settings are typed fields instead of `clauses` and `words`.
- Statements are sparse: absent fields are left out.
- Goals, reactions, checkpoints and inquests have their own first key.
- Indexing (`hours[i]`) is `{"index": …}`, so `at` is free for the method
  `who.at(spot)`.
- `[test]` passages are left out.
