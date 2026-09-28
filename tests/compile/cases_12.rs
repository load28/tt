/// Lines that end a statement at their line break although their last
/// token was once read as unfinished: a type ending in `>` or `void`,
/// contextual keywords used as names, and line terminators other than LF.
const TASK_491_STATEMENT_ENDS: &[&str] = &[
    "const a = o as Array<number>",
    "let g: () => void",
    "let p: Promise<void>",
    "x satisfies Record<K, unknown>",
    "const m = new Map<string, number>",
    "const of = 1",
    "const async = 2",
    "const n = { await: 1, yield: 2 }.await",
    "const r = /a/g",
];

const TASK_491_PRELUDE: &str = "type K = string;\n\
                                declare const o: unknown;\n\
                                declare const x: object;\n\
                                declare const e: Error;\n";

#[test]
fn a_let_else_block_diverges_after_a_line_ending_in_a_type_or_a_name() {
    for line in TASK_491_STATEMENT_ENDS {
        let source = format!(
            "{TASK_491_PRELUDE}export function f(v: Option<number>): number {{\n  const Some(value) = v else {{\n    {line}\n    throw e\n  }};\n  return value;\n}}\n"
        );
        assert_eq!(codes(&source), vec![], "{line}");
        ok(&source);
    }
}

#[test]
fn every_ecma_line_terminator_ends_a_statement() {
    for terminator in ["\r", "\r\n", "\u{2028}", "\u{2029}"] {
        let source = format!(
            "{TASK_491_PRELUDE}export function f(v: Option<number>): number {{\n  const Some(value) = v else {{\n    const a = 1{terminator}    throw e\n  }};\n  return value;\n}}\n"
        );
        assert_eq!(codes(&source), vec![], "{terminator:?}");
    }
    let source = "declare const e: Error;\nexport function f(v: Option<number>): number {\n  const Some(value) = v else {\n    // a comment ends at a CR\r    throw e\n  };\n  return value;\n}\n";
    assert_eq!(codes(source), vec![]);
}

#[test]
fn an_if_let_after_a_line_ending_in_a_type_starts_a_statement() {
    for line in TASK_491_STATEMENT_ENDS {
        let source = format!(
            "{TASK_491_PRELUDE}declare const v: Option<number>;\nexport function f() {{\n  {line}\n  if let Some(value) = v {{ console.log(value); }}\n}}\n"
        );
        assert_eq!(codes(&source), vec![], "{line}");
        assert!(ok(&source).contains(".kind === \"Some\""), "{line}");
    }
}

#[test]
fn a_pipeline_head_starts_on_the_line_after_a_type() {
    for line in TASK_491_STATEMENT_ENDS {
        let source = format!(
            "{TASK_491_PRELUDE}declare function inc(n: number): number;\n{line}\n2 |> inc;\n"
        );
        let out = ok(&source);
        assert!(out.contains(&format!("{line}\ninc(2);")), "{line}\n{out}");
    }
}

#[test]
fn a_try_after_a_line_ending_in_a_type_is_a_statement() {
    for line in TASK_491_STATEMENT_ENDS {
        let source = format!(
            "{TASK_491_PRELUDE}declare function read(): Result<number, string>;\nexport function f(): Result<number, string> {{\n  {line}\n  try read();\n  return Result.Ok(1);\n}}\n"
        );
        assert_eq!(codes(&source), vec![], "{line}");
        let out = ok(&source);
        assert!(!out.contains("$tt_v0"), "{line}\n{out}");
    }
}

#[test]
fn a_statement_match_after_a_line_ending_in_a_type_keeps_both_statements() {
    for line in TASK_491_STATEMENT_ENDS {
        let source = format!(
            "{TASK_491_PRELUDE}variant S {{ A, B }}\nexport function f(s: S) {{\n  {line}\n  match (s) {{ A => console.log(1), B => console.log(2) }}\n}}\n"
        );
        assert_eq!(codes(&source), vec![], "{line}");
        let out = ok(&source);
        assert!(out.contains(&format!("  {line}\n")), "{line}\n{out}");
        assert!(!out.contains("match (s)"), "{line}\n{out}");
    }
}

#[test]
fn a_block_after_a_call_opens_no_function() {
    let source = "declare function f(): Option<number>;\n\
                  declare function g(): Result<number, string>;\n\
                  if let Some(v) = f() { try g(); }\n";
    assert_eq!(codes(source), [DiagnosticCode::TryPlacement]);
}
