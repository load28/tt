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
//! use ttc::render::{Position, Report, Span, render};
//!
//! let out = render(
//!     &Report {
//!         severity: ttc::Severity::Error,
//!         code: Some("match-not-exhaustive"),
//!         message: "match on enum Shape is not exhaustive: missing \"Rect\"",
//!         path: "shapes.tt",
//!         span: Some(Span {
//!             start: Position { line: 1, col: 1 },
//!             end: Some(Position { line: 1, col: 14 }),
//!         }),
//!         suggestions: &[],
//!     },
//!     Some("match (shape) {\n}\n"),
//! );
//! assert!(out.starts_with("error[match-not-exhaustive]: match on enum Shape"));
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
pub fn render(report: &Report<'_>, source: Option<&str>) -> String {
    let mut out = String::new();
    match report.code {
        Some(code) => {
            let _ = write!(
                out,
                "{}[{}]: {}",
                report.severity.label(),
                code,
                report.message
            );
        }
        None => {
            let _ = write!(out, "{}: {}", report.severity.label(), report.message);
        }
    }

    let snippet = report
        .span
        .and_then(|span| source.map(|source| (span, source)));
    let Some((span, source)) = snippet else {
        // No span, or no text to quote it from: name the file (and the
        // position, when there is one) and stop.
        match report.span {
            Some(span) => {
                let _ = write!(
                    out,
                    "\n --> {}:{}:{}",
                    report.path, span.start.line, span.start.col
                );
            }
            None => {
                let _ = write!(out, "\n --> {}", report.path);
            }
        }
        write_suggestions(&mut out, report.suggestions, 0, false);
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

    let width = end_line.max(start.line).to_string().len();
    let bar = format!("{:width$} |", "", width = width);
    let _ = write!(
        out,
        "\n{:width$}--> {}:{}:{}\n{bar}",
        "",
        report.path,
        start.line,
        start.col,
        width = width
    );

    if end_line == start.line {
        write_single_line(&mut out, &lines, width, start, end);
    } else {
        write_multi_line(&mut out, &lines, width, start, end_line, end.col);
    }

    write_suggestions(&mut out, report.suggestions, width, true);
    out
}

/// Draws a [`crate::Diagnostic`] found in `source`, named as `path`.
///
/// The byte offsets the untyped pipeline reports in are converted here, so
/// a caller holding a compile report does not have to know that the
/// renderer works in line/column.
pub fn diagnostic(diagnostic: &crate::Diagnostic, source: &str, path: &str) -> String {
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
            suggestions: &diagnostic.suggestions,
        },
        Some(source),
    )
}

