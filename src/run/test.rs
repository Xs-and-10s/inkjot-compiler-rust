//! `[test]` passages: scripted playthroughs written in Inkjot.
//!
//! ```text
//! :: the_chin_callback [test]
//! start: arrival
//! do: "“Nothing. They’re nice. One of them scratched my chin.”"
//! expect: me.grievance == chin
//! expect text: "Scratched. Your. Chin."
//! ```
//!
//! A test's statements run in order:
//!
//! | Statement | Meaning |
//! |---|---|
//! | `seed: n` | Seed the run's dice and shuffles (first, if at all; default 1) |
//! | `start: passage ~ as who` | Begin there, looking through `who` |
//! | `start: passage ~ as a, b` | Begin with several Linkers, one playing each |
//! | `as: who` | Look through someone else; with several Linkers, serve the one playing `who` |
//! | `given: effect` | Change the state directly: `given: learn two_hands` |
//! | `do: "label"` or `do: hall#2` | Take an available Link |
//! | `answer: "Mara"` / `answer: gust, spark, hush` | Answer the first open input |
//! | `vote: vell ~ how device ~ proof cufflink, burn_pattern` | Cast a ballot for the jury |
//! | `expect: expression` | The expression must hold now |
//! | `expect text: "…"` / `expect no text: "…"` | Text shown since the last `do` does / does not contain this |
//! | `expect link: "…"` / `expect no link: "…"` | A Link with this label is / is not available |
//! | `expect at: passage` | The pov is at this passage |
//! | `expect events: ending_reached, died` | The engine reported these since the last action |
//! | `expect fork: "Suppose …" ~ for "Say …"` | An Inquest here offers this fork (standing in for that Link) |
//! | `take fork: "Suppose …"` | Take a fork, as an Inquest would |
//!
//! A failed `do` ends the test, since the rest of the script no longer
//! describes what is on screen. A failed `expect` is recorded and the test
//! goes on, so one run reports every broken expectation.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::{Run, Story};
use crate::json::Value;
use crate::syntax::{gather, Stmt};
use crate::twee;

/// What happened to one test.
#[derive(Debug)]
pub struct Outcome {
    pub name: String,
    /// Line of the test's `::` header.
    pub line: usize,
    /// Each failure, with the line of the statement that failed.
    pub failures: Vec<(usize, String)>,
    /// Everything the story showed, and every Link taken, in order.
    pub transcript: Vec<String>,
}

