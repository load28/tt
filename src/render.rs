//! The terminal form of a diagnostic — the compiler's one text renderer.
//!
//! A [`crate::Diagnostic`] carries a rule's code, a span, and how to fix
//! the problem. Which of those a reader actually sees is a question of
//! *medium*, not of data: an editor draws a squiggle and offers a code
//! action from the same facts this module draws as carets and `help:`
//! lines. Keeping the renderer here rather than in `main.rs` is what makes
//! that one set of facts — the CLI, `--watch` and any other consumer of the
//! library share it, so two surfaces cannot disagree about what a
//! diagnostic said.
//!
//! The renderer never adds a fact. It shows [`Report`] and nothing else; a
//! sentence the reader needs is a field on the diagnostic, not a special
//! case here.
//!
//! ```
//! use ttc::render::{Position, Report, Span, Styles, render};
//!
//! let out = render(
//!     &Report {
//!         severity: ttc::Severity::Error,
//!         code: Some("match-not-exhaustive"),
//!         message: "match on variant Shape is not exhaustive: missing \"Rect\"",
//!         path: "shapes.tt",
//!         span: Some(Span {
//!             start: Position { line: 1, col: 1 },
//!             end: Some(Position { line: 1, col: 14 }),
//!         }),
//!         labels: &[],
//!         suggestions: &[],
//!     },
//!     Some("match (shape) {\n}\n"),
//!     Styles::PLAIN,
//! );
//! assert!(out.starts_with("error[match-not-exhaustive]: match on variant Shape"));
//! assert!(out.contains("1 | match (shape) {"));
//! assert!(out.contains("  | ^^^^^^^^^^^^^"));
//! ```

use std::fmt::Write as _;

use crate::diagnostics::{Severity, Suggestion};

/// How many columns a tab is drawn as. The snippet expands tabs so a caret
/// lands under the character it is about whatever the reader's tab width
/// is — the alternative is a squiggle that only lines up in one terminal.
const TAB_WIDTH: usize = 4;

/// How many source lines a multi-line span draws before it elides the
/// middle. A span that covers a whole file is a position, not a picture.
const MAX_SPAN_LINES: usize = 8;

/// The ANSI sequences a rendered diagnostic is painted with.
///
/// Colour is a property of the *medium*, not of the diagnostic: the same
/// report is a picture in a terminal, plain text in a build log, and
/// structured data in an editor. So the renderer is told what to paint
/// with instead of deciding — and [`Styles::PLAIN`], whose every field is
/// empty, produces output byte-for-byte identical to no styling at all.
/// That is what lets `tests/fixtures/` stand as this module's regression
/// net whatever the terminal the tests run in.
///
/// ```
/// use ttc::render::Styles;
///
/// // A pipe, a file, a CI log: no escape sequences anywhere.
/// assert_eq!(Styles::for_stderr().is_plain(), !std::io::IsTerminal::is_terminal(&std::io::stderr()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Styles {
    /// `error[rule]` and the carets under the span it is about.
    pub error: &'static str,
    /// `warning[rule]`, and its carets.
    pub warning: &'static str,
    /// The sentence on the header line.
    pub message: &'static str,
    /// The frame: `-->`, the `|` bars, the line numbers, the span
    /// bracket a multi-line report draws.
    pub gutter: &'static str,
    /// The `help:` label a suggestion opens with.
    pub help: &'static str,
    /// What returns the terminal to its own colours. Empty exactly when
    /// every other field is.
    pub reset: &'static str,
}

impl Styles {
    /// No styling: the renderer writes the same bytes it always has.
    pub const PLAIN: Styles = Styles {
        error: "",
        warning: "",
        message: "",
        gutter: "",
        help: "",
        reset: "",
    };

    /// The colours a terminal gets — the layers rustc separates, so
    /// severity, frame and advice are told apart before the words are
    /// read.
    pub const ANSI: Styles = Styles {
        error: "\x1b[1;31m",
        warning: "\x1b[1;33m",
        message: "\x1b[1m",
        gutter: "\x1b[1;34m",
        help: "\x1b[1;36m",
        reset: "\x1b[0m",
    };

