//! Property-based tests: claims that must hold for *every* input, checked
//! against hundreds of generated ones.
//!
//! A plain test says "this input gives that output". A property says "no input
//! of this shape can ever break this rule", and proptest goes looking for one
//! that does. When it finds one it shrinks it to the smallest failing case and
//! saves it under `proptest-regressions/`, so it is retried on every run.

use inkjot::{check, expr, render, Severity};
use proptest::prelude::*;

const EXAMPLES: [&str; 3] = [
    include_str!("../examples/ex1-the-night-train.twee"),
    include_str!("../examples/ex2-the-ninefold-cell.twee"),
    include_str!("../examples/ex3-the-fall-of-the-old-regime.twee"),
];

/// Characters that mean something to Inkjot. Random damage made of these is
/// far more likely to reach an interesting code path than random Unicode.
const SHARP: &[char] = &[
    '~', '[', ']', '{', '}', '(', ')', '"', '\'', ':', '@', '>', '<', '-', '.', ',', '=', '!',
    '\n', ' ', '|', '’', '“', 'é', '…',
];

// ------------------------------------------------------------------ generators

/// One random edit to a source text.
#[derive(Debug, Clone)]
enum Edit {
    /// Delete `len` characters starting at a position.
    Delete { at: f64, len: usize },
    /// Insert some sharp characters at a position.
    Insert { at: f64, text: String },
    /// Cut the file off at a position.
    Truncate { at: f64 },
    /// Swap two lines.
    SwapLines { a: f64, b: f64 },
}

fn edit() -> impl Strategy<Value = Edit> {
    let at = || 0.0..1.0f64;
    let sharp = proptest::collection::vec(proptest::sample::select(SHARP), 1..6)
        .prop_map(|cs| cs.into_iter().collect::<String>());
    prop_oneof![
        (at(), 1..40usize).prop_map(|(at, len)| Edit::Delete { at, len }),
        (at(), sharp).prop_map(|(at, text)| Edit::Insert { at, text }),
        at().prop_map(|at| Edit::Truncate { at }),
        (at(), at()).prop_map(|(a, b)| Edit::SwapLines { a, b }),
    ]
}

/// Apply an edit. Works on characters, so it can never split a UTF-8 sequence.
fn apply(src: &str, edit: &Edit) -> String {
    let index = |at: f64, len: usize| ((at * len as f64) as usize).min(len.saturating_sub(1));
    let mut chars: Vec<char> = src.chars().collect();
    match edit {
        Edit::Delete { at, len } => {
            let start = index(*at, chars.len());
            let end = (start + len).min(chars.len());
            chars.drain(start..end);
        }
        Edit::Insert { at, text } => {
            let start = index(*at, chars.len());
            chars.splice(start..start, text.chars());
        }
        Edit::Truncate { at } => chars.truncate(index(*at, chars.len())),
        Edit::SwapLines { a, b } => {
            let mut lines: Vec<&str> = src.lines().collect();
            if lines.is_empty() {
                return String::new();
            }
            let (a, b) = (index(*a, lines.len()), index(*b, lines.len()));
            lines.swap(a, b);
            return lines.join("\n");
        }
    }
    chars.into_iter().collect()
}

/// Which example to damage, and one to four edits to make to it.
///
/// The strategy yields the *recipe*, not the damaged text. A failure report
/// then shows a few small edits instead of a whole 50 KB file.
fn damage() -> impl Strategy<Value = (usize, Vec<Edit>)> {
    (0..EXAMPLES.len(), proptest::collection::vec(edit(), 1..5))
}

fn damaged((which, edits): &(usize, Vec<Edit>)) -> String {
    edits
        .iter()
        .fold(EXAMPLES[*which].to_string(), |src, e| apply(&src, e))
}

