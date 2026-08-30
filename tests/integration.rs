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

#[test]
fn runtime_or_patterns_share_one_body() {
    require_toolchain!();
    let lines = run(r#"
variant Key {
  Enter(),
  Escape,
  Tab,
  Char(ch: string),
}

function action(k: Key): string {
  return match (k) {
    Enter => "submit",
    Escape | Tab => "cancel",
    Char(ch) => "type:" + ch,
  };
}

console.log(action(Key.Enter()));
console.log(action(Key.Escape));
console.log(action(Key.Tab));
console.log(action(Key.Char("z")));
"#);
    assert_eq!(lines, vec!["submit", "cancel", "cancel", "type:z"]);
}

#[test]
fn runtime_match_guards_fall_through_top_to_bottom() {
    require_toolchain!();
    let lines = run(r#"
variant Score {
  Graded(points: number),
  Pending,
}

function grade(s: Score): string {
  return match (s) {
    Graded(points) if points >= 90 => "A",
    Graded(points) if points >= 80 => "B",
    Graded(points) => "F",
    Pending => "-",
  };
}

function tally(s: Score): number {
  return match (s) {
    Graded(points) if points > 0 => {
      const doubled = points * 2;
      return doubled;
    },
    _ => 0,
  };
}

console.log(grade(Score.Graded(95)));
console.log(grade(Score.Graded(85)));
console.log(grade(Score.Graded(10)));
console.log(grade(Score.Pending));
console.log(tally(Score.Graded(3)));
console.log(tally(Score.Graded(-1)));
"#);
    assert_eq!(lines, vec!["A", "B", "F", "-", "6", "0"]);
}

#[test]
fn runtime_generic_variant() {
    require_toolchain!();
    let lines = run(r#"
variant TOption<T> {
  Some(value: T),
  None,
}

function unwrapOr<T>(o: TOption<T>, fallback: T): T {
  return match (o) {
    Some(value) => value,
    None => fallback,
  };
}

console.log(unwrapOr(TOption.Some(7), 0));
console.log(unwrapOr<number>(TOption.None, 42));
"#);
    assert_eq!(lines, vec!["7", "42"]);
}

#[test]
fn runtime_async_match_with_await() {
    require_toolchain!();
    let lines = run(r#"
variant Job {
  Fetch(n: number),
  Idle,
}

async function double(n: number): Promise<number> {
  return n * 2;
}

async function runJob(j: Job): Promise<number> {
  return match (j) {
    Fetch(n) => await double(n),
    Idle => 0,
  };
}

runJob(Job.Fetch(21)).then((a) => {
  console.log(a);
  return runJob(Job.Idle);
}).then((b) => {
  console.log(b);
});
"#);
    assert_eq!(lines, vec!["42", "0"]);
}

#[test]
fn runtime_unexpected_case_throws() {
    require_toolchain!();
    // The emitted default branch is a plain runtime guard — it protects when
    // the type system was bypassed (e.g. data from the outside world).
    let lines = run(r#"
variant AB { A(n: number), B }
function f(x: AB): number {
  return match (x) {
    A(n) => n,
    B => 2,
  };
}
const g = f as unknown as (x: { kind: string }) => number;
try {
  g({ kind: "C" });
} catch (e) {
  console.log("threw: " + (e as Error).message);
}
"#);
    assert_eq!(
        lines,
        vec![r#"threw: tt match: unexpected case {"kind":"C"}"#]
    );
}

#[test]
fn runtime_plain_typescript_enum_coexists() {
    require_toolchain!();
    // TypeScript enum stays untouched while a unit-only tt variant lowers.
    let lines = run(r#"
enum Color { Red, Green, Blue }
variant Shape { Circle(radius: number), Point }

console.log(Color.Green);
console.log(Color[Color.Blue]);
console.log(JSON.stringify(Shape.Circle(1)));
"#);
    assert_eq!(lines, vec!["1", "Blue", r#"{"kind":"Circle","radius":1}"#]);
}

#[test]
fn runtime_std_option_result_functional_pipeline() {
    require_toolchain!();
    let lines = run_with_std(
        r#"
import type { TOption, TResult } from "./tt/index.js";
import * as Option from "./tt/option.js";
import * as Result from "./tt/result.js";

function parseNum(raw: string): TResult<number, string> {
  const n = Number(raw);
  return Number.isNaN(n) ? Result.Err("not a number: " + raw) : Result.Ok(n);
}

const half = (n: number): TOption<number> =>
  n % 2 === 0 ? Option.Some(n / 2) : Option.None;

const describe = (raw: string): string =>
  match (parseNum(raw)) {
    Ok(value) => match (half(value)) {
      Some(value: h) => "half=" + h,
      None => "odd:" + value,
    },
    Err(error) => "error:" + error,
  };

console.log(describe("42"));
console.log(describe("7"));
console.log(describe("x"));
console.log(Option.unwrapOr(Option.map(Option.fromNullable([1, 2].find((n) => n > 1)), (n) => n * 2), -1));
console.log(Result.unwrapOr(Result.andThen(parseNum("10"), (n): TResult<number, string> => n > 5 ? Result.Ok(n * 2) : Result.Err("small")), -1));
console.log(Result.isErr(Result.fromThrowable(() => JSON.parse("{"))));
"#,
    );
    assert_eq!(
        lines,
        vec![
            "half=21",
            "odd:7",
            "error:not a number: x",
            "4",
            "20",
            "true"
        ]
    );
}

#[test]
fn runtime_std_new_combinators() {
    require_toolchain!();
    let lines = run_with_std(
        r#"
import type { TOption, TResult } from "./tt/index.js";
import * as Option from "./tt/option.js";
import * as Result from "./tt/result.js";

console.log(JSON.stringify(Option.zip(Option.Some(1), Option.Some("a"))));
console.log(JSON.stringify(Option.zip(Option.Some(1), Option.None)));
console.log(JSON.stringify(Option.flatten(Option.Some(Option.Some(2)))));
console.log(JSON.stringify(Option.collect([Option.Some(1), Option.Some(2)])));
console.log(JSON.stringify(Option.collect([Option.Some(1), Option.None])));
console.log(JSON.stringify(Option.transpose(Option.Some(Result.Ok<number>(3)))));
console.log(JSON.stringify(Result.collect([Result.Ok(1), Result.Ok(2)])));
console.log(JSON.stringify(Result.collect<number, string>([Result.Ok(1), Result.Err("x")])));
console.log(JSON.stringify(Result.flatten<number, string>(Result.Ok(Result.Ok(4)))));
const nested: TResult<TOption<number>, string> = Result.Ok(Option.None);
console.log(JSON.stringify(Result.transpose(nested)));
Result.fromPromise(Promise.resolve(5))
  .then((r) => console.log(JSON.stringify(r)))
  .then(() => Result.fromPromise(Promise.reject("boom")))
  .then((r) => console.log(JSON.stringify(r)));
"#,
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Some","value":[1,"a"]}"#,
            r#"{"kind":"None"}"#,
            r#"{"kind":"Some","value":2}"#,
            r#"{"kind":"Some","value":[1,2]}"#,
            r#"{"kind":"None"}"#,
            r#"{"kind":"Ok","value":{"kind":"Some","value":3}}"#,
            r#"{"kind":"Ok","value":[1,2]}"#,
            r#"{"kind":"Err","error":"x"}"#,
            r#"{"kind":"Ok","value":4}"#,
            r#"{"kind":"None"}"#,
            r#"{"kind":"Ok","value":5}"#,
            r#"{"kind":"Err","error":"boom"}"#,
        ]
    );
}

#[test]
fn runtime_try_error_propagation() {
    require_toolchain!();
    let lines = run_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

function parseNum(raw: string): TResult<number, string> {
  const n = Number(raw);
  return Number.isNaN(n) ? Result.Err("not a number: " + raw) : Result.Ok(n);
}

function sumList(raws: string[]): TResult<number, string> {
  let total = 0;
  for (const raw of raws) {
    const n = try parseNum(raw);
    total += n;
  }
  return Result.Ok(total);
}

function checked(raw: string): TResult<number, string> {
  try parseNum(raw);
  let big: number = try parseNum(raw);
  return Result.Ok(big * 10);
}

function adjusted(raw: string): TResult<number, string> {
  return Result.Ok(Math.round(try parseNum(raw) * 1.1));
}

console.log(JSON.stringify(sumList(["1", "2", "3"])));
console.log(JSON.stringify(sumList(["1", "x"])));
console.log(JSON.stringify(checked("4")));
console.log(JSON.stringify(adjusted("5")));
console.log(JSON.stringify(adjusted("x")));
"#,
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":6}"#,
            r#"{"kind":"Err","error":"not a number: x"}"#,
            r#"{"kind":"Ok","value":40}"#,
            r#"{"kind":"Ok","value":6}"#,
            r#"{"kind":"Err","error":"not a number: x"}"#,
        ]
    );
}

#[test]
fn runtime_or_patterns_in_let_else_and_if_let() {
    require_toolchain!();
    // tsc --strict must accept both shapes: the let-else guard narrows the
    // temporary to the alternatives' union for the shared destructuring,
    // and the if-let disjunction narrows inside the then-block.
    let lines = run(r#"
variant Shape { Circle(r: number), Square(r: number), Dot }

function side(s: Shape): number {
  const Circle(r) | Square(r) = s else { return 0; };
  return r;
}

function tell(s: Shape): string {
  if let Circle(r) | Square(r) = s {
    return "sized " + r;
  } else {
    return "dot";
  }
}

console.log(side(Shape.Circle(3)));
console.log(side(Shape.Square(4)));
console.log(side(Shape.Dot));
console.log(tell(Shape.Square(5)));
console.log(tell(Shape.Dot));
"#);
    assert_eq!(lines, vec!["3", "4", "0", "sized 5", "dot"]);
}

#[test]
fn runtime_try_inside_an_if_let_body_propagates_from_the_function() {
    require_toolchain!();
    // The if-let body is inline in the enclosing function, so the `try`
    // propagates from `f` — not from any construct in between.
    let lines = run_with_std(
        r#"
import type { TOption, TResult } from "./tt/index.js";
import * as Option from "./tt/option.js";
import * as Result from "./tt/result.js";

function parseNum(raw: string): TResult<number, string> {
  const n = Number(raw);
  return Number.isNaN(n) ? Result.Err("not a number: " + raw) : Result.Ok(n);
}

function f(o: TOption<string>): TResult<number, string> {
  if let Some(value) = o {
    const n = try parseNum(value);
    return Result.Ok(n * 10);
  }
  return Result.Ok(-1);
}

console.log(JSON.stringify(f(Option.Some("7"))));
console.log(JSON.stringify(f(Option.Some("x"))));
console.log(JSON.stringify(f(Option.None)));
"#,
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":70}"#,
            r#"{"kind":"Err","error":"not a number: x"}"#,
            r#"{"kind":"Ok","value":-1}"#,
        ]
    );
}

#[test]
fn runtime_try_inside_a_closure_propagates_from_the_closure() {
    require_toolchain!();
    // Rust's `?` inside a closure: the `try` inside the arrow written in a
    // match scrutinee returns from the *arrow*, and the match sees the
    // Result it produced.
    let lines = run_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

function parseNum(raw: string): TResult<number, string> {
  const n = Number(raw);
  return Number.isNaN(n) ? Result.Err("not a number: " + raw) : Result.Ok(n);
}

function describe(raw: string): string {
  return match (((): TResult<number, string> => {
    const n = try parseNum(raw);
    return Result.Ok(n * 2);
  })()) {
    Ok(value) => "doubled: " + value,
    Err(error) => "failed: " + error,
  };
}

console.log(describe("21"));
console.log(describe("x"));
"#,
    );
    assert_eq!(lines, vec!["doubled: 42", "failed: not a number: x"]);
}

#[test]
fn runtime_let_else_narrows_and_diverges() {
    require_toolchain!();
    // tsc --strict must accept the emitted destructuring: the diverging
    // else block narrows the temporary to the matched case.
    let lines = run_with_std(
        r#"
import type { TOption, TResult } from "./tt/index.js";
import * as Option from "./tt/option.js";
import * as Result from "./tt/result.js";

function findUser(id: number): TOption<string> {
  return id === 1 ? Option.Some("amy") : Option.None;
}

function greet(id: number): string {
  const Some(value: user) = findUser(id) else { return "who?"; };
  return "hello, " + user;
}

function parseNum(raw: string): TResult<number, string> {
  const n = Number(raw);
  return Number.isNaN(n) ? Result.Err("bad") : Result.Ok(n);
}

function double(raw: string): number {
  const Ok(value) = parseNum(raw) else { return -1; };
  return value * 2;
}

console.log(greet(1));
console.log(greet(2));
console.log(double("21"));
console.log(double("x"));
"#,
    );
    assert_eq!(lines, vec!["hello, amy", "who?", "42", "-1"]);
}

#[test]
fn runtime_let_else_diverges_through_every_statement_form() {
    require_toolchain!();
    // TASK-172: the flow graph accepts a `switch`, a loop with no normal
    // exit, a `try`/`catch`, and a labeled `break` as diverging. Each
    // else block here really does leave the function on every path, so
    // the emitted narrowing must hold for `tsc --strict` and the values
    // must come out right at run time.
    let lines = run_with_std(
        r#"
import type { TOption } from "./tt/index.js";
import * as Option from "./tt/option.js";

function findUser(id: number): TOption<string> {
  return id === 1 ? Option.Some("amy") : Option.None;
}

// Every clause leaves, and a `default` catches what no case matched.
function bySwitch(id: number, kind: string): string {
  const Some(value: user) = findUser(id) else {
    switch (kind) {
      case "quiet": return "";
      default: return "who?";
    }
  };
  return "hello, " + user;
}

// A guarded block and its handler both leave.
function byTry(id: number): string {
  const Some(value: user) = findUser(id) else {
    try {
      return "missing " + id;
    } catch (e) {
      throw e;
    }
  };
  return "hello, " + user;
}

// Everything leaving normally runs the `finally` first.
function byFinally(id: number): string {
  const Some(value: user) = findUser(id) else {
    try {
      log("looking");
    } finally {
      return "gone";
    }
  };
  return "hello, " + user;
}

// A labeled `break` lands after the block, on the `return`.
function byLabel(id: number): string {
  const Some(value: user) = findUser(id) else {
    search: {
      if (id < 0) { break search; }
      return "unknown " + id;
    }
    return "negative";
  };
  return "hello, " + user;
}

// A loop with no normal exit is left only by `return`.
function byLoop(id: number): string {
  const Some(value: user) = findUser(id) else {
    while (true) {
      return "spun " + id;
    }
  };
  return "hello, " + user;
}

function log(_m: string): void {}

console.log(bySwitch(1, "loud"));
console.log(bySwitch(2, "loud"));
console.log(bySwitch(2, "quiet"));
console.log(byTry(1));
console.log(byTry(2));
console.log(byFinally(2));
console.log(byLabel(2));
console.log(byLabel(-1));
console.log(byLoop(2));
"#,
    );
    assert_eq!(
        lines,
        vec![
            "hello, amy",
            "who?",
            "",
            "hello, amy",
            "missing 2",
            "gone",
            "unknown 2",
            "negative",
            "spun 2",
        ]
    );
}

#[test]
fn runtime_let_else_diverges_through_an_inline_if_let() {
    require_toolchain!();
    // TASK-172: `if let` is the one tt construct that can carry a block's
    // divergence — its body and `else` are inline, so an exit written in
    // either leaves `classify`, not the construct. tsc --strict must
    // accept the narrowing that follows, and the values must come out
    // right at run time.
    let lines = run_with_std(
        r#"
import type { TOption, TResult } from "./tt/index.js";
import * as Option from "./tt/option.js";
import * as Result from "./tt/result.js";

function findUser(id: number): TOption<string> {
  return id === 1 ? Option.Some("amy") : Option.None;
}

function backup(id: number): TResult<string, string> {
  return id === 2 ? Result.Ok("bob") : Result.Err("none for " + id);
}

function classify(id: number): string {
  const Some(value: user) = findUser(id) else {
    if let Ok(value: fallback) = backup(id) {
      return "backup " + fallback;
    } else {
      return "nobody " + id;
    }
  };
  return "hello, " + user;
}

// A chained `else if let`, and a nested one in the then-half.
function chained(id: number): string {
  const Some(value: user) = findUser(id) else {
    if let Ok(value: fallback) = backup(id) {
      if let Some(value: again) = findUser(id) {
        return "both " + again;
      } else {
        return "backup " + fallback;
      }
    } else if let Err(error) = backup(id) {
      throw new Error(error);
    } else {
      return "unreachable";
    }
  };
  return "hello, " + user;
}

console.log(classify(1));
console.log(classify(2));
console.log(classify(3));
console.log(chained(2));
try {
  chained(3);
} catch (e) {
  console.log("threw " + (e as Error).message);
}
"#,
    );
    assert_eq!(
        lines,
        vec![
            "hello, amy",
            "backup bob",
            "nobody 3",
            "backup bob",
            "threw none for 3",
        ]
    );
}

#[test]
fn runtime_let_else_else_block_returns_an_object_literal() {
    require_toolchain!();
    // The natural shape for a `Result`-returning function: the else block
    // propagates an `Err` as an object literal. Its `}` ends no statement,
    // so the divergence check still sees a `return`.
    let lines = run_with_std(
        r#"
import type { TOption, TResult } from "./tt/index.js";
import * as Option from "./tt/option.js";
import * as Result from "./tt/result.js";

function findUser(id: number): TOption<string> {
  return id === 1 ? Option.Some("amy") : Option.None;
}

function greet(id: number): TResult<string, string> {
  const Some(value: user) = findUser(id) else { return { kind: "Err", error: "no user " + id }; };
  return { kind: "Ok", value: "hello, " + user };
}

console.log(JSON.stringify(greet(1)));
console.log(JSON.stringify(greet(2)));
"#,
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":"hello, amy"}"#,
            r#"{"kind":"Err","error":"no user 2"}"#,
        ]
    );
}

/* ------------------------------------------------------------------ */
/* the generated output is plain TypeScript: tsc accepts it            */
/* ------------------------------------------------------------------ */

#[test]
fn typecheck_exhaustive_match_passes() {
    require_toolchain!();
    let (ok, out) = typecheck(
        r#"
variant Shape { Circle(radius: number), Point }
const f = (s: Shape) => match (s) {
  Circle(radius) => radius,
  Point => 0,
};
"#,
    );
    assert!(ok, "{out}");
}

#[test]
fn typecheck_wildcard_makes_partial_match_exhaustive() {
    require_toolchain!();
    let (ok, out) = typecheck(
        r#"
variant Shape { Circle(radius: number), Rect(w: number, h: number), Point }
const f = (s: Shape) => match (s) {
  Circle(radius) => radius,
  _ => 0,
};
"#,
    );
    assert!(ok, "{out}");
}

#[test]
fn std_result_constructors_type_only_their_own_variant() {
    require_toolchain!();
    // `Ok` carries no error type and `Err` carries no success type, so each
    // constructor is typed by its own variant — and both still fit a
    // `TResult<T, E>` wherever one is expected.
    let (ok, out) = typecheck_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";
import type { TOk, TErr } from "./tt/index.js";

type Exact<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;

const ok = Result.Ok(123);
const err = Result.Err("bad");
const okIsOkOfNumber: Exact<typeof ok, TOk<number>> = true;
const errIsErrOfString: Exact<typeof err, TErr<string>> = true;

const fromOk: TResult<number, string> = Result.Ok(1);
const fromErr: TResult<number, string> = Result.Err("bad");

function parse(value: string): TResult<number, string> {
  if (value.length === 0) {
    return Result.Err("empty");
  }
  return Result.Ok(Number(value));
}

console.log(okIsOkOfNumber, errIsErrOfString, fromOk, fromErr, parse("1"));
"#,
    );
    assert!(ok, "tsc rejected variant-typed constructors:\n{out}");
}

#[test]
fn try_error_types_infer_as_a_union_without_an_annotation() {
    require_toolchain!();
    // Two `try`s over results with different error types: the lowered early
    // returns plus `Result.Ok(...)` give tsc `TErr<UserError> | TErr<ConfigError>
    // | TOk<Data>`, which is exactly `TResult<Data, UserError | ConfigError>`.
    // ttc collects no error types of its own — this is tsc's union inference.
    let (ok, out) = typecheck_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

type User = { id: number };
type Config = { port: number };
type UserError = { tag: "user" };
type ConfigError = { tag: "config" };

declare function getUser(): TResult<User, UserError>;
declare function getConfig(): TResult<Config, ConfigError>;

function load() {
  const user = try getUser();
  const config = try getConfig();
  return Result.Ok({ user, config });
}

const loaded: TResult<{ user: User; config: Config }, UserError | ConfigError> = load();
console.log(loaded);
"#,
    );
    assert!(ok, "tsc lost the try error union:\n{out}");
}

#[test]
fn try_error_union_stays_checked_against_the_declared_return_type() {
    require_toolchain!();
    // The inference above is not a hole: an annotated function whose `Err`
    // type does not cover a propagated error is still a type error, reported
    // by tsc on the emitted early return.
    let (ok, out) = typecheck_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

declare function getUser(): TResult<number, { tag: "user" }>;

function load(): TResult<number, string> {
  const user = try getUser();
  return Result.Ok(user);
}

console.log(load());
"#,
    );
    assert!(!ok, "tsc accepted an uncovered error type:\n{out}");
}

/// Declarations shared by the `andThen` error-union tests: four steps, each
/// failing its own way, so a chain that loses an error type is visible in the
/// asserted union.
const ERROR_UNION_PRELUDE: &str = r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

type Exact<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;

type User = { id: number };
type Company = { name: string };
type Profile = { title: string };
type ConfigError = { tag: "config" };
type TokenError = { tag: "token" };
type FetchError = { tag: "fetch" };
type ValidationError = { tag: "validation" };

declare function loadConfig(): TResult<string, ConfigError>;
declare function loadToken(config: string): TResult<User, TokenError>;
declare function getCompany(user: User): TResult<Company, FetchError>;
declare function fetchProfile(user: User): TResult<Profile, FetchError>;
declare function validateProfile(profile: Profile): TResult<Profile, ValidationError>;
"#;

#[test]
fn std_result_and_then_unions_the_two_error_types() {
    require_toolchain!();
    // `andThen` takes the chained function's error type as its own generic,
    // so chaining a `TResult<User, TokenError>` with a step that fails with
    // `FetchError` keeps both — no `mapErr` to a common type first.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
declare const first: TResult<User, TokenError>;

const chained = Result.andThen(first, (user) => getCompany(user));
const exact: Exact<typeof chained, TResult<Company, TokenError | FetchError>> = true;

console.log(chained, exact);
"#
    ));
    assert!(ok, "andThen lost an error type:\n{out}");
}

