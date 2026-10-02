/// Parenthesized return types of arrow functions: the `=>` after each
/// belongs to the arrow function, because TypeScript's
/// `isUnambiguouslyStartOfFunctionType` does not read the `(` as a function
/// type's parameters (TASK-499).
const TASK_499_PARENTHESIZED: &[&str] = &[
    "(A | B)",
    "(void)",
    "(() => void)",
    "(\"a\" | \"b\")",
    "(A[])",
    "(typeof x)",
    "(keyof A)",
    "(readonly A[])",
    "([\"a\"])",
];

const TASK_499_PRELUDE: &str = "type A = { a: 1 };\ntype B = 2;\ndeclare const x: number;\n\
                                declare const o: { m(v: number): number };\n";

#[test]
fn an_arrow_body_after_a_parenthesized_return_type_is_a_body() {
    for ty in TASK_499_PARENTHESIZED {
        let source = format!(
            "{TASK_499_PRELUDE}export const f = (s: number): {ty} => {{\n  const n = s |> String\n  return n as any\n}}\n"
        );
        let out = ok(&source);
        assert!(
            out.contains("const n = (($tt_v, $tt_f) => $tt_f($tt_v))(s, String)\n"),
            "{ty}:\n{out}"
        );
    }
    let source = format!(
        "{TASK_499_PRELUDE}export const f = (): (void) => {{ const v = 1\n v |> o.m; }}\n"
    );
    let out = ok(&source);
    assert!(out.contains("const v = 1\n ;(("), "{out}");
}

#[test]
fn a_statement_after_an_arrow_with_a_parenthesized_return_type_keeps_its_text() {
    for ty in TASK_499_PARENTHESIZED {
        let source = format!(
            "{TASK_499_PRELUDE}export const f = (): {ty} => {{ return null as any }}\n/ val const q = 2 /.test(\"\")\n"
        );
        assert_eq!(ok(&source), source, "{ty}");
        let source = format!(
            "{TASK_499_PRELUDE}export const f = (): {ty} => {{ return null as any }}\n<b> val const q = 2 </b>\n"
        );
        assert_eq!(ok_tsx(&source), source, "{ty}");
    }
}

#[test]
fn a_function_return_type_still_continues_to_its_own_arrow() {
    for ty in [
        "() => void",
        "(...a: A[]) => void",
        "(a: A) => void",
        "(a, b) => void",
        "(a?: A) => void",
        "(a) => void",
        "(this: A) => void",
        "({ a }: A) => void",
        "([p, q = 1]: A[]) => void",
        "(public a: A) => void",
        "(readonly: A) => void",
    ] {
        let source = format!(
            "{TASK_499_PRELUDE}export const f = (): {ty} => {{\n  return () => {{}}\n}}\n/ val const q = 2 /.test(\"\")\n"
        );
        assert_eq!(ok(&source), source, "{ty}");
    }
}

/// Import-equals declarations: the module reference is `require("…")` or
/// an entity name, and a line break after it ends the declaration
/// (TypeScript's `parseModuleReference` and `parseSemicolon`, TASK-500).
const TASK_500_DECLARATIONS: &[&str] = &[
    "import fs = require(\"fs\")\n",
    "import type R = require(\"fs\")\n",
    "export import F = require(\"fs\")\n",
    "import A = B.C\n",
    "export import D = B.\n  C\n",
    "import E = B\n",
];

const TASK_500_PRELUDE: &str = "namespace B { export namespace C { export const q = 1 } }\n";

