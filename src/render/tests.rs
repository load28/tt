use super::*;
use crate::diagnostics::Edit;

const SOURCE: &str = "variant Shape { Circle(radius: number), Rect }\ndeclare const s: Shape;\nconst a = match (s) {\n  Circel(r) => r,\n};\n";

fn at(line: usize, col: usize) -> Position {
    Position { line, col }
}

fn report<'a>(span: Option<Span>, suggestions: &'a [Suggestion]) -> Report<'a> {
    Report {
        severity: Severity::Error,
        code: Some("unknown-case"),
        message: "variant Shape has no case `Circel`",
        path: "shapes.tt",
        span,
        labels: &[],
        suggestions,
    }
}

#[test]
fn no_styling_writes_the_same_bytes_it_always_has() {
    // The contract `tests/fixtures/` rests on: adding colour cannot
    // move a byte of the uncoloured picture.
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
    let plain = render(&report(Some(span), &fix), Some(SOURCE), Styles::PLAIN);
    assert!(!plain.contains('\x1b'));
    let painted = render(&report(Some(span), &fix), Some(SOURCE), Styles::ANSI);
    assert_eq!(strip(&painted), plain);
}

#[test]
fn colour_separates_severity_frame_and_advice() {
    let span = Span {
        start: at(4, 3),
        end: Some(at(4, 9)),
    };
    let fix = [Suggestion {
        message: "a case with a similar name exists".to_string(),
        edit: None,
    }];
    let out = render(&report(Some(span), &fix), Some(SOURCE), Styles::ANSI);
    assert!(
        out.starts_with("\x1b[1;31merror[unknown-case]\x1b[0m: "),
        "{out}"
    );
    assert!(out.contains("\x1b[1;34m -->\x1b[0m") || out.contains("\x1b[1;34m-->\x1b[0m"));
    assert!(out.contains("\x1b[1;31m^^^^^^\x1b[0m"), "{out}");
    assert!(out.contains("\x1b[1;36mhelp:\x1b[0m"), "{out}");
}

#[test]
fn a_warning_is_painted_in_its_own_colour() {
    let out = render(
        &Report {
            severity: Severity::Warning,
            code: None,
            message: "the output failed the TypeScript self-check",
            path: "shapes.tt",
            span: None,
            labels: &[],
            suggestions: &[],
        },
        Some(SOURCE),
        Styles::ANSI,
    );
    assert!(out.starts_with("\x1b[1;33mwarning\x1b[0m: "), "{out}");
}

#[test]
fn a_multi_line_span_survives_the_paint_unchanged() {
    let source = "const a = match (s) {\n  Circel(r) => r,\n};\n";
    let span = Span {
        start: at(1, 11),
        end: Some(at(3, 2)),
    };
    let plain = render(&report(Some(span), &[]), Some(source), Styles::PLAIN);
    let painted = render(&report(Some(span), &[]), Some(source), Styles::ANSI);
    assert_eq!(strip(&painted), plain);
}

/// Every SGR sequence removed — what the drawing says once the colour
/// is taken off it.
fn strip(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find('\x1b') {
        out.push_str(&rest[..at]);
        let Some(end) = rest[at..].find('m') else {
            break;
        };
        rest = &rest[at + end + 1..];
    }
    out.push_str(rest);
    out
}

