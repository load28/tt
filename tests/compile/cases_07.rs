#[test]
fn if_let_duplicate_binding_is_an_error() {
    let e = err("function f() {\n  if let Both(a: v, b: v) = o { g(v); }\n}\n");
    assert!(
        e.message.contains("binding `v` is used more than once"),
        "{}",
        e.message
    );
}

#[test]
fn plain_if_statements_pass_through() {
    let src = "if (x) { a(); } else if (y) { b(); } else { c(); }\nconst z = cond ? 1 : 2;\n";
    assert_eq!(ok(src), src);
}

/* ------------------------------------------------------------------ */
/* result computation block                                            */
/* ------------------------------------------------------------------ */

#[test]
fn statement_bodied_result_returns_a_propagated_value() {
    let out = ok("const value = result { return try read(); };\n");
    assert!(out.contains("const $tt_t0 = read();"), "{out}");
    assert!(
        compact(&out).contains("if (!(\"value\" in $tt_t0)) { $tt_v0 = $tt_t0; break $tt_v0; }"),
        "{out}"
    );
    assert!(
        compact(&out)
            .contains("$tt_v0 = { kind: \"Ok\" as const, value: $tt_t0.value }; break $tt_v0;"),
        "{out}"
    );
}

#[test]
fn statement_bodied_result_declaration_try_stays_in_the_result_scope() {
    let out = ok("const value = result { const item = try read(); return item; };\n");
    assert!(out.contains("const $tt_t0 = read();"), "{out}");
    assert!(
        compact(&out).contains("if (!(\"value\" in $tt_t0)) { $tt_v0 = $tt_t0; break $tt_v0; }"),
        "{out}"
    );
    assert!(out.contains("const item = $tt_t0.value;"), "{out}");
    assert!(
        compact(&out).contains("$tt_v0 = { kind: \"Ok\" as const, value: item }; break $tt_v0;"),
        "{out}"
    );
}

#[test]
fn result_exits_target_the_generated_boundary_through_breakable_statements() {
    let source = r#"
const fromFor = result { for (const item of items) { return try read(item); } return 0; };
const fromWhile = result { while (ready()) { return try read(); } return 0; };
const fromDo = result { do { return try read(); } while (ready()); return 0; };
const fromSwitch = result { switch (tag) { default: return try read(); } return 0; };
"#;
    let out = ok(source);
    for slot in ["$tt_v0", "$tt_v1", "$tt_v2", "$tt_v3"] {
        assert!(out.contains(&format!("{slot}: {{")), "{slot}\n{out}");
        assert!(
            out.matches(&format!("break {slot};")).count() >= 2,
            "{slot}\n{out}"
        );
    }
}

#[test]
fn result_exit_label_avoids_user_identifiers_and_labels() {
    let out = ok(
        "function run() { $tt_v0: while (ready()) { break $tt_v0; } const $tt_v1 = 0; const value = result { const item = try read(); return item; }; return value; }\n",
    );
    assert!(out.contains("$tt_v0_1: {"), "{out}");
    assert!(out.contains("break $tt_v0_1;"), "{out}");
}

#[test]
fn result_preserves_a_statement_position_match_dispatch() {
    let out = ok(
        "const value = result { const item = try read(); match (item) { 1 => useOne(), _ => useOther() }; return item; };\n",
    );
    assert!(out.contains("let $tt_v1;"), "{out}");
    assert!(out.contains("switch ($tt_m)"), "{out}");
    assert!(out.contains("useOne()"), "{out}");
    assert!(out.contains("useOther()"), "{out}");
    assert!(!out.contains("$tt_recovery"), "{out}");
}

#[test]
fn ordinary_result_success_returns_from_expression_boundaries() {
    let out = ok(
        r#"class Box { field = result { const value = try read(); return value; }; }
class SwitchBox { field = result { const value = try read(); switch (value) { case 0: return 0; default: return value; } }; }
function withDefault(value = result { const item = try read(); return item; }) { return value; }
function* values() { yield result { const item = try read(); return item; }; }
const text = `value=${result { const item = try read(); return item; }}`;
"#,
    );
    assert!(out.contains("field = $tt_expr(() =>"), "{out}");
    assert!(out.contains("return { kind: \"Ok\" as const"), "{out}");
    assert!(out.contains("yield $tt_v"), "{out}");
    assert!(out.contains("const text = `value=${$tt_v"), "{out}");
    assert!(!out.contains("value: undefined"), "{out}");
}

