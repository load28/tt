use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::work::measure;

fn statement_matches(count: usize) -> String {
    (0..count)
        .map(|i| format!("function f{i}(s: number) {{ match (s) {{ 1 => 1, _ => 2 }} }}\n"))
        .collect()
}

fn expression_matches(count: usize) -> String {
    (0..count)
        .map(|i| format!("const v{i} = (s: number) => match (s) {{ 1 => 1, _ => 2 }};\n"))
        .collect()
}

fn every_request(source: &str) {
    let path = Path::new("/scaling/main.tt");
    crate::compile(source, &crate::Options::default()).expect("the file compiles");
    crate::engine::semantic_tokens(source);
    crate::engine::tt_declarations(path, source);
    crate::engine::tt_hints(path, source);
}

fn assert_linear(small: &HashMap<&'static str, usize>, large: &HashMap<&'static str, usize>) {
    for (name, &work) in large {
        let before = small.get(name).copied().unwrap_or(0);
        assert!(
            work <= 2 * before + 64,
            "{name}: {before} units for n matches but {work} for 2n"
        );
    }
}

#[test]
fn yield_context_queries_do_linear_work_in_nested_expressions() {
    let source = |n| {
        format!(
            "function* g() {{ const values = {}{}0{}; }}",
            "[".repeat(n),
            "yield 1, ".repeat(n),
            "]".repeat(n),
        )
    };
    let lex = |n| {
        let text = source(n);
        crate::lexer::lex(&text, 0, text.len());
    };
    let small = measure(|| lex(100));
    let large = measure(|| lex(200));
    assert!(small["yield context probes"] > 0);
    assert_linear(&small, &large);
}

#[test]
fn every_request_does_linear_work_in_the_number_of_statement_matches() {
    let small = measure(|| every_request(&statement_matches(150)));
    let large = measure(|| every_request(&statement_matches(300)));
    assert_linear(&small, &large);
    assert!(large["generated name probes"] > 0);
    assert!(large["denied token visits"] > 0);
}

#[test]
fn every_request_does_linear_work_in_the_number_of_expression_matches() {
    let small = measure(|| every_request(&expression_matches(150)));
    let large = measure(|| every_request(&expression_matches(300)));
    assert_linear(&small, &large);
    assert!(large["concise arrow scans"] > 0);
}

fn joined_matches(count: usize, separator: &str) -> String {
    format!(
        "variant V {{ A, B }}\ndeclare const v: V;\ndeclare const c: boolean;\nexport const x = [{}];\nexport function g() {{ {}0{} }}\n",
        vec!["match (v) { A => 1, B => 2 }"; count].join(separator),
        "if (c) { const y = match (v) { A => 1, B => 2 }; ".repeat(count),
        " }".repeat(count),
    )
}

#[test]
fn compiling_does_linear_work_in_the_tt_values_of_one_expression() {
    for separator in [", ", " + "] {
        let compile = |count| {
            crate::compile(
                &joined_matches(count, separator),
                &crate::Options::default(),
            )
            .expect("the file compiles")
        };
        let small = measure(|| compile(100));
        let large = measure(|| compile(200));
        assert_linear(&small, &large);
        assert!(large["protocol step links"] > 0);
        assert!(large["parent path edges"] > 0);
        assert!(large["planned evaluation steps"] > 0);
    }
}

fn nested_matches(depth: usize) -> String {
    format!(
        "export variant V {{ A(v: V), B }}\ndeclare const a: V;\nexport const x = {}1{};\nexport function f() {{ {}g();{} }}\n",
        "match (a) { A(v) => ".repeat(depth),
        ", B => 2 }".repeat(depth),
        "if let A(v) = a { ".repeat(depth),
        " }".repeat(depth),
    )
}

#[test]
fn every_request_does_linear_work_in_the_nesting_depth_of_matches() {
    let small = measure(|| every_request(&nested_matches(60)));
    let large = measure(|| every_request(&nested_matches(120)));
    assert_linear(&small, &large);
    assert!(large["arm outline steps"] > 0);
}