/// A well-formed Inkjot expression, as source text.
///
/// `prop_recursive` builds small expressions first and wraps them in bigger
/// ones, which is also the order proptest shrinks back through.
fn expression() -> impl Strategy<Value = String> {
    let name = proptest::sample::select(vec![
        "me",
        "here",
        "vell",
        "clock",
        "trust",
        "x",
        "door_open",
    ]);
    let member = proptest::sample::select(vec!["knows", "holds", "now", "open", "in", "is", "any"]);
    let leaf = prop_oneof![
        name.clone().prop_map(str::to_string),
        (0..100u32).prop_map(|n| n.to_string()),
        Just("yes".to_string()),
        Just("\"a string\"".to_string()),
        (name.clone(), member.clone()).prop_map(|(n, m)| format!("{n}.{m}")),
        (name, member, 0..9u32).prop_map(|(n, m, a)| format!("{n}.{m}(x, in: here, max: {a})")),
        (0..9u32, 0..9u32).prop_map(|(a, b)| format!("{a}..={}", a + b)),
    ];
    leaf.prop_recursive(4, 32, 3, |inner| {
        let binary = proptest::sample::select(vec![
            "and", "or", "==", "!=", ">=", "<", "+", "-", "*", "mod", "in",
        ]);
        prop_oneof![
            (inner.clone(), binary, inner.clone())
                .prop_map(|(a, op, b)| format!("({a}) {op} ({b})")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("{a} and {b}")),
            inner.clone().prop_map(|a| format!("not ({a})")),
            proptest::collection::vec(inner, 0..4)
                .prop_map(|items| format!("[{}]", items.join(", "))),
        ]
    })
}

// ------------------------------------------------------------------ properties

/// Every diagnostic must point inside the file it is about.
fn assert_positions_are_inside(src: &str) -> Result<(), TestCaseError> {
    let lines: Vec<&str> = src.lines().collect();
    for d in check(src).diags {
        if d.line == 0 {
            continue; // about the file as a whole
        }
        prop_assert!(
            d.line <= lines.len(),
            "line {} is past the end: {}",
            d.line,
            d.msg
        );
        let width = lines[d.line - 1].chars().count();
        prop_assert!(
            d.col >= 1 && d.col <= width + 1,
            "column {} is outside line {} (width {width}): {}",
            d.col,
            d.line,
            d.msg
        );
        // Rendering must not panic either, whatever the line contains.
        prop_assert!(!render(&d, "file.twee", src).is_empty());
    }
    Ok(())
}

proptest! {
    /// Property 2: `check` returns a report for any text at all. It never panics.
    #[test]
    fn check_never_panics_on_arbitrary_text(src in "\\PC{0,400}") {
        let _ = check(&src);
    }

    /// Property 2, where it bites: text that is *almost* Inkjot.
    #[test]
    fn check_never_panics_on_damaged_examples(recipe in damage()) {
        let _ = check(&damaged(&recipe));
    }

    /// Property 3: every reported line and column lies inside the file.
    #[test]
    fn diagnostics_point_inside_the_file(recipe in damage()) {
        assert_positions_are_inside(&damaged(&recipe))?;
    }

    /// Property 3 again, for text built only from sharp characters and words.
    #[test]
    fn diagnostics_point_inside_noise(src in "(:: [a-z]{1,6}( \\[[a-z ]{0,8}\\])?\n)?([a-z~\\[\\]{}()\":@> .,=\n-]{0,60}\n){0,12}") {
        assert_positions_are_inside(&src)?;
    }

    /// The expression parser never panics, whatever it is given.
    #[test]
    fn the_expression_parser_never_panics(src in "[a-z0-9_ .,:()\\[\\]\"=!<>+*/-]{0,60}") {
        let _ = expr::parse(&src);
    }

    /// Generated expressions are well formed, so they must parse.
    #[test]
    fn well_formed_expressions_parse(src in expression()) {
        prop_assert!(expr::parse(&src).is_ok(), "{src}: {:?}", expr::parse(&src));
    }

    /// Property 4: `but` is an exact synonym for `and`.
    #[test]
    fn but_and_and_agree(src in expression()) {
        let with_but = src.replace(" and ", " but ");
        prop_assert_eq!(expr::parse(&src), expr::parse(&with_but));
    }

    /// A parse error's offset always lies inside the text it is about.
    #[test]
    fn parse_error_offsets_are_inside(src in "[a-z0-9_ .,:()\\[\\]\"=!<>+*/-]{0,60}") {
        if let Err((offset, _)) = expr::parse(&src) {
            prop_assert!(offset <= src.len(), "offset {offset} is past the end of `{src}`");
            prop_assert!(src.is_char_boundary(offset));
        }
    }
}

/// Property 4: `upon` is an exact synonym for `on`.
///
/// This one is not random: the claim is about three known files, so it checks
/// them directly.
#[test]
fn upon_and_on_agree() {
    for src in EXAMPLES {
        let with_upon: String = src
            .lines()
            .map(|line| match line.strip_prefix("on ") {
                Some(rest) => format!("upon {rest}"),
                None => line.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n");
        let errors = |s: &str| {
            check(s)
                .diags
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .count()
        };
        assert_eq!(errors(src), errors(&with_upon));
        assert_eq!(
            check(src).stats.expressions,
            check(&with_upon).stats.expressions
        );
    }
}
