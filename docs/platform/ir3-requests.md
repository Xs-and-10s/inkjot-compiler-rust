# IR 3: what the platform needs

> Copied from the platform's handoff of 20 Nov 2026. The platform's
> Inquest fixture was cut from an unpublished work; in this public copy its
> ids, labels and text are replaced, consistently, by a made-up river
> crossing, so they differ from the fixture in the platform repository.


IR version 2 is frozen. Everything here is a request for version 3, found by
building the platform against IR 2 and a hand-written Inquest fixture,
an Inquest fixture in the platform's `priv/inklinks/`: three chapters,
ch2 to the opening of ch4, with two forks for one Link, a fork
asked for from a later chapter, and ch3's own fork. Every field the fixture
uses that IR 2 lacks is listed here.

| # | Request | Needed for | Size |
|---|---|---|---|
| 0 | Consumers ignore fields they don't know | adding to IR 3 without an IR 4 | a rule, written down |
| 1 | `inquest` field on forks: `stands_in_for`, `from`, `credit`, `pending`; retire `kind: "inquest"` | Inquests at all | new field, two checks |
| 2 | Link ids that survive revisions | every Inker report, every event naming a Link | compiler change to `id` |
| 3 | Styled runs instead of asterisks, labels as parts | rendering without parsing Inkjot | new part kind |
| 4 | `state` declarations, with a `type` | Ex 4 (v0.6) | new `declare` kind |
| 5 | Passage titles (optional) | the earlier-visit fallback, Inker reports | optional field |

Ordered by what blocks most. 1 and 2 are the ones the platform cannot
work around in a real Inklink; 3 and 4 the stub works around today; 5 is
polish. 0 is what lets the rest grow without waiting for IR 4.

## 0. Unknown fields are ignored

State it in the IR spec: a consumer ignores an object field it does not
know, and a producer never changes the meaning of an existing field
without a new IR version. Then a new optional field (a MonkeyPaw `brief`,
a passage `title`, a `track` mark) is an addition, not a version, and the
platform and the compiler can move at their own pace. The platform's stub
already ignores unknown fields.

## 1. Inquests: a fork is a Link with an `inquest` field

```json
{
  "type": "link",
  "id": "swim_across",
  "label": "Swim across in the dark",
  "target": "ch2_alt",
  "kind": "go",
  "inquest": { "stands_in_for": "pay_the_ferryman", "from": ["ch3"], "credit": "inker" },
  "...": "every other Link field, as in IR 2"
}
```

| Field | Meaning |
|---|---|
| `inquest` | Present only on a fork. A Link without it is an offered Link (canon is implicit, as decided 15 Nov) |
| `inquest.stands_in_for` | The id of the offered Link this fork stands in for, in the same passage. **Always written out**, even when the source leaves it implicit ("the Link just above"): the compiler resolves the default, so a runtime never infers it from position |
| `inquest.credit` | Who wrote the fork as it stands: `inker`, `monkeypaw`, `ghostwriter` or `storyteller`. Attribution only: the content is fixed. A MonkeyPaw passage the Inker baked into the source keeps `credit: monkeypaw` |
| `inquest.pending` | Present only on a fork **not written yet**: a slot filled at runtime. `{"by": "monkeypaw" \| "storyteller" \| "ghostwriter", "rejoin": ["ch5"], "brief": "tragic_turns"}`. `by` names a **role**, never a person: who holds the Storyteller role is the platform's to decide. `brief` is optional (below) |
| `inquest.from` | Optional. Later passages from which this fork may also be asked for (`~ inquest from ch3`). The fork stays in its own passage, beside the Link it stands in for; `from` only says where else it can be asked for |

**Several forks for one offered Link** are allowed (decided 18 Nov): two or
more Links with `inquest.stands_in_for` naming the same Link. **Their order
is the order they are written in the passage**, and the IR keeps it (the
`body` is already in source order). The platform shows them as one group,
in that order, opened in the offered Link's place.

**Why `credit` and `pending`, not one `by`.** `by` meant two things:
who fills a slot at runtime, and who wrote content that is now fixed.
Baking (an Inker keeps the best MonkeyPaw output and writes it into the
source) turns the first into the second, so the IR needs both, and a
fork with `pending` is a slot while one without is written content.

**Slots that grow a subtree.** Today a slot is one fork that rejoins canon.
An Inker who writes only the canon line and leaves the divergence to
MonkeyPaw needs more: a slot that may write several passages, with its own
Links and forks, before it rejoins (or ends). Requested now, as shape only:

- `pending.rejoin` is a list: the passages the subtree may exit to;
  `"rejoin": []` with `"endings": true` lets it write its own endings. A
  written ending needs a stable id once generated, so ratings by ending
  still mean something.
- `pending.brief` names a **declared brief**, so the constraints live in
  one place that `inkjot check` validates, not in a one-line expression on
  the tag:

  ```json
  { "declare": "brief", "id": "tragic_turns",
    "passages": [1, 4], "links": [1, 3], "forks_per_link": [0, 2],
    "tone": {"tragedy": 5, "comedy": 1, "neutral": 2} }
  ```

  The brief's vocabulary can come later, field by field (rule 0). What IR
  3 needs is that `brief` exists and names a declaration.

Why a field and not `kind: "inquest"` (reserved in IR 2): being a fork is
orthogonal to what a Link does. A fork can be a plain move (`go`), or an act
(`take book`), so it needs its own `kind`. **Request: retire the reserved
`kind: "inquest"`.**