#[test]
fn propagation_uses_a_type_clean_structural_result_discriminator() {
    let out =
        ok("function run() { const value = try Result.Err(\"boom\"); return Result.Ok(value); }\n");
    assert!(
        compact(&out).contains("if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
        "{out}"
    );
    assert!(!out.contains(".kind !== \"Ok\""), "{out}");
}

#[test]
fn statement_bodied_result_requires_a_success_return() {
    let error = err("const value = result { const item = try read(); use(item); };\n");
    assert!(error.message.contains("without a success value"), "{error}");
}

#[test]
fn statement_bodied_result_requires_success_on_every_reachable_path() {
    let error = err(
        "const value = result { const item = try read(); if (item) return item; log(item); };\n",
    );
    assert_eq!(
        error.message,
        "`result` can reach the end of its body without a success value"
    );
}

#[test]
fn statement_bodied_result_accepts_branch_complete_success() {
    let out = ok(
        "const value = result { const item = try read(); if (item) return item; else return 0; };\n",
    );
    assert!(out.contains("kind: \"Ok\" as const"), "{out}");
}

#[test]
fn discarded_result_suppresses_the_redundant_missing_success_diagnostic() {
    let diagnostics = ttc::analyze(
        "result { const item = try read(); use(item); };\n",
        &Options::default(),
    );
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        diagnostics[0].code,
        ttc::DiagnosticCode::ResultValueDiscarded
    );
}

#[test]
fn result_rejects_control_transfers_to_an_outer_region() {
    let cases = [
        (
            "function run() { while (ready()) { const value = result { const item = try read(); break; return item; }; } }\n",
            ttc::DiagnosticCode::ResultBreakCrossing,
            "break",
        ),
        (
            "function run() { while (ready()) { const value = result { const item = try read(); continue; return item; }; } }\n",
            ttc::DiagnosticCode::ResultContinueCrossing,
            "continue",
        ),
        (
            "function* run() { const value = result { const item = try read(); yield item; return item; }; }\n",
            ttc::DiagnosticCode::ResultYieldCrossing,
            "yield",
        ),
        (
            "function* run() { const value = result { const item = try read(); const sent = yield item; return sent; }; }\n",
            ttc::DiagnosticCode::ResultYieldCrossing,
            "yield",
        ),
        (
            "function run() { outer: while (ready()) { const value = result { const item = try read(); break outer; return item; }; } }\n",
            ttc::DiagnosticCode::ResultLabelCrossing,
            "break",
        ),
    ];
    for (source, code, keyword) in cases {
        let diagnostics = ttc::analyze(source, &Options::default());
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code == code),
            "{diagnostics:#?}"
        );
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .expect("named diagnostic");
        assert_eq!(diagnostic.start, Some(source.find(keyword).unwrap()));
    }
}

#[test]
fn result_keeps_control_transfers_owned_by_its_own_loop() {
    let out = ok(
        "const value = result { const item = try read(); while (item) { break; } return item; };\n",
    );
    assert!(out.contains("while (item) { break; }"), "{out}");
}

#[test]
fn result_allows_let_else_when_each_else_path_completes_the_result() {
    let out = ok(
        "variant Item { Some(value: number), None }\nconst value = result { const item = try read(); let Some(found) = item else { return 0; }; return found; };\n",
    );
    assert!(out.contains("value: 0"), "{out}");
    assert!(out.contains("value: found"), "{out}");
}

#[test]
fn result_wraps_inline_if_let_returns_as_success() {
    let out = ok(
        "variant Item { Some(value: number), None }\nconst value = result { const item = try read(); if let Some(found) = item { return found; } else { return 0; } };\n",
    );
    assert!(out.contains("value: found"), "{out}");
    assert!(out.contains("value: 0"), "{out}");
}

#[test]
fn discarded_statement_bodied_result_is_a_named_diagnostic() {
    let error = err("result { const item = try read(); return item; };\n");
    assert!(error.message.contains("would be discarded"), "{error}");
}

