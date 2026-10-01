#[test]
fn let_else_divergence_covers_every_statement_form() {
    // TASK-172: the graph models the whole statement grammar, so a form
    // it once approximated as fall-through now answers precisely.
    for body in [
        // A `switch` with a `default` whose every clause leaves.
        "switch (k) { case \"a\": return 1; default: throw new Error(\"x\"); }",
        // Clauses fall through to the next one.
        "switch (k) { case \"a\": case \"b\": return 1; default: return 2; }",
        // A loop with no normal exit is left only by `break`/`return`/`throw`.
        "while (true) { log(\"x\"); }",
        "for (;;) { log(\"x\"); }",
        // `do … while` runs its body before the test.
        "do { return 1; } while (c);",
        // A `try` diverges when every half that can complete does not.
        "try { return 1; } catch (e) { throw e; }",
        "try { return 1; } finally { log(\"x\"); }",
        "try { log(\"x\"); } finally { return 1; }",
        // A labeled block's `break` lands after it, on a `return`.
        "outer: { break outer; } return 0;",
        // A `break` naming an outer loop leaves the inner one only.
        "outer: while (true) { while (true) { break outer; } } return 0;",
        // Statement boundaries do not need semicolons.
        "log(\"x\")\n    return 0",
    ] {
        ok(&format!(
            "function f(): number {{\n  const Some(v) = find() else {{ {body} }};\n  return v;\n}}\n"
        ));
    }
}

#[test]
fn let_else_divergence_still_rejects_every_normal_exit() {
    // The other half of the same precision: a form the graph now enters
    // must not be able to claim a divergence it does not have.
    for body in [
        // No `default` — an unmatched discriminant walks past the switch.
        "switch (k) { case \"a\": return 1; }",
        // A `break` targets the switch, not the else block.
        "switch (k) { case \"a\": break; default: return 1; }",
        // A test that can fail is a normal exit.
        "while (c) { return 1; }",
        "do { log(\"x\"); } while (c);",
        // A loop is left by its own `break`.
        "while (true) { break; }",
        "for (;;) { if (c) break; }",
        "outer: while (true) { break outer; }",
        // The handler can run in place of the guarded block.
        "try { return 1; } catch (e) { log(e); }",
        "try { log(\"x\"); } finally { log(\"x\"); }",
        // A labeled block's `break` lands after it, and nothing follows.
        "outer: { break outer; }",
        // A nested function body is opaque across a line break too.
        "const g = () => { return 1 }\n    log(g())",
    ] {
        let e = err(&format!(
            "function f(): number {{\n  const Some(v) = find() else {{ {body} }};\n  return v;\n}}\n"
        ));
        assert!(e.message.contains("must diverge"), "{body}: {}", e.message);
    }
}

#[test]
fn let_else_divergence_stops_at_an_isolated_value_region() {
    // The other half: a match arm, a `result` block and a `try` statement
    // are not approximations left as fall-through — an exit written in an
    // isolated value region belongs to the construct's value and can never
    // leave the block, and a `try` statement's early return is
    // conditional. An `if let` missing either half falls through too.
    for body in [
        "if let Ok(value) = r { return value; }",
        "if let Ok(value) = r { log(value); } else { return 1; }",
        "if let Ok(value) = r { return value; } else { log(\"x\"); }",
        "const x = match (r) { Ok(value) => value, Err(error) => 0 }; log(x);",
        "const y = result { const a = try find(); return a; }; log(y);",
        "try find();",
        "const Ok(value) = r else { return 1; }; log(value);",
    ] {
        let e = err(&format!(
            "variant Res {{ Ok(value: number), Err(error: string) }}\n\
             function f(r: Res): number {{\n  const Some(v) = find() else {{ {body} }};\n  return v;\n}}\n"
        ));
        assert!(e.message.contains("must diverge"), "{body}: {}", e.message);
    }
}

/* ------------------------------------------------------------------ */
/* source TypeScript that does not parse                               */
/* ------------------------------------------------------------------ */

#[test]
fn invalid_typescript_in_a_match_arm_body_reports_the_byte_not_the_construct() {
    // The `match` on this line parsed as tt perfectly well; only the arm
    // body's TypeScript did not. The report names the failing byte and
    // makes no claim about the construct around it.
    let src = "const x = match (s) { A(v) => { const q = ; return q; }, _ => 0 };\n";
    let report = ttc::compile_report(src, &Options::default());
    assert_eq!(report.diagnostics.len(), 1, "{:#?}", report.diagnostics);
    assert_eq!(
        report.diagnostics[0].code,
        ttc::DiagnosticCode::SourceNotTypeScript
    );
    assert_eq!(
        report.diagnostics[0].start,
        Some(42),
        "{:#?}",
        report.diagnostics[0]
    );
    assert!(
        !report.diagnostics[0]
            .message
            .contains("did not parse as a tt"),
        "{}",
        report.diagnostics[0].message
    );
    assert!(
        report.emit.is_none(),
        "a file with no owner model emits nothing"
    );
}

