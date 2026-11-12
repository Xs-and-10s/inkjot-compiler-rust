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
- Names inside expressions. Each must be a builtin (`me`, `here`, `clock`), a
  declared id, a tag, or a `let` in scope. `me.knows.x` wants a clue and
  `me.holds.x` an item. A misspelling gets a suggestion:

  ```
  error: `stair_cleer` is not declared: …; did you mean `stair_clear`?
  ```

  A `let` is in scope in its own passage and in that passage's sub-Nibs. A
  Link's `with {…}` parameters are in scope in the passage it leads to.

## What it does not cover yet

- A bare word on the right of `==`, `!=` or `in` is taken as a value
  (`me.treat == hold`), so a misspelling there is not caught.
- Member names after the first (`door.opn`) are not checked.
- Free tags on declarations (`~ evidence`, `~ wide`) are accepted as written.
- v0.6 features (the duel: `encounter`, `pattern`, `spell`; and `retell`) are
  skipped and counted in one notice. They are not checked.
- Nothing is emitted. The JSON IR is the next milestone.

## Building

The compiler has no dependencies beyond the Rust standard library.

```
cargo build --release
cargo test
```

## Testing

Three kinds of test, all run by `cargo test`:

- **Unit tests** beside the code (`src/expr.rs`).
- **End-to-end tests** in `tests/check.rs`. They break a small valid file one
  edit at a time and assert the one error each edit must cause.
- **Property tests** in `tests/properties.rs`, using
  [proptest](https://github.com/proptest-rs/proptest). A property is a claim
  about every input: `check` never panics, every reported position lies inside
  the file, printing an expression and parsing it back gives the same tree.
  proptest generates inputs looking for a
  counterexample, shrinks any it finds, and saves it in
  `tests/properties.proptest-regressions` so it is retried on every run.

proptest is a dev-dependency: it is built for `cargo test` only and is not part
of the `inkjot` binary.

To search harder than the default 256 cases per property:

```
PROPTEST_CASES=5000 cargo test --test properties
```

When a property finds a bug, fix it and add a plain test for it in
`tests/check.rs`, so the reason is written down next to the fix.

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
| `src/expr.rs` | The expression parser and printer |
| `src/resolve.rs` | Name resolution inside expressions |
| `src/check.rs` | Statements, Links, bodies, and name resolution |
| `src/diag.rs` | Diagnostics and how they print |
| `src/json.rs` | Just enough JSON to read `StoryData` |
| `src/main.rs` | The `inkjot` command |

## Licence

MIT.
