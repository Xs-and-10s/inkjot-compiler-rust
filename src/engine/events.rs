//! What the engine reports: one list of events per action.
//!
//! The platform keeps one log per reading (`docs/platform/events.md`). Some
//! events are the Linker's (`link_taken`), some the platform's
//! (`rating_asked`), and some the engine's: what follows from the story,
//! the seed and the actions, and so comes back exactly on a replay. Those
//! are these. The platform wraps each in its envelope (ids, stream,
//! version, times) with `origin: engine`.
//!
//! An action's events come in the order things happened: what the
//! interpreter noted while playing it (a death, a rewind, a turn's end),
//! then `nib_shown`, then `ending_reached` and `reading_finished` if they
//! apply.

use super::View;
use crate::json::Value;
use crate::run::Run;

/// One thing the engine reports.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    /// The event type: `nib_shown`. Always one of [`CATALOGUE`].
    pub kind: &'static str,
    /// Its facts: a JSON object with exactly the catalogue's keys.
    pub data: Value,
}

impl Event {
    /// Where every engine event comes from, in the platform's terms.
    pub const ORIGIN: &'static str = "engine";

    /// The shape of `data`. Every type is at version 1; a change to a
    /// type's keys is a new version.
    pub const TYPE_VERSION: u32 = 1;

    /// `{"type", "type_version", "origin", "data"}`: the parts of the
    /// platform's envelope the engine knows.
    pub fn to_json(&self) -> Value {
        Value::obj([
            ("type", Value::str(self.kind)),
            ("type_version", Value::Num(Self::TYPE_VERSION.to_string())),
            ("origin", Value::str(Self::ORIGIN)),
            ("data", self.data.clone()),
        ])
    }
}

/// An event type: its name, the keys of its data, and when it happens.
pub struct EventType {
    pub name: &'static str,
    pub keys: &'static [&'static str],
    pub when: &'static str,
}

/// Every event the engine reports. `docs/engine.md` shows this table, and
/// a test fails if the two differ.
pub const CATALOGUE: &[EventType] = &[
    EventType {
        name: "nib_shown",
        keys: &["nib", "passed", "links", "visit", "changes"],
        when: "After every action, and at the start: the Nib in front of the Linker, the passages the action went through on the way, the ids of the Links offered, how many times the Nib has been arrived at, and how the action changed the values marked `~ track` (`{\"leo.trust\": [2, 3]}`; empty when nothing tracked changed).",
    },
    EventType {
        name: "ending_reached",
        keys: &["ending"],
        when: "The action arrived at a `[finale]` Nib. Its name is the ending's id.",
    },
    EventType {
        name: "reading_finished",
        keys: &["ending"],
        when: "Nothing is offered and nothing asked. `ending` is the last finale reached, or null.",
    },
    EventType {
        name: "checkpoint_reached",
        keys: &["checkpoint"],
        when: "A passage says `checkpoint name`: the world is saved there.",
    },
    EventType {
        name: "died",
        keys: &["who", "at"],
        when: "A character died during the action, at that place.",
    },
    EventType {
        name: "rewound",
        keys: &["who", "stop", "checkpoint", "echo", "cause"],
        when: "Someone went back: to the start of `stop`, or to `checkpoint` (else null); as an echo re-living it, or with the world going back; `cause` is `rewind` (a Link) or `death`.",
    },
    EventType {
        name: "echo_ended",
        keys: &["who"],
        when: "An echo reached the present again.",
    },
    EventType {
        name: "turn_ended",
        keys: &["stop"],
        when: "The clock's stop `stop` ended.",
    },
];

/// The catalogue as a Markdown table, as `docs/engine.md` shows it.
pub fn catalogue_markdown() -> String {
    let mut out = String::from("| Type | `data` | When |\n|---|---|---|\n");
    for t in CATALOGUE {
        let keys: Vec<String> = t.keys.iter().map(|k| format!("`{k}`")).collect();
        out.push_str(&format!(
            "| `{}` | {} | {} |\n",
            t.name,
            keys.join(", "),
            t.when
        ));
    }
    out
}

/// The engine's events for one action, given the view it led to. `ending`
/// is the last finale reached in the reading, carried from state to state.
pub(super) fn events(
    run: &mut Run,
    view: &View,
    ending: &mut Option<String>,
    changes: Value,
) -> Vec<Event> {
    let mut out: Vec<Event> = run
        .take_noted()
        .into_iter()
        .map(|(kind, data)| Event { kind, data })
        .collect();
    let mut passed: Vec<String> = run.trail().to_vec();
    // The Nib itself is not passed. (After an act, the pov is back where
    // it was, and that Nib was not entered again.)
    if passed.last() == Some(&view.nib) {
        passed.pop();
    }
    let strings = |items: Vec<String>| Value::Arr(items.into_iter().map(Value::Str).collect());
    out.push(Event {
        kind: "nib_shown",
        data: Value::obj([
            ("nib", Value::str(&view.nib)),
            ("passed", strings(passed)),
            (
                "links",
                strings(view.links.iter().map(|l| l.id.clone()).collect()),
            ),
            ("visit", Value::Num(run.visits_to(&view.nib).to_string())),
            ("changes", changes),
        ]),
    });
    // Reached on arrival, not again for each answer given while there.
    let arrived = run.trail().contains(&view.nib);
    if arrived && view.tags.iter().any(|t| t == "finale") {
        *ending = Some(view.nib.clone());
        out.push(Event {
            kind: "ending_reached",
            data: Value::obj([("ending", Value::str(&view.nib))]),
        });
    }
    if view.over {
        let last = ending.as_deref().map_or(Value::Null, Value::str);
        out.push(Event {
            kind: "reading_finished",
            data: Value::obj([("ending", last)]),
        });
    }
    out
}
