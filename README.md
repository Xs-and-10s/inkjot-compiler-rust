# inkjot

The compiler for **Inkjot**, the Twee 3 dialect behind Inklinks. An Inklink is a
choice-based work: a story, a game, or something between. Inkjot keeps the
standard Twee 3 container (`:: passage [tags]`) and defines its own language for
what goes inside a passage.

This compiler targets **Inkjot v0.5**.

## Status

Milestone 1: `inkjot check`. It reads a `.twee` file and reports what is wrong
with it. It does not yet emit the JSON IR.

```
$ inkjot check examples/ex1-the-night-train.twee
examples/ex1-the-night-train.twee: ok (74 passages, 58 declarations, 31 Links, 0 speeches, 94 expressions)
```

An error shows the file, line and column, and the line itself:

```
error: `notebok` is not a declared item
  --> story.twee:249:8
    |
249 | ~ take notebok
    |        ^
```

The exit status is 0 when every file passes, 1 when any has errors, and 2 for
bad usage.

## What `check` covers

- The Twee container: passage headers, tags, duplicate names, sub-Nibs whose
  parent is missing.
- `StoryData`: valid JSON, `format` is `Inkjot`, the version is 0.5.x, the
  start passage exists.
- The config and the margins (`[config]`, `[npc]`, `[character]`, `[place]`,
  `[archetype]`): every statement starts with a known keyword; declarations
  have a name and a colon; `kind`, `mode`, `cohort` and `pov` agree.
- Names: Link targets, `next:`, `{embed nib}`, `then play`, `start:` and
  `starts:` name real passages. Items, clues, fuses, tables, lists, places,
  ways, archetypes, stats and speakers are declared before they are used.
- Links: `> [[label->target]]` and its `~` clauses, each of which must be a
  Link clause or a declared act or combo.
- Bodies: `@speaker (direction)` cue lines, `[modifiers]` and their pairing,
  `{inserts}`, and `~` continuations.
- Events: `on [before|after] event … when … ~ then …`, with `upon` as a synonym.
- Expressions, wherever they appear: after `when`, in `[if …]`, in `let` and
  `expect`. `but` is a synonym for `and`.

## What it does not cover yet

- Names inside expressions (`me.knows.secret`) are parsed, not resolved.
- Free tags on declarations (`~ evidence`, `~ wide`) are accepted as written.
- v0.6 features (the duel: `encounter`, `pattern`, `spell`; and `retell`) are
  skipped and counted in one notice. They are not checked.
- Nothing is emitted. The JSON IR is the next milestone.

## Building

No dependencies beyond the Rust standard library.

```
cargo build --release
cargo test
```

## Examples

`examples/` holds three complete Inklinks, which are also the test fixtures:

| File | What it is |
|---|---|
| `ex1-the-night-train.twee` | A multiplayer time-travel mystery on a train |
| `ex2-the-ninefold-cell.twee` | A solo two-hero escape room, with a v0.6 duel |
| `ex3-the-fall-of-the-old-regime.twee` | A dialogue comedy at a wake, told by cats |

## Layout

| Path | Role |
|---|---|
| `src/twee.rs` | Splits a file into passages |
| `src/expr.rs` | The expression parser |
| `src/check.rs` | Statements, Links, bodies, and name resolution |
| `src/diag.rs` | Diagnostics and how they print |
| `src/json.rs` | Just enough JSON to read `StoryData` |
| `src/main.rs` | The `inkjot` command |

## Licence

MIT.