impl Outcome {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Run every `[test]` passage in `src` against the compiled story `ir`.
pub fn run_all(src: &str, ir: &Value) -> Result<Vec<Outcome>, String> {
    let story = Arc::new(Story::new(ir)?);
    let passages = twee::parse(src, &mut Vec::new());
    Ok(passages
        .iter()
        .filter(|p| p.has_tag("test"))
        .map(|p| {
            let (stmts, _) = gather(p, &mut Vec::new());
            run_one(&story, &p.name, p.line, &stmts)
        })
        .collect())
}

/// The text of a `"…"` value, without its quotes.
fn unquote(s: &str) -> &str {
    s.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(s)
}

fn run_one(story: &Arc<Story>, name: &str, line: usize, stmts: &[Stmt]) -> Outcome {
    let mut out = Outcome {
        name: name.to_string(),
        line,
        failures: Vec::new(),
        transcript: Vec::new(),
    };
    let mut run = Run::new(Arc::clone(story), 1);
    let mut started = false;
    // Text shown to each Linker since their last `do`.
    let mut shown: BTreeMap<String, Vec<String>> = BTreeMap::new();
    // The engine's events since the last action.
    let mut events: Vec<String> = Vec::new();

    for (i, s) in stmts.iter().enumerate() {
        let head = &s.clauses[0];
        let at = head.line;
        let rest = head.text[s.kw.len()..].trim_start();
        // `expect text: "…"`: the words before the colon, and the value after.
        let (how, value) = match rest.split_once(':') {
            Some((how, value)) => (how.trim(), value.trim()),
            None => ("", rest.trim()),
        };
        let step = match s.kw.as_str() {
            "seed" if i == 0 => {
                run = Run::new(Arc::clone(story), value.parse().unwrap_or(1));
                Ok(())
            }
            "seed" => Err("`seed:` must be the first line of a test".to_string()),
            "start" => {
                let who: Vec<&str> = s.clauses[1..]
                    .iter()
                    .find_map(|c| c.text.strip_prefix("as "))
                    .map(|w| w.split(',').map(str::trim).collect())
                    .unwrap_or_default();
                started = true;
                events.clear();
                let result = if who.len() > 1 {
                    run.start_cohort(Some(value), &who)
                } else {
                    run.start(Some(value), who.first().copied())
                };
                note_events(&mut run, &mut events);
                result
            }
            // `take fork: "Suppose …"`: take it, as an Inquest would.
            "take" if how == "fork" => {
                collect(&mut run, &mut shown);
                let tag = seat_tag(&run);
                let mine = shown.remove(&run.pov).unwrap_or_default();
                out.transcript
                    .extend(mine.into_iter().map(|l| format!("{tag}{l}")));
                events.clear();
                let taken = take_fork(&mut run, unquote(value));
                note_events(&mut run, &mut events);
                match taken {
                    Ok(label) => {
                        out.transcript.push(format!("{tag}> {label} (an Inquest)"));
                        collect(&mut run, &mut shown);
                        Ok(())
                    }
                    Err(e) => {
                        out.failures.push((at, e));
                        break;
                    }
                }
            }
            "as" => {
                collect(&mut run, &mut shown);
                if run.seated().is_empty() {
                    run.pov = value.to_string();
                    Ok(())
                } else {
                    run.sit(value)
                }
            }
            "given" => {
                let effect = crate::ir::effect(value);
                run.apply(&effect, &[]).and_then(|()| run.settle_deaths())
            }
            "do" => {
                if !started {
                    started = true;
                    if let Err(e) = run.start(None, None) {
                        out.failures.push((at, e));
                        break;
                    }
                }
                collect(&mut run, &mut shown);
                let tag = seat_tag(&run);
                let mine = shown.remove(&run.pov).unwrap_or_default();
                out.transcript
                    .extend(mine.into_iter().map(|l| format!("{tag}{l}")));
                events.clear();
                let taken = take(&mut run, value);
                note_events(&mut run, &mut events);
                match taken {
                    Ok(label) => {
                        out.transcript.push(format!("{tag}> {label}"));
                        collect(&mut run, &mut shown);
                        Ok(())
                    }
                    Err(e) => {
                        out.failures.push((at, e));
                        break; // the rest of the script no longer applies
                    }
                }
            }
            "answer" => {
                collect(&mut run, &mut shown);
                let words: Vec<String> =
                    match value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
                        Some(text) => vec![text.to_string()],
                        None => value.split(',').map(|w| w.trim().to_string()).collect(),
                    };
                events.clear();
                let result = run.answer(&words);
                note_events(&mut run, &mut events);
                collect(&mut run, &mut shown);
                result
            }
            "vote" => {
                collect(&mut run, &mut shown);
                let clause = |word: &str| {
                    s.clauses[1..]
                        .iter()
                        .find_map(|c| c.text.strip_prefix(word))
                        .map(|t| t.trim().to_string())
                        .unwrap_or_default()
                };
                let proof: Vec<String> = clause("proof ")
                    .split(',')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect();
                events.clear();
                let result = run.vote(value, &clause("how "), &proof);
                note_events(&mut run, &mut events);
                collect(&mut run, &mut shown);
                result
            }
            "expect" if how == "events" => {
                let missing: Vec<&str> = value
                    .split(',')
                    .map(str::trim)
                    .filter(|k| !events.iter().any(|e| e == k))
                    .collect();
                if missing.is_empty() {
                    Ok(())
                } else {
                    Err(format!(
                        "expected the events {}; the engine reported: {}",
                        missing.join(", "),
                        events.join(", ")
                    ))
                }
            }
            "expect" if how == "fork" => {
                let stands_in_for = s.clauses[1..]
                    .iter()
                    .find_map(|c| c.text.strip_prefix("for "))
                    .map(|l| unquote(l.trim()).to_string());
                expect_fork(&mut run, unquote(value), stands_in_for.as_deref())
            }
            "expect" => {
                collect(&mut run, &mut shown);
                let mine = shown.get(&run.pov).cloned().unwrap_or_default();
                expect(&mut run, how, value, &mine)
            }
            other => Err(format!("`{other}:` is not a test statement")),
        };
        if let Err(e) = step {
            out.failures.push((at, e));
        }
        for failure in run.failures.drain(..) {
            out.failures
                .push((at, format!("the story's own check failed: {failure}")));
        }
    }
    // Whatever each Linker was last shown.
    let served = run.pov.clone();
    let mut everyone = run.seated().to_vec();
    if everyone.is_empty() {
        everyone.push(served.clone());
    }
    for who in everyone {
        if run.sit(&who).is_ok() {
            collect(&mut run, &mut shown);
            let tag = seat_tag(&run);
            let mine = shown.remove(&who).unwrap_or_default();
            out.transcript
                .extend(mine.into_iter().map(|l| format!("{tag}{l}")));
        }
    }
    let _ = run.sit(&served);
    out
}

/// What the engine reports for the action just taken, by type, as the
/// engine works them out (`src/engine/events.rs`): what the interpreter
/// noted, the Nib shown, an ending reached on arrival, the reading over.
fn note_events(run: &mut Run, events: &mut Vec<String>) {
    events.extend(run.take_noted().into_iter().map(|(k, _)| k.to_string()));
    events.push("nib_shown".into());
    let here = run.here().to_string();
    let finale = run.story.passage_tags(&here).iter().any(|t| t == "finale");
    if finale && run.trail().contains(&here) {
        events.push("ending_reached".into());
        if run.story.has_measures() {
            events.push("measured".into());
        }
    }
    let nothing_offered = run.choices().is_ok_and(|c| c.is_empty());
    let nothing_asked = run.asks().is_ok_and(|a| a.is_empty());
    if nothing_offered && nothing_asked && !run.waiting() {
        events.push("reading_finished".into());
    }
}

