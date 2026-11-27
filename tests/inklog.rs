//! Milestone 22a, Inklog: facts asked of bases, rules to a fixed point,
//! `not` where the world is closed, quantifiers, what a character knows,
//! and the bases a story declares. (21e's store, `assert … in`, is the
//! ground it stands on.)

use inkjot::run::test::run_all;

const GARDEN: &str = r#":: StoryTitle
Garden

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "garden"}

:: Inklink [config]
kind: story
mode: solo
pov: ann
character ann: "Ann" ~ selectable
npc hale: "Hale"
npc ashe: "Ashe"
knowledge world ~ truth
~ starts with {{ nw soil clay }}, {{ e soil sand }}
knowledge notebook ~ per character ~ manual ~ reads knows
knowledge record
knowledge dossier ~ for hale ~ learned
~ starts with {{ ann harmless }}
fact beds: "The north-west bed is clay; the east, sand." ~ means {{ nw soil clay }}, {{ e soil sand }}
fact names: "Hale is Ashe." ~ means {{ hale is ashe }}

:: garden_rules [rules]
rule wet: {{ sky rains }} and {{ B soil S }} => {{ B watered }}
rule slugs: {{ B watered }} and {{ B soil clay }} and not {{ B guarded }} => {{ B slugged }} ~ in world
rule hopes: {{ B soil S }} => {{ B hoped }} ~ in knows ~ learned from beds
rule both: {{ N is P }} and {{ N is Q }} and P != Q => {{ N confused }} ~ in record

:: garden
The garden.
> [[Rain->garden]] ~ then assert {{ sky rains }}
> [[Clear->garden]] ~ then retract {{ sky rains }}
> [[Guard the clay->garden]] ~ then assert {{ nw guarded }}
> [[Read about the beds->garden]] ~ then learn beds
> [[Write it down->garden]] ~ then assert {{ hale is ashe }} in notebook
> [[File two names->garden]] ~ then assert {{ hale is ashe }} in record ~ then assert {{ hale is lark }} in record

:: rules_make_facts [test]
start: garden
expect: not {{ nw watered }}
do: "Rain"
expect: {{ nw watered }} and {{ e watered }}
expect: count({{ ? watered }}) == 2
expect: {{ nw slugged }} and not {{ e slugged }}

:: not_asks_the_closed_world [test]
start: garden
do: "Rain"
do: "Guard the clay"
expect: {{ nw watered }} and not {{ nw slugged }}

:: what_follows_goes_with_what_it_followed_from [test]
start: garden
do: "Rain"
do: "Clear"
expect: not {{ nw watered }} and not {{ nw slugged }}

:: a_character_knows_what_they_have_learned [test]
start: garden
expect: not {{ nw soil clay }} in knows
expect: not {{ nw hoped }} in knows
do: "Read about the beds"
expect: {{ nw soil clay }} in knows
expect: {{ nw hoped }} in knows
expect: not {{ nw hoped }}
(((
The notebook reads what you know, and holds what you write.
)))
expect: {{ e soil sand }} in notebook
do: "Write it down"
expect: {{ hale is ashe }} in notebook
expect: not {{ hale is ashe }} in record

:: quantifiers_end_with_a_dot [test]
start: garden
expect: some B. {{ B soil clay }}
expect: no B. {{ B soil loam }}
expect: some B, S. {{ B soil S }} and S != clay
expect: not (some B. {{ B soil clay }} and {{ B soil sand }})

:: a_base_of_someone_elses [test]
start: garden
expect: {{ you harmless }} in dossier
expect: {{ ? harmless }} in dossier

:: rules_in_a_declared_base [test]
start: garden
do: "File two names"
expect: {{ hale confused }} in record
expect: not {{ hale confused }}
"#;

fn errors(src: &str) -> Vec<String> {
    inkjot::check(src)
        .diags
        .into_iter()
        .filter(|d| d.severity == inkjot::Severity::Error)
        .map(|d| d.msg)
        .collect()
}

