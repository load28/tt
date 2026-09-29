fn with_installed_tt_mapper(dir: &Path) {
    let package = dir.join("node_modules/@openload28/tt-lang");
    fs::create_dir_all(&package).unwrap();
    let manifest = serde_json::json!({
        "name": "@openload28/tt-lang",
        "version": "0.0.0-test",
        "typescript": {
            "contentMapper": { "exec": [env!("CARGO_BIN_EXE_ttc"), "--content-mapper"] },
        },
    });
    fs::write(package.join("package.json"), manifest.to_string()).unwrap();
}

fn language_project(dir: &Path, file: &str) -> ttc::engine::Project {
    let path = dir.join(file).canonicalize().unwrap();
    ttc::engine::Engine::new(None)
        .open_document_project(&path, &ttc::engine::ProjectOptions::default())
        .unwrap()
}

fn source_path(dir: &Path, file: &str) -> PathBuf {
    dir.join(file).canonicalize().unwrap()
}

fn utf16_position(source: &str, needle: &str) -> ttc::engine::Position {
    let at = source.find(needle).unwrap_or_else(|| panic!("{needle:?} in {source:?}"));
    let line_start = source[..at].rfind('\n').map_or(0, |newline| newline + 1);
    ttc::engine::Position {
        line: source[..at].matches('\n').count() as u32,
        character: source[line_start..at].encode_utf16().count() as u32,
    }
}

fn utf16_slice(source: &str, range: ttc::engine::Range) -> &str {
    let offset = |position: ttc::engine::Position| {
        let line_start = source
            .split_inclusive('\n')
            .take(position.line as usize)
            .map(str::len)
            .sum::<usize>();
        let mut units = 0;
        for (at, ch) in source[line_start..].char_indices() {
            if units >= position.character as usize {
                return line_start + at;
            }
            units += ch.len_utf16();
        }
        source.len()
    };
    &source[offset(range.start)..offset(range.end)]
}

const WIDE_SOURCE: (&str, &str) = (
    "src/wide.tt",
    "import seven from \"./a.tt\";\nconst 한글 = \"가나다\"; const z: string = seven;\nexport { 한글, z };\n",
);

fn relative_source(file: &str) -> &'static str {
    RELATIVE_SOURCES
        .iter()
        .chain([WIDE_SOURCE].iter())
        .find(|(name, _)| *name == file)
        .map(|(_, text)| *text)
        .unwrap()
}

fn esm_sources() -> Vec<(&'static str, &'static str)> {
    let mut files = RELATIVE_SOURCES.to_vec();
    files.push(WIDE_SOURCE);
    files
}

#[test]
fn the_language_service_resolves_tt_specifiers_under_node_esm_through_the_installed_mapper() {
    require_tsgo!();
    for (module, verbatim) in [("nodenext", true), ("node16", false)] {
        let dir = module_project("module", module, module, verbatim, &esm_sources());
        with_tt_content_mapper(&dir);
        with_installed_tt_mapper(&dir);
        let mut project = language_project(&dir, "src/c.ts");

        let c = source_path(&dir, "src/c.ts");
        let c_source = relative_source("src/c.ts");
        let diagnostics = project.service_diagnostics(&c).unwrap();
        assert_eq!(
            diagnostics.iter().map(|d| d.code).collect::<Vec<_>>(),
            vec![2322],
            "{module}: {diagnostics:?}"
        );
        assert_eq!(utf16_slice(c_source, diagnostics[0].range), "bad");

        let hover = project
            .hover(&c, utf16_position(c_source, "X(seven)"))
            .unwrap()
            .unwrap_or_else(|| panic!("{module}: no hover"));
        assert!(hover.signature.contains("X"), "{module}: {hover:?}");
        assert_eq!(utf16_slice(c_source, hover.range), "X");

        let definition = project
            .definition(&c, utf16_position(c_source, "A.X"))
            .unwrap();
        assert!(
            definition
                .iter()
                .any(|location| location.path == source_path(&dir, "src/a.tt")),
            "{module}: {definition:?}"
        );

        for file in ["src/d.tt", "src/view.ttx"] {
            let diagnostics = project
                .service_diagnostics(&source_path(&dir, file))
                .unwrap();
            assert!(diagnostics.is_empty(), "{module} {file}: {diagnostics:?}");
        }

        let d = source_path(&dir, "src/d.tt");
        let d_source = relative_source("src/d.tt");
        let hover = project
            .hover(&d, utf16_position(d_source, "n) =>"))
            .unwrap()
            .unwrap_or_else(|| panic!("{module}: no binding hover"));
        assert!(hover.signature.contains("n: number"), "{module}: {hover:?}");
        assert_eq!(utf16_slice(d_source, hover.range), "n");

        let wide = source_path(&dir, "src/wide.tt");
        let diagnostics = project.service_diagnostics(&wide).unwrap();
        assert_eq!(
            diagnostics.iter().map(|d| d.code).collect::<Vec<_>>(),
            vec![2322],
            "{module}: {diagnostics:?}"
        );
        assert_eq!(utf16_slice(WIDE_SOURCE.1, diagnostics[0].range), "z");
    }
}

