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
