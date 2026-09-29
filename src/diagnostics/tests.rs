use super::*;

#[test]
fn the_typed_and_untyped_wordings_are_one_renderer() {
    let missing = vec!["\"Square\"".to_string(), "\"Tri\"".to_string()];
    assert_eq!(
        non_exhaustive_message(Some("variant Shape"), &missing, 2, true, false),
        "match on variant Shape is not exhaustive: missing \"Square\", \"Tri\"",
    );
    assert_eq!(
        non_exhaustive_message(None, &missing, 2, true, false),
        "match is not exhaustive: missing \"Square\", \"Tri\"",
    );
}

#[test]
fn long_lists_truncate_the_same_way_on_both_paths() {
    let missing: Vec<String> = (0..6).map(|i| format!("\"C{i}\"")).collect();
    let said = non_exhaustive_message(None, &missing, 6, true, false);
    assert!(
        said.contains("\"C0\", \"C1\", \"C2\", … (6 in total)"),
        "{said}"
    );
    let combos: Vec<String> = (0..6).map(|i| format!("(A, B{i})")).collect();
    let said = non_exhaustive_message(None, &combos, 6, true, true);
    assert!(said.contains("… (6 combinations in total)"), "{said}");
    let said = non_exhaustive_message(None, &combos[..2], 9, true, true);
    assert!(
        said.ends_with("missing (A, B0), (A, B1), … (9 combinations in total)"),
        "{said}"
    );
    let said = non_exhaustive_message(None, &combos[..2], 2, false, true);
    assert!(
        said.ends_with("missing (A, B0), (A, B1), … (at least 2 combinations in total)"),
        "{said}"
    );
}

#[test]
fn every_rule_is_listed_once_and_explained() {
    // `as_str` and `explanation` are exhaustive matches, so the
    // compiler catches a new variant in both. `ALL` it cannot check:
    // this count is the prompt to list a new rule there too.
    assert_eq!(DiagnosticCode::ALL.len(), 48);
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
fn code_numbers_are_stable_and_start_at_one() {
    assert_eq!(DiagnosticCode::StrayPipe.number(), 1);
    assert_eq!(DiagnosticCode::MatchNotExhaustive.number(), 27);
    assert_eq!(DiagnosticCode::LoweringPlanFailed.number(), 34);
    assert_eq!(DiagnosticCode::ResultNoSuccessValue.number(), 35);
    assert_eq!(DiagnosticCode::TryCrossesValueRegion.number(), 42);
    assert_eq!(DiagnosticCode::VariantDefaultExport.number(), 50);
    assert_eq!(DiagnosticCode::MissingPipelineStep.number(), 51);
    assert_eq!(
        DiagnosticCode::retired("tt8"),
        Some("result-missing-keyword")
    );
    assert_eq!(DiagnosticCode::retired("33"), Some("result-tail-semicolon"));
    let mut seen = std::collections::HashSet::new();
    for code in DiagnosticCode::ALL {
        let number = code.number();
        assert_ne!(number, 0, "{} has no number", code.as_str());
        assert!(seen.insert(number), "{} shares a number", code.as_str());
    }
}

#[test]
fn a_code_is_looked_up_by_name_or_number() {
    for code in DiagnosticCode::ALL {
        let number = code.number();
        assert_eq!(DiagnosticCode::lookup(code.as_str()), Some(*code));
        assert_eq!(DiagnosticCode::lookup(&format!("tt{number}")), Some(*code));
        assert_eq!(DiagnosticCode::lookup(&number.to_string()), Some(*code));
        assert_eq!(DiagnosticCode::retired(&format!("tt{number}")), None);
    }
    for text in [
        "tt8",
        "8",
        "result-missing-keyword",
        "tt0",
        "0",
        "tt52",
        "tt",
        "",
        "tt-1",
        "ts27",
    ] {
        assert_eq!(DiagnosticCode::lookup(text), None, "{text}");
    }
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

#[test]
fn the_let_else_placement_explanation_allows_the_result_block_it_is_allowed_in() {
    let explanation = DiagnosticCode::LetElsePlacement.explanation();
    assert!(
        explanation.contains("complete\nthat block"),
        "{explanation}"
    );
    assert!(
        !explanation.contains("a `result` block, or"),
        "{explanation}"
    );
}
