/* ------------------------------------------------------------------ */
/* TASK-379 completed ownership: guards, operands, ambient, statements */
/* ------------------------------------------------------------------ */

#[test]
fn a_match_under_a_conditional_operation_in_a_guard_keeps_the_short_circuit() {
    let out = ok("variant S { A(v: number), B(w: number), C }\ndeclare const s: S;\nconst x = match (s) { A(v) if v > 0 && match (s) { B(w) => w > 0, _ => false } => 1, _ => 0 };\n");
    assert!(out.contains("const $tt_v2 = (v > 0);"), "{out}");
    assert!(out.contains("if ($tt_v2) {"), "{out}");
    assert!(out.contains("$tt_v3 = $tt_v1;"), "{out}");
    assert!(out.contains("$tt_v3 = $tt_v2;"), "{out}");
    assert!(out.contains("if ($tt_v3) {"), "{out}");
}

#[test]
fn a_match_in_a_guard_test_with_a_pipeline_is_lowered_before_the_test() {
    let out = ok("variant S { A(v: number), B(w: number), C }\ndeclare const s: S;\nconst z = match (s) { A(v) if match (s) { B(w) => w > 0, _ => false } |> Boolean => 1, _ => 0 };\n");
    assert!(out.contains("if ($tt_v"), "{out}");
}

#[test]
fn a_match_inside_an_arm_body_call_keeps_argument_order() {
    let out = ok("variant S { A(v: number), B(w: number), C }\ndeclare const s: S;\ndeclare function eff(): number;\ndeclare function g(a: number, b: number): number;\nconst x = match (s) { A(v) => g(eff(), match (s) { B(w) => w, _ => 0 }), _ => 0 };\n");
    let effect = out.find("= (eff());").expect("eff is captured first");
    let inner = out.find("case \"B\"").expect("the inner match follows");
    assert!(effect < inner, "{out}");
    assert!(out.contains("$tt_v0 = $tt_v2($tt_v3, $tt_v1);"), "{out}");
}

#[test]
fn a_match_under_a_conditional_operation_in_an_arm_body_is_a_region() {
    let out = ok("variant S { A(v: number), B(w: number), C }\ndeclare const s: S;\ndeclare function eff(): number;\nconst y = match (s) { A(v) => eff() > 0 && match (s) { B(w) => w > 0, _ => false }, _ => false };\n");
    assert!(out.contains("const $tt_v2 = (eff() > 0);\n      if ($tt_v2) {"), "{out}");
    assert!(out.contains("$tt_v0 = $tt_v2 && $tt_v1;"), "{out}");
}

#[test]
fn parenthesized_sibling_tries_both_propagate() {
    let out = ok("declare function a(): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\nfunction g() { const x = (try a()) + (try a()); return x; }\n");
    assert!(out.contains("const x = ($tt_v0) + ($tt_v1);"), "{out}");
}

#[test]
fn a_captured_operand_carries_an_earlier_sibling_value_by_its_slot() {
    let out = ok("declare function a(): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\nfunction h() { return (try a()).toFixed(1).length + (try a()); }\n");
    assert!(out.contains("($tt_v0).toFixed(1).length"), "{out}");
    assert!(out.contains("+ ($tt_v1)") || out.contains("+ $tt_v1"), "{out}");
}

#[test]
fn an_ambient_module_variant_declares_its_constructor_object() {
    let out = ok("declare namespace N { variant P { Q, R(v: number) } }\ndeclare module \"m\" { export variant P3 { Q } }\n");
    assert!(out.contains("const P: {\n  readonly Q: { readonly kind: \"Q\" };\n  readonly R: (v: number) => P;\n};"), "{out}");
    assert!(out.contains("export const P3: {"), "{out}");
    assert!(!out.contains("as const"), "{out}");
}

#[test]
fn a_declared_variant_keeps_its_modifier_on_both_declarations() {
    let out = ok("export declare variant P2 { Q }\ndeclare variant P4<T> { W(value: T) }\n");
    assert!(out.contains("export declare type P2 ="), "{out}");
    assert!(out.contains("export declare const P2: {"), "{out}");
    assert!(out.contains("declare const P4: {\n  readonly W: <T>(value: T) => P4<T>;\n};"), "{out}");
}

#[test]
fn val_on_a_rest_parameter_guards_its_elements() {
    let diagnostics = ttc::analyze(
        "function f(val ...args: { a: number }[]) { args[0].a = 2; }\n",
        &Options::default(),
    );
    let codes: Vec<_> = diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(codes, [DiagnosticCode::ValMutation], "{diagnostics:#?}");
}

#[test]
fn a_try_statement_as_an_unbraced_body_opens_its_own_block() {
    let out = ok("declare function a(): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\nfunction g() { if (true) try a(); return 1; }\nfunction i() { while (true) try a(); }\n");
    assert!(out.contains("if (true) {\n  const $tt_t0 = a();"), "{out}");
    assert!(out.contains("while (true) {\n  const $tt_t1 = a();"), "{out}");
}

#[test]
fn a_recoverable_syntax_error_still_reports_plan_diagnostics() {
    let diagnostics = ttc::analyze(
        "declare const s: { kind: \"A\" } | { kind: \"B\" };\nclass K { field = match (s) { A => 0, B => 1 }; }\nconst a = 1 const b = 2;\n",
        &Options::default(),
    );
    let codes: Vec<_> = diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(
        codes,
        [DiagnosticCode::MatchPlacement, DiagnosticCode::SourceNotTypeScript],
        "{diagnostics:#?}"
    );
}

#[test]
fn untyped_try_methods_survive_next_to_tt_constructs() {
    let out = ok("variant V { A }\ninterface X { try(x); }\n");
    assert!(out.contains("interface X { try(x); }"), "{out}");
}

#[test]
fn sibling_matches_in_a_guard_have_one_complete_evaluation_owner() {
    let out = ok("const x = match (1) { 1 if (match (2) { 2 => true, _ => false }) && (match (3) { 3 => true, _ => false }) => 10, _ => 20 };\n");
    assert!(!out.contains("match ("), "{out}");
}