#[test]
fn it_checks_clean() {
    assert_eq!(errors(GARDEN), Vec::<String>::new());
}

#[test]
fn its_tests_pass() {
    let ir = inkjot::build(GARDEN).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    for o in run_all(GARDEN, &ir).unwrap() {
        assert!(
            o.passed(),
            "`{}`: {:#?}\n{}",
            o.name,
            o.failures,
            o.transcript.join("\n")
        );
    }
}

#[test]
fn mistakes_are_refused() {
    let says = |from: &str, to: &str, what: &str| {
        let bad = GARDEN.replace(from, to);
        assert!(
            errors(&bad).iter().any(|e| e.contains(what)),
            "`{to}` should say {what:?}: {:?}",
            errors(&bad)
        );
    };
    says(
        "not {{ B guarded }} => {{ B slugged }} ~ in world",
        "not {{ B guarded }} => {{ B slugged }} ~ in record",
        "`not` is for a base",
    );
    says(
        "~ learned from beds",
        "~ learned from bedz",
        "`bedz` is not a declared fact",
    );
    says("=> {{ B watered }}", "=> B", "a rule ends `=> {{ … }}`");
    says(
        "expect: {{ you harmless }} in dossier",
        "expect: {{ you harmless }} in dosier",
        "`dosier` is not a declared knowledge",
    );
    says("{{ sky rains }} and", "{{ sky }} and", "verb");
}

use inkjot::run::{Run, Story};
use proptest::prelude::*;
use std::sync::Arc;

/// A world of `edges` and the rule that makes `above` transitive.
fn tower(edges: &[(u8, u8)]) -> String {
    let facts: Vec<String> = edges
        .iter()
        .map(|(a, b)| format!("{{{{ n{a} above n{b} }}}}"))
        .collect();
    let starts = if facts.is_empty() {
        String::new()
    } else {
        format!("~ starts with {}\n", facts.join(", "))
    };
    format!(
        r#":: StoryTitle
Tower

:: StoryData
{{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "top"}}

:: Inklink [config]
kind: story
mode: solo
pov: ann
character ann: "Ann" ~ selectable
knowledge world ~ truth
{starts}rule up: {{{{ A above B }}}} and {{{{ B above C }}}} => {{{{ A above C }}}}
rule root: {{{{ A above B }}}} and not {{{{ C above A }}}} => {{{{ A tops }}}}

:: top
The top.
"#
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Rules run to their fixed point: `above` is the transitive closure of
    /// the edges, as Warshall's algorithm finds it, and `tops` is every node
    /// with something below it and nothing above it.
    #[test]
    #[allow(clippy::needless_range_loop)]
    fn rules_reach_the_closure(edges in prop::collection::vec((0u8..5, 0u8..5), 0..8)) {
        let src = tower(&edges);
        let ir = inkjot::build(&src).map_err(|r| TestCaseError::fail(format!("{:?}", r.diags)))?;
        let mut run = Run::new(Arc::new(Story::new(&ir).unwrap()), 1);
        run.start(None, None).unwrap();
        let mut reach = [[false; 5]; 5];
        for (a, b) in &edges {
            reach[*a as usize][*b as usize] = true;
        }
        for k in 0..5 {
            for i in 0..5 {
                for j in 0..5 {
                    if reach[i][k] && reach[k][j] {
                        reach[i][j] = true;
                    }
                }
            }
        }
        for i in 0..5 {
            for j in 0..5 {
                let q = format!("{{{{ n{i} above n{j} }}}}");
                let got = run.eval(&inkjot::ir::lower_expression(&q)).unwrap().truthy();
                prop_assert_eq!(got, reach[i][j], "n{} above n{}", i, j);
            }
            let tops = (0..5).any(|j| reach[i][j]) && !(0..5).any(|c| reach[c][i]);
            let q = format!("{{{{ n{i} tops }}}}");
            prop_assert_eq!(run.eval(&inkjot::ir::lower_expression(&q)).unwrap().truthy(), tops, "n{} tops", i);
        }
    }
}
