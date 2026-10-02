use super::*;

#[test]
fn language_support_materializes_both_tt_packages() {
    let root = crate::test_workspace::Workspace::new("language-runtime");

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
            const broken = ready ? 1 : 2 |> f;\n";
    let doc = service_doc(Path::new("/p/src/a.tt"), source.to_string());
    assert!(
        projection_accepts_diagnostics(&doc.code, crate::SourceKind::TypeScript),
        "{}",
        doc.code
    );
    assert_eq!(doc.recovered.len(), 1);
    assert!(doc.code.contains("\"value\" in $tt_t0"), "{}", doc.code);
    assert!(
        doc.code.contains("const broken = ready ? 1 : 0     ;"),
        "{}",
        doc.code
    );
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
    // A line past the end clamps to the end of the text.
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
fn a_character_past_the_line_end_defaults_back_to_the_line_length() {
    let text = "ab한c\r\nsecond\nlast";
    let at = |line, character| u16_offset(text, Position { line, character });
    assert_eq!(at(0, 30), 4);
    assert_eq!(at(0, 5), 4);
    assert_eq!(at(1, 30), 12);
    assert_eq!(at(2, 30), 17);
    assert_eq!(at(1, 2), 8);
}

#[test]
fn protocol_positions_are_measured_in_the_decoded_text() {
    let source = "\u{feff}export const target = 1;\n";
    let target = mapper::to_utf16(source, source.find("target").unwrap());
    let position = Position {
        line: 0,
        character: 13,
    };
    assert_eq!(u16_position(source, target), position);
    assert_eq!(u16_offset(source, position), target);
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
fn an_edit_of_the_prelude_maps_only_between_its_declarations() {
    let source = "variant V { A, B }\ndeclare const v: V;\n\
export const n = match (v) { A => 1, B => 2 };\nexport const f = flow |> String |> .trim();\n";
    let doc = service_doc(Path::new("/p/src/a.tt"), source.to_string());
    let import = "import { $tt_fl } from \"@tt/runtime\";\n";
    assert!(doc.code.starts_with(import), "{}", doc.code);
    let edit_at = |byte: usize| {
        let position = byte_position(&crate::lines::LineMap::lsp(&doc.code), byte);
        let position =
            serde_json::json!({ "line": position.line, "character": position.character });
        let edit = serde_json::json!({
            "range": { "start": position, "end": position },
            "newText": "x",
        });
        source_edit(&doc.code, &doc.mappings, &doc.inserted, source, None, &edit)
            .map(|edit| edit.range.start)
    };
    let start = Some(Position {
        line: 0,
        character: 0,
    });
    assert_eq!(edit_at(0), start);
    assert_eq!(edit_at(import.len()), start);
    assert_eq!(edit_at("import { ".len()), None);
}

#[test]
fn a_cursor_between_chunks_split_in_the_output_keeps_its_side() {
    // `s.ki` was hoisted to output 40; the `, ` after it stayed at 10.
    let mappings = [
        crate::EmitMapping {
            src: 0,
            out: 40,
            len: 4,
        },
        crate::EmitMapping {
            src: 4,
            out: 10,
            len: 2,
        },
    ];
    let at = |affinity| mapper::cursor_to_output(&mappings, 4, affinity);
    assert_eq!(at(mapper::Affinity::Preceding), Some(44));
    assert_eq!(at(mapper::Affinity::Following), Some(10));
    // Chunks that touch in the output agree, and a lone chunk answers for
    // either side.
    assert_eq!(
        mapper::cursor_to_output(&mappings[..1], 4, mapper::Affinity::Following),
        Some(44)
    );
    assert_eq!(
        mapper::cursor_to_output(&mappings[1..], 4, mapper::Affinity::Preceding),
        Some(10)
    );
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
fn a_destructuring_stands_for_the_whole_list_it_destructures() {
    let src = "variant E { A(x: number, y: number), B(v: E, w: number), C }\n\
               const v = match (e) { A(x, y) => 1, B(v: A(x: p, y: q), w) => w, B(v, w) => 2, C => 0 };\n\
               const u = match (e) { A(x) | B(w: x) => x, C => 0 };\n";
    let doc = service_doc(Path::new("/p/a.tt"), src.to_string());
    let lists: Vec<(&str, &str)> = doc
        .destructured_lists
        .iter()
        .map(|list| {
            (
                &src[list.src..list.src_end],
                &doc.code[list.out..list.out_end],
            )
        })
        .collect();
    assert_eq!(
        lists,
        [
            ("(x, y)", "{ x, y }"),
            ("(x: p, y: q)", "{ x: p, y: q }"),
            ("(v, w)", "{ v, w }"),
        ],
        "{}",
        doc.code
    );
}

#[test]
fn an_or_pattern_binding_stands_for_every_alternative_it_is_written_in() {
    let src = "variant E { A(x: number), B(x: number), C }\n\
               const v = match (e) { A(x) | B(x: x) => x, C => 0 };\n\
               if let A(x: y) | B(x: y) = e { use(y); }\n\
               const t = match (e, e) { (A(w), A(x) | B(x)) => w + x, _ => 0 };\n";
    let doc = service_doc(Path::new("/p/a.tt"), src.to_string());
    let position = |byte: usize| {
        let u16 = mapper::to_utf16(src, byte);
        source_range(src, u16, u16).start
    };
    for (needle, name, shorthand) in [
        ("A(x) | B(x: x)", "x", vec![true, false]),
        ("A(x: y) | B(x: y)", "y", vec![false, false]),
        ("A(x) | B(x))", "x", vec![true, true]),
    ] {
        let base = src.find(needle).unwrap();
        let occurrences: Vec<usize> = needle
            .match_indices(name)
            .map(|(offset, _)| base + offset)
            .filter(|&offset| src.as_bytes()[offset + name.len()] == b')')
            .collect();
        let binding = doc
            .shared_bindings
            .iter()
            .find(|binding| binding.occurrences[0].src == occurrences[0])
            .unwrap_or_else(|| panic!("{needle}: {:?}", doc.shared_bindings));
        assert_eq!(&doc.code[binding.out..binding.out_end], name, "{needle}");
        assert_eq!(
            binding
                .occurrences
                .iter()
                .map(|occurrence| (occurrence.src, occurrence.shorthand))
                .collect::<Vec<_>>(),
            occurrences
                .iter()
                .copied()
                .zip(shorthand)
                .collect::<Vec<_>>(),
            "{needle}"
        );
        let generated = mapper::to_utf16(&doc.code, binding.out);
        for occurrence in occurrences {
            assert_eq!(to_service(&doc, position(occurrence)), None, "{needle}");
            assert_eq!(
                to_service_name(&doc, position(occurrence)),
                Some(generated),
                "{needle}"
            );
        }
    }
    let single = src.find("(A(w)").unwrap() + 3;
    assert!(
        doc.shared_bindings
            .iter()
            .all(|binding| binding.occurrences.iter().all(|o| o.src != single))
    );
    assert!(to_service(&doc, position(single)).is_some());
}

#[test]
fn variant_glue_names_stand_for_their_source_names_in_navigation_only() {
    for ambient in ["", "declare "] {
        let src = format!("{ambient}variant V {{ A(x: number), B }}\nconst w = V.B;\n");
        let doc = service_doc(Path::new("/p/a.tt"), src.clone());
        let source_of = |glue: &str, name: &str| {
            let at = doc
                .code
                .find(glue)
                .unwrap_or_else(|| panic!("{glue:?} in {}", doc.code))
                + glue.find(name).unwrap();
            let start = mapper::to_utf16(&doc.code, at);
            let end = start + name.len();
            assert_eq!(from_service_span(&doc, start, end), None, "{glue:?}");
            declared_name_span(&doc, start, end)
        };
        let variant = src.find("V {").unwrap();
        let case_a = src.find("A(").unwrap();
        let case_b = src.find("B }").unwrap();
        let field = src.find("x:").unwrap();
        assert_eq!(source_of("type V", "V"), Some((variant, variant + 1)));
        assert_eq!(source_of("const V", "V"), Some((variant, variant + 1)));
        assert_eq!(source_of("A: ", "A"), Some((case_a, case_a + 1)));
        assert_eq!(source_of("B: ", "B"), Some((case_b, case_b + 1)));
        assert_eq!(source_of("x: number }", "x"), Some((field, field + 1)));
        let kind = doc.code.find("\"A\"").unwrap();
        assert_eq!(declared_name_span(&doc, kind + 1, kind + 2), None);
    }
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
    let dir = crate::test_workspace::Workspace::new("analyses");
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
}

#[test]
fn the_editor_and_the_typed_pass_share_one_semantic_cache() {
    let dir = crate::test_workspace::Workspace::new("shared-cache");
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
    let analyses = analyses_for(Path::new("/p/a.ttx"), source, Texts::Disk);
    assert!(analyses.declarations.iter().any(|d| d.name == "Real"));
    assert!(!analyses.declarations.iter().any(|d| d.name == "Fake"));

    let dir = crate::test_workspace::Workspace::new("tsx-pattern-kind");
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
}

#[test]
fn isolated_pattern_hover_preserves_jsx_text() {
    let source = "variant E { A(x: string), B(x: number) }\nconst view = <div>let x = try value;</div>;\nconst v = match (e) { A(x) | B(x) => x };\n";
    let path = Path::new("/p/a.ttx");
    let analyses = analyses_for(path, source, Texts::Disk);
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
        "Admin(name) => name, Gue, _ => 'other'",
        "(Admin(name), _) => name, (Gue, _)",
        "(Admin(name), _) => name, (Gue, _), _ => 'other'",
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

#[test]
fn hover_markdown_separates_signature_from_documentation_and_tags() {
    let markdown = serde_json::json!({
        "kind": "markdown",
        "value": "```typescript\nfunction add(a: number): number\n```\nAdds.\n\n```ts\nadd(1)\n```\n\n*@param* `a` — the first",
    });
    assert_eq!(
        split_hover(&markdown),
        (
            "function add(a: number): number".to_string(),
            "Adds.\n\n```ts\nadd(1)\n```\n\n*@param* `a` — the first".to_string()
        )
    );
    let bare =
        serde_json::json!({ "kind": "markdown", "value": "```typescript\nconst u: 1\n```\n" });
    assert_eq!(
        split_hover(&bare),
        ("const u: 1".to_string(), String::new())
    );
    let plain = serde_json::json!({ "kind": "plaintext", "value": "const u: 1" });
    assert_eq!(
        split_hover(&plain),
        ("const u: 1".to_string(), String::new())
    );
    let marked = serde_json::json!({ "language": "typescript", "value": "let v: string" });
    assert_eq!(
        split_hover(&marked),
        ("let v: string".to_string(), String::new())
    );
}

#[test]
fn signature_help_is_asked_outside_every_generated_argument_list() {
    let code = "foo(bar(m, h), `${bar(k)}`)";
    let mappings = vec![
        EmitMapping {
            src: 0,
            out: 0,
            len: 4,
        },
        EmitMapping {
            src: 4,
            out: 8,
            len: 1,
        },
        EmitMapping {
            src: 5,
            out: 11,
            len: 1,
        },
        EmitMapping {
            src: 6,
            out: 13,
            len: 5,
        },
        EmitMapping {
            src: 11,
            out: 22,
            len: 1,
        },
        EmitMapping {
            src: 12,
            out: 24,
            len: 3,
        },
    ];
    let kind = crate::SourceKind::TypeScript;
    assert_eq!(signature_position(code, &mappings, kind, 12), 7);
    assert_eq!(signature_position(code, &mappings, kind, 9), 7);
    assert_eq!(signature_position(code, &mappings, kind, 23), 21);
    assert_eq!(signature_position(code, &mappings, kind, 2), 2);
    let copied = [EmitMapping {
        src: 0,
        out: 0,
        len: 9,
    }];
    assert_eq!(signature_position("foo(a, b)", &copied, kind, 6), 6);
}

#[test]
fn tt_tokens_replace_the_service_tokens_they_overlap() {
    let range = |line: u32, start: u32, end: u32| Range {
        start: Position {
            line,
            character: start,
        },
        end: Position {
            line,
            character: end,
        },
    };
    let service = |range: Range, token_type: &str, modifiers: &[&str]| ClassifiedToken {
        range,
        token_type: token_type.to_string(),
        modifiers: modifiers.iter().map(|m| m.to_string()).collect(),
    };
    let own = vec![
        crate::engine::tokens::SemanticToken {
            range: range(1, 4, 10),
            kind: crate::engine::tokens::SemanticTokenKind::Variable,
        },
        crate::engine::tokens::SemanticToken {
            range: range(0, 0, 5),
            kind: crate::engine::tokens::SemanticTokenKind::Keyword,
        },
    ];
    let merged = merge_tokens(
        own,
        vec![
            service(range(0, 2, 4), "function", &[]),
            service(range(0, 6, 7), "parameter", &["declaration"]),
            service(range(1, 4, 10), "variable", &["declaration", "readonly"]),
        ],
    );
    assert_eq!(
        merged,
        vec![
            service(range(0, 0, 5), "keyword", &[]),
            service(range(0, 6, 7), "parameter", &["declaration"]),
            service(range(1, 4, 10), "variable", &["declaration", "readonly"]),
        ]
    );
}

fn keyword_item(label: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::Keyword),
        tags: Vec::new(),
        sort_text: "15".to_string(),
        insert_text: None,
        filter_text: None,
        snippet: false,
        range: None,
        label_detail: None,
        description: None,
        detail: None,
        source: None,
    }
}

fn restated_keywords(source: &str, marker: &str, labels: &[&str]) -> Vec<String> {
    let doc = service_doc(Path::new("/p/keywords.tt"), source.to_string());
    let at = source.find(marker).unwrap() + marker.len();
    let out = mapper::cursor_to_output(&doc.mappings, at, mapper::Affinity::Preceding).unwrap();
    let served = mapper::to_utf16(&doc.code, out);
    let mut answer = CompletionAnswer {
        items: labels.iter().map(|label| keyword_item(label)).collect(),
        ..CompletionAnswer::default()
    };
    scope::restate_completions(
        &mut answer,
        &doc,
        doc.served(),
        crate::SourceKind::TypeScript,
        served,
        at,
    );
    answer.items.into_iter().map(|item| item.label).collect()
}

#[test]
fn a_module_level_arm_takes_a_function_body_keywords_whatever_the_answer_holds() {
    let source = "declare function f(n: number): number;\ndeclare const input: number;\nexport const top = match (input) { 1 => f(input), _ => input as number };\n";
    assert_eq!(
        restated_keywords(source, "1 => f(", &["abstract", "declare", "if", "return"]),
        ["if", "return"]
    );
    assert_eq!(
        restated_keywords(source, "input as ", &["number", "declare", "unknown"]),
        ["number", "declare", "unknown"]
    );
}

fn offers_all_keywords_at(code: &str) -> bool {
    let at = code.find('|').unwrap();
    let code = code.replacen('|', "", 1);
    let input = crate::host_input::HostInput::new(&code);
    let program = input
        .parser(crate::SourceKind::TypeScript)
        .parse_program()
        .unwrap();
    keyword_filter::offers_all_keywords(
        &code,
        crate::SourceKind::TypeScript,
        &program,
        input.origin(),
        at,
    )
}

#[test]
fn typescript_offers_every_keyword_only_outside_type_and_member_positions() {
    let prelude = "declare const x: number; declare function g<T>(v: T): T;\n";
    for (line, all) in [
        ("g(|x);", true),
        ("const a = |x;", true),
        ("const p = typeof |x;", true),
        ("let q: typeof |x;", true),
        ("const t = `${|x}`;", true),
        ("const z = /* c */|x;", true),
        ("const a: |number = 1;", false),
        ("const a: num|ber = 1;", false),
        ("const b = x as |number;", false),
        ("const c = x satisfies |number;", false),
        ("type T = |number;", false),
        ("let d: Array<|number>;", false),
        ("const e = g<|number>(1);", false),
        ("interface I { |a: number }", false),
        ("const s = \"te|xt\";", false),
        ("// com|ment", false),
    ] {
        let code = format!("{prelude}{line}\n");
        assert_eq!(offers_all_keywords_at(&code), all, "{line}");
    }
}
