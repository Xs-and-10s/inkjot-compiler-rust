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

- **the Doppelgänger Clan**: a killer may take the victim's face, once
  each; the morning finds no body, only the killer's old name missing
  (revised 25 Nov 2026, from a game designer's notes);
- **the Debt Collectors**: they mark two names a night instead of
  killing, and a marked name dies at the next nightfall unless someone
  pays with their vote;
- **the Cult of the Long Night**: they convert instead of killing, and
  only someone who voted with them that day; once there are two
  Acolytes, on odd nights only; the One Who Left can never be taken;
- **the Heist Crew**: no deaths; a third of the table's keepsakes, plus
  one, stolen from the players' piles and they win. A thief voted out
  gives back what they carried.

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
syndicate where: it.special == chosen_special`); a consensus that
remembers who chose each target, in order (`victim.chosen_by`), with
`first(…)`, so the first to vote for the chosen target does the deed for
the group (decided 21 Nov); a pile on a set of players (`all(guest,
where: …).loot`); putting each card back where it belongs (`move each
found to it.owner.keepsake`), for a thief voted out, whose loot is
returned (decided 21 Nov); a consensus on two targets (`~ consensus 2`,
`targets`, `first(…)`, `last(…)`), for the Collectors' two marks a night; whether a vote was decided by the
draw (`accusation.drawn`); and `measure` (built, 21 Nov), for what each
game reports when it ends.

All of it is built by milestone 21d but the Detective's knowledge base
(`knowledge findings`, `assert {{ … }} in findings`), which is milestone
22's. `tests/ex7_play.rs` plays the draft with those lines set aside: its
own tests, and random games, every one of which must end.

### Ex 7: tuning from playtests

`tests/ex7_rules.rs` tunes the specials against a plain syndicate with
players who choose at random. Real tables will differ, so every game
reports its measures when it ends (the engine's `measured` event), and
the questions they answer are the ones the tuning asked:

| Question | Measures |
|---|---|
| Is each special as fair as a plain syndicate, at each size? | the ending (who won), `special`, `players`, beside `syndicate == none` games of the same size |
| Is the five-player Cult too easy for the town, as the model says? | the ending, `special == cult`, `players == 5` |
| Do games run too long or too short? | `nights`, by `players` and `special` |
| Does the town read the table, or vote at random? | `lynched_syndicate` against `lynched_town`, `draws`, `finds` |
| Are the Doctor and the Detective too strong or too weak? | `saves`, `finds`, `doctor_lived`, `detective_lived` |
| Does each special's rule do what it was tuned to? | `takings` (Cult), `faces`, `disguised_lynched`, `replaced_doctor` and `replaced_detective` (Doppelgänger), `debts_paid` and `foreclosures` (Collectors), `haul`, `haul_needed` and `returned` (Heist) |
| Do tables ask for specials, and does the moderator change the game? | `asked_for_special`, `moderator` |

The first number to watch: the town's win rate per special and size, set
beside the model's table. Where they differ by more than the model's
tolerance (15 points), the rule is the one to look at, and the model is
the place to try a fix before a table sees it.

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

## Ex 9: Saltmarrow (`ex9-saltmarrow.twee`)

A Zork-like solo `game`, in general words only. Saltmarrow is an old salt
mine under a fen town, and its salt holds spells: a working is paid for
with a crystal on the tongue. Agnes Thwaite, the last saltwright, went down
three days ago; you go after her, with her lamp, her primer and four
crystals.

Fifteen places on three levels, four of them timbered; nineteen spells,
fourteen of them learned from carvings, notes and charms across the mine,
five of them greater workings made of two; the Gleaner, who wanders the
upper workings with a candle and steals tools; firedamp in the north
drift; and the Brinewarden, who lets anyone down and nobody up with
marrow salt. It can be fought (fresh water unmakes salt), sneaked past (it
feels warmth and hears footsteps), bargained with (it was never paid), or
outrun (it walks only on salt). Or the shard can be left behind.

Fire is the game's weather. A flame near dry timber (the lamp, a spell,
the Gleaner's candle, the firedamp) starts a fire that smoulders, burns,
blazes, reaches the next timbered place, and burns out. A burnt shaft can
be crossed three more times, the mice leaving one by one, and then falls
in behind you. No dead ends: every way that can fall has another beside
it, by magic or by engineering.

**Checked by playing it.** `tests/ex9_map.rs` reads the map, the ways, the
items, the spells' costs and where each is learned from the draft, plays
at random with fires, thefts and falling shafts, then searches for a way
to bring Thwaite out, once with the pick and once without (it can break,
and must never be the only way). It found three dead ends while the draft
was written: stranded on the surface with the ladderway down (now: the
cage rope always goes down); stranded in the deep with no spell to drain
the stair (now: Thwaite teaches *draw* when you find her); out of salt in
the deep (now: salt regrows in the Marrow). Undo any of the three and it
fails.

**Played by the interpreter** (milestones 20c and 20d). It checks clean. `tests/ex9_play.rs` plays
the draft itself at random, the Warden's fight included, and wins it
without a fight: a stretcher made in the timber yard, Thwaite carried
up through the winze, the shard left behind, 40 of 100, "a crust
scraper".

What it asks of v0.6, beyond the plan and the encounter milestone:

| Construct | Example |
|---|---|
| A duration on an effect | `set me.hushed: yes ~ for 4 turns`, `cover me ~ for 3 turns` |
| A delay on an effect | `set planks_out: yes ~ after 2 turns` |
| Spreading from a set, one way only | `spreads 1 from ablaze to nearest(place, where: …) ~ along way ~ within 1` |
| Handlers on a kind, naming the thing | `on enter shaft by me when entered.burnt`, `on leave shaft by me when left.crossings == 0` |
| Handlers on an NPC's move | `on move warden when warden.roused and warden.here` |
| Taking and dropping, offered like moves | `default takes: take, drop` |
| Links offered everywhere | the spellbook, a `[lib everywhere]` passage (built), with `~ at` Links (built in 20a) |
| Items that come in a number | `item stave: … ~ count 4`, `count(stave, where: it.held_by(me))`, `lose stave 2` |
| A recipe with a part twice | `recipe stretcher: stave, stave, sacking` |
| A tool used by a Link, so it may break | `[[Dig at the fall->…]] ~ uses pick` with `item pick: … ~ breaks 1 in 6` |
| A resource counted in a noun | `resource salt: 4 ~ counted as "crystal"` |
| A table read as a value | `table rank: score ~ 0..=19: "a lamp carrier"`, then `{rank}` |
| The next place toward another | `here.toward(out)`, for Dowse |
| NPCs that follow and pursue | `~ follows me when …`; goals with `~ every 2 turns` and `~ where: it.salted` |
| Kinds of item, for `where:` | `~ catches` (a flame takes), `~ burns` (a fire destroys) |

**`define`, tested again.** `ex9-spellbook-define.twee` writes the
spellbook with typed fields and the word `spell`. Leaving comments out, 90
lines in general words against 80. The difference that matters is not
length: in general words each spell says its numbers twice, once in the
book ("Lasts 4 turns", "Power 2") and once in its effect, and the two can
drift. `tests/ex9_map.rs` checks they agree, which is the cost of the
general words. Verdict in `docs/define.md`.

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
`docs/define.md`: the general draft is the one to build. Built in
milestone 20d: the general draft is now Ex 2's own duel, run by its tests.

## Rummy at Ashby (`rummy-at-ashby.twee`)

A card game inside a story. Nell, running from the bailiffs, plays the
ferryman for passage, and may ask her grandmother's ghost, unseen by day,
to read the old man's hand. A sketch for decks and piles: `deck`, `pile`
(`~ face down`, `~ per player`, `~ private`), `meld` (`set`, `run`),
`shuffle`, `deal`, `draw`, `turn up`, `lay down`, `discard`, and Links made
one per thing with `~ for each m in melds(my hand)`. The opponent plays by
a few header effects, in order.
