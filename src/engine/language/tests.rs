use super::*;

#[test]
fn language_support_materializes_both_tt_packages() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "tt-language-runtime-{}-{nonce}",
        std::process::id()
    ));

    // Both, and before the service resolves anything: which one a file
    // needs is a question about text that may not parse yet (TASK-217).
    ensure_std_module(&root);
    ensure_runtime_module(&root);
    assert!(root.join("node_modules/@tt/std/index.ts").exists());
    assert!(root.join("node_modules/@tt/runtime/index.ts").exists());

    // Neither is written over one the project already has.
    std::fs::write(root.join("node_modules/@tt/runtime/index.ts"), "// mine\n").unwrap();
    ensure_runtime_module(&root);
    assert_eq!(
        std::fs::read_to_string(root.join("node_modules/@tt/runtime/index.ts")).unwrap(),
        "// mine\n"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn diagnostic_projection_depends_on_parseability_not_diagnostic_numbers() {
    assert!(projection_accepts_diagnostics(
        "const value = { A: 1, A: 2 };",
        crate::SourceKind::TypeScript,
    ));
    assert!(!projection_accepts_diagnostics(
        "const = ;",
        crate::SourceKind::TypeScript,
    ));
}

#[test]
fn service_projection_recovers_parser_error_nodes() {
    let source = "function f(value: number) { const n = try value; return n; }\n\
            const broken = 1 |> ;\n";
    let doc = service_doc(Path::new("/p/src/a.tt"), source.to_string());
    assert!(
        projection_accepts_diagnostics(&doc.code, crate::SourceKind::TypeScript),
        "{}",
        doc.code
    );
    assert_eq!(doc.recovered.len(), 1);
    assert!(doc.code.contains("\"value\" in $tt_t0"), "{}", doc.code);
    assert!(doc.code.contains("const broken = 0"), "{}", doc.code);
}

#[test]
fn u16_positions_round_trip_over_multibyte_text() {
    let text = "한글\nab한c\n";
    // Offsets count UTF-16 units: each Hangul syllable is one unit.
    assert_eq!(
        u16_offset(
            text,
            Position {
                line: 1,
                character: 3
            }
        ),
        6
    );
    assert_eq!(
        u16_position(text, 6),
        Position {
            line: 1,
            character: 3
        }
    );
    // Past-the-line characters spill forward, clamped to the end.
    assert_eq!(
        u16_offset(
            text,
            Position {
                line: 9,
                character: 0
            }
        ),
        8
    );
}

#[test]
fn a_chunk_end_offset_belongs_to_the_chunk() {
    // The service lookup is inclusive of a chunk's end — completion and
    // hover sit at the end of what was just typed.
    let mappings = [crate::EmitMapping {
        src: 0,
        out: 10,
        len: 5,
    }];
    assert_eq!(mapper::to_output_inclusive(&mappings, 5), Some(15));
    assert_eq!(mapper::to_source_inclusive(&mappings, 15), Some(5));
    // ... and one past it is glue.
    assert_eq!(mapper::to_output_inclusive(&mappings, 6), None);
}

#[test]
fn isolating_an_alternative_maps_its_binding_into_narrowed_output() {
    let src =
        "variant E { A(x: string), B(x: number) }\nconst v = match (e) { A(x) | B(x) => x };\n";
    let analyses = crate::pattern_analyses(src, &[]);
    let b_x = src.find("B(x)").unwrap() + 2;
    let binding = analyses.binding_at(b_x).unwrap().clone();
    let (code, offset) = isolate_alternative(Path::new("/p/a.tt"), src, &binding, b_x).unwrap();
    // The or-arm became a single `B(x)` arm — the emitted switch
    // narrows to `B` alone...
    assert!(code.contains("case \"B\""), "{code}");
    assert!(!code.contains("case \"A\""), "{code}");
    // ...and the question lands on the (now mapped) destructured `x`.
    let byte = mapper::from_utf16(&code, offset);
    assert_eq!(&code[byte..byte + 1], "x");
    assert!(code[..byte].ends_with("const { "), "{code}");

    // The A occurrence isolates to the A arm the same way.
    let a_x = src.find("A(x)").unwrap() + 2;
    let binding = analyses.binding_at(a_x).unwrap().clone();
    let (code, _) = isolate_alternative(Path::new("/p/a.tt"), src, &binding, a_x).unwrap();
    assert!(code.contains("case \"A\""), "{code}");
    assert!(!code.contains("case \"B\""), "{code}");
}

#[test]
fn declared_hover_names_the_constructor_and_its_type() {
    let src =
        "variant E { A(x: string), B(x: number) }\nconst v = match (e) { A(x) | B(x) => x };\n";
    let analyses = crate::pattern_analyses(src, &[]);
    let binding = analyses.binding_at(src.find("B(x)").unwrap() + 2).unwrap();
    let range = source_range(src, 0, 1);
    let info = declared_binding_hover(binding, range).unwrap();
    assert_eq!(info.signature, "const x: number");
    assert!(
        info.documentation.contains("`E.B`"),
        "{}",
        info.documentation
    );

    // An unresolved subject answers nothing rather than guessing.
    let unknown = "const v = match (e) { What(x) | Ever(x) => x };\n";
    let analyses = crate::pattern_analyses(unknown, &[]);
    let binding = analyses
        .binding_at(unknown.find("What(x)").unwrap() + 5)
        .unwrap();
    assert!(declared_binding_hover(binding, range).is_none());
}

#[test]
fn analyses_collect_imported_declarations_like_the_cli() {
    let dir = std::env::temp_dir().join(format!("tt-analyses-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("token.tt"),
        "export variant Token { Num(value: number), Eof }\n",
    )
    .unwrap();
    let source = "import { Token as T } from \"./token.tt\";\nconst v = match (t) { Num(value) | Eof => 0 };\n";
    let main = dir.join("main.tt");
    std::fs::write(&main, source).unwrap();
    let main = main.canonicalize().unwrap();

    let engine = crate::engine::Engine::new(None);
    let mut project = engine
        .open_project(
            &[dir.to_string_lossy().to_string()],
            &crate::engine::ProjectOptions::default(),
        )
        .unwrap();

    // The disk copy answers...
    let semantics = project.semantic_analyses(&main, source);
    let binding = semantics
        .analyses
        .binding_at(source.find("Num(value)").unwrap() + 4)
        .unwrap();
    assert_eq!(binding.ty.as_deref(), Some("number"));
    assert_eq!(binding.variant_name.as_deref(), Some("T"));

    // ...the same question again is answered by the cache...
    assert_eq!(project.semantic_cache_hits(), 0);
    project.semantic_analyses(&main, source);
    assert_eq!(project.semantic_cache_hits(), 1);

    // ...and an overlay of the imported file wins over its disk copy —
    // the changed externs invalidate the cached entry, so this is a
    // recompute, not a stale hit.
    project.open_document(
        dir.join("token.tt").canonicalize().unwrap(),
        "export variant Token { Num(value: string), Eof }\n".to_string(),
    );
    let semantics = project.semantic_analyses(&main, source);
    let binding = semantics
        .analyses
        .binding_at(source.find("Num(value)").unwrap() + 4)
        .unwrap();
    assert_eq!(binding.ty.as_deref(), Some("string"));
    assert_eq!(project.semantic_cache_hits(), 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_editor_and_the_typed_pass_share_one_semantic_cache() {
    let dir = std::env::temp_dir().join(format!("tt-shared-cache-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("a.tt");
    let source = "variant E { A(x: number), B }\nconst v = match (e) { A(x) | B => 0 };\n";
    std::fs::write(&file, source).unwrap();

    let engine = crate::engine::Engine::new(None);
    let mut project = engine
        .open_project(
            &[dir.to_string_lossy().to_string()],
            &crate::engine::ProjectOptions::default(),
        )
        .unwrap();
    let files = project.initial_files();

    // The typed pass computes the file's semantics...
    let snapshot = project.update(&files).unwrap();
    project
        .check(&snapshot, &crate::engine::CheckRequest::default())
        .unwrap();
    assert_eq!(project.semantic_cache_hits(), 0);

    // ...and the editor's fallback question is a hit on that entry,
    // not a second computation of the same answer.
    project.semantic_analyses(&files[0], source);
    assert_eq!(project.semantic_cache_hits(), 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn member_context_walks_back_over_the_identifier() {
    assert!(is_member_context("value.le", 8));
    assert!(is_member_context("value.", 6));
    assert!(!is_member_context("value", 5));
    assert!(!is_member_context("a . b", 1));
}

#[test]
fn completion_probe_preserves_source_kind_and_cursor() {
    for (path, prefix, kind) in [
        (
            "/p/a.tt",
            "const title = \"문자\";\n",
            crate::SourceKind::TypeScript,
        ),
        (
            "/p/a.ttx",
            "const view = <div>문자 let x = try value;</div>;\n",
            crate::SourceKind::Tsx,
        ),
    ] {
        let source = format!("{prefix}const result = \"hello\" |> .;\n");
        let at = source.rfind("|> .").unwrap() + 4;
        let probe = build_probe(Path::new(path), &source, at, 1).unwrap();
        assert!(
            projection_accepts_diagnostics(&probe.code, kind),
            "{}",
            probe.code
        );
        let byte = mapper::from_utf16(&probe.code, probe.offset);
        assert!(probe.code[byte..].starts_with(PROBE_NAME), "{}", probe.code);
        assert!(is_member_context(&probe.code, probe.offset));
        assert!(probe.code.starts_with(prefix), "{}", probe.code);
    }
}

#[test]
fn ttx_pattern_analysis_does_not_parse_jsx_text() {
    let source = "variant Real { A }\nconst view = <div>variant Fake { A }</div>;\n";
    let analyses = analyses_for(Path::new("/p/a.ttx"), source);
    assert!(analyses.declarations.iter().any(|d| d.name == "Real"));
    assert!(!analyses.declarations.iter().any(|d| d.name == "Fake"));

    let dir = std::env::temp_dir().join(format!("tt-tsx-pattern-kind-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.ttx");
    std::fs::write(&path, source).unwrap();
    let project = crate::engine::Engine::new(None)
        .open_project(
            &[dir.to_string_lossy().to_string()],
            &crate::engine::ProjectOptions::default(),
        )
        .unwrap();
    let semantics = project.semantic_analyses(&path, source);
    assert!(
        semantics
            .analyses
            .declarations
            .iter()
            .any(|d| d.name == "Real")
    );
    assert!(
        !semantics
            .analyses
            .declarations
            .iter()
            .any(|d| d.name == "Fake")
    );
    project.semantic_analyses(&path, source);
    assert_eq!(project.semantic_cache_hits(), 1);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn isolated_pattern_hover_preserves_jsx_text() {
    let source = "variant E { A(x: string), B(x: number) }\nconst view = <div>let x = try value;</div>;\nconst v = match (e) { A(x) | B(x) => x };\n";
    let path = Path::new("/p/a.ttx");
    let analyses = analyses_for(path, source);
    let byte = source.find("B(x)").unwrap() + 2;
    let binding = analyses.binding_at(byte).unwrap();
    let (code, offset) = isolate_alternative(path, source, binding, byte).unwrap();
    assert!(code.contains("<div>let x = try value;</div>"), "{code}");
    assert!(
        projection_accepts_diagnostics(&code, crate::SourceKind::Tsx),
        "{code}"
    );
    assert!(code[mapper::from_utf16(&code, offset)..].starts_with('x'));
}

#[test]
fn incomplete_match_arms_preserve_sibling_projections() {
    for arms in [
        "Gue, Admin(name) => name",
        "Admin(name) => name, Gue, Guest => 'guest'",
        "Admin(name) => name, Gue",
        "(Admin(name), _) => name, (Gue, _)",
    ] {
        let source = format!(
            "variant User {{ Admin(name: string), Guest }}\ndeclare const user: User;\nconst label = match (user, user) {{ {arms} }};\n"
        );
        let doc = service_doc(Path::new("/p/a.tt"), source.clone());
        assert!(!doc.code.contains("match ("), "{}", doc.code);
        assert!(doc.code.contains("name"), "{}", doc.code);
        assert!(!doc.code.contains("Gue,"), "{}", doc.code);
        let byte = source.find("=> name").unwrap() + 3;
        let offset = to_service(&doc, u16_position(&source, mapper::to_utf16(&source, byte)))
            .expect("the valid arm body must retain a source mapping");
        let output_byte = mapper::from_utf16(&doc.code, offset);
        assert!(doc.code[output_byte..].starts_with("name"), "{}", doc.code);
        assert_eq!(
            mapper::to_source_inclusive(&doc.mappings, output_byte),
            Some(byte)
        );
        assert_eq!(doc.recovered.len(), 1);
        let (start, end) = doc.recovered[0];
        assert_eq!(
            source[start..end].trim().trim_end_matches(',').trim(),
            if arms.starts_with('(') {
                "(Gue, _)"
            } else {
                "Gue"
            }
        );
        assert!(
            doc.tt_diagnostics
                .iter()
                .any(|d| d.code == crate::DiagnosticCode::MalformedMatch
                    && d.start == Some(start)
                    && d.end == Some(end)),
            "{:?}",
            doc.tt_diagnostics
        );
    }
}

#[test]
fn completion_probe_uses_the_same_arm_recovery_as_hover() {
    let source = "variant User { Admin(name: string), Guest }\ndeclare const user: User;\nconst label = match (user) { Admin(name) => name., Gue };\n";
    let at = source.find("name.,").unwrap() + 5;
    let probe = build_probe(Path::new("/p/a.tt"), source, at, 1).unwrap();
    assert!(!probe.code.contains("match ("), "{}", probe.code);
    let byte = mapper::from_utf16(&probe.code, probe.offset);
    assert!(probe.code[..byte].ends_with("name."), "{}", probe.code);
}
