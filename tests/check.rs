//! End-to-end tests for `inkjot check`.

use inkjot::{check, Severity};

const BASE: &str = r#":: StoryTitle
A Small Hall

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.5.0", "start": "hall"}

:: Inklink [config]
kind: story
mode: solo
pov: ann
place great_hall: "the hall"
place cellar: "the cellar"
way stair: great_hall - cellar ~ open when stair_clear
fact stair_clear: no
character ann: "Ann" ~ playable
npc bob: "Bob" ~ in great_hall
item key: "a key" ~ in great_hall
clue secret: "A secret."
act examine: cost 1
list notes: "notes"

on enter cellar when not stair_clear
~ then say bob "Mind the step."
~ once

:: hall
learn secret
--
The hall is quiet. {present} are here.

@bob (softly) Hello, {ann.name}.
~ I have been waiting.

[if me.knows.secret but not stair_clear]
You know something.

[else]
You know nothing.

[end]
> [[Look around->hall:look]] ~ examine ~ when me.knows.secret
> [[Take the key->hall:look]]
~ take key
~ cost 2 when me.is(ann)

:: hall:look [act]
next: hall
--
You look.
"#;

/// The errors `check` reports for `src`, as (line, message).
fn errors(src: &str) -> Vec<(usize, String)> {
    check(src)
        .diags
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| (d.line, d.msg))
        .collect()
}

/// Apply one edit to the base file and return the single error it must cause.
fn broken(from: &str, to: &str) -> (usize, String) {
    assert!(BASE.contains(from), "the base file has no `{from}`");
    let src = BASE.replacen(from, to, 1);
    let line = 1 + src[..src.find(to).unwrap()].matches('\n').count();
    let errs = errors(&src);
    assert_eq!(errs.len(), 1, "expected exactly one error, got {errs:#?}");
    assert_eq!(errs[0].0, line, "the error is on the wrong line: {errs:#?}");
    errs[0].clone()
}

fn assert_broken(from: &str, to: &str, expect: &str) {
    let (_, msg) = broken(from, to);
    assert!(
        msg.contains(expect),
        "expected an error mentioning `{expect}`, got: {msg}"
    );
}

/// Like `assert_broken`, for edits whose error is reported on a different line.
fn assert_broken_elsewhere(from: &str, to: &str, expect: &str) {
    let errs = errors(&BASE.replacen(from, to, 1));
    assert_eq!(errs.len(), 1, "expected exactly one error, got {errs:#?}");
    assert!(
        errs[0].1.contains(expect),
        "expected an error mentioning `{expect}`, got: {}",
        errs[0].1
    );
}

#[test]
fn the_base_file_passes() {
    assert_eq!(errors(BASE), vec![]);
}

#[test]
fn the_three_examples_pass() {
    for name in [
        "ex1-the-night-train",
        "ex2-the-ninefold-cell",
        "ex3-the-fall-of-the-old-regime",
    ] {
        let path = format!("{}/examples/{name}.twee", env!("CARGO_MANIFEST_DIR"));
        let src = std::fs::read_to_string(&path).unwrap();
        let report = check(&src);
        assert!(report.ok(), "{name} has errors: {:#?}", report.diags);
        assert!(report.stats.passages > 20, "{name} lost its passages");
    }
}

#[test]
fn only_ex2_uses_stretch_features() {
    for (name, stretch) in [
        ("ex1-the-night-train", false),
        ("ex2-the-ninefold-cell", true),
        ("ex3-the-fall-of-the-old-regime", false),
    ] {
        let path = format!("{}/examples/{name}.twee", env!("CARGO_MANIFEST_DIR"));
        let report = check(&std::fs::read_to_string(&path).unwrap());
        let notices = report
            .diags
            .iter()
            .filter(|d| d.severity == Severity::Notice)
            .count();
        assert_eq!(notices > 0, stretch, "{name}");
    }
}

#[test]
fn link_target_must_exist() {
    assert_broken(
        "[[Look around->hall:look]]",
        "[[Look around->hall:lok]]",
        "`hall:lok` is not a passage",
    );
}

#[test]
fn link_clause_must_be_a_clause_or_an_act() {
    assert_broken(
        "~ examine ~ when",
        "~ examin ~ when",
        "neither a Link clause nor a declared act",
    );
}

#[test]
fn taken_item_must_be_declared() {
    assert_broken("~ take key", "~ take kee", "`kee` is not a declared item");
}