#[test]
fn tt_matches_never_need_more_host_parses_as_they_multiply() {
    let small = measure(|| crate::parser::parse(&statement_matches(150)));
    let large = measure(|| crate::parser::parse(&statement_matches(300)));
    assert_eq!(small.get("host parses"), large.get("host parses"));
}

fn variant_module(count: usize) -> String {
    let mut out = String::from("import { helper } from \"./helper.js\";\n\n");
    for n in 0..count {
        out.push_str(&format!(
            "export variant Shape{n} {{\n  Circle(radius: number),\n  Rect(width: number, height: number),\n  Empty,\n}}\n\n\
             export function area{n}(s: Shape{n}): number {{\n  return match (s) {{\n    Circle(radius) => Math.PI * radius ** 2,\n    Rect(width, height) => width * height,\n    Empty => 0,\n  }};\n}}\n\n\
             export const label{n} = (s: Shape{n}): string => {{\n  if let Circle(radius) = s {{\n    return radius.toFixed(1);\n  }}\n  return helper(String(s.kind));\n}};\n\n"
        ));
    }
    out
}

#[test]
fn compiling_a_file_lexes_its_source_projection_and_output_once_each() {
    for count in [1, 4, 8] {
        let source = variant_module(count);
        let work = measure(|| {
            crate::compile_mapped(
                &source,
                &crate::Options {
                    defer_to_checker: true,
                    ..crate::Options::default()
                },
            )
            .expect("the module compiles")
        });
        assert_eq!(work["source parses"], 1, "{count} variants");
        assert_eq!(work["whole-text lexes"], 3, "{count} variants");
    }
}

#[test]
fn projecting_a_file_for_a_snapshot_parses_it_once() {
    for count in [1, 4, 8] {
        let source = variant_module(count);
        let work = measure(|| {
            crate::engine::ProjectedDocument::project_for_snapshot(
                Path::new("/scaling/module.tt"),
                source.clone(),
                false,
            )
            .expect("the module projects")
        });
        assert_eq!(work["source parses"], 1, "{count} variants");
        assert_eq!(work["whole-text lexes"], 3, "{count} variants");
    }
}

#[test]
fn many_utf16_offsets_answer_what_one_offset_answers() {
    for source in [
        "",
        "abc",
        "\u{feff}const 한 = \"🎉\";\nx",
        "🎉ab",
        "a\u{feff}b",
    ] {
        let offsets = crate::Utf16Offsets::new(source);
        for byte in 0..source.len() + 3 {
            assert_eq!(
                offsets.offset(byte),
                crate::utf16_offset(source, byte),
                "{source:?} at {byte}"
            );
        }
    }
}

fn contextual_project(files: usize) -> (crate::test_workspace::Workspace, std::path::PathBuf) {
    contextual_files(
        crate::test_workspace::Workspace::with_subdir("contextual-scaling", "src"),
        files,
    )
}

fn contextual_files(
    root: crate::test_workspace::Workspace,
    files: usize,
) -> (crate::test_workspace::Workspace, std::path::PathBuf) {
    std::fs::write(
        root.join("tsconfig.json"),
        r#"{ "compilerOptions": { "strict": true, "target": "esnext", "module": "preserve", "moduleResolution": "bundler", "noEmit": true, "skipLibCheck": true }, "include": ["src"] }"#,
    )
    .unwrap();
    std::fs::write(root.join("src/dep.ts"), "export const d: string = \"x\";\n").unwrap();
    for i in 0..files {
        let import = if i == 0 {
            "import { d } from \"./dep\";\n".to_owned()
        } else {
            format!("import {{ f{} }} from \"./f{}.tt\";\n", i - 1, i - 1)
        };
        std::fs::write(
            root.join(format!("src/f{i}.tt")),
            format!(
                "{import}export function f{i}(s: number) {{ const v = match (s) {{ 1 => {}, _ => [s] }}; return v; }}\n",
                if i == 0 { "d" } else { "\"a\"" }
            ),
        )
        .unwrap();
    }
    let canonical = root.canonicalize().unwrap();
    (root, canonical)
}

