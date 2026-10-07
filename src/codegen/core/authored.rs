//! Authored trivia carried into rebuilt forms.

use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct AuthoredText(Vec<AuthoredPart>);

#[derive(Debug, Clone, PartialEq, Eq)]
enum AuthoredPart {
    Generated(String),
    Source(SourceSpan),
}

impl AuthoredText {
    pub(super) fn generated(text: impl Into<String>) -> Self {
        let mut authored = Self::default();
        authored.push_generated(&text.into());
        authored
    }

    pub(super) fn push_generated(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(AuthoredPart::Generated(last)) = self.0.last_mut() {
            last.push_str(text);
        } else {
            self.0.push(AuthoredPart::Generated(text.to_owned()));
        }
    }

    pub(super) fn push_gap(
        &mut self,
        source: &str,
        before: &str,
        gap: Option<SourceSpan>,
        after: &str,
    ) {
        match gap.and_then(|gap| gap_trivia(source, gap)) {
            None => {
                self.push_generated(before);
                self.push_generated(after);
            }
            Some(segments) => {
                self.push_generated(before.trim_end());
                for segment in segments {
                    self.0.push(AuthoredPart::Source(segment));
                }
                self.push_generated(after.trim_start());
            }
        }
    }

    pub(super) fn push_to<'a>(&self, source: &'a str, out: &mut Rope<'a>) {
        for part in &self.0 {
            match part {
                AuthoredPart::Generated(text) => out.push_lit(text.clone()),
                AuthoredPart::Source(span) => {
                    out.push_src(&source[span.start..span.end], span.start)
                }
            }
        }
    }

    pub(super) fn text(&self, source: &str) -> String {
        let mut text = String::new();
        for part in &self.0 {
            match part {
                AuthoredPart::Generated(generated) => text.push_str(generated),
                AuthoredPart::Source(span) => text.push_str(&source[span.start..span.end]),
            }
        }
        text
    }
}

pub(super) fn push_gap<'a>(
    source: &'a str,
    out: &mut Rope<'a>,
    before: &str,
    gap: Option<SourceSpan>,
    after: &str,
) {
    let mut authored = AuthoredText::default();
    authored.push_gap(source, before, gap, after);
    authored.push_to(source, out);
}

fn gap_trivia(source: &str, gap: SourceSpan) -> Option<Vec<SourceSpan>> {
    let bytes = source.as_bytes();
    let end = gap.end.min(bytes.len());
    let mut segments = Vec::new();
    let mut commented = false;
    let mut at = gap.start;
    while at < end {
        let (trivia, _) = crate::scanner::skip_trivia(bytes, at, end);
        if trivia > at {
            let text = &bytes[at..trivia];
            let comment = text.windows(2).any(|pair| pair == b"//" || pair == b"/*");
            commented |= comment;
            if comment || text.iter().any(|&byte| byte == b'\n' || byte == b'\r') {
                segments.push(SourceSpan {
                    start: at,
                    end: trivia,
                });
            }
            at = trivia;
            continue;
        }
        at += source[at..].chars().next().map_or(1, char::len_utf8);
    }
    commented.then_some(segments)
}
