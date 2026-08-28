//! Name-resolution tests: declaration collection, shadowing, identity, and
//! the fidelity of the conversion into the analysis surface (the analysis
//! consumes this resolver — Phase 3, TASK-123·129).

use ttc::hir::{self, FileId};
use ttc::resolve::{self, DefKind, Namespace, Res, Resolution, UseKind};
use ttc::{ExternVariant, ExternVariantCase, ExternVariantField};

fn resolved(source: &str, externs: &[ExternVariant]) -> (hir::HirFile, Resolution) {
    let mut hir = hir::lower_source(FileId(0), source);
    let decls: Vec<resolve::ExternDecl> = externs.iter().map(Into::into).collect();
    let resolution = resolve::resolve_file(&mut hir, &decls);
    (hir, resolution)
}

fn span_text<'s>(source: &'s str, hir: &hir::HirFile, node: hir::NodeId) -> &'s str {
    let span = hir.source_map.node_span(node).expect("node has a span");
    &source[span.start..span.end]
}

#[test]
fn a_variant_defines_a_type_and_a_constructor_value() {
    let src = "export variant Shape { Circle(radius: number), Point }\n";
    let (hir, resolution) = resolved(src, &[]);
    let type_def = resolution
        .lookup(Namespace::Type, "Shape")
        .expect("type def");
    let value_def = resolution
        .lookup(Namespace::Value, "Shape")
        .expect("value def");
    assert_ne!(type_def, value_def, "two definitions, one per namespace");
    let DefKind::VariantValue { variant_def } = resolution.defs[value_def].kind else {
        panic!("the value-namespace def is the constructor object");
    };
    assert_eq!(variant_def, type_def);
    // The definition's span is the declared name.
    let span = hir
        .source_map
        .def_span(type_def)
        .expect("local def has a span");
    assert_eq!(&src[span.start..span.end], "Shape");
}

#[test]
fn same_spelling_is_not_same_identity() {
    // Two variant declarations both declare `Empty`. The two uses resolve to *different*
    // variants — identity is the (variant, index) pair, not the string.
    let src = "variant A { Empty, Ok(x: number) }\n\
        variant B { Empty, Fail(code: number) }\n\
        const a = match (v) { Ok(x) => x, Empty => 0 };\n\
        const b = match (w) { Fail(code) => code, Empty => 0 };\n";
    let (hir, resolution) = resolved(src, &[]);
    let empties: Vec<resolve::VariantRef> = hir
        .sites
        .iter()
        .flat_map(|(_, site)| &site.arms)
        .filter_map(|arm| match &hir.patterns[arm.pattern] {
            hir::Pat::Constructor { path, .. } if path.name == "Empty" => {
                match resolution.uses.get(&path.node) {
                    Some(Res::Variant(v)) => Some(*v),
                    other => panic!("Empty did not resolve to a variant: {other:?}"),
                }
            }
            _ => None,
        })
        .collect();
    assert_eq!(empties.len(), 2);
    assert_ne!(empties[0].variant_def, empties[1].variant_def);
    assert_eq!(resolution.defs[empties[0].variant_def].name, "A");
    assert_eq!(resolution.defs[empties[1].variant_def].name, "B");
}

#[test]
fn locals_shadow_imports_shadow_builtins() {
    // A local `Option` replaces the built-in; an imported `Token` exists;
    // an imported `Result` would shadow the built-in the same way.
    let src = "variant Option { Nothing, Just(v: number) }\n\
        const a = match (o) { Just(v) => v, Nothing => 0 };\n";
    let externs = [ExternVariant {
        name: "Token".to_string(),
        generics: String::new(),
        cases: ["Num", "Eof"]
            .into_iter()
            .map(|tag| ttc::ExternVariantCase {
                tag: tag.to_string(),
                fields: None,
            })
            .collect(),
        from: Some("./token.tt".to_string()),
    }];
    let (hir, resolution) = resolved(src, &externs);
    let option = resolution.lookup(Namespace::Type, "Option").unwrap();
    let DefKind::Variant(data) = &resolution.defs[option].kind else {
        panic!()
    };
    assert!(matches!(data.origin, resolve::DeclOrigin::Local(_)));
    assert_eq!(
        data.variants.len(),
        2,
        "the local declaration, not the built-in"
    );
    // Result is still the built-in.
    let result = resolution.lookup(Namespace::Type, "Result").unwrap();
    let DefKind::Variant(data) = &resolution.defs[result].kind else {
        panic!()
    };
    assert!(matches!(data.origin, resolve::DeclOrigin::Builtin));
    // Token came in from the import, tags only.
    let token = resolution.lookup(Namespace::Type, "Token").unwrap();
    let DefKind::Variant(data) = &resolution.defs[token].kind else {
        panic!()
    };
    assert_eq!(
        data.origin,
        resolve::DeclOrigin::Imported {
            from: Some("./token.tt".to_string())
        }
    );
    // The match resolved against the *local* Option.
    let site = hir.sites.iter().next().unwrap().1;
    let subjects = &resolution.sites[&hir.sites.iter().next().unwrap().0].subjects;
    assert_eq!(subjects[0], Some(option));
    let _ = site;
}

