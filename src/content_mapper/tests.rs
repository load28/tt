use super::*;

#[test]
fn free_intervals_carve_around_occupied_stretches() {
    let occupied = [(5, 10), (12, 20)];
    assert_eq!(
        free_intervals(0, 25, &occupied),
        [(0, 5), (10, 12), (20, 25)]
    );
    assert_eq!(free_intervals(6, 9, &occupied), []);
    assert_eq!(free_intervals(10, 12, &occupied), [(10, 12)]);
}

#[test]
fn passthrough_maps_as_one_verbatim_span() {
    let source = "const n: number = 1;\n";
    let emit = ttc::emit_mapped(source);
    let spans = span_mappings(&emit.mappings, &emit.anchors);
    assert_eq!(
        spans,
        [serde_json::json!([
            0,
            source.len(),
            0,
            source.len(),
            SPAN_VERBATIM
        ])]
    );
}

#[test]
fn glue_maps_to_its_construct_without_features() {
    let source = "variant E { A(x: number), B }\nconst v = match (E.A(1)) { A(x) => x, B => 0 };\n";
    let emit = ttc::emit_mapped(source);
    let spans = span_mappings(&emit.mappings, &emit.anchors);
    // Spans are sorted and non-overlapping in the virtual text.
    let mut last_end = 0u64;
    for span in &spans {
        let start = span[0].as_u64().unwrap();
        let length = span[1].as_u64().unwrap();
        assert!(start >= last_end, "overlap at {start}");
        last_end = start + length;
    }
    // The lowering wrote glue, and every glue span is Atom + no features.
    let atoms: Vec<_> = spans
        .iter()
        .filter(|s| s[4].as_u64() == Some(SPAN_ATOM))
        .collect();
    assert!(!atoms.is_empty(), "a match lowering has glue");
    for atom in atoms {
        assert_eq!(atom[5].as_u64(), Some(FEATURES_NONE));
    }
}

#[test]
fn code_numbers_are_stable_and_start_at_one() {
    assert_eq!(code_number("stray-pipe"), 1);
    assert_eq!(code_number("match-not-exhaustive"), 27);
    assert_eq!(code_number("result-tail-semicolon"), 33);
    assert_eq!(code_number("lowering-plan-failed"), 34);
    assert_eq!(code_number("result-no-success-value"), 35);
    assert_eq!(code_number("try-crosses-value-region"), 42);
    for code in ttc::DiagnosticCode::ALL {
        assert_ne!(
            code_number(code.as_str()),
            0,
            "active diagnostic {} has no mapper wire number",
            code.as_str()
        );
    }
    assert_eq!(code_number("never-heard-of-it"), 0);
}

/// A scratch directory for one case, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!("tt-cm-{tag}-{}-{nonce}", std::process::id()));
        std::fs::create_dir_all(&path).expect("a writable temporary directory");
        Scratch(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn session() -> Session {
    Session {
        open_projects: HashSet::new(),
        ensured_roots: HashSet::new(),
    }
}

fn request(method: &str, params: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "jsonrpc": "2.0", "id": "api1", "method": method, "params": params })
}

#[test]
fn framed_messages_round_trip() {
    let mut wire = Vec::new();
    let message = serde_json::json!({ "id": "api1", "method": "initialize" });
    write_message(&mut wire, &message).unwrap();
    assert!(wire.starts_with(b"Content-Length: "));
    let mut reader = std::io::Cursor::new(wire);
    assert_eq!(read_message(&mut reader).unwrap(), Some(message));
    // The stream ends cleanly after the one message.
    assert_eq!(read_message(&mut reader).unwrap(), None);
}

