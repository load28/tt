#[test]
fn let_else_divergence_still_sees_block_statements() {
    // A *block* statement's `}` does end a statement, so the diverging
    // keyword after one is found — that is the half the object-literal fix
    // must not break.
    ok(
        "function f(): number {\n  const Some(v) = find() else { if (c) { log(\"x\"); } return 0; };\n  return v;\n}\n",
    );
    ok(
        "function f(): number {\n  const Some(v) = find() else { try { log(\"x\"); } catch (e) { log(\"y\"); } return 0; };\n  return v;\n}\n",
    );
    ok(
        "function f(): number {\n  const Some(v) = find() else { for (const x of xs) { log(x); } return 0; };\n  return v;\n}\n",
    );
    ok(
        "function f(): number {\n  const Some(v) = find() else { function g() { return 1; } return g(); };\n  return v;\n}\n",
    );
    // An arrow body and a declaration's initializer close nothing, and the
    // `;` after them is what starts the next statement.
    ok(
        "function f(): number {\n  const Some(v) = find() else { const g = () => { return 1; }; return g(); };\n  return v;\n}\n",
    );
    ok(
        "function f(): number {\n  const Some(v) = find() else { const o = { n: 1 }; return o.n; };\n  return v;\n}\n",
    );
}

#[test]
fn let_else_non_diverging_else_ending_in_a_brace_is_still_an_error() {
    // The flow answer must not turn every trailing `}` into a divergence:
    // an `if` without an `else` can fall through, a loop can run zero
    // times, an object literal is no statement at all.
    for body in [
        "const o = { n: 1 };",
        "if (c) { return 1; }",
        "for (const x of xs) { log(x); }",
    ] {
        let e = err(&format!(
            "function f(): number {{\n  const Some(v) = find() else {{ {body} }};\n  return v;\n}}\n"
        ));
        assert!(e.message.contains("must diverge"), "{body}: {}", e.message);
    }
}