#[test]
fn std_result_and_then_on_a_variant_typed_value_keeps_the_chained_error() {
    require_toolchain!();
    // A value typed as the `TOk<T>` variant alone (what `Result.Ok(...)` and a
    // never-failing function give) offers nothing to infer the incoming `E`
    // from. The `E = never` default is what keeps that case precise instead of
    // collapsing the union to `unknown`.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
const chained = Result.andThen(Result.Ok({{ id: 1 }}), (user: User) => fetchProfile(user));
const exact: Exact<typeof chained, TResult<Profile, FetchError>> = true;

console.log(chained, exact);
"#
    ));
    assert!(
        ok,
        "andThen on an Ok value lost the chained error type:\n{out}"
    );
}

#[test]
fn std_result_and_then_p_accumulates_error_types_along_a_pipeline() {
    require_toolchain!();
    // The end-to-end shape from the design: `try` collects two error types
    // into the function's inferred return type, and every `andThenP` step
    // adds its own. ttc collects nothing — this is tsc's union inference.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
function loadUser() {{
  const config = try loadConfig();
  const token = try loadToken(config);
  return Result.Ok(token);
}}

const profile = loadUser()
  |> Result.andThenP(fetchProfile)
  |> Result.andThenP(validateProfile);

const exact: Exact<
  typeof profile,
  TResult<Profile, ConfigError | TokenError | FetchError | ValidationError>
> = true;

console.log(profile, exact);
"#
    ));
    assert!(ok, "the pipeline lost an error type:\n{out}");
}

