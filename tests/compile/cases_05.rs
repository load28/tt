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
fn tt_imports_reports_specifiers_and_names() {
    use ttc::TtImportNames;
    let imports = ttc::tt_imports(
        r#"
import { Token, Kind as K, type T } from "./a.tt";
import * as ns from "../b.tt";
import "./side.tt";
export { X } from "./re.tt";
import { skip } from "./not-tt.ts";
"#,
    );
    assert_eq!(imports.len(), 4);
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
    let out = ok("const y = half(4) |> double |> label;\n");
    assert!(
        out.contains("const y = $tt_ap($tt_ap(half(4), double), label);"),
        "{out}"
    );
    assert!(out.contains("import { $tt_ap } from \"@tt/runtime\";"));
}

#[test]
fn pipeline_method_step_chains_postfix() {
    let out = ok("const t = s |> .trim() |> .split(\",\") |> f;\n");
    assert!(
        out.contains("const t = $tt_ap(s.trim().split(\",\"), f);"),
        "{out}"
    );
}

#[test]
fn a_lowering_is_laid_out_from_the_line_it_replaces() {
    // Generated block structure indents from the statement the construct
    // sits on, at every nesting depth, so the output reads as TypeScript
    // written where the tt source was.
    let out = ok(
        "variant E { A(v: number), B }\ndeclare const e: E;\nfunction f(): number {\n  if (true) {\n    const r = match (e) { A(v) => v, B => 0 };\n    return r;\n  }\n  return 0;\n}\n",
    );
    assert!(out.contains("\n    let $tt_v0;\n    {\n"), "{out}");
    assert!(out.contains("\n      const $tt_m = e;"), "{out}");
    assert!(out.contains("\n      switch ($tt_m.kind) {\n"), "{out}");
    assert!(
        compact(&out).contains("case \"A\": { const { v } = $tt_m; $tt_v0 = v; break; }"),
        "{out}"
    );
    assert!(out.contains("\n    }\n    const r = $tt_v0;"), "{out}");
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
fn a_nested_variant_declaration_is_laid_out_from_its_own_line() {
    let out = ok("function make() {\n  variant Inner { A(x: number), B }\n  return Inner.B;\n}\n");
    assert!(
        out.contains("\n  type Inner =\n    | { kind: \"A\"; x: number }"),
        "{out}"
    );
    assert!(
        out.contains("\n  const Inner = {\n    A: (x: number): Inner => ({ kind: \"A\", x }),"),
        "{out}"
    );
    assert!(out.contains("\n  };\n  return Inner.B;"), "{out}");
}

#[test]
fn a_delivered_value_keeps_only_the_parentheses_that_group_it() {
    // Everything but the comma operator binds tighter than the position a
    // lowered value lands in, so the parentheses go where they mean
    // something and nowhere else.
    let out = ok(
        "variant E { A(v: number), B }\ndeclare const e: E;\nconst plain = match (e) { A(v) => v + 1, B => 0 };\nconst seq = match (e) { A(v) => (v, v + 1), B => 0 };\n",
    );
    let compact = compact(&out);
    assert!(compact.contains("$tt_v0 = v + 1; break;"), "{out}");
    assert!(compact.contains("$tt_v0 = 0; break;"), "{out}");
    assert!(compact.contains("$tt_v1 = (v, v + 1); break;"), "{out}");
}

#[test]
fn generated_control_flow_uses_statement_lines_and_expanded_blocks() {
    let out = ok(
        "variant E { A(v: number), B }\nfunction f(e: E): Result<number, string> {\n  const value = try read();\n  const matched = match (e) { A(v) => v, B => 0 };\n  return Result.Ok(value + matched);\n}\nfunction block(e: E): number {\n  return match (e) {\n    A(v) if v > 0 => {\n      const doubled = v * 2;\n      return doubled;\n    },\n    _ => 0,\n  };\n}\nfunction bind(e: E): number {\n  const A(v) = e else {\n    return 0;\n  };\n  return v;\n}\nconst computed = result {\n  return try read();\n};\n",
    );
    for compressed in ["; if (", "; const ", "; break"] {
        assert!(
            !out.lines().any(|line| line.contains(compressed)),
            "generated statements share a line through {compressed:?}:\n{out}"
        );
    }
    assert!(
        out.contains(
            "case \"A\": {\n        const { v } = $tt_m;\n        $tt_v0 = v;\n        break;\n      }"
        ),
        "{out}"
    );
    assert!(
        out.contains("if (!(\"value\" in $tt_t0)) {\n    return $tt_t0;\n  }"),
        "{out}"
    );
    // Source-backed block contents keep their authored column. Rewritten
    // exits follow that column instead of the generated wrapper brace.
    assert!(
        out.contains(
            "          {\n      const doubled = v * 2;\n      $tt_v1 = doubled;\n      break;\n          }"
        ),
        "{out}"
    );
    assert!(
        out.contains("if ($tt_t1.kind !== \"A\") {\n    return 0;\n  }"),
        "{out}"
    );
    assert!(
        out.contains(
            "if (!(\"value\" in $tt_t2)) {\n    $tt_v2 = $tt_t2;\n    break $tt_v2;\n  }\n  $tt_v2 = { kind: \"Ok\" as const, value: $tt_t2.value };\n  break $tt_v2;"
        ),
        "{out}"
    );
}

#[test]
fn a_postfix_step_parenthesizes_only_a_receiver_that_needs_it() {
    // Member access binds tighter than every operator: a primary receiver
    // can lose the parentheses, `await p` and `a + b` cannot.
    let out = ok(
        "const a = s |> .trim();\nconst b = (x + y) |> .toFixed(2);\nconst c = await p |> .then(g);\n",
    );
    assert!(out.contains("const a = s.trim();"), "{out}");
    assert!(out.contains("const b = (x + y).toFixed(2);"), "{out}");
    assert!(out.contains("const c = (await p).then(g);"), "{out}");
}

#[test]
fn pipeline_runtime_is_imported_once_per_file() {
    let out = ok("const a = x |> f;\nconst b = y |> g;\n");
    assert_eq!(out.matches("$tt_ap(").count(), 2, "{out}");
    assert_eq!(out.matches("from \"@tt/runtime\"").count(), 1, "{out}");
}

#[test]
fn an_inert_pipeline_input_uses_a_direct_call() {
    let out = ok("const value = 1 |> String;\n");
    assert!(out.contains("const value = String(1);"), "{out}");
    assert!(!out.contains("$tt_ap"), "{out}");
}

#[test]
fn a_materialized_pipeline_accumulator_uses_a_direct_call() {
    let out = ok("variant E { A(value: number), B }\n\
         const value = match (E.A(1)) { A(value) => value, B => 0 } |> String;\n");
    assert!(out.contains("$tt_v0 = String($tt_v0);"), "{out}");
    assert!(!out.contains("$tt_ap"), "{out}");
}

#[test]
fn file_without_pipeline_gets_no_helper() {
    let out = ok("const a = f(x);\n");
    assert!(!out.contains("$tt_ap"), "{out}");
}

#[test]
fn pipeline_head_reclaims_a_lifted_template() {
    // The template token is lifted as a segment before the `|>` is seen —
    // the claim must rewind it into the head sub-program.
    let out = ok("const a = `v=${n}` |> f;\n");
    assert!(out.contains("const a = $tt_ap(`v=${n}`, f);"), "{out}");
}

#[test]
fn an_inert_member_receiver_needs_no_receiver_slot() {
    let out = ok("variant E { A(value: string), B }\n\
         const value = \"abc\".replace(\
           match (E.A(\"a\")) { A(value) => value, B => \"b\" },\
           \"x\",\
         );\n");
    assert!(out.contains("(\"abc\".replace).bind(\"abc\")"), "{out}");
    assert_eq!(out.matches("const $tt_v").count(), 1, "{out}");
}

#[test]
fn pipeline_head_reclaims_a_lifted_match() {
    let out = ok(
        "variant E { A(v: number), B }\nconst a = match (e) { A(v) => v, B => 0, } |> double;\n",
    );
    assert!(!out.contains("(() =>"), "{out}");
    assert!(out.contains("switch ($tt_m.kind)"), "{out}");
    assert!(out.contains("$tt_v0 = double($tt_v0);"), "{out}");
    assert!(out.contains("const a = $tt_v0;"), "{out}");
}

#[test]
fn pipeline_head_is_the_whole_call_not_the_inner_argument() {
    // Bracket tracking must restore the enclosing expression's start:
    // the head of `a(b) |> g` is `a(b)`, not `b`.
    let out = ok("const y = f(a(b) |> g);\n");
    assert!(out.contains("const y = f($tt_ap(a(b), g));"), "{out}");
}

#[test]
fn pipeline_inside_match_scrutinee_arm_and_template() {
    let out = ok(
        "variant E { A(v: number), B }\nconst r = match (x |> norm) {\n  A(v) => v |> double,\n  B => 0,\n};\nconst t = `n=${x |> f}`;\n",
    );
    assert!(out.contains("const $tt_m = $tt_ap(x, norm);"), "{out}");
    assert!(
        compact(&out).contains("$tt_v0 = $tt_ap(v, double); break;"),
        "{out}"
    );
    assert!(out.contains("`n=${$tt_ap(x, f)}`"), "{out}");
}

#[test]
fn pipeline_composes_with_try() {
    let out = ok(
        "function f(): Result<number, string> {\n  const a = try readCfg() |> norm;\n  return Result.Ok(a);\n}\n",
    );
    assert!(
        out.contains("const $tt_t0 = $tt_ap(readCfg(), norm);"),
        "{out}"
    );
}

#[test]
fn pipeline_await_in_head_needs_no_async_wrapper() {
    let out = ok("async function f(p: Promise<string>) {\n  return await p |> norm;\n}\n");
    assert!(out.contains("return $tt_ap(await p, norm);"), "{out}");
    assert!(!out.contains("async () =>"), "{out}");
}

#[test]
fn the_runtime_import_is_written_where_an_import_belongs() {
    // Which helpers a file needs is only known once it is emitted; where
    // an import goes is not a question about that (TASK-219).
    let out = ok(
        "declare function f(n: number): number;\ndeclare function g(n: number): number;\nexport const a = f(4) |> g |> .toFixed(1);\n",
    );
    assert!(out.starts_with("import { $tt_ap } from "), "{out}");
}

#[test]
fn the_runtime_import_never_displaces_a_directive_or_a_shebang() {
    // A directive is only a directive while nothing precedes it, so an
    // import above one would turn it into a string expression and a
    // bundler would stop seeing the boundary the author declared.
    let out = ok(
        "\"use client\";\ndeclare function f(n: number): number;\ndeclare function g(n: number): number;\nexport const a = f(4) |> g |> .toFixed(1);\n",
    );
    assert!(
        out.starts_with("\"use client\";\nimport { $tt_ap } from "),
        "{out}"
    );

    let out = ok(
        "#!/usr/bin/env node\ndeclare function f(n: number): number;\ndeclare function g(n: number): number;\nexport const a = f(4) |> g |> .toFixed(1);\n",
    );
    assert!(
        out.starts_with("#!/usr/bin/env node\nimport { $tt_ap } from "),
        "{out}"
    );

    // A leading string that is *not* a directive is an expression, and an
    // import may precede it.
    let out = ok(
        "declare const b: string;\n\"a\" + b;\ndeclare function f(n: number): number;\nexport const a = f(4) |> f |> .toFixed(1);\n",
    );
    assert!(out.starts_with("import { $tt_ap } from "), "{out}");
}

#[test]
fn a_block_arm_keeps_the_layout_its_author_wrote() {
    // The lowering writes the braces around a block arm's body, so the
    // line break and indentation after the author's own `{` is what the
    // rest of their block lines up against — dropping it put the first
    // statement in one column and the rest in another (TASK-219).
    let out = ok(
        "variant E { A(n: number), B }\ndeclare const e: E;\nconst v = match (e) {\n  A(n) => {\n    const m = n + 1;\n    return m;\n  },\n  B => 0,\n};\n",
    );
    let block: Vec<&str> = out
        .lines()
        .skip_while(|line| !line.contains("const m = n + 1;"))
        .take(2)
        .collect();
    assert_eq!(block.len(), 2, "{out}");
    let indent = |line: &str| line.len() - line.trim_start().len();
    assert_eq!(indent(block[0]), indent(block[1]), "{out}");
}

#[test]
fn unparenthesized_ternary_next_to_pipeline_is_an_error() {
    let src = "const a = c ? x : y |> f;\n";
    let e = err(src);
    // The message says what is wrong; how to fix it rides in the
    // suggestion channel, where every rule's advice lives (TASK-218).
    assert!(e.message.contains("could not be parsed"), "{}", e.message);
    assert_eq!((e.line, e.col), (1, 21));
    assert!(
        advice(src).iter().any(|a| a.contains("parenthesize")),
        "{:?}",
        advice(src)
    );
}

#[test]
fn parenthesized_ternary_head_compiles() {
    let out = ok("const a = (c ? x : y) |> f;\n");
    assert!(out.contains("$tt_ap((c ? x : y), f"), "{out}");
}

#[test]
fn unparenthesized_arrow_step_is_an_error() {
    let src = "const a = x |> n => n + 1;\n";
    let e = err(src);
    assert!(e.message.contains("could not be parsed"), "{}", e.message);
    assert!(
        advice(src).iter().any(|a| a.contains("parenthesize")),
        "{:?}",
        advice(src)
    );
}

#[test]
fn empty_or_dangling_step_is_an_error() {
    let e = err("const a = x |>;\n");
    assert!(e.message.contains("could not be parsed"), "{}", e.message);
    let e = err("const a = x |> |> f;\n");
    assert!(e.message.contains("could not be parsed"), "{}", e.message);
}

#[test]
fn optional_postfix_step_emits_the_complete_chain() {
    let out = ok("const a = x |> ?.trim();\n\
         const b = xs |> ?.[key]?.value;\n\
         const c = fn |> ?.(arg).value?.();\n");
    assert!(out.contains("const a = x?.trim();"), "{out}");
    assert!(out.contains("const b = xs?.[key]?.value;"), "{out}");
    assert!(out.contains("const c = fn?.(arg).value?.();"), "{out}");
}

#[test]
fn optional_postfix_uses_the_common_receiver_grouping_rule() {
    let out = ok("const a = value |> ?.member;\n\
         const b = left + right |> ?.member;\n\
         const c = make() |> ?.member;\n");
    assert!(out.contains("const a = value?.member;"), "{out}");
    assert!(out.contains("const b = (left + right)?.member;"), "{out}");
    assert!(out.contains("const c = make()?.member;"), "{out}");
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

#[test]
fn try_inside_a_function_inside_a_pipeline_step_is_allowed() {
    let out = ok("const a = x |> (n => { const b = try f(n); return b; });\n");
    assert!(
        compact(&out).contains("if (!(\"value\" in $tt_t0)) { return $tt_t0; }"),
        "{out}"
    );
}

/* ------------------------------------------------------------------ */
/* flow (function composition)                                         */
/* ------------------------------------------------------------------ */
