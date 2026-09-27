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

#[test]
fn a_malformed_variant_behind_modifiers_is_reported_once() {
    for source in [
        "export variant V { A { r: number } }\n",
        "export declare variant V { A { r: number } }\n",
        "declare variant V { A { r: number } }\n",
        "variant V { A { r: number } }\n",
    ] {
        let diagnostics = ttc::analyze(source, &Options::default());
        let malformed: Vec<_> = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagnosticCode::MalformedVariant)
            .collect();
        assert_eq!(malformed.len(), 1, "{source}{diagnostics:#?}");
    }
}

#[test]
fn an_exported_try_declaration_reports_only_its_placement() {
    for source in [
        "declare function f(): any;\nexport const a = try f();\n",
        "declare function f(): any;\nnamespace N { export const a = try f(); }\n",
    ] {
        let diagnostics = ttc::analyze(source, &Options::default());
        let codes: Vec<_> = diagnostics.iter().map(|diagnostic| diagnostic.code).collect();
        assert_eq!(codes, [DiagnosticCode::TryPlacement], "{source}{diagnostics:#?}");
    }
}

#[test]
fn numeric_literal_patterns_take_their_ecmascript_values() {
    let out = ok("declare const x: number; declare const b: bigint;\n\
         export const a = match (x) { 1e400 => 1, 0x100000000000000000000000000000000 => 2, 0xff => 3, 1_000 => 4, _ => 0 };\n\
         export const c = match (b) { 0x100000000000000000000000000000000n => 1, 0o7n => 2, _ => 0 };\n");
    assert!(out.contains("1e400"), "{out}");
    for (source, duplicate) in [
        (
            "declare const b: bigint;\nexport const c = match (b) { 0x100000000000000000000000000000000n => 1, 340282366920938463463374607431768211456n => 2, _ => 0 };\n",
            "duplicate arm 340282366920938463463374607431768211456n",
        ),
        (
            "declare const x: number;\nexport const d = match (x) { 1e400 => 1, 2e400 => 2, _ => 0 };\n",
            "duplicate arm Infinity",
        ),
        (
            "declare const x: number;\nexport const d = match (x) { 1e21 => 1, 1000000000000000000000 => 2, _ => 0 };\n",
            "duplicate arm 1e+21",
        ),
        (
            "declare const x: number;\nexport const d = match (x) { 0.0000001 => 1, 1e-7 => 2, _ => 0 };\n",
            "duplicate arm 1e-7",
        ),
    ] {
        let diagnostics = ttc::analyze(source, &Options::default());
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(duplicate)),
            "{source}{diagnostics:#?}"
        );
    }
}

#[test]
fn generated_names_are_allocated_around_the_files_identifiers() {
    let out = ok("const $tt_ap = 1;\nconst \\u0024tt_m = 2;\nconst xs = [1].map(x => x |> String);\nconst r = match (xs[0]) { \"1\" => $tt_m, _ => $tt_ap };\n");
    assert!(
        out.starts_with("import { $tt_ap as $tt_ap_1 } from \"@tt/runtime\";\n"),
        "{out}"
    );
    assert!(out.contains("x => $tt_ap_1(x, String)"), "{out}");
    assert!(out.contains("const $tt_m_1 = xs[0];"), "{out}");
    assert!(out.contains("= $tt_m;"), "{out}");
}

#[test]
fn a_try_in_a_concise_arrow_inside_a_result_block_targets_the_arrow() {
    let out = ok("declare function get(n: number): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\n\
         export const r = result {\n\
           const f = (n: number) => ({ kind: \"Ok\" as const, value: try get(n) + 1 });\n\
           const x = try get(1);\n\
           return f(x);\n\
         };\n");
    let arrow = out
        .split("const f = ")
        .nth(1)
        .and_then(|rest| rest.split("\n  };").next())
        .unwrap_or_default();
    assert!(arrow.contains("return $tt_t0;"), "{out}");
    assert!(!arrow.contains("break"), "{out}");
}

#[test]
fn a_result_block_whose_try_sits_in_a_match_arm_reports_the_crossing() {
    let diagnostics = ttc::analyze(
        "declare const o: { kind: \"Some\"; value: number } | { kind: \"None\" };\n\
         declare function get(n: number): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\n\
         export const r = result { const q = match (o) { Some(value) => try get(value), None => 0 }; return q; };\n",
        &Options::default(),
    );
    let codes: Vec<_> = diagnostics.iter().map(|diagnostic| diagnostic.code).collect();
    assert_eq!(codes, [DiagnosticCode::TryCrossesValueRegion], "{diagnostics:#?}");
}