#[test]
fn an_import_alias_is_the_name_in_scope() {
    // The CLI hands the resolver the alias-applied name (`T` for
    // `import { Token as T }`) — resolution happens under it.
    let externs = [ExternVariant {
        name: "T".to_string(),
        generics: String::new(),
        cases: ["Num", "Eof"]
            .into_iter()
            .map(|tag| ttc::ExternVariantCase {
                tag: tag.to_string(),
                fields: None,
            })
            .collect(),
        from: Some("./token.tt".to_string()),
    }];
    let src = "const v = match (t) { Num => 1, Eof => 0 };\n";
    let (hir, resolution) = resolved(src, &externs);
    let site_id = hir.sites.iter().next().unwrap().0;
    let subject = resolution.sites[&site_id].subjects[0].expect("identified");
    assert_eq!(resolution.defs[subject].name, "T");
}

#[test]
fn imported_payload_fields_resolve_with_their_source_declaration() {
    let externs = [ExternVariant {
        name: "PaymentMethod".to_string(),
        generics: String::new(),
        cases: vec![ExternVariantCase {
            tag: "Card".to_string(),
            fields: Some(vec![ExternVariantField {
                name: "brand".to_string(),
                optional: false,
                ty: "string".to_string(),
            }]),
        }],
        from: Some("./domain.tt".to_string()),
    }];
    let source = "const value = match (method) { Card(brnad) => brnad, _ => \"n/a\" };\n";
    let (hir, resolution) = resolved(source, &externs);
    let unresolved = resolution.unresolved.first().expect("field misspelling");
    assert_eq!(unresolved.kind, UseKind::Field);
    assert_eq!(unresolved.name, "brnad");
    assert_eq!(unresolved.suggestion, "brand");
    assert_eq!(span_text(source, &hir, unresolved.node), "brnad");
}

#[test]
fn a_wildcard_does_not_hide_a_unique_single_case_near_miss() {
    let externs = [ExternVariant {
        name: "PaymentMethod".to_string(),
        generics: String::new(),
        cases: vec![ExternVariantCase {
            tag: "Card".to_string(),
            fields: Some(vec![ExternVariantField {
                name: "brand".to_string(),
                optional: false,
                ty: "string".to_string(),
            }]),
        }],
        from: Some("./domain.tt".to_string()),
    }];
    let source = "const fee = match (method) { Crad(brand) => 1, _ => 0 };\n";
    let (hir, resolution) = resolved(source, &externs);
    let unresolved = resolution.unresolved.first().expect("case misspelling");
    assert_eq!(unresolved.kind, UseKind::Case);
    assert_eq!(unresolved.name, "Crad");
    assert_eq!(unresolved.suggestion, "Card");
    assert_eq!(span_text(source, &hir, unresolved.node), "Crad");

    let ambiguous = ["Left", "Right"].map(|name| ExternVariant {
        name: name.to_string(),
        generics: String::new(),
        cases: vec![ExternVariantCase {
            tag: "Card".to_string(),
            fields: None,
        }],
        from: None,
    });
    let (_, resolution) = resolved(source, &ambiguous);
    assert!(
        resolution.unresolved.is_empty(),
        "an ambiguous replacement is not a diagnostic"
    );
}