#[test]
fn a_line_after_an_import_equals_declaration_starts_a_statement() {
    for declaration in TASK_500_DECLARATIONS {
        let source = format!("{TASK_500_PRELUDE}{declaration}/a|>b/.test(\"a|>b\") && B\n");
        assert_eq!(ok(&source), source, "{declaration:?}");
        let source = format!("{TASK_500_PRELUDE}{declaration}/ val const q = 2 /.test(\"\")\n");
        assert_eq!(ok(&source), source, "{declaration:?}");
        let source = format!("{TASK_500_PRELUDE}{declaration}<p> val const text </p>\n");
        assert_eq!(ok_tsx(&source), source, "{declaration:?}");
        let source = format!("{TASK_500_PRELUDE}{declaration}[1] |> console.log\n");
        let out = ok(&source);
        assert!(
            out.contains(&format!("{declaration}console.log([1])")),
            "{declaration:?}:\n{out}"
        );
    }
}

/// Contextual words at the start of a type or a statement that TypeScript
/// reads as a prefix or modifier only under a lookahead condition; here each
/// is a name, so the line after it begins a statement (TASK-503).
const TASK_503_NAMES: &[&str] = &[
    "type asserts = number;\nexport let a: asserts\n",
    "type abstract = number;\nexport type A = abstract\n",
    "declare let namespace: any;\nnamespace instanceof Object;\n",
    "declare let module: any;\nmodule instanceof Object;\n",
    "declare let declare: any;\ndeclare instanceof Object;\n",
];

/// The same words where TypeScript does read them as prefixes.
const TASK_503_PREFIXES: &[&str] = &[
    "export function g(v: unknown): asserts v is string {}\n",
    "export function h(this: unknown): asserts this {}\n",
    "export type C = abstract new () => object\n",
    "export let k: new <T>(x: T) => T\n",
    "export type G = <T>(x: T) => T\n",
];

#[test]
fn a_contextual_type_or_statement_word_is_a_name_unless_typescript_reads_a_prefix() {
    for head in TASK_503_NAMES.iter().chain(TASK_503_PREFIXES) {
        let source = format!("{head}<p> val const text </p>\n");
        assert_eq!(ok_tsx(&source), source, "{head:?}");
        let source = format!("{head}/ val const q = 2 /.test(\"\")\n");
        assert_eq!(ok(&source), source, "{head:?}");
        let source = format!("{head}[1] |> console.log\n");
        let out = ok(&source);
        assert!(
            out.contains(&format!("{head}console.log([1])")),
            "{head:?}:\n{out}"
        );
    }
}

/* ------------------------------------------------------------------ */
/* TASK-501 hoisted values inside pipeline operands                     */
/* ------------------------------------------------------------------ */

const TASK_501_PRELUDE: &str = "declare function f(a: any, b?: any): any;\n\
declare function g(): any;\n\
declare class C { constructor(a: any); }\n\
declare const n: number;\n\
type R = { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\n\
declare function r(): R;\n";

const TASK_501_OPERANDS: &[&str] = &[
    "f(@)",
    "@ + 1",
    "1 + @",
    "[@]",
    "-@",
    "(@).toFixed()",
    "(f(@))",
    "f(g(), @)",
    "`a${@}b`",
    "f(`${f(@)}`)",
    "[@, @]",
    "f(@) + f(@)",
    "({ k: @ })",
    "new C(@)",
    "(g() && f(@))",
    "(g() ?? f(@))",
    "(g() ? f(@) : 0)",
    "(1 |> f(@))",
];

const TASK_501_POSITIONS: &[&str] = &[
    "# |> String",
    "1 |> #",
    "1 |> String |> # |> String",
    "String(# |> String)",
    "# |> # |> String",
];

#[test]
fn a_hoisted_value_in_any_pipeline_operand_is_emitted_once() {
    let values = [
        ("match (n) { 0 => 1, _ => 2 }", "switch ("),
        ("(try r())", "\"value\" in"),
    ];
    for (value, region) in values {
        for position in TASK_501_POSITIONS {
            for operand in TASK_501_OPERANDS {
                let expression = position.replace('#', operand).replace('@', value);
                let source = if value.contains("try") {
                    format!(
                        "{TASK_501_PRELUDE}export function h(): R {{\n  const v = {expression};\n  return {{ kind: \"Ok\", value: v }};\n}}\n"
                    )
                } else {
                    format!("{TASK_501_PRELUDE}export const v = {expression};\n")
                };
                let diagnostics = ttc::analyze(&source, &Options::default());
                assert!(diagnostics.is_empty(), "{source}\n{diagnostics:?}");
                let out = ok(&source);
                assert_eq!(
                    out.matches(region).count(),
                    expression.matches(value).count(),
                    "{source}\n{out}"
                );
            }
        }
    }
}

