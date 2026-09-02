//! End-to-end tests: compile tt → TypeScript, then run `tsc` to type-check
//! (exhaustiveness is checked by ttc itself; tsc sees plain TypeScript) and `node` to execute.
//!
//! These tests skip silently when `tsc` or `node` is not installed.

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
    let out = Command::new("tsc")
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
    if !have("tsc") {
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
    let out = Command::new("tsc")
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
    if !have("tsc") {
        return;
    }
    let dir = tmpdir();
    write_std(&dir);
    let std_imports = ttc::StdImports {
        types: Some("./tt/index.js"),
        option: Some("./tt/option.js"),
        result: Some("./tt/result.js"),
        runtime: Some("./tt/runtime.js"),
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
    let out = Command::new("tsc")
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
    let out = Command::new("tsc")
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
    let out = Command::new("tsc")
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
    if !have("tsc") {
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
    let out = Command::new("tsc")
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

#[test]
fn a_grouped_value_still_evaluates_to_what_the_arm_wrote() {
    // The parentheses codegen keeps around a lowered value are load
    // bearing: a comma expression delivered without them would take the
    // wrong operand, and a non-primary pipeline receiver would rebind the
    // member access. Both are executed here, not just matched as text.
    if !have("tsc") || !have("node") {
        return;
    }
    let out = run("variant E { A(v: number), B }\n\
         const e: E = E.A(1);\n\
         const seen: number[] = [];\n\
         const note = (n: number): number => { seen.push(n); return n; };\n\
         const seq = match (e) {\n\
           A(v) => {\n\
             return note(v), v + 10;\n\
           },\n\
           B => 0,\n\
         };\n\
         const width = 3;\n\
         const receiver = width + 0.5 |> .toFixed(1);\n\
         const chained = \"  pad  \" |> .trim() |> .length;\n\
         console.log(seq, seen.join(\",\"), receiver, chained);\n");
    // 11, not 1: the arm's `return` is rewritten into an assignment, and a
    // comma expression assigned without parentheses would take the LEFT
    // operand — `$tt_v = note(v), v + 10;` still parses, so only running it
    // catches that. "3.5", not "30.5": the receiver rule has to write
    // `(width + 0.5).toFixed(1)` — the head carries no parentheses of its
    // own, and `width + 0.5.toFixed(1)` type-checks just as well.
    assert_eq!(out, ["11 1 3.5 3"], "{out:?}");
}

#[test]
fn a_block_arm_yields_the_same_value_whether_or_not_it_can_fall_out() {
    // Dropping the fall-through of a block arm that always leaves is only
    // sound if it really always leaves: get it wrong on a `switch` and
    // control runs into the next case. All three shapes are executed —
    // always leaves, leaves conditionally, never leaves — and each is
    // followed by another arm that must not run.
    if !have("tsc") || !have("node") {
        return;
    }
    let out = run("variant E { A(v: number), B }\n\
         const run = (e: E): unknown => match (e) {\n\
           A(v) => {\n\
             if (v > 0) { return \"positive\"; }\n\
             throw new Error(\"not positive\");\n\
           },\n\
           B => \"b\",\n\
         };\n\
         const maybe = (e: E): unknown => match (e) {\n\
           A(v) => {\n\
             if (v > 0) { return \"positive\"; }\n\
           },\n\
           B => \"b\",\n\
         };\n\
         const never = (e: E): unknown => match (e) {\n\
           A(v) => {\n\
             void v;\n\
           },\n\
           B => \"b\",\n\
         };\n\
         let threw = \"no\";\n\
         try { run(E.A(-1)); } catch { threw = \"yes\"; }\n\
         console.log([\n\
           run(E.A(1)),\n\
           threw,\n\
           run(E.B),\n\
           maybe(E.A(1)),\n\
           String(maybe(E.A(-1))),\n\
           maybe(E.B),\n\
           String(never(E.A(1))),\n\
           never(E.B),\n\
         ].join(\"|\"));\n");
    // The `B` arm never runs for an `A` value: a dropped fall-through that
    // was not really unreachable would print "b" where "undefined" is.
    assert_eq!(
        out,
        ["positive|yes|b|positive|undefined|b|undefined|b"],
        "{out:?}"
    );
}

/// Compile a snippet that imports the standard library, emit JS for it and
/// the std package with tsc, execute with node, return stdout lines.
fn run_with_std(src: &str) -> Vec<String> {
    let code = compile(src, &options_with_runtime("./tt/runtime.js")).expect("tt compile failed");
    let dir = tmpdir();
    write_std(&dir);
    fs::write(dir.join("main.ts"), &code).unwrap();
    fs::write(dir.join("package.json"), "{ \"type\": \"module\" }\n").unwrap();
    let out = Command::new("tsc")
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
        if !have("tsc") || !have("node") {
            eprintln!("skipping: tsc/node not available");
            return;
        }
    };
}

/* ------------------------------------------------------------------ */
/* runtime behavior                                                    */
/* ------------------------------------------------------------------ */

#[test]
fn runtime_variant_construction_and_match() {
    require_toolchain!();
    let lines = run(r#"
variant Shape {
  Circle(radius: number),
  Rect(width: number, height: number),
  Point,
}

function area(s: Shape): number {
  return match (s) {
    Circle(radius) => Math.PI * radius * radius,
    Rect(width, height) => width * height,
    Point => 0,
  };
}

console.log(JSON.stringify([area(Shape.Circle(1)), area(Shape.Rect(3, 4)), area(Shape.Point)]));
console.log(JSON.stringify(Shape.Circle(2)));
console.log(JSON.stringify(Shape.Point));
"#);
    assert_eq!(
        lines,
        vec![
            format!("[{},12,0]", std::f64::consts::PI),
            r#"{"kind":"Circle","radius":2}"#.to_string(),
            r#"{"kind":"Point"}"#.to_string(),
        ]
    );
}

#[test]
fn runtime_binding_aliases_and_block_bodies() {
    require_toolchain!();
    let lines = run(r#"
variant Msg {
  Quit,
  Move(x: number, y: number),
  Write(text: string),
}

function describe(m: Msg): string {
  return match (m) {
    Move(x: px, y: py) => {
      const sum = px + py;
      return "move:" + sum;
    },
    Write(text) => "write:" + text,
    Quit => "quit",
  };
}

console.log(describe(Msg.Move(2, 3)));
console.log(describe(Msg.Write("hi")));
console.log(describe(Msg.Quit));
"#);
    assert_eq!(lines, vec!["move:5", "write:hi", "quit"]);
}

#[test]
fn runtime_owner_lowering_preserves_reference_order_and_block_exits() {
    require_toolchain!();
    let lines = run(r#"
variant E { A(value: number), B }
const events: string[] = [];
const receiver = {
  get method() {
    events.push("callee");
    return function (this: unknown, before: number, value: number, after: number) {
      events.push(`call:${this === receiver}:${before}:${value}:${after}`);
      return value;
    };
  },
};
function effect<T>(label: string, value: T): T {
  events.push(label);
  return value;
}
const value = receiver.method(
  effect("before", 1),
  match (effect("subject", E.A(2))) { A(value) => value, B => 0 },
  effect("after", 3),
);
const block = match (E.A(4)) {
  A(value) => {
    if (value > 0) return value * 2;
    return 0;
  },
  B => { return -1; },
};
const nested = match (E.A(5)) {
  A(value) => {
    const add = () => { return value + 1; };
    return add();
  },
  B => { return 0; },
};
console.log(events.join(","));
console.log(value, block, nested);
"#);
    assert_eq!(
        lines,
        ["callee,before,subject,after,call:true:1:2:3", "2 8 6",]
    );
}

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

#[test]
fn runtime_is_patterns_and_loop_test_regions_preserve_order_and_count() {
    require_toolchain!();
    let lines = run(r#"
class Keep extends Error {}
class Stop extends Error {}
let probes = 0;
let updates = 0;
let bodies = 0;
function probe(): Error {
  probes += 1;
  return probes <= 3 ? new Keep() : new Stop();
}
for (; match (probe()) { is Keep => true, _ => false }; updates += 1) {
  bodies += 1;
  if (bodies < 3) continue;
}
const message = match (new SyntaxError("bad")) {
  is SyntaxError { message } if message.length > 0 => message,
  is Error { message: detail } => detail,
  _ => "unknown",
};
console.log(probes, updates, bodies, message);
"#);
    assert_eq!(lines, ["4 3 3 bad"]);
}

#[test]
fn runtime_reference_protocol_preserves_optional_and_tagged_calls() {
    require_toolchain!();
    let lines = run(r#"
variant E { A(value: number), B }
const events: string[] = [];
const receiver = {
  get method() {
    events.push("method");
    return function (this: unknown, value: number) {
      events.push(`call:${this === receiver}:${value}`);
      return value;
    };
  },
  get tag() {
    events.push("tag");
    return function (this: unknown, strings: TemplateStringsArray, value: number) {
      events.push(`tag-call:${this === receiver}:${value}`);
      return (strings[0] ?? "") + value;
    };
  },
};
const absent: { method: ((value: number) => number) | null } = {
  get method() { return null as ((value: number) => number) | null; },
};
function effect(value: E): E {
  events.push("subject");
  return value;
}
const present = receiver.method?.(
  match (effect(E.A(2))) { A(value) => value, B => 0 },
);
const missing = absent.method?.(
  match (effect(E.A(3))) { A(value) => value, B => 0 },
);
const tagged = receiver.tag`value:${match (effect(E.A(4))) {
  A(value) => value,
  B => 0,
}}`;
console.log(events.join(","));
console.log(present, missing, tagged);
"#);
    assert_eq!(
        lines,
        [
            "method,subject,call:true:2,tag,subject,tag-call:true:4",
            "2 undefined value:4",
        ]
    );
}

include!("integration/cases_01.rs");
include!("integration/cases_02.rs");
include!("integration/cases_03.rs");
include!("integration/cases_04.rs");
include!("integration/cases_05.rs");