#[test]
fn the_language_service_keeps_the_lowered_arrangement_without_a_configured_installed_mapper() {
    require_tsgo!();
    for case in ["not installed", "not configured", "foreign mapper"] {
        let dir = module_project("module", "node16", "node16", false, RELATIVE_SOURCES);
        if case != "not configured" {
            with_tt_content_mapper(&dir);
        }
        if case != "not installed" {
            with_installed_tt_mapper(&dir);
        }
        if case == "foreign mapper" {
            let config = dir.join("tsconfig.json");
            let text = fs::read_to_string(&config).unwrap().replacen(
                "\"contentMappers\": [",
                "\"contentMappers\": [{ \"package\": \"foo-mapper\", \"extensions\": [\".foo\"] }, ",
                1,
            );
            fs::write(config, text).unwrap();
        }
        let mut project = language_project(&dir, "src/c.ts");
        let diagnostics = project
            .service_diagnostics(&source_path(&dir, "src/c.ts"))
            .unwrap();
        assert!(
            diagnostics.iter().any(|d| d.code == 2307),
            "{case}: {diagnostics:?}"
        );
    }
}

fn language_answers(dir: &Path) -> String {
    let mut project = language_project(dir, "src/c.ts");
    let c = source_path(dir, "src/c.ts");
    let a = source_path(dir, "src/a.tt");
    let d = source_path(dir, "src/d.tt");
    let c_source = relative_source("src/c.ts");
    let a_source = relative_source("src/a.tt");
    let d_source = relative_source("src/d.tt");
    let mut references = project
        .references(&a, utf16_position(a_source, "A {"))
        .unwrap();
    references.sort_by_key(|reference| {
        (
            reference.location.path.clone(),
            reference.location.range.start.line,
            reference.location.range.start.character,
        )
    });
    let answers = format!(
        "{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}",
        project.service_diagnostics(&c).unwrap(),
        project.service_diagnostics(&d).unwrap(),
        project
            .service_diagnostics(&source_path(dir, "src/view.ttx"))
            .unwrap(),
        project.hover(&c, utf16_position(c_source, "X(seven)")).unwrap(),
        project.hover(&c, utf16_position(c_source, "\"./a.tt\"")).unwrap(),
        project.hover(&d, utf16_position(d_source, "n) =>")).unwrap(),
        project.definition(&c, utf16_position(c_source, "A.X")).unwrap(),
        project
            .signature_help(&c, utf16_position(c_source, "seven);"))
            .unwrap(),
        references,
    );
    answers.replace(&dir.canonicalize().unwrap().display().to_string(), "<dir>")
}

#[test]
fn a_configured_installed_mapper_changes_no_answer_where_extension_probing_already_resolved() {
    require_tsgo!();
    for (package_type, module, resolution) in [
        ("module", "esnext", "bundler"),
        ("commonjs", "node16", "node16"),
        ("commonjs", "commonjs", "bundler"),
    ] {
        let lowered = module_project(package_type, module, resolution, false, RELATIVE_SOURCES);
        with_tt_content_mapper(&lowered);
        let mapped = module_project(package_type, module, resolution, false, RELATIVE_SOURCES);
        with_tt_content_mapper(&mapped);
        with_installed_tt_mapper(&mapped);
        let expected = language_answers(&lowered);
        assert!(!expected.contains("2307"), "{expected}");
        assert_eq!(
            language_answers(&mapped),
            expected,
            "{package_type}/{module}/{resolution}"
        );
    }
}

