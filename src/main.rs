use std::process::ExitCode;

const USAGE: &str = "\
inkjot: the Inkjot compiler

usage:
  inkjot check <file.twee>...   read each file and report what is wrong with it
  inkjot build <file.twee>      compile one file to its JSON IR, on standard output
  inkjot build <file.twee> -o <out.json>
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
