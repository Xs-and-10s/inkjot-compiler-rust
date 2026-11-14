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
//! | `expect: expression` | The expression must hold now |
//! | `expect text: "…"` / `expect no text: "…"` | Text shown since the last `do` does / does not contain this |
//! | `expect link: "…"` / `expect no link: "…"` | A Link with this label is / is not available |
//! | `expect at: passage` | The pov is at this passage |
//!
//! A failed `do` ends the test, since the rest of the script no longer
//! describes what is on screen. A failed `expect` is recorded and the test
//! goes on, so one run reports every broken expectation.

use std::collections::HashMap;

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
    let story = Story::new(ir)?;
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

fn run_one(story: &Story, name: &str, line: usize, stmts: &[Stmt]) -> Outcome {
    let mut out = Outcome {
        name: name.to_string(),
        line,
        failures: Vec::new(),
        transcript: Vec::new(),
    };
    let mut run = Run::new(story, 1);
    let mut started = false;
    // Text shown to each Linker since their last `do`.
    let mut shown: HashMap<String, Vec<String>> = HashMap::new();

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
                run = Run::new(story, value.parse().unwrap_or(1));
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
                if who.len() > 1 {
                    run.start_cohort(Some(value), &who)
                } else {
                    run.start(Some(value), who.first().copied())
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
                match take(&mut run, value) {
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

/// Move what the served Linker has been shown into their pile.
fn collect(run: &mut Run, shown: &mut HashMap<String, Vec<String>>) {
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
