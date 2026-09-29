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
