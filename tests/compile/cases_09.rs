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
fn result_body_uses_the_planned_slot_for_a_jsx_child_match() {
    let output = ok_tsx(
        r#"import type { TResult } from "@tt/std";
variant E { A, B }
variant F { Yes, No }
declare function step(n: number): number;
declare function fallible(n: number): TResult<number, string>;
export function run(e: E, f: F, n: number): number {
  const value = result {
    const first = try fallible(n);
    const chosen = match (e) { A => 1, B => 2 };
    const view = <section data-value={chosen}>{match (f) {
      Yes => <strong>{chosen |> step}</strong>,
      No => null,
    }}</section>;
    void view;
    return first + chosen;
  };
  return value.kind === "Ok" ? 0 : 1;
}
"#,
    );
    assert_eq!(output.matches("switch (").count(), 2, "{output}");
    assert!(output.contains(">{$tt_v2}</section>"), "{output}");
    assert!(!output.contains("{let $tt_v2;"), "{output}");
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

#[test]
fn a_witness_names_the_value_that_is_missing() {
    let e = err(r#"variant Inner { Yes(n: number), No }
variant Outer { Wrap(inner: Inner), Bare }
const a = match (o) { Wrap(inner: Yes(n)) => n, Bare => -1 };
"#);
    assert!(
        e.message.contains("missing \"Wrap(inner: No())\""),
        "{}",
        e.message
    );
}

#[test]
fn a_fully_guarded_match_still_names_every_case() {
    // No arm covers anything, so every constructor is a witness — the
    // column has no wildcard row to hide behind.
    let e =
        err("const f = (o: Option<number>) => match (o) { Some(value) if value > 0 => value };\n");
    assert!(
        e.message.contains("missing \"Some\", \"None\""),
        "{}",
        e.message
    );
}

#[test]
fn deeply_nested_exhaustiveness_terminates_and_answers() {
    // Three levels of payload, one hole at the bottom.
    let e = err(r#"variant A { A1(b: B), A2 }
variant B { B1(c: C), B2 }
variant C { C1(n: number), C2 }
const v = match (a) {
  A1(b: B1(c: C1(n))) => n,
  A1(b: B2()) => 2,
  A2 => 3,
};
"#);
    assert!(
        e.message.contains("missing \"A1(b: B1(c: C2()))\""),
        "{}",
        e.message
    );
}

#[test]
fn a_witness_can_be_pasted_back_as_an_arm() {
    // The message promises a pattern, not a description: whatever it names
    // must compile as the arm that covers it. A nested unit case is where
    // that promise used to break — `inner: No` *binds* the field to a name
    // called `No`, so the arm compiled and covered every `Wrap`.
    let base = r#"variant Inner { Yes(n: number), No }
variant Outer { Wrap(inner: Inner), Bare }
declare const o: Outer;
const a = match (o) {
  Wrap(inner: Yes(n)) => n,
  Bare => -1,
};
"#;
    let reported = err(base).message;
    let witnesses: Vec<&str> = reported.split('"').skip(1).step_by(2).collect();
    assert_eq!(witnesses, ["Wrap(inner: No())"], "{reported}");

    let arms: String = witnesses.iter().map(|w| format!("  {w} => 0,\n")).collect();
    let pasted = base.replace("  Bare => -1,\n", &format!("  Bare => -1,\n{arms}"));
    let out = ok(&pasted);
    // ...and it really is the No case, not a binding that swallows Wrap.
    assert!(out.contains("$tt_m.inner.kind === \"No\""), "{out}");
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
fn a_non_exhaustive_match_covers_its_head() {
    // The head — not the word the position lands on, and not the arms
    // below it, which are the user's own code.
    let src = "variant S { A(x: number), B }\nconst v = match (s) { A(x) => x };\n";
    let e = err(src);
    assert!(e.message.contains("is not exhaustive"), "{}", e.message);
    assert_eq!((e.line, e.col), (2, 11));
    assert_eq!(covered(src, &e), "match (s)");
}

#[test]
fn a_tuple_match_covers_every_scrutinee() {
    let src = "variant S { A(x: number), B }\nvariant T { C(), D }\nconst v = match (s, t) { (A(x), C) => x };\n";
    let e = err(src);
    assert!(e.message.contains("is not exhaustive"), "{}", e.message);
    assert_eq!(covered(src, &e), "match (s, t)");
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
fn a_misspelled_case_covers_the_name_as_written() {
    let src = "variant Shape { Circle(r: number), Square(s: number) }\nconst v = match (s) { Circel(r) => r, Square(s) => s };\n";
    let e = err(src);
    assert!(e.message.contains("has no case `Circel`"), "{}", e.message);
    assert_eq!(covered(src, &e), "Circel");
}

#[test]
fn a_misplaced_try_covers_the_propagation() {
    let src = "const x = match (r) {\n  Ok(v) => { const y = try f(v); return y; },\n  Err(e) => 0,\n};\n";
    let e = err(src);
    assert!(e.message.contains("`try` cannot be used"), "{}", e.message);
    assert_eq!(covered(src, &e), "try f(v)");
}

#[test]
fn a_val_mutation_covers_the_binding() {
    let src = "function f() {\n  val const cfg = { a: 1 };\n  cfg.a = 2;\n}\n";
    let e = err(src);
    assert!(e.message.contains("cannot mutate"), "{}", e.message);
    assert_eq!(covered(src, &e), "cfg");
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
fn compile_still_returns_the_first_error_in_source_order() {
    let src = "variant Shape { Circle(r: number), Square(s: number) }\n\
        export function f(x: Shape): number {\n  return match (x) { Circle(r) => r };\n}\n\
        export function g(x: Shape): number {\n  return match (x) { Square(s) => s };\n}\n";
    let e = err(src);
    assert_eq!(
        e.line, 3,
        "the first uncovered match decides compile()'s error"
    );
    assert!(e.message.contains("missing \"Square\""));
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
    let src = "const x = 1 |> ;\n";
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
    let src = "const x = 1 |> ;\nconst y = 2 |> ;\n";
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
fn a_loop_header_value_is_not_hoisted_out_of_the_loop() {
    // The value runs once per iteration. The loop owns the region so it is
    // neither hoisted before the loop nor hidden in an expression helper.
    let out = ok(
        "declare function id(v: number): number;\nlet n = 0;\nwhile (id(match (n) { 0 => 1, _ => 0 })) { n = n + 1; }\n",
    );
    let loop_at = out.find("while (true)").expect("loop");
    let lowering = out.find("switch").expect("lowering");
    assert!(loop_at < lowering, "{out}");
    assert!(out.contains("if (!($tt_v1($tt_v0))) break;"), "{out}");
    assert!(!out.contains("$tt_expr"), "{out}");
}

#[test]
fn a_loop_body_value_still_lowers_to_owner_statements() {
    let out =
        ok("let n = 0;\nwhile (n < 3) { const v = match (n) { 0 => 1, _ => 0 }; n = n + v; }\n");
    assert!(out.contains("let $tt_v0;"), "{out}");
    assert!(!out.contains("$tt_expr"), "{out}");
}

#[test]
fn a_capture_never_escapes_a_generated_conditional_region() {
    let source = "declare const flag: boolean;\ndeclare function id(v: number): number;\nexport const short = flag && id(match (flag) { true => 1, _ => 0 });\n";
    let out = ok(source);
    assert!(!out.contains("$tt_expr"), "{out}");
    assert!(out.contains("if ($tt_v2)"), "{out}");
    assert!(out.contains("$tt_v3 = $tt_v1($tt_v0);"), "{out}");
}

#[test]
fn a_capture_never_copies_a_sibling_tt_value() {
    let source = "declare function g(x: unknown, y: unknown): void;\ndeclare const a: boolean;\ng(a && match (a) { true => 1, _ => 0 }, match (a) { true => 2, _ => 3 });\n";
    let out = ok(source);
    assert!(!out.contains("$tt_expr"), "{out}");
    assert!(out.contains("$tt_v5 = $tt_v0;"), "{out}");
    assert!(out.contains("$tt_v3($tt_v5, $tt_v1)"), "{out}");
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

#[test]
fn an_initializer_inside_a_callback_still_lowers_to_statements() {
    // TASK-160: the evaluation protocol is owner-relative — an enclosing
    // call frame beyond the function boundary is not this owner's
    // obligation, so no expression boundary is needed here.
    let out = ok(
        "declare function f(cb: () => number): void;\nf(() => { const x = match (1) { 1 => 1, _ => 0 }; return x; });\n",
    );
    assert!(out.contains("let $tt_v0;"), "{out}");
    assert!(!out.contains("$tt_expr"), "{out}");
}

#[test]
fn a_conditional_operation_lowers_as_one_region() {
    // TASK-160 결정 17: every path of the operation assigns the result
    // slot, so TypeScript keeps the operation's type without `undefined`.
    let out = ok(
        "declare const flag: boolean;\nexport const a = flag && match (1) { 1 => 1, _ => 0 };\n",
    );
    assert!(out.contains("if ($tt_v1) {"), "{out}");
    assert!(out.contains("$tt_v2 = $tt_v1;"), "{out}");
    assert!(out.contains("export const a = $tt_v2;"), "{out}");
    assert!(!out.contains("$tt_expr"), "{out}");
    assert!(!out.contains("&&"), "{out}");
}

#[test]
fn a_ternary_with_one_tt_branch_relocates_the_other_branch() {
    let out = ok(
        "declare const flag: boolean;\nexport const pick = flag ? match (1) { 1 => 1, _ => 0 } : 9;\n",
    );
    assert!(out.contains("} else {"), "{out}");
    assert!(out.contains("= 9;"), "{out}");
    assert!(!out.contains("$tt_expr"), "{out}");
    assert!(!out.contains("?"), "{out}");
}

#[test]
fn an_optional_call_evaluates_arguments_only_past_its_check() {
    let out = ok(
        "declare const f: ((v: number, w: number) => number) | undefined;\ndeclare function pre(): number;\nexport const r = f?.(pre(), match (1) { 1 => 1, _ => 0 });\n",
    );
    let check = out.find("!= null) {").expect("nullish check");
    let prior = out.find("(pre())").expect("prior argument capture");
    assert!(check < prior, "{out}");
    assert!(out.contains("= undefined;"), "{out}");
    assert!(!out.contains("?."), "{out}");
}

#[test]
fn a_spread_argument_capture_takes_the_expression_not_the_dots() {
    let out = ok(
        "declare function sum(...xs: number[]): number;\ndeclare const rest: number[];\nexport const r = sum(...rest, match (1) { 1 => 3, _ => 0 });\n",
    );
    assert!(out.contains("= (rest);"), "{out}");
    assert!(out.contains("(...$tt_v"), "{out}");
}

#[test]
fn an_inert_argument_is_not_captured_but_an_effectful_one_is() {
    // TASK-160 §9: capture elision is proven by effects — a literal stays
    // in place, a call is captured to keep its evaluation order.
    let out = ok(
        "declare function g(a: number, b: number, c: number): void;\ndeclare function eff(): number;\ng(1, match (1) { 1 => 1, _ => 0 }, 2);\ng(eff(), match (1) { 1 => 1, _ => 0 }, 2);\n",
    );
    assert!(!out.contains("= (1);"), "{out}");
    assert!(!out.contains("= (2);"), "{out}");
    assert!(out.contains("= (eff());"), "{out}");
    assert!(out.contains("(1, $tt_v0, 2);"), "{out}");
}
