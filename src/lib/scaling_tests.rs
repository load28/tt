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
    let (_workspace, root) = contextual_project(files);
    let fresh: Vec<String> = (0..files)
        .map(|file| {
            let root = root.clone();
            std::thread::spawn(move || contextual_compile(&root, file))
                .join()
                .unwrap()
        })
        .collect();
    assert!(
        fresh[1].contains("let $tt_v0: (string) | (number[]);"),
        "{}",
        fresh[1]
    );
    let mut shared = Vec::new();
    let first = measure(|| shared.push(contextual_compile(&root, 0)));
    let rest = measure(|| {
        for file in 1..files {
            shared.push(contextual_compile(&root, file));
        }
    });
    assert_eq!(shared, fresh);
    assert_eq!(first["contextual projections"], files - 1);
    assert_eq!(rest["contextual projections"], files - 1);
    assert!(first["contextual checker asks"] > 0);
    assert_eq!(rest.get("contextual checker asks"), None);
    std::fs::write(root.join("src/dep.ts"), "export const d: boolean = true;\n").unwrap();
    let mut changed = String::new();
    let after = measure(|| changed = contextual_compile(&root, 0));
    let expected = {
        let root = root.clone();
        std::thread::spawn(move || contextual_compile(&root, 0))
            .join()
            .unwrap()
    };
    assert_eq!(changed, expected);
    assert!(changed.contains("boolean"), "{changed}");
    assert!(after["contextual checker asks"] > 0);
}