#[test]
fn every_other_diagnostic_is_still_reported_with_it() {
    // The precondition failing does not swallow what the semantic passes
    // already found: one run reports everything.
    let src = "variant E { A(x: number), A(y: number) }\nconst v = match (E.A(1)) { A(x) => { const q = ; return q; }, _ => 0 };\n";
    let report = ttc::compile_report(src, &Options::default());
    let codes: Vec<_> = report.diagnostics.iter().map(|d| d.code).collect();
    assert!(
        codes.contains(&ttc::DiagnosticCode::VariantDuplicateCase)
            && codes.contains(&ttc::DiagnosticCode::SourceNotTypeScript),
        "{codes:?}"
    );
}

#[test]
fn no_verify_does_not_bypass_the_lowering_precondition() {
    // `--no-verify` skips the *output* self-check. This is not that check:
    // without the owner model there is nothing to emit, so the error stands.
    let opts = Options {
        verify: false,
        ..Options::default()
    };
    let e = compile(
        "const r = result {\n  const a = try f();\n  const b = ;\n  return a;\n};\n",
        &opts,
    )
    .expect_err("expected a compile error");
    assert!(
        e.message.contains("the TypeScript here does not parse"),
        "{}",
        e.message
    );
}

#[test]
fn discarded_result_reports_a_named_diagnostic_without_unwinding() {
    let source = "function f() { result { const x = try next(); return x; }; }\n";
    let diagnostics = std::panic::catch_unwind(|| ttc::analyze(source, &Options::default()))
        .expect("discarded Result must not reach source-preservation ICE");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == ttc::DiagnosticCode::ResultValueDiscarded),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.start.is_some()),
        "{diagnostics:#?}"
    );
}

#[test]
fn try_in_a_for_test_is_a_repeated_loop_placement_at_the_try() {
    for source in [
        "function f() { for (; try next(); ) {} }\n",
        "function f(flag: boolean) { for (; flag && try next(); ) {} }\n",
        "function f(flag: boolean) { for (; flag ? try next() : 0; ) {} }\n",
    ] {
        let diagnostics = ttc::analyze(source, &Options::default());
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == ttc::DiagnosticCode::TryPlacement)
            .unwrap_or_else(|| panic!("{source}\n{diagnostics:#?}"));
        assert!(
            diagnostic.message.contains("repeated loop position"),
            "{diagnostics:#?}"
        );
        assert_eq!(
            diagnostic.start,
            Some(source.find("try").unwrap()),
            "{diagnostics:#?}"
        );
        assert_eq!(diagnostics.len(), 1, "{source}\n{diagnostics:#?}");
    }
}

/* ------------------------------------------------------------------ */
/* swc output verification                                             */
/* ------------------------------------------------------------------ */

#[test]
fn no_verify_passes_invalid_typescript_through() {
    let opts = Options {
        verify: false,
        ..Options::default()
    };
    let out = compile("const = 5;\n", &opts).unwrap();
    assert_eq!(out, "const = 5;\n");
}

#[test]
fn filename_appears_in_error_display() {
    let opts = Options {
        filename: Some("demo.tt"),
        ..Options::default()
    };
    let e = compile("const r = match (x) { A => 1, A => 2 };", &opts).expect_err("expected error");
    assert_eq!(e.to_string(), "demo.tt:1:31: match: duplicate arm \"A\"");
}

/* ------------------------------------------------------------------ */
/* import specifier rewriting                                          */
/* ------------------------------------------------------------------ */

#[test]
fn the_std_specifier_is_left_alone_by_default() {
    // A bundler plugin resolves `@tt/std` itself, so the untouched
    // specifier is the right default.
    let src = "import type { TOption, TResult } from \"@tt/std\";\n\
import * as Option from \"@tt/std/option\";\n\
import * as Result from \"@tt/std/result\";\n";
    assert_eq!(ok(src), src);
}

#[test]
fn the_std_specifier_is_rewritten_when_the_caller_places_the_module() {
    let opts = Options {
        std_imports: ttc::StdImports {
            types: Some("../tt/index.js"),
            option: Some("../tt/option.js"),
            result: Some("../tt/result.js"),
            runtime: Some("../tt/runtime.js"),
            commonjs: None,
        },
        ..Options::default()
    };
    let out = compile(
        "import type { TOption } from '@tt/std';\n\
import * as Option from '@tt/std/option';\n\
import * as Result from '@tt/std/result';\n",
        &opts,
    )
    .unwrap();
    // The quote style survives; only the specifier's text changes.
    assert_eq!(
        out,
        "import type { TOption } from '../tt/index.js';\n\
import * as Option from '../tt/option.js';\n\
import * as Result from '../tt/result.js';\n"
    );
}

#[test]
fn the_std_specifier_is_not_a_project_module() {
    // It has no file to follow, so it is not part of the module graph the
    // CLI walks for declarations.
    assert!(ttc::tt_imports("import type { TOption } from \"@tt/std\";\n").is_empty());
    assert!(ttc::imports_std("export * from \"@tt/std/result\";\n"));
    assert!(!ttc::imports_std("import { Option } from \"./tt.js\";\n"));
}