fn contextual_compile(root: &Path, file: usize) -> String {
    let path = root.join(format!("src/f{file}.tt"));
    let source = std::fs::read_to_string(&path).unwrap();
    let name = path.to_str().unwrap().to_owned();
    crate::compile(
        &source,
        &crate::Options {
            filename: Some(&name),
            ..crate::Options::default()
        },
    )
    .expect("the file compiles")
}

#[test]
fn project_files_share_one_projection_each_and_one_checker_materialization() {
    if crate::typescript::toolchain::client(Path::new(env!("CARGO_MANIFEST_DIR"))).is_err() {
        assert!(
            std::env::var_os("TTC_REQUIRE_TSGO")
                .is_none_or(|value| value.is_empty() || value == "0"),
            "TTC_REQUIRE_TSGO is set but no TypeScript toolchain was found"
        );
        return;
    }
    let files = 6;
    // A reference answer comes from a project no other call has seen.
    let fresh = |file: usize, dep: Option<&str>| {
        let (_workspace, root) = contextual_project(files);
        if let Some(dep) = dep {
            std::fs::write(root.join("src/dep.ts"), dep).unwrap();
        }
        contextual_compile(&root, file)
    };
    let expected: Vec<String> = (0..files).map(|file| fresh(file, None)).collect();
    assert!(
        expected[1].contains("let $tt_v0: (string) | (number[]);"),
        "{}",
        expected[1]
    );
    let (_workspace, root) = contextual_project(files);
    let mut shared = Vec::new();
    let first = measure(|| shared.push(contextual_compile(&root, 0)));
    let rest = measure(|| {
        for file in 1..files {
            shared.push(contextual_compile(&root, file));
        }
    });
    assert_eq!(shared, expected);
    assert_eq!(first["contextual projections"], files);
    assert!(first["contextual checker asks"] > 0);
    assert_eq!(rest.get("contextual projections"), None);
    assert_eq!(rest.get("contextual checker asks"), None);
    // Workers compiling the project in parallel share what it already knows.
    let workers: Vec<_> = (0..files)
        .map(|file| {
            let root = root.clone();
            std::thread::spawn(move || {
                let mut output = String::new();
                let counts = measure(|| output = contextual_compile(&root, file));
                (output, counts)
            })
        })
        .collect();
    for (file, worker) in workers.into_iter().enumerate() {
        let (output, counts) = worker.join().unwrap();
        assert_eq!(output, expected[file]);
        assert_eq!(counts.get("contextual projections"), None);
        assert_eq!(counts.get("contextual checker asks"), None);
    }
    let dep = "export const d: boolean = true;\n";
    std::fs::write(root.join("src/dep.ts"), dep).unwrap();
    let mut changed = String::new();
    let after = measure(|| changed = contextual_compile(&root, 0));
    assert_eq!(changed, fresh(0, Some(dep)));
    assert!(changed.contains("boolean"), "{changed}");
    assert!(after["contextual checker asks"] > 0);
}

fn toolchain_present() -> bool {
    if crate::typescript::toolchain::client(Path::new(env!("CARGO_MANIFEST_DIR"))).is_ok() {
        return true;
    }
    assert!(
        std::env::var_os("TTC_REQUIRE_TSGO").is_none_or(|value| value.is_empty() || value == "0"),
        "TTC_REQUIRE_TSGO is set but no TypeScript toolchain was found"
    );
    false
}

fn open_contextual_project(
    engine: &crate::engine::Engine,
    root: &Path,
    documents: &[(std::path::PathBuf, String)],
) -> crate::engine::Project {
    let mut project = engine
        .open_project(
            &[root.join("src").to_string_lossy().into_owned()],
            &crate::engine::ProjectOptions::default(),
        )
        .unwrap();
    for (path, text) in documents {
        project.open_document(path.clone(), text.clone());
    }
    project
}

fn project_emits(project: &mut crate::engine::Project) -> Vec<(std::path::PathBuf, String)> {
    let files = project.initial_files();
    let snapshot = project.update(&files).unwrap();
    let mut emits: Vec<_> = snapshot
        .files()
        .iter()
        .map(|file| (file.source_path.clone(), file.emit.code.clone()))
        .collect();
    emits.sort();
    emits
}