#[test]
fn an_unstructurable_conditional_in_a_pipeline_operand_is_a_placement_diagnostic() {
    let source = format!(
        "{TASK_501_PRELUDE}export const v = g() && (g() && f(match (n) {{ 0 => 1, _ => 2 }})) |> String;\n"
    );
    let diagnostics = ttc::analyze(&source, &Options::default());
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        diagnostics[0].code,
        DiagnosticCode::MatchPlacement,
        "{diagnostics:#?}"
    );
    assert!(compile(&source, &Options::default()).is_err());
}

const TASK_504_PRELUDE: &str = "declare const x: any;\n\
declare function g(): any;\n\
declare const n: number;\n\
type R = { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\n\
declare function r(): R;\n";

const TASK_504_STEPS: &[&str] = &[
    "x |> .m(@)",
    "x |> ?.m(@)",
    "g() |> .m(@)",
    "x |> .a(1).m(@)",
    "x |> .a(@).m(@)",
    "x |> .list[@]",
    "x |> .m(@) |> .k(@)",
    "x |> String |> .m(1, @) |> String",
    "x |> .m(`${@}`)",
    "x |> x.m(@)",
];

#[test]
fn a_hoisted_value_in_a_member_step_is_emitted_once_after_its_method() {
    let values = [
        ("match (n) { 0 => 1, _ => 2 }", "switch ("),
        ("(try r())", "\"value\" in"),
    ];
    for (value, region) in values {
        for step in TASK_504_STEPS {
            let expression = step.replace('@', value);
            let source = if value.contains("try") {
                format!(
                    "{TASK_504_PRELUDE}export function h(): R {{\n  const v = {expression};\n  return {{ kind: \"Ok\", value: v }};\n}}\n"
                )
            } else {
                format!("{TASK_504_PRELUDE}export const v = {expression};\n")
            };
            let diagnostics = ttc::analyze(&source, &Options::default());
            assert!(diagnostics.is_empty(), "{source}\n{diagnostics:?}");
            let out = ok(&source);
            assert_eq!(
                out.matches(region).count(),
                expression.matches(value).count(),
                "{source}\n{out}"
            );
            assert!(!out.contains("|>"), "{source}\n{out}");
        }
    }
}

#[test]
fn a_try_in_a_template_in_a_pipeline_operand_keeps_its_callee_before_it() {
    for expression in [
        "`${f(try r())}` |> String",
        "1 |> f(`${f(try r())}`)",
        "f(`a${f(try r())}b`) |> String",
        "`${`${f(try r())}`}` |> String",
    ] {
        let source = format!(
            "{TASK_501_PRELUDE}export function h(): R {{\n  const v = {expression};\n  return {{ kind: \"Ok\", value: v }};\n}}\n"
        );
        let diagnostics = ttc::analyze(&source, &Options::default());
        assert!(diagnostics.is_empty(), "{source}\n{diagnostics:?}");
        let out = ok(&source);
        let callee = out.rfind("= (f);").expect("the callee is captured");
        let region = out.find("= r();").expect("the try follows");
        assert!(callee < region, "{source}\n{out}");
    }
}