#[test]
fn framing_tolerates_extra_headers_and_case() {
    let body = r#"{"id":1}"#;
    let wire = format!(
        "Content-Type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut reader = std::io::Cursor::new(wire.into_bytes());
    assert_eq!(
        read_message(&mut reader).unwrap(),
        Some(serde_json::json!({ "id": 1 }))
    );
}

#[test]
fn framing_reports_a_broken_stream() {
    // A frame without Content-Length cannot be read past.
    let mut reader = std::io::Cursor::new(b"\r\n{}".to_vec());
    assert!(read_message(&mut reader).is_err());
    // EOF inside a header block is a died-mid-message peer.
    let mut reader = std::io::Cursor::new(b"Content-Length: 5\r\n".to_vec());
    assert!(read_message(&mut reader).is_err());
    // A body shorter than its declared length is the same.
    let mut reader = std::io::Cursor::new(b"Content-Length: 99\r\n\r\n{}".to_vec());
    assert!(read_message(&mut reader).is_err());
}

#[test]
fn initialize_picks_utf8_and_names_the_tt_source() {
    let result = initialize(&serde_json::json!({
        "positionEncodings": ["utf-8", "utf-16"],
    }))
    .unwrap();
    assert_eq!(result["positionEncoding"], "utf-8");
    assert_eq!(result["diagnosticSource"], "tt");
}

#[test]
fn initialize_refuses_a_peer_without_utf8() {
    let error = initialize(&serde_json::json!({ "positionEncodings": ["utf-16"] }))
        .expect_err("utf-8 is required");
    assert_eq!(error.code, INVALID_PARAMS);
}

#[test]
fn unknown_and_missing_methods_answer_method_not_found() {
    let mut session = session();
    let error = respond(&mut session, &request("shutdown", serde_json::json!({})))
        .expect_err("unknown method");
    assert_eq!(error.code, METHOD_NOT_FOUND);
    let error =
        respond(&mut session, &serde_json::json!({ "id": 1 })).expect_err("no method at all");
    assert_eq!(error.code, METHOD_NOT_FOUND);
}

#[test]
fn missing_string_params_answer_invalid_params() {
    let mut session = session();
    for method in ["openProject", "closeProject", "transform"] {
        let error = respond(&mut session, &request(method, serde_json::json!({})))
            .expect_err("params are required");
        assert_eq!(error.code, INVALID_PARAMS, "{method}");
    }
}

#[test]
fn open_project_tracks_the_handle_and_materializes_std() {
    let scratch = Scratch::new("open");
    std::fs::write(scratch.path().join("package.json"), "{}\n").unwrap();
    let config = scratch.path().join("tsconfig.json");
    let mut session = session();

    let result = respond(
        &mut session,
        &request(
            "openProject",
            serde_json::json!({
                "configFileName": config.to_str().unwrap(),
                "projectHandle": "p:0",
            }),
        ),
    )
    .unwrap();
    assert_eq!(result, serde_json::json!({}));
    assert!(session.open_projects.contains("p:0"));
    let std_entry = scratch.path().join("node_modules/@tt/std/index.ts");
    assert!(std_entry.exists());
    assert!(
        scratch
            .path()
            .join("node_modules/@tt/runtime/index.ts")
            .exists()
    );

    // Never over one that already exists: a project's own copy stays.
    std::fs::write(&std_entry, "// mine\n").unwrap();
    session.ensured_roots.clear();
    ensure_std_packages(&mut session, scratch.path());
    assert_eq!(std::fs::read_to_string(&std_entry).unwrap(), "// mine\n");

    let result = respond(
        &mut session,
        &request(
            "closeProject",
            serde_json::json!({ "projectHandle": "p:0" }),
        ),
    )
    .unwrap();
    assert_eq!(result, serde_json::json!({}));
    assert!(!session.open_projects.contains("p:0"));
}

#[test]
fn a_project_without_a_config_file_opens_too() {
    let mut session = session();
    let result = respond(
        &mut session,
        &request(
            "openProject",
            serde_json::json!({ "configFileName": "", "projectHandle": "p:1" }),
        ),
    )
    .unwrap();
    assert_eq!(result, serde_json::json!({}));
}

#[test]
fn transform_serves_a_variant_file_as_virtual_typescript() {
    let scratch = Scratch::new("transform");
    std::fs::write(scratch.path().join("package.json"), "{}\n").unwrap();
    let file = scratch.path().join("shape.tt");
    let source = "export variant Shape { Circle(radius: number), Point }\n\
             export const tag = (s: Shape) => match (s) { Circle(radius) => \"c\", Point => \"p\" };\n";
    let mut session = session();

    let result = respond(
        &mut session,
        &request(
            "transform",
            serde_json::json!({
                "fileName": file.to_str().unwrap(),
                "content": source,
                "projectHandle": "p:0",
            }),
        ),
    )
    .unwrap();
    assert_eq!(result["extension"], ".ts");
    assert_eq!(result["diagnostics"], serde_json::json!([]));
    let text = result["text"].as_str().unwrap();
    assert!(text.contains("kind: \"Circle\""));
    let mappings = result["mappings"].as_array().unwrap();
    assert!(
        mappings
            .iter()
            .any(|m| m[4].as_u64() == Some(SPAN_VERBATIM))
    );
    assert!(mappings.iter().any(|m| m[4].as_u64() == Some(SPAN_ATOM)));
}

#[test]
fn transform_reads_one_hop_imports_for_exhaustiveness() {
    let scratch = Scratch::new("extern");
    std::fs::write(
        scratch.path().join("shape.tt"),
        "export variant Shape { Circle(radius: number), Rect(width: number, height: number) }\n",
    )
    .unwrap();
    let file = scratch.path().join("partial.tt");
    let source = "import { Shape } from \"./shape.tt\";\n\
             export const tag = (s: Shape): string => match (s) { Circle(radius) => \"c\" };\n";
    let mut session = session();

    let result = respond(
        &mut session,
        &request(
            "transform",
            serde_json::json!({
                "fileName": file.to_str().unwrap(),
                "content": source,
                "projectHandle": "p:0",
            }),
        ),
    )
    .unwrap();
    let diagnostics = result["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0]["code"], code_number("match-not-exhaustive"));
    let message = diagnostics[0]["messageText"].as_str().unwrap();
    assert!(message.contains("not exhaustive"), "{message}");
    assert!(
        message.contains("help:"),
        "suggestions ride along: {message}"
    );
    // The diagnostic sits at the match keyword, as byte offsets.
    let start = diagnostics[0]["start"].as_u64().unwrap() as usize;
    assert_eq!(&source[start..start + 5], "match");
}

#[test]
fn namespace_imports_qualify_their_extern_variants() {
    let scratch = Scratch::new("ns");
    std::fs::write(
        scratch.path().join("dep.tt"),
        "export variant Mode { Fast(), Safe }\n",
    )
    .unwrap();
    let imports = [
        ttc::TtImport {
            specifier: "./dep.tt".to_string(),
            names: ttc::TtImportNames::Namespace("dep".to_string()),
        },
        ttc::TtImport {
            specifier: "./missing.tt".to_string(),
            names: ttc::TtImportNames::Named(vec![("Gone".to_string(), None)]),
        },
        ttc::TtImport {
            specifier: "./dep.tt".to_string(),
            names: ttc::TtImportNames::None,
        },
    ];
    let externs = collect_extern_variants(&scratch.path().join("main.tt"), &imports);
    assert_eq!(externs.len(), 1);
    assert_eq!(externs[0].name, "dep.Mode");
}

#[test]
fn a_blocked_projection_serves_an_empty_module_with_the_cause() {
    let mut session = session();
    let result = respond(
        &mut session,
        &request(
            "transform",
            serde_json::json!({
                "fileName": "/nonexistent/broken.tt",
                "content": "export variant Broken { Value(x: number]) }\n",
                "projectHandle": "p:0",
            }),
        ),
    )
    .unwrap();
    assert_eq!(result["text"], "");
    assert_eq!(result["mappings"], serde_json::json!([]));
    let diagnostics = result["diagnostics"].as_array().unwrap();
    assert!(!diagnostics.is_empty());
    assert_eq!(
        diagnostics[0]["code"],
        code_number("variant-invalid-field-type")
    );
}

#[test]
fn a_ttx_file_serves_as_tsx() {
    let mut session = session();
    let result = respond(
        &mut session,
        &request(
            "transform",
            serde_json::json!({
                "fileName": "/nonexistent/view.ttx",
                "content": "export const view = <div>hello</div>;\n",
                "projectHandle": "p:0",
            }),
        ),
    )
    .unwrap();
    assert_eq!(result["extension"], ".tsx");
    assert_eq!(result["text"], "export const view = <div>hello</div>;\n");
}

#[test]
fn std_imports_materialize_next_to_the_nearest_package_root() {
    let scratch = Scratch::new("std");
    std::fs::write(scratch.path().join("package.json"), "{}\n").unwrap();
    let nested = scratch.path().join("src/deep");
    std::fs::create_dir_all(&nested).unwrap();
    let mut session = session();

    respond(
            &mut session,
            &request(
                "transform",
                serde_json::json!({
                    "fileName": nested.join("opt.tt").to_str().unwrap(),
                    "content": "import * as Option from \"@tt/std/option\";\nexport const some = Option.Some(1);\n",
                    "projectHandle": "p:0",
                }),
            ),
        )
        .unwrap();
    assert!(
        scratch
            .path()
            .join("node_modules/@tt/std/option.ts")
            .exists()
    );
}

#[test]
fn package_root_walks_to_a_marker_or_gives_up() {
    let scratch = Scratch::new("root");
    let nested = scratch.path().join("a/b");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(scratch.path().join("package.json"), "{}\n").unwrap();
    assert_eq!(
        package_root(&nested.join("x.tt")),
        Some(scratch.path().to_path_buf())
    );
    assert_eq!(package_root(Path::new("/no/such/tree/x.tt")), None);
}

#[test]
fn positionless_diagnostics_serialize_as_zero_spans() {
    let diagnostic = &ttc::analyze("export variant Dup { A, A }\n", &Options::default())[0];
    let wire = mapper_diagnostic(diagnostic);
    assert_eq!(wire["code"], code_number("variant-duplicate-case"));
    assert!(wire["start"].as_u64().is_some());
    assert!(wire["length"].as_u64().is_some());
}
