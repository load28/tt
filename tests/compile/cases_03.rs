#[test]
fn try_without_semicolon_is_not_recognized() {
    // No terminating `;` → not tt syntax; the (invalid-TS) source passes
    // through and the output self-check reports it.
    let e = err("function f(): X {\n  const n = try g()\n  return h(n);\n}\n");
    // The `try` that did not parse is the thing to look at, and the
    // message says why the output no longer parses.
    assert!(
        e.message.contains("`try` here did not parse as a tt `try`"),
        "{}",
        e.message
    );
    assert_eq!((e.line, e.col), (2, 13));
    assert_eq!((e.end_line, e.end_col), (2, 16));
}

#[test]
fn try_inside_match_arm_is_an_error() {
    // Directly in an arm's statement stream the emitted `return` would
    // exit the switch IIFE — the match would *evaluate to* the `Err`
    // instead of propagating it.
    let e = err(
        "const x = match (r) {\n  Ok(value) => { const y = try f(value); return y; },\n  Err(error) => fallback(error),\n};\n",
    );
    assert!(
        e.message
            .contains("`try` cannot be used here, in an isolated value region"),
        "{}",
        e.message
    );
    // The propagation, not the declaration it is written in.
    assert_eq!((e.line, e.col), (2, 28));
    assert_eq!((e.end_line, e.end_col), (2, 40));
}

#[test]
fn try_at_module_top_level_is_an_error() {
    // The lowering's `return` would have no function to exit — before the
    // flow answer this fell through to the output self-check's "invalid
    // TypeScript or a ttc bug" backstop.
    let e = err("function f(): void {}\ntry g();\n");
    assert!(
        e.message.contains("`try` must be inside a function"),
        "{}",
        e.message
    );
    assert_eq!((e.line, e.col), (2, 1));

    let e = err("const x = try g();\n");
    assert!(
        e.message.contains("`try` must be inside a function"),
        "{}",
        e.message
    );

    // A namespace body is not a function body either.
    let e = err("namespace N {\n  try g();\n}\n");
    assert!(
        e.message.contains("`try` must be inside a function"),
        "{}",
        e.message
    );
}