#[test]
fn a_single_line_span_underlines_exactly_the_construct() {
    let span = Span {
        start: at(4, 3),
        end: Some(at(4, 9)),
    };
    let out = render(&report(Some(span), &[]), Some(SOURCE), Styles::PLAIN);
    assert_eq!(
        out,
        "error[unknown-case]: variant Shape has no case `Circel`\n \
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
    let out = render(&report(Some(span), &[]), Some(SOURCE), Styles::PLAIN);
    assert!(out.contains("3 |   const a = match (s) {\n"), "{out}");
    assert!(out.contains("  |  ___________^\n"), "{out}");
    assert!(out.contains("4 | |   Circel(r) => r,\n"), "{out}");
    assert!(out.ends_with("  | |_^"), "{out}");
}

#[test]
fn a_label_on_another_line_joins_the_snippet_with_dashes() {
    let span = Span {
        start: at(4, 3),
        end: Some(at(4, 9)),
    };
    let labels = [Label {
        span: Span {
            start: at(1, 17),
            end: Some(at(1, 23)),
        },
        message: "declared here",
        path: None,
    }];
    let out = render(
        &Report {
            labels: &labels,
            ..report(Some(span), &[])
        },
        Some(SOURCE),
        Styles::PLAIN,
    );
    assert_eq!(
        out,
        "error[unknown-case]: variant Shape has no case `Circel`\n \
             --> shapes.tt:4:3\n  \
             |\n\
             1 | variant Shape { Circle(radius: number), Rect }\n  \
             |                 ------ declared here\n\
             ... |\n\
             4 |   Circel(r) => r,\n  \
             |   ^^^^^^",
    );
}

#[test]
fn a_label_on_the_primary_line_stacks_under_the_carets() {
    let span = Span {
        start: at(4, 3),
        end: Some(at(4, 9)),
    };
    let labels = [Label {
        span: Span {
            start: at(4, 16),
            end: Some(at(4, 17)),
        },
        message: "because of this",
        path: None,
    }];
    let out = render(
        &Report {
            labels: &labels,
            ..report(Some(span), &[])
        },
        Some(SOURCE),
        Styles::PLAIN,
    );
    assert_eq!(
        out,
        "error[unknown-case]: variant Shape has no case `Circel`\n \
             --> shapes.tt:4:3\n  \
             |\n\
             4 |   Circel(r) => r,\n  \
             |   ^^^^^^\n  \
             |                - because of this",
    );
}

#[test]
fn an_adjacent_line_bridges_instead_of_eliding() {
    let span = Span {
        start: at(4, 3),
        end: Some(at(4, 9)),
    };
    let labels = [Label {
        span: Span {
            start: at(2, 15),
            end: Some(at(2, 20)),
        },
        message: "the scrutinee's type",
        path: None,
    }];
    let out = render(
        &Report {
            labels: &labels,
            ..report(Some(span), &[])
        },
        Some(SOURCE),
        Styles::PLAIN,
    );
    assert!(out.contains("2 | declare const s: Shape;"), "{out}");
    assert!(
        out.contains("3 | const a = match (s) {"),
        "one plain line bridges the gap: {out}"
    );
    assert!(!out.contains("..."), "{out}");
}

#[test]
fn a_cross_file_label_degrades_to_a_note() {
    let span = Span {
        start: at(4, 3),
        end: Some(at(4, 9)),
    };
    let labels = [Label {
        span: Span {
            start: at(7, 1),
            end: Some(at(7, 5)),
        },
        message: "first declared here",
        path: Some("other.tt"),
    }];
    let out = render(
        &Report {
            labels: &labels,
            ..report(Some(span), &[])
        },
        Some(SOURCE),
        Styles::PLAIN,
    );
    assert!(
        out.ends_with("  = note: first declared here --> other.tt:7:1"),
        "{out}"
    );
    assert!(!out.contains("first declared here -->\n"), "{out}");
}

#[test]
fn an_unlabeled_report_renders_byte_identically_to_before() {
    // The labels field must not move a byte of the picture existing
    // fixtures pin.
    let span = Span {
        start: at(4, 3),
        end: Some(at(4, 9)),
    };
    let out = render(&report(Some(span), &[]), Some(SOURCE), Styles::PLAIN);
    assert_eq!(
        out,
        "error[unknown-case]: variant Shape has no case `Circel`\n \
             --> shapes.tt:4:3\n  \
             |\n\
             4 |   Circel(r) => r,\n  \
             |   ^^^^^^",
    );
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
    let out = render(&report(Some(span), &fix), Some(SOURCE), Styles::PLAIN);
    assert!(
        out.ends_with("  = help: a case with a similar name exists: `Circle`"),
        "{out}",
    );
}

#[test]
fn a_deletion_suggestion_needs_no_empty_code_sample() {
    let fix = [Suggestion {
        message: "remove the trailing `;`".to_string(),
        edit: Some(Edit {
            start: 0,
            end: 1,
            replacement: String::new(),
        }),
    }];
    let out = render(&report(None, &fix), None, Styles::PLAIN);
    assert!(out.ends_with(" = help: remove the trailing `;`"), "{out}");
}

#[test]
fn advice_with_no_edit_still_renders_as_help() {
    let fix = [Suggestion {
        message: "add the missing arms or a final `_` arm".to_string(),
        edit: None,
    }];
    let out = render(&report(None, &fix), None, Styles::PLAIN);
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
    let out = render(&report(Some(span), &[]), None, Styles::PLAIN);
    assert_eq!(
        out,
        "error[unknown-case]: variant Shape has no case `Circel`\n --> shapes.tt:4:3",
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
            labels: &[],
            suggestions: &[],
        },
        Some(SOURCE),
        Styles::PLAIN,
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
    let out = render(&report(Some(span), &[]), Some(source), Styles::PLAIN);
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
    let out = render(&report(Some(span), &[]), Some(SOURCE), Styles::PLAIN);
    assert!(out.ends_with("  |   ^"), "{out}");
}

#[test]
fn a_span_past_the_end_of_a_stale_buffer_does_not_panic() {
    let span = Span {
        start: at(400, 90),
        end: Some(at(900, 3)),
    };
    let out = render(&report(Some(span), &[]), Some(SOURCE), Styles::PLAIN);
    assert!(out.contains("--> shapes.tt:400:90"), "{out}");
}

#[test]
fn a_long_span_elides_its_middle() {
    let source: String = (1..=40).map(|n| format!("line {n}\n")).collect();
    let span = Span {
        start: at(2, 1),
        end: Some(at(30, 2)),
    };
    let out = render(&report(Some(span), &[]), Some(&source), Styles::PLAIN);
    assert!(out.contains("| |_^"), "{out}");
    assert!(out.contains("... | |"), "{out}");
    assert!(out.contains("30 | | line 30"), "{out}");
    assert!(!out.contains("line 20"), "the middle must be elided\n{out}");
}
