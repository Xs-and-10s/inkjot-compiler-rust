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
garden from the start, and in your almanac (what you know, `knows`) once
you learn the fact. Rules hold in the garden always, and in your almanac
once known (`~ known`, `~ learned from pip_lore`). `~ in world` and `~ in
knows` narrow a rule to one side.

**Negation.** `not` is stratified: a rule may say `not {{ B shaded }}` only
if everything that concludes `shaded` sits in an earlier layer, so nothing
depends on its own negation. Only `~ in world` rules may say `not`: what you
don't know isn't false. The almanac deals in what you have seen instead
(`observe {{ here gets ? }}`), which makes it overconfident on purpose: its
`hopes` rule knows nothing of slugs until Old Sorrel teaches `slug_risk`.

What it uses, as written since milestone 22c:

| Construct | Example |
|---|---|
| Rules, and where they hold | `rule sunny: … and not {{ B shaded }} => {{ B gets sun }} ~ in world` |
| Learnable rules | `rule mirror: … ~ learned from pip_lore` |
| One value per subject | `verb soil ~ one value`, so digging in compost replaces the soil |
| Changing the garden | `assert {{ here composted }}`, `retract {{ ? watered }}` |
| Seeing | `observe {{ here gets ? }}`: forget what you knew, know what is |
| Asking each truth | `{{ c blessed }}` (the garden), `{{ ^sown.grows suits here }} in knows` (the almanac), `some P. {{ P thrives }} in knows and not {{ P thrives }}` |
| Showing the almanac | `{known about: here}`, `{known about: garden}` |
| Seasons | `period season: spring 3, summer 3, autumn 3`, `on season begins summer`, `clock.season` |
| The moments of a turn | `on turn begins` (after the NPCs move), `on turn ends` (before) |
| Making things | `recipe lantern: jam_jar, candle_end ~ make` |
| A margin for rules | `:: garden_rules [rules]` |
| A relation with every member of a kind | `relation trust: gardener, each gnome`, `self.trust` |
| Picking a place by a rule | `one(all(bed, where: not it.watched and {{ it has plant }}))` |
| The garden's shape, as facts | `fact layout: "…" ~ known ~ means {{ nw next_to n }}, …` |

**Decided 16 Nov 2026** and applied here: the rules language is Inklog;
quantifiers are `some`, `every`, `no`; `on turn begins` / `on turn ends`;
`where:` in kind filters; `you` is a reserved name for the pov.

**Brought up to date in milestone 22c**, and now it runs:
`tests/ex5_play.rs` checks it clean, runs its own `[test]` passages, and
plays 32 random years, every one of which ends. Three things changed in
the story, to make the whole of it reachable:

- The beds have paths between them (`route: nw - n - ne`, …): a `map` is
  a layout, not a set of ways. `beside` comes from a `layout` fact.
- The gnomes live under the thistle (`~ in c`).
- A gnome carries an offering back to Hester's dresser rather than keeping
  it, so the same honey can be left again: trust 2 (lore) and 3 (the
  blessing, the thistle) were out of reach with one offering each.

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

## Ex 11: The Meridian Key (`ex11-meridian-key.twee`)

A JRPG-style `game`, one level and its boss, in an original setting: four
scouts of the Lantern (Ress the vanguard, Ivo the arcanist, Tam the
medic, Quill the scout) steal
the Meridian Key from an Auric Dominion relay under a glacier. You look
through the lead and give the other three one command each a round; the
lead can change hands between fights. Two encounters: two hangar
sentries, then Prefect Orrel in the Lamplighter frame, whose armour turns
bullets until lightning breaks it, and who stops following the drill at
half strength.

What it asks of milestone 23:

| Construct | Example |
|---|---|
| Several opponents | `encounter sentry_a, sentry_b`, `~ at opponent here` |
| One command per ally a round | `> [[Ivo: ion bolt at {target}->…]] ~ ion_bolt ~ by ivo`, `~ one command each`, `end round` |
| Classes as archetypes | `:: vanguard [archetype]` with its `hp` and acts |
| Kinds of harm | `wound target 3 ~ as ion`; `~ resistance kinetic when self.armoured`, `~ weakness ion`, `~ immunity cold` |
| Cooldowns | `~ cooldown 3 rounds`, `ivo.cooling(frostfall)` |
| Pattern phases | `pattern drill: … ~ while orrel.hp > 15`, `pattern unbound: … ~ by weight`, `on pattern unbound begins` |
| A threat from the world | `unleash klaxon`, as the Key leaves its pillar |
| Healing and getting up | `heal target 4`, `revive target` |
| The lead | `lead ivo` |
| The encounter's state | `encounter.over`, `orrel.pattern.next` |

## Ex 13: Once Saved (`ex13-once-saved.twee`)

A debate `game`, solo, and an acceptance test for encounters that are not
fights. Thanksgiving, on the back porch: your cousin Ben, a Baptist Sunday
school teacher, holds "once saved, always saved"; you hold, with Trent,
that grace is lost by mortal sin and perseverance is a gift no one may
promise himself. His side is argued at its strongest, from the texts his
churches lead with (John 10, Romans 8, Ephesians 1 and 2, 1 John 5 and 2),
in the King James; yours from Trent's decree on justification, St Thomas
(I-II q.109 a.10) and Cornelius à Lapide, with the Douay-Rheims. Both are
public domain.

Objections are threats; answers are counters, tagged by kind
(distinction, Scripture, logic, analogy, authority, concession); pressing
a point of your own costs the round and brings his rebuttal. A good answer
moves him; citing the Church or granting his point helps him understand
you without moving him; an objection left standing costs you. Outcome
tiers: both, defended, convinced, a tie, lost; how well he understands you
colours each. No Inquests, no rewinds; hints are what he said before.

What it asks of milestone 23:

| Construct | Example |
|---|---|
| The kind of exchange | `encounter ben ~ as debate` (`debate` is proposed beside the decided five) |
| Moves by weight, each once | `pattern objections: … ~ by weight ~ each once`, with `~ weight 6 ~ when …` on each threat |
| Counters that are any tag | `~ answered by distinction, scripture, authority` |
| Answering | `answer ben`, `threat.answered_with` |
| Rebuttals | `hypothetical first when you.did(press_hebrews_six)` (Ex 2's `first when`) |
| A threat from someone outside the pattern | `unleash carols_question ~ by carol`, which takes the round's place |
| Rounds | `on round begins when encounter.round == 4`, `encounter.over`, `ben.objections.done` |
| Hints with conditions | `hint: "…" ~ when next == john_ten` |
| `landed` | the outcome once called `hit` |

**Checked while writing.** À Lapide's reading of Hebrews 6 is as the
Haydock commentary reports it: with Estius, he says the Greek Fathers and
most of the Latins read "impossible … to be renewed" of a second baptism.
Trent's lines are from Session VI (chapters 8, 9, 12, 13, 15; canons 16
and 23).

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

Since milestone 22b it also runs: `tests/ex6_play.rs` checks it clean,
runs its own `[test]` passages, and plays random evenings (32 of them,
notes and all), every one of which ends: at midnight, or in Keller's
notes. `retell` is still the stretch it was.

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
