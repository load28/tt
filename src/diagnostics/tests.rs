use super::*;

#[test]
fn the_typed_and_untyped_wordings_are_one_renderer() {
    let missing = vec!["\"Square\"".to_string(), "\"Tri\"".to_string()];
    assert_eq!(
        non_exhaustive_message(Some("variant Shape"), &missing, false),
        "match on variant Shape is not exhaustive: missing \"Square\", \"Tri\"",
    );
    assert_eq!(
        non_exhaustive_message(None, &missing, false),
        "match is not exhaustive: missing \"Square\", \"Tri\"",
    );
}

#[test]
fn long_lists_truncate_the_same_way_on_both_paths() {
    let missing: Vec<String> = (0..6).map(|i| format!("\"C{i}\"")).collect();
    let said = non_exhaustive_message(None, &missing, false);
    assert!(
        said.contains("\"C0\", \"C1\", \"C2\", … (6 in total)"),
        "{said}"
    );
    let combos: Vec<String> = (0..6).map(|i| format!("(A, B{i})")).collect();
    let said = non_exhaustive_message(None, &combos, true);
    assert!(said.contains("… (6 combinations in total)"), "{said}");
}

#[test]
fn every_rule_is_listed_once_and_explained() {
    // `as_str` and `explanation` are exhaustive matches, so the
    // compiler catches a new variant in both. `ALL` it cannot check:
    // this count is the prompt to list a new rule there too.
    assert_eq!(DiagnosticCode::ALL.len(), 44);
    let mut seen = std::collections::HashSet::new();
    for code in DiagnosticCode::ALL {
        let wire = code.as_str();
        assert!(seen.insert(wire), "two rules share the code {wire}");
        assert_eq!(
            DiagnosticCode::parse(wire),
            Some(*code),
            "{wire} does not round-trip through its wire form",
        );
        let explanation = code.explanation();
        assert!(
            explanation.lines().count() >= 2,
            "{wire} needs an explanation longer than its message",
        );
        assert!(
            !explanation.ends_with('\n'),
            "{wire}: the caller adds the trailing newline",
        );
    }
}

#[test]
fn an_unknown_code_has_no_rule() {
    assert_eq!(DiagnosticCode::parse("no-such-rule"), None);
    assert_eq!(DiagnosticCode::parse(""), None);
}

#[test]
fn a_diagnostic_converts_to_the_cli_error_form() {
    let d = Diagnostic {
        code: DiagnosticCode::MatchDuplicateArm,
        severity: Severity::Error,
        message: "match: duplicate arm \"A\"".to_string(),
        start: Some(5),
        end: Some(6),
        owner: None,
        suggestions: Vec::new(),
    };
    let e = d.to_compile_error("abc\ndef\n", Some("x.tt"));
    assert_eq!((e.line, e.col), (2, 2));
    assert_eq!(e.to_string(), "x.tt:2:2: match: duplicate arm \"A\"");
}