#[test]
fn a_tuple_missing_arm_suggestion_pastes_back_without_duplicate_bindings() {
    let source = "variant O { Some(value: number), None }\n\
         variant R { Ok(value: O), Err(error: string) }\n\
         declare const r: R; declare const s: R; declare const u: R;\n\
         export const x = match (r, s, u) { (Ok, Ok, Ok) => 1, (Err, _, _) => 2, (_, Err, _) => 3 };\n";
    let diagnostics = ttc::analyze(source, &Options::default());
    let missing = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagnosticCode::MatchNotExhaustive)
        .expect("the match is not exhaustive");
    let edit = missing.suggestions[0]
        .edit
        .as_ref()
        .expect("the missing-arm suggestion has an edit");
    let fixed = format!(
        "{}{}{}",
        &source[..edit.start],
        edit.replacement,
        &source[edit.end..]
    );
    let after = ttc::analyze(&fixed, &Options::default());
    assert!(after.is_empty(), "{fixed}{after:#?}");
}

#[test]
fn a_match_the_class_definition_evaluates_reports_its_placement() {
    let prelude = "variant S { A(n: number), B }\ndeclare const s: S;\ndeclare function tag(n: number): any;\ndeclare function say(m: string): any;\n";
    for body in [
        "class K extends say(\"x\") { @tag(match (s) { A(n) => n, B => 0 }) m() {} }\n",
        "class K { [match (s) { A => \"a\", B => \"b\" }]() {} }\n",
        "const E = class { @tag(match (s) { A(n) => n, B => 0 }) accessor a = 1; };\n",
        "@tag(match (s) { A(n) => n, B => 0 }) class K extends say(\"x\") {}\n",
        "@tag(1) class K extends match (s) { A => say(\"a\"), B => say(\"b\") } {}\n",
    ] {
        let source = format!("{prelude}{body}");
        let diagnostics = ttc::analyze(&source, &Options::default());
        let codes: Vec<_> = diagnostics.iter().map(|d| d.code).collect();
        assert_eq!(codes, [DiagnosticCode::MatchPlacement], "{source}{diagnostics:#?}");
        assert!(
            diagnostics[0].message.contains("decorator, a computed member name"),
            "{diagnostics:#?}"
        );
    }
    let out = ok(&format!(
        "{prelude}class K extends match (s) {{ A => say(\"a\"), B => say(\"b\") }} {{}}\n"
    ));
    assert!(out.contains("class K extends ($tt_v0 === 0 ? say(\"a\") : say(\"b\")) {}"), "{out}");
}

#[test]
fn a_yield_in_a_match_subject_or_guard_suspends_the_enclosing_generator() {
    let prelude = "variant S { A(n: number), B }\ndeclare const s: S;\n";
    for (body, expected) in [
        (
            "function* g(): Generator<number, number, any> {\n  const r = match (yield 1) { A(n) => n, B => 0 };\n  return r;\n}\n",
            "const $tt_m = yield 1;",
        ),
        (
            "function* g(): Generator<number, number, any> {\n  const r = match (s) { A(n) if (yield n) === 1 => n, _ => 0 };\n  return r;\n}\n",
            "if ((yield n) === 1) {",
        ),
        (
            "function* g(): Generator<number, number, any> {\n  const r = match ((yield 1) as any as S) { A(n) => n, B => 0 };\n  return r;\n}\n",
            "const $tt_m = (yield 1) as any as S;",
        ),
        (
            "function* g(): Generator<number, number, any> {\n  const r = match (s) { A(n) => match ((yield n) as S) { A(n: m) => m, B => 1 }, B => 0 };\n  return r;\n}\n",
            "const $tt_m = (yield n) as S;",
        ),
        (
            "function* g(): Generator<number, number, any> {\n  const r = match (s) { A(n) if [n].some((x) => x > 0) => match ((yield n) as S) { A(n: m) => m, B => 1 }, _ => 0 };\n  return r;\n}\n",
            "const $tt_m = (yield n) as S;",
        ),
        (
            "async function* g(): AsyncGenerator<number, number, any> {\n  const r = match (await (yield 1)) { A(n) => n, B => 0 };\n  return r;\n}\n",
            "const $tt_m = await (yield 1);",
        ),
        (
            "class K { m() { return 1; } }\nclass L extends K {\n  *g(): Generator<number, number, any> {\n    return match ((yield super.m()) as S) { A(n) => n, B => 0 };\n  }\n}\n",
            "const $tt_m = (yield super.m()) as S;",
        ),
    ] {
        let out = ok(&format!("{prelude}{body}"));
        assert!(out.contains(expected), "{body}{out}");
    }
}

