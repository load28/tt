/// The support modules an emission imports are what codegen wrote, so a
/// build writes exactly the modules its outputs need.
#[test]
fn an_emission_reports_the_support_modules_it_imports() {
    use ttc::StdModule;
    let imports = |source: &str| {
        ttc::compile_mapped(source, &Options::default())
            .unwrap()
            .support_imports
    };
    let pipeline = "declare function input(): number;\n\
                    declare const step: () => (value: number) => number;\n";
    assert_eq!(
        imports(&format!("{pipeline}export const v = input() |> step();\n")),
        [StdModule::Runtime]
    );
    // A literal head lowers to a direct call; a script inlines its helper.
    assert!(imports("export const a = 1 |> String;\n").is_empty());
    assert!(imports(&format!("{pipeline}const v = input() |> step();\n")).is_empty());
    assert_eq!(
        imports("import * as Option from \"@tt/std/option\";\nexport const o = Option;\n"),
        [StdModule::Option]
    );
    assert!(imports("export const plain = 1;\n").is_empty());
}

#[test]
fn local_variant_shadows_extern_of_same_name() {
    // The local Token has only two cases; the extern one must not resurrect
    // a third. Full local coverage compiles.
    let externs = [token_extern()];
    let opts = Options {
        extern_variants: &externs,
        ..Options::default()
    };
    let out = compile(
        "variant Token { Num(value: number), Ident(name: string) }\nconst s = match (t) { Num(value) => value, Ident(name) => 0 };\n",
        &opts,
    )
    .unwrap();
    assert!(out.contains("switch ($tt_m.kind)"));
}

#[test]
fn extern_variant_shadows_builtin_of_same_name() {
    // An imported `Option` with an extra case replaces the built-in: the
    // two-case match that satisfies the built-in must now be an error.
    let externs = [ttc::ExternVariant {
        name: "Option".to_string(),
        tags: ["Some", "None", "Maybe"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        from: Some("./opt.tt".to_string()),
    }];
    let opts = Options {
        extern_variants: &externs,
        ..Options::default()
    };
    let e = compile(
        "const s = match (o) { Some(value) => value, None => 0 };\n",
        &opts,
    )
    .expect_err("expected non-exhaustive error");
    assert!(e.message.contains("missing \"Maybe\""), "{}", e.message);
}

#[test]
fn extern_variants_do_not_affect_unrelated_matches() {
    // Tags that belong to no known variant stay unchecked (runtime guard only).
    let externs = [token_extern()];
    let opts = Options {
        extern_variants: &externs,
        ..Options::default()
    };
    let out = compile("const s = match (x) { Foo(a) => a, Bar => 0 };\n", &opts).unwrap();
    assert!(out.contains("switch ($tt_m.kind)"));
}

/* ------------------------------------------------------------------ */
/* declaration collection API                                          */
/* ------------------------------------------------------------------ */

#[test]
fn exported_variants_returns_exported_tt_enums_only() {
    let decls = ttc::exported_variants(
        "export variant Token { Num(value: number), Eof }\nenum Private { A, B }\nexport variant Color { Red, Green }\n",
    );
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].name, "Token");
    assert_eq!(decls[0].tags, ["Num", "Eof"]);
    assert_eq!(decls[0].from, None);
    assert_eq!(decls[1].name, "Color");
}

#[test]
fn exported_variants_names_a_variant_by_its_local_export_specifiers() {
    let decls = ttc::exported_variants(
        "variant Color { Red, Green }\n\
         variant Size { S, L }\n\
         variant Kept { K }\n\
         export variant Token { Eof }\n\
         export { Color as Hue, Size };\n\
         export type { Color as Tint };\n\
         export { Kept as default };\n\
         export { Token as Tok };\n\
         export { Other } from \"./other.tt\";\n\
         namespace N { export { Color as Inner }; }\n\
         declare const o: { export: any };\n\
         o.export({ Size: 1 });\n",
    );
    let names: Vec<(&str, Vec<&str>)> = decls
        .iter()
        .map(|d| (d.name.as_str(), d.tags.iter().map(String::as_str).collect()))
        .collect();
    assert_eq!(
        names,
        [
            ("Token", vec!["Eof"]),
            ("Hue", vec!["Red", "Green"]),
            ("Size", vec!["S", "L"]),
            ("Tint", vec!["Red", "Green"]),
            ("default", vec!["K"]),
            ("Tok", vec!["Eof"]),
        ]
    );
    let symbols = ttc::exported_variant_symbols("variant Color { Red }\nexport { Color as Hue };\n");
    assert_eq!(symbols[0].name, "Hue");
    assert!(symbols[0].exported);
    assert_eq!(symbols[0].offset, "variant ".len());
}

