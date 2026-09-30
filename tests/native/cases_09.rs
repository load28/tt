/// A solution `tsconfig.json` over two projects, `pb` referencing `pa` —
/// the layout TypeScript's project references describe. Returns the
/// workspace with each project's one file, canonical.
fn solution(a: &str, b: &str) -> (Workspace, PathBuf, PathBuf) {
    let dir = tmpdir();
    let options = r#""strict": true, "skipLibCheck": true, "target": "es2022", "module": "esnext", "moduleResolution": "bundler", "allowImportingTsExtensions": true"#;
    write(
        &dir,
        "tsconfig.json",
        r#"{ "files": [], "references": [{ "path": "./pa" }, { "path": "./pb" }] }"#,
    );
    fs::create_dir_all(dir.join("pa")).unwrap();
    fs::create_dir_all(dir.join("pb")).unwrap();
    write(
        &dir,
        "pa/tsconfig.json",
        &format!(
            r#"{{ "compilerOptions": {{ {options}, "composite": true, "emitDeclarationOnly": true }}, "include": ["*.tt", "*.ts"] }}"#
        ),
    );
    write(
        &dir,
        "pb/tsconfig.json",
        &format!(
            r#"{{ "compilerOptions": {{ {options}, "noEmit": true }}, "include": ["*.tt", "*.ts"], "references": [{{ "path": "../pa" }}] }}"#
        ),
    );
    write(&dir, "pa/a.tt", a);
    write(&dir, "pb/b.tt", b);
    let a = dir.join("pa/a.tt").canonicalize().unwrap();
    let b = dir.join("pb/b.tt").canonicalize().unwrap();
    (dir, a, b)
}

