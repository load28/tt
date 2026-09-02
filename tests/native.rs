//! The TypeScript 7 backend, end to end (`ttc --check-types` / `--types`).
//!
//! These tests need the TypeScript the repository installed — the same
//! `node_modules` a consumer project would have, resolved the same way
//! (`src/typescript/toolchain.rs`). Each case's project therefore lives
//! under the repository's `target/`, not in the system temp directory, so
//! the upward walk finds it. They skip silently when the install is not
//! there, exactly as the `tsc`/`node` tests do.
//!
//! Where a toolchain is *supposed* to be there — CI installs one — set
//! `TTC_REQUIRE_TSGO=1` and a missing one fails the suite instead of
//! skipping it. That is the whole of the CI guard: a skipped suite is
//! green in every other way, so something has to make it red.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Whether the repository has a TypeScript for these cases to run against,
/// resolved the way ttc resolves it: `node_modules` from here upwards.
fn toolchain() -> bool {
    if installed() {
        return true;
    }
    // A caller that asked for no skipping gets an error, not a pass.
    assert!(
        !required(),
        "TTC_REQUIRE_TSGO is set but this repository has no TypeScript \
         installed — run `npm ci` at the repository root"
    );
    false
}

/// True when the caller has declared that a toolchain must be present.
fn required() -> bool {
    std::env::var_os("TTC_REQUIRE_TSGO").is_some_and(|v| !v.is_empty() && v != "0")
}

/// The API client of an installed TypeScript, searched for the way
/// `toolchain.rs` searches — a guard that mirrors only part of the
/// compiler's rules reports "no toolchain" where the compiler finds one
/// (TASK-217).
fn installed() -> bool {
    const CLIENTS: [&str; 2] = ["typescript", "@typescript/native-preview"];
    let mut dir = Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    while let Some(current) = dir {
        for client in CLIENTS {
            if current
                .join("node_modules")
                .join(client)
                .join("dist/api/sync/api.js")
                .exists()
            {
                return true;
            }
        }
        dir = current.parent().map(Path::to_path_buf);
    }
    false
}

/// Any resolvable compiler — enough to check.
macro_rules! require_tsgo {
    () => {
        if !toolchain() {
            return;
        }
    };
}

/// A compiler that can also emit declarations. That API arrived in
/// TypeScript 7.1, so a project pinned to 7.0 checks but cannot emit.
macro_rules! require_emit {
    () => {
        if !toolchain() {
            return;
        }
        if !emits_declarations() {
            // Same rule as the toolchain guard: where CI says a toolchain
            // must be there, "it cannot emit" is a stale pin, not a skip.
            assert!(
                !required(),
                "TTC_REQUIRE_TSGO is set but the installed TypeScript has no \
                 declaration emit — pin a 7.1 in the root package.json"
            );
            return;
        }
    };
}

/// Whether the installed API client has the declaration-emit entry point
/// (`host.mjs` checks for the same method before asking for one).
fn emits_declarations() -> bool {
    let mut dir = Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    while let Some(current) = dir {
        for client in ["typescript", "@typescript/native-preview"] {
            let api = current
                .join("node_modules")
                .join(client)
                .join("dist/api/sync/api.js");
            if let Ok(text) = fs::read_to_string(&api) {
                return text.contains("getDeclarationEmit");
            }
        }
        dir = current.parent().map(Path::to_path_buf);
    }
    false
}

mod common;
use common::Workspace;

/// A directory for one case, with its `src/` tree — removed when the case
/// ends, kept when it failed (`tests/common/mod.rs`).
fn tmpdir() -> Workspace {
    Workspace::in_repo_with_subdir("native", "src")
}

fn write(dir: &Path, name: &str, text: &str) {
    fs::write(dir.join(name), text).unwrap();
}

/// A project whose `tsconfig.json` globs `src` — the lowered `.tt` modules
/// have to enter the program through the user's own configuration.
fn project(files: &[(&str, &str)]) -> Workspace {
    let dir = tmpdir();
    write(
        &dir,
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "target": "es2022",
    "module": "preserve",
    "moduleResolution": "bundler",
    "jsx": "preserve",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true
  },
  "include": ["src"]
}
"#,
    );
    for (name, text) in files {
        write(&dir, name, text);
    }
    dir
}

