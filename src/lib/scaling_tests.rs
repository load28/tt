use std::collections::HashMap;
use std::path::Path;

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
    let root = crate::test_workspace::Workspace::with_subdir("contextual-scaling", "src");
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
