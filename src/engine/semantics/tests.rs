use super::*;

/// The declaration table of a source, as a translation sees it.
fn table(source: &str) -> Vec<DeclaredVariant> {
    crate::pattern_analyses(source, &[]).declarations
}

/// The motivating example of TASK-118: the propagated `Err` and the
/// return type it does not fit, both said as the user declared them.
#[test]
fn a_case_is_named_and_a_full_union_is_its_variant() {
    let declarations = table(
        "variant Test { OutOfRange(value: number), Empty }\n\
             variant ParseError { NotANumber(text: string) }\n",
    );
    let said = translate(
        AnchorKind::TryExit,
        2322,
        "Type 'Err<{ kind: \"OutOfRange\"; value: number; }>' is not assignable to type \
             'Result<string, { kind: \"NotANumber\"; text: string; }>'.",
        &declarations,
    )
    .expect("translated");
    assert!(
        said.contains(
            "(in tt's names: Type 'Err<Test.OutOfRange>' is not assignable to type \
                 'Result<string, ParseError>'.)"
        ),
        "{said}"
    );
    // The original rides along, unchanged — the names are checkable.
    assert!(
        said.contains("(ts2322: Type 'Err<{ kind: \"OutOfRange\""),
        "{said}"
    );
}

#[test]
fn a_partial_union_stays_a_union_of_named_cases() {
    let declarations = table("variant E { A(x: number), B, C }\n");
    let named = name_types(
        "Type '{ kind: \"A\"; x: number; } | { kind: \"B\"; }' is not assignable to type 'E'.",
        &declarations,
    )
    .expect("named");
    assert_eq!(
        named,
        "Type 'E.A | E.B' is not assignable to type 'E'.".to_string()
    );
}

#[test]
fn a_tag_two_declarations_answer_to_is_not_named() {
    let declarations = table("variant A { Empty }\nvariant B { Empty }\n");
    assert_eq!(
        name_types("Type '{ kind: \"Empty\"; }'.", &declarations),
        None
    );
}

#[test]
fn a_type_that_only_shares_a_tag_is_not_named() {
    // Same tag, different payload — some other type, not this case.
    let declarations = table("variant E { A(x: number) }\n");
    assert_eq!(
        name_types("Type '{ kind: \"A\"; y: string; }'.", &declarations),
        None
    );
}

#[test]
fn a_message_with_no_structural_case_is_left_alone() {
    let declarations = table("variant E { A(x: number), B }\n");
    assert_eq!(
        name_types(
            "Property 'kind' does not exist on type 'number'.",
            &declarations
        ),
        None,
    );
    // ... and a translation of it carries no naming clause.
    let said = translate(
        AnchorKind::Try,
        2339,
        "Property 'kind' does not exist.",
        &declarations,
    )
    .expect("translated");
    assert!(!said.contains("in tt's names"), "{said}");
}

#[test]
fn a_case_nested_in_another_type_is_named_where_it_stands() {
    let declarations = table("variant E { A(x: number), B }\n");
    let named = name_types(
        "Type 'Array<{ kind: \"A\"; x: number; }>' is not assignable.",
        &declarations,
    )
    .expect("named");
    assert_eq!(named, "Type 'Array<E.A>' is not assignable.".to_string());
}

#[test]
fn a_tag_union_names_nothing() {
    // `{ kind: "A" | "B" }` is not one case, so it is not one name.
    let declarations = table("variant E { A(x: number), B }\n");
    assert_eq!(
        name_types("Type '{ kind: \"A\" | \"B\"; }'.", &declarations),
        None
    );
}

#[test]
fn translation_classes_group_incidental_ts_codes_by_tt_meaning() {
    assert_eq!(translation_class(AnchorKind::Try, 2339), Some("not-result"));
    assert_eq!(translation_class(AnchorKind::Try, 2571), Some("not-result"));
    assert_eq!(
        translation_class(AnchorKind::TryExit, 2322),
        Some("try-error-type")
    );
    assert_eq!(
        translation_class(AnchorKind::TryTest, 2322),
        Some("not-result")
    );
    assert_eq!(translation_class(AnchorKind::Try, 2322), None);
    assert_eq!(translation_class(AnchorKind::Pipe, 2339), None);
    assert_eq!(
        translation_class(AnchorKind::Pipe, 2345),
        Some("pipe-step-input")
    );
}

#[test]
fn a_pipe_mismatch_translates_to_the_steps_expectation() {
    let said = translate(
        AnchorKind::Pipe,
        2345,
        "Argument of type 'number' is not assignable to parameter of type 'string'.",
        &[],
    )
    .expect("translated");
    assert!(
        said.starts_with("this pipeline step expects `string`, but receives `number`"),
        "{said}"
    );
    assert!(said.contains("ts2345:"), "the original rides along: {said}");
}