#[test]
fn a_statement_match_that_ends_the_file_closes_the_block_it_hoists_into() {
    let prelude = "declare const x: { kind: \"A\" };\n";
    for (statement, head) in [
        ("match (x) { A => 1 }", "{"),
        ("if (x) match (x) { _ => 1 }", "if (x) {"),
        ("lbl: match (x) { _ => 1 }", "lbl: {"),
        ("while (x) match (x) { _ => 1 }", "while (x) {"),
        ("if (x) 0;\nelse match (x) { _ => 1 }", "else {"),
        ("match (x) { _ => 1 }\nmatch (x) { _ => 2 }", "{"),
    ] {
        for end in ["", "\n"] {
            let source = format!("{prelude}{statement}{end}");
            let out = ok(&source);
            let block = &out[out.rfind(head).expect("the owner opens its block")..];
            assert!(block.trim_end().ends_with("}\n  }\n}"), "{source:?}\n{out}");
        }
    }
}

#[test]
fn val_sees_a_mutation_through_every_typescript_wrapper() {
    let prelude = "val const cfg: { a?: number } = { a: 1 };\n";
    for mutation in [
        "cfg!.a = 1;",
        "(cfg as { a?: number }).a = 1;",
        "(cfg satisfies { a?: number }).a = 1;",
        "(<{ a?: number }>cfg).a = 1;",
        "((cfg as any)).a = 1;",
        "(cfg as any as { a: number }).a = 1;",
        "(cfg as any).a++;",
        "--(cfg as any).a;",
        "delete (cfg as any).a;",
        "[(cfg as any).a] = [1];",
        "({ k: (cfg as any).a } = { k: 1 });",
        "for ((cfg as any).a of [1]);",
    ] {
        let source = format!("{prelude}{mutation}\n");
        let root = prelude.len() + mutation.find("cfg").expect("the root is written");
        let diagnostics = ttc::analyze(&source, &Options::default());
        let mutations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.code == DiagnosticCode::ValMutation)
            .collect();
        assert_eq!(mutations.len(), 1, "{source}\n{diagnostics:#?}");
        let probes = ttc::val_probes(&source);
        assert!(
            probes.mutations.iter().any(|m| m.root == root && m.method.is_none()),
            "{source}\n{probes:#?}"
        );
    }
}