#[test]
fn project_requests_materialize_once_per_state_of_their_inputs() {
    if !toolchain_present() {
        return;
    }
    let (_workspace, root) = contextual_files(
        crate::test_workspace::Workspace::in_repo_with_subdir("contextual-requests", "src"),
        2,
    );
    let fresh = |documents: &[(std::path::PathBuf, String)]| {
        project_emits(&mut open_contextual_project(
            &crate::engine::Engine::new(None),
            &root,
            documents,
        ))
    };
    let edited = root.join("src/f1.tt");
    let dep = root.join("src/dep.ts");
    let mut documents = vec![(edited.clone(), std::fs::read_to_string(&edited).unwrap())];
    let engine = crate::engine::Engine::new(None);
    let mut project = open_contextual_project(&engine, &root, &documents);
    let hover = |project: &mut crate::engine::Project, character: u32| {
        project
            .hover(&edited, crate::engine::Position { line: 1, character })
            .unwrap();
    };
    let asks = |counts: &HashMap<&'static str, usize>| {
        counts.get("contextual checker asks").copied().unwrap_or(0)
    };

    let first = measure(|| hover(&mut project, 16));
    assert!(asks(&first) > 0);
    let repeated = measure(|| {
        for character in [16, 34, 40, 60, 16] {
            hover(&mut project, character);
        }
    });
    assert_eq!(asks(&repeated), 0, "unchanged hovers materialize again");
    // A hover settles the hovered file's reference closure. The first
    // whole-project request settles the modules outside it; asking again
    // reads the project's materialization.
    let mut emits = project_emits(&mut project);
    assert_eq!(emits, fresh(&documents));
    let unchanged = measure(|| emits = project_emits(&mut project));
    assert_eq!(asks(&unchanged), 0);
    assert_eq!(emits, fresh(&documents));

    documents[0].1 = documents[0].1.replace("_ => [s]", "_ => [s, s]");
    project.update_document(edited.clone(), documents[0].1.clone());
    let changed = measure(|| hover(&mut project, 16));
    assert!(asks(&changed) > 0, "an edited document reuses stale types");
    let again = measure(|| hover(&mut project, 34));
    assert_eq!(asks(&again), 0);
    assert_eq!(project_emits(&mut project), fresh(&documents));

    std::fs::write(&dep, "export const d: boolean = true;\n").unwrap();
    let disk = measure(|| emits = project_emits(&mut project));
    assert!(
        asks(&disk) > 0,
        "a disk dependency change reuses stale types"
    );
    assert!(
        emits.iter().any(|(_, code)| code.contains("boolean")),
        "{emits:?}"
    );
    assert_eq!(emits, fresh(&documents));

    documents.push((dep.clone(), "export const d: number = 1;\n".to_owned()));
    project.open_document(dep.clone(), documents[1].1.clone());
    let overlay = measure(|| emits = project_emits(&mut project));
    assert!(
        asks(&overlay) > 0,
        "a host overlay change reuses stale types"
    );
    assert!(
        emits.iter().all(|(_, code)| !code.contains("boolean")),
        "{emits:?}"
    );
    assert_eq!(emits, fresh(&documents));
    let settled = measure(|| hover(&mut project, 16));
    assert_eq!(asks(&settled), 0);
}

/// A materialization that read an unsaved host buffer is stale once the
/// buffer is closed: the file is read from disk again.
#[test]
fn a_file_question_after_closing_an_unsaved_dependency_reads_the_disk() {
    if !toolchain_present() {
        return;
    }
    let (_workspace, root) = contextual_files(
        crate::test_workspace::Workspace::in_repo_with_subdir("contextual-closed-overlay", "src"),
        1,
    );
    let target = root.join("src/f0.tt");
    let dep = root.join("src/dep.ts");
    let engine = crate::engine::Engine::new(None);
    let mut project = open_contextual_project(&engine, &root, &[]);
    let files = project.initial_files();
    let scoped = |project: &mut crate::engine::Project| {
        let snapshot = project.update_scoped(&files, Some(&target)).unwrap();
        snapshot
            .files()
            .iter()
            .find(|file| file.source_path == target)
            .map(|file| file.emit.code.clone())
            .unwrap()
    };
    let disk = scoped(&mut project);
    assert!(disk.contains("string"), "{disk}");
    project.open_document(dep.clone(), "export const d: boolean = true;\n".to_owned());
    let overlay = scoped(&mut project);
    assert!(overlay.contains("boolean"), "{overlay}");
    project.close_document(&dep);
    assert_eq!(scoped(&mut project), disk);
}

