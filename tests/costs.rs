//! Costs: what acts spend is the Inker's word, `default cost: …`, not the
//! engine's. Nothing in the engine is named after one Inklink.

use inkjot::run::test::run_all;
use inkjot::{check, migrate, Severity};

const MARKET: &str = r#":: StoryTitle
Market

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "stall"}

:: Inklink [config]
kind: game
mode: solo
pov: ann
character ann: "Ann" ~ selectable
place stall: "the stall"
resource coins: 5
resource breath: 3
default cost: coins
act haggle: cost 2
act shout: cost rest ~ min 1
act sprint: requires full coins ~ cost 1

:: stall
--
> [[Haggle->here]] ~ haggle
> [[Sprint->here]] ~ sprint
> [[Shout->here]] ~ shout
> [[Rest->here]] ~ cost 0

:: spending [test]
start: stall
do: "Haggle"
expect: ann.coins == 3 and ann.breath == 3 ~ ((coins are spent, not breath))
expect no link: "Sprint" ~ ((the purse is not full))
do: "Shout"
expect: ann.coins == 0
expect no link: "Haggle" ~ ((nothing left to spend))
expect link: "Rest"
"#;

fn diags(src: &str, severity: Severity) -> Vec<String> {
    check(src)
        .diags
        .into_iter()
        .filter(|d| d.severity == severity)
        .map(|d| d.msg)
        .collect()
}

fn passes(src: &str) {
    let ir = inkjot::build(src).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    for outcome in run_all(src, &ir).unwrap() {
        assert!(
            outcome.passed(),
            "{:#?}\n{}",
            outcome.failures,
            outcome.transcript.join("\n")
        );
    }
}

#[test]
fn acts_spend_the_resource_the_config_names() {
    assert_eq!(diags(MARKET, Severity::Error), Vec::<String>::new());
    passes(MARKET);
}

#[test]
fn the_ir_says_how_much_not_of_what() {
    let ir = inkjot::build(MARKET).unwrap();
    let text = ir.to_string();
    assert!(text.contains(r#""amount""#), "{text}");
    assert!(!text.contains(r#""vults""#), "{text}");
}

#[test]
fn without_a_default_cost_nothing_is_spent_and_the_checker_says_so() {
    let free = MARKET
        .replace("default cost: coins\n", "")
        .replace(
            "expect: ann.coins == 3 and ann.breath == 3 ~ ((coins are spent, not breath))",
            "expect: ann.coins == 5",
        )
        .replace(
            "expect no link: \"Sprint\" ~ ((the purse is not full))\n",
            "",
        )
        .replace("expect: ann.coins == 0", "expect: ann.coins == 5")
        .replace(
            "expect no link: \"Haggle\" ~ ((nothing left to spend))",
            "expect link: \"Haggle\"",
        );
    assert_eq!(diags(&free, Severity::Error), Vec::<String>::new());
    let notices = diags(&free, Severity::Notice);
    assert!(
        notices.iter().any(|n| n.contains("`default cost: vults`")),
        "{notices:?}"
    );
    passes(&free);
}

#[test]
fn a_default_cost_must_name_a_resource() {
    let typo = MARKET.replace("default cost: coins", "default cost: cions");
    assert_eq!(
        diags(&typo, Severity::Error),
        ["`default cost` names the resource acts spend; `cions` is not a declared resource"]
    );
}

#[test]
fn hushed_is_the_inkers_word_not_the_engines() {
    let src = MARKET.replace(
        "act haggle: cost 2",
        "act haggle: cost 2\non loud when not hushed\n~ then set ann.coins: 0",
    );
    let errs = diags(&src, Severity::Error);
    assert!(errs.iter().any(|e| e.contains("`hushed`")), "{errs:?}");
    let declared = src.replace("resource breath: 3", "resource breath: 3\nstate hushed: no");
    assert_eq!(diags(&declared, Severity::Error), Vec::<String>::new());
}

#[test]
fn migrating_a_v05_source_names_vults_as_the_cost() {
    let v05 = MARKET
        .replace("0.6.0", "0.5.0")
        .replace("default cost: coins\n", "")
        .replace("resource coins: 5", "resource vults: 5")
        .replace("full coins", "full vults")
        .replace("ann.coins", "ann.vults");
    let m = migrate::migrate(&v05);
    assert!(
        m.source
            .contains("resource vults: 5\ndefault cost: vults\n"),
        "{}",
        m.source
    );
    assert_eq!(diags(&m.source, Severity::Error), Vec::<String>::new());
    passes(&m.source);
}
