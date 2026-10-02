//! End-to-end tests: compile tt → TypeScript, then run `tsc` to type-check
//! (exhaustiveness is checked by ttc itself; tsc sees plain TypeScript) and `node` to execute.
//!
//! `tsc` is the TypeScript `package.json` pins (`common::tsc`). These tests
//! skip silently when it or `node` is not installed, and fail instead under
//! `TTC_REQUIRE_TSGO=1`.

use std::fs;
use std::process::Command;

use ttc::{Options, SourceKind, compile};

const TSC_FLAGS: &[&str] = &[
    "--strict",
    "--target",
    "es2022",
    "--module",
    "esnext",
    "--moduleResolution",
    "bundler",
    "--skipLibCheck",
];

fn have(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

mod common;
use common::Workspace;

/// A directory for one case, removed when the case ends — and kept, with
/// its path printed, when the case failed (`tests/common/mod.rs`).
fn tmpdir() -> Workspace {
    Workspace::new("test")
}

/// A directory for a case whose project needs **dependencies**: TypeScript
/// is resolved from `node_modules` walking upwards, so a project under the
/// repository inherits the repository's install while one in the system
/// temp directory has none (`tests/common/mod.rs`).
fn project_dir() -> Workspace {
    Workspace::in_repo("test")
}

/// Appended to every snippet so it is a module (like real tt files with
/// exports) — otherwise script-scope names collide with DOM globals
/// such as `Option`.
fn as_module(src: &str) -> String {
    format!("{src}\nexport {{}};\n")
}

fn write_std(dir: &std::path::Path) {
    let std_dir = dir.join("tt");
    fs::create_dir_all(&std_dir).unwrap();
    for module in ttc::StdModule::ALL {
        fs::write(std_dir.join(module.file_name()), module.source()).unwrap();
    }
}

fn options_with_runtime(specifier: &str) -> Options<'_> {
    Options {
        std_imports: ttc::StdImports {
            runtime: Some(specifier),
            ..ttc::StdImports::default()
        },
        ..Options::default()
    }
}

fn write_runtime(dir: &std::path::Path) {
    fs::write(dir.join("runtime.ts"), ttc::RUNTIME_SOURCE).unwrap();
}

/// Everything the child said, so a failure that is not a type error still
/// names itself.
///
/// `tsc`'s diagnostics go to stdout, and printing only those makes a run
/// that never got that far — killed for memory, missing from PATH, dead on
/// a signal — look like a check that simply found nothing. An intermittent
/// failure that leaves no evidence is one nobody can act on
/// (docs/tasks/TASK-222).
fn tsc_report(out: &std::process::Output) -> String {
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !stderr.trim().is_empty() {
        text.push_str("\n---tsc stderr---\n");
        text.push_str(&stderr);
    }
    if !out.status.success() {
        text.push_str(&format!("\n---tsc exit: {}---\n", out.status));
    }
    text
}

/// Compile tt source and type-check the output with tsc. Returns (ok, tsc output).
fn typecheck(src: &str) -> (bool, String) {
    let code =
        compile(&as_module(src), &options_with_runtime("./runtime.js")).expect("tt compile failed");
    let dir = tmpdir();
    write_runtime(&dir);
    let ts = dir.join("main.ts");
    fs::write(&ts, &code).unwrap();
    let out = common::tsc()
        .arg(&ts)
        .arg("--noEmit")
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    let text = tsc_report(&out);
    (
        out.status.success(),
        format!("{text}\n---compiled---\n{code}"),
    )
}

#[test]
fn ttx_output_typechecks_as_tsx() {
    if !common::tsc_available() {
        return;
    }
    let source = r#"declare global {
  namespace JSX { interface IntrinsicElements { main: {}; b: {}; } }
}

variant State { Ready(value: string), Empty }
export const render = (state: State) => <main>{match (state) {
  Ready(value) => <b>{value}</b>,
  Empty => null,
}}</main>;
"#;
    let code = compile(
        source,
        &Options {
            source_kind: SourceKind::Tsx,
            ..Options::default()
        },
    )
    .expect("ttx compile failed");
    let dir = tmpdir();
    let tsx = dir.join("main.tsx");
    fs::write(&tsx, &code).unwrap();
    let out = common::tsc()
        .arg(&tsx)
        .arg("--noEmit")
        .arg("--jsx")
        .arg("preserve")
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "{}\n---compiled---\n{code}",
        tsc_report(&out)
    );
}