/// Every answer a `ttc --server` gives to `requests`, sent in order.
fn server_answers(dir: &Path, requests: &[serde_json::Value]) -> Vec<serde_json::Value> {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .arg("--server")
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("server starts");
    for request in requests {
        writeln!(child.stdin.as_mut().unwrap(), "{request}").unwrap();
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("server answers");
    let answers: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSON response"))
        .collect();
    assert_eq!(answers.len(), requests.len(), "{answers:?}");
    assert!(
        answers.iter().all(|answer| answer.get("error").is_none()),
        "{answers:?}"
    );
    answers
}

const PROVIDER: &str = "export variant V { A(x: number), B }\n\
                        export function mk(n: number): V { return V.A(n); }\n";
const CONSUMER: &str = "import { mk, V } from \"../pa/a.tt\";\n\
                        const v: V = mk(1);\n\
                        export { v };\n";

#[test]
fn references_and_rename_reach_every_open_project_that_sees_the_symbol() {
    require_tsgo!();
    let (_dir, a, b) = solution(PROVIDER, CONSUMER);
    let mut workspace = ttc::engine::Workspace::new(ttc::engine::Engine::new(None));
    workspace.open_document(&a, PROVIDER.to_string()).unwrap();
    workspace.open_document(&b, CONSUMER.to_string()).unwrap();

    let declaration = source_location(&a, PROVIDER, "mk(n", 0, 2);
    let imported = source_location(&b, CONSUMER, "mk, V", 0, 2);
    let called = source_location(&b, CONSUMER, "mk(1)", 0, 2);
    let references = workspace
        .references(&a, declaration.range.start)
        .expect("references answer");
    let places: Vec<_> = references
        .iter()
        .map(|reference| (reference.location.clone(), reference.is_definition))
        .collect();
    assert_eq!(
        places,
        vec![
            (declaration.clone(), true),
            (imported.clone(), false),
            (called.clone(), false),
        ],
        "the declaration's project cannot see pb; pb can: {references:?}"
    );

    let edits = workspace
        .rename(&a, declaration.range.start)
        .expect("rename answers")
        .expect("the declaration renames");
    let in_file = |path: &Path| -> Vec<_> {
        edits
            .iter()
            .filter(|edit| edit.location.path == path)
            .cloned()
            .collect()
    };
    assert_eq!(
        apply_rename(PROVIDER, &in_file(&a), "make"),
        PROVIDER.replace("mk(n", "make(n")
    );
    assert_eq!(
        apply_rename(CONSUMER, &in_file(&b), "make"),
        CONSUMER.replace("mk", "make")
    );

    // From the import, TypeScript renames the local alias and leaves the
    // declaration alone — so no other project is asked to rename it.
    let edits = workspace
        .rename(&b, called.range.start)
        .expect("rename answers")
        .expect("the alias renames");
    assert!(edits.iter().all(|edit| edit.location.path == b), "{edits:?}");
    assert_eq!(
        apply_rename(CONSUMER, &edits, "make"),
        CONSUMER
            .replace("{ mk, V }", "{ mk as make, V }")
            .replace("mk(1)", "make(1)")
    );
}

#[test]
fn a_project_sees_the_modules_its_graph_reaches() {
    let (_dir, a, b) = solution(PROVIDER, CONSUMER);
    let engine = ttc::engine::Engine::new(None);
    let options = ttc::engine::ProjectOptions::default();
    let mut pa = engine.open_document_project(&a, &options).unwrap();
    let mut pb = engine.open_document_project(&b, &options).unwrap();
    assert!(pa.sees(&a).unwrap());
    assert!(!pa.sees(&b).unwrap(), "pa's graph never reaches its consumer");
    assert!(pb.sees(&b).unwrap());
    assert!(pb.sees(&a).unwrap(), "pb's graph reaches what b.tt imports");

    let unrelated = project(&[("src/c.tt", "export const c = 1;\n")]);
    let c = unrelated.join("src/c.tt").canonicalize().unwrap();
    let mut pc = engine.open_document_project(&c, &options).unwrap();
    assert!(!pc.sees(&a).unwrap() && !pc.sees(&b).unwrap());
}

#[test]
fn the_server_answers_references_from_every_open_project() {
    require_tsgo!();
    let (dir, a, b) = solution(PROVIDER, CONSUMER);
    let at = source_position(PROVIDER, "mk(n", 0);
    let answers = server_answers(
        &dir,
        &[
            serde_json::json!({ "id": 1, "method": "openDocument",
                "params": { "path": a, "text": PROVIDER } }),
            serde_json::json!({ "id": 2, "method": "openDocument",
                "params": { "path": b, "text": CONSUMER } }),
            serde_json::json!({ "id": 3, "method": "references",
                "params": { "path": a, "position": { "line": at.line, "character": at.character } } }),
            serde_json::json!({ "id": 4, "method": "rename",
                "params": { "path": a, "position": { "line": at.line, "character": at.character } } }),
        ],
    );
    let files = |answer: &serde_json::Value, key: &str| -> Vec<String> {
        answer["result"][key]
            .as_array()
            .unwrap_or_else(|| panic!("{answer}"))
            .iter()
            .map(|place| place["path"].as_str().unwrap().to_string())
            .collect()
    };
    let (a, b) = (a.display().to_string(), b.display().to_string());
    assert_eq!(files(&answers[2], "locations"), [a.as_str(), b.as_str(), b.as_str()], "{answers:?}");
    assert_eq!(files(&answers[3], "edits"), [a.as_str(), b.as_str(), b.as_str()], "{answers:?}");
}

#[test]
fn an_unsaved_edit_reaches_the_importers_in_another_project() {
    require_tsgo!();
    let (dir, a, b) = solution(PROVIDER, CONSUMER);
    let edited = PROVIDER.replace(
        "mk(n: number): V { return V.A(n); }",
        "mk(n: string): V { return V.B; }",
    );
    let use_site = source_position(CONSUMER, "mk(1)", 0);
    let at = serde_json::json!({ "line": use_site.line, "character": use_site.character });
    let answers = server_answers(
        &dir,
        &[
            serde_json::json!({ "id": 1, "method": "openDocument",
                "params": { "path": a, "text": PROVIDER } }),
            serde_json::json!({ "id": 2, "method": "openDocument",
                "params": { "path": b, "text": CONSUMER } }),
            serde_json::json!({ "id": 3, "method": "hover",
                "params": { "path": b, "position": at } }),
            serde_json::json!({ "id": 4, "method": "updateDocument",
                "params": { "path": a, "text": edited } }),
            serde_json::json!({ "id": 5, "method": "hover",
                "params": { "path": b, "position": at } }),
            serde_json::json!({ "id": 6, "method": "tsDiagnostics",
                "params": { "path": b } }),
            serde_json::json!({ "id": 7, "method": "typedCheck",
                "params": { "path": b, "text": CONSUMER, "includeTypes": true } }),
            serde_json::json!({ "id": 8, "method": "closeDocument",
                "params": { "path": a } }),
            serde_json::json!({ "id": 9, "method": "hover",
                "params": { "path": b, "position": at } }),
            serde_json::json!({ "id": 10, "method": "tsDiagnostics",
                "params": { "path": b } }),
        ],
    );
    let signature = |id: usize| {
        answers[id - 1]["result"]["signature"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    };
    assert!(signature(3).contains("mk(n: number)"), "{answers:?}");
    assert!(
        signature(5).contains("mk(n: string)"),
        "pb sees pa's buffer, not its disk: {answers:?}"
    );
    assert!(signature(9).contains("mk(n: number)"), "{answers:?}");
    let diagnostics = |id: usize| -> Vec<serde_json::Value> {
        answers[id - 1]["result"]["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("{answers:?}"))
            .clone()
    };
    let codes = |id: usize| -> Vec<serde_json::Value> {
        diagnostics(id).iter().map(|d| d["code"].clone()).collect()
    };
    assert_eq!(codes(6), [serde_json::json!(2345)], "{answers:?}");
    assert!(
        diagnostics(7).iter().any(|d| d["path"] == b.display().to_string()
            && (d["code"] == "ts2345" || d["code"] == 2345)),
        "{answers:?}"
    );
    assert!(codes(10).is_empty(), "{answers:?}");
}

#[test]
fn an_unsaved_typescript_module_reaches_the_importers_in_another_project() {
    require_tsgo!();
    let consumer = "import { limit } from \"../pa/limit.ts\";\n\
                    export const within: number = limit;\n";
    let (dir, _a, b) = solution(PROVIDER, consumer);
    write(&dir, "pa/limit.ts", "export const limit = 1;\n");
    let limit = dir.join("pa/limit.ts").canonicalize().unwrap();
    let answers = server_answers(
        &dir,
        &[
            serde_json::json!({ "id": 1, "method": "openDocument",
                "params": { "path": b, "text": consumer } }),
            serde_json::json!({ "id": 2, "method": "openDocument",
                "params": { "path": limit, "text": "export const limit = \"none\";\n" } }),
            serde_json::json!({ "id": 3, "method": "tsDiagnostics",
                "params": { "path": b } }),
            serde_json::json!({ "id": 4, "method": "typedCheck",
                "params": { "path": b, "text": consumer, "includeTypes": true } }),
        ],
    );
    let reports = |id: usize| {
        answers[id - 1]["result"]["diagnostics"]
            .as_array()
            .is_some_and(|diagnostics| {
                diagnostics
                    .iter()
                    .any(|d| d["code"] == "ts2322" || d["code"] == 2322)
            })
    };
    assert!(reports(3), "{answers:?}");
    assert!(reports(4), "{answers:?}");
}

#[test]
fn a_document_open_in_another_project_is_not_a_root_of_this_one() {
    require_tsgo!();
    let clean = "export const a: number = 1;\n";
    let broken = "import { a } from \"../src/a.tt\";\nexport const b: string = a;\n";
    let dir = project(&[("src/a.tt", clean)]);
    fs::create_dir_all(dir.join("test")).unwrap();
    write(
        &dir,
        "test/tsconfig.json",
        r#"{ "compilerOptions": { "strict": true, "noEmit": true, "module": "preserve", "moduleResolution": "bundler", "allowImportingTsExtensions": true }, "include": ["*.tt"] }"#,
    );
    write(&dir, "test/b.tt", broken);
    let a = dir.join("src/a.tt").canonicalize().unwrap();
    let b = dir.join("test/b.tt").canonicalize().unwrap();
    let answers = server_answers(
        &dir,
        &[
            serde_json::json!({ "id": 1, "method": "openDocument",
                "params": { "path": a, "text": clean } }),
            serde_json::json!({ "id": 2, "method": "openDocument",
                "params": { "path": b, "text": broken } }),
            serde_json::json!({ "id": 3, "method": "typedCheck",
                "params": { "path": a, "text": clean, "includeTypes": true } }),
            serde_json::json!({ "id": 4, "method": "typedCheck",
                "params": { "path": b, "text": broken, "includeTypes": true } }),
        ],
    );
    assert_eq!(
        answers[2]["result"]["diagnostics"],
        serde_json::json!([]),
        "src's check stays about src's program: {answers:?}"
    );
    assert!(
        answers[3]["result"]["diagnostics"]
            .as_array()
            .is_some_and(|diagnostics| diagnostics
                .iter()
                .any(|d| d["path"] == b.display().to_string())),
        "{answers:?}"
    );
}

#[test]
fn an_editor_check_keeps_each_typescript_diagnostic_in_its_own_words() {
    require_tsgo!();
    // TASK-585: an arity error is not an assignability report, and the
    // assignability error at the same argument is a diagnostic of its own.
    let source = "function g(a: number): number { return a; }\n\
                  function f(s: string): string { return s; }\n\
                  export const r = f(g());\n";
    let dir = project(&[("src/a.tt", source)]);
    let a = dir.join("src/a.tt").canonicalize().unwrap();
    let answers = server_answers(
        &dir,
        &[
            serde_json::json!({ "id": 1, "method": "openDocument",
                "params": { "path": a, "text": source } }),
            serde_json::json!({ "id": 2, "method": "typedCheck",
                "params": { "path": a, "text": source, "includeTypes": true } }),
        ],
    );
    let diagnostics = answers[1]["result"]["diagnostics"]
        .as_array()
        .unwrap_or_else(|| panic!("{answers:?}"))
        .clone();
    let said = |code: &str| {
        diagnostics
            .iter()
            .filter(|d| d["code"] == code)
            .map(|d| d["message"].as_str().unwrap_or_default().to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(said("ts2554"), ["Expected 1 arguments, but got 0."], "{answers:?}");
    assert_eq!(
        said("ts2345"),
        ["type mismatch: expected `string`, found `number`"],
        "{answers:?}"
    );
    assert_eq!(diagnostics.len(), 2, "{answers:?}");
}