#[test]
fn tt_imports_reports_specifiers_and_names() {
    use ttc::TtImportNames;
    let imports = ttc::tt_imports(
        r#"
import { Token, Kind as K, type T } from "./a.tt";
import * as ns from "../b.tt";
import "./side.tt";
export { X } from "./re.tt";
import { skip } from "./not-tt.ts";
import legacy = require("./legacy.tt");
declare module "./augmented.tt" {}
const lazy = import(`./lazy.tt`);
"#,
    );
    assert_eq!(imports.len(), 7);
    assert_eq!(imports[4].specifier, "./legacy.tt");
    assert_eq!(imports[4].names, TtImportNames::Namespace("legacy".to_string()));
    assert_eq!(imports[5].specifier, "./augmented.tt");
    assert_eq!(imports[5].names, TtImportNames::None);
    assert_eq!(imports[6].specifier, "./lazy.tt");
    assert_eq!(imports[0].specifier, "./a.tt");
    assert_eq!(
        imports[0].names,
        TtImportNames::Named(vec![
            ("Token".to_string(), None),
            ("Kind".to_string(), Some("K".to_string())),
            ("T".to_string(), None),
        ])
    );
    assert_eq!(imports[1].specifier, "../b.tt");
    assert_eq!(imports[1].names, TtImportNames::Namespace("ns".to_string()));
    assert_eq!(imports[2].names, TtImportNames::None);
    assert_eq!(imports[3].specifier, "./re.tt");
    assert_eq!(imports[3].names, TtImportNames::None);
}

#[test]
fn scan_module_answers_both_questions_in_one_pass() {
    // The single-fact helpers are defined in terms of the scan, so the two
    // views must never disagree — that equivalence is what lets the CLI
    // parse each input once.
    for source in [
        "import * as Option from \"@tt/std/option\";\nimport { T } from \"./t.tt\";\n",
        "import { T } from \"./t.tt\";\n",
        "export * from \"@tt/std/result\";\n",
        "const match = 1;\n",
        "",
    ] {
        let scan = ttc::scan_module(source);
        assert_eq!(scan.imports, ttc::tt_imports(source), "{source:?}");
        assert_eq!(scan.imports_std, ttc::imports_std(source), "{source:?}");
    }

    let scan = ttc::scan_module(
        "import * as Option from \"@tt/std/option\";\nimport * as ns from \"../b.tt\";\n",
    );
    assert!(scan.imports_std);
    assert_eq!(scan.imports.len(), 1);
    assert_eq!(scan.imports[0].specifier, "../b.tt");

    let nested = ttc::scan_module("const value = `${input |> step}`;\n");
    assert!(nested.uses_pipeline);
    assert!(!ttc::scan_module("const text = 'input |> step';\n").uses_pipeline);
}

/* ------------------------------------------------------------------ */
/* symbol API                                                          */
/* ------------------------------------------------------------------ */

#[test]
fn variant_symbols_carries_positions_and_field_shapes() {
    let src = "export variant Token {\n  Num(value: number),\n  Empty(),\n  Eof,\n}\nvariant Local { A }\n";
    let syms = ttc::variant_symbols(src);
    assert_eq!(syms.len(), 2);

    let token = &syms[0];
    assert_eq!(token.name, "Token");
    assert!(token.exported);
    assert_eq!(ttc::line_col(src, token.offset), (1, 16));
    assert_eq!(token.cases.len(), 3);
    assert_eq!(ttc::line_col(src, token.cases[0].offset), (2, 3));
    let fields = token.cases[0].fields.as_ref().unwrap();
    assert_eq!(fields[0].name, "value");
    assert_eq!(fields[0].ty, "number");
    assert!(!fields[0].optional);
    // `Empty()` has an empty field list; `Eof` has none at all.
    assert_eq!(token.cases[1].fields.as_deref(), Some(&[][..]));
    assert_eq!(token.cases[2].fields, None);

    assert_eq!(syms[1].name, "Local");
    assert!(!syms[1].exported);
}

/* ------------------------------------------------------------------ */
/* pipeline                                                            */
/* ------------------------------------------------------------------ */

