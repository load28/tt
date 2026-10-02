#[test]
fn expression_try_reports_a_typescript_control_flow_boundary() {
    for src in [
        "function f() { while (try condition()) work(); }\n",
        "function f(value = try read()) { return value; }\n",
        "class C { value = try read(); }\n",
    ] {
        let diagnostics = ttc::analyze(src, &Options::default());
        assert_eq!(diagnostics.len(), 1, "{src}\n{diagnostics:#?}");
        assert_eq!(diagnostics[0].code, ttc::DiagnosticCode::TryPlacement);
        assert!(
            diagnostics[0]
                .message
                .contains("TypeScript control-flow boundary"),
            "{src}\n{:#?}",
            diagnostics[0]
        );
    }
}

#[test]
fn try_in_constructor_or_generator_is_a_placement_error() {
    for src in [
        "class C { constructor() { try read(); } }\n",
        "class C { constructor() { const value = try read(); } }\n",
        "function* values() { try read(); }\n",
        "function* values() { yield try read(); }\n",
        "async function* values() { try read(); }\n",
        "async function* values() { yield try read(); }\n",
    ] {
        let diagnostics = ttc::analyze(src, &Options::default());
        assert_eq!(diagnostics.len(), 1, "{src}\n{diagnostics:#?}");
        assert_eq!(diagnostics[0].code, ttc::DiagnosticCode::TryPlacement);
    }
}

#[test]
fn try_placement_claims_for_update_and_destructuring_edges() {
    for src in [
        "function f() { for (let i = 0; i < 1; try advance()) {} }\n",
        "function f() { const [value = try read()] = input; }\n",
    ] {
        let diagnostics = ttc::analyze(src, &Options::default());
        assert_eq!(diagnostics.len(), 1, "{src}\n{diagnostics:#?}");
        assert_eq!(diagnostics[0].code, ttc::DiagnosticCode::TryPlacement);
        assert!(
            !err(src).message.contains("did not parse as tt try"),
            "{src}"
        );
    }
}

#[test]
fn result_try_crossing_an_isolated_match_arm_is_a_placement_diagnostic() {
    let source = "const value = result {\n  const item = try read();\n  match (item) { Ok(value) => try next(value), Err(error) => error }\n  return item;\n};\n";
    let diagnostics = ttc::analyze(source, &Options::default());
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        diagnostics[0].code,
        ttc::DiagnosticCode::TryCrossesValueRegion
    );
    assert_eq!(diagnostics[0].start, Some(source.rfind("try").unwrap()));
    assert!(
        diagnostics[0]
            .suggestions
            .iter()
            .all(|suggestion| suggestion.edit.is_none()),
        "{diagnostics:#?}"
    );
}

#[test]
fn placement_matrix_prerequisite_gate() {
    enum Expected {
        Accepted,
        Placement,
    }

    let cases = [
        (
            "function f() { const value = try read(); use(value); }\n",
            Expected::Accepted,
        ),
        (
            "function f() { consume(try read()); new Box(try read()); sink?.(try read()); }\n",
            Expected::Accepted,
        ),
        (
            "function f() { const value = ready ? try read() : fallback(); return value; }\n",
            Expected::Accepted,
        ),
        (
            "function f() { for (let i = try count();;) { break; } }\n",
            Expected::Accepted,
        ),
        (
            "function f() { for (const value of try values()) { use(value); } switch (try tag()) { default: break; } }\n",
            Expected::Accepted,
        ),
        (
            "function f() { using resource = try acquire(); use(resource); }\n",
            Expected::Accepted,
        ),
        (
            "async function f() { await using resource = try acquire(); use(resource); }\n",
            Expected::Accepted,
        ),
        (
            "const f = value => try read(value);\nconst g = value => (try read(value));\nconst h = value |> (item => try read(item));\n",
            Expected::Accepted,
        ),
        (
            "const value = result { const item = try read(); return item; };\n",
            Expected::Accepted,
        ),
        ("try read();\n", Expected::Placement),
        (
            "function f(value = try read()) { return value; }\n",
            Expected::Placement,
        ),
        (
            "class C { field = try read(); static { const value = { item: try read() }; } }\n",
            Expected::Placement,
        ),
        (
            "class C { constructor() { try read(); } }\nfunction* values() { yield try read(); }\nasync function* asyncValues() { yield try read(); }\n",
            Expected::Placement,
        ),
        (
            "function f() { while (try ready()) {} }\n",
            Expected::Placement,
        ),
        (
            "function f() { for (; try ready(); ) {} }\n",
            Expected::Placement,
        ),
        (
            "function f() { for (let i = 0; i < 1; try advance()) {} }\n",
            Expected::Placement,
        ),
        (
            "function f() { switch (value) { case try read(): break; } const [item = try read()] = input; object?.[try read()]; }\n",
            Expected::Placement,
        ),
        (
            "const value = match (source) { Ok(value) => { const item = try read(); return item; }, Err(error) => error };\n",
            Expected::Placement,
        ),
    ];

    for (source, expected) in cases {
        let diagnostics = std::panic::catch_unwind(|| ttc::analyze(source, &Options::default()))
            .expect("every placement row must report without unwinding");
        assert!(
            !diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == ttc::DiagnosticCode::VerifyFailed),
            "{source}\n{diagnostics:#?}"
        );
        match expected {
            Expected::Accepted => {
                assert!(diagnostics.is_empty(), "{source}\n{diagnostics:#?}");
                let output = ok(source);
                assert!(!output.is_empty(), "{source}");
            }
            Expected::Placement => {
                assert!(
                    diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == ttc::DiagnosticCode::TryPlacement),
                    "{source}\n{diagnostics:#?}"
                );
                let try_at = source.find("try").expect("placement source contains try");
                assert!(
                    diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.start == Some(try_at)),
                    "{source}\n{diagnostics:#?}"
                );
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* let-else — Rust-style refutable binding                             */
/* ------------------------------------------------------------------ */

#[test]
fn a_module_level_inline_try_reports_the_cause_not_the_backstop() {
    // At the module's top level the chain bottoms out in the module: the
    // tt diagnostic is the cause, and the output self-check's failure on
    // the emitted `return` is its effect — reported once, not twice.
    let src = "variant E { A(x: number), B }\nif let A(x) = e {\n  try g(x);\n}\n";
    let report = ttc::compile_report(src, &Options::default());
    assert_eq!(report.diagnostics.len(), 1, "{:#?}", report.diagnostics);
    assert_eq!(
        report.diagnostics[0].code,
        ttc::DiagnosticCode::TryPlacement
    );
    assert!(
        report.diagnostics[0]
            .message
            .contains("`try` must be inside a function"),
        "{}",
        report.diagnostics[0].message
    );
    assert!(report.emit.is_none(), "the invalid emit is withheld");
}
