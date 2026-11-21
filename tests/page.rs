//! Milestone 19, the page: what the reader sees, and the words the engine
//! writes into it.
//!
//! - `narration:` decides the pov's words: "you", or "I" and "me".
//! - Names are capitalised at the start of a sentence (`~ keep case` opts
//!   out), and speech cues are styled by `cue names:` and `cue directions:`.
//! - `[if c; then e]` and `[unless c; then e]`: e happens when shown.
//! - Blockquotes: `[quote letter ~ by "…"]` … `[end]`.
//! - Redaction, three ways, and the hidden text never reaches the view.

use inkjot::engine::{Inklink, Para, Quote, Start, State, BAR};
use proptest::prelude::*;

/// A one-passage Inklink: the config's extra lines, then the passage.
fn story(config: &str, passage: &str) -> String {
    format!(
        r#":: StoryTitle
Page

:: StoryData
{{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "room"}}

:: Inklink [config]
kind: story
mode: solo
pov: ann
character ann: "Ann" ~ selectable
{config}

:: room
{passage}
"#
    )
}

fn start(src: &str) -> State {
    let ir = inkjot::build(src).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    Inklink::from_ir(&ir)
        .unwrap()
        .start(1, &Start::default())
        .unwrap()
}

fn plain(state: &State) -> Vec<String> {
    state.view().text.iter().map(Para::plain).collect()
}

fn errors(src: &str) -> Vec<String> {
    inkjot::check(src)
        .diags
        .into_iter()
        .filter(|d| d.severity == inkjot::Severity::Error)
        .map(|d| d.msg)
        .collect()
}

// ------------------------------------------------------------ narration

#[test]
fn the_pov_is_told_as_narration_says() {
    let passage = "@you I came by the drove.\n{me} stopped. Mr. Fenwick looked at {me}.";
    let second = start(&story("narration: second person", passage));
    assert_eq!(
        plain(&second),
        [
            // A cue names the speaker; narration is for the prose.
            "Ann: I came by the drove.",
            "You stopped. Mr. Fenwick looked at you."
        ]
    );
    let first = start(&story("narration: first person", passage));
    assert_eq!(
        plain(&first),
        [
            "Ann: I came by the drove.",
            "I stopped. Mr. Fenwick looked at me."
        ]
    );
    let third = start(&story("narration: third person ~ omniscient", passage));
    assert_eq!(
        plain(&third),
        [
            "Ann: I came by the drove.",
            "Ann stopped. Mr. Fenwick looked at Ann."
        ]
    );
    // Not said: the name, as before.
    assert_eq!(plain(&start(&story("", passage))), plain(&third));
}

#[test]
fn a_passage_may_tell_itself_differently() {
    let src = story(
        "narration: second person",
        "narration: first person\n--\n{me} went.",
    );
    assert_eq!(plain(&start(&src)), ["I went."]);
}

#[test]
fn a_sub_nib_is_told_as_its_passage_is() {
    let src = story(
        "narration: second person",
        "narration: first person\n--\n{me} went.\n> [[On->room:on]]\n\n:: room:on\n@you {me} did.",
    );
    let state = start(&src);
    let id = state.view().links[0].id.clone();
    assert_eq!(plain(&state.take_link(&id).unwrap()), ["Ann: I did."]);
}

// ------------------------------------------------------------ names and cues

const NAMED: &str = r#"npc foreman: "the foreman"
npc poet: "ee cummings" ~ keep case
npc fenwick: "Mr. Fenwick""#;

#[test]
fn a_name_starting_a_sentence_is_capitalised() {
    let src = story(
        NAMED,
        "{foreman} stands. He looks at {foreman}. “{foreman}!” Then: {foreman} sits.\n{poet} reads.\n@foreman We find accidental death.",
    );
    assert_eq!(
        plain(&start(&src)),
        [
            "The foreman stands. He looks at the foreman. “The foreman!” Then: The foreman sits.",
            "ee cummings reads.",
            "The foreman: We find accidental death.",
        ]
    );
}