#[test]
fn expressions_are_parsed() {
    assert_broken(
        "~ cost 2 when me.is(ann)",
        "~ cost 2 when me.is(ann",
        "expected `)`",
    );
    assert_broken(
        "[if me.knows.secret but not stair_clear]",
        "[if me.knows.secret but]",
        "stops too soon",
    );
    assert_broken(
        "open when stair_clear",
        "open when stair_clear ==",
        "stops too soon",
    );
}

#[test]
fn speaker_must_be_declared() {
    assert_broken(
        "@bob (softly)",
        "@rob (softly)",
        "`rob` is not a declared character, NPC or voice",
    );
}

#[test]
fn learned_clue_must_be_declared() {
    assert_broken(
        "learn secret",
        "learn secrets",
        "`secrets` is not a declared clue",
    );
}

#[test]
fn unknown_statement_is_reported() {
    assert_broken("learn secret", "lern secret", "unknown statement `lern`");
}

#[test]
fn unknown_modifier_is_reported() {
    assert_broken("[end]", "[fin]", "unknown modifier `[fin]`");
}

#[test]
fn else_needs_an_if() {
    assert_broken_elsewhere(
        "[if me.knows.secret but not stair_clear]",
        "[note]",
        "`[else]` has no `[if …]`",
    );
}

#[test]
fn inserts_are_checked() {
    assert_broken("{present}", "{presnt here}", "not a known insert");
    assert_broken("{ann.name}", "{ann.name", "never closed");
}

#[test]
fn places_in_ways_must_be_declared() {
    assert_broken(
        "great_hall - cellar",
        "great_hall - celar",
        "`celar` is not a declared place",
    );
}

#[test]
fn events_are_checked() {
    assert_broken("on enter cellar", "on entr cellar", "is not an event");
    assert_broken(
        "on enter cellar",
        "on enter celar",
        "`celar` is not a declared place",
    );
    assert_broken("then say bob", "then sya bob", "`sya` is not an effect");
}

#[test]
fn next_must_name_a_passage() {
    assert_broken("next: hall", "next: hal", "`hal` is not a passage");
}

#[test]
fn config_rules() {
    assert_broken("kind: story", "kind: novel", "`kind` is one of");
    assert_broken(
        "pov: ann",
        "pov: anne",
        "`anne` is not a declared character",
    );
    let src = BASE.replacen("mode: solo", "mode: solo\ncohort: 2..=4", 1);
    let errs = errors(&src);
    assert!(
        errs.iter()
            .any(|(_, m)| m.contains("already means one Linker")),
        "{errs:#?}"
    );
}

#[test]
fn story_data_is_checked() {
    assert_broken_elsewhere(
        "\"format-version\": \"0.5.0\"",
        "\"format-version\": \"0.4.0\"",
        "targets Inkjot 0.4.0",
    );
    assert_broken_elsewhere(
        "\"start\": \"hall\"",
        "\"start\": \"hal\"",
        "`hal` is not a passage",
    );
}

#[test]
fn duplicates_are_reported() {
    let src = format!("{BASE}\n:: hall\nAgain.\n");
    assert!(errors(&src)
        .iter()
        .any(|(_, m)| m.contains("two passages named `hall`")));
    let src = BASE.replacen("clue secret:", "item key: \"another\"\nclue secret:", 1);
    assert!(errors(&src)
        .iter()
        .any(|(_, m)| m.contains("item `key` is declared twice")));
}

#[test]
fn a_stray_continuation_is_reported() {
    assert_broken_elsewhere("You look.", "\n~ cost 2", "nothing on the line before");
}

#[test]
fn columns_point_at_the_problem() {
    let src = BASE.replacen("[[Look around->hall:look]]", "[[Look around->nowhere]]", 1);
    let d = check(&src)
        .diags
        .into_iter()
        .find(|d| d.severity == Severity::Error)
        .unwrap();
    let line = src.lines().nth(d.line - 1).unwrap();
    let at: String = line.chars().skip(d.col - 1).collect();
    assert!(
        at.starts_with("nowhere"),
        "column {} points at `{at}`",
        d.col
    );
}

// ---- Bugs found by the property tests in `properties.rs`, pinned here. ----

#[test]
fn a_nameless_passage_reports_inside_the_line() {
    // Found by `diagnostics_point_inside_noise`: the column was hard-coded to 4,
    // which is past the end of a bare `::` line.
    let src = "::\n\n::\n\n";
    for d in check(src).diags.iter().filter(|d| d.line > 0) {
        let width = src.lines().nth(d.line - 1).unwrap().chars().count();
        assert!(
            d.col <= width + 1,
            "column {} on a line {width} wide",
            d.col
        );
    }
}