#[test]
fn a_value_hoisted_out_of_an_unbraced_body_opens_its_own_block() {
    let prelude = "variant S { A(n: number), B }\ndeclare const s: S;\ndeclare const c: boolean;\ndeclare function g(n: number): void;\ndeclare const r: { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\n";
    for (body, opened, closed) in [
        ("function f() { if (c) return match (s) { A(n) => n, B => 0 }; return -1; }", "if (c) { let $tt_v0: number;", "return $tt_v0; } return -1;"),
        ("function f(xs: number[]) { for (const q of xs) g(match (s) { A(n) => n, B => q }); }", "for (const q of xs) { const $tt_v1 = (g);", "} } }"),
        ("function f() { if (c) g(try r); else g(0); }", "if (c) { let $tt_v0: number;", "$tt_v1($tt_v0); } else g(0);"),
        ("function f() { lbl: match (s) { A(n) => g(n), B => g(0) }; }", "lbl: { let $tt_v0: void;", "} ; }"),
        ("function f() { while (c) match (s) { A(n) => g(n), B => g(0) } }", "while (c) { let $tt_v0: void;", "} } }"),
        ("function f() { if (c) g(0); else if (match (s) { A(n) => n > 0, B => false }) g(1); }", "else { let $tt_v0: boolean;", "if ($tt_v0) g(1); }"),
        ("function f() { if (c) for (let i = try r; i < 3; i++) g(i); }", "if (c) { const $tt_t0 = r;", "for (let i = $tt_t0.value; i < 3; i++) g(i); }"),
    ] {
        let out = compact(&ok(&format!("{prelude}{body}\n")));
        assert!(out.contains(opened), "{body}\n{out}");
        assert!(out.contains(closed), "{body}\n{out}");
    }
}

#[test]
fn a_yield_crossing_a_result_block_reports_only_the_crossing() {
    let source = "import type { TResult } from \"@tt/std\";\ndeclare const x: TResult<number, string>;\nfunction* g(): Generator<number, unknown, number> {\n  const r = result { const v = try x; const w = yield v; return w; };\n  return r;\n}\n";
    let diagnostics = ttc::analyze(source, &Options::default());
    let codes: Vec<_> = diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(codes, [DiagnosticCode::ResultYieldCrossing], "{diagnostics:#?}");
}

#[test]
fn a_pipeline_as_the_unbraced_body_of_a_statement_header_starts_after_the_header() {
    let out = ok("declare const c: boolean, x: number;\n\
         declare function g(n: number): any;\n\
         declare const xs: AsyncIterable<number>;\n\
         if (c) x |> g;\n\
         if (c) x |> g; else x |> g;\n\
         while (c) x |> g;\n\
         for (;;) x |> g;\n\
         for (const a of [1]) a |> g;\n\
         async function h() { for await (const v of xs) v |> g; }\n\
         const y = (c) |> g;\n");
    assert!(out.contains("if (c) $tt_ap(x, g);"), "{out}");
    assert!(
        out.contains("if (c) $tt_ap(x, g); else $tt_ap(x, g);"),
        "{out}"
    );
    assert!(out.contains("while (c) $tt_ap(x, g);"), "{out}");
    assert!(out.contains("for (;;) $tt_ap(x, g);"), "{out}");
    assert!(out.contains("for (const a of [1]) $tt_ap(a, g);"), "{out}");
    assert!(
        out.contains("for await (const v of xs) $tt_ap(v, g);"),
        "{out}"
    );
    assert!(out.contains("const y = $tt_ap((c), g);"), "{out}");
}

#[test]
fn try_binds_to_a_private_member_operand() {
    let out = ok("variant R { Ok(value: number), Err(error: string) }\n\
         class C {\n\
         #v: R = R.Ok(1);\n\
         #c: C = this;\n\
         #case: R = R.Ok(1);\n\
         #match(): R { return R.Ok(2); }\n\
         f(): R {\n\
         const a = try this.#v;\n\
         try this.#v;\n\
         const b = try this.#c?.#v;\n\
         try this.#case;\n\
         const d = try this.#match() * 2;\n\
         return R.Ok(a + b + d);\n\
         }\n\
         }\n");
    assert!(out.contains("const $tt_t0 = this.#v;"), "{out}");
    assert!(out.contains("const $tt_t1 = this.#v;"), "{out}");
    assert!(out.contains("const $tt_t2 = this.#c?.#v;"), "{out}");
    assert!(out.contains("const $tt_t3 = this.#case;"), "{out}");
    assert!(out.contains("const $tt_t4 = this.#match();"), "{out}");
    assert!(out.contains("const d = $tt_v0 * 2;"), "{out}");
}

#[test]
fn a_labeled_loop_keeps_its_label_on_the_loop_when_its_header_hoists_a_value() {
    let out = compact(&ok("variant S { A(n: number), B }\ndeclare const s: S;\ndeclare const c: boolean;\nfunction f(xs: number[][]) {\n  lbl: for (const q of match (s) { A(n) => xs[n], B => [] }) { if (c) continue lbl; }\n  if (c) outer: inner: for (const q of match (s) { A(n) => xs[n], B => [] }) { continue outer; }\n}\n"));
    assert!(out.contains("} lbl: for (const q of $tt_v0) { if (c) continue lbl; }"), "{out}");
    assert!(out.contains("if (c) { let $tt_v1: number[];"), "{out}");
    assert!(out.contains("} outer: inner: for (const q of $tt_v1) { continue outer; } }"), "{out}");
}
