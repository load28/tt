#[test]
fn apply_partitions_structured_children_by_their_function_host() {
    let source = "declare const flag: boolean;\n\
        function probe() {\n\
          const value = (match (flag) { true => 1, false => 2 })\n\
            |> ((input: number) => match (flag) { true => input, false => 0 });\n\
          return value;\n\
        }\n";
    let output = ok(source);
    assert_eq!(output.matches("switch (").count(), 2, "{output}");

    let parameter = "declare const flag: boolean;\n\
        function probe(\n\
          value = (match (flag) { true => 1, false => 2 })\n\
            |> ((input: number) => match (flag) { true => input, false => 0 })\n\
        ) { return value; }\n";
    let diagnostics = ttc::analyze(parameter, &Options::default());
    assert_eq!(
        diagnostics.first().map(|diagnostic| diagnostic.code),
        Some(ttc::DiagnosticCode::MatchPlacement),
        "{diagnostics:#?}"
    );
}

#[test]
fn semicolon_free_arrow_does_not_own_the_following_try() {
    let source = "type R<T> = { kind: \"Ok\"; value: T } | { kind: \"Err\"; error: string };\n\
        declare const flag: boolean; declare function load(): R<number>;\n\
        function* probe() {\n\
          const choose = () => flag ? 1 : 2\n\
          try load();\n\
          yield choose();\n\
        }\n";
    let diagnostics = ttc::analyze(source, &Options::default());
    assert_eq!(
        diagnostics.first().map(|diagnostic| diagnostic.code),
        Some(DiagnosticCode::TryPlacement),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics[0].message.contains("constructor or generator"),
        "{diagnostics:#?}"
    );
}

#[test]
fn match_arm_return_try_propagates_from_the_concise_arrow() {
    let source = "variant R { Ok(value: number), Err(error: string) }\n\
         declare const g: () => R;\n\
         const f = (b: boolean): R => match (b) {\n\
           true => { return try g(); }, false => R.Ok(0),\n\
         };\n";
    let diagnostics = ttc::analyze(source, &Options::default());
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let output = ok(source);
    assert!(
        compact(&output).contains("if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
        "{output}"
    );
}

#[test]
fn duplicate_variant_cases_do_not_duplicate_the_semantic_alphabet() {
    let diagnostics = ttc::analyze(
        "variant State { Ready, Wait, Wait }\nconst f = (s: State) => match (s) { Ready => 1 };\n",
        &Options::default(),
    );
    let missing = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == ttc::DiagnosticCode::MatchNotExhaustive)
        .expect("the match remains non-exhaustive");
    assert!(missing.message.contains("missing \"Wait\""), "{missing:?}");
    assert!(
        !missing.message.contains("\"Wait\", \"Wait\""),
        "{missing:?}"
    );
    let replacement = missing.suggestions[0]
        .edit
        .as_ref()
        .expect("the missing-arm suggestion has an edit")
        .replacement
        .as_str();
    assert_eq!(replacement.matches("Wait =>").count(), 1, "{missing:?}");
}

#[test]
fn ttx_rewrites_ttx_imports_for_each_target_surface() {
    let source = "import { View } from \"./view.ttx\";\nexport { View };\n";
    let js = ok_tsx(source);
    assert!(js.contains("from \"./view.jsx\""), "{js}");
    let ts = compile(
        source,
        &Options {
            source_kind: SourceKind::Tsx,
            rewrite_imports: ttc::ImportRewrite::Ts,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(ts.contains("from \"./view.tsx\""), "{ts}");
}

/* ------------------------------------------------------------------ */
/* variant                                                                */
/* ------------------------------------------------------------------ */

#[test]
fn malformed_unit_variant_is_a_variant_diagnostic() {
    let src = "variant Status { Active = 1 }\n";
    let e = err(src);
    assert_eq!(
        ttc::analyze(src, &Options::default())[0].code,
        ttc::DiagnosticCode::MalformedVariant
    );
    assert!(
        e.message.contains("tt `variant` could not be parsed"),
        "{e}"
    );
    assert_eq!((e.line, e.col), (1, 1));
}

#[test]
fn variant_invalid_field_type_passes_without_verify() {
    // Without swc validation the construct still parses; the broken type is
    // carried into the output (where tsc would catch it).
    let opts = Options {
        verify: false,
        ..Options::default()
    };
    let out = compile("variant X {\n  A(f: number number),\n}\n", &opts).unwrap();
    assert!(out.contains("f: number number"));
}

/* ------------------------------------------------------------------ */
/* match                                                               */
/* ------------------------------------------------------------------ */

#[test]
fn expression_only_match_owners_are_rejected_without_a_closure_fallback() {
    let source = "variant E { A(value: number), B }\n\
         function f(seed: number, value = match (E.A(seed)) { A(value) => value, B => 0 }) { return value; }\n\
         class C { value = match (E.A(2)) { A(value) => value, B => 0 }; }\n";
    let diagnostics = ttc::analyze(source, &Options::default());
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code == ttc::DiagnosticCode::MatchPlacement),
        "{diagnostics:#?}"
    );
    assert!(
        ttc::compile_report(source, &Options::default())
            .emit
            .is_none()
    );
}

#[test]
fn is_patterns_require_open_hierarchy_and_binding_rules() {
    let src = "const a = match (x) { is Error { } => 1 };\n\
        const b = match (x) { is A { value } | is B => value, _ => 0 };\n";
    let diagnostics = ttc::analyze(src, &Options::default());
    let codes: Vec<_> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(
        codes,
        [
            ttc::DiagnosticCode::MatchIsWildcardRequired,
            ttc::DiagnosticCode::MatchIsEmptyBindings,
            ttc::DiagnosticCode::MatchIsOrBindings,
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn is_call_syntax_points_to_property_pattern_braces() {
    let diagnostics = ttc::analyze(
        "const value = match (x) { is SyntaxError(message) => message, _ => \"\" };\n",
        &Options::default(),
    );
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].code, ttc::DiagnosticCode::MalformedMatch);
    assert!(
        diagnostics[0]
            .suggestions
            .iter()
            .any(|suggestion| suggestion.message.contains("is Type { field }")),
        "{diagnostics:#?}"
    );
}

#[test]
fn is_constructor_identity_crosses_or_and_binding_wrappers() {
    let src = "const value = match (x) {\n\
        is ns.Error | is TypeError => 1,\n\
        is ns.Error { message } => message,\n\
        _ => 0,\n\
    };\n";
    let diagnostics = ttc::analyze(src, &Options::default());
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].code, ttc::DiagnosticCode::MatchDuplicateArm);
}