#[test]
fn std_result_map_p_keeps_the_error_type_it_was_given() {
    require_toolchain!();
    // `map`/`mapP` add no failure of their own, so they carry `E` through
    // unchanged — including a union an earlier `andThenP` accumulated.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
declare const first: TResult<User, TokenError>;

const title = first
  |> Result.andThenP(fetchProfile)
  |> Result.mapP((profile) => profile.title);

const exact: Exact<typeof title, TResult<string, TokenError | FetchError>> = true;

console.log(title, exact);
"#
    ));
    assert!(ok, "mapP changed the error type:\n{out}");
}

#[test]
fn std_result_and_then_p_composes_under_flow() {
    require_toolchain!();
    // `andThenP` returns a function still generic in `E`, so a `flow`
    // composition of two steps stays open at its input end: applying it to a
    // `TResult<User, TokenError>` unions that error in too.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
declare const first: TResult<User, TokenError>;

const pipeline = flow
  |> Result.andThenP(fetchProfile)
  |> Result.andThenP(validateProfile);

const profile = pipeline(first);
const exact: Exact<
  typeof profile,
  TResult<Profile, TokenError | FetchError | ValidationError>
> = true;

console.log(profile, exact);
"#
    ));
    assert!(ok, "flow composition lost an error type:\n{out}");
}

#[test]
fn std_result_and_then_p_takes_an_annotated_inline_callback() {
    require_toolchain!();
    // The curried form reads `T` off the chained function, so an inline
    // callback carries its own parameter annotation. A named function (every
    // other test here) needs nothing.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
declare const first: TResult<User, TokenError>;

const profile = first |> Result.andThenP((user: User) => fetchProfile(user));
const exact: Exact<typeof profile, TResult<Profile, TokenError | FetchError>> = true;

console.log(profile, exact);
"#
    ));
    assert!(ok, "an annotated inline callback did not typecheck:\n{out}");
}

#[test]
fn std_result_block_output_pipes_into_and_then_p() {
    require_toolchain!();
    // A `result` block infers the same shape a `try` function does — one `Ok`
    // arm plus one `Err` arm per binding — so its value chains on with its
    // error types intact.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
const user = result {{
  const config = try loadConfig();
  const loaded = try loadToken(config);
  return loaded;
}};

const profile = user |> Result.andThenP(fetchProfile);
const exact: Exact<
  typeof profile,
  TResult<Profile, ConfigError | TokenError | FetchError>
> = true;

console.log(profile, exact);
"#
    ));
    assert!(
        ok,
        "a result block lost its error types in a pipeline:\n{out}"
    );
}

#[test]
fn std_result_and_then_error_union_stays_checked_against_an_annotation() {
    require_toolchain!();
    // Accumulating errors is not a hole either: a declared return type that
    // covers only one of the two chained error types is still a tsc error.
    let (ok, out) = typecheck_with_std(&format!(
        r#"{ERROR_UNION_PRELUDE}
declare const first: TResult<User, TokenError>;

function chain(): TResult<Profile, TokenError> {{
  return Result.andThen(first, (user) => fetchProfile(user));
}}

console.log(chain());
"#
    ));
    assert!(
        !ok,
        "tsc accepted a return type missing an error case:\n{out}"
    );
}

#[test]
fn runtime_result_and_then_chain_short_circuits_on_the_first_err() {
    require_toolchain!();
    // The types changed; the emitted values did not. Both spellings still
    // return the first `Err` untouched and run the rest only on `Ok`.
    let lines = run_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

type Parsed = { n: number };
type ParseError = { tag: "parse"; raw: string };
type RangeError = { tag: "range"; n: number };

const parse = (raw: string): TResult<Parsed, ParseError> =>
  Number.isNaN(Number(raw))
    ? Result.Err({ tag: "parse" as const, raw })
    : Result.Ok({ n: Number(raw) });

const inRange = (p: Parsed): TResult<number, RangeError> =>
  p.n <= 10 ? Result.Ok(p.n) : Result.Err({ tag: "range" as const, n: p.n });

const check = (raw: string) => parse(raw) |> Result.andThenP(inRange);

console.log(JSON.stringify(check("4")));
console.log(JSON.stringify(check("40")));
console.log(JSON.stringify(check("x")));
console.log(JSON.stringify(Result.andThen(parse("4"), inRange)));
console.log(JSON.stringify(Result.andThen(parse("x"), inRange)));
"#,
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":4}"#,
            r#"{"kind":"Err","error":{"tag":"range","n":40}}"#,
            r#"{"kind":"Err","error":{"tag":"parse","raw":"x"}}"#,
            r#"{"kind":"Ok","value":4}"#,
            r#"{"kind":"Err","error":{"tag":"parse","raw":"x"}}"#,
        ]
    );
}

#[test]
fn typecheck_match_on_handwritten_discriminated_union() {
    require_toolchain!();
    let (ok, out) = typecheck(
        r#"
type AppEvent =
  | { kind: "click"; x: number; y: number }
  | { kind: "key"; code: string };
const f = (e: AppEvent) => match (e) {
  click(x, y) => x + y,
  key(code) => code.length,
};
"#,
    );
    assert!(ok, "{out}");
}

/* ------------------------------------------------------------------ */
/* import specifier rewriting                                          */
/* ------------------------------------------------------------------ */

const ERROR_TT: &str = "export variant CalcError { DivByZero, Overflow(limit: number) }\n";
const MAIN_TT: &str = r#"import { CalcError } from "./error.tt";
const e = CalcError.Overflow(9);
const msg = match (e) {
  Overflow(limit) => `over ${limit}`,
  _ => "other",
};
console.log(msg);
export {};
"#;

#[test]
fn cross_file_tt_import_typechecks_and_runs() {
    require_toolchain!();
    let dir = tmpdir();
    let error_ts = compile(ERROR_TT, &Options::default()).expect("tt compile failed");
    let main_ts = compile(MAIN_TT, &Options::default()).expect("tt compile failed");
    assert!(main_ts.contains("\"./error.js\""), "{main_ts}");
    fs::write(dir.join("error.ts"), &error_ts).unwrap();
    fs::write(dir.join("main.ts"), &main_ts).unwrap();
    fs::write(dir.join("package.json"), "{ \"type\": \"module\" }\n").unwrap();
    let out = Command::new("tsc")
        .arg(dir.join("main.ts"))
        .arg("--outDir")
        .arg(&dir)
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "tsc failed:\n{}\n---main.ts---\n{main_ts}",
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
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "over 9");
}

/* ------------------------------------------------------------------ */
/* project-wide exhaustiveness through the CLI                         */
/* ------------------------------------------------------------------ */

const TOKEN_TT: &str =
    "export variant Token {\n  Num(value: number),\n  Ident(name: string),\n  Eof,\n}\n";

/// Runs the ttc binary itself — declaration collection across files lives
/// in the CLI, not in `compile`. No tsc/node needed.
fn run_ttc(dir: &std::path::Path, args: &[&str]) -> (bool, String) {
    run_ttc_env(dir, args)
}

