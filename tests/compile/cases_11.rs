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

#[test]
fn an_if_let_as_an_unbraced_body_is_projected_as_one_statement() {
    for source in [
        "variant O { Some(value: number), None }\nfunction f(xs: O[]): number {\n  let t = 0;\n  for (const x of xs) if let Some(value) = x { t += value; } else { break; }\n  return t;\n}\n",
        "variant O { Some(value: number), None }\nfunction f(c: boolean, x: O): number {\n  if (c) if let Some(value) = x { return value; } else { return 2; }\n  else { return 3; }\n}\n",
        "variant O { Some(value: number), None }\nfunction f(xs: O[]): number {\n  let t = 0;\n  outer: for (const x of xs) if let Some(value) = x { if (value > 5) continue outer; t += value; }\n  return t;\n}\n",
        "variant O { Some(value: number), None }\nfunction f(c: boolean, x: O, y: O): number {\n  while (c) if let Some(value) = x { return value; } else if let Some(value) = y { return value; } else { break; }\n  return 0;\n}\n",
    ] {
        let diagnostics = ttc::analyze(source, &Options::default());
        assert!(diagnostics.is_empty(), "{source}{diagnostics:#?}");
        ok(source);
    }
}

#[test]
fn a_default_exported_variant_is_reported_at_default_with_a_named_export_fix() {
    for (source, line) in [
        ("export default variant Dir { Up, Down }\n", 1),
        (
            "export variant Shape { Circle(r: number) }\nexport default variant Dir { Up, Down }\n",
            2,
        ),
    ] {
        let diagnostics = ttc::analyze(source, &Options::default());
        assert_eq!(diagnostics.len(), 1, "{source}{diagnostics:#?}");
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.code, DiagnosticCode::VariantDefaultExport);
        assert_eq!(diagnostic.message, "variant `Dir` cannot be a default export");
        let default = source.find("default").unwrap();
        assert_eq!(diagnostic.start, Some(default));
        assert_eq!(diagnostic.end, Some(default + "default".len()));
        let edit = diagnostic.suggestions[0]
            .edit
            .as_ref()
            .expect("a named-export edit");
        let mut fixed = source.to_string();
        fixed.replace_range(edit.start..edit.end, &edit.replacement);
        assert!(fixed.contains("export variant Dir { Up, Down }"), "{fixed}");
        assert!(ttc::analyze(&fixed, &Options::default()).is_empty(), "{fixed}");
        let e = err(source);
        assert_eq!((e.line, e.col), (line, 8), "{e}");
    }
}

#[test]
fn a_default_exported_variant_that_does_not_parse_reports_the_variant() {
    let source = "export default variant Dir { Up, Down(x: ) }\n";
    let diagnostics = ttc::analyze(source, &Options::default());
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].code, DiagnosticCode::MalformedVariant);
}

#[test]
fn a_generated_parse_failure_is_located_at_the_construct_that_generated_it() {
    for source in [
        "export variant Shape { Circle(r: number) }\nconst x = variant Dir { Up, Down };\n",
        "export variant Shape { Circle(r: number) }\nf(variant Dir { Up, Down });\n",
    ] {
        let diagnostics = ttc::analyze(source, &Options::default());
        let failure = diagnostics
            .iter()
            .find(|d| d.code == DiagnosticCode::LoweringPlanFailed)
            .unwrap_or_else(|| panic!("{source}{diagnostics:#?}"));
        let construct = source.find("variant Dir").unwrap();
        let name = source.find("Dir").unwrap();
        assert!(
            failure
                .start
                .is_some_and(|start| construct <= start && start <= name),
            "{source}{failure:#?}"
        );
    }
}

