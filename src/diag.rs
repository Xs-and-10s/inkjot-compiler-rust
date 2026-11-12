//! Diagnostics: what `inkjot check` reports.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Notice,
}

/// One finding. `line` and `col` are 1-based; `col` counts characters, not bytes.
/// `line == 0` means the finding is about the file as a whole.
#[derive(Debug, Clone)]
pub struct Diag {
    pub severity: Severity,
    pub line: usize,
    pub col: usize,
    pub msg: String,
}

impl Diag {
    pub fn error(line: usize, col: usize, msg: impl Into<String>) -> Self {
        Diag {
            severity: Severity::Error,
            line,
            col,
            msg: msg.into(),
        }
    }
    pub fn notice(line: usize, col: usize, msg: impl Into<String>) -> Self {
        Diag {
            severity: Severity::Notice,
            line,
            col,
            msg: msg.into(),
        }
    }
}

/// Render a diagnostic the way rustc does: message, location, source line, caret.
pub fn render(d: &Diag, file: &str, src: &str) -> String {
    let label = match d.severity {
        Severity::Error => "error",
        Severity::Notice => "notice",
    };
    let mut out = format!("{label}: {}\n", d.msg);
    if d.line == 0 {
        out.push_str(&format!("  --> {file}\n"));
        return out;
    }
    out.push_str(&format!("  --> {file}:{}:{}\n", d.line, d.col));
    if let Some(text) = src.lines().nth(d.line - 1) {
        let num = d.line.to_string();
        let pad = " ".repeat(num.len());
        // Long prose lines are clipped around the caret so the report stays readable.
        let chars: Vec<char> = text.chars().collect();
        let (start, end) = if chars.len() > 110 {
            let s = d.col.saturating_sub(40).min(chars.len());
            (s, (s + 110).min(chars.len()))
        } else {
            (0, chars.len())
        };
        let shown: String = chars[start..end].iter().collect();
        let lead = if start > 0 { "…" } else { "" };
        let trail = if end < chars.len() { "…" } else { "" };
        out.push_str(&format!("{pad} |\n{num} | {lead}{shown}{trail}\n"));
        let caret_at = d.col.saturating_sub(1).saturating_sub(start) + lead.chars().count();
        out.push_str(&format!("{pad} | {}^\n", " ".repeat(caret_at)));
    }
    out
}