#[test]
fn pipeline_emits_nested_apply_helper_calls() {
    let out = ok("const y = half(4) |> times(2) |> label();\nexport {};\n");
    assert!(
        out.contains("const y = $tt_ap($tt_ap(half(4), times(2)), label());"),
        "{out}"
    );
    assert!(out.contains("import { $tt_ap } from \"@tt/runtime\";"));
}

/// The output offset the source byte at `src` was copied to.
fn output_of(emit: &ttc::MappedEmit, src: usize) -> usize {
    emit.mappings
        .iter()
        .find(|m| m.src <= src && src < m.src + m.len)
        .map(|m| m.out + (src - m.src))
        .unwrap_or_else(|| panic!("source byte {src} was not copied to the output"))
}

/// The lines codegen *started* — lines whose first non-whitespace byte is
/// glue rather than copied source — between two source landmarks. That is
/// exactly the set layout decides the indentation of: a line beginning
/// inside a verbatim block keeps whatever indentation the source gave it.
fn generated_lines(source: &str, after: &str, before: &str) -> Vec<String> {
    let emit = ttc::emit_mapped(source);
    let mut verbatim = vec![false; emit.code.len()];
    for mapping in &emit.mappings {
        for byte in &mut verbatim[mapping.out..mapping.out + mapping.len] {
            *byte = true;
        }
    }
    let from = output_of(
        &emit,
        source.find(after).expect("landmark") + after.len() - 1,
    );
    let to = output_of(&emit, source.find(before).expect("landmark"));
    let mut lines = Vec::new();
    let mut at = 0usize;
    for line in emit.code.split_inclusive('\n') {
        let head = line.len() - line.trim_start().len();
        if at > from && at + head < to && !line.trim().is_empty() && !verbatim[at + head] {
            lines.push(line.trim_end_matches('\n').to_string());
        }
        at += line.len();
    }
    lines
}

#[test]
fn every_construct_lays_its_glue_out_from_the_line_it_replaces() {
    // The layout rule, checked as a rule instead of once per construct:
    // put each construct at four different indentations and assert every
    // line codegen wrote starts at that indentation plus whole levels, and
    // that indenting the construct changes nothing but the indentation —
    // the same lowering, the same number of generated lines.
    let constructs = [
        "const r = match (e) { A(v) => v, B => 0 };",
        "const r = match (e) { A(v) if v > 0 => v, B => 0, _ => 1 };",
        "const r = match (e) { A(v) => { return v; }, B => 0 };",
        "const r = match (n) { 1 => \"one\", _ => \"other\" };",
        "const r = match (e, e) { (A(v), B) => v, (_, _) => 0 };",
        // A pipeline whose head is itself a lowering, so the steps get a
        // region rather than one inline call.
        "const r = match (e) { A(v) => v, B => 0 } |> pick |> .toString();",
        "const r = result { const v = try ask(); return v; };",
        // These two lower inline; the rule still has to hold for them,
        // which here means staying inline at every indentation.
        "if let A(v) = e { use(v); }",
        "const A(v) = e else { throw new Error(\"no\"); };",
        "variant Inner { X(a: number), Y }",
    ];
    let prelude = "variant E { A(v: number), B }\ndeclare const e: E;\ndeclare const n: number;\n\
                   declare function use(v: unknown): void;\ndeclare function pick(v: E): string;\n\
                   declare function ask(): { kind: \"Ok\"; value: number } | { kind: \"Err\"; error: string };\n";
    let mut with_block_structure = 0;
    for construct in constructs {
        let mut counts = Vec::new();
        for base in ["", "  ", "      ", "\t\t"] {
            // A block gives the construct a line of its own to sit on; the
            // brace itself is source, so only the lowering answers here.
            let source =
                format!("{prelude}function host() {{\n{base}{construct}\n{base}return null;\n}}\n");
            let lines = generated_lines(&source, "function host() {", "return null;");
            counts.push(lines.len());
            for line in lines {
                let indent: String = line
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect();
                assert!(
                    indent.starts_with(base),
                    "line {line:?} does not start at the construct's indentation {base:?}\n\
                     construct: {construct}"
                );
                let inside = &indent[base.len()..];
                assert!(
                    inside.chars().all(|c| c == ' ') && inside.len().is_multiple_of(2),
                    "line {line:?} is indented {inside:?} past the base, not whole levels\n\
                     construct: {construct}"
                );
            }
        }
        assert!(
            counts.iter().all(|count| *count == counts[0]),
            "indenting the construct changed how many lines it lowers to: {counts:?}\n\
             construct: {construct}"
        );
        if counts[0] > 0 {
            with_block_structure += 1;
        }
    }
    // Constructs that lower inline contribute no generated lines, so the
    // corpus has to prove it is exercising block structure at all.
    assert!(
        with_block_structure >= 7,
        "only {with_block_structure} constructs produced block structure — the probe went blind"
    );
}

