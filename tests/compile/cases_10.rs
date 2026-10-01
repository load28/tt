/* ------------------------------------------------------------------ */
/* TASK-377 compiler boundary and lowering defects                     */
/* ------------------------------------------------------------------ */

#[test]
fn a_result_tail_expression_without_a_semicolon_reports_only_the_missing_value() {
    let diagnostics = ttc::analyze(
        "declare function read(): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\nexport function f() {\n  const r = result { const a = try read(); a };\n  return r;\n}\n",
        &Options::default(),
    );
    let codes: Vec<_> = diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(codes, [DiagnosticCode::ResultNoSuccessValue], "{diagnostics:#?}");
}

#[test]
fn the_runtime_import_precedes_generated_text_at_the_top_of_the_file() {
    for source_kind in [SourceKind::TypeScript, SourceKind::Tsx] {
        let options = Options { source_kind, ..Options::default() };
        let source = "variant S { A, B }\nconst xs = [1].map(x => x |> pick());\nexport {};\n";
        let out = compile(source, &options).unwrap();
        assert!(out.starts_with("import { $tt_ap } from \"@tt/runtime\";\ntype S =\n"), "{out}");
        assert!(out.contains("B: { kind: \"B\" } as const,\n};\nconst xs"), "{out}");

        let out = compile(&format!("\"use client\";\n{source}"), &options).unwrap();
        assert!(out.starts_with("\"use client\";\nimport { $tt_ap } from \"@tt/runtime\";\ntype S =\n"), "{out}");

        let out = compile(&format!("\u{feff}{source}"), &options).unwrap();
        assert!(out.starts_with("\u{feff}import { $tt_ap } from \"@tt/runtime\";\ntype S =\n"), "{out}");

        let out = compile(&format!("\u{feff}#!/usr/bin/env node\n{source}"), &options).unwrap();
        assert!(out.starts_with("\u{feff}#!/usr/bin/env node\nimport { $tt_ap } from \"@tt/runtime\";\ntype S =\n"), "{out}");
    }
}
