//! What the Linker reads: paragraphs of styled runs.
//!
//! The IR lowers emphasis to styled parts (`{"style": ["italic"], "parts":
//! […]}`); the interpreter keeps the styles as it renders, so a runtime
//! shows *italic* and **bold** without parsing anything. A struck list
//! entry is a run styled `struck`.
//!
//! `plain()` flattens a paragraph for the terminal and for `expect text:`:
//! styles drop away, except that struck text is written `~~like this~~`.

/// A stretch of text in one style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    /// `italic`, `bold`, `struck`, in that order; empty for plain text.
    pub style: Vec<&'static str>,
}

/// One paragraph.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Para {
    pub runs: Vec<Span>,
}

impl Para {
    /// A paragraph of plain text.
    pub fn plain_text(text: impl Into<String>) -> Para {
        let mut p = Para::default();
        p.push(&text.into(), &[]);
        p
    }

    /// Add text in a style, joining it to the last run if the style is the
    /// same.
    pub fn push(&mut self, text: &str, style: &[&'static str]) {
        if text.is_empty() {
            return;
        }
        let mut style: Vec<&'static str> = style.to_vec();
        style.sort_by_key(|s| ["italic", "bold", "struck"].iter().position(|k| k == s));
        style.dedup();
        match self.runs.last_mut() {
            Some(last) if last.style == style => last.text.push_str(text),
            _ => self.runs.push(Span {
                text: text.to_string(),
                style,
            }),
        }
    }

    /// Add another paragraph's runs after this one's.
    pub fn extend(&mut self, other: Para) {
        for run in other.runs {
            self.push(&run.text, &run.style);
        }
    }

    pub fn is_blank(&self) -> bool {
        self.runs.iter().all(|r| r.text.trim().is_empty())
    }

    /// The paragraph as plain text: styles dropped, struck text `~~struck~~`.
    pub fn plain(&self) -> String {
        self.runs
            .iter()
            .map(|r| {
                if r.style.contains(&"struck") {
                    format!("~~{}~~", r.text)
                } else {
                    r.text.clone()
                }
            })
            .collect()
    }
}

impl From<&str> for Para {
    fn from(text: &str) -> Para {
        Para::plain_text(text)
    }
}

impl From<String> for Para {
    fn from(text: String) -> Para {
        Para::plain_text(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_join_when_their_style_is_the_same() {
        let mut p = Para::plain_text("a ");
        p.push("b", &["bold", "italic"]);
        p.push("c", &["italic", "bold"]);
        p.push(" d", &[]);
        assert_eq!(p.runs.len(), 3);
        assert_eq!(p.runs[1].text, "bc");
        assert_eq!(p.runs[1].style, ["italic", "bold"]);
        assert_eq!(p.plain(), "a bc d");
    }

    #[test]
    fn struck_text_reads_struck() {
        let mut p = Para::default();
        p.push("the vet", &["struck"]);
        assert_eq!(p.plain(), "~~the vet~~");
    }
}
