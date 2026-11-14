//! Property-based tests: claims that must hold for *every* input, checked
//! against hundreds of generated ones.
//!
//! A plain test says "this input gives that output". A property says "no input
//! of this shape can ever break this rule", and proptest goes looking for one
//! that does. When it finds one it shrinks it to the smallest failing case and
//! saves it under `proptest-regressions/`, so it is retried on every run.

use inkjot::expr::{Arg, Expr};
use inkjot::json::{self, Value};
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

/// An expression *tree*, built directly instead of parsed from text.
///
/// The round-trip property needs trees the parser never chose the shape of,
/// so the printer has to cope with every nesting, not only the ones that
/// arise from text someone would write.
fn tree() -> impl Strategy<Value = Expr> {
    use proptest::collection::vec;
    use proptest::sample::select;

    let leaf = prop_oneof![
        select(vec!["me", "here", "door", "x", "a_b", "lounge@0"]).prop_map(Expr::name),
        (0..1000u32).prop_map(|n| Expr::Num(n.to_string())),
        (0..100u32, 0..100u32).prop_map(|(a, b)| Expr::Num(format!("{a}.{b}"))),
        "[a-z ]{0,6}".prop_map(Expr::Str),
    ];
    // Up to 6 levels deep and about 64 nodes: deep enough that operators nest
    // inside each other on both sides, which is where brackets matter.
    leaf.prop_recursive(6, 64, 3, |inner| {
        let boxed = || inner.clone().prop_map(Box::new);
        // After a dot any word is a member, keywords included.
        let member = select(vec!["knows", "open", "in", "and", "any", "now"]);
        let key = proptest::option::of(select(vec!["in", "max", "toward"]));
        let arg = (key, inner.clone()).prop_map(|(key, value)| Arg {
            key: key.map(str::to_string),
            value,
        });
        let binary = select(vec![
            "or", "and", "==", "!=", ">=", "<=", ">", "<", "in", "+", "-", "*", "/", "mod",
        ]);
        let range_op = select(vec!["..=", "..<"]);
        prop_oneof![
            (boxed(), member).prop_map(|(t, m)| Expr::Member(t, m.to_string())),
            (boxed(), vec(arg, 0..3)).prop_map(|(t, args)| Expr::Call(t, args)),
            (boxed(), boxed()).prop_map(|(t, i)| Expr::Index(t, i)),
            vec(inner.clone(), 0..4).prop_map(Expr::List),
            (select(vec!["not", "-"]), boxed()).prop_map(|(op, e)| Expr::Unary(op, e)),
            (binary, boxed(), boxed()).prop_map(|(op, a, b)| Expr::Binary(op, a, b)),
            (range_op.clone(), boxed(), boxed()).prop_map(|(op, lo, hi)| Expr::Range {
                lo: Some(lo),
                hi: Some(hi),
                op
            }),
            (range_op, boxed()).prop_map(|(op, hi)| Expr::Range {
                lo: None,
                hi: Some(hi),
                op
            }),
            boxed().prop_map(|lo| Expr::Range {
                lo: Some(lo),
                hi: None,
                op: ".."
            }),
        ]
    })
}