    /// What to paint diagnostics with when they go to stderr: colour when
    /// stderr is a terminal and `NO_COLOR` is not set, plain otherwise.
    ///
    /// Both conditions are about the *destination*, which is why nothing
    /// here is a flag: a pipe, a redirect and a CI log are all "not a
    /// terminal", and a reader who wants no colour anywhere says so once
    /// in their environment (<https://no-color.org>).
    pub fn for_stderr() -> Styles {
        Styles::for_terminal(std::io::IsTerminal::is_terminal(&std::io::stderr()))
    }

    /// The same decision with the terminal question already answered —
    /// what `for_stderr` is, minus the global it reads.
    pub fn for_terminal(is_terminal: bool) -> Styles {
        let suppressed = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
        if is_terminal && !suppressed {
            Styles::ANSI
        } else {
            Styles::PLAIN
        }
    }

    /// Whether this style set writes no escape sequences.
    pub fn is_plain(&self) -> bool {
        *self == Styles::PLAIN
    }

    /// The colour a report of `severity` is drawn in.
    fn severity(&self, severity: Severity) -> &'static str {
        match severity {
            Severity::Error => self.error,
            Severity::Warning => self.warning,
        }
    }

    /// `text` wrapped in `style`, or `text` itself when the style is
    /// empty — the one place an empty style becomes "no bytes at all".
    fn paint(&self, style: &str, text: &str) -> String {
        if style.is_empty() {
            text.to_string()
        } else {
            format!("{style}{text}{}", self.reset)
        }
    }
}

/// A 1-based line and column, columns counted in characters — the
/// coordinates [`crate::line_col`] produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// 1-based line.
    pub line: usize,
    /// 1-based column, in characters.
    pub col: usize,
}

/// The range a diagnostic underlines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Where the underline starts.
    pub start: Position,
    /// Just past the last character the underline covers. `None` narrows
    /// the underline to a single column — the reporter knew a position but
    /// not an extent.
    pub end: Option<Position>,
}

/// A secondary place a diagnostic points at, with its own words — rustc's
/// labeled spans ("expected because of this", "first declared here").
///
/// A label in the diagnostic's own file on one line joins the primary
/// snippet, underlined with `-` where the primary uses `^`. A label in
/// another file, or one whose span the snippet cannot draw, degrades to a
/// `= note:` line naming the place — the same honest fallback rustc uses.
#[derive(Debug, Clone, Copy)]
pub struct Label<'a> {
    /// Where the label points.
    pub span: Span,
    /// What this place explains.
    pub message: &'a str,
    /// The file the span is in, when it is not the diagnostic's own —
    /// `None` means [`Report::path`].
    pub path: Option<&'a str>,
}

/// One diagnostic in the form [`render`] draws.
///
/// This is deliberately not [`crate::Diagnostic`]: the typed pipeline
/// reports in line/column and the untyped one in byte offsets, and a
/// renderer that took either would have to know which. `Report` is the
/// shape they meet in.
#[derive(Debug, Clone, Copy)]
pub struct Report<'a> {
    /// Whether this stops the build.
    pub severity: Severity,
    /// The rule's stable identity, drawn as `error[code]`. `None` for a
    /// diagnostic no rule claims (a TypeScript answer already carrying its
    /// own `ts(NNNN)` wording, an I/O failure).
    pub code: Option<&'a str>,
    /// What is wrong. Never how to fix it — that is `suggestions`.
    pub message: &'a str,
    /// The file, as the reader would name it on a command line.
    pub path: &'a str,
    /// Where in the file, when the reporter knows.
    pub span: Option<Span>,
    /// Secondary places the diagnostic points at, each with its own words.
    pub labels: &'a [Label<'a>],
    /// How to fix it, drawn as one `= help:` line each.
    pub suggestions: &'a [Suggestion],
}

