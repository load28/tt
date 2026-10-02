#[test]
fn merge_conflict_markers_report_errors_without_panicking() {
    for source_kind in [SourceKind::TypeScript, SourceKind::Tsx] {
        let options = Options {
            source_kind,
            ..Options::default()
        };
        for marker in ["=======", "<<<<<<< ours", ">>>>>>> theirs", "||||||| base"] {
            for source in [
                format!("{marker}\n/\u{2}\n"),
                format!("const before = 1;\n  {marker}\n/regex/;"),
                format!("const value = `raw ${{\n{marker}\n/regex/}}`;"),
                format!("const value = 1 |> String;\n{marker}\n/regex/;"),
                format!("/* leading trivia */{marker}\n/regex/;"),
                format!("const before = 1; /*\n*/ {marker}\n/regex/;"),
                format!("const before = 1;\u{2028}{marker}\n/regex/;"),
                format!("const before = 1;\u{2029}  {marker}\r/regex/;"),
            ] {
                ttc::analyze(&source, &options);
                let error = compile(&source, &options).expect_err(&source);
                assert!(
                    error.message.contains("merge conflict marker"),
                    "{source_kind:?}: {source:?}: {error}"
                );
            }
        }
    }
}

#[test]
fn conflict_marker_text_in_literals_and_comments_is_preserved() {
    for source_kind in [SourceKind::TypeScript, SourceKind::Tsx] {
        let options = Options {
            source_kind,
            ..Options::default()
        };
        for source in [
            "const text = '======= <<<<<<< ours >>>>>>> theirs ||||||| base';",
            "const text = `\n=======\n<<<<<<< ours\n>>>>>>> theirs\n||||||| base`;",
            "/*\n=======\n<<<<<<< ours\n>>>>>>> theirs\n||||||| base\n*/\nconst n = 1;",
            "// =======\nconst pattern = /=======/;",
            "const text = `raw ${`\n=======\n`}`;",
        ] {
            assert_eq!(compile(source, &options).unwrap(), source);
        }
    }
    let source = "const view = <pre>\n=======\n||||||| base\n</pre>;";
    assert_eq!(ok_tsx(source), source);
}

#[test]
fn malformed_pipeline_tail_never_reaches_codegen_as_owned_source() {
    let source = "\u{6}|>'\u{b}";
    for source_kind in [SourceKind::TypeScript, SourceKind::Tsx] {
        let result = std::panic::catch_unwind(|| {
            compile(
                source,
                &Options {
                    source_kind,
                    ..Options::default()
                },
            )
        });
        assert!(result.is_ok(), "{source_kind:?} panicked");
    }
}

#[test]
fn malformed_namespaced_jsx_member_is_reported_without_panicking() {
    let options = Options {
        source_kind: SourceKind::Tsx,
        ..Options::default()
    };
    for source in ["<G:U.m", "<G:U.m |> String"] {
        let analyzed = std::panic::catch_unwind(|| ttc::analyze(source, &options));
        assert!(analyzed.is_ok(), "analysis panicked for {source:?}");

        let compiled = std::panic::catch_unwind(|| compile(source, &options));
        let error = compiled
            .unwrap_or_else(|_| panic!("compilation panicked for {source:?}"))
            .expect_err("malformed TSX must not compile");
        assert!(
            error
                .message
                .contains("JSX namespace name cannot be followed by member access"),
            "{source:?}: {error}"
        );
    }
}

#[test]
fn unterminated_template_interpolation_never_overlaps_source_spans() {
    // This is an incomplete editor buffer: the `${` has no closing brace.
    // The interpolation runs to the end, and codegen must still preserve
    // every byte exactly once.
    let source = String::from_utf8(vec![60, 96, 0, 0, 0, 123, 36, 123, 10, 0]).unwrap();
    for source_kind in [SourceKind::TypeScript, SourceKind::Tsx] {
        let result = std::panic::catch_unwind(|| {
            compile(
                &source,
                &Options {
                    source_kind,
                    ..Options::default()
                },
            )
        });
        assert!(result.is_ok(), "{source_kind:?} panicked");
    }
}