#[test]
fn result_return_expression_propagates_to_the_result_scope() {
    let out = ok("const value = result { return Math.round(try total() * 1.1); };\n");
    assert!(out.contains("const $tt_t0 = total();"), "{out}");
    assert!(out.contains("Math.round($tt_t0.value * 1.1)"), "{out}");
    assert!(out.contains("kind: \"Ok\" as const"), "{out}");
}

#[test]
fn nested_function_try_inside_a_result_preserves_its_own_function_boundary() {
    let out = ok(
        "const value = result { const inner = () => { return try step(); }; return try inner(); };\n",
    );
    assert!(out.contains("const inner = () =>"), "{out}");
    assert!(out.contains("return $tt_t0;"), "{out}");
}

#[test]
fn result_tail_is_an_ordinary_semicolon_terminated_statement() {
    let diagnostics = ttc::analyze(
        "const value = result { const item = try read(); log(item); };\n",
        &Options::default(),
    );
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        diagnostics[0].code,
        ttc::DiagnosticCode::ResultNoSuccessValue
    );
}

#[test]
fn generic_look_alike_with_adjacent_operators_passes_through() {
    let source =
        "declare const result: number;\nfunction f() {\n  result\n  { let x: Foo<-1>; }\n}\n";
    assert_eq!(ok(source), source);
}

/* ------------------------------------------------------------------ */
/* literal match patterns                                             */
/* ------------------------------------------------------------------ */

