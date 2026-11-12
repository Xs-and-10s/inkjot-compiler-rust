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
| 3 | IR version 2: declarations and settings as typed fields | #5 |

## Next

1. **`[test]` passages.** `given:` sets state, `do:` takes Links in order,
   `expect:` checks the result. Needs a small interpreter over the IR, which
   is also the first piece of a runtime.
2. **Typed insert arguments** (`{name input …}`, `{accuse …}`), the last text
   left in the IR outside v0.6.
3. **Inkjot v0.6**: the duel, `retell`, and the language questions listed in
   the working doc.

## How to resume

```
cargo test                                   # everything should pass
PROPTEST_CASES=5000 cargo test --test properties
UPDATE_IR=1 cargo test --test build          # after an intended IR change
```

Work happens on a branch per milestone, pushed early and often, with a pull
request into `main`.