/// Every fork an Inquest could offer here: `(fork id, its label, the
/// label of the Link it stands in for)`.
fn forks_here(run: &mut Run) -> Result<Vec<(String, String, String)>, String> {
    let mut out = Vec::new();
    for (id, label) in run.offers()? {
        for (fork, fork_label) in run.forks(&id, None)? {
            out.push((fork, fork_label.plain(), label.plain()));
        }
    }
    Ok(out)
}

fn take_fork(run: &mut Run, wanted: &str) -> Result<String, String> {
    let forks = forks_here(run)?;
    match forks.iter().find(|(_, label, _)| label == wanted) {
        Some((id, label, _)) => {
            let label = label.clone();
            run.choose_fork(&id.clone(), None)?;
            Ok(label)
        }
        None => Err(format!(
            "no fork \"{wanted}\" can be taken at `{}`; the forks are: {}",
            run.here(),
            labels_of(&forks)
        )),
    }
}

fn expect_fork(run: &mut Run, wanted: &str, stands_in_for: Option<&str>) -> Result<(), String> {
    let forks = forks_here(run)?;
    let found = forks
        .iter()
        .any(|(_, label, offered)| label == wanted && stands_in_for.is_none_or(|s| s == offered));
    if found {
        Ok(())
    } else {
        Err(format!(
            "expected a fork \"{wanted}\"{}; the forks are: {}",
            stands_in_for.map_or(String::new(), |s| format!(" for \"{s}\"")),
            labels_of(&forks)
        ))
    }
}

fn labels_of(forks: &[(String, String, String)]) -> String {
    if forks.is_empty() {
        return "none".into();
    }
    forks
        .iter()
        .map(|(_, label, offered)| format!("\"{label}\" for \"{offered}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Move what the served Linker has been shown into their pile.
fn collect(run: &mut Run, shown: &mut BTreeMap<String, Vec<String>>) {
    let lines = run.take_output();
    shown.entry(run.pov.clone()).or_default().extend(lines);
}

/// With several Linkers, transcript lines say whose they are: `[rook] `.
fn seat_tag(run: &Run) -> String {
    if run.seated().is_empty() {
        String::new()
    } else {
        format!("[{}] ", run.pov)
    }
}

/// Take the Link named by `do:`, by quoted label or by id.
fn take(run: &mut Run, value: &str) -> Result<String, String> {
    let choices = run.choices()?;
    let wanted = unquote(value);
    let found = choices.iter().find(|(id, label)| {
        if value.starts_with('"') {
            label == wanted
        } else {
            id == wanted
        }
    });
    match found {
        Some((id, label)) => {
            let label = label.clone();
            run.choose(&id.clone())?;
            Ok(label)
        }
        None => {
            let offered: Vec<String> = choices
                .iter()
                .map(|(id, label)| format!("{id} \"{label}\""))
                .collect();
            let offered = if offered.is_empty() {
                "none".to_string()
            } else {
                offered.join(", ")
            };
            Err(format!(
                "no available Link matches {value}; at `{}` the Links are: {offered}",
                run.here()
            ))
        }
    }
}

fn expect(run: &mut Run, how: &str, value: &str, shown: &[String]) -> Result<(), String> {
    let text = shown.join("\n");
    match how {
        "" => {
            let e = crate::ir::expression(value);
            if run.eval(&e)?.truthy() {
                Ok(())
            } else {
                Err(format!("expected `{value}` to hold, but it does not"))
            }
        }
        "text" | "no text" => {
            let wanted = unquote(value);
            match (how == "text", text.contains(wanted)) {
                (true, false) => Err(format!(
                    "expected the text to contain \"{wanted}\"; it was:\n{text}"
                )),
                (false, true) => Err(format!(
                    "expected the text not to contain \"{wanted}\", but it does"
                )),
                _ => Ok(()),
            }
        }
        "link" | "no link" => {
            let wanted = unquote(value);
            let labels: Vec<String> = run.choices()?.into_iter().map(|(_, label)| label).collect();
            match (how == "link", labels.iter().any(|l| l == wanted)) {
                (true, false) => Err(format!(
                    "expected a Link \"{wanted}\"; the Links are: {}",
                    labels.join(" | ")
                )),
                (false, true) => Err(format!("expected no Link \"{wanted}\", but there is one")),
                _ => Ok(()),
            }
        }
        "at" => {
            if run.here() == value {
                Ok(())
            } else {
                Err(format!(
                    "expected to be at `{value}`, but at `{}`",
                    run.here()
                ))
            }
        }
        other => Err(format!("`expect {other}:` is not a kind of expectation")),
    }
}