#[test]
fn a_pipe_translation_uses_the_deepest_elaborated_pair() {
    // A flow boundary mismatches as two function types; the elaboration
    // descends to the value types, and the deepest pair is the boundary.
    let said = translate(
        AnchorKind::Pipe,
        2345,
        "Argument of type '(n: number) => number' is not assignable to parameter of type \
             '(n: number) => string'.\n  Type 'number' is not assignable to type 'string'.",
        &[],
    )
    .expect("translated");
    assert!(
        said.starts_with("this pipeline step expects `string`, but receives `number`"),
        "{said}"
    );
}

#[test]
fn an_unparseable_pipe_mismatch_still_says_what_the_step_meant() {
    let said = translate(AnchorKind::Pipe, 2345, "something unusual.", &[]).expect("translated");
    assert!(
        said.starts_with("this pipeline step cannot accept the value flowing into it"),
        "{said}"
    );
}

#[test]
fn assignability_pairs_parse_both_sentence_forms() {
    assert_eq!(
        assignability_pair("Argument of type 'A' is not assignable to parameter of type 'B'."),
        Some(("A".to_string(), "B".to_string()))
    );
    assert_eq!(
        assignability_pair("Type 'A' is not assignable to type 'B'."),
        Some(("A".to_string(), "B".to_string()))
    );
    assert_eq!(assignability_pair("This expression is not callable."), None);
}

#[test]
fn an_ordinary_ts_message_also_uses_tt_names() {
    let declarations = table("variant E { A(x: number), B }\n");
    let said = ts_message(
        "Type '{ kind: \"A\"; x: number; }' is not assignable.",
        &declarations,
    );
    assert_eq!(
        said,
        "Type '{ kind: \"A\"; x: number; }' is not assignable. \
             (in tt's names: Type 'E.A' is not assignable.)"
    );
}

#[test]
fn diagnostics_are_source_sorted_and_only_identical_duplicates_are_merged() {
    let at = |line, code: &str| Diagnostic {
        path: PathBuf::from("/p/a.tt"),
        position: Some((line, 1)),
        end: Some((line, 2)),
        message: "same".to_string(),
        code: Some(code.to_string()),
        suggestions: Vec::new(),
        labels: Vec::new(),
    };
    let finished = finish_diagnostics(vec![
        at(2, "ts9999"),
        at(1, "ts1000"),
        at(2, "ts1001"),
        at(2, "ts9999"),
    ]);
    let codes: Vec<_> = finished.iter().map(|d| d.code.as_deref()).collect();
    assert_eq!(codes, [Some("ts1000"), Some("ts1001"), Some("ts9999")]);
    assert_eq!(finished[0].position, Some((1, 1)));
    assert_eq!(finished[1].position, Some((2, 1)));
}

/// `ProjectionReport::editor_only` (src/lib/compile.rs):
/// "This projection retains malformed host syntax for the editor only.
/// It must never authorize build or declaration output."
///
/// Declaration output reaches disk only through `Project::check` with
/// `CheckRequest::emit_declarations` (`ttc --types`, src/main/typed.rs),
/// whose modules come from [`match_declarations`]. These tests feed it the
/// projections `ProjectedDocument::project_for_snapshot` builds for an
/// editor_only source and an emitted declaration for that module, and
/// assert none is matched back for writing.
mod editor_only_declarations {
    use super::*;
    use crate::typescript::backend::Declaration;
    use std::path::Path;

    /// Structural host recovery: `compile_projection_report` marks this
    /// editor_only (pinned in tests/compile/cases_09.rs).
    const EDITOR_ONLY: &str = "const v = 1 |> add(2, \nconst w = 1;\n";

    fn snapshot(
        files: Vec<Arc<ProjectedDocument>>,
        blocked: Vec<Arc<crate::engine::snapshot::BlockedFile>>,
    ) -> Snapshot {
        Snapshot {
            id: 0,
            files,
            blocked,
            deferred: Vec::new(),
            host_overlays: Default::default(),
        }
    }

    fn emitted_for(module: PathBuf) -> Answers {
        Answers {
            declarations: vec![Declaration {
                module,
                text: "export declare const w = 1;\n".to_string(),
            }],
            ..Answers::default()
        }
    }

    #[test]
    fn the_report_is_editor_only() {
        let report = crate::compile_projection_report(EDITOR_ONLY, &crate::Options::default());
        assert!(report.editor_only, "{:?}", report.diagnostics);
    }

    #[test]
    fn a_closed_editor_only_file_is_blocked_and_writes_no_declaration() {
        // project_for_snapshot: "A file whose TypeScript does not parse is
        // blocked, as `tsc` reports only its syntax errors while there are
        // any, unless it is `open`". `ttc --types` opens no document.
        let path = PathBuf::from("/p/src/a.tt");
        let blocked =
            match ProjectedDocument::project_for_snapshot(&path, EDITOR_ONLY.to_string(), false) {
                Err(blocked) => blocked,
                Ok(_) => panic!("a closed editor_only projection must not join the program"),
            };
        let module = projection::module_path_of(&path);
        let snapshot = snapshot(Vec::new(), vec![Arc::new(blocked)]);
        let requested: HashSet<PathBuf> = [path].into_iter().collect();
        let declarations = match_declarations(&snapshot, &emitted_for(module), &requested);
        assert!(declarations.modules.is_empty());
    }