#[test]
fn pipeline_runtime_is_imported_once_per_file() {
    let out = ok("const a = x |> f();\nconst b = y |> g();\nexport {};\n");
    assert_eq!(out.matches("$tt_ap(").count(), 2, "{out}");
    assert_eq!(out.matches("from \"@tt/runtime\"").count(), 1, "{out}");
}

#[test]
fn empty_or_dangling_step_is_an_error() {
    for (src, col) in [
        ("const a = x |>;\n", 13),
        ("const a = x |> |> f;\n", 13),
        ("const a = x |> .length |>\nconst b = a;\n", 24),
    ] {
        let codes: Vec<_> = ttc::analyze(src, &Options::default())
            .iter()
            .map(|d| d.code)
            .collect();
        assert_eq!(codes, [ttc::DiagnosticCode::MissingPipelineStep], "{src}");
        let e = err(src);
        assert!(e.message.contains("`|>` has no step"), "{}", e.message);
        assert_eq!((e.line, e.col), (1, col), "{src}");
    }
}

#[test]
fn a_missing_step_keeps_the_pipeline_written_before_it() {
    // The steps written stay the pipeline, and the missing one applies
    // TypeScript's error type, so the projection still reads the head and
    // the statement after it.
    let src = "export function run(xs: number[]): number {\n  const n = xs |> .length |> \n  return n;\n}\n";
    let report = ttc::compile_projection_report(src, &Options::default());
    let emit = report.emit.expect("the projection emits");
    assert!(
        emit.code
            .contains("const n = (undefined as any)(xs.length) \n  return n;"),
        "{}",
        emit.code
    );
    assert!(report.recovered.is_empty(), "{:?}", report.recovered);
}

#[test]
fn an_arm_with_no_body_keeps_its_pattern_and_guard() {
    let decl = "variant Shape { Circle(radius: number), Rect(width: number) }\ndeclare const s: Shape;\n";
    let cases = [
        (
            "const a = match (s) { Circle(radius) => radius, Rect(width) if width > 0 };",
            "if (width > 0)",
        ),
        (
            "const a = match (s) { Circle(radius) => radius, Rect(width) if width > 0, _ => 0 };",
            "if (width > 0)",
        ),
        (
            "const a = match (s) { Circle(radius) => radius, Rect(width) => };",
            "const { width } = ",
        ),
        (
            "const a = match (s, s) { (Circle(r), _) => r, (Rect(w), _) if w > 0, _ => 0 };",
            "if (w > 0)",
        ),
        (
            "console.log(match (s) { Circle(radius) => radius, Rect(width) if width > 0 });",
            "width > 0",
        ),
    ];
    for (statement, kept) in cases {
        let src = format!("{decl}{statement}\n");
        let report = ttc::compile_projection_report(&src, &Options::default());
        let codes: Vec<_> = report.diagnostics.iter().map(|d| d.code).collect();
        assert!(
            codes.contains(&DiagnosticCode::MissingArmBody)
                && !codes.contains(&DiagnosticCode::MalformedMatch),
            "{src}: {codes:?}"
        );
        let emit = report.emit.expect("the projection emits");
        assert!(report.recovered.is_empty(), "{:?}", report.recovered);
        assert!(emit.code.contains(kept), "{}", emit.code);
        assert!(emit.code.contains("(undefined as any)"), "{}", emit.code);
    }
}

#[test]
fn an_arm_whose_guard_is_not_written_yet_is_a_malformed_arm() {
    let decl = "variant Shape { Circle(radius: number), Rect(width: number) }\ndeclare const s: Shape;\ndeclare const t: string;\n";
    for statement in [
        "const a = match (s) { Circle(radius) => radius, Rect(width) if };",
        "const a = match (s) { Circle(radius) => radius, Rect(width) if  };",
        "const a = match (s) { Circle(radius) => radius, Rect(width) if, _ => 0 };",
        "const a = match (s, s) { (Circle(r), _) => r, (Rect(w), _) if };",
        "const a = match (t) { \"a\" => 1, \"b\" if };",
    ] {
        let src = format!("{decl}{statement}\n");
        let codes: Vec<_> = ttc::analyze(&src, &Options::default())
            .iter()
            .map(|d| d.code)
            .collect();
        assert_eq!(codes, [DiagnosticCode::MalformedMatch], "{src}");
        err(&src);
    }
}