/// An interpolation whose `}` is missing is read as TypeScript's scanner
/// reads it: expression tokens to the end of the file. Its `${` is the
/// unbalanced delimiter the check reports, whatever the interpolation holds,
/// and the editor's projection keeps every byte where it was.
#[test]
fn an_unterminated_interpolation_is_an_open_expression() {
    for (source, column) in [
        ("const at = new Date();\nconst s = `returned ${at.", 21),
        ("const s = `a ${x} b ${y", 21),
        ("const s = `a ${x |> f", 14),
        ("const s = `a ${", 14),
    ] {
        for source_kind in [SourceKind::TypeScript, SourceKind::Tsx] {
            let error = compile(
                source,
                &Options {
                    source_kind,
                    ..Options::default()
                },
            )
            .expect_err("an unterminated template does not compile");
            assert!(
                error.message.contains("unbalanced TypeScript delimiter"),
                "{source}: {}",
                error.message
            );
            assert_eq!(
                (error.line, error.col),
                (source.matches('\n').count() + 1, column),
                "{source}"
            );
        }
        if !source.contains("|>") {
            let emit = ttc::emit_mapped(source);
            assert_eq!(emit.code, source);
            assert_eq!(
                emit.mappings,
                [ttc::EmitMapping {
                    src: 0,
                    out: 0,
                    len: source.len()
                }]
            );
        }
    }
}

#[test]
fn pipeline_values_containing_double_slashes_do_not_become_comments() {
    for source in [r#""//" |> String"#, r#"`//` |> String"#, r#"/\/\// |> String"#] {
        for source_kind in [SourceKind::TypeScript, SourceKind::Tsx] {
            compile(
                source,
                &Options {
                    source_kind,
                    ..Options::default()
                },
            )
            .unwrap_or_else(|error| panic!("{source_kind:?} rejected {source:?}: {error}"));
        }
    }
}

/* ------------------------------------------------------------------ */
/* diagnostic ranges (TASK-116)                                        */
/* ------------------------------------------------------------------ */

/// The source text an error's range covers — what an editor underlines.
fn covered(src: &str, e: &ttc::CompileError) -> String {
    let offset = |line: usize, col: usize| {
        src.split_inclusive('\n')
            .take(line - 1)
            .map(str::len)
            .sum::<usize>()
            + col
            - 1
    };
    src[offset(e.line, e.col)..offset(e.end_line, e.end_col)].to_string()
}

#[test]
fn a_duplicate_arm_covers_the_tag_it_repeats() {
    let src =
        "variant S { A(x: number), B }\nconst v = match (s) { A(x) => x, A(x) => 0, B => 1 };\n";
    let e = err(src);
    assert!(e.message.contains("duplicate arm"), "{}", e.message);
    assert_eq!(covered(src, &e), "A");
}

#[test]
fn an_error_without_a_known_extent_reports_a_position_only() {
    // No end means "the consumer decides the width" — the editor then
    // underlines the word at the position, as it always has.
    let src = "variant S { A(x: number), B }\nconst v = match (s) { A(x) => x, _ => 0, B => 1 };\n";
    let e = err(src);
    assert!(e.message.contains("must be the last arm"), "{}", e.message);
    assert_eq!(covered(src, &e), "_");
}

/* ------------------------------------------------------------------ */
/* multiple diagnostics (TASK-120)                                     */
/* ------------------------------------------------------------------ */

#[test]
fn analyze_reports_every_uncovered_match_in_source_order() {
    // TASK-117 symptom 1: the default path used to report one error per
    // run; tsc and rustc report them all.
    let src = "variant Shape { Circle(r: number), Square(s: number), Tri(a: number) }\n\
        export function f(x: Shape): number {\n  return match (x) { Circle(r) => r };\n}\n\
        export function g(x: Shape): number {\n  return match (x) { Square(s) => s };\n}\n\
        export function h(x: Shape): number {\n  return match (x) { Tri(a) => a };\n}\n";
    let diagnostics = ttc::analyze(src, &Options::default());
    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|d| d.code == ttc::DiagnosticCode::MatchNotExhaustive)
    );
    let starts: Vec<usize> = diagnostics.iter().map(|d| d.start.unwrap()).collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();
    assert_eq!(starts, sorted, "diagnostics arrive in source order");
    assert!(
        diagnostics[0]
            .message
            .contains("missing \"Square\", \"Tri\"")
    );
    assert!(
        diagnostics[1]
            .message
            .contains("missing \"Circle\", \"Tri\"")
    );
    assert!(
        diagnostics[2]
            .message
            .contains("missing \"Circle\", \"Square\"")
    );
}

