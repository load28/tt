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
    assert!(out.contains("} else {"), "{out}");
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
fn a_member_step_captures_its_method_from_the_piped_value_before_the_argument() {
    let out = ok(&format!(
        "{TASK_504_PRELUDE}export const v = g() |> .m(match (n) {{ 0 => 1, _ => 2 }});\n"
    ));
    let head = out.find("= g();").expect("the head is evaluated first");
    let method = out.find(".m).bind(").expect("the method is captured");
    let region = out.find("switch (").expect("the match follows");
    assert!(head < method && method < region, "{out}");
    assert_eq!(out.matches("= g()").count(), 1, "{out}");
}
