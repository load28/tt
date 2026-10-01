/* ------------------------------------------------------------------ */
/* result computation block                                            */
/* ------------------------------------------------------------------ */

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

/* ------------------------------------------------------------------ */
/* literal match patterns                                             */
/* ------------------------------------------------------------------ */

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
    assert!(
        error
            .message
            .contains("a literal pattern cannot be a tuple pattern element"),
        "{}",
        error.message
    );
    assert_eq!((error.line, error.col), (1, 27));
}

/* ------------------------------------------------------------------ */
/* val — binding modifier                                             */
/* ------------------------------------------------------------------ */

#[test]
fn val_array_pattern_parameter_is_erased_in_every_parameter_list() {
    let cases = [
        (
            "function a(val [x, y]: number[]) { return x + y; }\n",
            "function a([x, y]: number[]) { return x + y; }\n",
        ),
        (
            "const b = (val [x]: number[]) => x;\n",
            "const b = ([x]: number[]) => x;\n",
        ),
        (
            "const c = async (p: number, val [x]: number[]): Promise<number> => x;\n",
            "const c = async (p: number, [x]: number[]): Promise<number> => x;\n",
        ),
        (
            "const d = function* <T>(val [t]: T[]) { yield t; };\n",
            "const d = function* <T>([t]: T[]) { yield t; };\n",
        ),
        (
            "class K {\n  m(val [h]: number[]): { k: number } { return { k: h }; }\n  n(val [h]: number[])\n  {\n    return h;\n  }\n}\n",
            "class K {\n  m([h]: number[]): { k: number } { return { k: h }; }\n  n([h]: number[])\n  {\n    return h;\n  }\n}\n",
        ),
        (
            "const o = { m(val [u]: number[]) { return u; } };\n",
            "const o = { m([u]: number[]) { return u; } };\n",
        ),
        (
            "try { f(); } catch (val [e]: any) { g(e); }\n",
            "try { f(); } catch ([e]: any) { g(e); }\n",
        ),
        (
            "function over(val [x]: number[]): void;\nfunction over(val [x]: number[]) {}\n",
            "function over([x]: number[]): void;\nfunction over([x]: number[]) {}\n",
        ),
        (
            "const e = c ? (val [x]: number[]) => x : (w: number[]) => w;\n",
            "const e = c ? ([x]: number[]) => x : (w: number[]) => w;\n",
        ),
        (
            "class S { set s(val [v]: number[]) {} get g() { return 1; } }\n",
            "class S { set s([v]: number[]) {} get g() { return 1; } }\n",
        ),
        (
            "interface I { m(val [x]: number[]): void; new (val y: number): I; (val z: number): void }\ntype F = (val [x]: number[]) => void;\n",
            "interface I { m([x]: number[]): void; new (y: number): I; (z: number): void }\ntype F = ([x]: number[]) => void;\n",
        ),
        (
            "const t = `${(val [x]: number[]) => x}`;\n",
            "const t = `${([x]: number[]) => x}`;\n",
        ),
    ];
    for (src, expected) in cases {
        assert_eq!(ok(src), expected, "{src}");
    }
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
fn val_resolves_hoisted_declarations_to_their_scope() {
    for src in [
        "val const o = { x: 1 };\nfunction w() { o.x = 2; var o = { x: 1 }; }\n",
        "val const o = { x: 1 };\nfunction w() { { var o = { x: 1 }; } o.x = 2; }\n",
        "val const o = { x: 1 };\nfunction w() { for (var o of [{ x: 1 }]) {} o.x = 2; }\n",
        "val const o = { x: 1 };\nconst w = [1].map(n => { o.x = n; var o = { x: 1 }; return o; });\n",
        "val const o = { x: 1 };\nfunction w() { o.x = 2; function o() {} }\n",
        "val const o = { x: 1 };\nfunction w() { { o.x = 2; function o() {} } }\n",
    ] {
        assert_eq!(ok(src), src.replacen("val ", "", 1), "{src}");
    }
    for (src, at) in [
        ("val const o = { x: 1 };\nfunction w() { function g() { var o = { x: 1 }; } o.x = 2; }\n", (2, 51)),
        ("val const o = { x: 1 };\nfunction w() { try {} catch (e) { var o = { x: 1 }; } }\no.x = 2;\n", (3, 1)),
        ("val const o = { x: 1 };\nclass C { static { var o = { x: 1 }; } m() { o.x = 2; } }\n", (2, 46)),
        ("val const o = { x: 1 };\nfunction w() { { function o() {} } o.x = 2; }\n", (2, 36)),
        ("function w() { p.x = 1; { val var p = { x: 1 }; } }\n", (1, 16)),
        ("variant V { A(n: number), B }\nval const o = { x: 1 };\nfunction w(v: V) {\n  match (v) {\n    A(n) => { const h = () => { var o = { x: n }; }; },\n    B => {},\n  }\n  o.x = 2;\n}\n", (8, 3)),
    ] {
        let e = err(src);
        assert_eq!((e.line, e.col), at, "{src}");
        assert!(e.message.contains("cannot mutate through val binding"), "{src}: {}", e.message);
    }
    ok("variant V { A(n: number), B }\nval const o = { x: 1 };\nfunction w(v: V) {\n  match (v) {\n    A(n) => { var o = { x: n }; },\n    B => {},\n  }\n  o.x = 2;\n}\n");
}

#[test]
fn val_binds_a_function_or_class_expression_name_only_inside_itself() {
    for (src, at) in [
        ("val const s = { a: 1 };\nconst g = function s() {};\ns.a = 2;\n", (3, 1)),
        ("val const s = { a: 1 };\nconst g = function* s() {};\ns.a = 2;\n", (3, 1)),
        ("val const s = { a: 1 };\nconst g = async function s() {};\ns.a = 2;\n", (3, 1)),
        ("val const s = { a: 1 };\nconst K = class s {};\ns.a = 2;\n", (3, 1)),
        ("val const s = { a: 1 };\nuse(class s extends Base<{ a: 1 }> { m() {} });\ns.a = 2;\n", (3, 1)),
        ("function f(val p: { a: number }) { const cb = function p() {}; p.a = 1; }\n", (1, 64)),
        ("val const s = { a: 1 };\nconst g = function s(s: number) { return s; };\ns.a = 2;\n", (3, 1)),
    ] {
        let e = err(src);
        assert_eq!((e.line, e.col), at, "{src}");
        assert!(e.message.contains("cannot mutate through val binding"), "{src}: {}", e.message);
    }
    for src in [
        "val const s = { a: 1 };\nconst g = function s() { s.a = 2; };\n",
        "val const s = { a: 1 };\nconst g = function* s<T>(t: T) { s.a = 2; };\n",
        "val const s = { a: 1 };\nconst K = class s { static m() { s.a = 2; } };\n",
        "val const s = { a: 1 };\nfunction w() { class s {} s.a = 2; }\n",
        "val const s = { a: 1 };\nfunction w() { s.a = 2\nfunction s() {} }\n",
    ] {
        assert_eq!(ok(src), src.replacen("val ", "", 1), "{src}");
    }
}
