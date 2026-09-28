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

#[path = "integration/contextual.rs"]
mod contextual;

#[path = "integration/pr115.rs"]
mod pr115;

#[test]
fn captured_tt_expressions_and_statement_bodies_evaluate_once_in_order() {
    require_toolchain!();
    let lines = run(r#"
variant E { A(value: number), B }
const events: string[] = [];
function note(n: number) { events.push(`n:${n}`); return n; }
function callable() { events.push("callee"); return (n: number) => { events.push(`call:${n}`); return n; }; }
const values = [(note(1) |> ((n: number) => note(n + 1))), match(note(3)) { 3 => 3, _ => 0 }];
const calls = callable()(match(note(4)){4=>4,_=>0}) + callable()(match(note(5)){5=>5,_=>0});
const statements = [(() => { if let A(value) = E.A(note(6)) { return value; } return 0; })(), match(note(7)){7=>7,_=>0}];
const bindings = [(() => { const A(value) = E.A(note(8)) else { return 0; }; return value; })(), match(note(9)){9=>9,_=>0}];
console.log(JSON.stringify([values, calls, statements, bindings]));
console.log(events.join(","));
"#);
    assert_eq!(
        lines,
        [
            "[[2,3],9,[6,7],[8,9]]",
            "n:1,n:2,n:3,callee,n:4,call:4,callee,n:5,call:5,n:6,n:7,n:8,n:9"
        ]
    );
}

#[test]
fn nested_callback_returns_and_overlapping_call_captures_preserve_effects() {
    require_toolchain!();
    let lines = run(r#"
variant State { Ready(values: number[]), Empty }
const events: string[] = [];
const convert = (n: number) => { events.push(`convert:${n}`); return n + 10; };
const answer = match(State.Ready([2, 5])) {
    Ready(values) => { return values.map(n => convert(match(n){0=>0,_=>n}) + convert(match(n){0=>0,_=>n})); },
    Empty => [],
};
const keyed = { [String(match(1){1=>1,_=>0})]: match(2){2=>2,_=>0} };
const truthy = (1 |> ((n: number) => n + 1)) && match(3){3=>3,_=>0};
const fromBody = (() => { if let Ready(values) = State.Ready([4]) { return values[0]; } return 0; })() && match(4){4=>4,_=>0};
const take = (...values: number[]) => values;
const callArgs = take((1 |> ((n: number) => n + 1)), match(3){3=>3,_=>0});
console.log(JSON.stringify([answer, keyed, truthy, fromBody, callArgs]));
console.log(events.join(","));
"#);
    assert_eq!(
        lines,
        [
            "[[24,30],{\"1\":2},3,4,[2,3]]",
            "convert:2,convert:2,convert:5,convert:5"
        ]
    );
}

#[test]
fn a_propagated_value_region_keeps_its_block_returns() {
    require_toolchain!();
    let lines = run(r#"
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const Ok = <T,>(value: T): R<T> => ({ kind: "Ok", value });
const Err = (error: string): R<never> => ({ kind: "Err", error });
function statement(): R<number> {
    const q = try result { const w = try Ok(10); return w + 1; };
    return Ok(q * 2);
}
function product(fail: boolean): R<number> {
    const q = (try result { const w = try (fail ? Err("no") : Ok(10)); return w + 1; }) * 2;
    return Ok(q);
}
function discarded(): R<number> {
    try result { const w = try Err("inner"); return w; };
    return Ok(0);
}
function matched(b: boolean): R<number> {
    const q = (try match (b) { true => { return Ok(10); }, false => Ok(1) }) * 2;
    return Ok(q + 1000);
}
function guarded(ready: boolean, b: boolean): R<number> {
    const q = ready && (try match (b) { true => { return Ok(10); }, false => Ok(1) });
    return Ok(Number(q) + 1000);
}
function argument(b: boolean): R<number> {
    const q = String(try match (b) { true => { return Ok(10); }, false => Ok(1) });
    return Ok(Number(q) + 1000);
}
function alternate(ready: boolean): R<number> {
    const q = ready ? 5 : (try result { const w = try Ok(3); return w + 1; });
    return Ok(q);
}
function piped(): R<string> {
    const q = (try result { const w = try Ok(3); return w + 1; }) |> String;
    return Ok(q);
}
function interpolated(b: boolean): R<number> {
    const q = `${try match (b) { true => { return Ok(7); }, false => Err("x") }}`;
    return Ok(Number(q) + 1);
}
const nested = result { const q = try result { const w = try Ok(3); return w + 1; }; return q * 2; };
console.log(JSON.stringify([statement(), product(false), product(true), discarded()]));
console.log(JSON.stringify([matched(true), guarded(true, true), guarded(false, true), argument(true)]));
console.log(JSON.stringify([alternate(false), piped(), interpolated(true), nested]));
"#);
    assert_eq!(
        lines,
        [
            r#"[{"kind":"Ok","value":22},{"kind":"Ok","value":22},{"kind":"Err","error":"no"},{"kind":"Err","error":"inner"}]"#,
            r#"[{"kind":"Ok","value":1020},{"kind":"Ok","value":1010},{"kind":"Ok","value":1000},{"kind":"Ok","value":1010}]"#,
            r#"[{"kind":"Ok","value":4},{"kind":"Ok","value":"4"},{"kind":"Ok","value":8},{"kind":"Ok","value":8}]"#,
        ]
    );
}

#[test]
fn semicolon_free_statements_keep_their_automatic_boundaries() {
    require_toolchain!();
    let lines = run("variant O { A, B }\n\
const log: string[] = []\n\
const inc = (n: number) => n + 1\n\
const note = (text: string) => { log.push(text) }\n\
function run(x: O, y: O) {\n\
  log.push(\"start\")\n\
  match (x) {\n\
    A => { log.push(\"xa\") },\n\
    B => { log.push(\"xb\") },\n\
  }\n\
  match (y) {\n\
    A => { log.push(\"ya\") },\n\
    B => { log.push(\"yb\") },\n\
  }\n\
  const n = 1\n\
  n |> inc |> String |> note\n\
  const piped = 1 |> inc\n\
  log.push(String(piped))\n\
  const next = 2\n\
  next |> String |> note\n\
}\n\
run(O.A, O.B)\n\
console.log(log.join(\",\"))\n");
    assert_eq!(lines, ["start,xa,yb,2,2,2"]);
}

#[test]
fn a_semicolon_free_brace_after_an_expression_is_a_diverging_block() {
    require_toolchain!();
    let lines = run(
        "type Opt = { kind: \"Some\"; value: number } | { kind: \"None\" }\n\
let foo = 0, Foo = 0\n\
function unwrap(o: Opt): number {\n\
  let Some(value) = o else {\n\
    foo\n\
    Foo\n\
    { return -1 }\n\
  };\n\
  return value\n\
}\n\
function arm(o: Opt): number {\n\
  const n: number = match (o) {\n\
    Some(value) => {\n\
      foo\n\
      { return value * 2 }\n\
    },\n\
    None => 0,\n\
  }\n\
  return n\n\
}\n\
function body(o: Opt): number {\n\
  if let Some(value) = o {\n\
    Foo\n\
    { return value + 1 }\n\
  } else {\n\
    foo\n\
    { return -2 }\n\
  }\n\
}\n\
console.log([unwrap({ kind: \"Some\", value: 3 }), unwrap({ kind: \"None\" })].join(\",\"))\n\
console.log([arm({ kind: \"Some\", value: 3 }), body({ kind: \"Some\", value: 3 }), body({ kind: \"None\" })].join(\",\"))\n",
    );
    assert_eq!(lines, ["3,-1", "6,4,-2"]);
}

#[test]
fn a_member_step_calls_the_method_on_its_receiver() {
    require_toolchain!();
    let lines = run(r#"
const order: string[] = [];
const obj = { k: 10, add(n: number) { return n + this.k; } };
const key = "add" as const;
const traced = { k: 1, get m() { order.push("get"); return function (this: { k: number }, n: number) { return n + this.k; }; } };
class Base { k = 3; m(n: number) { return n + this.k; } }
class Derived extends Base {
    #p(n: number) { return n - this.k; }
    run() { return [2].map(x => x |> this.m |> this.#p); }
    parent() { return [4].map(x => x |> super.m); }
    composed() { return flow |> super.m |> this.#p; }
}
const gen = { id<T>(v: T): T { return v; } };
const inlined = 1 |> obj.add;
const nested = [1].map(x => x |> obj.add);
const chained = 1 |> obj.add |> obj.add;
const computed = [2].map(x => x |> obj[key]);
const generic: number = (1 + 1) |> gen.id;
const ordered = (order.push("head"), 1) |> traced.m;
const composed = flow |> ((n: number) => n * 2) |> obj.add |> String;
async function awaited() { return 3 |> (await Promise.resolve(obj)).add; }
awaited().then(q => {
    console.log(JSON.stringify([inlined, nested, chained, computed, generic, ordered, q]));
    console.log(JSON.stringify([new Derived().run(), new Derived().parent(), new Derived().composed()(5), composed(1), order]));
});
"#);
    assert_eq!(
        lines,
        [
            "[11,[11],21,[12],2,2,13]",
            r#"[[2],[7],5,"12",["head","get"]]"#
        ]
    );
}

#[test]
fn a_conditional_branch_owns_the_calls_around_its_value() {
    require_toolchain!();
    let lines = run(r#"
variant O { A, B }
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const Ok = <T,>(value: T): R<T> => ({ kind: "Ok", value });
const Err = (error: string): R<never> => ({ kind: "Err", error });
const log: string[] = [];
const f = (n: number) => { log.push("f" + n); return n * 10; };
const g = (n: number) => { log.push("g" + n); return n + 1; };
function both(c: boolean, o: O) { return c ? f(match (o) { A => 1, B => 2 }) : g(match (o) { A => 3, B => 4 }); }
function left(c: boolean, o: O) { return c ? f(match (o) { A => 1, B => 2 }) : 0; }
function right(c: boolean, o: O) { return c ? match (o) { A => 5, B => 6 } : g(match (o) { A => 7, B => 8 }); }
function wrapped(c: boolean, r: R<number>): R<number> { const v = c ? Ok(try r) : Ok(0); return v; }
function summed(c: boolean, r: R<number>): R<number> { const v = c ? (try r) + 1 : 0; return Ok(v); }
console.log(JSON.stringify([both(true, O.A), both(false, O.B), left(true, O.B), left(false, O.A), right(true, O.B), right(false, O.A), log]));
console.log(JSON.stringify([wrapped(true, Ok(4)), wrapped(true, Err("e")), wrapped(false, Err("e")), summed(true, Ok(4)), summed(true, Err("e")), summed(false, Err("e"))]));
"#);
    assert_eq!(
        lines,
        [
            r#"[10,5,20,0,6,8,["f1","g4","f2","g7"]]"#,
            r#"[{"kind":"Ok","value":4},{"kind":"Err","error":"e"},{"kind":"Ok","value":0},{"kind":"Ok","value":5},{"kind":"Err","error":"e"},{"kind":"Ok","value":0}]"#
        ]
    );
}

#[test]
fn a_c_style_loop_test_captures_its_left_operand_once() {
    require_toolchain!();
    let lines = run(r#"
const xs = [3];
let j = 0;
const seen: number[] = [];
for (; j < match (xs[0]) { 3 => 3, _ => 0 };) { seen.push(j); j++; }
for (let k = 0; k < match (xs[0]) { 3 => 2, _ => 0 }; k++) { seen.push(10 + k); }
console.log(JSON.stringify(seen));
"#);
    assert_eq!(lines, ["[0,1,2,10,11]"]);
}

#[test]
fn a_wildcard_only_match_reads_nothing_from_its_subject() {
    require_toolchain!();
    let lines = run(r#"
const values: (number | null | undefined)[] = [1, null, undefined];
console.log(JSON.stringify(values.map(v => match (v) { _ => "any" })));
"#);
    assert_eq!(lines, [r#"["any","any","any"]"#]);
}

#[test]
fn an_optional_member_step_is_the_optional_call() {
    require_toolchain!();
    let lines = run(r#"
class C { k = 3; m(x: number) { return x * this.k; } }
const pick = <T,>(value: T): T => [value][0];
const o: C | undefined = pick<C | undefined>(new C());
const none: C | undefined = pick<C | undefined>(undefined);
const nested: { c?: C } | undefined = pick<{ c?: C } | undefined>({ c: new C() });
const order: string[] = [];
const head = () => { order.push("head"); return 2; };
console.log(JSON.stringify([head() |> o?.m, head() |> none?.m, head() |> nested?.c?.m, order]));
"#);
    assert_eq!(lines, [r#"[6,null,6,["head","head","head"]]"#]);
}

#[test]
fn generated_guards_reach_the_host_error_and_json_past_user_declarations() {
    require_toolchain!();
    let lines = run(r#"
variant Error { Bad(msg: string), Worse }
function m(x: Error) { return match (x) { Bad(msg) => msg, Worse => "w" }; }
try { m({ kind: "Nope" } as unknown as Error); } catch (e) { console.log((e as globalThis.Error).message); }
const JSON = 1;
const k = (v: "a" | "b") => match (v) { "a" => 1, "b" => JSON };
try { k("c" as unknown as "a"); } catch (e) { console.log((e as globalThis.Error).message); }
"#);
    assert_eq!(
        lines,
        [
            r#"tt match: unexpected case {"kind":"Nope"}"#,
            r#"tt match: unexpected literal "c""#
        ]
    );
}

#[test]
fn generated_guards_reach_the_host_globals_past_a_shadowed_global_this() {
    require_toolchain!();
    let lines = run(r#"
variant O { Some(value: number), None }
function f(o: O, globalThis: unknown) { const Error = 5; return match (o) { Some(value) => value, None => Error }; }
try { f({ kind: "Nope" } as unknown as O, 1); } catch (e) { console.log(e instanceof RangeError, (e as { message: string }).message); }
function g(v: 1 | 2, s: "a" | "b", globalThis: unknown) {
  const JSON = 1, String = 2;
  return match (v) { 1 => JSON, 2 => String } + match (s) { "a" => 1, "b" => 2 };
}
try { g(3 as 1, "a", 1); } catch (e) { console.log((e as { message: string }).message); }
try { g(1, "c" as "a", 1); } catch (e) { console.log((e as { message: string }).message); }
"#);
    assert_eq!(
        lines,
        [
            r#"false tt match: unexpected case {"kind":"Nope"}"#,
            "tt match: unexpected literal 3",
            r#"tt match: unexpected literal "c""#
        ]
    );
    let lines = run(r#"
variant O { Some(value: number), None }
const globalThis = { Error: 1 };
function f(o: O) { const Error = 5; return match (o) { Some(value) => value, None => Error + globalThis.Error }; }
try { f({ kind: "Nope" } as unknown as O); } catch (e) { console.log((e as { message: string }).message); }
"#);
    assert_eq!(lines, [r#"tt match: unexpected case {"kind":"Nope"}"#]);
}

#[test]
fn runtime_a_type_assertion_after_a_pipeline_asserts_the_piped_value() {
    require_toolchain!();
    let out = run(r#"
const maybe = undefined as ((n: number) => string) | undefined;
const twice = (n: number) => n * 2;
const d = 1 |> String as string;
const e = 2 |> twice satisfies number;
const g = 3 |> ((x: number) => x + 1) as number;
const h = 4 |> twice as number |> String satisfies string |> .length;
const k = flow |> twice as (n: number) => number;
const l = 5 |> (maybe ?? String);
const m = 6 |> maybe ?? String;
console.log(d, e, g, h, k(7), l, m);
"#);
    assert_eq!(out, ["1 4 4 1 14 5 6"]);
}

#[test]
fn runtime_a_wrapped_concise_arrow_value_runs_in_its_own_async_body() {
    require_toolchain!();
    let out = run(r#"
variant V { A(n: number), B }
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const load = async (n: number): Promise<R<number>> => n > 0 ? { kind: "Ok", value: n } : { kind: "Err", error: "neg" };
const g = async (p: Promise<V>) => match (await p) { A(n) => n, B => 0 } as number;
const h = (v: V) => match (v) { A(n) => n, B => 0 } satisfies number;
const i = async (p: Promise<V>) => (match (await p) { A(n) => n * 2, B => 0 }) as number;
const j = (v: V) => (match (v) { A(n) => n, B => -1 });
const r = async (n: number) => result { const v = try await load(n); return v + 1; } as R<number>;
console.log(await g(Promise.resolve(V.A(3))), h(V.B), await i(Promise.resolve(V.A(4))), j(V.B));
console.log(JSON.stringify(await r(1)), JSON.stringify(await r(-1)));
"#);
    assert_eq!(
        out,
        [
            "3 0 8 -1",
            r#"{"kind":"Ok","value":2} {"kind":"Err","error":"neg"}"#
        ]
    );
}