What the platform does with `stands_in_for`:

1. **The flip.** In Inquest mode, an offered Link opens into the group of
   forks that stand in for it, in its place on the page.
2. **The box.** An offered Link no fork stands in for asks "what did you
   want instead" when chosen in Inquest mode.
3. **The record.** `fork_taken` and `fork_declined` name the pair:
   "took `swim_across` instead of `pay_the_ferryman`".
4. **Availability.** A fork is offered only while its offered Link is: if
   that Link's `when` fails, the fork is hidden too. A fork may still carry
   its own `when` (and `hidden_until`), which must also hold.

The budget is IR 2's resource declaration, unchanged:

```json
{ "declare": "resource", "id": "inquests", "value": 3 }
```

The platform turns Inquests on only when that is declared **and** the
Inklink has at least one fork (opt-in by the Inker, as decided).
`inkjot check` should refuse a fork in `mode: cohort` (decided: solo only),
a fork whose `stands_in_for` names no offered Link in the same passage, and
a `pending.brief` that names no `brief` declaration.

#### Asking from a later passage

`from` lists passage names. `inkjot check` should refuse a name that is not
a passage, and could warn when no path leads from the fork's passage to the
`from` passage. At runtime the platform finds, for the Nib the Linker is at,
the latest visit to each passage whose forks name it in `from`, shows that
visit's offered Links ("Back when you chose …"), and, if a fork is taken,
replays the reading to that visit and takes it there. A fork without `from`
can be asked for only from its own passage.

#### How far back (decided 18 Nov)

`from` names passages, so the Inker decides which. When the reading has
visited that passage more than once, the platform offers the visits where
the Linker chose differently, newest first: visits with the same choice
collapse into the latest (going further back for the same choice only
undoes more), at most three are offered, and each says how many choices
going back would undo. Tested on `test/fixtures/inklinks/the-well.json`, a
hub the reading returns to. Nothing in the IR changes for this; it is
recorded so the interpreter's own Inquest support (if it ever has one)
agrees.

## 2. Link ids that survive revisions

IR 2's `id` is positional (`ch2#2`), so reordering Links changes every id
after them, and an Inker's data (Inquest requests, ratings, forks taken)
would point at the wrong Link. The fixture uses the Inker's own ids
(`~ id pay_the_ferryman`). Request:

- `id` is the Inker's `~ id` when written;
- otherwise it is derived from the passage and the target, not the
  position (`ch2->ch3`), as decided 15 Nov, with a suffix only when two
  Links share both;
- optionally `"id_from": "inker" | "target"`, so tools can warn when an
  Inquest-enabled Inklink has forks or offered Links without Inker ids
  (`inkjot check` should warn, as decided).

## 3. Styled runs instead of asterisks

Lower emphasis at compile time, so no runtime parses Inkjot. A paragraph is
already a list of parts; add one more kind of part:

```json
["He opened his mouth to say ", {"style": ["italic"], "parts": ["I don’t remember"]}, ", and found that was not true."]
```

- `style` is a list of `"italic"` and `"bold"`; `***x***` is both.
- `parts` may hold anything a paragraph holds, inserts included, so
  `*{leo.aka}*` works.
- Escapes are settled by the compiler: `\*` arrives as `*`.
- The names describe typography, as the Inker wrote it. A renderer maps
  them to `<i>` and `<b>`: in fiction, italics are mostly thoughts, titles
  and foreign words, not stress.
- **Link labels too**: a label becomes a list of parts, like a paragraph,
  so a label can carry emphasis and, if string interpolation (proposed
  18 Nov) is adopted, values.

The fixture keeps IR 2's asterisks (five of them, from the draft's own
text); the stub lowers them by the v0.6 rules.

## 4. `state` declarations

v0.6's `state crossed_free: no` appears as

```json
{ "declare": "state", "id": "crossed_free", "type": "boolean", "value": false }
```

`value` is a JSON value (`false`, not `"false"`), and `type` says what it
may hold, in the IR's own casing. The type earns its keep where the value
alone is ambiguous: an integer or a decimal, a state that starts out
`null`, and enums:

```json
{ "declare": "state", "id": "mood", "type": "enum", "of": ["calm", "tense"], "value": "calm" }
```

The compiler already checks types; the type in the IR is for the engine,
the platform's reports, and anything else that reads the IR without the
source.

read as a bare name (`{"not": "crossed_free"}`) and set as one
(`{"do": "set", "target": "crossed_free", "value": true}`). The stub
treats it as IR 2 treats a `fact` without `belongs_to`.

## 5. Passage titles (optional)

```json
{ "type": "passage", "name": "ch2_alt", "title": "The Ford", "...": "…" }
```

From `:: ch2_alt "The Ford"`, or a chapter heading the Inker already
writes. Two places want a human name where the platform has only a
passage id: the earlier-visit line when no choice can name it (today:
"Earlier:"), and Inker reports ("where readers stop: `ch2_alt`"). Absent
means the platform falls back as it does now.

## What the fixture does not need yet

Analysis tags (`~ beat`, `~ weighs`, `~ predict` …), `theme`, and MonkeyPaw
pending slots are in the Ex 4 draft but not in the fixture: the slice reads
none of them. When the platform shows Inkers their reports, `~ predict`
against the actual share of Linkers who took a Link is the first to want.
