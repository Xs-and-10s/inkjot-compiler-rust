//! Pacing in the view: `[after 2s]`, `[beat 2s]` and `[verse]` reach the
//! paragraphs a runtime reads, so it can pace the text without the IR.

use inkjot::engine::Pause;
use inkjot::engine::{Inklink, Start};

const PACED: &str = r#":: StoryTitle
Paced

:: StoryData
{"ifid": "x", "format": "Inkjot", "format-version": "0.6.0", "start": "lounge"}

:: Inklink [config]
kind: story
mode: solo
pov: ann
character ann: "Ann" ~ selectable
npc leo: "Leo"
state hidden: no

:: lounge
He winds it once, twice—

[after 2s; append]
The lights die.

[after 4s]
When they return, the Professor lies across the table.

[after 1s]
[if hidden]
This is never shown, so its pause goes with it.

[end]
@leo Do you come back from the big vet?
[beat 1.5s]
@leo No.

[verse]
Flame for the fool,
and frost for the brave.

[end]
After the verse.
"#;

fn pause(ms: u32, kind: &'static str) -> Option<Pause> {
    Some(Pause { ms, kind })
}

#[test]
fn pauses_and_verse_reach_the_view() {
    let ir = inkjot::build(PACED).unwrap_or_else(|r| panic!("{:#?}", r.diags));
    let state = Inklink::from_ir(&ir)
        .unwrap()
        .start(1, &Start::default())
        .unwrap();
    let text = &state.view().text;
    let plain: Vec<String> = text.iter().map(|p| p.plain()).collect();
    assert_eq!(
        plain,
        [
            "He winds it once, twice— The lights die.",
            "When they return, the Professor lies across the table.",
            "Leo: Do you come back from the big vet?",
            "Leo: No.",
            "Flame for the fool,",
            "and frost for the brave.",
            "After the verse.",
        ]
    );
    // Appended after a pause: the wait is on the run, mid-paragraph.
    assert_eq!(text[0].pause, None);
    assert_eq!(text[0].runs.len(), 2);
    assert_eq!(text[0].runs[1].pause, pause(2000, "after"));
    assert_eq!(text[1].pause, pause(4000, "after"));
    // A block not shown takes its pause with it.
    assert_eq!(text[2].pause, None);
    assert_eq!(text[3].pause, pause(1500, "beat"));
    assert!(text[4].verse && text[5].verse);
    assert!(!text[6].verse && !text[3].verse);
}
