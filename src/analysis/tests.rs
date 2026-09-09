use super::*;

/// The witnesses of a coverage as plain rows, for comparison.
fn patterns(coverage: &Coverage) -> Vec<Vec<&str>> {
    coverage
        .missing
        .iter()
        .map(|m| m.pattern.iter().map(String::as_str).collect())
        .collect()
}

fn at(src: &str, needle: &str, delta: usize) -> usize {
    src.find(needle).expect("needle") + delta
}

/// `A(x)`-style lookup: the binding two bytes into the needle.
fn binding<'a>(
    analyses: &'a PatternAnalyses,
    src: &str,
    needle: &str,
    delta: usize,
) -> &'a PatternBinding {
    analyses
        .binding_at(at(src, needle, delta))
        .unwrap_or_else(|| panic!("no binding at {needle:?}+{delta}"))
}

#[test]
fn single_constructor_binding_and_body_share_the_payload_type() {
    let src = "variant E { A(x: string), B }\nconst v = match (e) { A(x) => x, B => 0 };\n";
    let analyses = pattern_analyses(src, &[]);
    let b = binding(&analyses, src, "A(x)", 2);
    assert_eq!(b.ty.as_deref(), Some("string"));
    assert_eq!(b.tag, "A");
    assert_eq!(b.variant_name.as_deref(), Some("E"));
    assert_eq!(b.alternatives, 1);
    let arm = &analyses.matches[0].arms[0];
    assert_eq!(arm.body_bindings.len(), 1);
    assert_eq!(arm.body_bindings[0].name, "x");
    assert_eq!(arm.body_bindings[0].ty.as_deref(), Some("string"));
}

#[test]
fn or_pattern_occurrences_keep_their_own_types_and_the_body_merges_them() {
    let src =
        "variant E { A(x: string), B(x: number) }\nconst v = match (e) { A(x) | B(x) => x };\n";
    let analyses = pattern_analyses(src, &[]);
    let a = binding(&analyses, src, "A(x)", 2);
    let b = binding(&analyses, src, "B(x)", 2);
    assert_eq!(a.ty.as_deref(), Some("string"));
    assert_eq!(b.ty.as_deref(), Some("number"));
    assert_eq!((a.alternatives, b.alternatives), (2, 2));
    // Both occurrences share the group span; each keeps its own
    // alternative span.
    assert_eq!(&src[a.group_start..a.group_end], "A(x) | B(x)");
    assert_eq!(&src[a.alt_start..a.alt_end], "A(x)");
    assert_eq!(&src[b.alt_start..b.alt_end], "B(x)");
    let body = &analyses.matches[0].arms[0].body_bindings;
    assert_eq!(body[0].ty.as_deref(), Some("string | number"));
}

#[test]
fn agreeing_alternatives_merge_to_a_single_type() {
    let src =
        "variant E { A(x: string), B(x: string) }\nconst v = match (e) { A(x) | B(x) => x };\n";
    let analyses = pattern_analyses(src, &[]);
    assert_eq!(
        analyses.matches[0].arms[0].body_bindings[0].ty.as_deref(),
        Some("string")
    );
}

#[test]
fn aliases_bind_the_alias_span_and_optional_fields_widen() {
    let src = "variant E { A(v?: string), B(v: number) }\nconst r = match (e) { A(v: x) | B(v: x) => x };\n";
    let analyses = pattern_analyses(src, &[]);
    let a = binding(&analyses, src, "A(v: x)", 5);
    assert_eq!(a.name, "x");
    assert_eq!(a.ty.as_deref(), Some("string | undefined"));
    // The field-name span is not a binding.
    assert!(analyses.binding_at(at(src, "A(v: x)", 2)).is_none());
    let body = &analyses.matches[0].arms[0].body_bindings;
    assert_eq!(body[0].ty.as_deref(), Some("string | undefined | number"));
}

