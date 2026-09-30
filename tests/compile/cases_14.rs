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
        assert!(out.contains("const n = $tt_ap(s, String)\n"), "{ty}:\n{out}");
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
fn a_hoisted_value_in_a_pipeline_operand_keeps_the_callee_before_it() {
    let out = ok(&format!(
        "{TASK_501_PRELUDE}export const v = f(match (n) {{ 0 => 1, _ => 2 }}) |> String;\n"
    ));
    let callee = out.find("= (f);").expect("the callee is captured");
    let region = out.find("switch (").expect("the match follows");
    assert!(callee < region, "{out}");
    assert_eq!(out.matches("f(").count(), 1, "{out}");
}

#[test]
fn a_conditional_operand_owns_its_branch_in_a_pipeline_head() {
    let out = ok(&format!(
        "{TASK_501_PRELUDE}export const v = g() && f(match (n) {{ 0 => 1, _ => 2 }}) |> String;\n"
    ));
    assert!(out.contains("if ($tt_v3 = g()) {"), "{out}");
    assert!(out.contains("$tt_v4 = $tt_v3;"), "{out}");
    assert!(!out.contains("&&"), "{out}");
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
fn a_member_step_calls_its_method_on_the_piped_value_after_the_argument() {
    let out = ok(&format!(
        "{TASK_504_PRELUDE}export const v = g() |> .m(match (n) {{ 0 => 1, _ => 2 }});\n"
    ));
    let head = out.find("= g();").expect("the head is evaluated first");
    let region = out.find("switch (").expect("the match follows");
    let call = out.find(".m($tt_v1)").expect("the call reads the method");
    assert!(head < region && region < call, "{out}");
    assert!(!out.contains(".bind("), "{out}");
    assert_eq!(out.matches("= g()").count(), 1, "{out}");
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
fn an_assignment_captures_its_target_before_a_hoisted_right_operand() {
    let prelude = "type R = { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\n\
         declare function read(): R;\n\
         declare function target(): { v: number };\n\
         declare function key(): \"v\";\n\
         declare let state: { v: number };\n";
    let out = ok(&format!(
        "{prelude}export function f(): R {{\n  target()[key()] -= try read();\n  return {{ kind: \"Ok\", value: 0 }};\n}}\n"
    ));
    let object = out.find("= (target());").expect("the object is captured");
    let key = out.find("= (key());").expect("the key is captured");
    let current = out.find("= ($tt_v1[$tt_v2]);").expect("the target is read");
    let region = out.find("= read();").expect("the right operand follows");
    assert!(object < key && key < current && current < region, "{out}");
    assert!(out.contains(" -= $tt_v0;"), "{out}");

    let out = ok(&format!(
        "{prelude}export function g(): R {{\n  state.v *= try read();\n  this.v = try read();\n  return {{ kind: \"Ok\", value: 0 }};\n}}\n"
    ));
    assert!(out.contains("let $tt_v1 = (state.v);"), "{out}");
    assert!(out.contains("state.v = $tt_v1 *= $tt_v0;"), "{out}");
    assert!(out.contains("this.v = $tt_v2;"), "{out}");
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

#[test]
fn a_case_named_like_the_prototype_setter_is_an_own_constructor_property() {
    let out = ok("variant V { __proto__(x: number), B }\nvariant U { __proto__, C }\n");
    assert!(
        out.contains("  [\"__proto__\"]: (x: number): V => ({ kind: \"__proto__\", x }),"),
        "{out}"
    );
    assert!(
        out.contains("  [\"__proto__\"]: { kind: \"__proto__\" } as const,"),
        "{out}"
    );
    assert!(!out.contains("\n  __proto__:"), "{out}");
    let ambient = ok("declare variant V { __proto__(x: number), B }\n");
    assert!(ambient.contains("readonly __proto__: (x: number) => V;"), "{ambient}");
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
fn a_value_in_a_later_declarator_splits_the_declaration_before_its_prelude() {
    let prelude = "import * as Result from \"@tt/std/result\";\n\
                   import type { TResult } from \"@tt/std\";\n\
                   variant O { A(n: number), B }\n\
                   declare const o: O;\n\
                   declare function t(s: string): number;\n\
                   declare function r(n: number): TResult<number, string>;\n";
    for (declaration, head, tail) in [
        (
            "const a = t(\"a\"), b = match (o) { A(n) => a + n, B => 0 };",
            "const a = t(\"a\");",
            "const b = $tt_v0;",
        ),
        (
            "var a = t(\"a\"), b = match (o) { A(n) => a + n, B => 0 }, c = b;",
            "var a = t(\"a\");",
            "var b = $tt_v0, c = b;",
        ),
        (
            "let a = t(\"a\"), b = 1 + try r(a);",
            "let a = t(\"a\");",
            "let b = 1 + $tt_v0;",
        ),
        (
            "const a = t(\"a\"), b = result { const x = try r(a); return x; };",
            "const a = t(\"a\");",
            "const b = $tt_v0;",
        ),
    ] {
        let out = ok(&format!(
            "{prelude}export function f(): TResult<number, string> {{\n  {declaration}\n  return Result.Ok(0);\n}}\n"
        ));
        let text = compact(&out);
        let head_at = text
            .find(head)
            .unwrap_or_else(|| panic!("{declaration}\n{out}"));
        let slot_at = text
            .find("let $tt_v0")
            .unwrap_or_else(|| panic!("{declaration}\n{out}"));
        let tail_at = text
            .find(tail)
            .unwrap_or_else(|| panic!("{declaration}\n{out}"));
        assert!(
            head_at < slot_at && slot_at < tail_at,
            "{declaration}\n{out}"
        );
    }
    let out = ok(&format!(
        "{prelude}export const a = t(\"a\"), b = match (o) {{ A(n) => a + n, B => 0 }};\n"
    ));
    assert!(out.contains("export const a = t(\"a\");\n"), "{out}");
    assert!(out.contains("export const b = $tt_v0;"), "{out}");
    let out = ok(&format!(
        "{prelude}export function g(c: boolean) {{\n  if (c) var a = t(\"a\"), b = match (o) {{ A(n) => a + n, B => 0 }};\n  return b;\n}}\n"
    ));
    assert!(
        compact(&out).contains("if (c) { var a = t(\"a\"); let $tt_v0"),
        "{out}"
    );
    assert!(compact(&out).contains("var b = $tt_v0; } return b;"), "{out}");
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
fn a_conditional_operation_tests_its_condition_where_it_evaluates_it() {
    let prelude = "variant O { A(n: number), B }\ndeclare const o: O;\ndeclare const cfg: { name?: string };\n";
    let cases: &[(&str, &str)] = &[
        ("cfg.name ? match (o) { A(n) => n, B => 0 } : 1", "if (cfg.name) {"),
        ("cfg.name && match (o) { A(n) => n, B => 0 }", "if ($tt_v1 = cfg.name) {"),
        ("!cfg.name || match (o) { A(n) => n, B => 0 }", "if ($tt_v1 = !cfg.name) {\n  $tt_v2 = $tt_v1;\n} else {"),
        ("cfg.name ?? match (o) { A(n) => n, B => 0 }", "if (($tt_v1 = cfg.name) == null) {"),
        ("(cfg.name, cfg) && match (o) { A(n) => n, B => 0 }", "if ($tt_v1 = (cfg.name, cfg)) {"),
    ];
    for (value, test) in cases {
        let out = ok(&format!("{prelude}export const v = {value};\n"));
        assert!(out.contains(test), "{value}: {out}");
        assert!(!out.contains("const $tt_v1"), "{value}: {out}");
        assert_eq!(out.matches("cfg.name").count(), 1, "{value}: {out}");
    }
}