fn unbalanced_openers(kind: crate::SourceKind, source: &str) {
    crate::engine::semantic_tokens_with_kind(source, kind);
    let _ = crate::check_report(
        source,
        &crate::Options {
            source_kind: kind,
            ..crate::Options::default()
        },
    );
}

#[test]
fn every_request_does_linear_work_in_unclosed_type_shaped_openers() {
    let fuzzed = "\tK<-[(\tK<\tK<[({[( -[(\t(\tK<\tK<[({[(\tK<[({[(\tK<[({[<[({[(\tK<[({[(.....z.........\tK<[K<[({[(\tK<[({[<[({[(\tK<[({[(.....z.........\tK<[({[<0";
    for kind in [crate::SourceKind::TypeScript, crate::SourceKind::Tsx] {
        for (text, count) in [(fuzzed, 20), ("K<[({[(", 200)] {
            let [one, two, four] = [1, 2, 4]
                .map(|times| measure(|| unbalanced_openers(kind, &text.repeat(times * count))));
            for (name, &work) in &four {
                let at =
                    |counts: &HashMap<&'static str, usize>| counts.get(name).copied().unwrap_or(0);
                assert!(
                    work.saturating_sub(at(&two)) <= 2 * at(&two).saturating_sub(at(&one)) + 64,
                    "{kind:?} {name}: {} units for n openers, {} for 2n, {work} for 4n",
                    at(&one),
                    at(&two),
                );
            }
            assert!(four["type argument lookahead steps"] > 0, "{kind:?}");
            if text == fuzzed {
                assert!(four["expression lookahead steps"] > 0, "{kind:?}");
            }
            if kind == crate::SourceKind::Tsx {
                assert!(four["operand probe frames"] > 0);
            }
        }
    }
}

fn nested_templates(depth: usize) -> String {
    format!(
        "declare const x: string;\nexport const y = {}x{};\n",
        "`${".repeat(depth),
        "}`".repeat(depth),
    )
}

#[test]
fn every_request_does_linear_work_in_the_nesting_depth_of_templates() {
    let small = measure(|| every_request(&nested_templates(100)));
    let large = measure(|| every_request(&nested_templates(200)));
    assert_linear(&small, &large);
    assert!(large["statement form decisions"] > 0);
}

#[test]
fn every_request_does_linear_work_in_the_nesting_depth_of_templates_around_a_match() {
    let source = |depth: usize| {
        format!(
            "declare const v: number;\nexport const y = {}match (v) {{ 1 => 1, _ => 2 }}{};\n",
            "`a${".repeat(depth),
            "}b`".repeat(depth),
        )
    };
    let small = measure(|| every_request(&source(200)));
    let large = measure(|| every_request(&source(400)));
    assert_linear(&small, &large);
    assert!(large["match-owned offsets"] > 0);
}

#[test]
fn a_nested_template_with_a_host_candidate_collects_its_facts_once_per_level() {
    let source = |depth: usize| {
        format!(
            "declare const match: any;\nexport const y = match as {{ f: () => void }};\nexport const z = {}match{};\n",
            "`${".repeat(depth),
            "}`".repeat(depth),
        )
    };
    let small = measure(|| crate::parser::parse(&source(100)));
    let large = measure(|| crate::parser::parse(&source(200)));
    assert_linear(&small, &large);
}