#[test]
fn nested_patterns_resolve_through_the_field_type() {
    let src = "variant Inner { Some(value: number), None }\nvariant E { A(o: Inner), B(o: Inner) }\nconst v = match (e) { A(o: Some(value)) => value, B(o: None()) => 0 };\n";
    let analyses = pattern_analyses(src, &[]);
    let x = binding(&analyses, src, "Some(value)", 5);
    assert_eq!(x.ty.as_deref(), Some("number"));
    assert_eq!(x.tag, "Some");
    assert_eq!(x.variant_name.as_deref(), Some("Inner"));
    assert_eq!(
        analyses.matches[0].arms[0].body_bindings[0].ty.as_deref(),
        Some("number")
    );
}

#[test]
fn generic_field_types_resolve_their_base_variant() {
    let src = "variant E { A(o: Option<number>) }\nconst v = match (e) { A(o: Some(value)) => value, _ => 0 };\n";
    let analyses = pattern_analyses(src, &[]);
    let x = binding(&analyses, src, "Some(value)", 5);
    // Declared, not instantiated: the checker's answer supersedes this.
    assert_eq!(x.ty.as_deref(), Some("T"));
    assert_eq!(x.variant_name.as_deref(), Some("Option"));
}

#[test]
fn tuple_elements_resolve_per_position() {
    let src = "variant L { A(x: string), B }\nvariant R { C(y: number), D }\nconst v = match (l, r) { (A(x) | B, C(y) | D) => 0, _ => 1 };\n";
    let analyses = pattern_analyses(src, &[]);
    let x = binding(&analyses, src, "A(x)", 2);
    let y = binding(&analyses, src, "C(y)", 2);
    assert_eq!(x.ty.as_deref(), Some("string"));
    assert_eq!(y.ty.as_deref(), Some("number"));
    assert_eq!(&src[x.group_start..x.group_end], "A(x) | B");
    let m = &analyses.matches[0];
    assert_eq!(m.subjects.len(), 2);
    assert_eq!(m.subjects[0].as_ref().unwrap().variant_name, "L");
    assert_eq!(m.subjects[1].as_ref().unwrap().variant_name, "R");
}

#[test]
fn builtins_answer_and_locals_shadow_them() {
    let src = "const v = match (o) { Some(value) => value, None => 0 };\n";
    let analyses = pattern_analyses(src, &[]);
    let value = binding(&analyses, src, "Some(value)", 5);
    assert_eq!(value.ty.as_deref(), Some("T"));
    assert_eq!(value.variant_name.as_deref(), Some("Option"));

    let shadowed = "variant Option { Some(value: string), None }\nconst v = match (o) { Some(value) => value, None => 0 };\n";
    let analyses = pattern_analyses(shadowed, &[]);
    let value = binding(&analyses, shadowed, "Some(value)", 5);
    assert_eq!(value.ty.as_deref(), Some("string"));
}

#[test]
fn extern_declarations_answer_under_their_in_scope_names() {
    let externs = vec![VariantSymbol {
        name: "T".to_string(),
        offset: 0,
        exported: true,
        generics: String::new(),
        cases: vec![
            crate::VariantCaseSymbol {
                tag: "Num".to_string(),
                offset: 0,
                fields: Some(vec![crate::VariantFieldSymbol {
                    name: "value".to_string(),
                    offset: 0,
                    optional: false,
                    ty: "number".to_string(),
                }]),
            },
            crate::VariantCaseSymbol {
                tag: "Eof".to_string(),
                offset: 0,
                fields: None,
            },
        ],
    }];
    let src = "const v = match (t) { Num(value) => value, Eof => 0 };\n";
    let analyses = pattern_analyses(src, &externs);
    let value = binding(&analyses, src, "Num(value)", 4);
    assert_eq!(value.ty.as_deref(), Some("number"));
    assert_eq!(value.variant_name.as_deref(), Some("T"));
}

#[test]
fn an_unresolved_subject_keeps_spans_but_knows_no_types() {
    let src = "const v = match (e) { What(x) | Ever(x) => x };\n";
    let analyses = pattern_analyses(src, &[]);
    let x = binding(&analyses, src, "What(x)", 5);
    assert_eq!(x.ty, None);
    assert_eq!(x.alternatives, 2);
    assert_eq!(analyses.matches[0].subjects[0], None);
    assert_eq!(analyses.matches[0].arms[0].body_bindings[0].ty, None);
}