#[test]
fn a_step_with_an_open_list_ends_where_typescript_ends_the_list() {
    // The list runs to the next statement, as TypeScript reads `add(2, `
    // with `const` after it; the statement stays outside the step.
    let src = "const v = 1 |> add(2, \nconst w = 1;\n";
    let report = ttc::compile_projection_report(src, &Options::default());
    let emit = report.withheld.expect("the faithful projection");
    assert!(emit.code.ends_with("(1)const w = 1;\n"), "{}", emit.code);
    let src = "function f() {\n  const v = 1 |> add(2, \n}\n";
    let report = ttc::compile_projection_report(src, &Options::default());
    let emit = report.withheld.expect("the faithful projection");
    assert!(emit.code.ends_with("(1)}\n"), "{}", emit.code);
}

#[test]
fn a_stray_pipe_recovers_only_to_the_end_of_its_statement() {
    let src = "export function run(a: boolean, f: (n: number) => number): number {\n  const n = a ? 1 : 2 |> f\n  const m = n + 1\n  return m;\n}\n";
    let report = ttc::compile_projection_report(src, &Options::default());
    let next = src.find("const m").unwrap();
    assert!(
        report
            .recovered
            .iter()
            .all(|&(start, end)| end <= next || start >= next),
        "{:?}",
        report.recovered
    );
    let emit = report.emit.expect("the projection emits");
    assert!(
        emit.code.contains("const m = n + 1\n  return m;"),
        "{}",
        emit.code
    );
}

#[test]
fn a_stray_if_let_recovers_only_the_statement_typescript_reads() {
    let after = "  const b = 1;\n  return b;\n}\nfunction g() { return 2; }\n";
    for (head, projected) in [
        (
            "if let Some(v) = find(id)",
            "void             find(id)",
        ),
        (
            "if let Some(v) = find(id.)",
            "void             find(id.)",
        ),
        ("if let Some(", ";           "),
        ("if let Some(v) =", ";               "),
        (
            "if let 1(x) = y { x } else if let B(z) = w { z } else { q }",
            "void          y ;",
        ),
    ] {
        let src = format!("function f(id: string) {{\n  {head}\n{after}");
        let report = ttc::compile_projection_report(&src, &Options::default());
        let emit = report
            .emit
            .or(report.withheld)
            .expect("the projection emits");
        assert_eq!(
            emit.code,
            format!(
                "function f(id: string) {{\n  {projected:<width$}\n{after}",
                width = head.len()
            ),
            "{src}"
        );
    }
}

#[test]
fn malformed_optional_postfix_is_one_owned_diagnostic() {
    for src in [
        "const a = x |> ?.;\n",
        "const a = x |> ?.#private;\n",
        "const a = x |> ?.tag`value`;\n",
        "const a = x |> ?.member + other |> next;\n",
    ] {
        let diagnostics = ttc::analyze(src, &Options::default());
        assert_eq!(diagnostics.len(), 1, "{src}\n{diagnostics:?}");
        assert_eq!(
            diagnostics[0].code,
            ttc::DiagnosticCode::MalformedPipelinePostfix,
            "{src}"
        );
        assert_eq!(
            diagnostics[0].owner.as_ref().map(|owner| owner.start),
            Some(10)
        );
    }
}

#[test]
fn bare_super_is_not_an_optional_receiver() {
    for src in [
        "class C extends B { m() { return super |> ?.value; } }\n",
        "class C extends B { m() { return /* kept */ super |> ?.value; } }\n",
    ] {
        let report = ttc::compile_report(src, &Options::default());
        assert!(report.emit.is_none(), "{src}\n{:?}", report.diagnostics);
        assert_eq!(report.diagnostics.len(), 1, "{report:?}");
        assert_eq!(
            report.diagnostics[0].code,
            ttc::DiagnosticCode::InvalidOptionalReceiver
        );
    }

    let out = ok("class C extends B { m() { return super |> .value |> ?.name; } }\n");
    assert!(out.contains("return super.value?.name;"), "{out}");
}