/// Draws a [`crate::CompileError`] — the single-error form the library's
/// own `Display` renders as one line.
///
/// A `CompileError` carries no rule code, so the header opens with the
/// severity alone. `source` is the text it was found in, when the caller
/// holds it.
pub fn compile_error(error: &crate::CompileError, source: Option<&str>, path: &str) -> String {
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
            suggestions: &[],
        },
        source,
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
) -> String {
    let at = |(line, col): (usize, usize)| Position { line, col };
    let span = diagnostic.position.map(|start| Span {
        start: at(start),
        end: diagnostic.end.map(at),
    });
    render(
        &Report {
            // Every diagnostic a checked project reports stops the build;
            // the severity space exists for tt rules that do not yet.
            severity: Severity::Error,
            code: diagnostic.code.as_deref(),
            message: &diagnostic.message,
            path,
            span,
            suggestions: &diagnostic.suggestions,
        },
        source,
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
) {
    let from = display_col(lines, start.line, start.col);
    let to = display_col(lines, start.line, end.col);
    let carets = to.saturating_sub(from).max(1);
    let _ = write!(
        out,
        "\n{:>width$} | {}\n{:width$} | {}{}",
        start.line,
        shown_line(lines, start.line),
        "",
        " ".repeat(from - 1),
        "^".repeat(carets),
        width = width
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
fn write_multi_line(
    out: &mut String,
    lines: &[&str],
    width: usize,
    start: Position,
    end_line: usize,
    end_col: usize,
) {
    let from = display_col(lines, start.line, start.col);
    let _ = write!(
        out,
        "\n{:>width$} |   {}\n{:width$} |  {}^",
        start.line,
        shown_line(lines, start.line),
        "",
        "_".repeat(from),
        width = width
    );

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
            let _ = write!(out, "\n{:<width$} | |", "...", width = width);
        }
        let _ = write!(
            out,
            "\n{:>width$} | | {}",
            line,
            shown_line(lines, *line),
            width = width
        );
    }

    let to = display_col(lines, end_line, end_col.saturating_sub(1).max(1));
    let _ = write!(out, "\n{:width$} | |{}^", "", "_".repeat(to), width = width);
}

/// The `= help:` lines. A suggestion that names a replacement shows it, so
/// the reader can apply the fix without opening an editor's lightbulb.
fn write_suggestions(out: &mut String, suggestions: &[Suggestion], width: usize, spaced: bool) {
    if suggestions.is_empty() {
        return;
    }
    if spaced {
        let _ = write!(out, "\n{:width$} |", "", width = width);
    }
    for suggestion in suggestions {
        let _ = write!(
            out,
            "\n{:width$} = help: {}",
            "",
            suggestion.message,
            width = width
        );
        if let Some(edit) = &suggestion.edit {
            let _ = write!(out, ": `{}`", edit.replacement);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Edit;

    const SOURCE: &str = "enum Shape { Circle(radius: number), Rect }\ndeclare const s: Shape;\nconst a = match (s) {\n  Circel(r) => r,\n};\n";

    fn at(line: usize, col: usize) -> Position {
        Position { line, col }
    }

    fn report<'a>(span: Option<Span>, suggestions: &'a [Suggestion]) -> Report<'a> {
        Report {
            severity: Severity::Error,
            code: Some("unknown-case"),
            message: "enum Shape has no case `Circel`",
            path: "shapes.tt",
            span,
            suggestions,
        }
    }

    #[test]
    fn a_single_line_span_underlines_exactly_the_construct() {
        let span = Span {
            start: at(4, 3),
            end: Some(at(4, 9)),
        };
        let out = render(&report(Some(span), &[]), Some(SOURCE));
        assert_eq!(
            out,
            "error[unknown-case]: enum Shape has no case `Circel`\n \
             --> shapes.tt:4:3\n  \
             |\n\
             4 |   Circel(r) => r,\n  \
             |   ^^^^^^",
        );
    }

    #[test]
    fn a_multi_line_span_brackets_the_lines_it_covers() {
        let span = Span {
            start: at(3, 11),
            end: Some(at(5, 2)),
        };
        let out = render(&report(Some(span), &[]), Some(SOURCE));
        assert!(out.contains("3 |   const a = match (s) {\n"), "{out}");
        assert!(out.contains("  |  ___________^\n"), "{out}");
        assert!(out.contains("4 | |   Circel(r) => r,\n"), "{out}");
        assert!(out.ends_with("  | |_^"), "{out}");
    }

    #[test]
    fn a_suggestion_shows_its_replacement() {
        let span = Span {
            start: at(4, 3),
            end: Some(at(4, 9)),
        };
        let fix = [Suggestion {
            message: "a case with a similar name exists".to_string(),
            edit: Some(Edit {
                start: 0,
                end: 0,
                replacement: "Circle".to_string(),
            }),
        }];
        let out = render(&report(Some(span), &fix), Some(SOURCE));
        assert!(
            out.ends_with("  = help: a case with a similar name exists: `Circle`"),
            "{out}",
        );
    }

    #[test]
    fn advice_with_no_edit_still_renders_as_help() {
        let fix = [Suggestion {
            message: "add the missing arms or a final `_` arm".to_string(),
            edit: None,
        }];
        let out = render(&report(None, &fix), None);
        assert!(
            out.ends_with(" = help: add the missing arms or a final `_` arm"),
            "{out}",
        );
    }

    #[test]
    fn without_source_the_header_and_location_still_render() {
        let span = Span {
            start: at(4, 3),
            end: Some(at(4, 9)),
        };
        let out = render(&report(Some(span), &[]), None);
        assert_eq!(
            out,
            "error[unknown-case]: enum Shape has no case `Circel`\n --> shapes.tt:4:3",
        );
    }

    #[test]
    fn a_positionless_diagnostic_names_only_its_file() {
        let out = render(
            &Report {
                severity: Severity::Warning,
                code: None,
                message: "the output failed the TypeScript self-check",
                path: "shapes.tt",
                span: None,
                suggestions: &[],
            },
            Some(SOURCE),
        );
        assert_eq!(
            out,
            "warning: the output failed the TypeScript self-check\n --> shapes.tt",
        );
    }

    #[test]
    fn tabs_expand_so_the_caret_lands_under_its_construct() {
        let source = "f();\n\t\tCircel(r);\n";
        let span = Span {
            start: at(2, 3),
            end: Some(at(2, 9)),
        };
        let out = render(&report(Some(span), &[]), Some(source));
        let lines: Vec<&str> = out.lines().collect();
        let text = lines.iter().find(|l| l.starts_with("2 |")).unwrap();
        let carets = lines.iter().find(|l| l.contains('^')).unwrap();
        assert_eq!(
            text.find("Circel"),
            carets.find('^'),
            "caret and construct must start in the same column\n{out}",
        );
    }

    #[test]
    fn an_end_before_its_start_still_renders_one_caret() {
        let span = Span {
            start: at(4, 3),
            end: Some(at(2, 1)),
        };
        let out = render(&report(Some(span), &[]), Some(SOURCE));
        assert!(out.ends_with("  |   ^"), "{out}");
    }

    #[test]
    fn a_span_past_the_end_of_a_stale_buffer_does_not_panic() {
        let span = Span {
            start: at(400, 90),
            end: Some(at(900, 3)),
        };
        let out = render(&report(Some(span), &[]), Some(SOURCE));
        assert!(out.contains("--> shapes.tt:400:90"), "{out}");
    }

    #[test]
    fn a_long_span_elides_its_middle() {
        let source: String = (1..=40).map(|n| format!("line {n}\n")).collect();
        let span = Span {
            start: at(2, 1),
            end: Some(at(30, 2)),
        };
        let out = render(&report(Some(span), &[]), Some(&source));
        assert!(out.contains("| |_^"), "{out}");
        assert!(out.contains("... | |"), "{out}");
        assert!(out.contains("30 | | line 30"), "{out}");
        assert!(!out.contains("line 20"), "the middle must be elided\n{out}");
    }
}