#[test]
fn a_type_assertion_after_a_pipeline_step_applies_to_the_whole_pipeline() {
    let out = compact(&ok("declare const f: (n: number) => string;\nconst d = 1 |> String as string;\nconst e = 1 |> String satisfies string;\nconst g = 1 |> ((x: number) => x) as number;\nconst h = 1 |> f as string | undefined;\nconst k = flow |> f as (n: number) => string;\nconst m = [1 |> f as string |> .length satisfies number |> String, 2];\n"));
    assert!(out.contains("const d = String(1) as string;"), "{out}");
    assert!(out.contains("const e = String(1) satisfies string;"), "{out}");
    assert!(out.contains("const g = ((x: number) => x)(1) as number;"), "{out}");
    assert!(out.contains("const h = f(1) as string | undefined;"), "{out}");
    assert!(out.contains("const k = f as (n: number) => string;"), "{out}");
    assert!(
        out.contains("const m = [$tt_ap((f(1) as string).length satisfies number, String), 2];"),
        "{out}"
    );
}

#[test]
fn a_pipeline_step_that_is_not_a_primary_expression_is_called_as_a_group() {
    let out = compact(&ok("declare const f: ((n: number) => string) | undefined;\ndeclare const g: (n: number) => string;\ndeclare const as: (n: number) => string;\nconst a = 1 |> f ?? g;\nconst b = 1 |> await Promise.resolve(g);\nconst c = 1 |> as;\nexport {};\n"));
    assert!(out.contains("const a = (f ?? g)(1);"), "{out}");
    assert!(out.contains("const b = (await Promise.resolve(g))(1);"), "{out}");
    assert!(out.contains("const c = as(1);"), "{out}");
}

#[test]
fn a_type_assertion_on_the_line_after_a_pipeline_is_rejected_as_typescript_rejects_it() {
    let report = ttc::compile_report("const d = 1 |> String\n  as string;\n", &Options::default());
    let codes: Vec<_> = report.diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(codes, [DiagnosticCode::SourceNotTypeScript], "{:#?}", report.diagnostics);
}

#[test]
fn a_wrapped_concise_arrow_value_lowers_to_a_block_body_not_an_iife() {
    let out = compact(&ok("variant V { A(n: number), B }\nconst g = async (p: Promise<V>) => match (await p) { A(n) => n, B => 0 } as number;\nconst h = (v: V) => match (v) { A(n) => n, B => 0 } satisfies number;\nconst i = (v: V) => (match (v) { A(n) => n, B => 0 }) as number;\nconst j = (v: V) => (match (v) { A(n) => n, B => 0 });\n"));
    assert!(!out.contains("})()"), "{out}");
    assert!(out.contains("const g = async (p: Promise<V>) => { let $tt_v0: number; { const $tt_m = await p;"), "{out}");
    assert!(out.contains("return $tt_v0 as number; };"), "{out}");
    assert!(out.contains("return $tt_v1 satisfies number; };"), "{out}");
    assert!(out.contains("return ($tt_v2) as number; };"), "{out}");
    assert!(out.contains("return ($tt_v3); };"), "{out}");
}

#[test]
fn a_wrapped_value_in_a_parenthesized_step_arrow_stays_inside_that_arrow() {
    let out = compact(&ok("type R<T> = { kind: \"Ok\"; value: T } | { kind: \"Err\"; error: string };\nvariant V { A(n: number), B }\ndeclare function next(): R<number>;\ndeclare const value: number;\nconst f = value |> (x => (try next()));\nconst h = value |> ((v: number) => (match (V.A(v)) { A(n) => n, B => 0 }) as number);\n"));
    assert!(out.contains("const f = $tt_ap(value, ((x => { let $tt_v0:"), "{out}");
    assert!(out.contains("return ($tt_v0); })));"), "{out}");
    assert!(out.contains("const h = $tt_ap(value, (((v: number) => { let $tt_v1: number;"), "{out}");
    assert!(out.contains("return ($tt_v1) as number; })));"), "{out}");
}