#[test]
fn cues_are_styled_as_the_config_says() {
    let src = story(
        &format!("{NAMED}\ncue names: upper, bold\ncue directions: italic"),
        "@fenwick (to the jury) Gentlemen.",
    );
    let state = start(&src);
    let runs = &state.view().text[0].runs;
    assert_eq!(
        state.view().text[0].plain(),
        "MR. FENWICK (to the jury): Gentlemen."
    );
    assert_eq!(runs[0].text, "MR. FENWICK");
    assert_eq!(runs[0].style, ["bold"]);
    assert_eq!(runs[2].text, "(to the jury)");
    assert_eq!(runs[2].style, ["italic"]);
}

// ------------------------------------------------------------ effects when shown

#[test]
fn a_block_shown_does_its_effect_and_may_unlock_the_next() {
    let src = story(
        "state rang: no\nstate counted: 0",
        "[if not rang; then set rang: yes]\nThe bell rings.\n[if rang; then raise counted 1]\nSomeone counts.\n[unless rang; then raise counted 10]\nNever shown.\n[end]\nCounted {counted}.",
    );
    let state = start(&src);
    assert_eq!(
        plain(&state),
        ["The bell rings.", "Someone counts.", "Counted 1."]
    );
}

// ------------------------------------------------------------ blockquotes

#[test]
fn a_quote_is_one_block_and_its_end_is_its_own() {
    let src = story(
        "state struck: yes\nstate nope: no",
        r#"[if struck]
You find the words.

[quote letter ~ by "E. Garrard, struck from the record"]
The Fen floods every winter.
Eight hundred acres.
[end]

You leave the line where it is.

[end]
[if nope]
[quote poster]
Hidden poster.
[end]
Also hidden.
[end]
After."#,
    );
    let state = start(&src);
    let text = &state.view().text;
    assert_eq!(
        plain(&state),
        [
            "You find the words.",
            "The Fen floods every winter.",
            "Eight hundred acres.",
            "You leave the line where it is.",
            "After."
        ]
    );
    let letter = |first| {
        Some(Quote {
            kind: "letter".into(),
            by: Some("E. Garrard, struck from the record".into()),
            first,
        })
    };
    assert_eq!(text[0].quote, None);
    assert_eq!(text[1].quote, letter(true));
    assert_eq!(text[2].quote, letter(false));
    assert_eq!(text[3].quote, None);
}

// ------------------------------------------------------------ redaction

#[test]
fn redacted_text_is_a_bar_until_it_is_revealed() {
    let src = story(
        "state closed: no\nnpc garrard: \"the Squire\"\nnpc organist: \"the Organist\" ~ redacted until closed",
        r#"[redacted until closed]
@garrard The Fen floods every winter.
[end]
WREN TO CONFIRM BY {redacted "ORGANIST" until closed} AT MIDNIGHT.
{organist} plays.
> [[Close the record->room:closed]]

:: room:closed
set closed: yes
--
[redacted until closed]
@garrard The Fen floods every winter.
[end]
BY {redacted "ORGANIST" until closed}. {organist} plays."#,
    );
    let state = start(&src);
    let bar = |n: usize| BAR.to_string().repeat(n);
    let text = plain(&state);
    // The speaker is kept; what he said is not.
    assert_eq!(text[0], format!("The Squire: {}", bar(28)));
    assert_eq!(
        text[1],
        format!("WREN TO CONFIRM BY {} AT MIDNIGHT.", bar(8))
    );
    assert_eq!(text[2], format!("{} plays.", bar(12)));
    let all: String = text.concat();
    for word in ["Fen", "ORGANIST", "Organist"] {
        assert!(!all.contains(word), "`{word}` reached the page: {all}");
    }
    let redacted = &state.view().text[0].runs.last().unwrap().style;
    assert_eq!(redacted, &["redacted"]);
    let id = state.view().links[0].id.clone();
    let after = state.take_link(&id).unwrap();
    assert_eq!(
        plain(&after),
        [
            "The Squire: The Fen floods every winter.",
            "BY ORGANIST. The Organist plays."
        ]
    );
}

// ------------------------------------------------------------ the checker