/// Runs ttc in `dir`. Nothing about the toolchain is passed: ttc resolves
/// the project's own TypeScript, which is the whole contract.
fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("ttc runs")
}

#[test]
fn mixed_source_fixture_covers_every_directed_edge_and_typechecks() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = root.join("tests/fixtures/mixed-source-matrix/src");
    let modules = [
        ("plain.ts", "./plain"),
        ("plain-jsx.tsx", "./plain-jsx"),
        ("language.tt", "./language.tt"),
        ("language-jsx.ttx", "./language-jsx.ttx"),
    ];
    let mut edges = 0;
    for (file, own_specifier) in modules {
        let source = fs::read_to_string(fixture.join(file)).expect("matrix fixture source");
        for (_, specifier) in modules {
            if specifier == own_specifier {
                continue;
            }
            assert!(
                source.contains(&format!("from \"{specifier}\"")),
                "{file} is missing its directed edge to {specifier}"
            );
            edges += 1;
        }
    }
    assert_eq!(edges, 12);
    let tt_source = fs::read_to_string(fixture.join("language.tt")).expect("tt fixture source");
    assert!(
        tt_source.contains("FromTtx(value) => readTtx(value)"),
        ".tt must consume the imported .ttx payload, not only its type"
    );

    require_tsgo!();
    let out = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args([
            "--check-types",
            "--project",
            "tests/fixtures/mixed-source-matrix/tsconfig.json",
            "tests/fixtures/mixed-source-matrix/src",
        ])
        .current_dir(root)
        .output()
        .expect("ttc runs");
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    require_emit!();
    let dir = project(&[
        (
            "src/plain.ts",
            include_str!("fixtures/mixed-source-matrix/src/plain.ts"),
        ),
        (
            "src/plain-jsx.tsx",
            include_str!("fixtures/mixed-source-matrix/src/plain-jsx.tsx"),
        ),
        (
            "src/language.tt",
            include_str!("fixtures/mixed-source-matrix/src/language.tt"),
        ),
        (
            "src/language-jsx.ttx",
            include_str!("fixtures/mixed-source-matrix/src/language-jsx.ttx"),
        ),
    ]);
    let out_dir = dir.join("out");
    let out = run(&dir, &["--types", "src", "-o", out_dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    for declaration in ["language.tt.d.ts", "language-jsx.ttx.d.ts"] {
        assert!(
            out_dir.join(declaration).is_file(),
            "missing mixed-source sidecar {declaration}"
        );
    }
}

/// Runs `ttc --check-types src` in `dir`, returning its diagnostics.
/// The one rendered diagnostic containing `needle`.
///
/// A diagnostic is a block now, not a line: the rule and message open it,
/// `-->` places it, and the snippet and `= help:` lines follow. Asserting a
/// message and a location separately would let two different diagnostics
/// satisfy one test, so the block keeps them paired.
fn block<'a>(out: &'a str, needle: &str) -> &'a str {
    let mut starts: Vec<usize> = out
        .match_indices('\n')
        .map(|(at, _)| at + 1)
        .filter(|at| out[*at..].starts_with("error") || out[*at..].starts_with("warning"))
        .collect();
    starts.insert(0, 0);
    starts
        .iter()
        .enumerate()
        .map(|(index, start)| &out[*start..starts.get(index + 1).copied().unwrap_or(out.len())])
        .find(|block| block.contains(needle))
        .unwrap_or_else(|| panic!("no diagnostic mentioning {needle:?}:\n{out}"))
}

