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

#[test]
fn a_declaration_carries_a_map_back_to_the_tt_source() {
    require_emit!();
    let dir = project(&[(
        "src/token.tt",
        "export variant Token { Num(value: number), Eof }\n\
         export function width(t: Token): number {\n\
         \x20 return match (t) { Num(value) => value, Eof => 0 };\n\
         }\n",
    )]);
    let out_dir = dir.join("out");
    let out = run(&dir, &["--types", "src", "-o", out_dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The sidecar takes the name a `"./token.tt"` specifier resolves to
    // when no compiler is running — which is what makes it a sidecar.
    let declarations = fs::read_to_string(out_dir.join("token.tt.d.ts")).expect("the sidecar");
    assert!(
        declarations.contains("//# sourceMappingURL=token.tt.d.ts.map"),
        "and points at its map: {declarations}"
    );
    let map = fs::read_to_string(out_dir.join("token.tt.d.ts.map")).expect("the map");
    assert!(
        map.contains("token.tt\"") && map.contains("\"mappings\""),
        "whose sources is the .tt file, so go-to-definition lands there: {map}"
    );
}

#[test]
fn declarations_are_emitted_by_the_compiler_itself() {
    require_emit!();
    let dir = project(&[(
        "src/shape.tt",
        "export variant Shape { Circle(radius: number), Point }\n\
         export function area(s: Shape): number {\n\
         \x20 return match (s) { Circle(radius) => radius, Point => 0 };\n\
         }\n",
    )]);
    let out_dir = dir.join("out");
    let out = run(&dir, &["--types", "src", "-o", out_dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let declaration = fs::read_to_string(out_dir.join("shape.tt.d.ts")).expect("a .d.ts");
    // ttc writes no declaration syntax of its own: this is what the compiler
    // emits for the module ttc lowered, exactly as for a hand-written one.
    assert!(
        declaration.contains("kind: \"Circle\"") && declaration.contains("radius: number"),
        "the variant's union type: {declaration}"
    );
    assert!(
        declaration.contains("export declare function area(s: Shape): number;"),
        "the function's signature: {declaration}"
    );
}

#[test]
fn the_standard_library_enters_the_graph_as_a_module_of_the_project() {
    require_emit!();
    let dir = project(&[(
        "src/parse.tt",
        "import type { TResult } from \"@tt/std\";\n\
         import * as Result from \"@tt/std/result\";\n\
         export function parse(text: string): TResult<number, string> {\n\
         \x20 const n = Number(text);\n\
         \x20 return Number.isNaN(n) ? Result.Err(\"not a number\") : Result.Ok(n);\n\
         }\n",
    )]);
    let out_dir = dir.join("out");
    let out = run(&dir, &["--types", "src", "-o", out_dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "@tt/std has to resolve, and its types have to check: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    // The library is a module of the project, resolved by ordinary node
    // resolution — so the specifier stays bare, in the source and in the
    // declaration alike, and no `paths` entry is involved in this compile.
    let declaration = fs::read_to_string(out_dir.join("parse.tt.d.ts")).expect("a .d.ts");
    assert!(
        declaration.contains("from \"@tt/std\""),
        "the declaration keeps the specifier the user wrote: {declaration}"
    );
}

#[test]
fn the_pipeline_runtime_enters_the_typed_project_once() {
    require_emit!();
    let dir = project(&[
        (
            "src/a.tt",
            "declare const input: number;\ndeclare const step: (value: number) => string;\nexport const value = input |> step;\n",
        ),
        (
            "src/b.tt",
            "declare const input: number;\ndeclare const step: (value: number) => string;\nexport const value = input |> step;\n",
        ),
    ]);
    let out = run(&dir, &["--check-types", "src"]);
    assert!(
        out.status.success(),
        "@tt/runtime has to resolve once for every pipeline module: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn a_diagnostic_on_generated_code_is_restated_in_tts_words() {
    require_tsgo!();
    // A plain TypeScript enum is not a tt variant, so matching on one lowers
    // to a `.kind` switch over a value that has no `kind`. The error is
    // real and it is the user's, but the text TypeScript points at is code
    // ttc wrote — so ttc says what the construct meant, at the construct
    // (TASK-104), with TypeScript's own sentence alongside for checking.
    let dir = project(&[(
        "src/ts_enum.tt",
        "export enum Plain { A, B }\n\
         export function f(p: Plain): number {\n\
         \x20 return match (p) { A => 1 };\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("--> src/ts_enum.tt:3:10"),
        "reported at the `match` keyword in the .tt file: {out}"
    );
    assert!(
        out.contains("match on a tag pattern needs a value with a `kind` discriminant"),
        "in tt's words: {out}"
    );
    assert!(
        out.contains("ts2339: Property 'kind' does not exist on type 'Plain'."),
        "with the original alongside: {out}"
    );
}

#[test]
fn a_restated_diagnostic_calls_a_case_by_its_declared_name() {
    require_tsgo!();
    // TypeScript has no word for a tt case, so a narrowed one prints as
    // the object type it lowers to. tt names both sides from declarations.
    let dir = project(&[(
        "src/named.tt",
        "import type { TResult } from \"@tt/std\";\n\
         import * as Result from \"@tt/std/result\";\n\
         variant Wire { OutOfRange(value: number), Missing }\n\
         variant ParseError { NotANumber(text: string) }\n\
         function inner(w: Wire) {\n\
         \x20 if (w.kind === \"OutOfRange\") { return Result.Err(w); }\n\
         \x20 return Result.Ok(1);\n\
         }\n\
         export function outer(w: Wire): TResult<number, ParseError> {\n\
         \x20 const n = try inner(w);\n\
         \x20 return Result.Ok(n);\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("type mismatch: expected `ParseError`, found `Wire.OutOfRange`")
            && out.contains("required type: `TResult<number, ParseError>`"),
        "the case and surrounding obligation use tt declaration names: {out}"
    );
    assert!(
        !out.contains("{ kind: \"OutOfRange\"; value: number; }") && !out.contains("in tt's names"),
        "the lowered representation and duplicate prose stay hidden: {out}"
    );
}

#[test]
fn assignability_diagnostics_report_the_minimal_type_difference() {
    require_tsgo!();
    let dir = project(&[(
        "src/mismatch.tt",
        "import type { TResult } from \"@tt/std\";\n\
         import * as Result from \"@tt/std/result\";\n\
         variant InputError { Empty, NotANumber(raw: string) }\n\
         variant RangeError { TooLarge(value: number, max: number) }\n\
         export function port(value: number): TResult<number, InputError> {\n\
         \x20 return value > 65535\n\
         \x20   ? Result.Err(RangeError.TooLarge(value, 65535))\n\
         \x20   : Result.Err(InputError.Empty);\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("type mismatch: expected `InputError`, found `RangeError`"),
        "minimal incompatible leaf: {out}"
    );
    assert!(
        out.contains("required type: `TResult<number, InputError>`"),
        "the surrounding obligation remains visible: {out}"
    );
    assert!(
        !out.contains("Property 'raw' is missing") && !out.contains("in tt's names"),
        "the nested checker prose is not duplicated: {out}"
    );
}

#[test]
fn structured_type_mismatches_are_not_tied_to_an_tt_construct() {
    require_tsgo!();
    let dir = project(&[(
        "src/plain.tt",
        "const annotated: string = 1;\n\
         function takesString(value: string): void {}\n\
         takesString(2);\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("type mismatch: expected `string`, found `1`")
            && out.contains("type mismatch: expected `string`, found `2`"),
        "annotation and call argument use the same relation: {out}"
    );
}

#[test]
fn one_structured_cause_replaces_try_lowering_consequences() {
    require_tsgo!();
    let dir = project(&[(
        "src/try.tt",
        "import type { TResult } from \"@tt/std\";\n\
         import * as Result from \"@tt/std/result\";\n\
         const a = () => Result.Err(10);\n\
         function test(): TResult<string, string> {\n\
         \x20 const value = try a();\n\
         \x20 return value;\n\
         }\n",
    )]);
    let out = check(&dir);
    assert_eq!(
        out.matches("type mismatch:").count(),
        1,
        "one failed type obligation is one diagnostic: {out}"
    );
    assert!(
        out.contains("expected `string`, found `number`")
            && out.contains("required type: `TResult<string, string>`"),
        "the checker-proven incompatible types are reported: {out}"
    );
    assert!(
        !out.contains("`try` needs a Result") && !out.contains("no overlap"),
        "property and comparison consequences from lowering are suppressed: {out}"
    );
}

#[test]
fn a_precise_tt_error_owns_an_overlapping_type_consequence() {
    require_tsgo!();
    let dir = project(&[(
        "src/field.tt",
        "variant Shape { Circle(radius: number), Point }\n\
         export const radiusOf = (shape: Shape): number =>\n\
         \x20 match (shape) { Circle(radiuz) => radiuz, Point => 0 };\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("case `Circle` has no field `radiuz`") && !out.contains("type mismatch:"),
        "the direct tt cause replaces its broader checker consequence: {out}"
    );
}

#[test]
fn proven_statement_and_tuple_errors_own_only_their_checker_cascades() {
    require_tsgo!();
    let source = "variant PaymentMethod { Card(brand: string, last4: string), Cash }\n\
        variant Fulfillment { Pending, Picked, Cancelled }\n\
        export function card(method: PaymentMethod): string {\n\
        \x20 const Card(brand, last4) = method else { console.log(\"other\"); };\n\
        \x20 return brand + last4;\n\
        }\n\
        export function label(state: Fulfillment, method: PaymentMethod): string {\n\
        \x20 return match (state, method) {\n\
        \x20   (Picked, Card) => \"picked card\",\n\
        \x20   (Picked, Cash) => \"picked cash\",\n\
        \x20   (Picked) => \"picked\",\n\
        \x20   _ => \"other\",\n\
        \x20 };\n\
        }\n\
        const independent: string = 1;\n";
    let dir = project(&[("src/cascades.tt", source)]);

    let out = check(&dir);
    assert!(out.contains("error[let-else-not-diverging]"), "{out}");
    assert!(out.contains("error[match-tuple-arity]"), "{out}");
    assert!(out.contains("tuple pattern has 1 element"), "{out}");
    assert!(
        !out.contains("error[ts2339]") && !out.contains("error[ts2367]"),
        "checker consequences owned by the invalid constructs remain: {out}"
    );
    assert!(
        out.contains("type mismatch: expected `string`, found `1`"),
        "the independent source error must remain: {out}"
    );

    let answer = typed_server(&dir, "src/cascades.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == "let-else-not-diverging"),
        "{answer}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == "match-tuple-arity"),
        "{answer}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| { diagnostic["code"] != "ts2339" && diagnostic["code"] != "ts2367" })
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == "ts2322"),
        "{answer}"
    );
}

#[test]
fn an_imported_field_error_is_identical_on_typed_cli_and_server_paths() {
    require_tsgo!();
    let source = "import { PaymentMethod } from \"./domain.tt\";\n\
        export function brand(method: PaymentMethod): string {\n\
        \x20 return match (method) { Card(brnad) => brnad, _ => \"n/a\" };\n\
        }\n";
    let dir = project(&[
        (
            "src/domain.tt",
            "export variant PaymentMethod { Card(brand: string, last4: string) }\n",
        ),
        ("src/payment.tt", source),
    ]);

    let out = check(&dir);
    assert!(
        out.contains("case `Card` has no field `brnad`")
            && out.contains("a field with a similar name exists: `brand`")
            && !out.contains("type mismatch:"),
        "the typed CLI reports the source cause only: {out}"
    );

    let answer = typed_server(&dir, "src/payment.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    let field = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == "unknown-field")
        .unwrap_or_else(|| panic!("missing imported field diagnostic: {answer}"));
    assert_eq!(source_slice(source, field), "brnad");
    assert_eq!(field["suggestions"][0]["edit"]["replacement"], "brand");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic["code"] != "ts2339"),
        "the source error owns its generated consequence: {answer}"
    );
}

#[test]
fn a_nested_imported_field_error_uses_checker_evidence_and_source_span() {
    require_tsgo!();
    let source = "import type { TResult } from \"@tt/std\";\n\
        import { PaymentMethod } from \"./domain.tt\";\n\
        export function brand(r: TResult<PaymentMethod, string>): string {\n\
        \x20 return match (r) {\n\
        \x20   Ok(value: Card(brnd)) => brnd,\n\
        \x20   Ok(value) => \"other\",\n\
        \x20   Err(error) => \"error\",\n\
        \x20 };\n\
        }\n";
    let dir = project(&[
        (
            "src/domain.tt",
            "export variant PaymentMethod { Card(brand: string), Cash }\n",
        ),
        ("src/nested.tt", source),
    ]);

    let out = check(&dir);
    assert!(
        out.contains("error[ts2339]: Property 'brnd' does not exist"),
        "{out}"
    );
    assert!(out.contains("--> src/nested.tt:5:20"), "{out}");
    assert!(
        !out.contains("expected `{ brnd: any; }`") && !out.contains("type mismatch:"),
        "the direct property fact replaces the generated structural mismatch: {out}"
    );

    let answer = typed_server(&dir, "src/nested.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    let field = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == "ts2339")
        .unwrap_or_else(|| panic!("missing nested field diagnostic: {answer}"));
    assert_eq!(source_slice(source, field), "brnd");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic["code"] != "unknown-field"),
        "{answer}"
    );
}

#[test]
fn deep_expression_try_is_accepted_on_typed_cli_and_server_paths() {
    require_tsgo!();
    let source = "import type { TResult } from \"@tt/std\";\n\
        import * as Result from \"@tt/std/result\";\n\
        declare function total(): TResult<number, string>;\n\
        export function amount(): TResult<number, string> {\n\
        \x20 return Result.Ok(Math.round(try total() * 1.1));\n\
        }\n";
    let dir = project(&[("src/deep-try.tt", source)]);

    let out = check(&dir);
    assert!(!out.contains("error["), "{out}");

    let answer = typed_server(&dir, "src/deep-try.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    assert!(diagnostics.is_empty(), "{answer}");
}

#[test]
fn a_pipeline_mismatch_names_the_step_that_rejects_the_value() {
    require_tsgo!();
    // The mismatch is at the second boundary, where the checker blames the
    // accumulated helper call — compiler glue. The per-step anchor re-homes
    // it onto the step that rejected the value instead of underlining the
    // whole pipeline (TASK-263).
    let dir = project(&[(
        "src/pipe.tt",
        "const inc = (n: number): number => n + 1;\n\
         const shout = (s: string): string => s.toUpperCase();\n\
         const a = 1 |> inc |> shout;\n",
    )]);
    let out = check(&dir);
    let step = block(&out, "ts2345");
    assert!(
        step.contains("this pipeline step expects `string`, but receives `number`"),
        "the boundary is said in pipeline vocabulary: {out}"
    );
    assert!(
        step.contains("--> src/pipe.tt:3:23"),
        "reported at the rejecting step, not over the whole pipeline: {out}"
    );
}

#[test]
fn each_failing_pipeline_boundary_gets_its_own_step_diagnostic() {
    require_tsgo!();
    let dir = project(&[(
        "src/chain.tt",
        "const inc = (n: number): number => n + 1;\n\
         const shout = (s: string): string => s.toUpperCase();\n\
         const g = 10\n\
         \x20 |> inc\n\
         \x20 |> shout\n\
         \x20 |> inc;\n",
    )]);
    let out = check(&dir);
    let first = block(&out, "src/chain.tt:5:6");
    assert!(
        first.contains("this pipeline step expects `string`, but receives `number`"),
        "the step rejecting the number is the first boundary: {out}"
    );
    let second = block(&out, "src/chain.tt:6:6");
    assert!(
        second.contains("this pipeline step expects `number`, but receives `string`"),
        "the step after the failed one reports its own boundary: {out}"
    );
}

#[test]
fn a_flow_mismatch_names_the_composed_step_and_the_boundary_types() {
    require_tsgo!();
    // A `flow` boundary mismatches as two function types; the diagnostic
    // descends to the value types of the boundary and keeps the complete
    // function obligation as context.
    let dir = project(&[(
        "src/flow.tt",
        "const inc = (n: number): number => n + 1;\n\
         const shout = (s: string): string => s.toUpperCase();\n\
         const label = flow |> inc |> inc |> shout;\n",
    )]);
    let out = check(&dir);
    let step = block(&out, "ts2345");
    assert!(
        step.contains("this pipeline step expects `string`, but receives `number`"),
        "the boundary's value types, not the whole function types: {out}"
    );
    assert!(
        step.contains("required type: `(n: number) => string`"),
        "the complete obligation remains visible: {out}"
    );
    assert!(
        step.contains("--> src/flow.tt:3:37"),
        "reported at the composed step that rejects the chain: {out}"
    );
}

#[test]
fn a_curried_combinator_chain_blames_the_step_with_the_wrong_argument() {
    require_tsgo!();
    // The report's original shape: std combinator steps whose error used to
    // underline the whole chain. `unwrapOrP(0)` fixes `T = number` while
    // the previous step produced `TOption<string>`.
    let dir = project(&[(
        "src/labels.tt",
        "import type { TOption } from \"@tt/std\";\n\
         import * as Option from \"@tt/std/option\";\n\
         declare function half(n: number): TOption<number>;\n\
         export function halfLabel(n: number): string {\n\
         \x20 return half(n)\n\
         \x20   |> Option.mapP((x: number) => String(x))\n\
         \x20   |> Option.unwrapOrP(0)\n\
         \x20   |> .toUpperCase();\n\
         }\n",
    )]);
    let out = check(&dir);
    let step = block(&out, "ts2345");
    assert!(
        step.contains("this pipeline step expects `number`, but receives `string`"),
        "the incompatible payloads, not the lowered object types: {out}"
    );
    assert!(
        step.contains("required type: `TOption<number>`"),
        "the step's complete obligation remains visible: {out}"
    );
    assert!(
        step.contains("|> Option.unwrapOrP(0)"),
        "the snippet shows the rejecting step's own line: {out}"
    );
    // The healthy step before it appears only as the producer label —
    // dashes, never the primary carets.
    let caret_rows = step.lines().filter(|line| line.contains('^')).count();
    assert_eq!(caret_rows, 1, "one primary underline: {out}");
    assert!(
        step.contains("--- the piped value is produced here"),
        "the producing step is labeled: {out}"
    );
}

#[test]
fn a_whole_pipeline_mismatch_keeps_the_generic_wording() {
    require_tsgo!();
    // Every boundary of this pipeline is fine; its *result* does not fit
    // the call it sits in. That diagnostic lands on the whole-pipeline
    // anchor (no producer context) and must not claim a step rejected
    // anything (PR #85 review).
    let dir = project(&[(
        "src/arg.tt",
        "const inc = (n: number): number => n + 1;\n\
         declare function takesString(s: string): void;\n\
         takesString(1 |> inc);\n",
    )]);
    let out = check(&dir);
    let mismatch = block(&out, "ts2345");
    assert!(
        mismatch.contains("type mismatch: expected `string`, found `number`"),
        "the pipeline's result is an ordinary mismatch: {out}"
    );
    assert!(
        !out.contains("this pipeline step"),
        "no step is blamed when no boundary failed: {out}"
    );
}

#[test]
fn a_pipeline_mismatch_labels_the_producing_step() {
    require_tsgo!();
    // Rust-style secondary span: the primary carets sit on the rejecting
    // step, and a `-` label points back at the step that produced the
    // value.
    let dir = project(&[(
        "src/pipe.tt",
        "const inc = (n: number): number => n + 1;\n\
         const shout = (s: string): string => s.toUpperCase();\n\
         const a = 1 |> inc |> shout;\n",
    )]);
    let out = check(&dir);
    let step = block(&out, "ts2345");
    assert!(
        step.contains("--- the piped value is produced here"),
        "the producer is labeled under the snippet: {out}"
    );
}

#[test]
fn a_checker_related_place_becomes_a_labeled_span() {
    require_tsgo!();
    // The checker's own related information — here the property whose
    // declared type the literal violates — is mapped back to `.tt`
    // coordinates and drawn as a label, the way rustc labels "expected
    // because of this".
    let dir = project(&[(
        "src/opts.tt",
        "type Opts = { name: string };\n\
         export const o: Opts = { name: 1 };\n",
    )]);
    let out = check(&dir);
    let mismatch = block(&out, "ts2322");
    assert!(
        mismatch.contains("---- The expected type comes from property 'name'"),
        "the declaration is labeled: {out}"
    );
    assert!(
        mismatch.contains("1 | type Opts = { name: string };"),
        "the labeled line is quoted in the same snippet: {out}"
    );
}

#[test]
fn the_server_carries_pipeline_labels() {
    require_tsgo!();
    let source = "const inc = (n: number): number => n + 1;\n\
        const shout = (s: string): string => s.toUpperCase();\n\
        const a = 1 |> inc |> shout;\n";
    let dir = project(&[("src/pipe.tt", source)]);
    let answer = typed_server(&dir, "src/pipe.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    let mismatch = diagnostics
        .iter()
        .find(|d| d["code"] == "ts2345")
        .unwrap_or_else(|| panic!("no ts2345 diagnostic: {diagnostics:?}"));
    let labels = mismatch["labels"].as_array().unwrap_or_else(|| {
        panic!("the wire diagnostic carries its labels: {mismatch:?}");
    });
    assert_eq!(
        labels[0]["message"], "the piped value is produced here",
        "{labels:?}"
    );
    // 1-based coordinates, like the diagnostic itself: `inc` on line 3.
    assert_eq!(labels[0]["line"], 3, "{labels:?}");
}

#[test]
fn the_server_reports_a_pipeline_mismatch_over_the_step_text() {
    require_tsgo!();
    let source = "const inc = (n: number): number => n + 1;\n\
        const shout = (s: string): string => s.toUpperCase();\n\
        const a = 1 |> inc |> shout;\n";
    let dir = project(&[("src/pipe.tt", source)]);
    let answer = typed_server(&dir, "src/pipe.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    let mismatch = diagnostics
        .iter()
        .find(|d| d["code"] == "ts2345")
        .unwrap_or_else(|| panic!("no ts2345 diagnostic: {diagnostics:?}"));
    assert_eq!(
        source_slice(source, mismatch),
        "shout",
        "the range covers exactly the rejecting step: {mismatch:?}"
    );
    assert!(
        mismatch["message"]
            .as_str()
            .unwrap()
            .contains("this pipeline step expects `string`, but receives `number`"),
        "the server carries the same wording as the CLI: {mismatch:?}"
    );
}

#[test]
fn typed_diagnostic_ranges_follow_source_ownership_not_mapping_accidents() {
    require_tsgo!();
    let source = "import type { TResult } from \"@tt/std\";\n\
        import * as Result from \"@tt/std/result\";\n\
        variant Input { Blank, Num(value: number) }\n\
        variant InputError { Empty }\n\
        variant RangeError { TooLarge(value: number) }\n\
        variant Conn { Up(value: number), Down }\n\
        export function toPort(input: Input): TResult<number, InputError> {\n\
        \x20 return match (input) {\n\
        \x20   Blank => Result.Err(InputError.Empty),\n\
        \x20   Num(value) => Result.Err(RangeError.TooLarge(value)),\n\
        \x20 };\n\
        }\n\
        const test = (): TResult<string, number> => Result.Err(10);\n\
        export function bind(): TResult<number, InputError> {\n\
        \x20 return result {\n\
        \x20   const n = try test();\n\
        \x20   return n;\n\
        \x20 };\n\
        }\n\
        export const mixed = (c: Conn): string =>\n\
        \x20 match (c) { Up(value) => \"up\", 404 => \"gone\", Down => \"down\" };\n";
    let dir = project(&[("src/ranges.tt", source)]);
    let answer = typed_server(&dir, "src/ranges.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();

    let match_mismatch = diagnostics
        .iter()
        .find(|d| {
            d["message"]
                .as_str()
                .is_some_and(|m| m.contains("found `RangeError`"))
        })
        .unwrap_or_else(|| panic!("missing match mismatch: {answer}"));
    // The generated slot now carries the authored return annotation, so
    // TypeScript can point at the exact arm value that violates it instead
    // of discovering the mismatch only when the completed match is returned.
    assert_eq!(
        source_slice(source, match_mismatch),
        "Result.Err(RangeError.TooLarge(value))"
    );

    let result_mismatch = diagnostics
        .iter()
        .find(|d| {
            d["message"]
                .as_str()
                .is_some_and(|m| m.contains("required type: `TResult<number, InputError>`"))
                && d["line"].as_u64().is_some_and(|line| line > 10)
        })
        .unwrap_or_else(|| panic!("missing result mismatch: {answer}"));
    assert_eq!(source_slice(source, result_mismatch), "try test()");

    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"] == "match-mixed-patterns"),
        "the direct tt cause remains: {answer}"
    );
    assert!(
        diagnostics.iter().all(|d| d["code"] != "ts2678"),
        "checker consequences owned by the invalid match are suppressed: {answer}"
    );
}

#[test]
fn nested_result_return_is_reported_only_for_a_checker_proven_shape() {
    require_tsgo!();
    let source = r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
declare function read(): Res<number, string>;

const definite = result { const value = try read(); return Res.Ok(value); };
const 값: Res<number, string> = Res.Ok(1);
const definiteUnicode = result { const value = try read(); return 값; };
const union = result { const value = try read(); const candidate: Res<number, string> | number = value; return candidate; };
const nonResult = result { const value = try read(); return String(value); };
const unknown = result { const value = try read(); const candidate: unknown = value; return candidate; };
function generic<T>(candidate: T) { return result { const value = try read(); return candidate; }; }
"#;
    let dir = project(&[("src/nested.tt", source)]);
    let answer = typed_server(&dir, "src/nested.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    let nested: Vec<_> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == "result-return-nested")
        .collect();
    assert_eq!(nested.len(), 2, "{answer}");
    assert_eq!(source_slice(source, nested[0]), "Res.Ok(value)");
    assert_eq!(source_slice(source, nested[1]), "값");
    let edit = &nested[0]["suggestions"][0]["edit"];
    assert_eq!(edit["replacement"], "try ");
}

#[test]
fn a_pattern_typo_suppresses_typed_exhaustiveness_for_that_match() {
    require_tsgo!();
    let dir = project(&[(
        "src/typo.tt",
        "variant Shape { Circle(radius: number), Square(size: number) }\n\
         export function area(shape: Shape): number {\n\
         \x20 return match (shape) { Circel(radius) => radius, Square(size) => size * size };\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("has no case `Circel`"),
        "the cause is reported: {out}"
    );
    assert!(
        !out.contains("not exhaustive"),
        "the typo's typed cascade is suppressed: {out}"
    );
}

#[test]
fn a_reported_imported_field_error_owns_only_its_match_exhaustiveness() {
    require_tsgo!();
    let source = "import { Fulfillment, PaymentMethod } from \"./domain.tt\";\n\
        export function label(state: Fulfillment): string {\n\
        \x20 return match (state) {\n\
        \x20   Pending => \"Pending\",\n\
        \x20   Shipped(carrier, trackng) => `${carrier} ${trackng}`,\n\
        \x20 };\n\
        }\n\
        export function fee(method: PaymentMethod): number {\n\
        \x20 return match (method) { Card(brand) => brand.length };\n\
        }\n";
    let dir = project(&[
        (
            "src/domain.tt",
            "export variant Fulfillment {\n\
             \x20 Pending,\n\
             \x20 Shipped(carrier: string, tracking: string),\n\
             \x20 Delivered,\n\
             \x20 Cancelled,\n\
             }\n\
             export variant PaymentMethod { Card(brand: string), BankTransfer(iban: string) }\n",
        ),
        ("src/combo.tt", source),
    ]);

    let out = check(&dir);
    assert!(
        out.contains("case `Shipped` has no field `trackng`"),
        "the source cause is reported: {out}"
    );
    assert!(
        !out.contains("Delivered") && !out.contains("Cancelled"),
        "the reported cause owns its match's coverage consequence: {out}"
    );
    assert!(
        out.contains("missing \"BankTransfer\""),
        "an independent match keeps its coverage result: {out}"
    );

    let answer = typed_server(&dir, "src/combo.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == "unknown-field"),
        "the server reports the same owner: {answer}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic["code"] == "match-not-exhaustive"
                && diagnostic["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("BankTransfer"))
        }),
        "the server preserves the independent coverage result: {answer}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic["message"].as_str().is_none_or(|message| {
                !message.contains("Delivered") && !message.contains("Cancelled")
            })
        }),
        "the owned coverage consequence stays suppressed: {answer}"
    );
}

#[test]
fn an_imported_case_without_declaration_ownership_uses_checker_evidence() {
    require_tsgo!();
    let source = "import { PaymentMethod } from \"./domain.tt\";\n\
        export function fee(method: PaymentMethod): number {\n\
        \x20 return match (method) { Crad(brand) => 1, _ => 0 };\n\
        }\n";
    let dir = project(&[
        (
            "src/domain.tt",
            "export variant PaymentMethod { Card(brand: string), BankTransfer(iban: string) }\n",
        ),
        ("src/payment.tt", source),
    ]);

    let out = check(&dir);
    assert!(
        out.contains("error[ts2678]")
            && out.contains("Type '\"Crad\"' is not comparable")
            && !out.contains("unknown-case"),
        "the typed CLI reports the checker-proven incompatibility: {out}"
    );

    let answer = typed_server(&dir, "src/payment.tt", source);
    let diagnostics = answer["result"]["diagnostics"].as_array().unwrap();
    let case = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == "ts2678")
        .unwrap_or_else(|| panic!("missing imported case diagnostic: {answer}"));
    assert!(
        case["message"]
            .as_str()
            .is_some_and(|message| message.contains("Crad")),
        "the checker fact names the incompatible case: {answer}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic["code"] != "unknown-case"),
        "no spelling-based owner is inferred: {answer}"
    );
}

#[test]
fn parser_errors_do_not_hide_an_independent_type_error_in_the_same_file() {
    require_tsgo!();
    let dir = project(&[(
        "src/recovery.tt",
        "import type { TResult } from \"@tt/std\";\n\
         import * as Result from \"@tt/std/result\";\n\
         function read(value: number): TResult<number, string> {\n\
         \x20 return Result.Ok(value);\n\
         }\n\
         export function nested(value: number): TResult<number, string> {\n\
         \x20 return result {\n\
         \x20   const first = try read(value);\n\
         \x20   if (first > 0) { const second = try read(first); }\n\
         \x20   return first;\n\
         \x20 };\n\
         }\n\
         const wrong = (): TResult<string, number> => Result.Err(10);\n\
         export function bindNonResult(): TResult<number, string> {\n\
         \x20 return result { const value = try wrong(); return value; };\n\
         }\n\
         export const malformed = match value { Missing => 0 };\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("tt `match` could not be parsed")
            && out.contains("a match scrutinee is parenthesized"),
        "the malformed construct remains visible, with its fix: {out}"
    );
    assert!(
        out.contains("type mismatch: expected `string`, found `number`")
            && out.contains("required type: `TResult<number, string>`"),
        "the independent bindNonResult type error survives recovery: {out}"
    );
}

#[test]
fn a_ts_file_and_an_tt_file_share_one_project_graph() {
    require_tsgo!();
    let dir = project(&[
        (
            "src/user.ts",
            "export type State = \"idle\" | \"loading\" | \"done\";\n",
        ),
        (
            "src/state.tt",
            "import type { State } from \"./user\";\n\
             export function render(state: State): number {\n\
             \x20 return match (state) { \"idle\" => 0, \"loading\" => 1, \"done\" => 2 };\n\
             }\n",
        ),
    ]);
    // The type comes from the `.ts` file; the match is exhaustive over it.
    assert_eq!(check(&dir), "");
}

#[test]
fn literal_exhaustiveness_uses_the_narrowed_type_at_the_match() {
    require_tsgo!();
    let dir = project(&[
        (
            "src/user.ts",
            "export type State = \"idle\" | \"loading\" | \"done\";\n",
        ),
        (
            "src/state.tt",
            "import type { State } from \"./user\";\n\
             export function render(state: State): number {\n\
             \x20 if (state !== \"idle\") {\n\
             \x20   return match (state) { \"loading\" => 1 };\n\
             \x20 }\n\
             \x20 return 0;\n\
             }\n",
        ),
    ]);
    let out = check(&dir);
    assert!(
        out.contains("missing \"done\""),
        "the narrowed type still allows \"done\": {out}"
    );
    assert!(
        !out.contains("idle"),
        "the guard removed \"idle\" before the match: {out}"
    );
}

#[test]
fn variant_exhaustiveness_uses_the_narrowed_type_at_the_match() {
    require_tsgo!();
    let dir = project(&[(
        "src/shape.tt",
        "export variant Shape { Circle(radius: number), Square(side: number), Point }\n\
         export function area(s: Shape): number {\n\
         \x20 if (s.kind !== \"Point\") {\n\
         \x20   return match (s) { Circle(radius) => radius };\n\
         \x20 }\n\
         \x20 return 0;\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("missing \"Square\""),
        "the narrowed type still allows Square: {out}"
    );
    assert!(
        !out.contains("Point"),
        "the guard removed Point before the match: {out}"
    );
}

#[test]
fn val_holds_on_a_parameter_and_across_a_function_boundary() {
    require_tsgo!();
    let dir = project(&[(
        "src/pass.tt",
        "interface User { name: string; tags: string[] }\n         function update(user: User) { user.name = \"Lee\"; }\n         export function process(val user: User) {\n         \x20 user.name = \"Lee\";\n         \x20 user.tags.push(\"x\");\n         \x20 update(user);\n         }\n",
    )]);
    // `val` has two syntactic homes and three rules; a mode that checks
    // only declarations, or only mutation paths, silently passes code the
    // tt-level check rejects.
    let out = check(&dir);
    assert!(
        out.contains("cannot mutate through val binding `user`"),
        "a val parameter is a val binding: {out}"
    );
    assert!(
        out.contains("mutating method `push` through val binding `user`"),
        "and its access paths are read-only too: {out}"
    );
    assert!(
        out.contains("cannot pass val binding `user` to mutable parameter `user` of `update`"),
        "and it cannot be handed to a parameter that is not `val`: {out}"
    );
}

#[test]
fn exhaustiveness_holds_when_the_scrutinee_is_not_a_name() {
    require_tsgo!();
    let dir = project(&[(
        "src/shape.tt",
        "export variant Shape { Circle(radius: number), Rect(w: number, h: number) }\n         declare function getShape(): Shape;\n         type State = \"idle\" | \"loading\" | \"done\";\n         declare function getState(): State;\n         export const area = match (getShape()) { Circle(radius) => radius };\n         export const label = match (getState()) { \"idle\" => 0, \"loading\" => 1 };\n",
    )]);
    // The question is asked about the temporary the match binds, not about
    // the scrutinee's text: at `getShape` the checker answers "a function",
    // which has no cases and no literals, and both questions came back
    // silent when that was where they were asked.
    let out = check(&dir);
    assert!(
        out.contains("missing \"Rect\""),
        "a call scrutinee still has an variant type: {out}"
    );
    assert!(
        out.contains("missing \"done\""),
        "a call scrutinee still has a literal union type: {out}"
    );
}

#[test]
fn a_variant_from_another_module_needs_no_declaration_collecting() {
    require_tsgo!();
    let dir = project(&[
        (
            "src/token.tt",
            "export variant Token { Num(value: number), Eof }\n",
        ),
        (
            "src/parse.tt",
            "import { Token } from \"./token.tt\";\n\
             export function width(t: Token): number {\n\
             \x20 return match (t) { Num(value) => value };\n\
             }\n",
        ),
    ]);
    let out = check(&dir);
    assert!(
        out.contains("missing \"Eof\""),
        "the variant's cases come from the imported module's own type: {out}"
    );
}

#[test]
fn val_mutation_is_decided_by_the_method_the_call_resolves_to() {
    require_tsgo!();
    let dir = project(&[
        (
            "src/store.ts",
            "export class Store {\n  set(key: string, value: string): void {}\n}\n",
        ),
        (
            "src/use.tt",
            "import { Store } from \"./store\";\n\
             export function go(): void {\n\
             \x20 val const map = new Map<string, number>();\n\
             \x20 map.set(\"a\", 1);\n\
             \x20 val const store = new Store();\n\
             \x20 store.set(\"a\", \"b\");\n\
             }\n",
        ),
    ]);
    let out = check(&dir);
    assert!(
        out.contains("mutating method `set` through val binding `map`"),
        "Map#set is declared in TypeScript's own lib: {out}"
    );
    assert!(
        !out.contains("val binding `store`"),
        "Store#set only shares the name: {out}"
    );
}

#[test]
fn a_shadowing_binding_is_a_different_binding() {
    require_tsgo!();
    let dir = project(&[(
        "src/shadow.tt",
        "export function go(): void {\n\
         \x20 val const items = new Map<string, number>();\n\
         \x20 {\n\
         \x20   const items = new Map<string, number>();\n\
         \x20   items.set(\"inner\", 1);\n\
         \x20 }\n\
         }\n",
    )]);
    assert_eq!(
        check(&dir),
        "",
        "the inner `items` is an ordinary binding that shares a name"
    );
}

#[test]
fn a_direct_mutation_through_a_val_binding_is_reported() {
    require_tsgo!();
    let dir = project(&[(
        "src/direct.tt",
        "export function go(): void {\n\
         \x20 val const user = { name: \"a\", count: 0 };\n\
         \x20 user.name = \"b\";\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("cannot mutate through val binding `user`"),
        "an assignment mutates on syntax alone: {out}"
    );
}

#[test]
fn a_mutation_through_an_unmarked_binding_is_left_alone() {
    require_tsgo!();
    let dir = project(&[(
        "src/plain.tt",
        "export function go(): void {\n\
         \x20 const items: number[] = [];\n\
         \x20 items.push(1);\n\
         \x20 const user = { name: \"a\" };\n\
         \x20 user.name = \"b\";\n\
         }\n",
    )]);
    assert_eq!(check(&dir), "");
}

#[test]
fn an_any_receiver_is_never_called_a_mutation() {
    require_tsgo!();
    let dir = project(&[(
        "src/any.tt",
        "export function go(x: any): void {\n\
         \x20 val const y = x;\n\
         \x20 y.set(\"a\", 1);\n\
         \x20 y.push(1);\n\
         }\n",
    )]);
    assert_eq!(check(&dir), "");
}

#[test]
fn a_call_is_checked_against_the_declaration_it_resolves_to() {
    // Two functions share a name; which one a call names is the callee
    // symbol's answer, not the name's. The outer call reaches the
    // top-level declaration (mutable parameter — an error); the inner
    // call reaches the block's val-parameter arrow (fine). The
    // name-keyed model had to skip both as ambiguous.
    require_tsgo!();
    let dir = project(&[(
        "src/who.tt",
        "type U = { name: string };\n\
         export function go(): void {\n\
         \x20 val const user: U = { name: \"a\" };\n\
         \x20 handle(user);\n\
         \x20 {\n\
         \x20   const handle = (val u: U): void => {};\n\
         \x20   handle(user);\n\
         \x20 }\n\
         }\n\
         function handle(u: U): void { u.name = \"b\"; }\n",
    )]);
    let out = check(&dir);
    assert_eq!(
        out.lines()
            .filter(|l| l.contains("cannot pass val binding `user`"))
            .count(),
        1,
        "only the call that names the mutable-parameter declaration: {out}"
    );
    assert!(
        out.contains("src/who.tt:4:10") && out.contains("mutable parameter `u` of `handle`"),
        "reported at the outer call's argument: {out}"
    );
}

#[test]
fn an_answer_past_the_pipe_buffer_still_arrives() {
    // A few hundred diagnostics make the host's one-line answer larger
    // than a pipe buffer (64 KB on Linux). The host must flush the whole
    // line synchronously before it turns around to wait for the next
    // request — an async write that queued the tail past the buffer
    // deadlocked the session: the host blocked reading, the compiler
    // blocked waiting for the rest of the answer.
    require_tsgo!();
    let mut source = String::new();
    for i in 0..400 {
        source.push_str(&format!("export const a{i}: number = \"x{i}\";\n"));
    }
    let dir = project(&[("src/big.tt", source.as_str())]);
    let out = check(&dir);
    assert_eq!(
        out.lines()
            .filter(|l| l.contains("type mismatch: expected `number`"))
            .count(),
        400,
        "every diagnostic of a >64 KB answer arrives: {out}"
    );
}

#[test]
fn a_non_mutating_builtin_method_is_not_a_mutation() {
    // Collection asks about every method call through a `val` path; the
    // verdict is two halves — the checker's (a built-in's method) and tt's
    // policy (one of the mutating ones). A built-in read fails the second,
    // so widening collection must never widen what is reported.
    require_tsgo!();
    let dir = project(&[(
        "src/read.tt",
        "export function go(): void {\n\
         \x20 val const m = new Map<string, number>();\n\
         \x20 m.get(\"a\");\n\
         \x20 m.has(\"a\");\n\
         \x20 val const items: number[] = [];\n\
         \x20 items.at(0);\n\
         \x20 items.includes(1);\n\
         }\n",
    )]);
    assert_eq!(
        check(&dir),
        "",
        "a built-in method outside tt's mutator policy reads, it does not mutate"
    );
}

#[test]
fn batched_answers_land_on_their_own_questions() {
    // One ask carries every module's questions; the host groups them by
    // module for the checker's batch endpoints and scatters the answers
    // back by index. Each diagnostic must land on its own file and line,
    // whichever module its group ran under.
    require_tsgo!();
    let dir = project(&[
        (
            "src/a.tt",
            "declare const x: \"a\" | \"b\";\n\
             export const va = match (x) { \"a\" => 1 };\n\
             export function fa(): void {\n\
             \x20 val const ua = { n: 0 };\n\
             \x20 ua.n = 1;\n\
             }\n",
        ),
        (
            "src/b.tt",
            "declare const y: \"c\" | \"d\";\n\
             export const vb = match (y) { \"c\" => 1 };\n\
             export function fb(): void {\n\
             \x20 val const ub = { m: 0 };\n\
             \x20 ub.m = 1;\n\
             }\n",
        ),
    ]);
    let out = check(&dir);
    for (at, said) in [
        ("--> src/a.tt:2:", "missing \"b\""),
        ("--> src/b.tt:2:", "missing \"d\""),
        ("--> src/a.tt:5:3", "cannot mutate through val binding `ua`"),
        ("--> src/b.tt:5:3", "cannot mutate through val binding `ub`"),
    ] {
        // The message and the position have to be one diagnostic, not two
        // that happen to both be present.
        assert!(
            block(&out, said).contains(at),
            "expected {said} at {at}: {out}"
        );
    }
}

#[test]
fn a_type_error_is_reported_at_its_position_in_the_tt_source() {
    require_tsgo!();
    let dir = project(&[(
        "src/bad.tt",
        // A multi-byte prefix: TypeScript counts UTF-16 code units and the
        // `.tt` position is a byte offset, so the two have to be converted.
        "export function go(): void {\n  const 한글: string = 1;\n}\n",
    )]);
    let out = check(&dir);
    let reported = block(&out, "type mismatch:");
    assert!(
        reported.contains("--> src/bad.tt:2:22"),
        "the diagnostic belongs at the incompatible expression in the .tt file: {out}"
    );
}

#[test]
fn typed_exhaustiveness_sees_a_hole_inside_a_payload() {
    require_tsgo!();
    // The checker names the scrutinee's constituents; tt runs its own
    // exhaustiveness algorithm over that alphabet, so a nested pattern's
    // hole is seen on this path too (TASK-108). Before, the typed path
    // asked only "which top-level tags are missing?" and answered nothing
    // here, while `--check` reported the hole.
    let dir = project(&[(
        "src/nest.tt",
        "variant Inner { Yes(n: number), No }\n\
         variant Outer { Wrap(inner: Inner), Bare }\n\
         declare const o: Outer;\n\
         export const a = match (o) { Wrap(inner: Yes(n)) => n, Bare => -1 };\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("match is not exhaustive: missing \"Wrap(inner: No())\""),
        "the typed path sees the payload hole: {out}"
    );
}

#[test]
fn typed_exhaustiveness_still_answers_from_the_narrowed_type() {
    require_tsgo!();
    // The point of asking the checker at all: a case an earlier test
    // removed is not demanded back. `--check`, which knows only the
    // declaration, does report it.
    let dir = project(&[(
        "src/narrow.tt",
        "variant Shape { Circle(radius: number), Point }\n\
         export function f(x: Shape): number {\n\
         \x20 if (x.kind === \"Point\") return 0;\n\
         \x20 return match (x) { Circle(radius) => radius };\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        !out.contains("not exhaustive"),
        "Point is already excluded here: {out}"
    );
}

#[test]
fn a_hand_written_payload_union_is_named_by_the_checker() {
    require_tsgo!();
    // The payload's declared type is a hand-written union, so no tt
    // declaration describes it — the one thing the declaration table can
    // never answer. The emitted condition tests that payload at exactly
    // its type, and asking there names the column's alphabet (TASK-109).
    let dir = project(&[(
        "src/opaque.tt",
        "type Inner = { kind: \"Yes\"; n: number } | { kind: \"No\" };\n\
         variant Outer { Wrap(inner: Inner), Bare }\n\
         declare const o: Outer;\n\
         export const a = match (o) { Wrap(inner: Yes(n)) => n, Bare => -1 };\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("match is not exhaustive: missing \"Wrap(inner: No())\""),
        "the checker names the payload's constituents: {out}"
    );
}

#[test]
fn a_hand_written_payload_union_fully_covered_is_exhaustive() {
    require_tsgo!();
    // The other half of the same answer: covering the payload's cases
    // makes the match exhaustive, and nothing is reported. Before the
    // payload question existed this stayed quiet too — but only because tt
    // refused to guess, which is a different thing from knowing.
    let dir = project(&[(
        "src/opaque_full.tt",
        "type Inner = { kind: \"Yes\"; n: number } | { kind: \"No\" };\n\
         variant Outer { Wrap(inner: Inner), Bare }\n\
         declare const o: Outer;\n\
         export const a = match (o) {\n\
         \x20 Wrap(inner: Yes(n)) => n,\n\
         \x20 Wrap(inner: No()) => 0,\n\
         \x20 Bare => -1,\n\
         };\n",
    )]);
    let out = check(&dir);
    assert!(!out.contains("not exhaustive"), "covered: {out}");
}

#[test]
fn typed_exhaustiveness_resolves_a_payload_declared_in_another_module() {
    require_tsgo!();
    // The nested column is resolved from declarations, so the imported
    // ones have to be collected on this path too — the same 1-hop
    // collection the default path does.
    let dir = project(&[
        (
            "src/token.tt",
            "export variant Tok { Num(n: number), Eof }\n",
        ),
        (
            "src/line.tt",
            "import { Tok } from \"./token.tt\";\n\
             variant Line { Head(t: Tok), Blank }\n\
             declare const l: Line;\n\
             export const a = match (l) { Head(t: Num(n)) => n, Blank => 0 };\n",
        ),
    ]);
    let out = check(&dir);
    assert!(
        out.contains("match is not exhaustive: missing \"Head(t: Eof())\""),
        "the imported payload variant is resolved: {out}"
    );
}

#[test]
fn typed_exhaustiveness_covers_tuple_matches_too() {
    require_tsgo!();
    // A tuple match asks one question per position. Before, it asked none:
    // the typed path skipped tuple matches entirely, so the product was
    // checked only by the default path's declaration table (TASK-111).
    let dir = project(&[(
        "src/tuple.tt",
        "variant Dir { North(dx: number), South }\n\
         variant Speed { Fast(v: number), Slow }\n\
         declare const d: Dir;\n\
         declare const s: Speed;\n\
         export const n = match (d, s) { (North(dx), Fast(v)) => dx + v, (South, _) => 0 };\n",
    )]);
    let out = check(&dir);
    assert!(
        out.contains("match is not exhaustive: missing (North, Slow)"),
        "the missing combination is named: {out}"
    );
}

#[test]
fn a_tuple_position_the_checker_narrowed_is_not_demanded_back() {
    require_tsgo!();
    // The reason to ask at all: `South` is impossible at the match, so the
    // combinations that need it are not missing. The default path, which
    // knows only the declaration, does report them.
    let dir = project(&[(
        "src/narrowed_tuple.tt",
        "variant Dir { North(dx: number), South }\n\
         variant Speed { Fast(v: number), Slow }\n\
         export function f(d: Dir, s: Speed): number {\n\
         \x20 if (d.kind === \"South\") return 0;\n\
         \x20 return match (d, s) { (North(dx), Fast(v)) => dx + v, (North(dx), Slow) => dx };\n\
         }\n",
    )]);
    let out = check(&dir);
    assert!(
        !out.contains("not exhaustive"),
        "South is impossible: {out}"
    );
}

/// The editor's hardest question, at the compiler layer: completion at a
/// `.` or `?.` the user has just typed, in a pipeline whose value is a
/// `Result`.
///
/// The buffer does not parse — both tails are incomplete — so nothing about
/// it can be decided by parsing it. The probe mends it, and the mended form
/// emits `$tt_ap`, so `@tt/runtime` has to already be resolvable in the
/// workspace or the whole expression comes back untyped and the answer is
/// empty (TASK-217).
#[test]
fn a_probe_answers_in_a_pipeline_the_buffer_cannot_parse_yet() {
    // The engine runs in-process, resolving the toolchain by the same
    // rules this guard mirrors — so a pass here means the compiler found
    // one, not that the test pointed it at one.
    require_tsgo!();
    for tail in [".", "?."] {
        let source = format!(
            "import type {{ TResult }} from \"@tt/std\";\n\
             import * as Result from \"@tt/std/result\";\n\
             \n\
             declare const r: TResult<number, string>;\n\
             const out = r\n\
             \x20 |> Result.mapP((n) => n + 1)\n\
             \x20 |> {tail}"
        );
        let dir = tmpdir();
        fs::create_dir_all(dir.join("src")).unwrap();
        let file = dir.join("src/probe.tt");
        fs::write(&file, &source).unwrap();

        let engine = ttc::engine::Engine::new(None);
        let mut project = engine
            .open_project(
                &[file.to_string_lossy().to_string()],
                &ttc::engine::ProjectOptions::default(),
            )
            .expect("the project opens");
        let lines: Vec<&str> = source.split('\n').collect();
        let position = ttc::engine::Position {
            line: lines.len() as u32 - 1,
            character: lines[lines.len() - 1].chars().count() as u32,
        };
        let answer = project
            .completion(&file, position, true)
            .expect("the probe answers");
        let labels: Vec<&str> = answer.items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            answer.probe.is_some(),
            "the {tail} members had to come from a probe: {labels:?}"
        );
        assert!(
            labels.contains(&"kind"),
            "the value at the {tail} step is a Result: {labels:?}"
        );
    }
}