/// Any JSON value: every kind, any nesting, any characters in strings.
fn json_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i32>().prop_map(|n| Value::Num(n.to_string())),
        (any::<i16>(), 0..1000u32).prop_map(|(a, b)| Value::Num(format!("{a}.{b}"))),
        any::<String>().prop_map(Value::Str),
    ];
    leaf.prop_recursive(4, 48, 4, |inner| {
        prop_oneof![
            proptest::collection::vec(inner.clone(), 0..4).prop_map(Value::Arr),
            proptest::collection::vec((any::<String>(), inner), 0..4).prop_map(Value::Obj),
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

proptest! {
    // These are cheap, so run many more cases than the default 256. A printer
    // bug only shows on particular nestings (the same operator on the right
    // of itself, say), and 256 shallow trees can miss it.
    #![proptest_config(ProptestConfig::with_cases(4000))]

    /// Property 1, the round trip: print any tree, parse the text, and get an
    /// equal tree back. This pins the printer and the parser to each other:
    /// a missing bracket in one or a precedence slip in the other breaks it.
    #[test]
    fn printing_then_parsing_gives_the_same_tree(tree in tree()) {
        let text = tree.to_string();
        let reparsed = expr::parse(&text);
        prop_assert_eq!(reparsed.as_ref(), Ok(&tree), "printed as: {}", text);
    }

    /// Printing is stable: text that came from the printer prints as itself.
    #[test]
    fn printing_is_stable(src in expression()) {
        let once = expr::parse(&src).unwrap().to_string();
        let twice = expr::parse(&once).unwrap().to_string();
        prop_assert_eq!(once, twice);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// The JSON writer and reader agree: any value printed and read back is
    /// the same value. Strings may hold quotes, backslashes, newlines,
    /// control characters and any Unicode.
    #[test]
    fn json_survives_printing_and_reading(value in json_value()) {
        let text = value.to_string();
        let reread = json::parse(&text);
        prop_assert_eq!(reread.as_ref(), Ok(&value), "printed as: {}", text);
    }

    /// Lowering never panics, and what it produces is valid JSON.
    #[test]
    fn lowered_expressions_are_valid_json(tree in tree()) {
        let lowered = inkjot::ir::lower(&tree);
        let text = lowered.to_string();
        let reread = json::parse(&text);
        prop_assert_eq!(reread.as_ref(), Ok(&lowered), "printed as: {}", text);
    }
}

proptest! {
    /// `build` never panics: damaged source is either rejected with a report
    /// or compiled, and what is compiled is valid JSON.
    #[test]
    fn build_never_panics_on_damaged_examples(recipe in damage()) {
        if let Ok(ir) = inkjot::build(&damaged(&recipe)) {
            let text = ir.to_string();
            prop_assert!(json::parse(&text).is_ok());
        }
    }
}

// ------------------------------------------------------- `let`, `get` and cycles

/// A tiny story with one room whose header is `header`, a body, and one act
/// that raises `a`.
fn room_story(header: &str, body: &str, act: &str) -> String {
    format!(
        ":: StoryTitle\nRoom\n\n:: StoryData\n{{\"ifid\": \"x\", \"format\": \"Inkjot\", \"format-version\": \"0.5.0\", \"start\": \"room\"}}\n\n\
:: Inklink [config]\nkind: story\nmode: solo\npov: ann\ncharacter ann: \"Ann\" ~ playable\nfact a: 0\nfact b: 0\nfact c: 0\n\n\
:: room\n{header}--\n{body}\n> [[Bump->room:bump]]\n\n:: room:bump [act]\nraise a 1\n--\n{act}\n"
    )
}

/// Gets that read each other: `deps[i]` lists the gets `g{i}` reads.
fn dependencies() -> impl Strategy<Value = Vec<Vec<usize>>> {
    (1..7usize)
        .prop_flat_map(|n| proptest::collection::vec(proptest::collection::vec(0..n, 0..3), n))
}

/// The reference: does the graph have a cycle? (Depth-first search with
/// three colours.)
fn has_cycle(deps: &[Vec<usize>]) -> bool {
    fn visit(i: usize, deps: &[Vec<usize>], colour: &mut [u8]) -> bool {
        colour[i] = 1;
        for &j in &deps[i] {
            if colour[j] == 1 || (colour[j] == 0 && visit(j, deps, colour)) {
                return true;
            }
        }
        colour[i] = 2;
        false
    }
    let mut colour = vec![0; deps.len()];
    (0..deps.len()).any(|i| colour[i] == 0 && visit(i, deps, &mut colour))
}

/// A small arithmetic expression over the facts `a`, `b`, `c`, fully
/// parenthesized, with its value as a function of `(a, b, c)`.
#[derive(Debug, Clone)]
enum Arith {
    Num(i64),
    Fact(usize),
    Op(char, Box<Arith>, Box<Arith>),
}

impl Arith {
    fn source(&self) -> String {
        match self {
            Arith::Num(n) => n.to_string(),
            Arith::Fact(i) => ["a", "b", "c"][*i].to_string(),
            Arith::Op(op, l, r) => format!("({} {op} {})", l.source(), r.source()),
        }
    }

    fn value(&self, facts: [i64; 3]) -> i64 {
        match self {
            Arith::Num(n) => *n,
            Arith::Fact(i) => facts[*i],
            Arith::Op('+', l, r) => l.value(facts) + r.value(facts),
            Arith::Op('-', l, r) => l.value(facts) - r.value(facts),
            Arith::Op(_, l, r) => l.value(facts) * r.value(facts),
        }
    }
}

fn arith() -> impl Strategy<Value = Arith> {
    let leaf = prop_oneof![
        (0..10i64).prop_map(Arith::Num),
        (0..3usize).prop_map(Arith::Fact)
    ];
    leaf.prop_recursive(3, 12, 2, |inner| {
        (
            prop::sample::select(vec!['+', '-', '*']),
            inner.clone(),
            inner,
        )
            .prop_map(|(op, l, r)| Arith::Op(op, Box::new(l), Box::new(r)))
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// `inkjot check` reports a `get` that depends on itself exactly when the
    /// gets form a cycle, and the cycle it names is real: each `get` in it
    /// reads the next.
    #[test]
    fn get_cycles_are_found_exactly(deps in dependencies()) {
        let mut header = String::new();
        for (i, reads) in deps.iter().enumerate() {
            let mut terms: Vec<String> = reads.iter().map(|j| format!("g{j}")).collect();
            terms.push("a".into());
            header.push_str(&format!("get g{i}: {}\n", terms.join(" + ")));
        }
        let src = room_story(&header, "Here.", "Bumped.");
        let report = check(&src);
        let cycles: Vec<&str> = report.diags.iter()
            .map(|d| d.msg.as_str())
            .filter(|m| m.contains("depends on itself"))
            .collect();
        prop_assert_eq!(!cycles.is_empty(), has_cycle(&deps), "{:?}", cycles);
        for msg in cycles {
            // "… depends on itself: g0 → g2 → g0"
            let path: Vec<usize> = msg.rsplit(": ").next().unwrap_or("")
                .split(" → ")
                .filter_map(|g| g.trim().trim_start_matches('g').parse().ok())
                .collect();
            prop_assert!(path.len() >= 2 && path.first() == path.last(), "{}", msg);
            for pair in path.windows(2) {
                prop_assert!(deps[pair[0]].contains(&pair[1]), "g{} does not read g{}: {}", pair[0], pair[1], msg);
            }
        }
    }

    /// A `let` and a `get` of the same expression agree on arrival. After
    /// the act raises `a`, the `let` still has the old value and the `get`
    /// the new one.
    #[test]
    fn let_keeps_and_get_follows(e in arith()) {
        let header = format!("let x: {0}\nget y: {0}\n", e.source());
        let src = room_story(&header, "Then: {x} and {y}.", "Now: {x} and {y}.");
        let ir = inkjot::build(&src).map_err(|r| TestCaseError::fail(format!("{:?}", r.diags)))?;
        let outcomes = inkjot::run::test::run_all(
            &format!("{src}\n:: t [test]\nstart: room\ndo: \"Bump\"\n"), &ir,
        ).unwrap();
        let text = outcomes[0].transcript.join("\n");
        let before = e.value([0, 0, 0]);
        let after = e.value([1, 0, 0]);
        let then_line = format!("Then: {before} and {before}.");
        let now_line = format!("Now: {before} and {after}.");
        prop_assert!(text.contains(&then_line), "{}\n{}", e.source(), text);
        prop_assert!(text.contains(&now_line), "{}\n{}", e.source(), text);
    }
}

// -------------------------------------------------------------------- comments

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Comments are invisible: putting `((…))` lines anywhere in an example,
    /// and `~ ((…))` at the end of any statement line, leaves its IR exactly
    /// as it was.
    #[test]
    fn comments_change_nothing(which in 0..3usize, spots in proptest::collection::vec((any::<prop::sample::Index>(), any::<bool>()), 1..30)) {
        let src = EXAMPLES[which];
        let before = inkjot::build(src).map_err(|r| TestCaseError::fail(format!("{:?}", r.diags)))?;
        let mut lines: Vec<String> = src.lines().map(str::to_string).collect();
        for (spot, trailing) in spots {
            let i = spot.index(lines.len());
            let line = lines[i].clone();
            let t = line.trim();
            // A statement line (not prose, a Link, a `::` header or JSON)
            // may end in a comment; any line may be preceded by one.
            let statement = !t.is_empty() && t.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                && t.split_whitespace().next().is_some_and(|w| w.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
                && !t.contains("((");
            if trailing && statement {
                lines[i] = format!("{line} ~ ((a note: {i}))");
            } else if !t.starts_with("::") && !lines[..i].iter().all(|l| !l.starts_with("::")) {
                lines.insert(i, format!("((a note: {i}))"));
            }
        }
        let commented = lines.join("\n");
        let after = inkjot::build(&commented).map_err(|r| TestCaseError::fail(format!("{:?}", r.diags)))?;
        prop_assert!(before == after, "the IR changed");
    }
}
