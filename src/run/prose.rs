//! What the Linker reads: paragraphs of styled runs.
//!
//! The IR lowers emphasis to styled parts (`{"style": ["italic"], "parts":
//! […]}`); the interpreter keeps the styles as it renders, so a runtime
//! shows *italic* and **bold** without parsing anything. A struck list
//! entry is a run styled `struck`.
//!
//! `plain()` flattens a paragraph for the terminal and for `expect text:`:
//! styles drop away, except that struck text is written `~~like this~~`.
//!
//! Pacing rides along, so a runtime that reads only the view can pace the
//! text: `[after 2s]` and `[beat 2s]` become a `Pause` on the next
//! paragraph (a wait after the one before it is shown), or on a run when
//! the text is appended to the paragraph before (`[after 2s; append]`:
//! "He winds it once, twice— *(two seconds)* The lights die."). `[verse]`
//! marks its paragraphs as lines of verse.
//!
//! Two things the page shows that are not words:
//!
//! - **Blockquotes.** `[quote letter ~ by "A. K."]` … `[end]`: each
//!   paragraph inside carries its `Quote` (kind and author), and the first
//!   says it begins one, so a runtime draws the letter, the poster or the
//!   edict as one block.
//! - **Redaction.** Text redacted and not yet revealed never reaches the
//!   page: in its place is a run of `█`, one for each character hidden,
//!   styled `redacted`. A runtime draws it as a bar; only its length is
//!   given away.

/// A stretch of text in one style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    /// `italic`, `bold`, `struck`, `redacted`, in that order; empty for
    /// plain text. A `redacted` run's text is `█`, one for each character
    /// hidden.
    pub style: Vec<&'static str>,
    /// A wait before this run is shown, within its paragraph. Only a run
    /// appended after a pause has one; a paragraph's own wait is its
    /// `pause`.
    pub pause: Option<Pause>,
}

/// A wait in the text: `[after 2s]` or `[beat 2s]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pause {
    /// How long, after what came before is shown.
    pub ms: u32,
    /// `after` (the text simply waits) or `beat` (a dramatic pause, which a
    /// runtime that reveals on tap may let the reader skip).
    pub kind: &'static str,
}

impl Pause {
    /// `2s`, `1.5s`, `400ms`: a wait in milliseconds.
    pub fn parse(time: &str, kind: &'static str) -> Option<Pause> {
        let t = time.trim();
        let ms = match t.strip_suffix("ms") {
            Some(n) => n.trim().parse::<f64>().ok()?,
            None => t.strip_suffix('s')?.trim().parse::<f64>().ok()? * 1000.0,
        };
        (ms.is_finite() && (0.0..=3_600_000.0).contains(&ms)).then(|| Pause {
            ms: ms.round() as u32,
            kind,
        })
    }
}

/// The bar redacted text is shown as: one of these for each character.
pub const BAR: char = '█';

/// A blockquote that a paragraph belongs to: `[quote letter ~ by "A. K."]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quote {
    /// `sign`, `letter`, `poster` or `edict`.
    pub kind: String,
    /// Who wrote it, if the quote says: `~ by "A. K."`.
    pub by: Option<String>,
    /// This paragraph begins the quote.
    pub first: bool,
}

/// One paragraph.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Para {
    pub runs: Vec<Span>,
    /// A wait before this paragraph is shown, after the one before it.
    pub pause: Option<Pause>,
    /// A line of verse (`[verse]`): kept as a line, set close to the next.
    pub verse: bool,
    /// The blockquote this paragraph is part of, if any.
    pub quote: Option<Quote>,
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
        style.sort_by_key(|s| {
            ["italic", "bold", "struck", "redacted"]
                .iter()
                .position(|k| k == s)
        });
        style.dedup();
        match self.runs.last_mut() {
            Some(last) if last.style == style => last.text.push_str(text),
            _ => self.runs.push(Span {
                text: text.to_string(),
                style,
                pause: None,
            }),
        }
    }

    /// Add another paragraph's runs after this one's. Its pause, if any,
    /// goes on its first run, so the wait still happens mid-paragraph.
    pub fn extend(&mut self, other: Para) {
        let mut pause = other.pause;
        for run in other.runs {
            match pause.take() {
                Some(p) if !run.text.is_empty() => self.runs.push(Span {
                    pause: Some(p),
                    ..run
                }),
                Some(p) => pause = Some(p),
                None => self.push(&run.text, &run.style),
            }
        }
    }

    /// The paragraph's text from character `from` on, hidden: one run of
    /// `█`, a bar as long as what it hides. What came before (a speaker's
    /// name, say) is kept.
    pub fn redact_from(&mut self, from: usize) {
        let mut kept = Para {
            pause: self.pause,
            verse: self.verse,
            quote: self.quote.clone(),
            ..Para::default()
        };
        let mut seen = 0;
        let mut hidden = 0;
        for run in &self.runs {
            for ch in run.text.chars() {
                if seen < from {
                    kept.push(&ch.to_string(), &run.style);
                } else {
                    hidden += 1;
                }
                seen += 1;
            }
        }
        kept.push_bar(hidden, &[]);
        *self = kept;
    }

    /// A bar of `n` characters, in `style` and `redacted`.
    pub fn push_bar(&mut self, n: usize, style: &[&'static str]) {
        let mut style = style.to_vec();
        style.push("redacted");
        self.push(&BAR.to_string().repeat(n), &style);
    }

    /// How many characters the paragraph holds.
    pub fn len(&self) -> usize {
        self.runs.iter().map(|r| r.text.chars().count()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Does the text so far end where a sentence starts? At the start of a
    /// paragraph, or after `.`, `!`, `?` or `:` and a space, with any
    /// opening quote marks between.
    pub fn at_sentence_start(&self) -> bool {
        let text: String = self.runs.iter().map(|r| r.text.as_str()).collect();
        let t = text.trim_end_matches(['“', '‘', '"', '(', '[']);
        if t.trim().is_empty() {
            return true;
        }
        let spaced = t.len() > t.trim_end().len();
        spaced && t.trim_end().ends_with(['.', '!', '?', ':'])
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
    fn times_read_as_milliseconds() {
        let ms = |t| Pause::parse(t, "after").map(|p| p.ms);
        assert_eq!(ms("2s"), Some(2000));
        assert_eq!(ms("1.5s"), Some(1500));
        assert_eq!(ms("400ms"), Some(400));
        assert_eq!(ms("0s"), Some(0));
        assert_eq!(ms("2"), None);
        assert_eq!(ms("soon"), None);
        assert_eq!(ms("-1s"), None);
    }

    #[test]
    fn an_appended_paragraph_keeps_its_pause_on_its_first_run() {
        let mut p = Para::plain_text("He winds it once, twice—");
        let mut more = Para::plain_text("The lights die.");
        more.pause = Pause::parse("2s", "after");
        p.push(" ", &[]);
        p.extend(more);
        assert_eq!(p.runs.len(), 2);
        assert_eq!(p.runs[0].pause, None);
        assert_eq!(p.runs[1].pause.map(|x| x.ms), Some(2000));
        assert_eq!(p.plain(), "He winds it once, twice— The lights die.");
    }

    #[test]
    fn struck_text_reads_struck() {
        let mut p = Para::default();
        p.push("the vet", &["struck"]);
        assert_eq!(p.plain(), "~~the vet~~");
    }
}