#[test]
fn unknown_case_suggestions_stay_in_the_identified_variants_domain() {
    // `Circel` misses in Shape; Other also has a `Circle`. The suggestion
    // must come from Shape (the identified subject), and resolution must
    // not silently wire the pattern to Other's homonym.
    let src = "variant Shape { Circle(r: number), Empty }\n\
        variant Other { Circle(r: number), Rest }\n\
        const v = match (s) { Circel(r) => r, Empty => 0 };\n";
    let (hir, resolution) = resolved(src, &[]);
    assert_eq!(resolution.unresolved.len(), 1);
    let miss = &resolution.unresolved[0];
    assert_eq!(miss.name, "Circel");
    assert_eq!(miss.suggestion, "Circle");
    assert_eq!(resolution.defs[miss.against].name, "Shape");
    assert_eq!(span_text(src, &hir, miss.node), "Circel");
}

#[test]
fn a_hand_written_union_resolves_to_silence() {
    // No declaration knows these tags: no subject, no uses, no report —
    // tag patterns match hand-written unions by design.
    let src = "const v = match (u) { Loading => 0, Ready => 1, _ => 2 };\n";
    let (hir, resolution) = resolved(src, &[]);
    let site_id = hir.sites.iter().next().unwrap().0;
    assert_eq!(resolution.sites[&site_id].subjects[0], None);
    assert!(resolution.unresolved.is_empty());
}

#[test]
fn nested_patterns_resolve_against_the_fields_declared_type() {
    let src = "variant Inner { Leaf(v: number), Nil }\n\
        variant Outer { Wrap(inner: Inner), Bare }\n\
        const v = match (o) { Wrap(inner: Leaf(n)) => n, Wrap(inner: Nil) => 0, Bare => 1 };\n";
    let (hir, resolution) = resolved(src, &[]);
    let inner_def = resolution.lookup(Namespace::Type, "Inner").unwrap();
    // Find the nested `Leaf` use and check it resolved into Inner.
    let mut found = false;
    for (_, pat) in hir.patterns.iter() {
        if let hir::Pat::Constructor { path, .. } = pat
            && path.name == "Leaf"
        {
            let Some(Res::Variant(v)) = resolution.uses.get(&path.node) else {
                panic!("Leaf did not resolve");
            };
            assert_eq!(v.variant_def, inner_def);
            found = true;
        }
    }
    assert!(found);
}

#[test]
fn a_unique_nested_tag_resolves_a_generic_payload_declaration() {
    let source = "variant PaymentMethod { Card(brand: string), Cash }\n\
        const brand = match (result) {\n\
        \x20 Ok(value: Card(brnd)) => brnd,\n\
        \x20 Ok(value) => \"other\",\n\
        \x20 Err(error) => \"error\",\n\
        };\n";
    let (hir, resolution) = resolved(source, &[]);
    let unresolved = resolution.unresolved.first().expect("nested field typo");
    assert_eq!(unresolved.kind, UseKind::Field);
    assert_eq!(unresolved.name, "brnd");
    assert_eq!(unresolved.tag.as_deref(), Some("Card"));
    assert_eq!(unresolved.suggestion, "brand");
    assert_eq!(span_text(source, &hir, unresolved.node), "brnd");

    let ambiguous = "variant PaymentMethod { Card(brand: string) }\n\
        variant Identity { Card(number: string) }\n\
        const brand = match (result) {\n\
        \x20 Ok(value: Card(brnd)) => brnd,\n\
        \x20 Err(error) => \"error\",\n\
        };\n";
    let (_, resolution) = resolved(ambiguous, &[]);
    assert!(
        resolution.unresolved.is_empty(),
        "two exact Card declarations provide no source-level owner"
    );
}

#[test]
fn a_single_pattern_site_reports_only_a_unique_one_edit_miss() {
    // `Circel` is one transposition from Shape's `Circle` and nothing
    // else is that close → reported. `Cxxcle` is not → silence.
    let src = "variant Shape { Circle(radius: number), Empty }\n\
        if let Circel(radius) = s {\n  use(radius);\n}\n\
        if let Cxxcle(radius) = s {\n  use(radius);\n}\n";
    let (_, resolution) = resolved(src, &[]);
    assert_eq!(resolution.unresolved.len(), 1);
    assert_eq!(resolution.unresolved[0].name, "Circel");
    assert_eq!(resolution.unresolved[0].kind, UseKind::Case);
}

