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

## Next

1. **On handlers and NPC agendas** in the interpreter, so Ex 1 and Ex 2 run
   their villains and Ex 2's Host speaks. See `docs/run.md` for the full list
   of what the interpreter does not do yet.
2. **Several Linkers at once**: simultaneous turns and clashes, for Ex 1.
3. **Typed insert arguments** (`{name input …}`, `{accuse …}`).
4. **Inkjot v0.6**: decided so far are `state` / `fact` / `clue`, structured
   facts with rules, and `i` beside `me` (see the working doc). Held until
   more ideas are in.

## How to resume

```
cargo test                                   # everything should pass
for ex in examples/*.twee; do                # the stories' own tests
  inkjot test "$ex" "examples/tests/$(basename "$ex" .twee).tests.twee"
done
PROPTEST_CASES=5000 cargo test --test properties
UPDATE_IR=1 cargo test --test build          # after an intended IR change
```

Work happens on a branch per milestone, pushed early and often, with a pull
request into `main`.