#[test]
fn a_write_to_a_call_result_is_not_a_write_to_its_argument() {
    let diagnostics = ttc::analyze(
        "declare function f(val v: unknown): { y: number };\n\
         val const cfg = { a: 1 };\n\
         f(cfg).y = 1;\n",
        &Options::default(),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let probes = ttc::val_probes(
        "declare function f(val v: unknown): { y: number };\nval const cfg = { a: 1 };\nf(cfg).y = 1;\n",
    );
    assert!(probes.mutations.is_empty(), "{probes:#?}");
}

/// Subjects of a let-else or `if let` that contain a tt value below their
/// top level (TASK-544). Each is lowered like a `match` subject: the value
/// runs behind the captures the subject's own evaluation takes, and the
/// subject is delivered once into the decision's temporary.
const TASK_544_PRELUDE: &str = "declare function opt(n: number): { kind: \"Some\"; value: number } | { kind: \"None\" };\n\
     declare function r(): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\n\
     declare function ro(): { kind: \"Ok\"; value: { kind: \"Some\"; value: number } | { kind: \"None\" } } | { kind: \"Err\"; error: string };\n";

#[test]
fn a_value_inside_a_let_else_or_if_let_subject_is_lowered_once() {
    let subjects = [
        ("opt(match (k) { 1 => 1, _ => 2 })", true),
        ("opt(try r())", true),
        ("(try ro())", false),
        ("[match (k) { 1 => opt(1), _ => opt(2) }][0]", false),
        ("match (k) { 1 => opt(1), _ => opt(2) }", false),
    ];
    for (subject, captures_callee) in subjects {
        for body in [
            format!("const Some(value: v) = {subject} else {{ return 0; }};\n  return v;"),
            format!("if let Some(value: v) = {subject} {{ return v; }}\n  return 0;"),
            format!(
                "if let Some(value: z) = opt(0) {{ return z; }}\n  else if let Some(value: v) = {subject} {{ return v; }}\n  return 1;"
            ),
        ] {
            for source in [
                format!("{TASK_544_PRELUDE}export function g(k: 1 | 2) {{\n  {body}\n}}\n"),
                format!(
                    "{TASK_544_PRELUDE}export function g(k: 1 | 2) {{\n  return result {{\n  const n = try r();\n  {body}\n  }};\n}}\n"
                ),
            ] {
                let out = ok(&source);
                assert!(
                    !out.contains("if let") && !out.contains("Some(value"),
                    "{source}\n{out}"
                );
                assert_eq!(
                    out.contains(" = (opt);"),
                    captures_callee,
                    "{source}\n{out}"
                );
            }
        }
    }
}

/// An optional call is skipped at the first `?.` of its chain whose base is
/// nullish (TASK-545): the call's own `?.(` tests the callee, the callee's
/// `?.name` tests the receiver, and a `?.` further inside the callee cannot
/// be tested from the call's captured inputs.
#[test]
fn an_optional_call_tests_the_link_its_chain_is_skipped_at() {
    let prelude = "declare const o: { m(v: number): number } | null;\n\
                   declare const a: { b: { m(v: number): number } } | null;\n\
                   declare const f: (() => (v: number) => number) | null;\n";
    for (call, test) in [
        ("o?.m", "if (o != null) {"),
        ("o?.[\"m\"]", "if (o != null) {"),
        ("o?.m?.", "if ($tt_v1 != null) {"),
        ("f?.()?.", "if ($tt_v1 != null) {"),
    ] {
        let out = ok(&format!(
            "{prelude}export function g() {{ return {call}(match (1) {{ _ => 1 }}); }}\n"
        ));
        assert!(out.contains(test), "{call}\n{out}");
    }
    let out = ok(&format!(
        "{prelude}export function g() {{ return o?.m(match (1) {{ _ => 1 }}); }}\n"
    ));
    assert!(compact(&out).contains("if (o != null) {"), "{out}");
    assert!(out.contains("= o?.m(1);"), "{out}");
    for call in ["a?.b.m", "a?.b.m?.", "f?.()"] {
        let diagnostics = ttc::analyze(
            &format!("{prelude}export function g() {{ return {call}(match (1) {{ _ => 1 }}); }}\n"),
            &Options::default(),
        );
        assert_eq!(
            diagnostics.iter().map(|d| d.code).collect::<Vec<_>>(),
            [DiagnosticCode::MatchPlacement],
            "{call}: {diagnostics:#?}"
        );
    }
}

#[test]
fn a_statement_value_in_a_later_loop_head_declarator_is_a_placement_error() {
    let prelude = "import * as Result from \"@tt/std/result\";\n\
                   import type { TResult } from \"@tt/std\";\n\
                   variant O { A(n: number), B }\n\
                   declare const o: O;\n\
                   declare function r(n: number): TResult<number, string>;\n";
    for (head, code) in [
        (
            "let i = 0, j = match (o) { A(n) => i + n, B => 0 }",
            DiagnosticCode::MatchPlacement,
        ),
        (
            "var i = 0, j = match (o) { A(n) => i + n, B => 0 }",
            DiagnosticCode::MatchPlacement,
        ),
        (
            "const i = 0, j = [match (o) { A(n) => i + n, B => 0 }]",
            DiagnosticCode::MatchPlacement,
        ),
        ("let i = 0, j = try r(i)", DiagnosticCode::TryPlacement),
    ] {
        let diagnostics = ttc::analyze(
            &format!(
                "{prelude}export function f(): TResult<number, string> {{\n  for ({head}; i < 1; ) break;\n  return Result.Ok(0);\n}}\n"
            ),
            &Options::default(),
        );
        assert_eq!(
            diagnostics.iter().map(|d| d.code).collect::<Vec<_>>(),
            [code],
            "{head}: {diagnostics:#?}"
        );
        assert!(
            diagnostics[0]
                .message
                .contains("later declarator of a `for` loop head"),
            "{head}: {diagnostics:#?}"
        );
    }
    let out = ok(&format!(
        "{prelude}export function f() {{\n  for (let i = 0, j = result {{ const x = try r(i); return x; }}; i < 1; ) return j;\n  for (let k = match (o) {{ A(n) => n, B => 0 }}, m = 1; k < m; ) return k;\n}}\n"
    ));
    assert!(out.contains("for (let i = 0, j = $tt_expr(() => {"), "{out}");
    assert!(out.contains("for (let k = $tt_v1, m = 1; k < m; )"), "{out}");
}

#[test]
fn a_statement_value_in_an_enum_member_initializer_is_a_placement_error() {
    let prelude = "import * as Result from \"@tt/std/result\";\n\
                   import type { TResult } from \"@tt/std\";\n\
                   variant O { A(n: number), B }\n\
                   declare const o: O;\n\
                   declare function r(n: number): TResult<number, string>;\n\
                   const P = 100;\n";
    for (member, code) in [
        (
            "Q = match (o) { A(n) => P + n, B => 0 }",
            DiagnosticCode::MatchPlacement,
        ),
        (
            "Q = 1 + match (o) { A(n) => P + n, B => 0 }",
            DiagnosticCode::MatchPlacement,
        ),
        ("Q = try r(P)", DiagnosticCode::TryPlacement),
    ] {
        let diagnostics = ttc::analyze(
            &format!(
                "{prelude}export function f(): TResult<number, string> {{\n  enum F {{ P = 7, {member} }}\n  return Result.Ok(F.Q);\n}}\n"
            ),
            &Options::default(),
        );
        assert_eq!(
            diagnostics.iter().map(|d| d.code).collect::<Vec<_>>(),
            [code],
            "{member}: {diagnostics:#?}"
        );
        assert!(
            diagnostics[0].message.contains("enum member initializer"),
            "{member}: {diagnostics:#?}"
        );
    }
    let out = ok(&format!(
        "{prelude}enum F {{ P = 7, Q = (result {{ const x = try r(P); return x; }}).kind === \"Ok\" ? 1 : 2, R = [0].map(() => match (o) {{ A(n) => n, B => 0 }})[0]! }}\n"
    ));
    assert!(
        out.contains("enum F { P = 7, Q = ($tt_expr(() => {"),
        "{out}"
    );
    assert!(
        compact(&out).contains("R = [0].map(() => { let $tt_v"),
        "{out}"
    );
}

#[test]
fn an_assertion_operand_takes_no_storage_type_from_outside_the_assertion() {
    let prelude = "variant O { A, B }\ndeclare const o: O;\ntype Ev = { kind: \"a\" } | { kind: \"b\" };\n";
    let syntactic = |src: &str| {
        compile(
            &format!("{prelude}{src}"),
            &Options {
                defer_to_checker: true,
                ..Options::default()
            },
        )
        .expect("compile failed")
    };
    let cases: &[(&str, &str)] = &[
        (
            "export const q: string = match (o) { A => 1, B => 2 } as unknown as string;\n",
            "let $tt_v0;",
        ),
        (
            "export function h(): number { return match (o) { A => 1, B => 2 } as any; }\n",
            "let $tt_v0;",
        ),
        (
            "export const n: number = <number>match (o) { A => 1, B => 2 };\n",
            "let $tt_v0;",
        ),
        (
            "export const e = match (o) { A => ({ kind: \"a\" }), B => ({ kind: \"b\" }) } satisfies Ev;\n",
            "let $tt_v0: Ev;",
        ),
        (
            "export function r(): unknown { return match (o) { A => ({ kind: \"a\" }), B => ({ kind: \"b\" }) } satisfies Ev; }\n",
            "let $tt_v0: Ev;",
        ),
        (
            "export const k: number = match (o) { A => 1, B => 2 };\n",
            "let $tt_v0: number;",
        ),
    ];
    for (src, declaration) in cases {
        let out = syntactic(src);
        assert!(out.contains(declaration), "{src}: {out}");
    }
}

#[test]
fn a_try_in_a_template_interpolation_claims_its_result_block() {
    let prelude = "declare const r: { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\nvariant O { A, B }\ndeclare const o: O;\n";
    for body in [
        "return `x${try r}`;",
        "return `x${`y${try r}`}`;",
        "return `x${match (o) { A => try r, B => 0 }}`;",
        "const v = `${(() => 1)()}${try r}`; return v;",
    ] {
        let source = format!("{prelude}export const b = result {{ {body} }};\n");
        let diagnostics = ttc::analyze(&source, &Options::default());
        assert_eq!(diagnostics.len(), 1, "{body}: {diagnostics:#?}");
        assert_eq!(
            diagnostics[0].code,
            DiagnosticCode::TryCrossesValueRegion,
            "{body}: {diagnostics:#?}"
        );
        assert_eq!(
            diagnostics[0].start,
            Some(source.rfind("try").unwrap()),
            "{body}: {diagnostics:#?}"
        );
    }
    let out = ok(&format!(
        "{prelude}export const f = result {{ const g = () => `${{(() => 1)()}}`; const v = try r; return `${{g()}}${{v}}`; }};\n"
    ));
    assert!(!out.contains("result {"), "{out}");
    assert!(out.contains("$tt_t0.value"), "{out}");
}

#[test]
fn a_for_head_initializer_that_reads_a_head_binding_is_a_placement_error() {
    let prelude = "variant O { A(n: number), B }\ndeclare const o: O;\ntype R = { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\ndeclare function r(): R;\ndeclare function id<T>(v: T): T;\n";
    let rejected: &[(&str, DiagnosticCode)] = &[
        (
            "for (let g = match (o) { A(n) => () => g, B => null }; g !== null; g = null) {}",
            DiagnosticCode::MatchPlacement,
        ),
        (
            "for (let h = id(() => h) && match (o) { A(n) => n, B => 0 }; !h; h = 1) {}",
            DiagnosticCode::MatchPlacement,
        ),
        (
            "for (const [a, b] = match (o) { A(n) => [n, () => b], B => [0, null] }; ; ) break;",
            DiagnosticCode::MatchPlacement,
        ),
        (
            "for (let w = id(() => w) && try r(); !w; w = 1) {}",
            DiagnosticCode::TryPlacement,
        ),
    ];
    for (head, code) in rejected {
        let source = format!("{prelude}export function f(): R {{ {head} return r(); }}\n");
        let diagnostics = ttc::analyze(&source, &Options::default());
        assert_eq!(diagnostics.len(), 1, "{head}: {diagnostics:#?}");
        assert_eq!(diagnostics[0].code, *code, "{head}: {diagnostics:#?}");
        assert!(
            diagnostics[0]
                .message
                .contains("refers to a binding the head declares"),
            "{head}: {diagnostics:#?}"
        );
    }
    let accepted = [
        "for (let n = match (o) { A(n) => n, B => 0 }; n < 3; n++) {}",
        "for (let k = match (o) { A(n) => (k: number) => k + n, B => null }; k; k = null) {}",
        "for (var v = match (o) { A(n) => () => v, B => null }; v; v = null) {}",
        "for (let q = match (o) { A(n) => { const q = n; return () => q; }, B => null }; q; q = null) {}",
    ];
    for head in accepted {
        let out = ok(&format!(
            "{prelude}export function f(): R {{ {head} return r(); }}\n"
        ));
        assert!(!out.contains("match ("), "{head}: {out}");
    }
}