#[test]
fn a_mapped_project_answers_for_a_tt_file_outside_its_configuration() {
    require_tsgo!();
    let dir = module_project("module", "node16", "node16", false, RELATIVE_SOURCES);
    with_tt_content_mapper(&dir);
    with_installed_tt_mapper(&dir);
    fs::create_dir_all(dir.join("other")).unwrap();
    let outside = "import seven from \"../src/a.tt\";\nexport const y: string = seven;\n";
    write(&dir, "other/y.tt", outside);
    let y = source_path(&dir, "other/y.tt");
    let mut project = language_project(&dir, "other/y.tt");
    let diagnostics = project.service_diagnostics(&y).unwrap();
    assert_eq!(
        diagnostics.iter().map(|d| d.code).collect::<Vec<_>>(),
        vec![2322],
        "{diagnostics:?}"
    );
    assert_eq!(utf16_slice(outside, diagnostics[0].range), "y");
    let hover = project
        .hover(&y, utf16_position(outside, "seven;"))
        .unwrap()
        .expect("hover");
    assert!(hover.signature.ends_with(": 7"), "{hover:?}");
}

#[test]
fn a_mapped_project_answers_inside_a_buffer_whose_lowering_does_not_parse() {
    require_tsgo!();
    let dir = module_project("module", "node16", "node16", false, RELATIVE_SOURCES);
    with_tt_content_mapper(&dir);
    with_installed_tt_mapper(&dir);
    let d = source_path(&dir, "src/d.tt");
    let broken = "import { A } from \"./a.tt\";\nconst t = A.\n";
    let mut project = language_project(&dir, "src/d.tt");
    project.open_document(d.clone(), broken.to_string());
    let end = ttc::engine::Position {
        line: 1,
        character: "const t = A.".len() as u32,
    };
    let completion = project.completion(&d, end, true).unwrap();
    let labels: Vec<_> = completion.items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"X") && labels.contains(&"Y"), "{labels:?}");
    let hover = project
        .hover(&d, utf16_position(broken, "A.\n"))
        .unwrap()
        .expect("hover");
    assert!(hover.signature.contains("X"), "{hover:?}");
}

#[test]
fn the_server_answers_node_esm_tt_imports_through_the_installed_mapper() {
    use std::io::Write;
    require_tsgo!();
    let dir = module_project("module", "nodenext", "nodenext", true, RELATIVE_SOURCES);
    with_tt_content_mapper(&dir);
    with_installed_tt_mapper(&dir);
    let c = source_path(&dir, "src/c.ts");
    let d = source_path(&dir, "src/d.tt");
    let c_source = relative_source("src/c.ts");
    let at = utf16_position(c_source, "X(seven)");
    let requests = [
        serde_json::json!({ "id": 1, "method": "tsDiagnostics", "params": { "path": d } }),
        serde_json::json!({ "id": 2, "method": "tsDiagnostics", "params": { "path": c } }),
        serde_json::json!({ "id": 3, "method": "hover",
            "params": { "path": c, "position": { "line": at.line, "character": at.character } } }),
    ];
    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .arg("--server")
        .current_dir(&dir)
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
    let answer = |id: u64| {
        answers
            .iter()
            .find(|answer| answer["id"] == id)
            .unwrap_or_else(|| panic!("no answer {id}: {answers:?}"))
    };
    assert_eq!(
        answer(1)["result"]["diagnostics"],
        serde_json::json!([]),
        "{answers:?}"
    );
    let codes: Vec<_> = answer(2)["result"]["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .map(|d| d["code"].clone())
        .collect();
    assert_eq!(codes, vec![serde_json::json!(2322)], "{answers:?}");
    assert!(
        answer(3)["result"]["signature"]
            .as_str()
            .is_some_and(|signature| signature.contains("X")),
        "{answers:?}"
    );
}