fn run_ttc_env(dir: &std::path::Path, args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to run ttc");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Whether ttc can resolve a TypeScript to drive *and* emit declarations
/// with it. Asked by running `--types` over a trivial project and looking
/// for the sidecar: the answer is ttc's own resolution, not a guess about
/// the machine. A released `typescript@7` can check but not emit, so it
/// answers `false` here and the `--types` success tests skip.
fn usable_typescript_for_types() -> bool {
    let dir = project_dir();
    fs::write(dir.join("probe.tt"), "export const n: number = 1;\n").unwrap();
    let (ok, _) = run_ttc(&dir, &["--types", "probe.tt", "-o", "."]);
    ok && dir.join("probe.tt.d.ts").exists()
}

/// Skip a `--types` success test when no TypeScript that can emit
/// declarations is reachable.
macro_rules! require_types_typescript {
    () => {
        if !usable_typescript_for_types() {
            eprintln!("skipping: no TypeScript for ttc to drive, or it cannot emit declarations");
            return;
        }
    };
}

#[test]
fn cli_checks_exhaustiveness_across_tt_imports() {
    let dir = tmpdir();
    fs::write(dir.join("token.tt"), TOKEN_TT).unwrap();
    fs::write(
        dir.join("parser.tt"),
        "import { Token } from \"./token.tt\";\nconst show = (t: Token) =>\n  match (t) {\n    Num(value) => value,\n    Ident(name) => 0,\n  };\n",
    )
    .unwrap();
    let (ok, err) = run_ttc(&dir, &["--check", "parser.tt"]);
    assert!(!ok, "expected failure:\n{err}");
    // The rendered form: the rule and its message on the header line, the
    // file and position on the location line, the construct underlined.
    assert!(
        err.contains(
            "error[match-not-exhaustive]: match on variant Token (imported from \"./token.tt\") \
             is not exhaustive: missing \"Eof\""
        ),
        "{err}"
    );
    assert!(err.contains("--> parser.tt:3:3"), "{err}");
    assert!(err.contains("3 |   match (t) {"), "{err}");
    assert!(err.contains("  |   ^^^^^^^^^"), "{err}");

    fs::write(
        dir.join("parser.tt"),
        "import { Token } from \"./token.tt\";\nconst show = (t: Token) =>\n  match (t) {\n    Num(value) => value,\n    Ident(name) => 0,\n    Eof => -1,\n  };\n",
    )
    .unwrap();
    let (ok, err) = run_ttc(&dir, &["--check", "parser.tt"]);
    assert!(ok, "expected success:\n{err}");
}

#[test]
fn untyped_cli_does_not_infer_imported_field_ownership() {
    let dir = tmpdir();
    fs::write(
        dir.join("domain.tt"),
        "export variant PaymentMethod { Card(brand: string, last4: string) }\n",
    )
    .unwrap();
    fs::write(
        dir.join("payment.tt"),
        "import { PaymentMethod } from \"./domain.tt\";\n\
         export function brand(method: PaymentMethod): string {\n\
         \x20 return match (method) { Card(brnad) => brnad, _ => \"n/a\" };\n\
         }\n",
    )
    .unwrap();

    let (ok, err) = run_ttc(&dir, &["--check", "payment.tt"]);
    assert!(ok, "the typed checker owns imported field identity:\n{err}");
}

#[test]
fn untyped_cli_does_not_infer_a_single_imported_case_owner() {
    let dir = tmpdir();
    fs::write(
        dir.join("domain.tt"),
        "export variant PaymentMethod { Card(brand: string), BankTransfer(iban: string) }\n",
    )
    .unwrap();
    fs::write(
        dir.join("payment.tt"),
        "import { PaymentMethod } from \"./domain.tt\";\n\
         export function fee(method: PaymentMethod): number {\n\
         \x20 return match (method) { Crad(brand) => 1, _ => 0 };\n\
         }\n",
    )
    .unwrap();

    let (ok, err) = run_ttc(&dir, &["--check", "payment.tt"]);
    assert!(
        ok,
        "the typed checker owns the scrutinee's case domain:\n{err}"
    );
}

#[test]
fn untyped_cli_does_not_infer_a_generic_payload_owner() {
    let dir = tmpdir();
    fs::write(
        dir.join("domain.tt"),
        "export variant PaymentMethod { Card(brand: string), Cash }\n",
    )
    .unwrap();
    fs::write(
        dir.join("nested.tt"),
        "import type { TResult } from \"@tt/std\";\n\
         import { PaymentMethod } from \"./domain.tt\";\n\
         export function brand(r: TResult<PaymentMethod, string>): string {\n\
         \x20 return match (r) {\n\
         \x20   Ok(value: Card(brnd)) => brnd,\n\
         \x20   Ok(value) => \"other\",\n\
         \x20   Err(error) => \"error\",\n\
         \x20 };\n\
         }\n",
    )
    .unwrap();

    let (ok, err) = run_ttc(&dir, &["--check", "nested.tt"]);
    assert!(
        ok,
        "generic substitution belongs to the typed checker:\n{err}"
    );
}

#[test]
fn cli_skips_unresolvable_imports_silently() {
    // A missing module is tsc's problem (TS2307); the match simply stays
    // unchecked, as before phase 2.
    let dir = tmpdir();
    fs::write(
        dir.join("main.tt"),
        "import { Gone } from \"./missing.tt\";\nconst x = match (g) { A(v) => v, B => 0 };\n",
    )
    .unwrap();
    let (ok, err) = run_ttc(&dir, &["--check", "main.tt"]);
    assert!(ok, "expected success:\n{err}");
}

#[test]
fn cli_cross_file_match_runs_end_to_end() {
    require_toolchain!();
    let dir = tmpdir();
    fs::write(dir.join("token.tt"), TOKEN_TT).unwrap();
    fs::write(
        dir.join("main.tt"),
        "import { Token } from \"./token.tt\";\nconst t = Token.Ident(\"x\");\nconsole.log(match (t) {\n  Num(value) => `n${value}`,\n  Ident(name) => `i${name}`,\n  Eof => \"eof\",\n});\nexport {};\n",
    )
    .unwrap();
    let (ok, err) = run_ttc(&dir, &["token.tt", "main.tt"]);
    assert!(ok, "ttc failed:\n{err}");
    fs::write(dir.join("package.json"), "{ \"type\": \"module\" }\n").unwrap();
    let out = Command::new("tsc")
        .arg(dir.join("main.ts"))
        .arg("--outDir")
        .arg(&dir)
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "tsc failed:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = Command::new("node")
        .arg(dir.join("main.js"))
        .output()
        .expect("failed to run node");
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ix");
}

/* ------------------------------------------------------------------ */
/* symbol interface (--symbols)                                        */
/* ------------------------------------------------------------------ */

#[test]
fn symbols_reports_imports_and_positions_as_valid_json() {
    let dir = tmpdir();
    fs::write(dir.join("token.tt"), TOKEN_TT).unwrap();
    fs::write(
        dir.join("parser.tt"),
        "import { Token as Tok } from \"./token.tt\";\nimport { Gone } from \"./missing.tt\";\nvariant Local { A(x: number) }\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .current_dir(&dir)
        .args(["--symbols", "parser.tt"])
        .output()
        .expect("failed to run ttc");
    assert!(out.status.success());
    let json = String::from_utf8_lossy(&out.stdout).into_owned();

    // Shape: the local variant with its position, the resolved import with the
    // referenced file's exported declarations, and the unresolvable import
    // marked null.
    assert!(json.contains("\"file\":\"parser.tt\""), "{json}");
    assert!(json.contains("\"variants\":["), "{json}");
    assert!(!json.contains("\"enums\""), "{json}");
    assert!(json.contains("\"name\":\"Local\""), "{json}");
    assert!(
        json.contains("\"entries\":[{\"name\":\"Token\",\"alias\":\"Tok\"}]"),
        "{json}"
    );
    assert!(
        json.contains(
            "\"name\":\"Token\",\"exported\":true,\"generics\":\"\",\"line\":1,\"col\":16"
        ),
        "{json}"
    );
    assert!(
        json.contains("\"tag\":\"Eof\",\"line\":4,\"col\":3,\"fields\":null"),
        "{json}"
    );
    assert!(json.contains("\"specifier\":\"./missing.tt\""), "{json}");
    assert!(json.contains("\"resolved\":null,\"variants\":[]"), "{json}");

    // And it must be JSON a real parser accepts.
    if have("node") {
        let mut child = Command::new("node")
            .args([
                "-e",
                "let d='';process.stdin.on('data',c=>d+=c).on('end',()=>JSON.parse(d))",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("failed to run node");
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();
        assert!(child.wait().unwrap().success(), "not valid JSON:\n{json}");
    }
}

/* ------------------------------------------------------------------ */
/* the unified pipeline through the CLI: build and --types             */
/* ------------------------------------------------------------------ */

const LEVEL_TT: &str = "export variant Level {\n  Low,\n  High(threshold: number),\n}\n";

const NOTICE_TT: &str = "import type { TOption } from \"@tt/std\";\nimport * as Option from \"@tt/std/option\";\nimport { Level } from \"./level.tt\";\n\nexport variant Notice {\n  Info(text: string),\n  Warn(text: string, code: number),\n}\n\nexport function render(n: Notice): string {\n  return match (n) {\n    Info(text) => `info: ${text}`,\n    Warn(text, code) => `warn[${code}]: ${text}`,\n  };\n}\n\nexport function gate(l: Level): number {\n  return match (l) {\n    Low => 0,\n    High(threshold) => threshold,\n  };\n}\n\nexport function first(list: Notice[]): TOption<Notice> {\n  return list.length > 0 ? Option.Some(list[0]) : Option.None;\n}\n";

const CONSUMER_MAIN_TS: &str = "import * as Option from \"@tt/std/option\";\nimport { Notice, render, first } from \"./notice.tt\";\n\nconst items = [Notice.Info(\"hello\"), Notice.Warn(\"careful\", 7)];\nfor (const n of items) console.log(render(n));\nconsole.log(Option.isSome(first(items)));\n";

/// A mixed source tree: two `.tt` modules (one importing the other and the
/// standard library) plus a hand-written `.ts` entry that imports `.tt`.
/// Every file under `dir`, recursively.
fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // node_modules is the linked TypeScript, not project output.
        if path.file_name().is_some_and(|name| name == "node_modules") {
            continue;
        }
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}

fn write_consumer_tree(dir: &std::path::Path) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/level.tt"), LEVEL_TT).unwrap();
    fs::write(dir.join("src/notice.tt"), NOTICE_TT).unwrap();
    fs::write(dir.join("src/main.ts"), CONSUMER_MAIN_TS).unwrap();
}

#[test]
fn cli_build_emits_a_complete_tree_that_runs() {
    require_toolchain!();
    let dir = tmpdir();
    write_consumer_tree(&dir);

    let (ok, err) = run_ttc(&dir, &["-o", "build", "--no-banner", "src"]);
    assert!(ok, "build failed:\n{err}");

    // Hand-written TypeScript rides along byte-for-byte except for its
    // relative `.tt` (and `@tt/std`) specifiers.
    let main_ts = fs::read_to_string(dir.join("build/main.ts")).unwrap();
    assert_eq!(
        main_ts,
        CONSUMER_MAIN_TS
            .replace("./notice.tt", "./notice.js")
            .replace("@tt/std/option", "./tt/option.js")
    );
    for module in ttc::StdModule::STANDARD {
        assert!(dir.join("build/tt").join(module.file_name()).exists());
    }

    // The emitted tree stands on its own: tsc compiles it, node runs it.
    fs::write(dir.join("build/package.json"), "{ \"type\": \"module\" }\n").unwrap();
    let out = Command::new("tsc")
        .current_dir(&dir)
        .args(["build/main.ts", "--outDir", "build"])
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "tsc failed:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = Command::new("node")
        .current_dir(&dir)
        .arg("build/main.js")
        .output()
        .expect("failed to run node");
    assert!(
        out.status.success(),
        "node failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        ["info: hello", "warn[7]: careful", "true"]
    );
}

#[test]
fn cli_refuses_to_overwrite_a_pass_through_input() {
    let dir = tmpdir();
    fs::write(dir.join("main.ts"), "export const x = 1;\n").unwrap();

    // In place, a pass-through `.ts` would land on top of itself.
    let (ok, err) = run_ttc(&dir, &["main.ts"]);
    assert!(!ok, "expected failure:\n{err}");
    assert!(err.contains("output would overwrite the input"), "{err}");
    let untouched = fs::read_to_string(dir.join("main.ts")).unwrap();
    assert_eq!(untouched, "export const x = 1;\n");

    // A separate output tree is fine.
    let (ok, err) = run_ttc(&dir, &["-o", "out", "main.ts"]);
    assert!(ok, "build failed:\n{err}");
}

#[test]
fn cli_types_leaves_nothing_but_the_sidecars() {
    require_toolchain!();
    require_types_typescript!();
    let dir = project_dir();
    write_consumer_tree(&dir);

    let (ok, err) = run_ttc(&dir, &["--types", "src"]);
    assert!(ok, "--types failed:\n{err}");

    // Declaration emit runs in memory: no cache tree, and above all no
    // copy of the hand-written TypeScript anywhere.
    assert!(!dir.join(".tt-build").exists(), "a cache tree was created");
    let copies: Vec<String> = walk(&dir)
        .into_iter()
        .filter(|path| {
            path.file_name().is_some_and(|name| name == "main.ts")
                && !path.starts_with(dir.join("src"))
        })
        .map(|path| path.display().to_string())
        .collect();
    assert!(
        copies.is_empty(),
        "hand-written source was copied: {copies:?}"
    );

    // What it does leave: one sidecar pair per .tt, plus the std types.
    assert!(dir.join(".tt-types/notice.tt.d.ts").exists());
    assert!(dir.join(".tt-types/notice.tt.d.ts.map").exists());
    assert!(dir.join(".tt-types/level.tt.d.ts").exists());
    for module in ttc::StdModule::STANDARD {
        assert!(
            dir.join(".tt-types/tt")
                .join(module.file_name())
                .with_extension("d.ts")
                .exists()
        );
    }
}

#[test]
fn cli_types_reports_type_errors_but_keeps_the_sidecars_fresh() {
    require_toolchain!();
    require_types_typescript!();
    let dir = project_dir();
    write_consumer_tree(&dir);
    // A type error in the consumer, not a tt-level one: declarations are
    // still emitted, so the sidecars must be written and the run must fail.
    fs::write(
        dir.join("src/main.ts"),
        format!("{CONSUMER_MAIN_TS}\nconst wrong: number = \"text\";\n"),
    )
    .unwrap();

    let (ok, err) = run_ttc(&dir, &["--types", "src"]);
    assert!(!ok, "expected a failing exit code:\n{err}");
    assert!(
        err.contains("main.ts"),
        "diagnostic should name the file: {err}"
    );
    assert!(
        dir.join(".tt-types/notice.tt.d.ts").exists(),
        "sidecars should still be written: {err}"
    );
}

#[test]
fn cli_types_reports_tt_type_errors_at_the_source_position() {
    require_toolchain!();
    require_types_typescript!();
    let dir = project_dir();
    write_consumer_tree(&dir);
    // A type error *inside* tt syntax. The emitted TypeScript is a switch
    // IIFE that moves the offending expression far from where it was
    // written, and the file it lives in is never written to disk — the
    // diagnostic has to name `bad.tt` and the source line/column anyway.
    let bad = "import type { TResult } from \"@tt/std\";\n\
               import * as Result from \"@tt/std/result\";\n\
               \n\
               declare function evaluate(): TResult<number, string>;\n\
               \n\
               export const bad = evaluate() |> Result.mapP((n) => n.length);\n";
    fs::write(dir.join("src/bad.tt"), bad).unwrap();

    let (ok, err) = run_ttc(&dir, &["--types", "src"]);
    assert!(!ok, "expected a failing exit code:\n{err}");

    // `length` sits at column 55 of line 5 of the source. The emitted code
    // puts it elsewhere entirely, and there is no `bad.ts` to open. The
    // message and the position have to belong to the *same* diagnostic, so
    // this reads the rendered block rather than two independent lines.
    let reported = err
        .split("error[")
        .find(|block| block.contains("does not exist on type"))
        .unwrap_or_else(|| panic!("no type error reported:\n{err}"));
    assert!(
        reported.contains("--> src/bad.tt:6:55"),
        "diagnostic should point into the .tt source: {reported}"
    );
    assert!(
        !err.contains("bad.ts"),
        "named a file that does not exist: {err}"
    );
}

#[test]
fn cli_types_without_typescript_says_so() {
    require_toolchain!();
    let dir = tmpdir();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/level.tt"), LEVEL_TT).unwrap();
    // No TypeScript on purpose: a project's TypeScript comes from its own
    // `node_modules` and nowhere else, and a temporary directory has none
    // above it. So this runs everywhere, rather than skipping on any
    // machine that happens to have a compiler installed somewhere.
    let (ok, err) = run_ttc_env(&dir, &["--types", "src"]);
    assert!(!ok, "expected failure:\n{err}");
    assert!(err.contains("no TypeScript compiler found"), "{err}");
}

#[test]
fn cli_types_sidecars_typecheck_the_source_tree() {
    require_toolchain!();
    require_types_typescript!();
    let dir = project_dir();
    write_consumer_tree(&dir);

    let (ok, err) = run_ttc(&dir, &["--types", "src"]);
    assert!(ok, "--types failed:\n{err}");

    // The declarations keep the *source* specifiers — that is what resolves
    // in the consumer's merged view.
    let sidecar = fs::read_to_string(dir.join(".tt-types/notice.tt.d.ts")).unwrap();
    assert!(sidecar.contains("from \"@tt/std\""), "{sidecar}");
    assert!(sidecar.contains("from \"./level.tt\""), "{sidecar}");
    assert!(
        sidecar.contains("export declare function render"),
        "{sidecar}"
    );
    assert!(dir.join(".tt-types/notice.tt.d.ts.map").exists());
    assert!(dir.join(".tt-types/level.tt.d.ts").exists());
    for module in ttc::StdModule::STANDARD {
        assert!(
            dir.join(".tt-types/tt")
                .join(module.file_name())
                .with_extension("d.ts")
                .exists(),
            "std declaration missing: {:?}",
            module
        );
    }

    // Round trip: the untouched source tree typechecks once the sidecars
    // are merged in (`rootDirs`) and `@tt/std` is mapped (`paths`).
    fs::write(
        dir.join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "target": "es2022",
    "module": "preserve",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true,
    "rootDirs": ["./src", "./.tt-types"],
    "paths": {
      "@tt/std": ["./.tt-types/tt/index.d.ts"],
      "@tt/std/*": ["./.tt-types/tt/*.d.ts"]
    }
  },
  "include": ["src"]
}
"#,
    )
    .unwrap();
    let out = Command::new("tsc")
        .current_dir(&dir)
        .args(["-p", "tsconfig.json"])
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "consumer typecheck failed:\n{}\n---sidecar---\n{sidecar}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/* ------------------------------------------------------------------ */
/* pipeline                                                            */
/* ------------------------------------------------------------------ */

// Inline the curried std combinators so the snippets need no module
// resolution (the std source itself is covered by tests/stdlib.rs).
const PIPE_PRELUDE: &str = r#"
type TOption<T> = { kind: "Some"; value: T } | { kind: "None" };
const Option = {
  Some: <T>(value: T): TOption<T> => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
  mapP:
    <T, U>(f: (value: T) => U) =>
    (o: TOption<T>): TOption<U> =>
      o.kind === "Some" ? { kind: "Some", value: f(o.value) } : { kind: "None" },
  unwrapOrP:
    <T>(fallback: T) =>
    (o: TOption<T>): T =>
      o.kind === "Some" ? o.value : fallback,
};
const half = (n: number): TOption<number> =>
  n % 2 === 0 ? Option.Some(n / 2) : Option.None;
"#;

#[test]
fn pipeline_curried_combinator_steps_infer_without_annotations() {
    require_toolchain!();
    // The whole point of the $tt_ap emission: `x` in the curried step must
    // infer as number (a direct-application emission collapses it to
    // `unknown` — TS18046).
    let (ok, out) = typecheck(&format!(
        "{PIPE_PRELUDE}\nconst label: string = half(4) |> Option.mapP(x => x + 1) |> Option.unwrapOrP(0) |> .toFixed(1);\n"
    ));
    assert!(ok, "{out}");
}

#[test]
fn pipeline_generic_user_functions_instantiate() {
    require_toolchain!();
    // Composing generic functions is where pipe() libraries lose inference;
    // step-by-step application must keep it.
    let (ok, out) = typecheck(
        "const wrap = <T,>(v: T): T[] => [v];\nconst arr: number[][] = 3 |> wrap |> wrap;\n",
    );
    assert!(ok, "{out}");
}

#[test]
fn pipeline_files_import_one_shared_runtime() {
    require_toolchain!();
    let dir = tmpdir();
    let mut files = Vec::new();
    for suffix in ["a", "b"] {
        let source = format!(
            "declare function input_{suffix}(): number;\n\
             declare const step_{suffix}: (value: number) => number;\n\
             const value_{suffix} = input_{suffix}() |> step_{suffix};\n\
             const flow_{suffix} = flow |> step_{suffix} |> step_{suffix};\n"
        );
        let code =
            compile(&source, &options_with_runtime("./runtime.js")).expect("tt compile failed");
        assert!(!code.lines().any(|line| line.starts_with("function $tt_")));
        assert!(!code.lines().any(|line| line.starts_with("var $tt_")));
        assert!(code.contains("from \"./runtime.js\""));
        let file = dir.join(format!("{suffix}.ts"));
        fs::write(&file, code).unwrap();
        files.push(file);
    }
    write_runtime(&dir);
    files.push(dir.join("runtime.ts"));

    let out = Command::new("tsc")
        .args(&files)
        .arg("--noEmit")
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn pipeline_type_error_in_a_step_is_reported_on_user_text() {
    require_toolchain!();
    // A step that is not a unary function is the user's type error — tsc
    // must reject it (ttc emits it untouched).
    let (ok, out) = typecheck("const n: number = 1 |> ((a: string) => a.length);\n");
    assert!(!ok, "{out}");
}

#[test]
fn a_direct_pipeline_call_keeps_contextual_typing() {
    require_toolchain!();
    let (ok, out) = typecheck("const value: number = 1 |> (x => x + 1);\n");
    assert!(ok, "{out}");
}

#[test]
fn pipeline_runs_left_to_right() {
    require_toolchain!();
    let lines = run(r#"
const order: string[] = [];
const tap = <T,>(name: string) => (v: T): T => { order.push(name); return v; };
const out = (order.push("head"), 10) |> tap("s1") |> .toFixed(0) |> tap("s2");
console.log(order.join(","), out);
"#);
    assert_eq!(lines, ["head,s1,s2 10"]);
}

#[test]
fn optional_postfix_preserves_short_circuit_order_and_method_receiver() {
    require_toolchain!();
    let lines = run(r#"
const order: string[] = [];
const mark = (name: string, value: number): number => { order.push(name); return value; };
const key = (): "method" => { order.push("key"); return "method"; };
const live = {
  base: 10,
  method(value: number): number {
    order.push(this === live ? "this" : "lost-this");
    return this.base + value;
  },
};
const absent = (() => undefined as typeof live | undefined)();
const hit = (order.push("head-hit"), live) |> ?.[key()]?.(mark("arg", 2));
const miss = (order.push("head-miss"), absent) |> ?.[key()]?.(mark("skipped", 3));
const after = miss |> (value => { order.push("after"); return value ?? -1; });
console.log(hit, miss, after, order.join(","));
"#);
    assert_eq!(
        lines,
        ["12 undefined -1 head-hit,key,arg,this,head-miss,after"],
        "{lines:?}"
    );
}

#[test]
fn optional_postfix_keeps_nested_tt_values_inside_the_conditional_tail() {
    require_toolchain!();
    let lines = run(r#"
variant E { A(value: number), B }
const order: string[] = [];
const subject = (): E => { order.push("subject"); return E.A(4); };
const live = { method(value: number): number { order.push("method"); return value; } };
const absent = (() => undefined as typeof live | undefined)();
const miss = absent |> ?.method(match (subject()) { A(value) => value, B => 0 });
const hit = live |> ?.method(match (subject()) { A(value) => value, B => 0 });
console.log(miss, hit, order.join(","));
"#);
    assert_eq!(lines, ["undefined 4 subject,method"], "{lines:?}");
}

#[test]
fn optional_postfix_types_are_checked_as_plain_typescript() {
    require_toolchain!();
    let (ok, out) = typecheck(
        "declare const value: { n: number } | undefined;\n\
         const maybe: number | undefined = value |> ?.n;\n\
         const project = flow |> ((v: { n: number } | undefined) => v) |> ?.n;\n\
         const also: number | undefined = project(value);\n",
    );
    assert!(ok, "{out}");

    let (ok, out) = typecheck(
        "declare const value: { n: number } | undefined;\n\
         const bad = value |> ?.n |> ((n: number) => n + 1);\n",
    );
    assert!(!ok, "{out}");
}

#[test]
fn a_materialized_pipeline_keeps_head_before_callee() {
    require_toolchain!();
    let lines = run(r#"
variant E { A(value: number), B }
const order: string[] = [];
const head = (): E => { order.push("head"); return E.A(2); };
const step = () => { order.push("step"); return (value: number) => {
  order.push("call");
  return value + 1;
}; };
const value = match (head()) { A(value) => value, B => 0 } |> step();
console.log(order.join(","), value);
"#);
    assert_eq!(lines, ["head,step,call 3"]);
}

#[test]
fn flow_composition_infers_input_from_its_first_step() {
    require_toolchain!();
    // The composed function's parameter type comes from the first step,
    // and every later step (curried combinator, method step) infers from
    // the previous step's return type — no annotations anywhere.
    let (ok, out) = typecheck(&format!(
        "{PIPE_PRELUDE}\nconst label = flow |> half |> Option.mapP(x => x + 1) \
         |> Option.unwrapOrP(0) |> .toFixed(1);\nconst s: string = label(4);\n"
    ));
    assert!(ok, "{out}");
}

#[test]
fn flow_composition_keeps_the_first_step_arity() {
    require_toolchain!();
    // Composition is emitted with a rest-tuple parameter, so a multi-argument
    // first step stays multi-argument (a unary `flow` type would lose this).
    let (ok, out) = typecheck(
        "const add = (a: number, b: number) => a + b;\nconst f = flow |> add |> ((n: number) => n * 2);\nconst v: number = f(1, 2);\n",
    );
    assert!(ok, "{out}");
}

#[test]
fn flow_composition_input_mismatch_is_a_type_error_on_user_text() {
    require_toolchain!();
    // Calling the composed function with the wrong argument type is the
    // user's error — ttc emits no type tricks that could hide it.
    let (ok, out) = typecheck(
        "const parse = (s: string) => s.length;\nconst f = flow |> parse |> ((n: number) => n + 1);\nconst v = f(3);\n",
    );
    assert!(!ok, "{out}");
}

#[test]
fn flow_composition_runs_left_to_right_when_called() {
    require_toolchain!();
    let lines = run(r#"
const order: string[] = [];
const tap = <T,>(name: string) => (v: T): T => { order.push(name); return v; };
const f = flow |> tap<number>("s1") |> .toFixed(0) |> tap("s2");
console.log(order.join(","), "|", f(10), "|", order.join(","));
"#);
    assert_eq!(lines, [" | 10 | s1,s2"]); // nothing ran until the call
}

#[test]
fn pipeline_await_in_head_runs_in_the_surrounding_async_context() {
    require_toolchain!();
    let lines = run(r#"
const upper = (s: string) => s.toUpperCase();
async function main() {
  const v = await Promise.resolve("ok") |> upper |> .concat("!");
  console.log(v);
}
await main();
"#);
    assert_eq!(lines, ["OK!"]);
}

/* ------------------------------------------------------------------ */
/* tuple match                                                         */
/* ------------------------------------------------------------------ */

#[test]
fn runtime_tuple_match_dispatches_on_the_combination() {
    require_toolchain!();
    let lines = run(r#"
variant Conn { Online(latency: number), Offline }
variant Mode { Auto(), Manual(level: number) }

function decide(c: Conn, m: Mode): number {
  return match (c, m) {
    (Online(latency), Auto) if latency < 50 => 10,
    (Online, Auto) => 5,
    (Online, Manual(level)) => level,
    (Offline, _) => 0,
  };
}

console.log(decide(Conn.Online(10), Mode.Auto()));
console.log(decide(Conn.Online(80), Mode.Auto()));
console.log(decide(Conn.Online(10), Mode.Manual(7)));
console.log(decide(Conn.Offline, Mode.Auto()));
"#);
    assert_eq!(lines, vec!["10", "5", "7", "0"]);
}

#[test]
fn tuple_match_bindings_typecheck_per_position() {
    require_toolchain!();
    let (ok, out) = typecheck(
        r#"
variant Left { A(n: number), B }
variant Right { C(s: string), D }
function f(l: Left, r: Right): string {
  return match (l, r) {
    (A(n), C(s)) => s.repeat(n),
    (A(n), D) => n.toFixed(0),
    (B, C(s)) => s,
    (B, D) => "",
  };
}
"#,
    );
    assert!(ok, "{out}");
}

#[test]
fn tuple_match_scrutinees_evaluate_once_each_left_to_right() {
    require_toolchain!();
    let lines = run(r#"
variant Coin { Heads(), Tails }
const order: string[] = [];
function heads(name: string): Coin { order.push(name); return Coin.Heads(); }
const r = match (heads("a"), heads("b")) {
  (Heads, Heads) => 1,
  _ => 0,
};
console.log(order.join(","), r);
"#);
    assert_eq!(lines, vec!["a,b 1"]);
}

/* ------------------------------------------------------------------ */
/* nested patterns                                                     */
/* ------------------------------------------------------------------ */

#[test]
fn runtime_nested_pattern_falls_through_on_inner_mismatch() {
    require_toolchain!();
    let lines = run(r#"
variant Opt { Some(value: number), None }
variant Res { Ok(value: Opt), Err(error: string) }

function grade(r: Res): string {
  return match (r) {
    Ok(value: Some(value: v)) if v > 9000 => "over",
    Ok(value: Some(value: v)) => "num:" + v,
    Ok(value: None()) => "empty",
    Err(error) => "err:" + error,
    // v1 exhaustiveness: nested arms cover nothing, so `Ok` counts as
    // uncovered without a final wildcard (documented, like guards).
    _ => "unreachable",
  };
}

console.log(grade(Res.Ok(Opt.Some(9001))));
console.log(grade(Res.Ok(Opt.Some(3))));
console.log(grade(Res.Ok(Opt.None)));
console.log(grade(Res.Err("boom")));
"#);
    assert_eq!(lines, vec!["over", "num:3", "empty", "err:boom"]);
}

#[test]
fn nested_pattern_bindings_typecheck_through_the_paths() {
    require_toolchain!();
    // The emitted condition chain must narrow $tt_m.value for the
    // destructuring — no type tricks, plain control-flow analysis.
    let (ok, out) = typecheck(
        r#"
variant Opt { Some(value: number), None }
variant Res { Ok(value: Opt), Err(error: string) }
function f(r: Res): number {
  return match (r) {
    Ok(value: Some(value: v)) => v + 1,
    _ => 0,
  };
}
"#,
    );
    assert!(ok, "{out}");
}

/* ------------------------------------------------------------------ */
/* if let                                                              */
/* ------------------------------------------------------------------ */

#[test]
fn runtime_if_let_chains_and_falls_back() {
    require_toolchain!();
    let lines = run(r#"
variant Opt { Some(value: number), None }

function pick(a: Opt, b: Opt): number {
  let out = -1;
  if let Some(value) = a {
    out = value;
  } else if let Some(value) = b {
    out = value * 10;
  } else {
    out = 0;
  }
  return out;
}

console.log(pick(Opt.Some(1), Opt.Some(2)));
console.log(pick(Opt.None, Opt.Some(2)));
console.log(pick(Opt.None, Opt.None));
"#);
    assert_eq!(lines, vec!["1", "20", "0"]);
}

#[test]
fn if_let_bindings_stay_narrowed_inside_closures() {
    require_toolchain!();
    // The binding materializes as a const, so the narrowed type survives
    // closure boundaries — the gap that motivated the feature (TASK-042 G5).
    let (ok, out) = typecheck(
        r#"
variant Opt { Some(value: string), None }
function f(o: Opt, xs: number[]): string[] {
  const collected: string[] = [];
  if let Some(value) = o {
    xs.forEach(() => collected.push(value.toUpperCase()));
  }
  return collected;
}
"#,
    );
    assert!(ok, "{out}");
}

/* ------------------------------------------------------------------ */
/* result computation block                                            */
/* ------------------------------------------------------------------ */

/// tt variants in exactly the shape `@tt/std`'s `Result` has, so the block
/// tests need no module setup.
const RESULT_PRELUDE: &str = r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
variant UserError { NoUser() }
variant CompanyError { NoCompany(id: number) }
type User = { id: number; name: string; companyId: number };
type Company = { id: number; name: string };
declare function getUser(id: number): Res<User, UserError>;
declare function getCompany(id: number): Res<Company, CompanyError>;
"#;

#[test]
fn result_block_unions_the_error_types_of_its_bindings() {
    require_toolchain!();
    // The whole error-type question: two bindings with different error
    // types must produce `Res<_, UserError | CompanyError>` with no help
    // from ttc and no change to the combinators' signatures.
    let (ok, out) = typecheck(&format!(
        r#"{RESULT_PRELUDE}
const view = (id: number): Res<string, UserError | CompanyError> => result {{
  const user = try getUser(id);
  const company = try getCompany(user.companyId);
  return user.name + "@" + company.name;
}};
"#
    ));
    assert!(ok, "{out}");
}

#[test]
fn result_block_missing_an_error_type_is_a_type_error() {
    require_toolchain!();
    // The other half: an annotation that forgets one binding's error type
    // is tsc's error, reported on the user's own annotation.
    let (ok, out) = typecheck(&format!(
        r#"{RESULT_PRELUDE}
const view = (id: number): Res<string, UserError> => result {{
  const user = try getUser(id);
  const company = try getCompany(user.companyId);
  return user.name + "@" + company.name;
}};
"#
    ));
    assert!(!ok, "{out}");
}

#[test]
fn result_block_bindings_are_narrowed_success_values() {
    require_toolchain!();
    // No annotations anywhere: each binding must be the `Ok` payload type,
    // and the block's value type must flow out of the block.
    let (ok, out) = typecheck(&format!(
        r#"{RESULT_PRELUDE}
const view = (id: number) => result {{
  const user = try getUser(id);
  const company = try getCompany(user.companyId);
  const label: string = user.name.toUpperCase() + company.name;
  return {{ user, company, label }};
}};
const check = (id: number): string => match (view(id)) {{
  Ok(value) => value.label,
  Err(error) => match (error) {{
    NoUser => "no user",
    NoCompany(id: missing) => "no company " + missing,
  }},
}};
"#
    ));
    assert!(ok, "{out}");
}

#[test]
fn runtime_result_block_short_circuits_on_the_first_err() {
    require_toolchain!();
    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }

const steps: string[] = [];
const step = (name: string, ok: boolean): Res<string, string> => {
  steps.push(name);
  return ok ? Res.Ok(name) : Res.Err("failed:" + name);
};

const chain = (secondOk: boolean) => result {
  const a = try step("a", true);
  const b = try step("b", secondOk);
  const c = try step("c", true);
  return a + b + c;
};

console.log(JSON.stringify(chain(true)), steps.join(","));
steps.length = 0;
console.log(JSON.stringify(chain(false)), steps.join(","));
"#);
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":"abc"} a,b,c"#,
            r#"{"kind":"Err","error":"failed:b"} a,b"#,
        ]
    );
}

#[test]
fn runtime_using_disposes_when_try_propagates_err() {
    require_toolchain!();
    let lines = run_with_tsc_flags(
        r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }

const events: string[] = [];
const fail = (): Res<number, string> => Res.Err("boom");

const sync = () => {
  using resource = {
    [Symbol.dispose]() { events.push("sync-dispose"); },
  };
  const value = try fail();
  return Res.Ok(value);
};

const asyncRun = async () => {
  await using resource = {
    async [Symbol.asyncDispose]() { events.push("async-dispose"); },
  };
  const value = try fail();
  return Res.Ok(value);
};

console.log(JSON.stringify(sync()), events.join(","));
events.length = 0;
asyncRun().then((value) => console.log(JSON.stringify(value), events.join(",")));
"#,
        &["--lib", "es2022,dom,esnext.disposable"],
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Err","error":"boom"} sync-dispose"#,
            r#"{"kind":"Err","error":"boom"} async-dispose"#,
        ]
    );
}

#[test]
fn runtime_nested_results_preserve_constructor_and_generator_protocols() {
    require_toolchain!();
    for source in [
        "class C { constructor() { try fail(); } }\n",
        "function* values() { yield try fail(); }\n",
    ] {
        let diagnostics = ttc::analyze(source, &Options::default());
        assert_eq!(diagnostics.len(), 1, "{source}\n{diagnostics:#?}");
        assert_eq!(diagnostics[0].code, ttc::DiagnosticCode::TryPlacement);
    }

    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
const fail = (): Res<number, string> => Res.Err("boom");

class C {
  outcome;
  constructor() {
    this.outcome = result { return try fail(); };
  }
}

function* values() {
  yield result { return try fail(); };
  yield "after";
}

const instance = new C();
console.log(instance instanceof C, JSON.stringify(instance.outcome));
const iterator = values();
console.log(JSON.stringify(iterator.next()));
console.log(JSON.stringify(iterator.next()));
console.log(Array.from(values()).map((value) => JSON.stringify(value)).join(","));
"#);
    assert_eq!(
        lines,
        vec![
            r#"true {"kind":"Err","error":"boom"}"#,
            r#"{"value":{"kind":"Err","error":"boom"},"done":false}"#,
            r#"{"value":"after","done":false}"#,
            r#"{"kind":"Err","error":"boom"},"after""#,
        ]
    );
}

#[test]
fn runtime_result_block_with_await_resolves_to_a_result() {
    require_toolchain!();
    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }

const fetchNum = async (n: number): Promise<Res<number, string>> =>
  n > 0 ? Res.Ok(n) : Res.Err("not positive");

const total = async (a: number, b: number) => result {
  const x = try await fetchNum(a);
  const y = try await fetchNum(b);
  return x + y;
};

total(2, 3).then((r) => console.log(JSON.stringify(r)));
total(2, -1).then((r) => console.log(JSON.stringify(r)));
"#);
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":5}"#,
            r#"{"kind":"Err","error":"not positive"}"#,
        ]
    );
}

#[test]
fn runtime_result_exits_cross_user_breakable_statements() {
    require_toolchain!();
    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
const events: string[] = [];
const step = (ok: boolean, name: string): Res<number, string> =>
  ok ? Res.Ok(name.length) : Res.Err(name);

const fromFor = (ok: boolean) => result {
  for (const name of ["for"]) { return try step(ok, name); }
  events.push("for-tail");
  return 99;
};
const fromWhile = (ok: boolean) => result {
  while (true) { return try step(ok, "while"); }
  events.push("while-tail");
  return 99;
};
const fromDo = (ok: boolean) => result {
  do { return try step(ok, "do"); } while (false);
  events.push("do-tail");
  return 99;
};
const fromSwitch = (ok: boolean) => result {
  switch (ok) { default: return try step(ok, "switch"); }
  events.push("switch-tail");
  return 99;
};

for (const run of [fromFor, fromWhile, fromDo, fromSwitch]) {
  console.log(JSON.stringify(run(true)), JSON.stringify(run(false)));
}
console.log(events.join(","));
"#);
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":3} {"kind":"Err","error":"for"}"#,
            r#"{"kind":"Ok","value":5} {"kind":"Err","error":"while"}"#,
            r#"{"kind":"Ok","value":2} {"kind":"Err","error":"do"}"#,
            r#"{"kind":"Ok","value":6} {"kind":"Err","error":"switch"}"#,
            "",
        ]
    );
}

#[test]
fn runtime_result_preserves_statement_match_effect_order() {
    require_toolchain!();
    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
const events: string[] = [];
const read = (): Res<number, string> => Res.Ok(7);
const subject = (tag: number) => { events.push("subject-" + tag); return tag; };

const run = (tag: number) => result {
  const value = try read();
  match (subject(tag)) {
    1 => { events.push("one"); },
    _ => { events.push("other"); },
  }
  events.push("after");
  return value;
};

console.log(JSON.stringify(run(1)), events.join(","));
events.length = 0;
console.log(JSON.stringify(run(2)), events.join(","));
"#);
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":7} subject-1,one,after"#,
            r#"{"kind":"Ok","value":7} subject-2,other,after"#,
        ]
    );
}

