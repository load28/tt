//! Non-exhaustive-match wording and insertion suggestions.

use super::*;

/// The shared wording for match coverage holes.
pub(crate) fn non_exhaustive_message(
    subject: Option<&str>,
    missing: &[String],
    tuple: bool,
) -> String {
    let shown = if missing.len() > 4 {
        let unit = if tuple {
            "combinations in total"
        } else {
            "in total"
        };
        format!("{}, … ({} {unit})", missing[..3].join(", "), missing.len())
    } else {
        missing.join(", ")
    };
    let on = match subject {
        Some(subject) => format!(" on {subject}"),
        None => String::new(),
    };
    format!("match{on} is not exhaustive: missing {shown}")
}

/// The one wording of how to close a match's holes, by writing the arms.
///
/// It rides with the diagnostic as a [`Suggestion`] rather than inside
/// [`non_exhaustive_message`]: the missing tags are the *problem*, and
/// what to write instead is the *fix*. Both pipelines attach this same
/// constant, so the advice cannot drift apart either.
pub(crate) const NON_EXHAUSTIVE_HELP: &str = "add the missing arms";

/// The other way to close them: one arm that covers whatever is left.
pub(crate) const NON_EXHAUSTIVE_WILDCARD_HELP: &str = "or add a final `_` arm";

/// The body a compiler-authored arm gets. It is a placeholder on purpose —
/// what the case should evaluate to is the one thing the compiler cannot
/// know — and `undefined` is the value TypeScript will complain about if
/// the reader forgets to replace it, which is the right kind of reminder.
const ARM_BODY: &str = "=> undefined,";

/// Where a match is written: what a diagnostic about the match as a whole
/// underlines, and the braces an arm-insertion edit writes between.
///
/// Both exhaustiveness pipelines carry these four offsets — the default
/// one off the parsed match, the typed one off the probe the emission
/// recorded — so the edits below have one implementation rather than one
/// per pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MatchSite {
    /// Byte offset of the `match` keyword.
    pub keyword_off: usize,
    /// Byte offset of the body's opening `{`.
    pub body_open: usize,
    /// Byte offset of the body's closing `}`.
    pub body_close: usize,
}

/// How to close a non-exhaustive match, as the two edits that do it: write
/// the missing arms, or write a final `_`.
///
/// The compiler authors the text because it is the only party that knows
/// all three of what is missing, what each case's payload is called, and
/// where the body's braces are. A consumer that reads the arms back out of
/// the rendered message would be recognizing a sentence by its shape —
/// which is what this replaces (TASK-216).
pub(crate) fn non_exhaustive_suggestions(
    source: &str,
    site: MatchSite,
    arms: &[String],
) -> Vec<Suggestion> {
    let mut out = Vec::new();
    if !arms.is_empty()
        && let Some(edit) = insert_arms(
            source,
            site,
            &arms
                .iter()
                .map(|pattern| format!("{pattern} {ARM_BODY}"))
                .collect::<Vec<_>>(),
        )
    {
        out.push(Suggestion {
            message: NON_EXHAUSTIVE_HELP.to_string(),
            edit: Some(edit),
        });
    }
    if let Some(edit) = insert_arms(source, site, &[format!("_ {ARM_BODY}")]) {
        out.push(Suggestion {
            message: NON_EXHAUSTIVE_WILDCARD_HELP.to_string(),
            edit: Some(edit),
        });
    }
    // A site whose braces do not line up with the text (a stale buffer, a
    // recovered parse) yields no edit rather than a wrong one — the advice
    // is still worth saying.
    if out.is_empty() {
        out.push(Suggestion {
            message: format!("{NON_EXHAUSTIVE_HELP} {NON_EXHAUSTIVE_WILDCARD_HELP}"),
            edit: None,
        });
    }
    out
}

/// The edit that writes `arms` into a match body, matching how the body is
/// already laid out: above the closing brace when it stands on its own
/// line, spliced in before it when the whole match is on one line.
fn insert_arms(source: &str, site: MatchSite, arms: &[String]) -> Option<Edit> {
    let bytes = source.as_bytes();
    if site.keyword_off > site.body_open
        || site.body_open >= site.body_close
        || site.body_close >= bytes.len()
        || bytes[site.body_open] != b'{'
        || bytes[site.body_close] != b'}'
    {
        return None;
    }
    let line_start = |at: usize| source[..at].rfind('\n').map_or(0, |nl| nl + 1);
    let close_line = line_start(site.body_close);
    if source[close_line..site.body_close].trim().is_empty() {
        // `}` on its own line: whole arm lines above it, indented one step
        // in from the `match` keyword's own line.
        let keyword_line = line_start(site.keyword_off);
        let indent: String = source[keyword_line..site.keyword_off]
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let text: String = arms
            .iter()
            .map(|arm| format!("{indent}  {arm}\n"))
            .collect();
        return Some(Edit {
            start: close_line,
            end: close_line,
            replacement: text,
        });
    }
    // One-line match: splice the arms in after the last written arm,
    // adding the comma that arm may be missing. The range starts where the
    // body's text ends rather than at the `}`, so the padding before the
    // brace is rewritten instead of being left in the middle.
    let body = &source[site.body_open + 1..site.body_close];
    let written = body.trim_end();
    let separator = if written.is_empty() || written.ends_with(',') {
        " "
    } else {
        ", "
    };
    Some(Edit {
        start: site.body_open + 1 + written.len(),
        end: site.body_close,
        replacement: format!("{separator}{} ", arms.join(" ")),
    })
}