#[test]
fn a_duplicate_arm_does_not_hide_the_files_other_diagnostics() {
    // TASK-117 symptom 3, the tt half: one recoverable error used to stop
    // the whole check.
    let src = "variant Shape { Circle(r: number), Square(s: number), Tri(a: number) }\n\
        export function f(x: Shape): number {\n\
          return match (x) { Circle(r) => r, Circle(r) => 0, Square(s) => s, Tri(a) => a };\n\
        }\n\
        export function g(x: Shape): number { return match (x) { Square(s) => s }; }\n\
        export function h(x: Shape): number { return match (x) { Tri(a) => a }; }\n";
    let diagnostics = ttc::analyze(src, &Options::default());
    let codes: Vec<_> = diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(
        codes,
        [
            ttc::DiagnosticCode::MatchDuplicateArm,
            ttc::DiagnosticCode::MatchNotExhaustive,
            ttc::DiagnosticCode::MatchNotExhaustive,
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn a_typo_suppresses_coverage_for_its_own_match_only() {
    // The recovery boundary is the match, not the file: `Circel` silences
    // f's exhaustiveness question (the typo is the cause), while g's hole
    // is still reported.
    let src = "variant Shape { Circle(r: number), Empty }\n\
        export function f(x: Shape): number {\n\
          return match (x) { Circel(r) => r, Empty => 0 };\n\
        }\n\
        export function g(x: Shape): number { return match (x) { Empty => 0 }; }\n";
    let diagnostics = ttc::analyze(src, &Options::default());
    let codes: Vec<_> = diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(
        codes,
        [
            ttc::DiagnosticCode::UnknownCase,
            ttc::DiagnosticCode::MatchNotExhaustive,
        ],
        "{diagnostics:#?}"
    );
    assert!(diagnostics[0].message.contains("has no case `Circel`"));
    assert!(diagnostics[1].message.contains("missing \"Circle\""));
}

#[test]
fn sema_and_val_diagnostics_merge_in_source_order() {
    let src = "variant E { A(x: number), B }\n\
        val const cfg = { a: 1 };\n\
        cfg.a = 2;\n\
        const v = match (E.B) { B => 0 };\n";
    let diagnostics = ttc::analyze(src, &Options::default());
    let codes: Vec<_> = diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(
        codes,
        [
            ttc::DiagnosticCode::ValMutation,
            ttc::DiagnosticCode::MatchNotExhaustive,
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn compile_report_still_emits_under_recoverable_errors() {
    // Codegen is infallible, so a duplicate arm does not withhold the
    // lowered TypeScript — that is what lets the typed pass run and report
    // alongside the tt errors (TASK-117 symptom 3).
    let src = "variant E { A(x: number), B }\n\
        const v = match (E.A(1)) { A(x) => x, A(x) => 0, B => 1 };\n";
    let report = ttc::compile_report(src, &Options::default());
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(
        report.diagnostics[0].code,
        ttc::DiagnosticCode::MatchDuplicateArm
    );
    let emit = report.emit.expect("recoverable errors still emit");
    assert!(emit.code.contains("switch ($tt_m.kind)"));
}

#[test]
fn duplicate_variant_case_emits_only_one_constructor_property() {
    let src = "variant E { A(x: number), B, A(y: number) }\n";
    let report = ttc::compile_report(src, &Options::default());
    assert_eq!(report.diagnostics.len(), 1, "{:#?}", report.diagnostics);
    assert_eq!(
        report.diagnostics[0].code,
        ttc::DiagnosticCode::VariantDuplicateCase
    );
    let code = report.emit.expect("duplicate cases are recoverable").code;
    assert_eq!(code.matches("  A:").count(), 1, "{code}");
    assert!(code.contains("  A: (x: number)"), "{code}");
}

#[test]
fn duplicate_pattern_binding_is_renamed_in_recovery_output() {
    let src = "variant E { A(left: number, right: number), B }\n\
        const value = match (E.A(1, 2)) { A(left: x, right: x) => x, B => 0 };\n";
    let report = ttc::compile_report(src, &Options::default());
    assert_eq!(report.diagnostics.len(), 1, "{:#?}", report.diagnostics);
    assert_eq!(
        report.diagnostics[0].code,
        ttc::DiagnosticCode::PatternDuplicateBinding
    );
    let code = report
        .emit
        .expect("duplicate bindings are recoverable")
        .code;
    assert!(
        code.contains("const { left: x, right: $tt_discard0 } = $tt_m;"),
        "{code}"
    );
}

#[test]
fn duplicate_nested_binding_is_renamed_across_destructuring_statements() {
    let src = "variant Inner { Some(value: number), None }\n\
        variant Outer { Ok(value: Inner, error: number), Err }\n\
        const value = match (Outer.Err) {\n\
          Ok(value: Some(value), error: value) => value,\n\
          Err => 0,\n\
        };\n";
    let report = ttc::compile_report(src, &Options::default());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == ttc::DiagnosticCode::PatternDuplicateBinding),
        "{:#?}",
        report.diagnostics
    );
    let code = report
        .emit
        .expect("duplicate bindings are recoverable")
        .code;
    assert!(
        compact(&code).contains(
            "const { error: value } = $tt_m; const { value: $tt_discard0 } = $tt_m.value;"
        ),
        "{code}"
    );
}

#[test]
fn duplicate_tuple_binding_is_renamed_across_tuple_elements() {
    let src = "variant E { A(value: number), B }\n\
        const value = match (E.A(1), E.A(2)) { (A(value: x), A(value: x)) => x, _ => 0 };\n";
    let report = ttc::compile_report(src, &Options::default());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == ttc::DiagnosticCode::PatternDuplicateBinding),
        "{:#?}",
        report.diagnostics
    );
    let code = report
        .emit
        .expect("duplicate bindings are recoverable")
        .code;
    assert!(
        compact(&code)
            .contains("const { value: x } = $tt_m0; const { value: $tt_discard0 } = $tt_m1;"),
        "{code}"
    );
}

#[test]
fn compile_report_withholds_emission_when_the_output_cannot_be_typescript() {
    // A stray `|>` passes through verbatim, so the output would not parse:
    // that diagnostic blocks projection.
    let src = "const x = a ? 1 : 2 |> f;\n";
    let report = ttc::compile_report(src, &Options::default());
    assert!(report.emit.is_none());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == ttc::DiagnosticCode::StrayPipe),
        "{:#?}",
        report.diagnostics
    );
}

