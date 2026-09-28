//! Non-exhaustive-match wording and insertion suggestions.

use super::*;

/// The shared wording for match coverage holes.
pub(crate) fn non_exhaustive_message(
    subject: Option<&str>,
    missing: &[String],
    total: usize,
    exact: bool,
    tuple: bool,
) -> String {
    let shown = if missing.len() > 4 || total > missing.len() || !exact {
        let unit = if tuple {
            "combinations in total"
        } else {
            "in total"
        };
        let head = &missing[..missing.len().min(3)];
        let bound = if exact { "" } else { "at least " };
        format!("{}, … ({bound}{total} {unit})", head.join(", "))
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
/// underlines, the braces an arm-insertion edit writes between, and where
/// the arms already written end.
///
/// Both exhaustiveness pipelines carry these offsets — the default one off
/// the parsed match, the typed one off the probe the emission recorded — so
/// the edits below have one implementation rather than one per pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MatchSite {
    /// Byte offset of the `match` keyword.
    pub keyword_off: usize,
    /// Byte offset of the body's opening `{`.
    pub body_open: usize,
    /// Byte offset of the body's closing `}`.
    pub body_close: usize,
    /// Where the parsed arms end, and whether the last one is followed by
    /// its separator.
    pub tail: crate::ArmsTail,
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
/// already laid out: whole lines above the closing brace when it stands on
/// its own line, spliced in before it when it does not.
///
/// The last written arm is found from the parse, not from the text: its
/// final token and whether a `,` token follows it are what the parser saw,
/// so a comment or a block body after that arm cannot be mistaken for, or
/// hide, the separator (TASK-460). When the separator is missing, the one
/// edit starts right after that arm, writes the comma, and carries the
/// text between the arm and the insertion point over unchanged.
fn insert_arms(source: &str, site: MatchSite, arms: &[String]) -> Option<Edit> {
    let bytes = source.as_bytes();
    let tail = site.tail;
    if site.keyword_off > site.body_open
        || site.body_open >= tail.last_start
        || tail.last_start >= tail.last_end
        || tail.last_end > site.body_close
        || site.body_close >= bytes.len()
        || bytes[site.body_open] != b'{'
        || bytes[site.body_close] != b'}'
    {
        return None;
    }
    let line_start = |at: usize| source[..at].rfind('\n').map_or(0, |nl| nl + 1);
    let leading = |at: usize| -> Option<&str> {
        let prefix = &source[line_start(at)..at];
        prefix
            .bytes()
            .all(|b| b == b' ' || b == b'\t')
            .then_some(prefix)
    };
    let close_line = line_start(site.body_close);
    let (at, text) = if close_line > tail.last_end && leading(site.body_close).is_some() {
        let indent = match leading(tail.last_start) {
            Some(indent) => indent.to_string(),
            None => {
                let keyword_line = line_start(site.keyword_off);
                let outer: String = source[keyword_line..site.keyword_off]
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect();
                let step = if outer.starts_with('\t') { "\t" } else { "  " };
                format!("{outer}{step}")
            }
        };
        let text: String = arms.iter().map(|arm| format!("{indent}{arm}\n")).collect();
        (close_line, text)
    } else {
        let padded = bytes[site.body_close - 1].is_ascii_whitespace();
        let text = format!("{}{} ", if padded { "" } else { " " }, arms.join(" "));
        (site.body_close, text)
    };
    if tail.separated {
        return Some(Edit {
            start: at,
            end: at,
            replacement: text,
        });
    }
    Some(Edit {
        start: tail.last_end,
        end: at,
        replacement: format!(",{}{text}", &source[tail.last_end..at]),
    })
}