#[test]
fn ts_mode_points_at_the_emitted_file() {
    // With `allowImportingTsExtensions` + `rewriteRelativeImportExtensions`,
    // tsc accepts `.ts` specifiers and rewrites them to `.js` on emit — so
    // ttc only has to name the file it actually produces.
    let opts = Options {
        rewrite_imports: ttc::ImportRewrite::Ts,
        ..Options::default()
    };
    let out = compile("import { E } from \"./error.tt\";\n", &opts).unwrap();
    assert_eq!(out, "import { E } from \"./error.ts\";\n");
}

#[test]
fn ts_mode_preserves_the_quote_style_and_path() {
    let opts = Options {
        rewrite_imports: ttc::ImportRewrite::Ts,
        ..Options::default()
    };
    let out = compile(
        "import a from './x.tt';\nexport * from \"../up/y.tt\";\n",
        &opts,
    )
    .unwrap();
    assert_eq!(
        out,
        "import a from './x.ts';\nexport * from \"../up/y.ts\";\n"
    );
}

#[test]
fn off_mode_leaves_the_specifier_untouched() {
    let opts = Options {
        rewrite_imports: ttc::ImportRewrite::Off,
        ..Options::default()
    };
    let src = "import { E } from \"./error.tt\";\n";
    assert_eq!(compile(src, &opts).unwrap(), src);
}

#[test]
fn import_equals_require_reference_is_rewritten() {
    for (source, expected) in [
        (
            "import fs = require(\"./legacy.tt\");\n",
            "import fs = require(\"./legacy.js\");\n",
        ),
        (
            "export import view = require('../view.ttx');\n",
            "export import view = require('../view.jsx');\n",
        ),
        (
            "import type T = require(\"./types.tt\");\n",
            "import type T = require(\"./types.js\");\n",
        ),
        (
            "import type = require(\"./named-type.tt\");\n",
            "import type = require(\"./named-type.js\");\n",
        ),
    ] {
        assert_eq!(ok(source), expected, "{source}");
    }
    let options = Options {
        rewrite_imports: ttc::ImportRewrite::Ts,
        ..Options::default()
    };
    assert_eq!(
        compile("import fs = require(\"./legacy.tt\");\n", &options).unwrap(),
        "import fs = require(\"./legacy.ts\");\n"
    );
}

/* ------------------------------------------------------------------ */
/* project-wide exhaustiveness (extern variants)                       */
/* ------------------------------------------------------------------ */

fn token_extern() -> ttc::ExternVariant {
    ttc::ExternVariant {
        name: "Token".to_string(),
        tags: ["Num", "Ident", "Eof"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        from: Some("./token.tt".to_string()),
    }
}

#[test]
fn extern_variant_makes_match_checked() {
    let externs = [token_extern()];
    let opts = Options {
        extern_variants: &externs,
        ..Options::default()
    };
    let e = compile(
        "const s = match (t) {\n  Num(value) => value,\n  Ident(name) => 0,\n};\n",
        &opts,
    )
    .expect_err("expected non-exhaustive error");
    assert!(
        e.message
            .contains("match on variant Token (imported from \"./token.tt\") is not exhaustive"),
        "{}",
        e.message
    );
    assert!(e.message.contains("missing \"Eof\""), "{}", e.message);
    assert_eq!((e.line, e.col), (1, 11));
}

#[test]
fn extern_variant_full_coverage_compiles() {
    let externs = [token_extern()];
    let opts = Options {
        extern_variants: &externs,
        ..Options::default()
    };
    let out = compile(
        "const s = match (t) { Num(value) => value, Ident(name) => 0, Eof => -1 };\n",
        &opts,
    )
    .unwrap();
    assert!(out.contains("switch ($tt_m.kind)"));
}

#[test]
fn literal_import_rewrite_matrix_preserves_surrounding_syntax() {
    let hosts = [
        "const load = () => import(SPEC);",
        "const load = () => import(/* before */ SPEC /* after */, {with: {type: 'json'}});",
        "type Module = typeof import(SPEC);",
        "type Value = import(SPEC).Value;",
        "export type Value = import(SPEC, {with: {'resolution-mode': 'import'}}).Value;",
        "async function load() { return (await import(SPEC)).value; }",
        "const load = `${import(SPEC)}`;",
    ];
    for kind in [ttc::SourceKind::TypeScript, ttc::SourceKind::Tsx] {
        for (extension, js, ts) in [("tt", "js", "ts"), ("ttx", "jsx", "tsx")] {
            for quote in ["'", "\""] {
                for host in hosts {
                    let path = format!("../feature.{extension}");
                    let source = host.replace("SPEC", &format!("{quote}{path}{quote}"));
                    for (mode, expected_extension) in [
                        (ttc::ImportRewrite::Js, js),
                        (ttc::ImportRewrite::Ts, ts),
                        (ttc::ImportRewrite::Off, extension),
                    ] {
                        let options = Options { source_kind: kind, rewrite_imports: mode, ..Options::default() };
                        let output = compile(&source, &options).unwrap();
                        assert_eq!(output, source.replace(&path, &format!("../feature.{expected_extension}")), "{source}");
                    }
                }
            }
        }
    }
}
