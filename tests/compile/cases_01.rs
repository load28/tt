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
fn generated_value_slots_preserve_authored_contextual_types() {
    let result = ok(
        "type TResult<T, E> = { kind: \"Ok\"; value: T } | { kind: \"Err\"; error: E };\n\
         declare const g: () => TResult<number, string>;\n\
         const f = (): TResult<readonly number[], string> => result {\n\
           const n = try g(); if (n === 0) return []; return [n];\n\
         };\n",
    );
    assert!(
        result.contains("let $tt_v0: TResult<readonly number[], string>;"),
        "{result}"
    );
    assert!(!result.contains("const $tt_result = []"), "{result}");

    let matched = ok("type Toggle = \"on\" | \"off\";\n\
         const flip = (value: Toggle): Toggle => match (value) {\n\
           \"on\" => \"off\", \"off\" => \"on\",\n\
         };\n");
    assert!(matched.contains("let $tt_v0: Toggle;"), "{matched}");
}

#[test]
fn match_arm_single_statement_if_keeps_synthetic_exit_conditional() {
    let output = ok("variant V { A(n: number), B }\n\
         const f = (v: V): number => match (v) {\n\
           A(n) => { if (n === 0) return 100; return n; },\n\
           B => -1,\n\
         };\n");
    assert!(
        compact(&output).contains("if (n === 0) { $tt_v0 = 100; break; }"),
        "{output}"
    );
}

#[test]
fn result_region_composes_embedded_try_and_pipeline_try() {
    let embedded = ok("variant R { Ok(value: number), Err(error: string) }\n\
         declare const g: () => R;\n\
         const f = (): R => result { return Math.round(try g() * 1.1); };\n");
    assert!(embedded.contains("Math.round("), "{embedded}");
    assert!(!embedded.contains("try g"), "{embedded}");

    let pipeline = ok("variant R { Ok(value: number), Err(error: string) }\n\
         declare const g: () => R; declare const step: (x: R) => R;\n\
         const f = (): R => result { const v = try (g() |> step); return v; };\n");
    assert!(!pipeline.contains("try ("), "{pipeline}");
}

#[test]
fn result_region_pipeline_head_completes_before_the_pipeline_step() {
    let output = ok("variant R { Ok(value: number), Err(error: string) }\n\
         declare const g: () => R; declare const unwrap: (x: R) => number;\n\
         const f = (): number => { const n = result {\n\
           const value = try g(); return value * 2;\n\
         } |> unwrap; return n; };\n");
    let completion = output.find("kind: \"Ok\"").expect("result completion");
    let pipeline = output.find("unwrap(").expect("pipeline step");
    assert!(completion < pipeline, "{output}");
    assert!(!output[..pipeline].contains("return value * 2"), "{output}");
}

#[test]
fn async_concise_arrow_claims_a_result_region() {
    let output = ok("variant R { Ok(value: number), Err(error: string) }\n\
         declare const g: () => R;\n\
         const f = async (): Promise<R> => result { const x = try g(); return x + 1; };\n");
    assert!(
        output.contains("const f = async (): Promise<R> => {"),
        "{output}"
    );
    assert!(!output.contains("=> result"), "{output}");
}

#[test]
fn jsx_expression_pipeline_rewrites_only_the_container_expression() {
    let child = ok_tsx(
        "declare const raw: string; declare const up: (x: string) => string;\n\
         const child = <p>{raw |> up}</p>;\n",
    );
    assert!(child.contains("<p>{$tt_ap(raw, up)}</p>"), "{child}");

    let attribute = ok_tsx(
        "declare const raw: string; declare const up: (x: string) => string;\n\
         const child = <P value={raw |> up} />;\n",
    );
    assert!(attribute.contains("value={$tt_ap(raw, up)}"), "{attribute}");
}

#[test]
fn generator_statement_owner_accepts_match_initializers() {
    let output = ok("function* f(code: number): Generator<string> {\n\
           const line = match (code) { 200 => \"ok\", _ => \"err\" };\n\
           yield line;\n\
         }\n");
    assert!(output.contains("const line = $tt_v0;"), "{output}");
}

#[test]
fn match_arm_return_try_reports_placement_instead_of_panicking() {
    let diagnostics = ttc::analyze(
        "variant R { Ok(value: number), Err(error: string) }\n\
         declare const g: () => R;\n\
         const f = (b: boolean): R => match (b) {\n\
           true => { return try g(); }, false => R.Ok(0),\n\
         };\n",
        &Options::default(),
    );
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].code, ttc::DiagnosticCode::TryPlacement);
}