#[test]
fn the_renamed_words_say_how_to_fix_them() {
    let told = errors(&story("", "told: second person\n--\nHi."));
    assert!(
        told.iter().any(|e| e.contains("now `narration:`")),
        "{told:?}"
    );
    let playable = errors(&story("character bo: \"Bo\" ~ playable", "Hi."));
    assert!(
        playable.iter().any(|e| e.contains("now `~ selectable`")),
        "{playable:?}"
    );
}

#[test]
fn the_checker_reads_the_new_words() {
    let bad = [
        ("narration: fourth person", "Hi.", "`narration` is one of"),
        ("cue names: loud", "Hi.", "not a cue style"),
        ("", "[quote billboard]\nHi.\n[end]", "`[quote …]` is one of"),
        ("", "[quote letter]\nHi.", "never closed"),
        ("", "[redacted]\nHi.\n[end]", "says until when"),
        ("", "Hi {redacted \"x\"}.", "says until when"),
        ("", "[if yes; then]\nHi.", "`then` needs an effect"),
    ];
    for (config, passage, wanted) in bad {
        let errs = errors(&story(config, passage));
        assert!(
            errs.iter().any(|e| e.contains(wanted)),
            "{config} / {passage}: wanted `{wanted}`, got {errs:?}"
        );
    }
    let good = story(
        "narration: second person ~ omniscient\ncue names: capital, bold\ncue directions: italic\nnpc x: \"X\" ~ keep case ~ redacted until yes",
        "[if yes; then say x \"Hi.\"]\nHi.\n[quote edict ~ by \"J. F.\"]\nNotice.\n[end]\n[redacted until yes]\nGone.\n[end]\n{redacted \"x\" until yes}",
    );
    assert_eq!(errors(&good), Vec::<String>::new());
}

#[test]
fn a_test_can_expect_events() {
    let src = format!(
        "{}\n> [[Go->end]]\n\n:: end [finale]\nThe end.\n\n:: t [test]\nstart: room\nexpect events: nib_shown\ndo: \"Go\"\nexpect events: ending_reached, reading_finished\n",
        story("", "Hi.").trim_end()
    );
    let ir = inkjot::build(&src).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    for outcome in inkjot::run::test::run_all(&src, &ir).unwrap() {
        assert!(outcome.passed(), "{:?}", outcome.failures);
    }
    let wrong = src.replace(
        "expect events: ending_reached, reading_finished",
        "expect events: died",
    );
    let outcomes = inkjot::run::test::run_all(&wrong, &ir).unwrap();
    assert!(outcomes.iter().any(|o| !o.passed()));
}

// ------------------------------------------------------------ properties

/// Text a passage can hold as prose: letters, digits, spaces, and the
/// punctuation that means nothing to Inkjot.
fn prose() -> impl Strategy<Value = String> {
    "[A-Za-z0-9][A-Za-z0-9 ,.;!?'’—-]{0,60}".prop_map(|s| s.trim_end().to_string())
}

/// A tree of blocks, as a body would nest them.
#[derive(Clone, Debug)]
enum Node {
    Line(usize),
    If(bool, Vec<Node>),
    Quote(Vec<Node>),
    Redacted(Vec<Node>),
}

fn tree() -> impl Strategy<Value = Vec<Node>> {
    let leaf = (0usize..1000).prop_map(Node::Line);
    let node = leaf.prop_recursive(4, 24, 4, |inner| {
        prop_oneof![
            (any::<bool>(), prop::collection::vec(inner.clone(), 1..4))
                .prop_map(|(c, v)| Node::If(c, v)),
            prop::collection::vec(inner.clone(), 1..4).prop_map(Node::Quote),
            prop::collection::vec(inner, 1..4).prop_map(Node::Redacted),
        ]
    });
    prop::collection::vec(node, 1..5)
        .prop_filter("an [if] directly in an [if] continues its chain", |n| {
            !n.iter().any(if_in_if)
        })
}

/// An `[if]` written straight inside another, with no block between.
fn if_in_if(n: &Node) -> bool {
    match n {
        Node::Line(_) => false,
        Node::If(_, inner) => inner
            .iter()
            .any(|m| matches!(m, Node::If(..)) || if_in_if(m)),
        Node::Quote(inner) | Node::Redacted(inner) => inner.iter().any(if_in_if),
    }
}