#[test]
fn runtime_ordinary_result_success_preserves_expression_host_protocols() {
    require_toolchain!();
    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
const read = (value: number): Res<number, string> => Res.Ok(value);

class FieldBox {
  field = result { const value = try read(1); return value; };
}
class ConstructorBox {
  outcome;
  constructor() {
    this.outcome = result { const value = try read(2); return value; };
  }
}
function withDefault(value = result { const item = try read(3); return item; }) {
  return value;
}
function* values() {
  yield result { const item = try read(4); return item; };
  yield "after";
}
const text = `value=${result { const item = try read(5); return item; }}`;

console.log(JSON.stringify(new FieldBox().field));
console.log(JSON.stringify(new ConstructorBox().outcome));
console.log(JSON.stringify(withDefault()));
console.log(Array.from(values()).map((value) => JSON.stringify(value)).join(","));
console.log(text);
"#);
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":1}"#,
            r#"{"kind":"Ok","value":2}"#,
            r#"{"kind":"Ok","value":3}"#,
            r#"{"kind":"Ok","value":4},"after""#,
            "value=[object Object]",
        ]
    );
}

#[test]
fn strict_typescript_accepts_all_result_discriminator_shapes() {
    require_toolchain!();
    let lines = run(r#"
variant Res<T, E> { Ok(value: T), Err(error: E) }
type Alias<T, E> = Res<T, E>;

const directErr = () => {
  const value = try Res.Err("direct");
  return Res.Ok(value);
};
const directOk = () => {
  const value = try Res.Ok(1);
  return Res.Ok(value + 1);
};
const widened = (input: Res<number, string>): Res<number, string> => {
  const value = try input;
  return Res.Ok(value + 1);
};
const aliased = (input: Alias<number, string>): Alias<number, string> => {
  const value = try input;
  return Res.Ok(value + 1);
};
function generic<T, E>(input: Res<T, E>): Res<T, E> {
  const value = try input;
  return Res.Ok(value);
}

console.log(JSON.stringify(directErr()));
console.log(JSON.stringify(directOk()));
console.log(JSON.stringify(widened(Res.Err("wide"))));
console.log(JSON.stringify(aliased(Res.Ok(2))));
console.log(JSON.stringify(generic(Res.Ok("generic"))));
"#);
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Err","error":"direct"}"#,
            r#"{"kind":"Ok","value":2}"#,
            r#"{"kind":"Err","error":"wide"}"#,
            r#"{"kind":"Ok","value":3}"#,
            r#"{"kind":"Ok","value":"generic"}"#,
        ]
    );
}

