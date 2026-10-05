//! A materialization scoped to one file's reference closure settles that
//! file exactly as a whole-project materialization does, cold and from the
//! per-module cache, before and after an edit.

use std::path::{Path, PathBuf};

use super::{Engine, Project, ProjectOptions};

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

/// The units of a multi-file case: `// @filename: name` sections.
fn units(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in text.split_inclusive('\n') {
        if let Some(name) = line.trim().strip_prefix("// @filename:") {
            out.push((name.trim().to_string(), String::new()));
        } else if let Some((_, content)) = out.last_mut() {
            content.push_str(line);
        }
    }
    out
}

fn cases() -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "tt" || e == "ttx") {
                let text = std::fs::read_to_string(&path).unwrap();
                let units = units(&text);
                let tt = units
                    .iter()
                    .filter(|(name, _)| name.ends_with(".tt") || name.ends_with(".ttx"))
                    .count();
                let lowered = units.iter().any(|(_, content)| {
                    content.contains("match") || content.contains("|>") || content.contains("try ")
                });
                if tt >= 1
                    && lowered
                    && !text.contains("@expectErrors")
                    && (tt >= 2 || found.len() % 7 == 0)
                {
                    found.push(path);
                }
            }
        }
    }
    found.sort();
    found
}

fn emits(
    project: &mut Project,
    files: &[PathBuf],
    target: Option<&Path>,
) -> Vec<(PathBuf, String)> {
    let snapshot = project.update_scoped(files, target).unwrap();
    let mut emits: Vec<_> = snapshot
        .files()
        .iter()
        .map(|file| (file.source_path.clone(), file.emit.code.clone()))
        .collect();
    emits.sort();
    emits
}

fn emit_of(emits: &[(PathBuf, String)], path: &Path) -> Option<String> {
    emits
        .iter()
        .find(|(file, _)| file == path)
        .map(|(_, code)| code.clone())
}

#[test]
fn a_scoped_materialization_settles_its_file_as_the_whole_project_does() {
    if !toolchain_present() {
        return;
    }
    let mut cases: Vec<(String, String)> = cases()
        .into_iter()
        .take(80)
        .map(|path| {
            (
                path.display().to_string(),
                std::fs::read_to_string(path).unwrap(),
            )
        })
        .collect();
    cases.push(("synthetic chain".into(), chain()));
    cases.push(("global declaration".into(), global_declaration()));
    let mut compared = 0;
    let mut annotated = 0;
    for (case, text) in &cases {
        let workspace = crate::test_workspace::Workspace::in_repo("scoped-materialization");
        let root = workspace.path().to_path_buf();
        for (name, content) in units(text) {
            let path = root.join(&name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, content).unwrap();
        }
        let engine = Engine::new(None);
        let open = || {
            engine
                .open_project(
                    &[root.to_string_lossy().into_owned()],
                    &ProjectOptions::default(),
                )
                .unwrap()
        };
        let mut whole = open();
        let files = whole.initial_files();
        let targets: Vec<PathBuf> = files
            .iter()
            .filter(|file| file.extension().is_some_and(|e| e == "tt" || e == "ttx"))
            .cloned()
            .collect();
        let Ok(snapshot) = whole.update(&files) else {
            continue;
        };
        drop(snapshot);
        let reference = emits(&mut whole, &files, None);
        let mut scoped = open();
        for round in 0..2 {
            if round == 1 {
                // An edit that changes no type: every cached module of the
                // edited file's closure is settled again.
                let first = &targets[0];
                let edited = format!("{}\n// edited\n", std::fs::read_to_string(first).unwrap());
                whole.open_document(first.clone(), edited.clone());
                scoped.open_document(first.clone(), edited);
            }
            let reference = if round == 0 {
                reference.clone()
            } else {
                emits(&mut whole, &files, None)
            };
            for target in &targets {
                let answer = emits(&mut scoped, &files, Some(target));
                assert_eq!(
                    emit_of(&answer, target),
                    emit_of(&reference, target),
                    "{}: {} (round {round})",
                    case,
                    target.display()
                );
                compared += 1;
                if emit_of(&answer, target).is_some_and(|code| {
                    code.contains("let $tt_v")
                        && code.lines().any(|line| {
                            line.trim_start().starts_with("let $tt_v") && line.contains(':')
                        })
                }) {
                    annotated += 1;
                }
            }
        }
    }
    eprintln!(
        "scoped materialization: {compared} comparisons, {annotated} with settled storage annotations"
    );
    assert!(compared > 0);
    assert!(annotated > 0);
}

/// Modules whose exported types are inferred through generated storage that
/// contextual materialization settles, each consumed in a contextual position
/// by the next.
fn chain() -> String {
    let mut text = String::from(
        "// @filename: tsconfig.json\n{\"compilerOptions\":{\"strict\":true,\"module\":\"esnext\",\"moduleResolution\":\"bundler\",\"allowImportingTsExtensions\":true,\"noEmit\":true}}\n",
    );
    for i in 0..5 {
        text.push_str(&format!("// @filename: m{i}.tt\n"));
        if i > 0 {
            text.push_str(&format!(
                "import {{ make{p}, pick{p} }} from \"./m{p}.tt\";\n",
                p = i - 1
            ));
        }
        text.push_str(&format!(
            "export variant V{i} {{ A(n: number), B(s: string), C }}\n"
        ));
        text.push_str(&format!(
            "export const make{i} = (flag: boolean) => match (flag) {{ true => {{ id: {i}, run: (x: number) => x + 1 }}, false => {{ id: {i}, run: (x: number) => x * 2 }} }};\n"
        ));
        text.push_str(&format!(
            "export function pick{i}(v: V{i}) {{ return match (v) {{ A(n) => [n], B(s) => [s.length], C => [] }}; }}\n"
        ));
        if i > 0 {
            text.push_str(&format!(
                "export const use{i}: {{ id: number; run: (x: number) => number }}[] = [make{p}(true), match (pick{p}(V{p}.C).length) {{ 0 => make{p}(false), _ => make{p}(true) }}];\n",
                p = i - 1
            ));
        }
    }
    text
}

/// A declaration file that affects the global scope names a type of a module
/// no other module imports; a module that uses the global type depends on
/// that module through it.
fn global_declaration() -> String {
    let mut text = String::from(
        "// @filename: tsconfig.json\n{\"compilerOptions\":{\"strict\":true,\"module\":\"esnext\",\"moduleResolution\":\"bundler\",\"allowImportingTsExtensions\":true,\"noEmit\":true}}\n",
    );
    text.push_str(
        "// @filename: globals.d.ts\ntype Shared = ReturnType<typeof import(\"./g0.tt\").make>;\n",
    );
    text.push_str("// @filename: g0.tt\nexport const make = (flag: boolean) => match (flag) { true => { id: 0, run: (x: number) => x + 1 }, false => { id: 1, run: (x: number) => x * 2 } };\n");
    text.push_str("// @filename: g1.tt\nexport const use: Shared[] = [match (1 > 0) { true => { id: 2, run: (x) => x }, false => { id: 3, run: (x) => -x } }];\n");
    text
}