#[test]
fn coverage_mirrors_the_exhaustiveness_rule() {
    let src =
        "variant E { A(s: string), B, C }\nconst v = match (e) { A(x) => x, B if f() => 1 };\n";
    let analyses = pattern_analyses(src, &[]);
    let coverage = analyses.matches[0].coverage.as_ref().unwrap();
    assert_eq!(coverage.covered, ["A"]);
    // The guarded `B` arm identifies the variant but covers nothing.
    assert_eq!(coverage.missing_tags(), ["B", "C"]);
    assert_eq!(
        coverage.positions[0].as_ref().map(|e| (&e.name, &e.origin)),
        Some((&"E".to_string(), &Origin::Local))
    );

    let with_wildcard = "variant E { A, B }\nconst v = match (e) { A => 0, _ => 1 };\n";
    assert_eq!(
        pattern_analyses(with_wildcard, &[]).matches[0].coverage,
        None
    );
}

#[test]
fn coverage_prefers_the_candidate_the_arms_satisfy() {
    // Both variants contain every arm tag; `Small` is fully covered, so the
    // match is exhaustive even though `Big` is missing a case. This is
    // the rule sema has always reported — an arm set that satisfies
    // *some* candidate is not a missing-case error.
    let src = "variant Big { A(s: string), B, C }\nvariant Small { A(s: string), B }\nconst v = match (e) { A(x) => x, B => 1 };\n";
    let coverage = pattern_analyses(src, &[]).matches[0]
        .coverage
        .clone()
        .expect("resolved");
    assert!(coverage.missing.is_empty());
    assert_eq!(coverage.positions[0].as_ref().unwrap().name, "Small");

    // With no satisfied candidate, the one left fewest cases is named.
    let unsatisfied = "variant Big { A(s: string), B, C, D }\nvariant Small { A(s: string), B, C }\nconst v = match (e) { A(x) => x, B => 1 };\n";
    let coverage = pattern_analyses(unsatisfied, &[]).matches[0]
        .coverage
        .clone()
        .expect("resolved");
    assert_eq!(coverage.positions[0].as_ref().unwrap().name, "Small");
    assert_eq!(coverage.missing_tags(), ["C"]);
}

#[test]
fn coverage_of_an_imported_variant_carries_its_specifier() {
    let src = "import { Token } from \"./token.tt\";\nconst v = match (t) { Word => 0 };\n";
    let externs = [ExternVariant {
        name: "Token".to_string(),
        tags: vec!["Word".to_string(), "Punct".to_string()],
        from: Some("./token.tt".to_string()),
    }];
    let program = crate::parser::parse(src);
    let analyses = coverage_analyses(src, &program, &externs);
    let coverage = analyses.matches[0].coverage.as_ref().unwrap();
    assert_eq!(coverage.missing_tags(), ["Punct"]);
    assert_eq!(
        coverage.positions[0].as_ref().unwrap().origin,
        Origin::Imported {
            from: Some("./token.tt".to_string())
        }
    );
    // Coverage-only analyses skip binding work entirely.
    assert!(analyses.matches[0].arms[0].pattern_bindings.is_empty());
}

#[test]
fn tuple_coverage_is_the_product_of_its_positions() {
    let src = "variant A { X(v: number), Y }\nvariant B { P(v: number), Q }\nconst v = match (a, b) { (X, P) => 0, (Y, _) => 1 };\n";
    let coverage = pattern_analyses(src, &[]).matches[0]
        .coverage
        .clone()
        .expect("resolved");
    let names: Vec<&str> = coverage
        .positions
        .iter()
        .map(|p| p.as_ref().map_or("_", |e| e.name.as_str()))
        .collect();
    assert_eq!(names, ["A", "B"]);
    // (X, Q) is the only combination no arm handles.
    assert_eq!(patterns(&coverage), [["X", "Q"]]);
    // A tuple arm covers a combination, not a tag.
    assert!(coverage.covered.is_empty());
    assert!(coverage.missing_tags().is_empty());
}