    #[test]
    fn an_open_editor_only_file_writes_no_declaration() {
        // reaches_placeholders (semantics/declarations.rs): "a projection
        // whose declarations are not the file's own (an unparsed one, ...)
        // ... Their declarations are not written".
        let path = PathBuf::from("/p/src/a.tt");
        let doc = ProjectedDocument::project_for_snapshot(&path, EDITOR_ONLY.to_string(), true)
            .expect("an open document is projected for the editor");
        assert!(
            doc.unparsed,
            "an editor_only projection is served as unparsed"
        );
        let module = doc.module_path.clone();
        let snapshot = snapshot(vec![Arc::new(doc)], Vec::new());
        let requested: HashSet<PathBuf> = [path].into_iter().collect();
        let declarations = match_declarations(&snapshot, &emitted_for(module), &requested);
        assert!(declarations.modules.is_empty());
    }

    /// `b.tt` importing `a.tt`, written to disk: `reaches_placeholders`
    /// resolves imports with `canonicalize()`. Returns the workspace and the
    /// two canonical paths.
    fn importer_of(a_source: &str) -> (crate::test_workspace::Workspace, PathBuf, PathBuf) {
        let dir = crate::test_workspace::Workspace::new("editor-only-importer");
        std::fs::write(dir.join("a.tt"), a_source).expect("writes a.tt");
        std::fs::write(dir.join("b.tt"), IMPORTER).expect("writes b.tt");
        let a = dir.join("a.tt").canonicalize().expect("a.tt exists");
        let b = dir.join("b.tt").canonicalize().expect("b.tt exists");
        (dir, a, b)
    }

    const IMPORTER: &str = "import { w } from \"./a.tt\";\nexport const x = w;\n";

    #[test]
    fn an_importer_of_a_closed_editor_only_file_writes_no_declaration() {
        // reaches_placeholders (semantics/declarations.rs): "a blocked file
        // served as an empty module), and every file whose `.tt` imports
        // reach one. Their declarations are not written". b.tt is clean and
        // requested; only its import of the blocked a.tt can drop it.
        let (_dir, a, b) = importer_of(EDITOR_ONLY);
        let blocked =
            match ProjectedDocument::project_for_snapshot(&a, EDITOR_ONLY.to_string(), false) {
                Err(blocked) => blocked,
                Ok(_) => panic!("a closed editor_only projection must not join the program"),
            };
        let importer = ProjectedDocument::project_for_snapshot(&b, IMPORTER.to_string(), false)
            .expect("the importer projects");
        assert!(!importer.unparsed && !importer.recovered_declaration);
        assert!(!importer.tt_imports().is_empty(), "b.tt imports a.tt");
        let module = importer.module_path.clone();
        let snapshot = snapshot(vec![Arc::new(importer)], vec![Arc::new(blocked)]);
        let requested: HashSet<PathBuf> = [a, b].into_iter().collect();
        let declarations = match_declarations(&snapshot, &emitted_for(module), &requested);
        assert!(declarations.modules.is_empty());
    }

    #[test]
    fn an_importer_of_a_clean_file_writes_its_declaration() {
        // Control for the test above: the same b.tt, with a.tt clean, keeps
        // the declaration emitted for it, so the blocked a.tt is what drops b.
        const CLEAN: &str = "export const w = 1;\n";
        let (_dir, a, b) = importer_of(CLEAN);
        let clean = ProjectedDocument::project_for_snapshot(&a, CLEAN.to_string(), false)
            .expect("a clean file projects");
        let importer = ProjectedDocument::project_for_snapshot(&b, IMPORTER.to_string(), false)
            .expect("the importer projects");
        let module = importer.module_path.clone();
        let snapshot = snapshot(vec![Arc::new(clean), Arc::new(importer)], Vec::new());
        let requested: HashSet<PathBuf> = [a, b.clone()].into_iter().collect();
        let declarations = match_declarations(&snapshot, &emitted_for(module), &requested);
        assert_eq!(declarations.modules.len(), 1);
        assert_eq!(declarations.modules[0].file.source_path, b);
    }

    #[test]
    fn a_clean_file_still_writes_its_declaration() {
        // Control: match_declarations keeps a requested file's emitted
        // declaration ("Only requested files are kept"), so the two tests
        // above are not vacuous.
        let path = PathBuf::from("/p/src/a.tt");
        let doc = ProjectedDocument::project_for_snapshot(
            Path::new(&path),
            "export const w = 1;\n".to_string(),
            false,
        )
        .expect("a clean file projects");
        assert!(!doc.unparsed);
        let module = doc.module_path.clone();
        let snapshot = snapshot(vec![Arc::new(doc)], Vec::new());
        let requested: HashSet<PathBuf> = [path].into_iter().collect();
        let declarations = match_declarations(&snapshot, &emitted_for(module), &requested);
        assert_eq!(declarations.modules.len(), 1);
    }
}