#[test]
fn a_lone_bracket_in_a_header_is_an_error_not_a_panic() {
    // Found by `diagnostics_point_inside_the_file`: slicing `text[1..len - 1]`
    // panicked when the header line was just `[`.
    let src = BASE.replacen("learn secret", "[", 1);
    let errs = errors(&src);
    assert!(
        errs.iter()
            .any(|(_, m)| m.contains("missing its closing `]`")),
        "{errs:#?}"
    );
}

#[test]
fn play_with_no_passage_reports_inside_the_line() {
    // Found by `diagnostics_point_inside_the_file`: `then play` with nothing
    // after it reported a column past the end of the line.
    let src = BASE.replacen("~ then say bob \"Mind the step.\"", "~ then play", 1);
    let d = check(&src)
        .diags
        .into_iter()
        .find(|d| d.severity == Severity::Error)
        .unwrap();
    assert!(d.msg.contains("`play` needs a passage name"), "{}", d.msg);
    let width = src.lines().nth(d.line - 1).unwrap().chars().count();
    assert!(d.col <= width + 1);
}

// ---- Names inside expressions ----

#[test]
fn names_in_expressions_must_resolve() {
    assert_broken(
        "open when stair_clear",
        "open when stair_cleer",
        "`stair_cleer` is not declared",
    );
    assert_broken(
        "[if me.knows.secret but",
        "[if me.knows.secrt but",
        "`secrt` is not a declared clue",
    );
    assert_broken(
        "when me.is(ann)",
        "when me.is(anne)",
        "`anne` is not declared",
    );
    assert_broken("{ann.name}", "{anne.name}", "`anne` is not declared");
    assert_broken(
        "when not stair_clear\n",
        "when nott(stair_clear)\n",
        "`nott` is not a function",
    );
}

#[test]
fn a_misspelling_gets_a_suggestion() {
    assert_broken(
        "open when stair_clear",
        "open when stair_cleer",
        "did you mean `stair_clear`?",
    );
    assert_broken(
        "[if me.knows.secret but",
        "[if me.knows.secrt but",
        "did you mean `secret`?",
    );
}

#[test]
fn the_error_points_at_the_name() {
    let src = BASE.replacen("when me.is(ann)", "when me.is(anne)", 1);
    let d = check(&src)
        .diags
        .into_iter()
        .find(|d| d.severity == Severity::Error)
        .unwrap();
    let at: String = src
        .lines()
        .nth(d.line - 1)
        .unwrap()
        .chars()
        .skip(d.col - 1)
        .collect();
    assert!(at.starts_with("anne"), "column {} points at `{at}`", d.col);
}

#[test]
fn lets_are_in_scope_in_their_passage_and_its_sub_nibs() {
    // `ready` is declared in `hall`; `hall:look` is a sub-Nib, so it may use it.
    let src = BASE
        .replacen(
            "learn secret\n--",
            "learn secret\nlet ready: me.knows.secret\n--",
            1,
        )
        .replacen("You look.", "[if ready]\nYou look.", 1);
    assert_eq!(errors(&src), vec![]);
    // A passage that is not a sub-Nib of `hall` may not.
    let src = format!("{src}\n:: elsewhere\n[if ready]\nNo.\n");
    let errs = errors(&src);
    assert!(
        errs.iter()
            .any(|(_, m)| m.contains("`ready` is not declared")),
        "{errs:#?}"
    );
}

#[test]
fn a_bare_word_after_a_comparison_is_a_value() {
    let src = BASE.replacen("when me.knows.secret\n", "when me.mood == cheerful\n", 1);
    assert_eq!(errors(&src), vec![]);
}

#[test]
fn link_parameters_are_in_scope_in_the_target() {
    let src = BASE
        .replacen(
            "~ examine ~ when me.knows.secret",
            "~ examine ~ with {thing: key}",
            1,
        )
        .replacen("You look.", "You look at {thing.name}.", 1);
    assert_eq!(errors(&src), vec![]);
    let src = src.replacen("{thing.name}", "{thng.name}", 1);
    assert!(
        errors(&src)
            .iter()
            .any(|(_, m)| m.contains("did you mean `thing`?")),
        "{:#?}",
        errors(&src)
    );
}