#[test]
fn mixed_source_fixture_emits_one_type_clean_typescript_tree() {
    if !common::tsc_available() {
        return;
    }
    let dir = tmpdir();
    write_std(&dir);
    let std_imports = ttc::StdImports {
        types: Some("./tt/index.js"),
        option: Some("./tt/option.js"),
        result: Some("./tt/result.js"),
        runtime: Some("./tt/runtime.js"),
        commonjs: None,
    };
    let files = [
        (
            "plain.ts",
            include_str!("fixtures/mixed-source-matrix/src/plain.ts"),
            SourceKind::TypeScript,
        ),
        (
            "same.ts",
            include_str!("fixtures/mixed-source-matrix/src/same.ts"),
            SourceKind::TypeScript,
        ),
        (
            "plain-jsx.tsx",
            include_str!("fixtures/mixed-source-matrix/src/plain-jsx.tsx"),
            SourceKind::Tsx,
        ),
        (
            "same-jsx.tsx",
            include_str!("fixtures/mixed-source-matrix/src/same-jsx.tsx"),
            SourceKind::Tsx,
        ),
        (
            "language.ts",
            include_str!("fixtures/mixed-source-matrix/src/language.tt"),
            SourceKind::TypeScript,
        ),
        (
            "same-tt.ts",
            include_str!("fixtures/mixed-source-matrix/src/same-tt.tt"),
            SourceKind::TypeScript,
        ),
        (
            "language-jsx.tsx",
            include_str!("fixtures/mixed-source-matrix/src/language-jsx.ttx"),
            SourceKind::Tsx,
        ),
        (
            "same-ttx.tsx",
            include_str!("fixtures/mixed-source-matrix/src/same-ttx.ttx"),
            SourceKind::Tsx,
        ),
    ];
    let mut emitted = Vec::new();
    for (name, source, source_kind) in files {
        let output = compile(
            source,
            &Options {
                source_kind,
                std_imports,
                ..Options::default()
            },
        )
        .unwrap_or_else(|error| panic!("{name} failed to compile: {error:#?}"));
        let path = dir.join(name);
        fs::write(&path, output).unwrap();
        emitted.push(path);
    }
    let out = common::tsc()
        .args(&emitted)
        .args([
            dir.join("tt/index.ts"),
            dir.join("tt/option.ts"),
            dir.join("tt/result.ts"),
            dir.join("tt/runtime.ts"),
        ])
        .arg("--noEmit")
        .arg("--jsx")
        .arg("preserve")
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    assert!(out.status.success(), "{}", tsc_report(&out));
}

/// Type-check code emitted despite recoverable tt diagnostics.
fn typecheck_recovery(src: &str) -> (bool, String) {
    let report = ttc::compile_report(&as_module(src), &options_with_runtime("./runtime.js"));
    assert!(!report.diagnostics.is_empty(), "expected a tt diagnostic");
    let code = report
        .emit
        .expect("recoverable diagnostics still emit")
        .code;
    let dir = tmpdir();
    write_runtime(&dir);
    let ts = dir.join("main.ts");
    fs::write(&ts, &code).unwrap();
    let out = common::tsc()
        .arg(&ts)
        .arg("--noEmit")
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    let text = tsc_report(&out);
    (
        out.status.success(),
        format!("{text}\n---compiled---\n{code}"),
    )
}

/// Type-check a snippet that imports the standard library: the std module is
/// written under `tt/` and all files go through tsc (`--noEmit`).
/// Returns (ok, tsc output + compiled source).
fn typecheck_with_std(src: &str) -> (bool, String) {
    let code = compile(&as_module(src), &options_with_runtime("./tt/runtime.js"))
        .expect("tt compile failed");
    let dir = tmpdir();
    write_std(&dir);
    fs::write(dir.join("main.ts"), &code).unwrap();
    let out = common::tsc()
        .arg(dir.join("main.ts"))
        .arg(dir.join("tt/index.ts"))
        .arg(dir.join("tt/option.ts"))
        .arg(dir.join("tt/result.ts"))
        .arg("--noEmit")
        .args([
            "--strict",
            "--target",
            "es2022",
            "--module",
            "nodenext",
            "--moduleResolution",
            "nodenext",
        ])
        .output()
        .expect("failed to run tsc");
    let text = tsc_report(&out);
    (
        out.status.success(),
        format!("{text}\n---compiled---\n{code}"),
    )
}

