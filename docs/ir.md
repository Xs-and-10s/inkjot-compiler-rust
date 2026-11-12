# The Inkjot IR

`inkjot build story.twee -o story.json` compiles an Inklink to one JSON file.
A runtime reads that file and never parses Inkjot itself.

This describes IR format version `"1"`. The three files under `examples/ir/`
are the compiler's real output for the three examples, and a test keeps them
current.

## Top level

```json
{
  "ir": "1",
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

A statement is an object whose first key says what it is.

| First key | Source | Other keys |
|---|---|---|
| `declare` | `item key: "a key" ~ in hall` | `id`, `label`, `value`, `words`, `when`, `clauses` |
| `setting` | `mode: solo`, `goal flee:` | `value`, `words`, `when`, `clauses` |
| `effect` | `learn secret` | the effect; `clauses` if it has any |
| `let` | `let ready: me.knows.secret` | `value` (an expression, or `{"roll": "d6"}`), `when` |
| `expect` | `expect always: …` | `always`, `else` |
| `on` | `on enter cellar when …` | `timing`, `what`, `by`, `when`, `then`, `once`, `at_most`, `tags` |
| `stretch` | a v0.6 statement | `source`: the text as written |

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
| `hours[i]` | `{"at": ["hours", "i"]}` |

A method call puts its receiver first: `here.has(vell)` is
`{"has": ["here", "vell"]}`. The `in` operator is spelled `within` so it can
never be confused with a method named `in`. `but` is lowered as `and`.

## What version 1 does not lower

These are carried as text plus a `words` list, not as typed fields:

- config settings (`cohort: 2..=5 ~ best 3..=4`)
- most declaration clauses (`~ in dining`, `~ held by wren`, `~ moves walk, dash`)
- NPC agendas: `goal` and `reaction` appear as settings with clauses
- insert arguments for inputs, `accuse`, `reveal` and sounds (`args` is text)
- v0.6 features, which appear as `stretch` with their source text

A runtime can act on `words` without parsing, but it has to know the
vocabulary. Typing these is the next step for the IR.
