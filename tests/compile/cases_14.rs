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