fn check(dir: &Path) -> String {
    let out = run(dir, &["--check-types", "src"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("no TypeScript compiler found"),
        "the toolchain guard passed but ttc disagreed: {stderr}"
    );
    // Diagnostics are diagnostics: stderr, in ttc's own form. stdout is
    // reserved for the modes that pipe.
    assert!(
        out.stdout.is_empty(),
        "a checking mode wrote to stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    stderr.into_owned()
}

fn typed_server(dir: &Path, relative: &str, source: &str) -> serde_json::Value {
    use std::io::Write;

    let file = dir.join(relative).canonicalize().unwrap();
    let request = serde_json::json!({
        "id": 1,
        "method": "typedCheck",
        "params": {
            "path": file,
            "text": source,
            "includeTypes": true,
        },
    });
    let mut command = Command::new(env!("CARGO_BIN_EXE_ttc"));
    command
        .arg("--server")
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());
    let mut child = command.spawn().expect("server starts");
    writeln!(child.stdin.as_mut().unwrap(), "{request}").unwrap();
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("server answers");
    serde_json::from_slice(String::from_utf8_lossy(&output.stdout).trim().as_bytes())
        .expect("one JSON response")
}

fn source_slice<'a>(source: &'a str, diagnostic: &serde_json::Value) -> &'a str {
    fn offset(source: &str, line: usize, col: usize) -> usize {
        let line_start = source
            .split_inclusive('\n')
            .take(line.saturating_sub(1))
            .map(str::len)
            .sum::<usize>();
        line_start
            + source[line_start..]
                .char_indices()
                .nth(col.saturating_sub(1))
                .map_or(source[line_start..].len(), |(at, _)| at)
    }
    let start = offset(
        source,
        diagnostic["line"].as_u64().unwrap() as usize,
        diagnostic["col"].as_u64().unwrap() as usize,
    );
    let end = offset(
        source,
        diagnostic["endLine"].as_u64().unwrap() as usize,
        diagnostic["endCol"].as_u64().unwrap() as usize,
    );
    &source[start..end]
}

#[test]
fn watching_re_checks_against_the_compiler_it_already_started() {
    require_tsgo!();
    let dir = project(&[(
        "src/color.tt",
        "export variant Color { Red(), Green() }\n\
         export function name(c: Color): string {\n\
         \x20 return match (c) { Red => \"red\", Green => \"green\" };\n\
         }\n",
    )]);

    let mut command = Command::new(env!("CARGO_BIN_EXE_ttc"));
    command
        .args(["--check-types", "src", "-w"])
        .current_dir(&dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("ttc runs");

    // Let the first pass finish, then add a case with no arm: the watch has
    // to see the edit through the compiler it is already holding.
    std::thread::sleep(std::time::Duration::from_secs(6));
    write(
        &dir,
        "src/color.tt",
        "export variant Color { Red(), Green(), Blue() }\n\
         export function name(c: Color): string {\n\
         \x20 return match (c) { Red => \"red\", Green => \"green\" };\n\
         }\n",
    );
    std::thread::sleep(std::time::Duration::from_secs(5));
    let _ = child.kill();
    let out = child.wait_with_output().expect("ttc exits");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("missing \"Blue\""),
        "the second pass saw the edit: {stdout}{stderr}"
    );
    assert_eq!(
        stderr.matches("— watching").count(),
        2,
        "one pass at startup and one for the edit: {stderr}"
    );
}

#[test]
fn a_hand_written_ts_file_imports_an_tt_file_by_the_specifier_it_writes() {
    require_tsgo!();
    // `"./shape.tt"` is what a user writes, and it needs no configuration:
    // the lowered module is served at `shape.tt.ts`, which is what ordinary
    // TypeScript resolution finds for that specifier. The project's
    // tsconfig here sets no tt-specific option at all.
    let dir = project(&[
        (
            "src/shape.tt",
            "export variant Shape { Circle(radius: number), Point }\n",
        ),
        (
            "src/use.ts",
            "import { Shape } from \"./shape.tt\";\n\
             export const s: Shape = Shape.Point;\n\
             export const bad: number = Shape.Point;\n",
        ),
    ]);
    let out = check(&dir);
    // The import resolved — the only error is the deliberate one, reported
    // in the hand-written file at TypeScript's own coordinates.
    // Positionless: the checker's answer is about the hand-written file as
    // a whole, so the block names the file and quotes nothing.
    assert!(
        block(&out, "type mismatch: expected `number`").contains("--> src/use.ts"),
        "the .ts file's own error, in one project with the .tt: {out}"
    );
    assert!(
        !out.contains("2307") && !out.contains("Cannot find module"),
        "and nothing failed to resolve: {out}"
    );
}