#[test]
fn recoverable_codegen_errors_do_not_create_tsc_errors() {
    if !common::tsc_available() {
        return;
    }

    let duplicate_case = "variant E { A(x: number), B, A(y: number) }\n";
    let (ok, out) = typecheck_recovery(duplicate_case);
    assert!(ok, "tsc rejected duplicate-case recovery:\n{out}");

    let duplicate_binding = "variant E { A(left: number, right: number), B }\n\
        const value = match (E.A(1, 2)) { A(left: x, right: x) => x, B => 0 };\n";
    let (ok, out) = typecheck_recovery(duplicate_binding);
    assert!(ok, "tsc rejected duplicate-binding recovery:\n{out}");
}

/// Compile tt source, emit JS with tsc, execute with node, return stdout lines.
fn run(src: &str) -> Vec<String> {
    run_with_tsc_flags(src, &[])
}

/// Run one program with extra TypeScript flags needed by a language feature.
fn run_with_tsc_flags(src: &str, extra_flags: &[&str]) -> Vec<String> {
    let code =
        compile(&as_module(src), &options_with_runtime("./runtime.js")).expect("tt compile failed");
    let dir = tmpdir();
    write_runtime(&dir);
    let ts = dir.join("main.ts");
    fs::write(&ts, &code).unwrap();
    // the emitted .js contains `export {}` — run it as an ES module
    fs::write(dir.join("package.json"), "{ \"type\": \"module\" }\n").unwrap();
    let out = common::tsc()
        .arg(&ts)
        .arg("--outDir")
        .arg(&dir)
        .args(TSC_FLAGS)
        .args(extra_flags)
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "tsc failed:\n{}\n---compiled---\n{code}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = Command::new("node")
        .arg(dir.join("main.js"))
        .output()
        .expect("failed to run node");
    assert!(
        out.status.success(),
        "node failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

/// Compile a snippet that imports the standard library, emit JS for it and
/// the std package with tsc, execute with node, return stdout lines.
fn run_with_std(src: &str) -> Vec<String> {
    let code = compile(src, &options_with_runtime("./tt/runtime.js")).expect("tt compile failed");
    let dir = tmpdir();
    write_std(&dir);
    fs::write(dir.join("main.ts"), &code).unwrap();
    fs::write(dir.join("package.json"), "{ \"type\": \"module\" }\n").unwrap();
    let out = common::tsc()
        .arg(dir.join("main.ts"))
        .arg(dir.join("tt/index.ts"))
        .arg(dir.join("tt/option.ts"))
        .arg(dir.join("tt/result.ts"))
        .arg("--outDir")
        .arg(&dir)
        .args([
            "--strict",
            "--target",
            "es2022",
            "--module",
            "nodenext",
            "--moduleResolution",
            "nodenext",
        ])
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "tsc failed:\n{}\n---compiled---\n{code}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = Command::new("node")
        .arg(dir.join("main.js"))
        .output()
        .expect("failed to run node");
    assert!(
        out.status.success(),
        "node failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

macro_rules! require_toolchain {
    () => {
        if !common::tsc_available() {
            eprintln!("skipping: node or the pinned TypeScript is not installed");
            return;
        }
    };
}

/* ------------------------------------------------------------------ */
/* runtime behavior                                                    */
/* ------------------------------------------------------------------ */

#[test]
fn parameter_and_field_matches_require_a_statement_owner() {
    let source = r#"
variant E { A(value: number), B }
function parameter(
  seed: number,
  value = match (E.A(seed + arguments.length)) {
    A(value) => { return value; },
    B => { return 0; },
  },
) {
  return value;
}
class Counter {
  seed = 4;
  value = match (E.A(this.seed + 1)) {
    A(value) => { return value; },
    B => { return 0; },
  };
}
console.log(parameter.length, parameter(3));
console.log(new Counter().value);
"#;
    assert!(compile(source, &Options::default()).is_err());
}

include!("integration/cases_01.rs");
include!("integration/cases_02.rs");
include!("integration/cases_03.rs");
include!("integration/cases_04.rs");
include!("integration/cases_05.rs");

#[path = "integration/contextual.rs"]
mod contextual;
