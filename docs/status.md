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
| 11 | Inquests | #14 |

## Next

1. **What Ex 1 still needs to play whole**: NPCs who see acts
   (`self.saw(dash)`, so Vell gives chase), sleepers woken by a loud move
   (`~ wakes on loud`), a Nib's own `moves:` (the mail car allows only
   walking), and echoes that see the NPCs where they were then.
2. **The rest of the input family**, when a story needs one. None does yet.
3. **Inkjot v0.6**: decided so far are `state` / `fact` / `clue`, structured
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