/// The tree as Inkjot. An `[if]` holds only lines and blocks, and is
/// closed by `[end]` only where no block is open inside it, as Inkjot's
/// flat `[if]` chains are written.
fn write(nodes: &[Node], out: &mut String) {
    for n in nodes {
        match n {
            Node::Line(k) => out.push_str(&format!("Line {k}.\n")),
            Node::If(c, inner) => {
                out.push_str(&format!("[if {}]\n", if *c { "yes" } else { "no" }));
                write(inner, out);
                out.push_str("[end]\n");
            }
            Node::Quote(inner) => {
                out.push_str("[quote sign]\n");
                write(inner, out);
                out.push_str("[end]\n");
            }
            Node::Redacted(inner) => {
                out.push_str("[redacted until no]\n");
                write(inner, out);
                out.push_str("[end]\n");
            }
        }
    }
}

/// What should be shown: `(line, quoted, redacted)`.
fn model(nodes: &[Node], quoted: bool, redacted: bool, out: &mut Vec<(usize, bool, bool)>) {
    for n in nodes {
        match n {
            Node::Line(k) => out.push((*k, quoted, redacted)),
            Node::If(c, inner) => {
                if *c {
                    model(inner, quoted, redacted, out)
                }
            }
            Node::Quote(inner) => model(inner, true, redacted, out),
            Node::Redacted(inner) => model(inner, quoted, true, out),
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Redacted text never reaches the page: the bar is exactly as long as
    /// what it hides, and nothing else of it is there.
    #[test]
    fn redaction_shows_only_a_length(text in prose(), word in "[a-z]{1,12}") {
        let src = story(
            "state open: no",
            &format!("[redacted until open]\n{text}\n[end]\nX {{redacted \"{word}\" until open}} Y"),
        );
        let state = start(&src);
        let view = &state.view().text;
        prop_assert_eq!(view[0].plain(), BAR.to_string().repeat(text.chars().count()));
        prop_assert_eq!(
            view[1].plain(),
            format!("X {} Y", BAR.to_string().repeat(word.chars().count()))
        );
        for run in view.iter().flat_map(|p| &p.runs) {
            if run.style.contains(&"redacted") {
                prop_assert!(run.text.chars().all(|c| c == BAR));
            }
        }
    }

    /// Capitalising a name touches its first letter only, and only at the
    /// start of a sentence.
    #[test]
    fn a_name_changes_only_its_first_letter(name in "[a-z][a-z ]{0,20}[a-z]") {
        let src = story(
            &format!("npc x: \"{name}\""),
            "{x} came. Then {x} went.",
        );
        let shown = plain(&start(&src))[0].clone();
        let mut cap = name.chars();
        let first: String = cap.next().unwrap().to_uppercase().chain(cap).collect();
        prop_assert_eq!(shown, format!("{first} came. Then {name} went."));
    }

    /// However blocks nest, a line shows exactly when every `[if]` around
    /// it holds, inside the quotes and redactions around it, in order.
    #[test]
    fn blocks_nest_as_written(nodes in tree()) {
        let mut body = String::new();
        write(&nodes, &mut body);
        body.push_str("The end.\n");
        let src = story("", &body);
        let errs = errors(&src);
        prop_assert!(errs.is_empty(), "{:?}\n{}", errs, body);
        let state = start(&src);
        let mut wanted = Vec::new();
        model(&nodes, false, false, &mut wanted);
        let view = &state.view().text;
        prop_assert_eq!(view.len(), wanted.len() + 1, "{}", body);
        for (para, (k, quoted, redacted)) in view.iter().zip(&wanted) {
            let line = format!("Line {k}.");
            if *redacted {
                prop_assert_eq!(para.plain(), BAR.to_string().repeat(line.chars().count()));
            } else {
                prop_assert_eq!(para.plain(), line);
            }
            prop_assert_eq!(para.quote.is_some(), *quoted);
        }
        prop_assert_eq!(view.last().unwrap().plain(), "The end.");
    }
}
