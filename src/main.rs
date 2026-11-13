use std::process::ExitCode;

const USAGE: &str = "\
inkjot: the Inkjot compiler

usage:
  inkjot check <file.twee>...   read each file and report what is wrong with it
  inkjot build <file.twee>      compile one file to its JSON IR, on standard output
  inkjot build <file.twee> -o <out.json>
  inkjot test <file.twee>...    run the [test] passages; the files are read as one story
  inkjot play <file.twee>...    play the story in the terminal
  inkjot --version
  inkjot --help

exit status: 0 when every file passes, 1 when any has errors, 2 for bad usage";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") if args.len() > 1 => check(&args[1..]),
        Some("build") => match &args[1..] {
            [file] => build(file, None),
            [file, flag, out] if flag == "-o" => build(file, Some(out)),
            _ => {
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        },
        Some("test") if args.len() > 1 => test(&args[1..]),
        Some("play") if args.len() > 1 => play(&args[1..]),
        Some("--version") | Some("-V") => {
            println!(
                "inkjot {} (Inkjot {}.x)",
                env!("CARGO_PKG_VERSION"),
                inkjot::VERSION
            );
            ExitCode::SUCCESS
        }
        Some("--help") | Some("-h") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn check(files: &[String]) -> ExitCode {
    let mut failed = false;
    for file in files {
        let src = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: cannot read {file}: {e}");
                failed = true;
                continue;
            }
        };
        let report = inkjot::check(&src);
        for d in &report.diags {
            eprintln!("{}", inkjot::render(d, file, &src));
        }
        let s = &report.stats;
        let counts = format!(
            "{} passages, {} declarations, {} Links, {} speeches, {} expressions",
            s.passages, s.declarations, s.links, s.speeches, s.expressions
        );
        if report.ok() {
            println!("{file}: ok ({counts})");
        } else {
            let n = report.errors();
            println!(
                "{file}: {n} error{} ({counts})",
                if n == 1 { "" } else { "s" }
            );
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn build(file: &str, out: Option<&String>) -> ExitCode {
    let src = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {file}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let ir = match inkjot::build(&src) {
        Ok(ir) => ir,
        Err(report) => {
            for d in &report.diags {
                eprintln!("{}", inkjot::render(d, file, &src));
            }
            let n = report.errors();
            eprintln!(
                "{file}: not built; fix the {n} error{} above",
                if n == 1 { "" } else { "s" }
            );
            return ExitCode::FAILURE;
        }
    };
    let text = format!("{ir}\n");
    match out {
        None => print!("{text}"),
        Some(path) => {
            if let Err(e) = std::fs::write(path, &text) {
                eprintln!("error: cannot write {path}: {e}");
                return ExitCode::FAILURE;
            }
            println!("{file}: built {path} ({} bytes)", text.len());
        }
    }
    ExitCode::SUCCESS
}

/// Several files read as one story, as Twine does with a story split across
/// files. Remembers where each file starts, so a diagnostic can name the file
/// and line it really came from.
struct Sources {
    files: Vec<(String, String)>,
    text: String,
}

impl Sources {
    fn read(paths: &[String]) -> Result<Sources, String> {
        let mut files = Vec::new();
        let mut text = String::new();
        for path in paths {
            let src =
                std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
            text.push_str(&src);
            if !src.ends_with('\n') {
                text.push('\n');
            }
            files.push((path.clone(), src));
        }
        Ok(Sources { files, text })
    }

    /// Print a diagnostic against the file it belongs to.
    fn print(&self, d: &inkjot::Diag) {
        let mut first = 1;
        for (path, src) in &self.files {
            let lines = src.lines().count().max(1);
            if d.line == 0 || d.line < first + lines {
                let mut local = d.clone();
                if d.line > 0 {
                    local.line = d.line - first + 1;
                }
                eprintln!("{}", inkjot::render(&local, path, src));
                return;
            }
            first += lines;
        }
    }

    /// Check and build, printing any errors. `None` if the story has errors.
    fn compile(&self) -> Option<inkjot::json::Value> {
        match inkjot::build(&self.text) {
            Ok(ir) => Some(ir),
            Err(report) => {
                for d in &report.diags {
                    self.print(d);
                }
                eprintln!("not run: fix the {} error(s) above", report.errors());
                None
            }
        }
    }
}

fn test(paths: &[String]) -> ExitCode {
    let sources = match Sources::read(paths) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let Some(ir) = sources.compile() else {
        return ExitCode::FAILURE;
    };
    let outcomes = match inkjot::run::test::run_all(&sources.text, &ir) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    if outcomes.is_empty() {
        eprintln!("no [test] passages found");
        return ExitCode::FAILURE;
    }
    let failed = outcomes.iter().filter(|o| !o.passed()).count();
    for o in &outcomes {
        if o.passed() {
            println!("test {} ... ok", o.name);
            continue;
        }
        println!("test {} ... FAILED", o.name);
        for (line, why) in &o.failures {
            let d = inkjot::Diag::error(*line, 1, why.clone());
            sources.print(&d);
        }
        println!("  transcript:");
        for t in &o.transcript {
            println!("    {t}");
        }
    }
    println!("\n{} passed, {failed} failed", outcomes.len() - failed);
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn play(paths: &[String]) -> ExitCode {
    use std::io::{BufRead, Write};
    let sources = match Sources::read(paths) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let Some(ir) = sources.compile() else {
        return ExitCode::FAILURE;
    };
    let story = match inkjot::run::Story::new(&ir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut run = inkjot::run::Run::new(&story, 1);
    if let Err(e) = run.start(None, None) {
        eprintln!("error: {e}");
        return ExitCode::FAILURE;
    }
    let stdin = std::io::stdin();
    loop {
        for line in run.take_output() {
            println!("{line}\n");
        }
        let choices = match run.choices() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::FAILURE;
            }
        };
        if choices.is_empty() {
            println!("(the end)");
            return ExitCode::SUCCESS;
        }
        for (i, (_, label)) in choices.iter().enumerate() {
            println!("  {}. {label}", i + 1);
        }
        print!("> ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 || line.trim() == "q" {
            return ExitCode::SUCCESS;
        }
        let Some((id, _)) = line
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|n| choices.get(n.wrapping_sub(1)))
        else {
            println!("(type a number from the list, or q to stop)");
            continue;
        };
        println!();
        if let Err(e) = run.choose(&id.clone()) {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    }
}