#[test]
fn a_universal_tuple_position_shows_as_a_hole() {
    // Nothing is ever written at position 1, so it constrains nothing.
    let src = "variant A { X(v: number), Y }\nconst v = match (a, b) { (X, _) => 0 };\n";
    let coverage = pattern_analyses(src, &[]).matches[0]
        .coverage
        .clone()
        .expect("resolved");
    assert_eq!(coverage.positions[1], None);
    assert_eq!(patterns(&coverage), [["Y", "_"]]);

    // A bare `_` arm covers everything; there is nothing to enumerate.
    let bare = "variant A { X(v: number), Y }\nconst v = match (a, b) { (X, _) => 0, _ => 1 };\n";
    assert_eq!(pattern_analyses(bare, &[]).matches[0].coverage, None);
}

#[test]
fn body_definitions_answer_the_innermost_arm() {
    let src =
        "variant E { A(x: string), B(x: number) }\nconst v = match (e) { A(x) | B(x) => x };\n";
    let analyses = pattern_analyses(src, &[]);
    let body_x = src.rfind("=> x").unwrap() + 3;
    let spans = analyses.body_definitions(src, body_x);
    assert_eq!(spans.len(), 2);
    assert_eq!(&src[spans[0].0..spans[0].1], "x");
    assert_eq!(&src[spans[1].0..spans[1].1], "x");
    assert!(spans[0].0 < spans[1].0);
    // A name the arm does not bind answers nothing.
    assert!(
        analyses
            .body_definitions(src, at(src, "match (e)", 7))
            .is_empty()
    );
}

#[test]
fn body_binding_lookup_merges_like_the_body_map() {
    let src =
        "variant E { A(x: string), B(x: number) }\nconst v = match (e) { A(x) | B(x) => x };\n";
    let analyses = pattern_analyses(src, &[]);
    let body_x = src.rfind("=> x").unwrap() + 3;
    let (b, span) = analyses.body_binding_at(src, body_x).unwrap();
    assert_eq!(b.ty.as_deref(), Some("string | number"));
    assert_eq!(&src[span.0..span.1], "x");
    assert!(analyses.body_binding_at(src, 0).is_none());
}

#[test]
fn let_else_and_if_let_are_pattern_sites_with_typed_bindings() {
    let src = "variant E { A(x: string), B }\n\
                   const A(x) = e else { throw 0; };\n\
                   if let A(x: y) = e { use(y); }\n";
    let analyses = pattern_analyses(src, &[]);
    assert_eq!(
        analyses.sites.iter().map(|s| s.kind).collect::<Vec<_>>(),
        [SiteKind::LetElse, SiteKind::IfLet]
    );
    assert_eq!(
        analyses.sites[0]
            .subject
            .as_ref()
            .map(|s| s.variant_name.as_str()),
        Some("E")
    );
    assert_eq!(
        binding(&analyses, src, "A(x)", 2).ty.as_deref(),
        Some("string")
    );
    assert_eq!(
        binding(&analyses, src, "A(x: y)", 5).ty.as_deref(),
        Some("string")
    );
}

#[test]
fn each_link_of_an_if_let_chain_is_its_own_site() {
    let src = "variant E { A(x: string), B(y: number) }\n\
                   if let A(x) = e { use(x); } else if let B(y) = e { use(y); }\n";
    let analyses = pattern_analyses(src, &[]);
    assert_eq!(analyses.sites.len(), 2);
    assert_eq!(
        binding(&analyses, src, "B(y)", 2).ty.as_deref(),
        Some("number")
    );
}

