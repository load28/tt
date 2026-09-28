/// Statements the grammar has completed before the next line: a `/` or
/// `<` there begins an operand of the next statement (TASK-494). A prefix
/// ending in two spaces opened a body the case closes after the operand.
const TASK_494_FINISHED: &[&str] = &[
    "if (1) a;\n",
    "if (1) ;\n",
    "if (1) a; else b;\n",
    "if (1) {}\n",
    "while (0) a;\n",
    "for (;;) a;\n",
    "for (;;) {}\n",
    "L: a;\n",
    "L: {}\n",
    "function f() {}\n",
    "export function g() {}\n",
    "class C {}\n",
    "interface I {}\n",
    "enum E {}\n",
    "namespace N {}\n",
    "declare module \"m\" {}\n",
    "try {} catch {}\n",
    "switch (1) {}\n",
    "export default class {}\n",
    "let c: number\n",
    "type T = number\n",
    "declare const d: number\n",
    "import \"a\"\n",
    "do {} while (0) ",
    "{ a; } ",
    "function* gy() {\n  yield\n  ",
    "L1: for (;;) {\n  break L1\n  ",
    "L2: for (;;) {\n  continue L2\n  ",
    "for (;;) {\n  break\n  ",
    "function db() {\n  debugger\n  ",
];

fn task_494_case(prefix: &str, operand: &str) -> String {
    let close = if prefix.ends_with("  ") {
        ";\n}\n"
    } else {
        ";\n"
    };
    format!("declare const a: any, b: any;\n{prefix}{operand}{close}")
}

#[test]
fn a_regex_after_a_finished_statement_keeps_its_text() {
    for prefix in TASK_494_FINISHED {
        let source = task_494_case(prefix, "/ val const q = 2 /.test(\"\")");
        assert_eq!(ok(&source), source, "{prefix:?}");
    }
    let source = task_494_case("export default function () {}\n", "/ val const q = 2 /.test(\"\")");
    assert_eq!(ok(&source), source);
}

#[test]
fn a_jsx_element_after_a_finished_statement_keeps_its_text() {
    for prefix in TASK_494_FINISHED {
        let source = task_494_case(prefix, "<b> val const q = 2 </b>");
        assert_eq!(ok_tsx(&source), source, "{prefix:?}");
    }
}

#[test]
fn a_match_after_a_regex_statement_after_an_if_compiles() {
    let output = ok("declare const x: Option<number>;\n\
         export function f(s: string) {\n  if (!s) return;\n  /`/.test(s) && s;\n  return match (x) { Some(v) => v, None => 0 };\n}\n");
    assert!(output.contains("  /`/.test(s) && s;\n"), "{output}");
    assert!(output.contains("switch ($tt_m.kind)"), "{output}");
}

const TASK_495_PRELUDE: &str = "declare function f<A, B>(v: any): any;\n\
                                declare function g<A, B>(v: any): Option<number>;\n\
                                type A = 1;\ntype B = 2;\ndeclare const x: number;\n";

/// A `,` inside type arguments is inside a bracket pair for every tt
/// construct that splits at a top-level `,` or ends at a top-level token
/// (TASK-495).
#[test]
fn a_comma_inside_type_arguments_stays_inside_its_construct() {
    for (source, expected) in [
        (
            "export const r = match (x) { 1 => f<A, B>(x), _ => 2 };\n",
            "$tt_v0 = f<A, B>(x);",
        ),
        (
            "export const r = match (x) { 1 => new Map<A, B>(), _ => 2 };\n",
            "$tt_v0 = new Map<A, B>();",
        ),
        (
            "export const r = match (x) { 1 if f<A, Map<A, B>>(x) => f<B, A>(x), _ => 2 };\n",
            "if (f<A, Map<A, B>>(x)) {",
        ),
        (
            "export function h() {\n  let Some(v) = g<A, B>(x) else { return 0 };\n  return v;\n}\n",
            "const $tt_t0 = g<A, B>(x);",
        ),
        (
            "export function h() {\n  if let Some(v) = g<A, B>(x) { return v; }\n  return 0;\n}\n",
            "const $tt_t0 = g<A, B>(x);",
        ),
        (
            "export const p = x |> f<A, B> |> f<B, A>;\n",
            "$tt_ap($tt_ap(x, f<A, B>), f<B, A>)",
        ),
        (
            "export const s = match (g<A, B>(x), x) { (Some(v), _) => v, _ => 2 };\n",
            "const $tt_m0 = g<A, B>(x);",
        ),
    ] {
        let output = ok(&format!("{TASK_495_PRELUDE}{source}"));
        assert!(output.contains(expected), "{source}\n{output}");
    }
}

#[test]
fn a_comparison_comma_still_separates_scrutinees() {
    let output = ok("declare const a: number, b: number, c: number;\n\
         export const r = match (a < b, c > a) { (_, _) => 1 };\n");
    assert!(output.contains("const $tt_m0 = a < b;"), "{output}");
    assert!(output.contains("const $tt_m1 = c > a;"), "{output}");
}