#[test]
fn literal_string_match_switches_on_the_scrutinee_itself() {
    let out = ok(r#"
const label = match (dir) {
  "north" => "N",
  "south" => "S",
  _ => "?",
};
"#);
    assert!(out.contains("const $tt_m = dir;"));
    assert!(out.contains("switch ($tt_m) {"));
    assert!(!out.contains("$tt_m.kind"));
    let compact = compact(&out);
    assert!(compact.contains(r#"case "north": { $tt_v0 = "N"; break; }"#));
    assert!(compact.contains(r#"case "south": { $tt_v0 = "S"; break; }"#));
    assert!(compact.contains(r#"default: { $tt_v0 = "?"; break; }"#));
}

#[test]
fn literal_number_match_emits_number_cases() {
    let out = ok(r#"
const message = match (status) {
  200 => "ok",
  404 => "not found",
  500 => "error",
  _ => "unknown",
};
"#);
    assert!(out.contains("switch ($tt_m) {"));
    let compact = compact(&out);
    assert!(compact.contains(r#"case 200: { $tt_v0 = "ok"; break; }"#));
    assert!(compact.contains(r#"case 404: { $tt_v0 = "not found"; break; }"#));
    assert!(compact.contains(r#"case 500: { $tt_v0 = "error"; break; }"#));
}

#[test]
fn literal_boolean_match_emits_true_and_false_cases() {
    let out = ok("const v = match (flag) { true => 1, false => 0 };");
    assert!(out.contains("switch ($tt_m) {"));
    let compact = compact(&out);
    assert!(compact.contains("case true: { $tt_v0 = 1; break; }"));
    assert!(compact.contains("case false: { $tt_v0 = 0; break; }"));
}

#[test]
fn literal_or_pattern_shares_one_body_via_fallthrough() {
    let out = ok(r#"
const kind = match (code) {
  200 | 201 | 204 => "success",
  400 | 404 => "client error",
  _ => "unknown",
};
"#);
    let compact = compact(&out);
    assert!(compact.contains(r#"case 200: case 201: case 204: { $tt_v0 = "success"; break; }"#));
    assert!(compact.contains(r#"case 400: case 404: { $tt_v0 = "client error"; break; }"#));
    // one body per arm, never duplicated per alternative
    assert_eq!(out.matches(r#"$tt_v0 = "success""#).count(), 1);
}

#[test]
fn literal_match_keeps_the_number_spelling_of_the_source() {
    let out = ok("const v = match (x) { 0xff => 1, 1_000 => 2, 1.5e2 => 3, -1 => 4, _ => 0 };");
    assert!(out.contains("case 0xff:"));
    assert!(out.contains("case 1_000:"));
    assert!(out.contains("case 1.5e2:"));
    assert!(out.contains("case -1:"));
}

#[test]
fn literal_match_without_a_wildcard_gets_a_runtime_guard() {
    let out = ok(r#"const label = match (dir) { "a" => 1, "b" => 2 };"#);
    assert!(compact(&out).contains(
        r#"default: { throw new Error("tt match: unexpected literal " + JSON.stringify($tt_m)); }"#
    ));
}

#[test]
fn literal_match_evaluates_the_scrutinee_once() {
    let out = ok(r#"const v = match (getValue()) { "a" => foo(), _ => bar() };"#);
    assert_eq!(out.matches("getValue()").count(), 1);
    assert!(out.contains("const $tt_m = getValue();"));
}

#[test]
fn literal_match_block_bodies_break_out_of_the_switch() {
    let out = ok(r#"const v = match (s) { "a" => { return 1; }, _ => 0 };"#);
    assert!(!out.contains("(() =>"), "{out}");
    // The `switch` the region already generates is the nearest `break`
    // target, so the rewritten `return` leaves through it and the region
    // needs no label of its own (TASK-160 §6).
    assert!(
        compact(&out).contains(r#"case "a": { $tt_v0 = 1; break; }"#),
        "{out}"
    );
    assert!(!out.contains("$tt_y_"), "{out}");
}

#[test]
fn a_block_arm_exit_inside_a_loop_still_needs_the_region_label() {
    // A `break` written inside the arm's own loop would be swallowed by
    // it, so this is the one shape that keeps the label.
    let out = ok(
        r#"const v = match (s) { "a" => { for (const x of xs) { return x; } return 0; }, _ => 0 };"#,
    );
    assert!(out.contains("$tt_y_v0: {"), "{out}");
    assert!(out.contains("break $tt_y_v0;"), "{out}");
}

#[test]
fn a_conditional_match_uses_one_exit_target() {
    let out = ok("const value = match (item) {\n\
         \x20 A if ready => { consume(); },\n\
         \x20 _ => 0,\n\
         };\n");
    assert!(out.contains("$tt_b: {"), "{out}");
    assert!(!out.contains("do {"), "{out}");
    assert!(out.contains("break $tt_b;"), "{out}");
}

#[test]
fn literal_match_with_a_guard_becomes_an_if_chain() {
    let out = ok("const v = match (code) { 200 if ok => 1, 200 => 2, _ => 3 };");
    assert!(!out.contains("switch ("));
    let compact = compact(&out);
    assert!(compact.contains("if ($tt_m === 200) { if (ok) { $tt_v0 = 1; break; } }"));
    assert!(compact.contains("if ($tt_m === 200) { $tt_v0 = 2; break; }"));
}

#[test]
fn literal_or_pattern_if_chain_tests_each_alternative() {
    let out = ok(r#"const v = match (s) { "a" | "b" if ok => 1, _ => 2 };"#);
    assert!(out.contains(r#"if ($tt_m === "a" || $tt_m === "b")"#));
}

#[test]
fn literal_match_without_a_wildcard_has_no_if_chain_case_guard() {
    let out = ok("const v = match (code) { 200 if ok => 1, 404 => 2 };");
    assert!(
        out.contains(
            r#"throw new Error("tt match: unexpected literal " + JSON.stringify($tt_m));"#
        )
    );
}

#[test]
fn literal_duplicate_arm_is_error() {
    let e = err(r#"const v = match (x) { "a" => 1, "a" => 2 };"#);
    assert!(e.message.contains(r#"duplicate arm "a""#), "{}", e.message);
    assert_eq!((e.line, e.col), (1, 33));
}

#[test]
fn literal_duplicate_across_or_alternatives_is_error() {
    let e = err(r#"const v = match (x) { "a" | "b" => 1, "b" | "c" => 2 };"#);
    assert!(e.message.contains(r#"duplicate arm "b""#), "{}", e.message);
    assert_eq!((e.line, e.col), (1, 39));
}

#[test]
fn literal_duplicate_compares_values_not_spellings() {
    // `200`, `0xc8` and `2e2` are one `switch` case — `===` says so.
    let e = err("const v = match (x) { 200 => 1, 0xc8 => 2 };");
    assert!(e.message.contains("duplicate arm 200"), "{}", e.message);
    let e = err(r#"const v = match (x) { "a" => 1, '\x61' => 2 };"#);
    assert!(e.message.contains(r#"duplicate arm "a""#), "{}", e.message);
    let e = err("const v = match (x) { true => 1, true => 2 };");
    assert!(e.message.contains("duplicate arm true"), "{}", e.message);
}

#[test]
fn literal_duplicate_is_allowed_between_guarded_arms() {
    // A guard may be false, so a guarded arm covers nothing — the same rule
    // tag patterns follow.
    let out = ok("const v = match (x) { 1 if a => 1, 1 if b => 2, 1 => 3, _ => 4 };");
    assert_eq!(out.matches("$tt_m === 1").count(), 3);
}

#[test]
fn literal_and_tag_patterns_cannot_be_mixed() {
    let e = err(r#"const v = match (x) { Some(v) => v, "none" => 0 };"#);
    assert!(
        e.message
            .contains("cannot mix tag patterns and literal patterns"),
        "{}",
        e.message
    );
    assert_eq!((e.line, e.col), (1, 37));
}

#[test]
fn literal_and_tag_patterns_cannot_be_mixed_in_either_order() {
    let e = err(r#"const v = match (x) { "none" => 0, Some(v) => v };"#);
    assert!(
        e.message
            .contains("cannot mix tag patterns and literal patterns"),
        "{}",
        e.message
    );
}

#[test]
fn literal_or_pattern_alternatives_must_share_a_kind() {
    let e = err(r#"const v = match (x) { "a" | 1 => 1, _ => 2 };"#);
    assert!(e.message.contains("same kind of literal"), "{}", e.message);
    assert_eq!((e.line, e.col), (1, 29));
}

#[test]
fn literal_match_wildcard_must_be_last() {
    let e = err(r#"const v = match (x) { _ => 0, "a" => 1 };"#);
    assert!(e.message.contains("must be the last arm"), "{}", e.message);
    assert_eq!((e.line, e.col), (1, 23));
}

#[test]
fn literal_match_is_not_checked_against_variants() {
    // A literal match carries no tags, so the tag exhaustiveness pass must
    // not adopt it — `Option`/`Result` are always in the candidate table.
    let out = ok(r#"const v = match (x) { "Some" => 1, "None" => 2 };"#);
    assert!(out.contains("switch ($tt_m) {"));
}

#[test]
fn literal_patterns_nest_inside_arm_bodies() {
    let out = ok(r#"
const v = match (a) {
  "x" => match (b) { 1 => "one", _ => "other" },
  _ => "none",
};
"#);
    assert_eq!(out.matches("switch ($tt_m) {").count(), 2);
}

#[test]
fn direct_return_literal_match_keeps_await_in_the_host_function() {
    let out = ok(r#"async function f() { return match (s) { "a" => await g(), _ => null }; }"#);
    assert!(!out.contains("async () =>"), "{out}");
    assert!(out.contains("$tt_v0 = await g();"), "{out}");
    assert!(out.contains("return $tt_v0;"), "{out}");
}

#[test]
fn tuple_patterns_do_not_accept_literals() {
    // v1 keeps literals out of tuple positions (design §18). The arrow arm
    // commits the construct to tt, then the tuple-pattern rule rejects it.
    let opts = Options {
        verify: false,
        ..Options::default()
    };
    let src = r#"const v = match (a, b) { ("x", 1) => 1, _ => 0 };"#;
    let error = compile(src, &opts).expect_err("tuple literals are malformed tt");
    assert!(error.message.contains("tt `match` could not be parsed"));
}

#[test]
fn a_block_of_literals_is_not_a_match() {
    // A call to a function named `match` followed by a block statement:
    // the arms have no `=>`, so the candidate is not claimed and the bytes
    // pass through.
    let src = "match (x)\n{ 1 }\n";
    assert_eq!(ok(src), src);
}

/* ------------------------------------------------------------------ */
/* val — binding modifier                                             */
/* ------------------------------------------------------------------ */

#[test]
fn plain_typescript_bindings_stay_mutable() {
    // The default is unchanged TypeScript semantics: without `val` every
    // mutation is legal and the bytes pass through.
    let src = "const x = { a: 1 };\nx.a = 2;\nlet y = { a: 1 };\ny.a = 2;\ny = { a: 3 };\n";
    assert_eq!(ok(src), src);
    let src = "function f(x: X) {\n  x.value = 1;\n}\n";
    assert_eq!(ok(src), src);
}

#[test]
fn val_declaration_modifier_is_erased_from_the_output() {
    assert_eq!(
        ok("val const user = getUser();\n"),
        "const user = getUser();\n"
    );
    assert_eq!(
        ok("val let state = getState();\n"),
        "let state = getState();\n"
    );
    assert_eq!(
        ok("export val const cfg = load();\n"),
        "export const cfg = load();\n"
    );
    // the keyword's trailing spaces go with it; a comment stays
    assert_eq!(ok("val\tconst a = 1;\n"), "const a = 1;\n");
    assert_eq!(ok("val /*c*/ const a = 1;\n"), "/*c*/ const a = 1;\n");
}

#[test]
fn val_parameter_modifier_is_erased_from_the_output() {
    assert_eq!(
        ok("function read(val user: User) {\n  return user.name;\n}\n"),
        "function read(user: User) {\n  return user.name;\n}\n",
    );
    assert_eq!(
        ok("const read = (val user: User) => user.name;\n"),
        "const read = (user: User) => user.name;\n",
    );
    assert_eq!(
        ok("function pick(a: A, val b: B, val { c }: C) {}\n"),
        "function pick(a: A, b: B, { c }: C) {}\n",
    );
}

#[test]
fn val_const_forbids_property_assignment() {
    let e = err("val const x = { a: 1 };\nx.a = 2;\n");
    assert_eq!((e.line, e.col), (2, 1));
    assert!(e.message.contains("cannot mutate through val binding `x`"));
}

#[test]
fn val_const_forbids_mutation_at_any_depth() {
    let e = err("val const x = { nested: { a: 1 } };\nx.nested.a = 2;\n");
    assert_eq!((e.line, e.col), (2, 1));
    let e = err("val const s = { u: { p: { n: 0 } } };\ns.u.p.n += 1;\n");
    assert_eq!((e.line, e.col), (2, 1));
    assert!(e.message.contains("val binding `s`"));
}

#[test]
fn val_let_forbids_mutation_but_allows_rebinding() {
    let e = err("val let state = { count: 0 };\nstate.count++;\n");
    assert_eq!((e.line, e.col), (2, 1));
    assert!(e.message.contains("`state`"));
    let src = "val let state = { count: 0 };\nstate = { ...state, count: state.count + 1 };\n";
    assert_eq!(ok(src), src.replacen("val ", "", 1));
}

#[test]
fn val_forbids_every_mutating_operator() {
    for stmt in [
        "x.a = 1;",
        "x.a += 1;",
        "x.a -= 1;",
        "x.a *= 1;",
        "x.a /= 1;",
        "x.a **= 2;",
        "x.a ||= 1;",
        "x.a &&= 1;",
        "x.a ??= 1;",
        "x.a >>= 1;",
        "x.a >>>= 1;",
        "x.a++;",
        "x.a--;",
        "++x.a;",
        "--x.a;",
        "delete x.a;",
        "x[0] = 1;",
        "x[0] += 1;",
        "x[0]++;",
        "delete x[0];",
        "x!.a = 1;",
        "x?.a.b = 1;",
    ] {
        let src = format!("val const x = load();\n{stmt}\n");
        let e = compile(&src, &Options::default())
            .err()
            .unwrap_or_else(|| panic!("expected `{stmt}` to be rejected"));
        assert!(
            e.message.contains("val binding `x`"),
            "{stmt}: {}",
            e.message
        );
    }
}

#[test]
fn val_leaves_reads_and_comparisons_alone() {
    // Nothing here mutates `x`, and none of these operators may be
    // mistaken for an assignment.
    let src = "val const x = load();\nconst r = [x.a == 1, x.a === 1, x.a != 1, x.a >= 1, x.a <= 1, x.a && 1, x.a || 1, x.a ?? 1, x.a + 1, x.a > 1];\nconst y = x.a;\nconst z = { ...x };\n";
    assert_eq!(ok(src), src.replacen("val ", "", 1));
}