#[test]
fn resolution_reports_owned_misspellings_and_stays_quiet_otherwise() {
    let typo = pattern_analyses(
        "variant E { Alpha(x: string), Beta }\nconst v = match (e) { Alhpa(x) => x, Beta => 0 };\n",
        &[],
    );
    assert_eq!(typo.unresolved.len(), 1);
    assert_eq!(typo.unresolved[0].kind, NameKind::Case);
    assert_eq!(typo.unresolved[0].suggestion, "Alpha");
    assert_eq!(
        typo.unresolved[0].match_owner,
        Some(typo.matches[0].keyword_off)
    );
    assert!(typo.match_has_resolution_error(typo.matches[0].keyword_off));

    let unowned_if_let = pattern_analyses(
        "variant E { Alpha(x: string), Beta }\nif let Alhpa(x) = e { use(x); }\n",
        &[],
    );
    assert!(unowned_if_let.unresolved.is_empty());

    // A name that is nobody's misspelling is not an error: the pattern
    // may be over a hand-written tagged union.
    let union = pattern_analyses(
        "variant E { Alpha(x: string), Beta }\nconst v = match (m) { Beta => 0, Gamma(q) => q };\n",
        &[],
    );
    assert!(union.unresolved.is_empty());
}

#[test]
fn an_ambiguous_tag_identifies_no_variant() {
    // Both variants contain `A`, so neither is *the* subject — and a
    // suggestion ttc cannot choose is no suggestion.
    let analyses = pattern_analyses(
        "variant L { A(x: string), Left(n: number) }\n\
             variant R { A(x: string), Righ(n: number) }\n\
             const v = match (e) { A(x) => x, Right(n) => n };\n",
        &[],
    );
    assert!(analyses.unresolved.is_empty());
}

#[test]
fn unreachable_arms_are_computed_but_not_an_error() {
    // `A` is already covered, so the third arm matches nothing new.
    let src = "variant E { A(x: string), B(y: number) }\n\
                   const v = match (e) { A(x) => x, B(y) => y, A(x: z) => z };\n";
    let coverage = pattern_analyses(src, &[]).matches[0]
        .coverage
        .clone()
        .expect("resolved");
    assert_eq!(coverage.unreachable, [2]);
    assert!(coverage.missing.is_empty());
}

#[test]
fn a_guarded_arm_leaves_its_case_uncovered() {
    let src = "variant E { A(x: string), B(y: number) }\n\
                   const v = match (e) { A(x) if ok => x, B(y) => y };\n";
    let coverage = pattern_analyses(src, &[]).matches[0]
        .coverage
        .clone()
        .expect("resolved");
    assert_eq!(patterns(&coverage), [["A"]]);
}

#[test]
fn a_nested_pattern_is_a_column_of_its_own() {
    let src = "variant I { Y(n: number), N }\n\
                   variant O { W(i: I), B }\n\
                   const v = match (o) { W(i: Y(n)) => n, B => 0 };\n";
    let coverage = pattern_analyses(src, &[]).matches[0]
        .coverage
        .clone()
        .expect("resolved");
    assert_eq!(patterns(&coverage), [["W(i: N())"]]);
}

#[test]
fn a_witness_from_an_unidentifiable_column_is_not_certain() {
    // `Inner` names no tt variant, so the payload column's alphabet is
    // unknown and the witness is a guess. The default path reports it
    // anyway (nothing better is available without types); a consumer
    // with a checker filters on this flag and asks instead.
    let src = "variant O { W(i: Inner), B }\n\
                   const v = match (o) { W(i: Yes(n)) => n, B => 0 };\n";
    let coverage = pattern_analyses(src, &[]).matches[0]
        .coverage
        .clone()
        .expect("resolved");
    assert_eq!(patterns(&coverage), [["W"]]);
    assert!(!coverage.missing[0].certain);

    // A column the table *can* name is certain.
    let src = "variant I { Y(n: number), N }\n\
                   variant O { W(i: I), B }\n\
                   const v = match (o) { W(i: Y(n)) => n, B => 0 };\n";
    let coverage = pattern_analyses(src, &[]).matches[0]
        .coverage
        .clone()
        .expect("resolved");
    assert!(coverage.missing[0].certain);
}

#[test]
fn nested_matches_are_all_collected() {
    let src = "variant E { A(x: string), B }\nconst v = match (e) { A(x) => match (e) { A(x: y) | B => 0 }, B => 1 };\n";
    let analyses = pattern_analyses(src, &[]);
    assert_eq!(analyses.matches.len(), 2);
    let y = binding(&analyses, src, "A(x: y)", 5);
    assert_eq!(y.name, "y");
    assert_eq!(y.ty.as_deref(), Some("string"));
}