fn statement_decisions(count: usize) -> String {
    let declarations = "variant Opt<T> { Has(item: T), Nope }\n\
                        import type { TResult } from \"@tt/std\";\n";
    let bodies: String = (0..count)
        .map(|i| {
            format!(
                "export function e{i}(o: Opt<number>): number {{ const Has(item: g{i}) = o else {{ return 0; }}; return g{i}; }}\n\
                 export function i{i}(o: Opt<number>): number {{ if let Has(item) = o {{ return item; }} return 0; }}\n\
                 export function t{i}(r: TResult<number, string>): TResult<number, string> {{ const v = try r; return {{ kind: \"Ok\", value: v }}; }}\n"
            )
        })
        .collect();
    format!("{declarations}{bodies}")
}

#[test]
fn every_request_does_linear_work_in_the_number_of_statement_decisions() {
    let small = measure(|| every_request(&statement_decisions(100)));
    let large = measure(|| every_request(&statement_decisions(200)));
    for name in [
        "function body table entries",
        "statements beside an edit",
        "completion scope candidates",
    ] {
        assert!(
            small.get(name).is_some_and(|&work| work > 0),
            "{name}: {small:?}"
        );
    }
    assert_linear(&small, &large);
}

fn erroring_project(name: &str, count: usize) -> (crate::test_workspace::Workspace, PathBuf) {
    let root = crate::test_workspace::Workspace::in_repo_with_subdir(name, "src");
    std::fs::write(
        root.join("tsconfig.json"),
        r#"{ "compilerOptions": { "strict": true, "target": "esnext", "module": "preserve", "moduleResolution": "bundler", "noEmit": true, "skipLibCheck": true }, "include": ["src"] }"#,
    )
    .unwrap();
    let functions = |prefix: &str| -> String {
        (0..count)
            .map(|i| {
                format!(
                    "export function {prefix}{i}(r: number): number {{ const v: string = r; return v; }}\n"
                )
            })
            .collect()
    };
    std::fs::write(
        root.join("src/main.tt"),
        format!(
            "{}export const m = (s: number) => match (s) {{ 1 => \"é\", _ => s }};\n",
            functions("t")
        ),
    )
    .unwrap();
    std::fs::write(root.join("src/side.ts"), functions("s")).unwrap();
    let canonical = root.canonicalize().unwrap();
    (root, canonical)
}

#[test]
fn a_check_measures_each_file_once_however_many_diagnostics_it_reports() {
    if !toolchain_present() {
        return;
    }
    let check = |name: &str, count: usize| {
        let (_workspace, root) = erroring_project(name, count);
        let engine = crate::engine::Engine::new(None);
        let mut project = engine
            .open_project(
                &[root.join("src").to_string_lossy().into_owned()],
                &crate::engine::ProjectOptions::default(),
            )
            .unwrap();
        let files = project.initial_files();
        let snapshot = project.update(&files).unwrap();
        let mut reported = 0;
        let work = measure(|| {
            reported = project
                .check(&snapshot, &crate::engine::CheckRequest::default())
                .unwrap()
                .diagnostics
                .len();
        });
        (reported, work)
    };
    let (small_reported, small) = check("measured-once-small", 20);
    let (large_reported, large) = check("measured-once-large", 40);
    assert_eq!(small_reported, 80);
    assert_eq!(large_reported, 160);
    for name in ["line measurements", "utf-16 measurements", "utf-16 scans"] {
        let before = small.get(name).copied().unwrap_or(0);
        let after = large.get(name).copied().unwrap_or(0);
        assert_eq!(
            before, after,
            "{name}: {before} for {small_reported} diagnostics but {after} for {large_reported}"
        );
    }
}

#[test]
fn compiling_maps_each_projected_span_once_for_nested_tt_values() {
    let chain = |count| {
        format!(
            "export variant V {{ A, B }}\ndeclare const v: V;\nexport const x = {};\n",
            vec!["match (v) { A => 1, B => 2 }"; count].join(" + "),
        )
    };
    let small = measure(|| every_request(&chain(100)));
    let large = measure(|| every_request(&chain(200)));
    assert_linear(&small, &large);
    assert!(large["projection span lookups"] > 0);
}