#[test]
fn try_is_a_value_in_deep_expression_positions() {
    let out = ok(
        "function f(): TResult<number, string> {\n  return Result.Ok(Math.round(try total() * 1.1));\n}\n",
    );
    assert!(out.contains("const $tt_t0 = total();"), "{out}");
    assert!(out.contains("$tt_v0 * 1.1"), "{out}");

    let out = ok(
        "function f(flag: boolean): TResult<number, string> { const value = try (flag ? left() : right()); return Result.Ok(value); }\n",
    );
    assert!(
        out.contains("const $tt_t0 = (flag ? left() : right());"),
        "{out}"
    );

    let out =
        ok("function f(): TResult<string, string> { return Result.Ok(`v=${try read()}`); }\n");
    assert!(out.contains("`v=${$tt_v0}`"), "{out}");

    let out = ok(
        "function f(): TResult<{ amount: number }, string> { return Result.Ok({ amount: try total() }); }\n",
    );
    assert!(out.contains("{ amount: $tt_v0 }"), "{out}");

    let out = ok(
        "function f(r: R): TResult<number, string> { return match (r) { A => try total(), B => Result.Ok(0) }; }\n",
    );
    assert!(
        compact(&out).contains("if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
        "{out}"
    );
}

#[test]
fn try_preserves_argument_and_conditional_evaluation_order() {
    let out = ok(
        "function f(flag: boolean): TResult<number, string> {\n  return Result.Ok(call(first(), flag && try second(), third()));\n}\n",
    );
    let first = out.find("first()").unwrap();
    let propagation = out.find("const $tt_t0 = second();").unwrap();
    let third = out.find("third()").unwrap();
    assert!(first < propagation && propagation < third, "{out}");
    assert!(out[..propagation].contains("if ("), "{out}");

    let out = ok(
        "function f(maybe: any): TResult<number, string> { return Result.Ok(maybe?.(first(), try second(), third())); }\n",
    );
    let guard = out.find("!= null").unwrap();
    let propagation = out.find("const $tt_t0 = second();").unwrap();
    assert!(guard < propagation, "{out}");
}

#[test]
fn try_turns_a_concise_arrow_into_a_propagating_block() {
    let out = ok("const f = (): TResult<number, string> => Result.Ok(try read());\n");
    assert!(out.contains("=> {"), "{out}");
    assert!(
        compact(&out).contains("if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
        "{out}"
    );
}

#[test]
fn parenthesized_concise_arrow_keeps_try_in_the_arrow() {
    let parenthesized = ok("const f = () => (try next());\n");
    assert!(parenthesized.contains("=> {"), "{parenthesized}");
    assert!(
        compact(&parenthesized).contains("if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
        "{parenthesized}"
    );
}

#[test]
fn pipeline_concise_arrow_keeps_try_in_the_arrow() {
    let pipeline = ok("const f = value |> (x => try next());\n");
    assert!(pipeline.contains("=> {"), "{pipeline}");
    assert!(
        compact(&pipeline).contains("if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
        "{pipeline}"
    );
}

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
fn try_in_spread_operands_enters_the_evaluation_protocol() {
    for src in [
        "function f() { const value = { ...try read() }; }\n",
        "function f() { const value = [ ...try read() ]; }\n",
    ] {
        let output = ok(src);
        assert!(
            compact(&output).contains("if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
            "{output}"
        );
        assert!(output.contains("const value ="), "{output}");
    }
}

#[test]
fn match_in_spread_operands_enters_the_evaluation_protocol() {
    for src in [
        "const value = { ...match (kind) { A => ({ a: 1 }), _ => ({}) } };\n",
        "const value = [ ...match (kind) { A => [1], _ => [] } ];\n",
        "consume(...match (kind) { A => [1], _ => [] });\n",
    ] {
        let output = ok(src);
        assert!(output.contains("...($tt_v"), "{output}");
        assert!(!output.contains("...match"), "{output}");
    }
}

#[test]
fn typescript_members_and_properties_named_try_are_passthrough() {
    let source = "const object = { try: 1 };\nobject.try();\n";
    assert_eq!(ok(source), source);
}

#[test]
fn try_placement_reports_the_owning_reason() {
    let cases = [
        (
            "function f() { for (let i = 0; i < 1; try advance()) {} }\n",
            "repeated loop position",
        ),
        (
            "function f(value = try read()) { return value; }\n",
            "parameter initializer",
        ),
        (
            "class C { static { const value = { item: try read() }; } }\n",
            "class static block",
        ),
        ("class C { constructor() { try read(); } }\n", "constructor"),
        (
            "const value = match (source) { Ok(value) => { const item = try read(); return item; }, Err(error) => error };\n",
            "isolated value region",
        ),
    ];
    for (source, reason) in cases {
        let error = err(source);
        assert!(error.message.contains(reason), "{source}\n{error:#?}");
        assert_eq!(error.line, 1, "{source}\n{error:#?}");
        assert_eq!(
            error.col,
            source.find("try").unwrap() + 1,
            "{source}\n{error:#?}"
        );
    }
}

#[test]
fn a_static_block_does_not_capture_a_nested_function_try() {
    let output = ok("class C { static { const run = () => { try read(); }; } }\n");
    assert!(
        compact(&output).contains("if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
        "{output}"
    );
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
        LoweringPlan,
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
            Expected::LoweringPlan,
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
            Expected::LoweringPlan => {
                assert!(
                    diagnostics.iter().any(|diagnostic| {
                        diagnostic.code == ttc::DiagnosticCode::LoweringPlanFailed
                    }),
                    "{source}\n{diagnostics:#?}"
                );
            }
        }
    }
}

#[test]
fn try_inside_a_function_inside_a_scrutinee_is_allowed() {
    let out = ok(
        "const x = match (run(() => { try g(); return h(); })) {\n  Ok(value) => value,\n  Err(error) => 0,\n};\n",
    );
    assert!(
        compact(&out).contains("if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
        "{out}"
    );
}

#[test]
fn try_inside_a_function_inside_an_arm_body_is_allowed() {
    let out = ok(
        "const x = match (r) {\n  Ok(value) => { const f = () => { try g(value); return 1; }; return f(); },\n  Err(error) => 0,\n};\n",
    );
    assert!(
        compact(&out).contains("if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
        "{out}"
    );
}

#[test]
fn try_inside_a_function_inside_a_template_interpolation_is_allowed() {
    let out = ok("const s = `${run(() => { try g(); return h(); })}`;\n");
    assert!(
        compact(&out).contains("if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
        "{out}"
    );
}

/* ------------------------------------------------------------------ */
/* let-else — Rust-style refutable binding                             */
/* ------------------------------------------------------------------ */

#[test]
fn let_else_emits_guard_and_bind() {
    let out = ok(
        "function f(): number {\n  const Some(value) = find() else { return 0; };\n  return value;\n}\n",
    );
    assert!(
        compact(&out).contains(
            "const $tt_t0 = find(); if ($tt_t0.kind !== \"Some\") { return 0; } const { value } = $tt_t0;"
        ),
        "{out}"
    );
}

#[test]
fn let_else_binding_alias_and_keyword() {
    let out = ok(
        "function f(): string {\n  let Some(value: user) = find() else { throw new Error(\"none\"); };\n  return user;\n}\n",
    );
    assert!(out.contains("let { value: user } = $tt_t0;"), "{out}");
}

#[test]
fn let_else_empty_bindings_checks_only() {
    let out =
        ok("function f(): number {\n  const Ok() = check() else { return -1; };\n  return 1;\n}\n");
    assert!(
        compact(&out).contains("if ($tt_t0.kind !== \"Ok\") { return -1; }"),
        "{out}"
    );
    assert!(!out.contains("} = $tt_t0;"), "{out}");
}

#[test]
fn let_else_or_pattern_shares_one_destructuring() {
    // Rust's `let (A(x) | B(x)) = …` — the guard tests every alternative
    // and the destructuring is shared, written by the compiler (unmapped)
    // because it stands for every alternative at once, as in a match
    // or-arm.
    let out = ok(
        "variant E { A(x: number), B(x: number), C }\nfunction f(e: E): number {\n  const A(x) | B(x) = e else { return 0; };\n  return x;\n}\n",
    );
    assert!(
        compact(&out).contains(
            "if ($tt_t0.kind !== \"A\" && $tt_t0.kind !== \"B\") { return 0; } const { x } = $tt_t0;"
        ),
        "{out}"
    );
}

#[test]
fn let_else_or_pattern_with_a_bare_alternative_checks_only() {
    // A bare later alternative binds nothing, so the whole pattern must —
    // first alternative with empty parens, membership test only.
    let out = ok(
        "variant E { A(x: number), B(x: number), C }\nfunction f(e: E): number {\n  const A() | C = e else { return 0; };\n  return 1;\n}\n",
    );
    assert!(
        compact(&out).contains("if ($tt_t0.kind !== \"A\" && $tt_t0.kind !== \"C\") { return 0; }"),
        "{out}"
    );
    assert!(!out.contains("} = $tt_t0;"), "{out}");
}

#[test]
fn let_else_or_alternatives_must_bind_the_same_names() {
    let e = err(
        "variant E { A(x: number), B(y: number), C }\nfunction f(e: E): number {\n  const A(x) | B(y) = e else { return 0; };\n  return x;\n}\n",
    );
    assert!(
        e.message
            .contains("let-else: or-pattern alternatives must bind the same names"),
        "{}",
        e.message
    );
    // The mismatching alternative is the position.
    assert_eq!((e.line, e.col), (3, 16));
}

#[test]
fn if_let_or_pattern_condition_is_a_disjunction() {
    let out = ok(
        "variant E { A(x: number), B(x: number), C }\nfunction g(e: E): number {\n  if let A(x) | B(x) = e {\n    return x;\n  }\n  return -1;\n}\n",
    );
    assert!(
        compact(&out).contains(
            "if ($tt_t0.kind === \"A\" || $tt_t0.kind === \"B\") { const { x } = $tt_t0;"
        ),
        "{out}"
    );
}

#[test]
fn if_let_nested_patterns_cannot_combine_with_or() {
    let e = err(
        "function g(o: X): number {\n  if let Some(value: Ok(v)) | None() = o {\n    return 1;\n  }\n  return 0;\n}\n",
    );
    assert!(
        e.message
            .contains("if let: nested patterns cannot be combined with or-patterns"),
        "{}",
        e.message
    );
}

#[test]
fn inline_bodies_inherit_the_enclosing_functions_place() {
    // An `if let` body and a let-else `else` block are inline — their
    // statements run where the statement stands — so a `try` (or a
    // let-else) inside them exits the function the chain bottoms out in,
    // exactly as it would outside the construct.
    let out = ok(
        "variant E { A(x: number), B }\nfunction f(e: E): Result<number, string> {\n  if let A(x) = e {\n    const n = try g(x);\n    return Result.Ok(n);\n  }\n  return Result.Ok(0);\n}\n",
    );
    assert!(
        compact(&out).contains("if (!(\"value\" in $tt_t1)) { return $tt_t1; }"),
        "{out}"
    );

    let out = ok(
        "variant E { A(x: number), B }\nfunction f(e: E): number {\n  const Some(v) = find(e) else {\n    const B() = e else { throw new Error(\"a\"); };\n    return 0;\n  };\n  return v;\n}\n",
    );
    assert!(out.contains("if ($tt_t1.kind !== \"B\")"), "{out}");
}

#[test]
fn an_inline_chain_bottoming_out_in_an_iife_still_rejects_try() {
    // The same body inside a match arm: the chain bottoms out in the
    // arm's IIFE, so the emitted `return` would corrupt the match value.
    let e = err(
        "variant E { A(x: number), B }\nconst r = match (e) {\n  A(x) => { if let A(y) = f(x) { const n = try g(y); return n; } return 0; },\n  B => 0,\n};\n",
    );
    assert!(
        e.message.contains("`try` cannot be used here"),
        "{}",
        e.message
    );
}

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

#[test]
fn let_else_shares_try_temp_counter() {
    let out = ok(
        "function f(): X {\n  const n = try g();\n  const Some(v) = h(n) else { return fallback(); };\n  return wrap(v);\n}\n",
    );
    assert!(out.contains("if (!(\"value\" in $tt_t0))"), "{out}");
    assert!(
        compact(&out).contains("const $tt_t1 = h(n); if ($tt_t1.kind !== \"Some\""),
        "{out}"
    );
}

#[test]
fn let_else_diverges_via_throw_and_continue() {
    ok(
        "function f(): number {\n  const Some(v) = find() else { throw new Error(\"no\"); };\n  return v;\n}\n",
    );
    ok(
        "function f(): number {\n  for (const x of xs) {\n    const Some(v) = find(x) else { continue; };\n    use(v);\n  }\n  return 0;\n}\n",
    );
}

#[test]
fn let_else_expression_may_be_a_match() {
    let out = ok(
        "function f(): number {\n  const Some(v) = match (x) { A => some(1), _ => none() } else { return 0; };\n  return v;\n}\n",
    );
    assert!(out.contains("if ($tt_t0.kind !== \"Some\")"), "{out}");
    assert!(out.contains("switch ($tt_m.kind)"), "{out}");
}

#[test]
fn let_else_diverges_when_the_return_value_is_an_object_literal() {
    // The `}` of an object literal ends no statement, so `return { ... };`
    // is still a `return` — the shape every `Result`-returning function
    // writes.
    let out = ok(
        "function f(): number {\n  const Some(v) = find() else { return { kind: \"Err\", error: \"no\" }; };\n  return v;\n}\n",
    );
    assert!(
        compact(&out).contains("{ return { kind: \"Err\", error: \"no\" }; }"),
        "{out}"
    );
    // Same with a statement in front of it, and with a tt construct as
    // the returned value.
    ok(
        "function f(): number {\n  const Some(v) = find() else { log(\"x\"); return { k: 1 }; };\n  return v;\n}\n",
    );
    ok(
        "function f(): number {\n  const Some(v) = find() else { return match (o) { A => 1, _ => 0 }; };\n  return v;\n}\n",
    );
    ok(
        "function f(): number {\n  const Some(v) = find() else { throw { code: 1 }; };\n  return v;\n}\n",
    );
}