#[test]
fn a_var_let_else_as_an_unbraced_body_is_lowered_inside_one_block() {
    for (source, lowered) in [
        (
            "variant O { Some(value: number), None }\nfunction f(o: O) {\n  if (true) var Some(value: hv) = o else { return 0; };\n  return hv;\n}\n",
            "if (true) { const $tt_t0 = o; if ($tt_t0.kind !== \"Some\") { return 0; } var { value: hv } = $tt_t0; } return hv;",
        ),
        (
            "variant O { Some(value: number), None }\nfunction f(o: O) {\n  while (true) var Some(value: wv) = o else { return 0; };\n}\n",
            "while (true) { const $tt_t0 = o; if ($tt_t0.kind !== \"Some\") { return 0; } var { value: wv } = $tt_t0; } }",
        ),
        (
            "variant O { Some(value: number), None }\nfunction f(c: boolean, o: O) {\n  if (c) f(c, o); else var Some(value: ev) = o else { return 0; };\n  return ev;\n}\n",
            "if (c) f(c, o); else { const $tt_t0 = o; if ($tt_t0.kind !== \"Some\") { return 0; } var { value: ev } = $tt_t0; } return ev;",
        ),
        (
            "variant O { Some(value: number), None }\nfunction f(o: O) {\n  lbl: var Some(value: lv) = o else { return 0; };\n  return lv;\n}\n",
            "lbl: { const $tt_t0 = o; if ($tt_t0.kind !== \"Some\") { return 0; } var { value: lv } = $tt_t0; } return lv;",
        ),
        (
            "variant O { Some(value: number), None }\nfunction f(xs: O[]) {\n  for (const x of xs) var Some(value: fv) = x else { break; };\n  return fv;\n}\n",
            "for (const x of xs) { const $tt_t0 = x; if ($tt_t0.kind !== \"Some\") { break; } var { value: fv } = $tt_t0; } return fv;",
        ),
    ] {
        let diagnostics = ttc::analyze(source, &Options::default());
        assert!(diagnostics.is_empty(), "{source}{diagnostics:#?}");
        let out = ok(source);
        assert!(compact(&out).contains(lowered), "{out}");
    }
}

#[test]
fn a_lexical_binding_statement_as_an_unbraced_body_is_a_placement_error() {
    for (source, code, head) in [
        (
            "variant O { Some(value: number), None }\nfunction f(c: boolean, o: O) {\n  if (c) const Some(value) = o else { return 0; };\n  return 1;\n}\n",
            DiagnosticCode::LetElsePlacement,
            "const Some(value) = o",
        ),
        (
            "variant O { Some(value: number), None }\nfunction f(o: O) {\n  while (true) let Some(value) = o else { return 0; };\n}\n",
            DiagnosticCode::LetElsePlacement,
            "let Some(value) = o",
        ),
        (
            "variant O { Some(value: number), None }\nfunction f(o: O) {\n  lbl: const Some(value) = o else { return 0; };\n}\n",
            DiagnosticCode::LetElsePlacement,
            "const Some(value) = o",
        ),
        (
            "variant R { Ok(value: number), Err(error: string) }\ndeclare const p: () => R;\nfunction f(c: boolean): R {\n  if (c) const x = try p();\n  return R.Ok(0);\n}\n",
            DiagnosticCode::TryPlacement,
            "const x = try p()",
        ),
    ] {
        let diagnostics = ttc::analyze(source, &Options::default());
        assert_eq!(diagnostics.len(), 1, "{source}{diagnostics:#?}");
        assert_eq!(diagnostics[0].code, code, "{source}{diagnostics:#?}");
        assert_eq!(
            diagnostics[0].start,
            Some(source.find(head).unwrap()),
            "{source}{diagnostics:#?}"
        );
        err(source);
    }
    for source in [
        "variant O { Some(value: number), None }\nfunction f(c: boolean, o: O) {\n  if (c) { const Some(value) = o else { return 0; }; return value; }\n  return 1;\n}\n",
        "variant R { Ok(value: number), Err(error: string) }\ndeclare const p: () => R;\nfunction f(c: boolean): R {\n  if (c) { const x = try p(); return R.Ok(x); }\n  if (c) var y = try p();\n  return R.Ok(0);\n}\n",
    ] {
        let diagnostics = ttc::analyze(source, &Options::default());
        assert!(diagnostics.is_empty(), "{source}{diagnostics:#?}");
        ok(source);
    }
}