#[test]
fn runtime_result_block_replaces_nested_combinator_callbacks() {
    require_toolchain!();
    // The motivating shape: three dependent steps that all stay in scope,
    // written flat, against the real standard library.
    let lines = run_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

type User = { id: number; companyId: number; name: string };
type Company = { id: number; name: string };

const getUser = (id: number): TResult<User, string> =>
  id === 1 ? Result.Ok({ id, companyId: 7, name: " Ada " }) : Result.Err("no user " + id);
const getCompany = (id: number): TResult<Company, string> =>
  Result.Ok({ id, name: "Acme" });
const getPermission = (u: User, c: Company): TResult<string, string> =>
  Result.Ok(u.name.trim() + "@" + c.name);

const view = (id: number) => result {
  const user = try getUser(id);
  const company = try getCompany(user.companyId);
  const normalized = user.name |> .trim() |> .toLowerCase();
  const permission = try getPermission(user, company);
  return { user, company, permission, normalized };
};

console.log(JSON.stringify(view(1)));
console.log(JSON.stringify(view(2)));
"#,
    );
    assert_eq!(
        lines,
        vec![
            r#"{"kind":"Ok","value":{"user":{"id":1,"companyId":7,"name":" Ada "},"company":{"id":7,"name":"Acme"},"permission":"Ada@Acme","normalized":"ada"}}"#,
            r#"{"kind":"Err","error":"no user 2"}"#,
        ]
    );
}

