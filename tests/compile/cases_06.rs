#[test]
fn flow_emits_nested_composition_helper_calls() {
    let out = ok("const f = flow |> parse |> double |> label;\nexport {};\n");
    assert!(
        out.contains("const f = $tt_fl($tt_fl(parse, double), label);"),
        "{out}"
    );
    assert!(out.contains("import { $tt_fl } from \"@tt/runtime\";"));
    // a composition is not a value pipeline — no apply helper
    assert!(!out.contains("$tt_ap"), "{out}");
}

#[test]
fn flow_runtime_is_imported_once_per_file() {
    let out = ok("const a = flow |> f |> g;\nconst b = flow |> h |> i;\nexport {};\n");
    assert_eq!(out.matches("$tt_fl(").count(), 2, "{out}");
    assert_eq!(out.matches("from \"@tt/runtime\"").count(), 1, "{out}");
}

/* ------------------------------------------------------------------ */
/* tuple match                                                         */
/* ------------------------------------------------------------------ */

#[test]
fn tuple_match_arity_mismatch_is_an_error() {
    let src = "const r = match (a, b) {\n  (A, B, C) => 1,\n  _ => 0,\n};\n";
    let e = err(src);
    assert!(
        e.message
            .contains("tuple pattern has 3 elements but the match has 2 scrutinees"),
        "{}",
        e.message
    );
    assert_eq!((e.line, e.col), (2, 3));
    let diagnostic = ttc::analyze(src, &Options::default())
        .into_iter()
        .find(|d| d.code == ttc::DiagnosticCode::MatchTupleArity)
        .unwrap();
    assert_eq!(
        &src[diagnostic.start.unwrap()..diagnostic.end.unwrap()],
        "(A, B, C)"
    );
}

#[test]
fn tuple_match_arity_mismatch_lowers_over_the_subjects_it_has() {
    // A tuple element is a subject index, so an arm naming more positions
    // than the match has scrutinees describes places that do not exist.
    // Lowering keeps only the positions with a subject: sema still reports
    // the arity, and every pass below stays total instead of indexing past
    // the decision's own subjects.
    for src in [
        "variant V { A, B }\ndeclare const a: V;\nconst r = match (a) {\n  (A, B) => 1,\n};\n",
        "variant V { A, B }\ndeclare const a: V;\ndeclare const b: V;\n\
         const r = match (a, b) {\n  (A, B, A) => 1,\n  _ => 0,\n};\n",
        "variant V { A, B }\ndeclare const a: V;\ndeclare const b: V;\n\
         const r = match (a, b) {\n  (A, B) => 1,\n  (B) => 2,\n  _ => 0,\n};\n",
    ] {
        let codes: Vec<_> = ttc::analyze(src, &Options::default())
            .into_iter()
            .map(|d| d.code)
            .collect();
        assert!(
            codes.contains(&ttc::DiagnosticCode::MatchTupleArity),
            "{codes:?}"
        );
        // The tooling emit is infallible by contract and an editor drives
        // it on every keystroke: it must answer, and answer with the
        // TypeScript its own self-check accepts.
        let emit = ttc::emit_mapped(src);
        assert!(
            ttc::compile(&emit.code, &Options::default()).is_ok(),
            "emitted output does not re-parse:\n{}",
            emit.code
        );
    }
}

#[test]
fn a_value_pattern_in_a_tuple_element_is_reported_at_that_element() {
    let cases = [
        (
            "declare const a: number;\ndeclare const b: string;\n\
             const r = match (a, b) {\n  (1, \"x\") => 1,\n  _ => 0,\n};\n",
            "1",
            "a literal pattern cannot be a tuple pattern element",
        ),
        (
            "variant O { Some(value: number), None }\ndeclare const a: O;\ndeclare const b: O;\n\
             const r = match (a, b) {\n  (Some(value), None) => value,\n  (None, 1 | 2) => 0,\n  _ => -1,\n};\n",
            "1 | 2",
            "a literal pattern cannot be a tuple pattern element",
        ),
        (
            "variant O { Some(value: number), None }\ndeclare const a: O;\ndeclare const n: unknown;\n\
             const r = match (a, n) {\n  (None, is Date) => 0,\n  _ => 1,\n};\n",
            "is Date",
            "an `is` pattern cannot be a tuple pattern element",
        ),
    ];
    for (src, element, message) in cases {
        let diagnostics = ttc::analyze(src, &Options::default());
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        let d = &diagnostics[0];
        assert_eq!(d.code, ttc::DiagnosticCode::MalformedMatch);
        assert_eq!(d.message, message);
        assert_eq!(&src[d.start.unwrap()..d.end.unwrap()], element);
        let help: Vec<&str> = d.suggestions.iter().map(|s| s.message.as_str()).collect();
        assert_eq!(
            help,
            ["tuple pattern elements are tag patterns or `_`; test this value in an arm guard or a nested `match`"]
        );
    }
}

#[test]
fn match_without_scrutinee_parentheses_is_a_malformed_tt_match() {
    let src = "const r = match value { A => 1, _ => 0 };\n";
    let e = err(src);
    assert!(e.message.contains("could not be parsed"), "{e}");
    assert_eq!((e.line, e.col), (1, 11));
    // The one malformed-match shape whose fix the parser can write: the
    // scrutinee text is there, it only lacks its parentheses.
    let d = &ttc::analyze(src, &Options::default())[0];
    assert_eq!(d.code, ttc::DiagnosticCode::MalformedMatch);
    let edit = d.suggestions[0].edit.as_ref().expect("an applicable edit");
    assert_eq!(&src[edit.start..edit.end], "value");
    assert_eq!(edit.replacement, "(value)");
    assert_eq!(
        with_suggestion_applied(src, d, 0),
        "const r = match (value) { A => 1, _ => 0 };\n"
    );
}