#[test]
fn a_conditional_try_keeps_its_operand_when_a_later_sibling_captures_it() {
    for body in [
        "const x = [c ? try p(s) : 0, c ? try p(s) : 1]; return R.Ok(x[0]);",
        "const x = [c ? try p(s) : 0, c ? 5 : 1, c ? try p(s) : 1]; return R.Ok(x[0]);",
        "return R.Ok((n && try p(s)) + (n && try p(s)));",
        "return R.Ok(pair(n && try p(s), n && try p(s)));",
        "return R.Ok(`${c ? try p(s) : 0}-${c ? try p(s) : 1}`.length);",
    ] {
        let source = format!(
            "variant R {{ Ok(value: number), Err(error: string) }}\ndeclare const p: (s: string) => R;\ndeclare const pair: (a: number, b: number) => number;\nfunction f(c: boolean, n: number, s: string): R {{\n  {body}\n}}\n"
        );
        let diagnostics = ttc::analyze(&source, &Options::default());
        assert!(diagnostics.is_empty(), "{source}{diagnostics:#?}");
        let out = ok(&source);
        assert_eq!(out.matches("= p(s);").count(), body.matches("try p(s)").count(), "{out}");
        assert!(!out.contains("= ;"), "{out}");
    }
}

fn codes(src: &str) -> Vec<DiagnosticCode> {
    ttc::analyze(src, &Options::default())
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn explained_examples_behave_as_their_explanations_say() {
    let nested = DiagnosticCode::MatchNestedInOrPattern.explanation();
    let arms = "Ok(value: Some(v)) => v,\n    Ok(value: None()) => 0,\n    Err(error) => -1,";
    assert!(nested.contains(arms), "{nested}");
    assert!(nested.contains("`Ok(value: Some(v) | None())`"), "{nested}");
    let prelude = "declare const r: { kind: \"Ok\"; value: { kind: \"Some\"; value: number } | { kind: \"None\" } } | { kind: \"Err\"; error: string };\n";
    assert_eq!(
        codes(&format!("{prelude}const a = match (r) {{ {arms} }};\n")),
        vec![]
    );
    assert_eq!(
        codes(&format!(
            "{prelude}const a = match (r) {{ Ok(value: Some(v) | None()) => 1, Err(error) => -1 }};\n"
        )),
        vec![DiagnosticCode::MalformedMatch]
    );
    assert_eq!(
        codes(&format!(
            "{prelude}const a = match (r) {{ Ok(value: Some(v)) | Err(error) => 1, _ => 0 }};\n"
        ))[0],
        DiagnosticCode::MatchNestedInOrPattern
    );
    assert_eq!(
        codes(
            "variant V { A, B, C }\ndeclare const v: V;\nconst a = match (v, v) { (A, B | C) => 1, _ => 0 };\n"
        ),
        vec![]
    );

    let placement = DiagnosticCode::TryPlacement.explanation();
    for host in ["constructor", "generator", "static block"] {
        assert!(placement.contains(host), "{host}: {placement}");
    }
    for source in [
        "declare function f(): any;\nclass C { constructor() { const x = try f(); } }\n",
        "declare function f(): any;\nfunction* g() { const x = try f(); yield x; }\n",
        "declare function f(): any;\nasync function* g() { const x = try f(); yield x; }\n",
        "declare function f(): any;\nclass C { static { const x = try f(); } }\n",
    ] {
        assert_eq!(codes(source), vec![DiagnosticCode::TryPlacement], "{source}");
    }

    let hole = "missing \"Ok(value: None())\"";
    let exhaustive = DiagnosticCode::MatchNotExhaustive.explanation();
    assert!(exhaustive.contains(hole), "{exhaustive}");
    let missing = ttc::analyze(
        &format!("{prelude}const a = match (r) {{ Ok(value: Some(v)) => v, Err(error) => -1 }};\n"),
        &Options::default(),
    );
    assert!(missing[0].message.contains(hole), "{missing:?}");

    let arity = DiagnosticCode::MatchTupleArity.explanation();
    assert!(arity.contains("`match ((a < b), c > (d))`"), "{arity}");
    let tuple = "variant V { A, B }\ndeclare const a: any, b: number, c: number, d: number;\n";
    assert_eq!(
        codes(&format!(
            "{tuple}const x = match ((a < b), c > (d)) {{ (A, _) => 1, _ => 2 }};\n"
        )),
        vec![]
    );
    assert_eq!(
        codes(&format!(
            "{tuple}const x = match (a < b, c > (d)) {{ (A, _) => 1, _ => 2 }};\n"
        )),
        vec![DiagnosticCode::MatchTupleArity]
    );
}

/* ------------------------------------------------------------------ */
/* TASK-480 `if let` in expression position                            */
/* ------------------------------------------------------------------ */

#[test]
fn a_guarded_all_wildcard_tuple_arm_is_tested_by_its_guard_alone() {
    let out = ok("variant T { A, B }\nfunction f(a: T, b: T, cond: boolean): number {\n  return match (a, b) {\n    (A, _) => 1,\n    (_, _) if cond => 2,\n    _ => 3,\n  };\n}\n");
    assert!(!out.contains("if ()"), "{out}");
    assert!(out.contains("      if (cond) {\n        $tt_v0 = 2;\n        break;\n      }\n      $tt_v0 = 3;"), "{out}");
}

#[test]
fn an_if_let_in_any_expression_position_reports_only_its_placement() {
    let prelude = "variant O { Some(value: number), None }\n\
                   declare const o: O;\n\
                   declare const c: boolean;\n\
                   declare function g(x: unknown): number;\n";
    for body in [
        "function f() { const x = if let Some(value) = o { value }; return x; }",
        "const x = if let Some(value) = o { value };",
        "function f() { const x = `${if let Some(value) = o { value }}`; return x; }",
        "function f() { g(if let Some(value) = o { value }); }",
        "const h = () => if let Some(value) = o { value };",
        "function f() { return if let Some(value) = o { value }; }",
        "function f() { throw if let Some(value) = o { value }; }",
        "function f() { return (\n  if let Some(value) = o { value }); }",
        "const x = 1 + if let Some(value) = o { value } else { 0 };",
        "const x = [if let Some(value) = o { value }];",
        "const x = { a: if let Some(value) = o { value } };",
        "const x = c ? if let Some(value) = o { value } : 1;",
        "for (let i = 0; i < 1; if let Some(value) = o { value }) {}",
        "class C { m() { const x = if let Some(value) = o { value }; } }",
        "const x = match (o) { Some(value) => if let Some(value: v) = o { v }, None => 0 };",
        "const x = match (if let Some(value) = o { value }) { Some(value) => 1, None => 0 };",
        "const x = if let Some(value) = o { value } |> g;",
        "const x = o |> if let Some(value) = o { value };",
        "const x = (if let Some(value) = o { value }) |> g;",
        "function f() { const x = try if let Some(value) = o { value }; }",
        "function f() { g(try if let Some(value) = o { value }); }",
        "function f() { try if let Some(value) = o { value }; }",
    ] {
        let source = format!("{prelude}{body}\n");
        let diagnostics = ttc::analyze(&source, &Options::default());
        let codes: Vec<_> = diagnostics.iter().map(|d| d.code).collect();
        assert_eq!(
            codes,
            [DiagnosticCode::IfLetPlacement],
            "{body}\n{diagnostics:#?}"
        );
        let at = diagnostics[0].start.expect("located");
        assert!(
            source[at..].starts_with("if let"),
            "{body}: {:?}",
            &source[at..]
        );
    }
}

#[test]
fn an_if_let_that_starts_a_statement_stays_a_statement() {
    let prelude = "variant O { Some(value: number), None }\n\
                   declare const o: O;\n\
                   declare function g(x: unknown): number;\n";
    for body in [
        "const x = 1\nif let Some(value) = o { g(value); }",
        "const h = () => 1\nif let Some(value) = o { g(value); }",
        "if (o) if let Some(value) = o { g(value); }",
        "outer: if let Some(value) = o { g(value); }",
        "const x = `${(() => { if let Some(value) = o { return value; } return 0; })()}`;",
        "const x = match (o) { Some(value) => { if let Some(value: v) = o { g(v); } return 1; }, None => 0 };",
    ] {
        let source = format!("{prelude}{body}\n");
        assert_eq!(codes(&source), vec![], "{body}");
        let out = ok(&source);
        assert!(out.contains(".kind === \"Some\""), "{out}");
    }
}

#[test]
fn a_try_operand_never_starts_with_a_statement_keyword() {
    assert_eq!(
        codes("declare function f(): any;\nfunction g() { const x = try if (f()) {}; }\n"),
        codes("declare function f(): any;\nfunction g() { const x = if (f()) {}; }\n"),
    );
}

#[test]
fn if_let_placement_explanation_names_value_positions() {
    let text = DiagnosticCode::IfLetPlacement.explanation();
    for position in ["initializer", "argument", "concise arrow body", "`return`"] {
        assert!(text.contains(position), "{position}: {text}");
    }
}

/* ------------------------------------------------------------------ */
/* TASK-481 jumps crossing a `result` block                            */
/* ------------------------------------------------------------------ */

#[test]
fn a_jump_crossing_a_result_block_reports_only_the_crossing() {
    use DiagnosticCode::{ResultBreakCrossing, ResultContinueCrossing, ResultLabelCrossing};
    let prelude = "import type { TResult } from \"@tt/std\";\n\
                   declare const x: TResult<number, string>;\n";
    let cases: [(&str, &[DiagnosticCode]); 12] = [
        (
            "function f() { for (;;) { const r = result { const v = try x; if (v) break; return v; }; } }",
            &[ResultBreakCrossing],
        ),
        (
            "function f() { for (;;) { const r = result { const v = try x; if (v) continue; return v; }; } }",
            &[ResultContinueCrossing],
        ),
        (
            "function f() { outer: for (;;) { const r = result { const v = try x; if (v) break outer; return v; }; } }",
            &[ResultLabelCrossing],
        ),
        (
            "function f() { outer: for (;;) { const r = result { const v = try x; if (v) continue outer; return v; }; } }",
            &[ResultLabelCrossing],
        ),
        (
            "function f() { outer: { const r = result { const v = try x; if (v) break outer; return v; }; } }",
            &[ResultLabelCrossing],
        ),
        (
            "function f() { outer: for (;;) { const r = result { const v = try x; inner: for (;;) { if (v) break outer; } return v; }; } }",
            &[ResultLabelCrossing],
        ),
        (
            "function f() { switch (1) { case 1: const r = result { const v = try x; if (v) break; return v; }; } }",
            &[ResultBreakCrossing],
        ),
        (
            "function* g() { for (;;) { const r = result { const v = try x; if (v) break; return v; }; } }",
            &[ResultBreakCrossing],
        ),
        (
            "async function f() { for (;;) { const r = result { const v = try x; if (v) break; return v; }; } }",
            &[ResultBreakCrossing],
        ),
        (
            "function f() { for (;;) { const r = `${result { const v = try x; if (v) break; return v; }}`; } }",
            &[ResultBreakCrossing],
        ),
        (
            "function f() { for (const a of [1]) { const r = (result { const v = try x; if (v) continue; return v; }) |> String; } }",
            &[ResultContinueCrossing],
        ),
        (
            "function f() { outer: for (;;) { const r = result { const v = try x; if (v) break outer; if (v) continue; return v; }; } }",
            &[ResultLabelCrossing, ResultContinueCrossing],
        ),
    ];
    for (body, expected) in cases {
        let source = format!("{prelude}{body}\n");
        let diagnostics = ttc::analyze(&source, &Options::default());
        let found: Vec<_> = diagnostics.iter().map(|d| d.code).collect();
        assert_eq!(found, expected, "{body}\n{diagnostics:#?}");
        for diagnostic in &diagnostics {
            let at = diagnostic.start.expect("located");
            assert!(
                source[at..].starts_with("break") || source[at..].starts_with("continue"),
                "{body}: {:?}",
                &source[at..]
            );
        }
    }
}

#[test]
fn a_jump_crossing_a_result_block_leaves_the_rest_of_the_file_planned() {
    let source = "import type { TResult } from \"@tt/std\";\n\
                  declare const x: TResult<number, string>;\n\
                  function f() { for (;;) { const r = result { const v = try x; if (v) break; return v; }; } }\n\
                  class C { y = try x; }\n";
    assert_eq!(
        codes(source),
        vec![
            DiagnosticCode::ResultBreakCrossing,
            DiagnosticCode::TryPlacement
        ]
    );
}

#[test]
fn jumps_owned_inside_a_result_block_still_compile() {
    let out = ok("import type { TResult } from \"@tt/std\";\n\
                  declare const x: TResult<number, string>;\n\
                  function f() { for (;;) { const r = result { const v = try x; inner: for (;;) { if (v) break inner; continue inner; } for (;;) { break; } return v; }; } }\n");
    assert!(out.contains("break inner;"), "{out}");
    assert!(out.contains("continue inner;"), "{out}");
}

/* ------------------------------------------------------------------ */
/* TASK-482 automatic semicolon boundaries after postfix and restricted */
/* ------------------------------------------------------------------ */

#[test]
fn an_if_let_after_an_automatic_semicolon_boundary_starts_a_statement() {
    let prelude = "variant O { Some(value: number), None }\n\
                   declare const o: O;\n\
                   declare function g(x: unknown): number;\n";
    for body in [
        "function f() {\n  let q = 1\n  q++\n  if let Some(value) = o { g(value); }\n}",
        "function f() {\n  let q = 1\n  q--\n  if let Some(value) = o { g(value); }\n}",
        "function f(p: number | undefined) {\n  p!\n  if let Some(value) = o { g(value); }\n}",
        "function f(p: { a?: number }) {\n  p.a!!\n  if let Some(value) = o { g(value); }\n}",
        "function f() {\n  const k = [1] as const\n  if let Some(value) = o { g(value); }\n}",
        "function f() {\n  return\n  if let Some(value) = o { g(value); }\n}",
        "function* f() {\n  yield\n  if let Some(value) = o { g(value); }\n}",
        "function f() {\n  const k = { return: 1 }.return\n  if let Some(value) = o { g(value); }\n}",
        "function f() {\n  for (;;) {\n    break\n    if let Some(value) = o { g(value); }\n  }\n}",
    ] {
        let source = format!("{prelude}{body}\n");
        assert_eq!(codes(&source), vec![], "{body}");
        let out = ok(&source);
        assert!(out.contains(".kind === \"Some\""), "{out}");
    }
    let out = ok_tsx(&format!(
        "{prelude}export const e = <button onClick={{() => {{\n  let q = 1\n  q++\n  if let Some(value) = o {{ g(value); }}\n}}}} />;\n"
    ));
    assert!(out.contains(".kind === \"Some\""), "{out}");
}

#[test]
fn a_pipeline_head_starts_after_a_postfix_or_restricted_boundary() {
    let prelude = "declare const o: number;\n";
    for (body, head) in [
        ("let q = 1\nq++\no |> String;", "q++\n$tt_ap(o, String)"),
        ("let p: number | undefined\np!\no |> String;", "p!\n$tt_ap(o, String)"),
        ("const k = [1] as const\no |> String;", "as const\n$tt_ap(o, String)"),
        ("function f() {\n  return\n  o |> String;\n}", "return\n  $tt_ap(o, String)"),
    ] {
        let out = ok(&format!("{prelude}{body}\n"));
        assert!(out.contains(head), "{body}\n{out}");
    }
}

#[test]
fn a_line_break_inside_an_expression_still_continues_it() {
    let prelude = "variant O { Some(value: number), None }\n\
                   declare let q: number;\n\
                   declare const o: O;\n\
                   declare const p: ((x: number) => number) | undefined;\n";
    for body in [
        "const x = q++\n  + if let Some(value) = o { value };",
        "const x = p!\n  (if let Some(value) = o { value });",
    ] {
        let source = format!("{prelude}{body}\n");
        assert_eq!(codes(&source), [DiagnosticCode::IfLetPlacement], "{body}");
    }
}