impl Severity {
    /// The word a rendered diagnostic opens with.
    fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

/// Draws `report` against `source`, the text of the file it is about.
///
/// `source` is optional because a consumer does not always hold it — a
/// diagnostic about a file the CLI never read still has a header and a
/// location worth printing. Without it the snippet is omitted and nothing
/// else changes, so a missing file degrades the picture rather than the
/// report.
///
/// The result carries no trailing newline.
pub fn render(report: &Report<'_>, source: Option<&str>, styles: Styles) -> String {
    let mut out = String::new();
    let severity = styles.severity(report.severity);
    let header = match report.code {
        Some(code) => format!("{}[{code}]", report.severity.label()),
        None => report.severity.label().to_string(),
    };
    let _ = write!(
        out,
        "{}: {}",
        styles.paint(severity, &header),
        styles.paint(styles.message, report.message)
    );

    // A snippet needs both a place to point at and the text to quote.
    let Some((span, source)) = report.span.zip(source) else {
        // No span, or no text to quote it from: name the file (and the
        // position, when there is one) and stop.
        let arrow = styles.paint(styles.gutter, "-->");
        match report.span {
            Some(span) => {
                let _ = write!(
                    out,
                    "\n {arrow} {}:{}:{}",
                    report.path, span.start.line, span.start.col
                );
            }
            None => {
                let _ = write!(out, "\n {arrow} {}", report.path);
            }
        }
        let all: Vec<&Label<'_>> = report.labels.iter().collect();
        write_notes(&mut out, report, &all, 0, styles);
        write_suggestions(&mut out, report.suggestions, 0, false, styles);
        return out;
    };

    let lines: Vec<&str> = source.split('\n').collect();
    let start = span.start;
    let end = span.end.unwrap_or(Position {
        line: start.line,
        col: start.col + 1,
    });
    // A reporter that knows only a position, or one whose end lands before
    // its start, still gets a one-column caret rather than a panic.
    let end = if end.line < start.line || (end.line == start.line && end.col <= start.col) {
        Position {
            line: start.line,
            col: start.col + 1,
        }
    } else {
        end
    };
    // Clamp to the text actually held: a stale buffer must not silently
    // index past the end. Clamping can pull the end *before* the start
    // (a span whose whole range is past the buffer), so the start is the
    // floor — the picture then degrades to one caret rather than an
    // underflow.
    let end_line = end.line.min(lines.len().max(1)).max(start.line);

    // Which labels can join the primary snippet, and which degrade to
    // notes: same file, one line, inside the text held — and only under a
    // single-line primary, whose picture has room for more rows.
    let mut inline: Vec<&Label<'_>> = Vec::new();
    let mut notes: Vec<&Label<'_>> = Vec::new();
    for label in report.labels {
        let line = label.span.start.line;
        let one_line = label.span.end.is_none_or(|e| e.line == line);
        if end_line == start.line
            && label.path.is_none()
            && one_line
            && line >= 1
            && line <= lines.len()
        {
            inline.push(label);
        } else {
            notes.push(label);
        }
    }

    let width = inline
        .iter()
        .map(|label| label.span.start.line)
        .chain([end_line.max(start.line)])
        .max()
        .unwrap_or(1)
        .to_string()
        .len();
    let _ = write!(
        out,
        "\n{:width$}{} {}:{}:{}\n{}",
        "",
        styles.paint(styles.gutter, "-->"),
        report.path,
        start.line,
        start.col,
        bar(width, styles),
        width = width
    );

    if !inline.is_empty() {
        write_annotated(
            &mut out, &lines, width, start, end, &inline, severity, styles,
        );
    } else if end_line == start.line {
        write_single_line(&mut out, &lines, width, start, end, severity, styles);
    } else {
        write_multi_line(
            &mut out, &lines, width, start, end_line, end.col, severity, styles,
        );
    }

    write_notes(&mut out, report, &notes, width, styles);
    write_suggestions(&mut out, report.suggestions, width, true, styles);
    out
}

/// The empty gutter line — `  |` — painted as the frame.
fn bar(width: usize, styles: Styles) -> String {
    styles.paint(styles.gutter, &format!("{:width$} |", "", width = width))
}

/// A gutter carrying a line number — `12 |` — painted as the frame.
fn numbered_bar(line: usize, width: usize, styles: Styles) -> String {
    styles.paint(styles.gutter, &format!("{:>width$} |", line, width = width))
}

/// Draws a [`crate::Diagnostic`] found in `source`, named as `path`.
///
/// The byte offsets the untyped pipeline reports in are converted here, so
/// a caller holding a compile report does not have to know that the
/// renderer works in line/column.
pub fn diagnostic(
    diagnostic: &crate::Diagnostic,
    source: &str,
    path: &str,
    styles: Styles,
) -> String {
    let at = |offset: usize| {
        let (line, col) = crate::line_col(source, offset);
        Position { line, col }
    };
    let span = diagnostic.start.map(|start| Span {
        start: at(start),
        end: diagnostic.end.map(at),
    });
    render(
        &Report {
            severity: diagnostic.severity,
            code: Some(diagnostic.code.as_str()),
            message: &diagnostic.message,
            path,
            span,
            labels: &[],
            suggestions: &diagnostic.suggestions,
        },
        Some(source),
        styles,
    )
}

/// Draws a [`crate::CompileError`] — the single-error form the library's
/// own `Display` renders as one line.
///
/// A `CompileError` carries no rule code, so the header opens with the
/// severity alone. `source` is the text it was found in, when the caller
/// holds it.
pub fn compile_error(
    error: &crate::CompileError,
    source: Option<&str>,
    path: &str,
    styles: Styles,
) -> String {
    let span = (error.line > 0).then(|| Span {
        start: Position {
            line: error.line,
            col: error.col,
        },
        end: (error.end_line > 0).then_some(Position {
            line: error.end_line,
            col: error.end_col,
        }),
    });
    render(
        &Report {
            severity: Severity::Error,
            code: None,
            message: &error.message,
            path,
            span,
            labels: &[],
            suggestions: &[],
        },
        source,
        styles,
    )
}

/// Draws a checked project's [`crate::engine::Diagnostic`], named as
/// `path`.
///
/// `source` is what the check ran against — see
/// [`crate::engine::Snapshot::source_of`], not the file on disk, which an
/// `--overlay` or a later edit may already disagree with. `None` when the
/// caller does not hold it; the header and location still render.
pub fn engine_diagnostic(
    diagnostic: &crate::engine::Diagnostic,
    source: Option<&str>,
    path: &str,
    styles: Styles,
) -> String {
    let at = |(line, col): (usize, usize)| Position { line, col };
    let span = diagnostic.position.map(|start| Span {
        start: at(start),
        end: diagnostic.end.map(at),
    });
    let label_paths: Vec<Option<std::borrow::Cow<'_, str>>> = diagnostic
        .labels
        .iter()
        .map(|label| label.path.as_deref().map(|p| p.to_string_lossy()))
        .collect();
    let labels: Vec<Label<'_>> = diagnostic
        .labels
        .iter()
        .zip(&label_paths)
        .map(|(label, path)| Label {
            span: Span {
                start: at(label.position),
                end: Some(at(label.end)),
            },
            message: &label.message,
            path: path.as_deref(),
        })
        .collect();
    render(
        &Report {
            // Every diagnostic a checked project reports stops the build;
            // the severity space exists for tt rules that do not yet.
            severity: Severity::Error,
            code: diagnostic.code.as_deref(),
            message: &diagnostic.message,
            path,
            span,
            labels: &labels,
            suggestions: &diagnostic.suggestions,
        },
        source,
        styles,
    )
}

/// The text of a 1-based line, with tabs expanded and any `\r` dropped.
fn shown_line(lines: &[&str], line: usize) -> String {
    let raw = lines.get(line.wrapping_sub(1)).copied().unwrap_or("");
    raw.trim_end_matches('\r')
        .replace('\t', &" ".repeat(TAB_WIDTH))
}

/// The display column a 1-based character column sits at, once tabs are
/// expanded. Columns past the end of the line clamp to just past it, so a
/// span that outruns a stale buffer still points somewhere real.
fn display_col(lines: &[&str], line: usize, col: usize) -> usize {
    let raw = lines
        .get(line.wrapping_sub(1))
        .copied()
        .unwrap_or("")
        .trim_end_matches('\r');
    let mut at = 1;
    for (index, ch) in raw.chars().enumerate() {
        if index + 1 >= col {
            return at;
        }
        at += if ch == '\t' { TAB_WIDTH } else { 1 };
    }
    at
}

/// `12 | match (shape) {` and the caret run beneath it.
fn write_single_line(
    out: &mut String,
    lines: &[&str],
    width: usize,
    start: Position,
    end: Position,
    severity: &str,
    styles: Styles,
) {
    let from = display_col(lines, start.line, start.col);
    let to = display_col(lines, start.line, end.col);
    let carets = to.saturating_sub(from).max(1);
    let _ = write!(
        out,
        "\n{} {}\n{} {}{}",
        numbered_bar(start.line, width, styles),
        shown_line(lines, start.line),
        bar(width, styles),
        " ".repeat(from - 1),
        styles.paint(severity, &"^".repeat(carets)),
    );
}

/// The bracketed form, for a construct that spans lines:
///
/// ```text
/// 12 |   match (shape) {
///    |  _^
/// 13 | |   Circle(r) => area(r),
/// 14 | | }
///    | |_^
/// ```
#[allow(clippy::too_many_arguments)]
fn write_multi_line(
    out: &mut String,
    lines: &[&str],
    width: usize,
    start: Position,
    end_line: usize,
    end_col: usize,
    severity: &str,
    styles: Styles,
) {
    let from = display_col(lines, start.line, start.col);
    let _ = write!(
        out,
        "\n{}   {}\n{}  {}",
        numbered_bar(start.line, width, styles),
        shown_line(lines, start.line),
        bar(width, styles),
        styles.paint(severity, &format!("{}^", "_".repeat(from))),
    );

    // The bracket down the left of the quoted block belongs to the span,
    // not to the frame, so it is painted with the severity — which is
    // what makes "these lines are the construct" readable at a glance.
    let bracket = styles.paint(severity, "|");
    // Long spans show their head and their tail; the middle is elided
    // rather than scrolled past.
    let body: Vec<usize> = if end_line - start.line + 1 > MAX_SPAN_LINES {
        (start.line + 1..start.line + 3)
            .chain(end_line..=end_line)
            .collect()
    } else {
        (start.line + 1..=end_line).collect()
    };
    for (index, line) in body.iter().enumerate() {
        if index > 0 && *line > body[index - 1] + 1 {
            let _ = write!(
                out,
                "\n{} {bracket}",
                styles.paint(
                    styles.gutter,
                    &format!("{:<width$} |", "...", width = width)
                ),
            );
        }
        let _ = write!(
            out,
            "\n{} {bracket} {}",
            numbered_bar(*line, width, styles),
            shown_line(lines, *line),
        );
    }

    let to = display_col(lines, end_line, end_col.saturating_sub(1).max(1));
    let _ = write!(
        out,
        "\n{} {}",
        bar(width, styles),
        styles.paint(severity, &format!("|{}^", "_".repeat(to))),
    );
}

/// The primary span and its same-file labels in one snippet — rustc's
/// annotated block:
///
/// ```text
/// 11 |     |> Option.mapP((x: number) => String(x))
///    |        ------------------------------------- the piped value comes from this step
/// 12 |     |> Option.unwrapOrP(0)
///    |        ^^^^^^^^^^^^^^^^^^^
/// ```
///
/// Annotated lines are drawn in order; one plain line bridges a gap of
/// exactly one, a wider gap elides to `...`. A label sharing the primary's
/// line gets its own underline row beneath the caret row.
#[allow(clippy::too_many_arguments)]
fn write_annotated(
    out: &mut String,
    lines: &[&str],
    width: usize,
    start: Position,
    end: Position,
    labels: &[&Label<'_>],
    severity: &str,
    styles: Styles,
) {
    // One row per underline; rows on one source line draw under one quote
    // of it, the primary's carets first.
    struct Row<'a> {
        line: usize,
        col: usize,
        end_col: usize,
        message: Option<&'a str>,
    }
    let mut rows: Vec<Row<'_>> = Vec::with_capacity(labels.len() + 1);
    rows.push(Row {
        line: start.line,
        col: start.col,
        end_col: end.col,
        message: None,
    });
    for label in labels {
        let col = label.span.start.col;
        rows.push(Row {
            line: label.span.start.line,
            col,
            end_col: label.span.end.map_or(col + 1, |e| e.col),
            message: Some(label.message),
        });
    }
    rows.sort_by_key(|row| (row.line, row.message.is_some(), row.col));

    let mut previous: Option<usize> = None;
    let mut index = 0;
    while index < rows.len() {
        let line = rows[index].line;
        match previous {
            Some(p) if line == p + 2 => {
                let _ = write!(
                    out,
                    "\n{} {}",
                    numbered_bar(p + 1, width, styles),
                    shown_line(lines, p + 1),
                );
            }
            Some(p) if line > p + 2 => {
                let _ = write!(
                    out,
                    "\n{}",
                    styles.paint(
                        styles.gutter,
                        &format!("{:<width$} |", "...", width = width)
                    ),
                );
            }
            _ => {}
        }
        if previous != Some(line) {
            let _ = write!(
                out,
                "\n{} {}",
                numbered_bar(line, width, styles),
                shown_line(lines, line),
            );
        }
        let row = &rows[index];
        let from = display_col(lines, row.line, row.col);
        let to = display_col(lines, row.line, row.end_col);
        let marks = to.saturating_sub(from).max(1);
        let underline = match row.message {
            None => styles.paint(severity, &"^".repeat(marks)),
            Some(_) => styles.paint(styles.gutter, &"-".repeat(marks)),
        };
        let _ = write!(
            out,
            "\n{} {}{}",
            bar(width, styles),
            " ".repeat(from - 1),
            underline
        );
        if let Some(message) = row.message
            && !message.is_empty()
        {
            let _ = write!(out, " {message}");
        }
        previous = Some(line);
        index += 1;
    }
}

/// The `= note:` lines — labels the snippet could not draw: a place in
/// another file, or a span the quoted text cannot underline.
fn write_notes(
    out: &mut String,
    report: &Report<'_>,
    labels: &[&Label<'_>],
    width: usize,
    styles: Styles,
) {
    for label in labels {
        let path = label.path.unwrap_or(report.path);
        let _ = write!(
            out,
            "\n{:width$} = {} {} --> {}:{}:{}",
            "",
            styles.paint(styles.help, "note:"),
            label.message,
            path,
            label.span.start.line,
            label.span.start.col,
            width = width
        );
    }
}

/// The `= help:` lines. A suggestion that names a replacement shows it, so
/// the reader can apply the fix without opening an editor's lightbulb.
fn write_suggestions(
    out: &mut String,
    suggestions: &[Suggestion],
    width: usize,
    spaced: bool,
    styles: Styles,
) {
    if suggestions.is_empty() {
        return;
    }
    if spaced {
        let _ = write!(out, "\n{}", bar(width, styles));
    }
    for suggestion in suggestions {
        let _ = write!(
            out,
            "\n{:width$} = {} {}",
            "",
            styles.paint(styles.help, "help:"),
            suggestion.message,
            width = width
        );
        if let Some(edit) = &suggestion.edit {
            // The edit's own bytes carry whatever spacing the insertion
            // needs at its exact offset; on the page that padding is
            // noise, so the *picture* of the fix is trimmed. The bytes a
            // machine applies are unchanged — they travel in the wire
            // form, not in this drawing.
            let text = edit.replacement.trim_matches('\n');
            if text.is_empty() {
                // The sentence already names a deletion. Empty backticks
                // would depict no source and make the actionable advice
                // harder to read.
            } else if text.contains('\n') {
                // A fix that spans lines is quoted the way the snippet
                // above quotes source, so it reads as code rather than as
                // a sentence with newlines in it. Each line keeps the
                // indentation the edit writes, so what the reader sees is
                // what the file would get.
                for line in text.lines() {
                    let _ = write!(out, "\n{} {line}", bar(width, styles));
                }
            } else {
                let _ = write!(out, ": `{}`", text.trim_matches(' '));
            }
        }
    }
}

#[cfg(test)]
mod tests;
