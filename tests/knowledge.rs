//! Milestone 21e's slice of knowledge bases: declared, and facts kept.
//! Inklog's rules and questions are milestone 22's.

use inkjot::run::test::run_all;

const NOTES: &str = r#":: StoryTitle
Notes

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "desk"}

:: Inklink [config]
kind: story
mode: solo
pov: ann
character ann: "Ann" ~ selectable
npc hale: "Hale"
npc ashe: "Ashe"
period hour: nine, ten
knowledge notebook ~ per character ~ learned
knowledge record

:: desk
> [[Write it down->desk]] ~ then assert {{ hale is ashe }} in notebook
> [[File it->desk]] ~ then assert {{ hale seen @ ten }} in record

:: facts_are_kept_once_each [test]
start: desk
do: "Write it down"
do: "Write it down"
expect: count(ann.notebook) == 1
expect: first(first(ann.notebook)) == hale and last(first(ann.notebook)) == ashe
do: "File it"
expect: count(record) == 1
expect: count(hale.notebook) == 0
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
fn it_checks_clean_and_keeps_facts() {
    assert_eq!(errors(NOTES), Vec::<String>::new());
    let ir = inkjot::build(NOTES).unwrap();
    for o in run_all(NOTES, &ir).unwrap() {
        assert!(o.passed(), "`{}`: {:#?}", o.name, o.failures);
    }
}

#[test]
fn the_fact_is_in_the_ir() {
    let ir = inkjot::build(NOTES).unwrap();
    let text: String = ir.to_string().split_whitespace().collect();
    for want in [
        r#"{"do":"assert","fact":{"subject":"hale","verb":"is","object":"ashe","at":null},"in":"notebook""#,
        r#""fact":{"subject":"hale","verb":"seen","object":null,"at":"ten"}"#,
        r#""declare":"knowledge","id":"notebook","per":"character""#,
    ] {
        assert!(text.contains(want), "the IR should have {want}");
    }
}

#[test]
fn mistakes_are_refused() {
    let says = |from: &str, to: &str, what: &str| {
        let bad = NOTES.replace(from, to);
        assert!(
            errors(&bad).iter().any(|e| e.contains(what)),
            "`{to}` should say {what:?}: {:?}",
            errors(&bad)
        );
    };
    says(
        "in notebook",
        "in notbook",
        "`notbook` is not a declared knowledge base",
    );
    says(
        "{{ hale is ashe }}",
        "{{ hale is ashe now }}",
        "two terms or three",
    );
    says("{{ hale is ashe }}", "{{ P is ashe }}", "Inklog variable");
    says(
        "assert {{ hale is ashe }} in notebook",
        "assert hale is ashe",
        "`assert` is written",
    );
}