/* ------------------------------------------------------------------ */
/* literal match patterns                                              */
/* ------------------------------------------------------------------ */

#[test]
fn runtime_literal_string_match() {
    require_toolchain!();
    let lines = run(r#"
type Direction = "north" | "south" | "east" | "west";

function short(dir: Direction) {
  return match (dir) {
    "north" => "N",
    "south" => "S",
    "east" => "E",
    "west" => "W",
  };
}

console.log(short("north"), short("south"), short("east"), short("west"));
"#);
    assert_eq!(lines, ["N S E W"]);
}

#[test]
fn runtime_literal_number_match_with_or_patterns() {
    require_toolchain!();
    let lines = run(r#"
function status(code: 200 | 201 | 404 | 500) {
  return match (code) {
    200 | 201 => "success",
    404 => "not found",
    500 => "server error",
  };
}

console.log(status(200), status(201), status(404), status(500));
"#);
    assert_eq!(lines, ["success success not found server error"]);
}

#[test]
fn runtime_literal_boolean_match() {
    require_toolchain!();
    let lines = run(r#"
function label(flag: boolean) {
  return match (flag) {
    true => "yes",
    false => "no",
  };
}

console.log(label(true), label(false));
"#);
    assert_eq!(lines, ["yes no"]);
}

#[test]
fn runtime_literal_match_keeps_number_spellings() {
    require_toolchain!();
    let lines = run(r#"
function pick(n: number) {
  return match (n) {
    0xff => "hex",
    1_000 => "sep",
    1.5e2 => "exp",
    -1 => "neg",
    _ => "other",
  };
}

console.log(pick(255), pick(1000), pick(150), pick(-1), pick(0));
"#);
    assert_eq!(lines, ["hex sep exp neg other"]);
}

#[test]
fn runtime_literal_match_evaluates_the_scrutinee_once() {
    require_toolchain!();
    let lines = run(r#"
let calls = 0;
function getValue(): string {
  calls += 1;
  return "b";
}

const picked = match (getValue()) {
  "a" => 1,
  "b" => 2,
  _ => 3,
};
console.log(picked, calls);
"#);
    assert_eq!(lines, ["2 1"]);
}

#[test]
fn runtime_literal_match_runtime_guard_throws() {
    require_toolchain!();
    let lines = run(r#"
function label(dir: string) {
  return match (dir as "a" | "b") {
    "a" => 1,
    "b" => 2,
  };
}

try {
  label("zzz");
  console.log("no throw");
} catch (e) {
  console.log((e as Error).message);
}
"#);
    assert_eq!(lines, [r#"tt match: unexpected literal "zzz""#]);
}

#[test]
fn runtime_literal_match_with_guard() {
    require_toolchain!();
    let lines = run(r#"
function classify(code: number, retry: boolean) {
  return match (code) {
    500 if retry => "retrying",
    500 => "failed",
    _ => "ok",
  };
}

console.log(classify(500, true), classify(500, false), classify(200, true));
"#);
    assert_eq!(lines, ["retrying failed ok"]);
}

#[test]
fn typecheck_literal_match_narrows_each_arm() {
    require_toolchain!();
    // The switch discriminates on the value itself, so tsc narrows the
    // scrutinee inside each arm with no type tricks.
    let (ok, out) = typecheck(
        r#"
type Size = "sm" | "md" | "lg";
const px: number = match ("sm" as Size) {
  "sm" => 12,
  "md" => 16,
  "lg" => 20,
};
"#,
    );
    assert!(ok, "{out}");
}

#[test]
fn typecheck_literal_match_block_bodies() {
    require_toolchain!();
    let (ok, out) = typecheck(
        r#"
const label: string = match ("a" as "a" | "b") {
  "a" => { return "first"; },
  "b" => { return "second"; },
};
"#,
    );
    assert!(ok, "{out}");
}

/* ------------------------------------------------------------------ */
/* val — binding modifier                                             */
/* ------------------------------------------------------------------ */

#[test]
fn typecheck_val_bindings_are_plain_typescript() {
    require_toolchain!();
    // `val` is compile-time only: what reaches tsc is an ordinary
    // declaration and an ordinary parameter, with no readonly types and
    // no runtime helper.
    let (ok, out) = typecheck(
        r#"
type User = { name: string; tags: string[] };

val const user: User = { name: "Kim", tags: ["dev"] };

function inspect(val u: User): string {
  return u.name + u.tags.length;
}

val let state = { count: 0 };
state = { ...state, count: state.count + 1 };

const label = inspect(user) + state.count;
"#,
    );
    assert!(ok, "{out}");
    assert!(
        !out.contains("val "),
        "the modifier leaked into the output: {out}"
    );
    assert!(!out.contains("readonly"), "{out}");
}

#[test]
fn run_val_program_behaves_exactly_like_the_typescript_it_erases_to() {
    require_toolchain!();
    let lines = run(r#"
val const config = { name: "tt", tags: ["dev"] };
val let state = { count: 0 };

function describe(val c: { name: string; tags: string[] }): string {
  return `${c.name}:${c.tags.length}`;
}

function bump(s: { count: number }) {
  s.count += 1;
  return s;
}

state = { count: state.count + 1 };
const mutable = { count: 0 };
bump(mutable);

console.log(describe(config));
console.log(String(state.count));
console.log(String(mutable.count));
"#);
    assert_eq!(lines, ["tt:1", "1", "1"]);
}

#[test]
fn a_loop_header_match_is_evaluated_every_iteration() {
    if !have("tsc") || !have("node") {
        return;
    }
    // TASK-160 issue 14: this used to hoist the match out of the loop and
    // never re-evaluate it.
    let lines = run(r#"
let n = 0;
function next(): number { n = n + 1; return n; }
function id(v: number): number { return v; }
const seen: number[] = [];
while (id(match (next()) { 1 => 1, 2 => 1, _ => 0 })) {
  seen.push(n);
}
console.log(JSON.stringify(seen), n);
"#);
    assert_eq!(lines, ["[1,2] 3"]);
}

#[test]
fn a_short_circuited_argument_match_does_not_evaluate() {
    if !have("tsc") || !have("node") {
        return;
    }
    // TASK-160 issue 15: the match argument (and its subject's effects)
    // must not run when `&&` short-circuits, and the output must still
    // typecheck without the capture escaping its region.
    let lines = run(r#"
const trace: string[] = [];
function subject(tag: string): number { trace.push(tag); return 1; }
function id(v: number): number { return v; }
declare const globalThis: { flagOn: boolean };
const on = true as boolean;
const off = false as boolean;
const a = on && id(match (subject("on")) { 1 => 10, _ => 0 });
const b = off && id(match (subject("off")) { 1 => 20, _ => 0 });
console.log(JSON.stringify(trace), a, b);
"#);
    assert_eq!(lines, ["[\"on\"] 10 false"]);
}

#[test]
fn sibling_values_beside_a_short_circuit_keep_left_to_right_order() {
    if !have("tsc") || !have("node") {
        return;
    }
    // TASK-160 issue 16: this shape used to duplicate and drop source
    // bytes; now both values evaluate in place, in argument order.
    let lines = run(r#"
const trace: number[] = [];
function mark(n: number): number { trace.push(n); return n; }
function g(x: unknown, y: unknown): void { console.log(x, y); }
const a = true as boolean;
g(a && match (mark(1)) { 1 => 11, _ => 0 }, match (mark(2)) { 2 => 22, _ => 0 });
console.log(JSON.stringify(trace));
"#);
    assert_eq!(lines, ["11 22", "[1,2]"]);
}

#[test]
fn conditional_operations_keep_their_types_without_undefined() {
    if !have("tsc") {
        return;
    }
    // TASK-160 결정 17: promoting only the value used to widen every
    // conditional operation's type with `undefined`.
    let (ok, out) = typecheck(
        r#"
declare const flag: boolean;
declare const maybe: number | undefined;
export const a: number | boolean = flag && match (1) { 1 => 1, _ => 0 };
export const b: number | boolean = flag || match (1) { 1 => 2, _ => 0 };
export const c: number = maybe ?? match (1) { 1 => 3, _ => 0 };
export const d: number = flag ? match (1) { 1 => 4, _ => 0 } : 9;
declare const f: ((v: number) => number) | undefined;
export const e: number | undefined = f?.(match (1) { 1 => 5, _ => 0 });
declare const host: { g?: (v: number) => number };
export const g: number | undefined = host.g?.(match (1) { 1 => 6, _ => 0 });
"#,
    );
    assert!(ok, "{out}");
}

#[test]
fn an_optional_call_operation_preserves_this_check_order_and_short_circuit() {
    if !have("tsc") || !have("node") {
        return;
    }
    let lines = run(r#"
const trace: string[] = [];
const live = {
  base: 7,
  m(v: number): number { trace.push("call:" + (this === live)); return this.base + v; },
};
const dead: { m?: (v: number) => number } = {};
function arg(tag: string): number { trace.push(tag); return 1; }
const hit = live.m?.(match (arg("live")) { 1 => 1, _ => 0 });
const miss = dead.m?.(match (arg("dead")) { 1 => 1, _ => 0 });
console.log(JSON.stringify(trace), hit, miss);
"#);
    assert_eq!(lines, ["[\"live\",\"call:true\"] 8 undefined"]);
}

#[test]
fn a_logical_operation_returns_the_condition_value_when_it_short_circuits() {
    if !have("tsc") || !have("node") {
        return;
    }
    let lines = run(r#"
const zero = 0 as number;
const empty = "" as string;
const a = zero && match (1) { 1 => 1, _ => 0 };
const b = empty || match (1) { 1 => 2, _ => 0 };
const c = (zero as number | null) ?? match (1) { 1 => 3, _ => 0 };
console.log(a, JSON.stringify(b), c);
"#);
    assert_eq!(lines, ["0 2 0"]);
}

#[test]
fn eager_arguments_keep_left_to_right_order_at_runtime() {
    if !have("tsc") || !have("node") {
        return;
    }
    // The schedule captures every effectful earlier argument; only a
    // provably inert one may stay in place (TASK-160 §9). If the effect
    // judgement overreached, `mark(1)` would run after the match region.
    let lines = run(r#"
const trace: number[] = [];
function mark(n: number): number { trace.push(n); return n; }
function g(a: number, b: number, c: number): void { console.log(a, b, c); }
g(mark(1), match (mark(2)) { 2 => 20, _ => 0 }, mark(3));
console.log(JSON.stringify(trace));
"#);
    assert_eq!(lines, ["1 20 3", "[1,2,3]"]);
}

#[test]
fn a_block_arm_exit_leaves_the_region_from_inside_a_loop() {
    if !have("tsc") || !have("node") {
        return;
    }
    // TASK-160 §6: the region keeps a label exactly when the rewritten
    // `return` sits inside a statement that would swallow an unlabeled
    // `break`. If the label were dropped here the `break` would leave the
    // loop and fall through to the next statement instead.
    let lines = run(r#"
variant Pick { Scan(from: number), Zero }
declare const nothing: number;
function choose(p: Pick): number {
  return match (p) {
    Scan(from) => {
      for (const x of [from, from + 1, from + 2]) {
        if (x % 3 === 0) { return x; }
      }
      return -1;
    },
    Zero => 0,
  };
}
console.log(choose(Pick.Scan(2)), choose(Pick.Scan(4)), choose(Pick.Zero));
"#);
    assert_eq!(lines, ["3 6 0"]);
}

#[test]
fn a_block_arm_exit_without_a_loop_still_yields_its_value() {
    if !have("tsc") || !have("node") {
        return;
    }
    let lines = run(r#"
variant Pick { Some(v: number), None }
function choose(p: Pick): number {
  return match (p) {
    Some(v) => { const doubled = v * 2; return doubled; },
    None => 0,
  };
}
const guarded = (n: number): number => match (n) {
  0 if true => 1,
  _ => { return n + 100; },
};
console.log(choose(Pick.Some(21)), choose(Pick.None), guarded(0), guarded(5));
"#);
    assert_eq!(lines, ["42 0 1 105"]);
}

#[test]
fn a_node_stack_trace_points_at_the_tt_source() {
    if !have("node") {
        return;
    }
    // TASK-200's whole point: the frame a user sees names the construct
    // they wrote, at the line and column they wrote it, not a position in
    // a file nobody authored.
    let dir = tmpdir();
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let source = src_dir.join("app.tt");
    fs::write(
        &source,
        "variant Shape { Circle(r: number), Rect(w: number, h: number) }\n\
         \n\
         function area(s: Shape): number {\n\
         \x20 return match (s) {\n\
         \x20   Circle(r) => { throw new Error(\"boom\"); },\n\
         \x20   Rect(w, h) => w * h,\n\
         \x20 };\n\
         }\n\
         \n\
         area(Shape.Circle(1));\n",
    )
    .unwrap();
    let out_dir = dir.join("out");
    let compiled = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["-o", out_dir.to_str().unwrap()])
        .args(["--source-map", "file"])
        .arg("--no-banner")
        .arg(&source)
        .output()
        .expect("failed to run ttc");
    assert!(compiled.status.success(), "{compiled:?}");

    let script = out_dir.join("app.ts");
    let run = Command::new("node")
        .arg("--enable-source-maps")
        .arg("--experimental-strip-types")
        .arg(&script)
        .output()
        .expect("failed to run node");
    let trace = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    // The throw sits on line 5 of the `.tt`, inside the arm body.
    assert!(trace.contains("app.tt:5:"), "{trace}");
    // And the call that reached it is line 10.
    assert!(trace.contains("app.tt:10:"), "{trace}");
    // No frame should name the generated file.
    assert!(!trace.contains("app.ts:"), "{trace}");
}

#[test]
fn a_frame_inside_generated_glue_names_the_construct_that_wrote_it() {
    if !have("node") {
        return;
    }
    // A throw the compiler itself wrote — the unexhausted-case guard —
    // has no source text of its own, so it maps to the `match` that owns
    // it rather than to nothing.
    let dir = tmpdir();
    let source = dir.join("app.tt");
    fs::write(
        &source,
        "variant E { A(v: number), B }\n\
         function pick(e: E): number {\n\
         \x20 return match (e) {\n\
         \x20   A(v) => v,\n\
         \x20   B => 2,\n\
         \x20 };\n\
         }\n\
         pick({ kind: \"C\" } as unknown as E);\n",
    )
    .unwrap();
    let out_dir = dir.join("out");
    let compiled = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["-o", out_dir.to_str().unwrap()])
        .args(["--source-map", "file"])
        .arg("--no-banner")
        .arg(&source)
        .output()
        .expect("failed to run ttc");
    assert!(compiled.status.success(), "{compiled:?}");
    let run = Command::new("node")
        .arg("--enable-source-maps")
        .arg("--experimental-strip-types")
        .arg(out_dir.join("app.ts"))
        .output()
        .expect("failed to run node");
    let trace = String::from_utf8_lossy(&run.stderr).into_owned();
    assert!(trace.contains("unexpected case"), "{trace}");
    // Line 3 is `return match (e) {` — the construct the guard belongs to.
    assert!(trace.contains("app.tt:3:"), "{trace}");
}