/* ------------------------------------------------------------------ */
/* if let                                                              */
/* ------------------------------------------------------------------ */

#[test]
fn a_stray_else_of_an_if_let_chain_is_reported_once_where_the_chain_stops() {
    // TASK-599: the chain is one statement, so its one failure is reported
    // once, at the `else` that cannot continue it, not at every `if let`
    // the parser met on the way.
    let source = "variant O { A(n: number), B(s: string) }\ndeclare const o: O;\ndeclare const c: boolean;\nfunction f() {\n  if let A(n) = o { g(n); } else if let B(s) = o { g(s); } else if (c) { h(); }\n}\n";
    let diagnostics = ttc::analyze(source, &Options::default());
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].code, DiagnosticCode::StrayIfLet);
    assert_eq!(
        diagnostics[0].start,
        Some(source.rfind("else if (c)").unwrap()),
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics[0].end,
        Some(source.rfind("else if (c)").unwrap() + "else if".len()),
        "{diagnostics:#?}"
    );

    let source = "variant O { A(n: number), B(s: string) }\ndeclare const o: O;\nfunction f() {\n  if let A(n) = o { g(n); } else if let B = o { h(); }\n}\n";
    let diagnostics = ttc::analyze(source, &Options::default());
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        diagnostics[0].start,
        Some(source.rfind("if let B").unwrap()),
        "{diagnostics:#?}"
    );
}

/* ------------------------------------------------------------------ */
/* concise arrow bodies                                                */
/* ------------------------------------------------------------------ */

#[test]
fn a_tt_value_anywhere_in_a_concise_arrow_body_keeps_the_block_balanced() {
    // Lowering rewrites a concise arrow body to a block, so the block has
    // to close where that body ends — not where the tt value ends, and not
    // never. `compile` runs the output self-check, so an unbalanced or
    // early brace fails here.
    let head = "variant Shape { Circle(radius: number), Point }\ndeclare const s: Shape;\n\
                declare function f(a: number, b: number): number;\n";
    let matched = "match (s) { Circle(radius) => radius, Point => 0 }";
    for body in [
        "M",
        "x + M",
        "M + x",
        "x + M + x",
        "M + x + x",
        "x && M",
        "x ? M : 0",
        "[x, M]",
        "f(x, M)",
    ] {
        let src = format!(
            "{head}export const v = [1].map((x: number) => {});\n",
            body.replace('M', matched)
        );
        let out = ok(&src);
        assert_eq!(
            out.matches('{').count(),
            out.matches('}').count(),
            "unbalanced braces for `{body}`:\n{out}"
        );
    }
}

#[test]
fn a_payload_field_cannot_be_named_like_the_case_tag_s_property() {
    // Every case carries its tag in one fixed property. A payload field of
    // that name has nowhere to go: the declaration would name the property
    // twice, and the constructor would write the payload over the tag, so
    // the value could no longer say which case it is.
    let e = err("variant Token { Word(kind: string) }\n");
    assert!(e.message.contains("cannot have a field named `kind`"), "{e}");
    assert_eq!((e.line, e.col), (1, 22));
    let diagnostic = &ttc::analyze("variant Token { Word(kind: string) }\n", &Options::default())[0];
    assert_eq!(diagnostic.code, ttc::DiagnosticCode::VariantFieldShadowsTag);

    // Any other field name is fine, and the tag itself is untouched.
    let out = ok("variant Token { Word(text: string) }\n");
    assert!(out.contains("{ kind: \"Word\"; text: string }"), "{out}");
}

#[test]
fn val_writes_follow_assignment_targets_not_neighboring_tokens() {
    for statement in ["cfg.a = other;", "(cfg).a = other;", "((cfg.a)) = other;", "[cfg.a] = other;", "({ a: cfg.a } = other);", "({ a: [cfg.a = 1] } = other);", "[...cfg.a] = other;"] {
        let source = format!("val const cfg = {{ a: 1 }};\n{statement}\n");
        let diagnostics = ttc::analyze(&source, &Options::default());
        assert_eq!(diagnostics.iter().filter(|d| d.code == ttc::DiagnosticCode::ValMutation).count(), 1, "{source}: {diagnostics:?}");
        assert_eq!(ttc::val_probes(&source).mutations.iter().filter(|m| m.method.is_none()).count(), 1, "{source}");
    }
    for statement in ["const x = cfg.a in other;", "({ [cfg.a]: other.a } = value);", "[other.a = cfg.a] = value;", "function f(cfg: any) { (cfg).a = 2; }"] {
        let source = format!("val const cfg = {{ a: 1 }};\n{statement}\n");
        assert!(!ttc::analyze(&source, &Options::default()).iter().any(|d| d.code == ttc::DiagnosticCode::ValMutation), "{source}");
    }
}

#[test]
fn a_required_variant_field_cannot_follow_an_optional_one() {
    let src = "variant W { C(opt?: number, req: string, more: boolean), D(a?: number, b?: string) }\n";
    let diagnostics = ttc::analyze(src, &Options::default());
    let found = diagnostics
        .iter()
        .filter(|d| d.code == ttc::DiagnosticCode::VariantRequiredAfterOptional)
        .map(|d| (d.start, d.end))
        .collect::<Vec<_>>();
    let at = |name: &str| src.find(name).map(|start| (Some(start), Some(start + name.len())));
    assert_eq!(found, [at("req").unwrap(), at("more").unwrap()], "{diagnostics:?}");
    let e = err(src);
    assert!(e.message.contains("required field `req` after optional field `opt`"), "{e}");
}