#[test]
fn resolution_matches_the_analysis_answer() {
    // The analysis consumes the resolver (Phase 3), so this no longer
    // guards two implementations against drifting — it pins the fidelity
    // of the conversion: what the resolver reports is exactly what the
    // analysis surface hands on, span for span, word for word.
    let cases = [
        // a match typo
        "variant Shape { Circle(r: number), Empty }\n\
         const v = match (s) { Circel(r) => r, Empty => 0 };\n",
        // a field typo
        "variant Shape { Circle(radius: number), Empty }\n\
         const v = match (s) { Circle(radiuz) => radiuz, Empty => 0 };\n",
        // a nested typo
        "variant Inner { Leaf(v: number), Nil }\n\
         variant Outer { Wrap(inner: Inner), Bare }\n\
         const v = match (o) { Wrap(inner: Laef(n)) => n, Bare => 1, _ => 2 };\n",
        // an if-let typo, and one too far to report
        "variant Shape { Circle(radius: number), Empty }\n\
         if let Circel(radius) = s {\n  use(radius);\n}\n\
         if let Zzz(radius) = s {\n  use(radius);\n}\n",
        // a hand-written union: silence
        "const v = match (u) { Loading => 0, Ready => 1, _ => 2 };\n",
        // clean code: silence
        "variant Shape { Circle(r: number), Empty }\n\
         const v = match (s) { Circle(r) => r, Empty => 0 };\n",
    ];
    for src in cases {
        let (hir, resolution) = resolved(src, &[]);
        let analysis = ttc::pattern_analyses(src, &[]);
        let ours: Vec<(usize, usize, &str, &str)> = resolution
            .unresolved
            .iter()
            .map(|u| {
                let span = hir.source_map.node_span(u.node).unwrap();
                (span.start, span.end, u.name.as_str(), u.suggestion.as_str())
            })
            .collect();
        let theirs: Vec<(usize, usize, &str, &str)> = analysis
            .unresolved
            .iter()
            .map(|u| (u.start, u.end, u.name.as_str(), u.suggestion.as_str()))
            .collect();
        assert_eq!(ours, theirs, "diverged on:\n{src}");
    }
}

#[test]
fn tuple_positions_identify_independently() {
    // Unit-only declarations are tt variants without a disambiguating payload.
    let src = "variant A { X(), Y }\nvariant B { P(), Q }\n\
        const v = match (a, b) { (X, P) => 0, (Y, Q) => 1 };\n";
    let (hir, resolution) = resolved(src, &[]);
    let site_id = hir
        .sites
        .iter()
        .find(|(_, s)| s.kind == hir::SiteKind::TupleMatch)
        .unwrap()
        .0;
    let subjects = &resolution.sites[&site_id].subjects;
    assert_eq!(subjects.len(), 2);
    assert_eq!(resolution.defs[subjects[0].unwrap()].name, "A");
    assert_eq!(resolution.defs[subjects[1].unwrap()].name, "B");
}

#[test]
fn an_or_pattern_site_gets_match_grade_identification() {
    // Two tags are match-grade evidence: the unique best-overlap holder
    // identifies the variant, so a typo two edits away is reported — the
    // strict one-edit licence only governs a site whose evidence is a
    // single tag.
    let src = "variant Shape { Circle(radius: number), Empty }\n\
        function f(s: Shape): number {\n\
        \x20 const Cyrcla(radius) | Empty() = s else { return 0; };\n\
        \x20 return radius;\n\
        }\n";
    let (_, resolution) = resolved(src, &[]);
    assert_eq!(resolution.unresolved.len(), 1);
    assert_eq!(resolution.unresolved[0].name, "Cyrcla");
    assert_eq!(resolution.unresolved[0].suggestion, "Circle");

    // The same typo as the only alternative stays unreported: one tag is
    // thin evidence and `Cyrcla` is two edits from `Circle`.
    let alone = "variant Shape { Circle(radius: number), Empty }\n\
        function f(s: Shape): number {\n\
        \x20 const Cyrcla(radius) = s else { return 0; };\n\
        \x20 return radius;\n\
        }\n";
    let (_, resolution) = resolved(alone, &[]);
    assert_eq!(resolution.unresolved.len(), 0);
}