#[test]
fn every_stray_construct_is_reported_not_just_the_first() {
    let src = "const x = a ? 1 : 2 |> f;\nconst y = a ? 1 : 2 |> f;\n";
    let diagnostics = ttc::analyze(src, &Options::default());
    let strays = diagnostics
        .iter()
        .filter(|d| d.code == ttc::DiagnosticCode::StrayPipe)
        .count();
    assert_eq!(strays, 2, "{diagnostics:#?}");
}

#[test]
fn a_mixed_match_reports_the_cause_and_suppresses_its_coverage() {
    // The mixed-pattern error is the cause; that match's own exhaustiveness
    // answer would be an effect stacked on it.
    let src = "const v = match (x) {\n  Some(v) => v,\n  222 => 0,\n};\n";
    let diagnostics = ttc::analyze(src, &Options::default());
    let codes: Vec<_> = diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(
        codes,
        [ttc::DiagnosticCode::MatchMixedPatterns],
        "{diagnostics:#?}"
    );
    assert_eq!(
        &src[diagnostics[0].start.unwrap()..diagnostics[0].end.unwrap()],
        "222"
    );
}

#[test]
fn diagnostic_codes_are_stable_strings() {
    assert_eq!(
        ttc::DiagnosticCode::MatchNotExhaustive.as_str(),
        "match-not-exhaustive"
    );
    assert_eq!(ttc::DiagnosticCode::ValMutation.as_str(), "val-mutation");
    assert!(ttc::DiagnosticCode::StrayPipe.blocks_projection());
    assert!(ttc::DiagnosticCode::MalformedMatch.blocks_projection());
    assert!(!ttc::DiagnosticCode::MatchDuplicateArm.blocks_projection());
}

#[test]
fn malformed_match_blocks_codegen_even_beside_a_lowered_variant() {
    let src = "variant Shape { Circle(r: number), Square(s: number) }\n\
        export function area(shape: Shape): number {\n\
          return match shape { Circle(r) => r, Square(s) => s };\n\
        }\n";
    let report = ttc::compile_report(src, &Options::default());
    assert!(report.emit.is_none());
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == ttc::DiagnosticCode::MalformedMatch),
        "{:#?}",
        report.diagnostics
    );
}

#[test]
fn a_switch_case_test_value_stays_behind_its_case() {
    let diagnostics = ttc::analyze(
        "declare const n: number;\nswitch (n) { case match (n) { 1 => 1, _ => 0 }: break; }\n",
        &Options::default(),
    );
    assert_eq!(diagnostics[0].code, ttc::DiagnosticCode::MatchPlacement);
}

#[test]
fn a_destructuring_default_value_stays_inside_the_default() {
    let diagnostics = ttc::analyze(
        "declare const source: { value?: number };\nexport const { value = match (1) { 1 => 1, _ => 0 } } = source;\n",
        &Options::default(),
    );
    assert_eq!(diagnostics[0].code, ttc::DiagnosticCode::MatchPlacement);
}