#[test]
fn nested_match_subject_uses_collision_free_slots() {
    let output = ok("variant V { A, B }\n\
         declare const v: V;\n\
         const n = match (match (v) { A => V.B, B => V.A }) { A => 1, B => 2 };\n");
    assert_eq!(output.matches("switch (").count(), 2, "{output}");
    assert!(
        !output.contains(
            "const $tt_m = v;\n  switch ($tt_m.kind) {\n    case \"A\": {\n      $tt_m ="
        ),
        "{output}"
    );
}

#[test]
fn tuple_match_accepts_comparison_expression_subjects() {
    let output = ok("variant V { A, B }\n\
         declare const a: number; declare const b: number; declare const v: V;\n\
         declare const id: <T>(value: T) => T;\n\
         const n = match (a < b, v) { (_, A) => 1, _ => 0 };\n\
         const m = match (id<number>(0), v) { (_, A) => 1, _ => 0 };\n");
    assert!(output.contains("a < b"), "{output}");
    assert!(output.contains("id<number>(0)"), "{output}");
}

#[test]
fn expression_arms_compose_nested_tt_value_regions() {
    let template = ok("variant V { A(n: number), B }\n\
         declare const v: V; declare const w: V;\n\
         const text = match (v) { A(n) => `${match (w) { A(m) => m, B => n }}`, B => \"\" };\n");
    assert_eq!(template.matches("switch (").count(), 2, "{template}");

    let result = ok(
        "variant V { A(n: number), B } variant R { Ok(value: number), Err(error: string) }\n\
         declare const v: V; declare const g: () => R; declare const unwrap: (r: R) => number;\n\
         const n = match (v) { A(n) => unwrap(result { const x = try g(); return n + x; }), B => -1 };\n",
    );
    assert!(result.contains("kind: \"Ok\""), "{result}");
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
fn ttx_lowers_constructs_in_jsx_children_and_attributes() {
    let source = r#"variant State { Ready(value: string), Empty }
declare const state: State;
const child = <section>{match (state) {
  Ready(value) => <strong>{value}</strong>,
  Empty => <span>empty</span>,
}}</section>;
const prop = <Panel before={mark("before")} render={() => match (state) {
  Ready(value) => <strong>{value}</strong>,
  Empty => null,
}} after={mark("after")} />;
const ordered = (state: State) => <Panel before={mark("first")} value={match (state) {
  Ready(value) => value,
  Empty => "",
}} after={mark("last")} />;
"#;
    let output = ok_tsx(source);
    assert!(output.contains("const child = <section>{"), "{output}");
    assert!(output.contains("<strong>{value}</strong>"), "{output}");
    assert!(output.contains("const prop = <Panel"), "{output}");
    assert_eq!(output.matches("switch ($tt_m.kind)").count(), 3, "{output}");
    let prop_start = output.find("const prop").unwrap();
    let before = output[prop_start..].find("mark(\"before\")").unwrap() + prop_start;
    let decision = output[prop_start..].find("switch ($tt_m.kind)").unwrap() + prop_start;
    let after = output[prop_start..].find("mark(\"after\")").unwrap() + prop_start;
    assert!(before < decision && decision < after, "{output}");
    let ordered_start = output.find("const ordered").unwrap();
    let first = output[ordered_start..].find("mark(\"first\")").unwrap() + ordered_start;
    let ordered_decision =
        output[ordered_start..].find("switch ($tt_m.kind)").unwrap() + ordered_start;
    let last = output[ordered_start..].find("mark(\"last\")").unwrap() + ordered_start;
    assert!(
        first < ordered_decision && ordered_decision < last,
        "{output}"
    );
    assert!(
        output[ordered_start..].contains("return <Panel"),
        "{output}"
    );
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

#[test]
fn ttx_expression_boundaries_ignore_delimiters_inside_regex_literals() {
    let output = ok_tsx(
        r#"variant State { Ready(value: string), Empty }
declare const state: State;
const view = <Panel visible={/}/.test("}")} value={match (state) {
  Ready(value) => value,
  Empty => "",
}} />;
"#,
    );
    assert!(output.contains("(/}/.test(\"}\"))"), "{output}");
    assert!(output.contains("switch ($tt_m.kind)"), "{output}");
}

/* ------------------------------------------------------------------ */
/* variant                                                                */
/* ------------------------------------------------------------------ */

#[test]
fn variant_with_payload_emits_union_type_and_constructors() {
    let out = ok(r#"
variant Shape {
  Circle(radius: number),
  Rect(width: number, height: number),
  Point,
}
"#);
    assert!(out.contains("type Shape ="));
    assert!(out.contains("{ kind: \"Circle\"; radius: number }"));
    assert!(out.contains("{ kind: \"Rect\"; width: number; height: number }"));
    assert!(out.contains("{ kind: \"Point\" }"));
    assert!(out.contains("const Shape = {"));
    assert!(out.contains("Circle: (radius: number): Shape => ({ kind: \"Circle\", radius })"));
    assert!(out.contains("Point: { kind: \"Point\" } as const"));
}

#[test]
fn typescript_enum_is_passthrough() {
    // Every `enum` declaration belongs to TypeScript and
    // must pass through byte for byte.
    let src = "enum Color { Red, Green, Blue }\n";
    assert_eq!(ok(src), src);
    let src = "export enum Color { Red, Green, Blue }\n";
    assert_eq!(ok(src), src);
}

#[test]
fn unit_only_variant_has_tt_semantics_without_parentheses() {
    let out = ok("variant Status { Active, Inactive }\n");
    assert!(out.contains("type Status ="));
    assert!(out.contains("Active: { kind: \"Active\" } as const"));
    assert!(out.contains("Inactive: { kind: \"Inactive\" } as const"));
}

#[test]
fn variant_supports_generics() {
    let out = ok("variant Pair<T> { First, Second }\n");
    assert!(out.contains("type Pair<T> ="));
}

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
fn variant_export_prefix_on_both_declarations() {
    let out = ok("export variant Shape { Circle(radius: number), Point }");
    assert!(out.contains("export type Shape ="));
    assert!(out.contains("export const Shape = {"));
}

#[test]
fn variant_generics_flow_into_constructors() {
    let out = ok("variant Option<T> {\n  Some(value: T),\n  None,\n}\n");
    assert!(out.contains("type Option<T> ="));
    assert!(out.contains("Some: <T>(value: T): Option<T> => ({ kind: \"Some\", value })"));
}

#[test]
fn variant_duplicate_case_is_error_with_position() {
    let e = err("const a = 1;\nvariant X { A(v: number), A }\n");
    assert!(e.message.contains("duplicate case \"A\""), "{}", e.message);
    assert_eq!((e.line, e.col), (2, 27));
}

#[test]
fn variant_complex_field_types() {
    let out = ok(r#"
variant Node {
  Leaf(entries: Map<string, number[]>),
  Branch(children: Array<string>, meta: { tag: string, depth: number }),
}
"#);
    assert!(out.contains("entries: Map<string, number[]>"));
    assert!(out.contains("meta: { tag: string, depth: number }"));
}

#[test]
fn variant_invalid_field_type_is_rejected_by_swc_with_position() {
    let e = err("variant X {\n  A(f: number number),\n}\n");
    assert!(
        e.message.contains("invalid type for field `f`"),
        "{}",
        e.message
    );
    assert_eq!(e.line, 2);
    assert_eq!(e.col, 8); // points at the start of the type annotation
}

#[test]
fn variant_with_unbalanced_field_type_is_a_field_type_error() {
    let e = err("variant E { A(value: number]) }\n");
    assert!(e.message.contains("invalid type for field `value`"), "{e}");
    assert_eq!((e.line, e.col), (1, 22));
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
fn match_compiles_to_switch_with_runtime_guard_only() {
    let out = ok(r#"
variant Shape { Circle(radius: number), Point }
const area = match (shape) {
  Circle(radius) => 3.14 * radius * radius,
  Point => 0,
};
"#);
    assert!(out.contains("switch ($tt_m.kind)"));
    let compact = compact(&out);
    assert!(compact.contains(
        "case \"Circle\": { const { radius } = $tt_m; $tt_v0 = 3.14 * radius * radius; break; }"
    ));
    assert!(compact.contains("case \"Point\": { $tt_v0 = 0; break; }"));
    // The output is plain TypeScript: a runtime guard, no type-level tricks.
    assert!(compact.contains(
        "default: { throw new Error(\"tt match: unexpected case \" + JSON.stringify($tt_m)); }"
    ));
    assert!(!out.contains("never"));
}

#[test]
fn match_wildcard_becomes_default() {
    let out = ok("const r = match (x) { A => 1, _ => 0 };");
    assert!(compact(&out).contains("default: { $tt_v0 = 0; break; }"));
    assert!(!out.contains("never"));
}

#[test]
fn whole_initializer_match_uses_a_statement_slot_without_an_iife() {
    let out = ok("const r = match (x) { A => 1, _ => 0 };\n");
    assert!(!out.contains("(() =>"), "{out}");
    assert!(out.contains("let $tt_v0;"), "{out}");
    assert!(out.contains("$tt_v0 = 1;"), "{out}");
    assert!(out.contains("const r = $tt_v0;"), "{out}");
}

#[test]
fn expression_bodied_arrow_match_becomes_a_block_without_an_iife() {
    let out = ok("variant E { A, B }\nconst f = (e: E) => match (e) { A => 1, B => 2 };\n");
    assert!(!out.contains("(() =>"), "{out}");
    assert!(
        out.contains("const f = (e: E) => {\n  let $tt_v0;"),
        "{out}"
    );
    assert!(out.contains("  return $tt_v0;\n};"), "{out}");
}

#[test]
fn nested_initializer_match_inherits_the_parent_assignment_continuation() {
    let out = ok(
        "variant Outer { A, B }\nenum Inner { X, Y }\nconst value = match (outer) { A => match (inner) { X => 1, Y => 2 }, B => 0 };\n",
    );
    assert!(!out.contains("(() =>"), "{out}");
    assert!(out.matches("switch (").count() >= 2, "{out}");
}

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
fn is_patterns_lower_to_host_owned_instanceof_control_flow() {
    let out = ok("const msg = match (err) {\n\
           is SyntaxError { message } if message.length > 0 => `syntax: ${message}`,\n\
           is RangeError | is TypeError => \"bad value\",\n\
           is Error { message: detail } => detail,\n\
           _ => String(err),\n\
         };\n");
    assert!(!out.contains("(() =>"), "{out}");
    assert!(!out.contains("$tt_expr"), "{out}");
    assert!(out.contains("$tt_m instanceof SyntaxError"), "{out}");
    assert!(
        out.contains("$tt_m instanceof RangeError || $tt_m instanceof TypeError"),
        "{out}"
    );
    assert!(out.contains("const { message } = $tt_m;"), "{out}");
    assert!(out.contains("const { message: detail } = $tt_m;"), "{out}");
    assert!(out.contains("const msg = $tt_v0;"), "{out}");
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

#[test]
fn is_patterns_may_share_an_ordered_chain_with_literals() {
    let out = ok(
        "const value = match (x) { is Error => \"error\", \"ok\" => \"ok\", _ => \"other\" };\n",
    );
    assert!(out.contains("$tt_m instanceof Error"), "{out}");
    assert!(out.contains("$tt_m === \"ok\""), "{out}");
}

#[test]
fn one_shot_for_headers_hoist_match_control_flow_once() {
    let for_of =
        ok("for (const value of match (source) { is Array => source, _ => [] }) { use(value); }\n");
    assert!(!for_of.contains("$tt_expr"), "{for_of}");
    assert!(for_of.find("instanceof Array").unwrap() < for_of.find("for (").unwrap());
    assert!(for_of.contains("for (const value of $tt_v0)"), "{for_of}");

    let initializer = ok(
        "for (let value = match (source) { is Number => 1, _ => 0 }; value < 2; value++) { use(value); }\n",
    );
    assert!(!initializer.contains("$tt_expr"), "{initializer}");
    assert!(initializer.find("instanceof Number").unwrap() < initializer.find("for (").unwrap());
    assert!(
        initializer.contains("for (let value = $tt_v0;"),
        "{initializer}"
    );
}

#[test]
fn repeated_loop_tests_own_the_match_region_per_iteration() {
    let while_output = ok("while (match (next()) { is Error => false, _ => true }) { work(); }\n");
    assert!(!while_output.contains("$tt_expr"), "{while_output}");
    assert!(while_output.contains("while (true)"), "{while_output}");
    assert!(
        while_output.find("const $tt_m = next()").unwrap()
            > while_output.find("while (true)").unwrap(),
        "{while_output}"
    );

    let for_output =
        ok("for (; match (next()) { is Error => false, _ => true }; tick()) { work(); }\n");
    assert!(!for_output.contains("$tt_expr"), "{for_output}");
    assert!(for_output.contains("for (; ; tick())"), "{for_output}");
    assert!(
        for_output.find("const $tt_m = next()").unwrap()
            > for_output.find("for (; ; tick())").unwrap(),
        "{for_output}"
    );
}

#[test]
fn loop_test_rewrites_compose_with_conditionals_nesting_and_initializers() {
    let conditional = ok(
        "declare const flag: boolean; declare function next(): unknown;\nwhile (flag && match (next()) { is Error => false, _ => true }) { work(); }\n",
    );
    assert!(
        conditional.contains("if (!($tt_v2)) break;"),
        "{conditional}"
    );
    assert!(!conditional.contains("$tt_expr"), "{conditional}");

    let nested = ok(
        "declare function a(): number; declare function b(): number;\nwhile (match (a()) { 1 => true, _ => false }) { while (match (b()) { 2 => true, _ => false }) { work(); } }\n",
    );
    assert_eq!(nested.matches("while (true)").count(), 2, "{nested}");

    let logical = ok(
        "declare function a(): number; declare function b(): number;\nwhile (match (a()) { 1 => true, _ => false } || match (b()) { 2 => true, _ => false }) { work(); }\n",
    );
    assert!(logical.contains("if (!($tt_v2)) break;"), "{logical}");
    assert!(!logical.contains("|| ))"), "{logical}");

    let both = ok(
        "for (let a = match (1) { 1 => 1, _ => 0 }; match (a) { 1 => true, _ => false }; a++) { use(a); }\n",
    );
    assert!(!both.contains("$tt_expr"), "{both}");
    assert!(both.contains("for (let a = $tt_v0; ; a++)"), "{both}");
}
