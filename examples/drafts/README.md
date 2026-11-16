# Drafts for Inkjot v0.6

Examples written ahead of the language, the way Ex 1 to 3 were written
before the compiler existed. They do not pass `inkjot check` 0.5, and the
tests leave them out. Each one says what it needs.

## Ex 4: Littlejon (`ex4-littlejon.twee`)

A `story` in eight chapters, built to test Inquests. Chapter One is Mark's
first draft of *Darklands: Winter 1*, unchanged; the rest is an imagined
novella grown from it.

What `inkjot check` 0.5 refuses, which v0.6 has to add:

| Refused | Count | v0.6 |
|---|---|---|
| `format-version: 0.6.0` | 1 | the version |
| `state widow_freed: no` (and reading it) | 4 + 9 | `state` beside `fact` |
| `~ id free_the_widow` | 21 | optional Link ids |
| `~ inquest by storyteller ~ rejoin ch5` | 1 | Storyteller forks |
| `~ inquest`, `~ inquest for obey` | 8 notices | Inquests (flow D) |

`pending: neutral ~ by storyteller ~ fallback ch5 ~ timeout 5m` is accepted
as reserved.
