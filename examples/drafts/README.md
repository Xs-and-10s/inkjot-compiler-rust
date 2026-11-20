# Drafts for Inkjot v0.6

Examples written ahead of the language, the way Ex 1 to 3 were written
before the compiler existed. They do not pass `inkjot check` 0.5, and the
tests leave them out. Each one says what it needs.

## Ex 4: withdrawn

Ex 4 was a novella built on a chapter of an unpublished novel. It has been
taken out of this public repository (20 Nov 2026); a new `story` written
for Inquests will take its place. What it taught the design (forks that
stand in for an offered Link, `~ inquest for`, Link ids, MonkeyPaw slots,
analysis tags, emphasis) is recorded in the working doc and in the IR 3
requests.

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
| Asking each truth | `{{ c blessed }}` (the garden), `me.knows {{ P suits here }}` (the almanac), `some P: you.know {{ P thrives }} and not {{ P thrives }}` |
| Showing the almanac | `{known about: here}` |
| Seasons | `clock: 0..=8 ~ season 3 ~ seasons spring, summer, autumn`, `on season summer`, `clock.season` |
| The moments of a turn | `on turn begins` (after the NPCs move), `on turn ends` (before) |
| Making things | `recipe lantern: jam_jar, candle_end ~ make` |
| A margin for rules | `:: garden_rules [rules]` |
| A relation with every member of a kind | `relation trust: gardener, each gnome`, `trust with self` |
| Picking a place by a rule | `one(bed, unwatched, where: {{ it has plant }})` |

**Decided 16 Nov 2026** and applied here: the rules language is Inklog;
quantifiers are `some`, `every`, `no`; `on turn begins` / `on turn ends`;
`where:` in kind filters; `you` is a reserved name for the pov.

**Found while writing it.** `inkjot check` 0.5 reads `{{ P in B }}` in a
margin as an insert, and `{one {{ ? prizewinning }}}` cannot be read at all.
v0.6 needs `{{` and `}}` as tokens of their own, lexed before inserts.

## Rummy at Ashby (`rummy-at-ashby.twee`)

A card game inside a story. Nell, running from the bailiffs, plays the
ferryman for passage, and may ask her grandmother's ghost, unseen by day,
to read the old man's hand. A sketch for decks and piles: `deck`, `pile`
(`~ face down`, `~ per player`, `~ private`), `meld` (`set`, `run`),
`shuffle`, `deal`, `draw`, `turn up`, `lay down`, `discard`, and Links made
one per thing with `~ for each m in melds(my hand)`. The opponent plays by
a few header effects, in order.