#[test]
fn files_outside_tsconfig_do_not_receive_typed_queries() {
    require_tsgo!();
    let source = "import { importedMutation } from \"../shared/reachable.tt\";\n\
        export function mutate(): number {\n\
        \x20 val const values = new Map<string, number>();\n\
        \x20 values.set(\"answer\", 42);\n\
        \x20 return values.size + importedMutation();\n\
        }\n";
    let dir = project(&[("src/main.tt", source)]);
    fs::create_dir_all(dir.join("shared")).unwrap();
    write(
        &dir,
        "shared/reachable.tt",
        "export function importedMutation(): number {\n\
         \x20 val const values = new Set<number>();\n\
         \x20 values.add(1);\n\
         \x20 return values.size;\n\
         }\n",
    );
    fs::create_dir_all(dir.join("examples")).unwrap();
    write(
        &dir,
        "examples/demo.tt",
        "export function shout(name: string): string {\n\
         \x20 val const text = name.trim();\n\
         \x20 return `${text}!`;\n\
         }\n",
    );

    let out = run(&dir, &["--check-types", "src"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(1),
        "the val diagnostic is a code error: {stderr}"
    );
    assert!(stderr.contains("error[val-mutation]"), "{stderr}");
    assert!(stderr.contains("--> src/main.tt:4:3"), "{stderr}");
    assert!(stderr.contains("--> shared/reachable.tt:3:3"), "{stderr}");
    assert!(
        !stderr.contains("backend failed") && !stderr.contains("demo.tt.ts"),
        "{stderr}"
    );

    let answer = typed_server(&dir, "src/main.tt", source);
    assert!(answer.get("error").is_none(), "{answer}");
    assert!(answer["result"]["backendError"].is_null(), "{answer}");
    assert!(
        answer["result"]["diagnostics"]
            .as_array()
            .is_some_and(|diagnostics| diagnostics.iter().any(|d| d["code"] == "val-mutation")),
        "{answer}"
    );
}

#[test]
fn a_malformed_file_outside_tsconfig_does_not_enter_typed_diagnostics() {
    require_tsgo!();
    let source = "export const answer: number = 42;\n";
    let dir = project(&[("src/main.tt", source)]);
    fs::create_dir_all(dir.join("examples")).unwrap();
    write(
        &dir,
        "examples/demo.tt",
        "export function demo(): number {\n return result { value; };\n}\n",
    );

    let out = run(&dir, &["--check-types", "src"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(!stderr.contains("examples/demo.tt"), "{stderr}");

    let answer = typed_server(&dir, "src/main.tt", source);
    assert!(
        answer["result"]["diagnostics"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "{answer}"
    );
}

#[test]
fn a_malformed_included_file_still_fails_single_file_typed_checks() {
    require_tsgo!();
    let source = "export const answer: number = 42;\n";
    let dir = project(&[
        ("src/main.tt", source),
        (
            "src/orphan.tt",
            "export function demo(): number {\n return result { value; };\n}\n",
        ),
    ]);

    let out = run(&dir, &["--check-types", "src/main.tt"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("error["), "{stderr}");
    assert!(stderr.contains("--> src/orphan.tt"), "{stderr}");

    let answer = typed_server(&dir, "src/main.tt", source);
    assert!(answer.get("error").is_none(), "{answer}");
    assert!(answer["result"]["backendError"].is_null(), "{answer}");
    assert!(
        answer["result"]["diagnostics"]
            .as_array()
            .is_some_and(|diagnostics| diagnostics.iter().any(|diagnostic| {
                diagnostic["path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("src/orphan.tt"))
            })),
        "{answer}"
    );
}

#[test]
fn an_import_reached_blocked_file_outside_tsconfig_is_reported() {
    require_tsgo!();
    let source = "import { demo } from \"../shared/broken.tt\";\n\
                  export const answer = demo();\n";
    let dir = project(&[("src/main.tt", source)]);
    fs::create_dir_all(dir.join("shared")).unwrap();
    write(
        &dir,
        "shared/broken.tt",
        "export function demo(): number {\n return result { value; };\n}\n",
    );

    let out = run(&dir, &["--check-types", "src/main.tt"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("error["), "{stderr}");
    assert!(stderr.contains("--> shared/broken.tt"), "{stderr}");
}

#[test]
fn a_backend_contract_failure_uses_cli_ice_and_server_backend_error_contracts() {
    require_tsgo!();
    let source = "val const values = new Map<string, number>();\nvalues.set(\"a\", 1);\n";
    let dir = project(&[("src/main.tt", source)]);

    let out = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["--check-types", "src"])
        .env("TTC_TYPESCRIPT_BACKEND_FAIL_FOR_TEST", "1")
        .current_dir(&dir)
        .output()
        .expect("ttc runs");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(101), "{stderr}");
    assert!(
        stderr.starts_with("error: internal compiler error:"),
        "{stderr}"
    );
    assert!(
        stderr.contains("injected TypeScript backend contract failure"),
        "{stderr}"
    );
    assert!(stderr.contains("github.com/load28/tt/issues"), "{stderr}");
    assert!(
        !stderr.contains("at handle") && !stderr.contains("host.mjs:"),
        "{stderr}"
    );

    use std::io::Write;
    let file = dir.join("src/main.tt").canonicalize().unwrap();
    let request = serde_json::json!({
        "id": 1,
        "method": "typedCheck",
        "params": { "path": file, "text": source, "includeTypes": true },
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .arg("--server")
        .env("TTC_TYPESCRIPT_BACKEND_FAIL_FOR_TEST", "1")
        .current_dir(&dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("server starts");
    writeln!(child.stdin.as_mut().unwrap(), "{request}").unwrap();
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("server answers");
    let answer: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(answer.get("error").is_none(), "{answer}");
    assert_eq!(
        answer["result"]["backendError"]["kind"], "internal",
        "{answer}"
    );
    assert!(
        answer["result"]["backendError"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("injected TypeScript backend contract failure")),
        "{answer}"
    );
}

#[test]
fn a_hand_written_tsx_file_imports_an_ttx_file_by_its_specifier() {
    require_tsgo!();
    let dir = project(&[
        (
            "src/view.ttx",
            "export variant State { Ready(value: string), Empty }\n\
             export const render = (state: State) => <main>{match (state) {\n\
             Ready(value) => <b>{value}</b>, Empty => null\n\
             }}</main>;\n",
        ),
        (
            "src/use.tsx",
            "import { State, render } from \"./view.ttx\";\n\
             declare global { namespace JSX { interface IntrinsicElements { main: {}; b: {}; } } }\n\
             export const value = render(State.Ready(\"ok\"));\n",
        ),
    ]);
    let out = check(&dir);
    assert!(
        !out.contains("2307") && !out.contains("Cannot find module"),
        "{out}"
    );
    assert!(!out.contains("view.ttx"), "{out}");
}

#[test]
fn naming_one_file_still_compiles_against_the_whole_project() {
    require_emit!();
    let dir = project(&[
        (
            "src/token.tt",
            "export variant Token { Num(value: number), Eof }\n",
        ),
        (
            "src/parse.tt",
            "import { Token } from \"./token.tt\";\n\
             export function width(t: Token): number {\n\
             \x20 return match (t) { Num(value) => value, Eof => 0 };\n\
             }\n",
        ),
    ]);
    let out_dir = dir.join("out");
    let out = run(
        &dir,
        &["--types", "src/parse.tt", "-o", out_dir.to_str().unwrap()],
    );
    // `./token.tt` was never named, but it is part of the project, so it is
    // part of the graph — otherwise this would be TS2307.
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(
        out_dir.join("parse.tt.d.ts").is_file(),
        "the named input is written"
    );
    assert!(
        !out_dir.join("token.tt.d.ts").exists(),
        "what was not named is in the graph, not in the output"
    );
}

include!("native/cases_01.rs");
include!("native/cases_02.rs");
include!("native/cases_03.rs");
