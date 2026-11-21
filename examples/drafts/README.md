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

## Ex 7: Mafia, by Lamplight (`ex7-mafia.twee`)

The party game for 5 to 12 Linkers, with the engine as the moderator. Each
game draws a syndicate from a shuffled table of ten. The syndicate sets
the place and year, the roles' titles, how people die, and a list of
flourishes for each death, so the theme builds as the bodies fall. Four
special syndicates change the rules:

- **the Doppelgänger Clan**: a killer takes the victim's face, so the
  morning reports the wrong death;
- **the Debt Collectors**: they mark instead of killing, and a marked
  name dies at the next nightfall unless someone pays with their vote;
- **the Cult of the Long Night**: they convert instead of killing, and
  only someone who voted with them that day;
- **the Heist Crew**: no deaths; three keepsakes stolen from the players'
  piles and they win.

Three ways to moderate: the engine's tables; MonkeyPaw, writing each
morning fresh with the table's line as the fallback; or a Storyteller who
plays no role, sees everything, and may rewrite each morning's report.
Whispers, a rumour that spreads one seat a day, is always on offer once a
game.

**`where:` is the heart of it.** Nearly every choice is "one of the
players who…", so nearly every Link is `~ at guest where: …`. The config's
opening note teaches the difference: *which* is `where:`, *whether* is
`when`.

What it asks of v0.6, beyond the plan: one character per Linker
(`~ one per linker`); a seating ring to spread along; decks sized by
expressions; tables of text with named columns (`{syndicate.mafia}`);
lists picked like `aka` (`{one syndicate.deaths}`); Link groups for whoever
`where:` says, choosing together (`~ consensus`); a pending passage that
is not a fork (a morning MonkeyPaw writes); a `shown_as` that can be set
(the Doppelgänger's swap); `voters:` on the jury; a seat that exists only
in one mode (the Storyteller); a general ballot (`{ballot for: chosen,
from: specials, ties: draw}`, for the table choosing a special syndicate
when more than half ask for one); and a draw limited by `where:` (`roll
syndicate where: it.special == chosen_special`).

## Ex 8: The Inquest (`ex8-the-inquest.twee`)

A short solo `story` written for Inquests, in place of the withdrawn Ex 4.
Hythe Fen, November 1893: the lock-keeper drowned the night the sluice
was opened, and the inquest sits in the long room of the Plough. The frame
is in the second person, to the coroner's clerk; each testimony is in the
first person, as the witness, and the Linker chooses what the witness did
and what they say about it, which is not always the same thing.

An Inquest is the coroner's *suppose*: "Suppose you had gone back for the
lantern." The witness imagines the other night and, imagining it,
remembers something. Two of the five Supposes are written; two are
pending, for MonkeyPaw, each with one home (`pending:` in the placeholder
passage, whose body is the fallback). The verdict reads only what the
record holds: accident, misadventure, an open verdict, or unlawful
killing, with a rider on the Squire if the shilling came out.

The lock-keeper's son checks the gates in a fixed order, and on a bad
night five times. `{list checks ~ rotate 2s}` lets the reader live his
counting; the story takes it seriously, and his precision is what the
verdict turns on.

What it exercises: forks written and pending, `~ inquest for`, `~ credit`,
`theme`, `brief`, analysis tags on most Links, `~ track` on everything the
record holds, emphasis, names as the reader knows them (`shown as` until
`reveal`, `{x.aka}`), inline `if … then … else`, `[if …; then …]`,
`told:`, blockquotes, and a struck remark redacted until the record
closes.

**It runs.** Everything in it but `told:`, `[if …; then …]`, blockquotes,
redaction and `expect events:` is built, so `tests/ex8.rs` rewrites those
few into what they will mean and runs the rest on today's engine: its own
`[test]` passages, each Inquest through the engine API, and a property
that every reading, whatever is said and supposed, reaches exactly one
verdict and ends on the record.

## Ex 6, the public cut: Cloakroom (`ex6-cloakroom.twee`)

The full Ex 6, *Seven Names*, lives in the Inklinks Library. This is a
small story of its own with the same machinery, so the public tests can
cover it: New Year's Eve, 1961, a lakeside hotel, the cloakroom
attendant, one evening and three names.

What it exercises: two knowledge bases racing (your notebook, `~ per
linker ~ manual ~ reads knows`; the assistant manager's dossier, fed by
the doorman and the cigarette girl); integrity constraints that refuse
or flag notes; facts with a time (`{{ orme at bar @ new_years_eve.ten }}`);
uneven periods made of periods (`period evening: new_years_eve ~ of
hour`) and a clock written in their names; all three forms of redaction;
blockquotes; pronouns and `possessive()`; `person`, `where:` and
quantifiers ending in a dot; `~ at` and Link groups; a death that falls
back to the start of the hour, keeping the notebook; `finale when:`;
`expect events:`.

`tests/drafts.rs` lints it on every run (fact shapes, times, how a base
is asked, quantifiers, speech cues, Link targets, tested labels, comment
placement), with mutation properties that check the lint catches each
kind of slip anywhere in the file.

## Ex 2's duel, four ways (`ex2-duel-*.twee`)

The duel of Ex 2 rewritten for the encounter milestone: once in general
words (`ex2-duel-general.twee`), then once for each step of the `define`
proposal (`-step1-like`, `-step2-fields`, `-step3-define`), each holding
only what differs from the step before. What they showed is in
`docs/define.md`: the general draft is the one to build.

## Rummy at Ashby (`rummy-at-ashby.twee`)

A card game inside a story. Nell, running from the bailiffs, plays the
ferryman for passage, and may ask her grandmother's ghost, unseen by day,
to read the old man's hand. A sketch for decks and piles: `deck`, `pile`
(`~ face down`, `~ per player`, `~ private`), `meld` (`set`, `run`),
`shuffle`, `deal`, `draw`, `turn up`, `lay down`, `discard`, and Links made
one per thing with `~ for each m in melds(my hand)`. The opponent plays by
a few header effects, in order.
