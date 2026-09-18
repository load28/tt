/* ------------------------------------------------------------------ */
/* TASK-377 compiler boundary and lowering defects                     */
/* ------------------------------------------------------------------ */

#[test]
fn a_byte_order_mark_passes_through_ahead_of_lowered_constructs() {
    let out = ok("\u{feff}variant O { Some(value: number), None }\nexport const f = (o: O) => match (o) { Some(value) => value, None => 0 };\n");
    assert!(out.starts_with('\u{feff}'), "{out}");
    assert!(out.contains("switch ($tt_m.kind)"), "{out}");
}

#[test]
fn a_crlf_source_gets_crlf_glue() {
    let out = ok("variant O { Some(value: number), None }\r\nexport const f = (o: O) => match (o) {\r\n  Some(value) => value,\r\n  None => 0,\r\n};\r\n");
    for line in out.split_inclusive('\n') {
        assert!(line.ends_with("\r\n"), "line without CRLF: {line:?}\n{out}");
    }
}

#[test]
fn a_concise_arrow_body_lays_its_glue_out_like_a_block_body() {
    let out = ok("variant O { Some(value: number), None }\nexport const g = (o: O) => ({ v: match (o) { Some(value) => value, None => 0 } });\n");
    assert!(out.contains("=> {\n  let $tt_v0"), "{out}");
    assert!(out.contains(";\n  {\n    const $tt_m = o;"), "{out}");
    assert!(out.contains("\n  return ({ v: $tt_v0 });\n};"), "{out}");
}

#[test]
fn sibling_values_in_one_concise_arrow_start_on_their_own_lines() {
    let out = ok("variant O { Some(value: number), None }\nexport const j = (o: O) => [match (o) { Some(value) => value, None => 0 }, match (o) { Some(value) => value, None => 1 }];\n");
    assert!(!out.contains("}{"), "{out}");
    assert!(out.contains("  }\n  {\n    const $tt_m = o;"), "{out}");
}

#[test]
fn a_match_whose_subject_is_a_match_keeps_both_subjects_apart() {
    let out = ok("variant S { A(v: number), B(w: number), C }\nconst mm = match (match (S.A(1)) { A(v) => S.B(v), _ => S.C }) { B(w) => \"b\" + w, _ => \"x\" };\n");
    assert!(out.contains("let $tt_m_1"), "{out}");
    assert!(out.contains("; {\n    const $tt_m = S.A(1);"), "{out}");
    assert!(out.contains("$tt_m_1 = S.B(v);"), "{out}");
    assert!(out.contains("switch ($tt_m_1.kind)"), "{out}");
}

#[test]
fn a_tuple_match_over_matches_names_each_nested_subject() {
    let out = ok("variant S { A(v: number), B(w: number), C }\ndeclare const s: S;\ndeclare const t: S;\nconst mm = match (match (s) { A(v) => S.B(v), _ => S.C }, match (t) { A(v) => S.B(v), _ => S.C }) { (B(w), B) => \"b\" + w, _ => \"x\" };\n");
    assert!(out.contains("let $tt_m0_1"), "{out}");
    assert!(out.contains("let $tt_m1_1"), "{out}");
    assert!(out.contains("$tt_m0_1.kind === \"B\" && $tt_m1_1.kind === \"B\""), "{out}");
}

#[test]
fn a_match_in_a_guard_is_lowered_before_the_guard_test() {
    let out = ok("variant S { A(v: number), B(w: number), C }\ndeclare const s: S;\nconst x = match (s) { A(v) if match (s) { B(w) => w > 0, _ => false } => 1, _ => 0 };\n");
    assert!(out.contains("const { v } = $tt_m;\n      let $tt_v1"), "{out}");
    assert!(out.contains(";\n      {\n        const $tt_m = s;"), "{out}");
    assert!(out.contains("if ($tt_v1) {"), "{out}");
}

#[test]
fn a_parenthesized_match_in_a_guard_keeps_its_parentheses() {
    let out = ok("variant S { A(v: number), B(w: number), C }\ndeclare const s: S;\nconst y = match (s) { A(v) if (match (s) { B(w) => w > 0, _ => false }) => 1, _ => 0 };\n");
    assert!(out.contains("if (($tt_v1)) {"), "{out}");
}


#[test]
fn a_declaration_try_binds_to_the_primary_expression_like_the_value_form() {
    let out = ok("declare function total(): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\nfunction g() { const x = try total() * 1.1; return x; }\n");
    assert!(out.contains("const $tt_t0 = total();"), "{out}");
    assert!(out.contains("const x = $tt_v0 * 1.1;"), "{out}");
    assert!(!out.contains("total() * 1.1;\n"), "{out}");
}

#[test]
fn two_tries_in_one_initializer_both_propagate() {
    let out = ok("declare function a(): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\nfunction g() { const x = try a() + try a(); return x; }\n");
    assert!(out.contains("const x = $tt_v0 + $tt_v1;"), "{out}");
}

#[test]
fn a_whole_primary_initializer_is_still_the_statement_form() {
    let out = ok("declare function a(): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\nfunction g() { const x = try a(); const y = try (a()); return x + y; }\n");
    assert!(out.contains("const x = $tt_t0.value;"), "{out}");
    assert!(out.contains("const y = $tt_t1.value;"), "{out}");
}

#[test]
fn a_result_return_without_a_semicolon_completes_the_block() {
    let out = ok("declare function read(): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\nexport function f() {\n  const r = result {\n    const a = try read();\n    return a\n  };\n  return r;\n}\n");
    assert!(out.contains("$tt_v0 = { kind: \"Ok\" as const, value: a };"), "{out}");
}

#[test]
fn a_result_tail_expression_without_a_semicolon_reports_only_the_missing_value() {
    let diagnostics = ttc::analyze(
        "declare function read(): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\nexport function f() {\n  const r = result { const a = try read(); a };\n  return r;\n}\n",
        &Options::default(),
    );
    let codes: Vec<_> = diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(codes, [DiagnosticCode::ResultNoSuccessValue], "{diagnostics:#?}");
}


#[test]
fn a_try_operand_may_be_an_awaited_primary() {
    let out = ok("declare function a(): Promise<{ kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string }>;\nasync function g() { const x = try await a(); const y = try await a() * 2; return x + y; }\n");
    assert!(out.contains("const $tt_t0 = await a();"), "{out}");
    assert!(out.contains("const x = $tt_t0.value;"), "{out}");
    assert!(out.contains("const y = $tt_v0 * 2;"), "{out}");
}

#[test]
fn a_declaration_try_without_a_semicolon_is_the_value_form_under_asi() {
    let out = ok("declare function g(): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\nfunction f() {\n  const n = try g()\n  return n;\n}\n");
    assert!(out.contains("const n = $tt_v0\n  return n;"), "{out}");
}
