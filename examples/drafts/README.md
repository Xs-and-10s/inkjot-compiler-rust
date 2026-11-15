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

## Ex 5: Thistledown (`ex5-thistledown.twee`)

A solo `story-game`, built to test v0.6's rules. Nine beds on a 3 by 3 map,
an old gardener's half-blank almanac, four gnomes, a rival, his cat, and a
Harvest Fair after nine months. The story is one fragment for each gnome who
comes to trust you.

**Two truths.** Every fact's meaning (`~ means {{ … }}`) holds in the
garden from the start, and in your almanac once you learn the fact. Rules
hold in the garden always, and in your almanac once known (`~ known`,
`~ learned from pip_lore`). `~ world only` and `~ almanac only` narrow a rule
to one side.

**Negation.** `not` is stratified: a rule may say `not {{ B shaded }}` only
if everything that concludes `shaded` sits in an earlier layer, so nothing
depends on its own negation; and every variable in a `not` also appears in
a positive condition. Only `world only` rules may say `not`. The almanac
deals in observed opposites instead (`observe {{ here gets ? }}`), which
makes it overconfident on purpose: its `hopes` rule knows nothing of slugs
until Old Sorrel teaches `slug_risk`.

What it asks of v0.6, beyond Ex 4's list:

| Construct | Example |
|---|---|
| Rules, and where they hold | `rule sunny: … and not {{ B shaded }} => {{ B gets sun }} ~ world only` |
| Learnable rules | `rule mirror: … ~ learned from pip_lore` |
| One value per subject | `verb gets ~ one value`, so `observe` replaces what it replaces |
| Changing the garden | `assert {{ here composted }}`, `retract {{ here soil ? }}`, `assert … until next chronon` |
| Asking each truth | `{{ c blessed }}` (the garden), `me.knows {{ P suits here }}` (the almanac), `any P: me.knows {{ P thrives }} and not {{ P thrives }}` |
| Showing the almanac | `{known about: here}` |
| Seasons | `clock: 0..=8 ~ season 3 ~ seasons spring, summer, autumn`, `on season summer`, `clock.season` |
| After the NPCs move | `on dawn` (where `turn` comes before them) |
| Making things | `recipe lantern: jam_jar, candle_end ~ make` |
| A margin for rules | `:: garden_rules [rules]` |
| A relation with every member of a kind | `relation trust: you, each gnome`, `trust with self` |
| Picking a place by a rule | `one(bed, unwatched, where: {{ it has plant }})` |

**Found while writing it.** `inkjot check` 0.5 reads `{{ P in B }}` in a
margin as an insert, and `{one {{ ? prizewinning }}}` cannot be read at all.
v0.6 needs `{{` and `}}` as tokens of their own, lexed before inserts.