#[test]
fn let_else_divergence_is_a_flow_answer_not_a_last_keyword_check() {
    // The CFG (TASK-125) accepts what the last-keyword heuristic wrongly
    // rejected: both-branch if/else, a diverging bare block, and code
    // after a `return` (unreachable, not a hole).
    for body in [
        "if (c) { return 1; } else { return 2; }",
        "if (c) { return 1; } else if (d) { throw e; } else { return 2; }",
        "if (c) return 1; else return 2;",
        "{ return 1; }",
        "return 0; log(\"never\");",
    ] {
        ok(&format!(
            "function f(): number {{\n  const Some(v) = find() else {{ {body} }};\n  return v;\n}}\n"
        ));
    }
}

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
fn let_else_divergence_sees_an_inline_if_let() {
    // TASK-172: an `if let` body and its `else` are inline, so an exit
    // written in either leaves the enclosing function — the statement
    // carries the block's divergence exactly as an `if` does.
    for body in [
        "if let Ok(value) = r { return value; } else { return 1; }",
        "if let Ok(value) = r { return value; } else { throw new Error(\"x\"); }",
        "if let Ok(value) = r { return value; } else if let Err(error) = r { throw new Error(error); } else { return 0; }",
        "if let Ok(value) = r { if let Ok(inner) = r { return inner; } else { return value; } } else { return 1; }",
        "if let Ok(value) = r { while (true) { return value; } } else { return 1; }",
    ] {
        ok(&format!(
            "variant Res {{ Ok(value: number), Err(error: string) }}\n\
             function f(r: Res): number {{\n  const Some(v) = find() else {{ {body} }};\n  return v;\n}}\n"
        ));
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

#[test]
fn let_else_non_diverging_else_is_error() {
    let e =
        err("function f(): number {\n  const Some(v) = find() else { log(); };\n  return v;\n}\n");
    assert!(e.message.contains("must diverge"), "{}", e.message);
    assert_eq!((e.line, e.col), (2, 26)); // points at the `else` keyword
}

#[test]
fn let_else_empty_else_block_is_error() {
    let e = err("function f(): number {\n  const Some(v) = find() else { };\n  return v;\n}\n");
    assert!(e.message.contains("must diverge"), "{}", e.message);
}

#[test]
fn let_else_inside_match_arm_is_error() {
    let e = err(
        "const x = match (r) {\n  Ok(value) => { const Some(v) = h(value) else { return 0; }; return v; },\n  _ => 0,\n};\n",
    );
    assert!(
        e.message.contains("let-else cannot be used here"),
        "{}",
        e.message
    );
}

#[test]
fn let_else_inside_a_function_inside_an_arm_body_is_allowed() {
    // The `else`'s `return` exits the arrow the user wrote — the same
    // flow fact that places `try`.
    let out = ok(
        "const x = match (r) {\n  Ok(value) => { const f = () => { const Some(v) = h(value) else { return 0; }; return v; }; return f(); },\n  _ => 0,\n};\n",
    );
    assert!(out.contains("if ($tt_t0.kind !== \"Some\")"), "{out}");
}

#[test]
fn let_else_without_semicolon_is_not_recognized() {
    // No terminating `;` → not tt syntax; the (invalid-TS) source passes
    // through and the output self-check reports it.
    let e = err(
        "function f(): number {\n  const Some(v) = find() else { return 0; }\n  return v;\n}\n",
    );
    assert!(
        e.message.contains("generated TypeScript failed to parse"),
        "{}",
        e.message
    );
}

#[test]
fn let_else_requires_parens_on_the_pattern() {
    // `const Point = e else { ... };` (no parens) is not tt syntax — the
    // invalid-TS text passes through to the output self-check.
    let e =
        err("function f(): number {\n  const Point = find() else { return 0; };\n  return 1;\n}\n");
    assert!(
        e.message.contains("generated TypeScript failed to parse"),
        "{}",
        e.message
    );
}

/* ------------------------------------------------------------------ */
/* source TypeScript that does not parse                               */
/* ------------------------------------------------------------------ */

#[test]
fn invalid_typescript_inside_a_claimed_construct_is_a_located_error() {
    // Host lowering models the file's TypeScript, so it needs the file to
    // *be* TypeScript. When it is not, that is a fact about the input
    // reported at the byte the parse stopped on — not an internal error
    // out of emission.
    let e = err("const r = result {\n  const a = try f();\n  const b = ;\n  return a;\n};\n");
    assert_eq!((e.line, e.col), (3, 13), "{}", e.message);
    assert!(
        e.message.contains("the TypeScript here does not parse"),
        "{}",
        e.message
    );
}

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
fn try_declaration_in_for_initializer_runs_before_the_loop() {
    let output = ok("function f() { for (let i = try next();;) {} }\n");
    let prelude = output.find("const $tt_t0 = next();").unwrap();
    let loop_header = output.find("for (let i = $tt_t0.value;;)").unwrap();
    assert!(prelude < loop_header, "{output}");
    assert!(
        compact(&output).contains("if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
        "{output}"
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
fn try_in_repeated_for_test_reports_a_located_lowering_diagnostic() {
    let source = "function f() { for (; try next(); ) {} }\n";
    let diagnostics = std::panic::catch_unwind(|| ttc::analyze(source, &Options::default()))
        .expect("repeated for-test propagation must not reach output verification");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == ttc::DiagnosticCode::LoweringPlanFailed)
        .unwrap_or_else(|| panic!("{diagnostics:#?}"));
    assert_eq!(
        diagnostic.start,
        Some(source.find("try").unwrap()),
        "{diagnostics:#?}"
    );
}

#[test]
fn try_assignment_in_for_initializer_reports_a_located_lowering_diagnostic() {
    let source = "function f() { for (i = try next();;) {} }\n";
    let diagnostics = ttc::analyze(source, &Options::default());
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == ttc::DiagnosticCode::LoweringPlanFailed),
        "{diagnostics:#?}"
    );
    assert_eq!(diagnostics[0].start, Some(source.find("try").unwrap()));
}

#[test]
fn a_file_without_tt_constructs_still_reports_through_the_output_self_check() {
    // Nothing to lower ⇒ no projection is built, and the backstop that
    // owned this case keeps owning it.
    let e = err("const q = ;\n");
    assert!(
        e.message.contains("generated TypeScript failed to parse"),
        "{}",
        e.message
    );
}

/* ------------------------------------------------------------------ */
/* swc output verification                                             */
/* ------------------------------------------------------------------ */

#[test]
fn verify_rejects_invalid_passthrough_typescript() {
    let e = err("const = 5;\n");
    assert!(
        e.message.contains("generated TypeScript failed to parse"),
        "{}",
        e.message
    );
}

#[test]
fn verify_does_not_invent_tt_intent_from_strings_or_comments() {
    for source in [
        "const note = 'try'; const = 5;\n",
        "/* match result flow */ const = 5;\n",
    ] {
        let e = err(source);
        assert!(
            e.message.contains("generated TypeScript failed to parse"),
            "{}",
            e.message
        );
        assert!(
            !e.message.contains("did not parse as a tt"),
            "{}",
            e.message
        );
    }
}

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
fn relative_tt_import_is_rewritten_to_js_by_default() {
    let out = ok("import { CalcError } from \"./error.tt\";\n");
    assert_eq!(out, "import { CalcError } from \"./error.js\";\n");
}

#[test]
fn rewrite_covers_all_static_import_forms() {
    let out = ok(r#"
import def from "./a.tt";
import def2, { named as alias } from "./b.tt";
import * as ns from "./c.tt";
import type { T } from "./d.tt";
import "./side.tt";
export { x, y as z } from "./e.tt";
export * from "./f.tt";
export * as g from "./g.tt";
export type { U } from "./h.tt";
"#);
    for stem in ["a", "b", "c", "d", "side", "e", "f", "g", "h"] {
        assert!(out.contains(&format!("\"./{stem}.js\"")), "{out}");
        assert!(!out.contains(&format!("\"./{stem}.tt\"")), "{out}");
    }
}

#[test]
fn rewrite_keeps_quote_style_and_parent_paths() {
    let out = ok("import a from './x.tt';\nimport b from \"../up/y.tt\";\n");
    assert_eq!(
        out,
        "import a from './x.js';\nimport b from \"../up/y.js\";\n"
    );
}

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
fn non_relative_tt_specifiers_are_untouched() {
    // Only relative paths are rewritten — package-like and absolute
    // specifiers keep their bytes.
    let src = "import a from \"pkg.tt\";\nimport b from \"/abs/x.tt\";\nimport c from \"@scope/p/x.tt\";\n";
    assert_eq!(ok(src), src);
}

#[test]
fn dynamic_import_and_import_meta_are_untouched() {
    let src = "const m = import(\"./x.tt\");\nconst u = import.meta.url;\n";
    assert_eq!(ok(src), src);
}

#[test]
fn import_assignment_is_untouched() {
    // TS import-assignment is not a static import declaration.
    let src = "import fs = require(\"./legacy.tt\");\n";
    assert_eq!(ok(src), src);
}

#[test]
fn rewrite_composes_with_tt_constructs_in_the_same_file() {
    let out = ok(r#"
import { CalcError } from "./error.tt";
variant Shape { Circle(radius: number), Point }
const area = match (Shape.Point) {
  Circle(radius) => radius,
  Point => 0,
};
"#);
    assert!(out.contains("\"./error.js\""), "{out}");
    assert!(out.contains("switch ($tt_m.kind)"), "{out}");
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
