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
fn a_node_stack_frame_on_a_copied_line_names_its_column() {
    if !have("node") {
        return;
    }
    // TASK-443: a consumer takes the nearest mapping at or before a frame,
    // so a line copied as one chunk reported column 1 for every frame on it.
    let dir = tmpdir();
    let source = dir.join("app.tt");
    fs::write(
        &source,
        "variant E { A(v: number), B }\n\
         export const n = match (E.B) { A(v) => v, B => 0 };\n\
         const  x = 1;   function boom() { return [1].map(() => { throw new Error(\"boom\"); }); }\n\
         boom();\n",
    )
    .unwrap();
    let out_dir = dir.join("out");
    let compiled = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .args(["-o", out_dir.to_str().unwrap()])
        .args(["--source-map", "file"])
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
    // `new Error` and the `.map` call that reached it, at their own columns.
    assert!(trace.contains("app.tt:3:64)"), "{trace}");
    assert!(trace.contains("app.tt:3:46)"), "{trace}");
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

#[test]
fn optional_variant_fields_are_absent_when_their_argument_is() {
    require_toolchain!();
    let src = r#"
variant V { C(opt?: number), D }
variant G<T> { P(label: string, value?: T, note?: string) }
const bare = V.C();
const full = V.C(2);
const generic = G.P<number>("p", undefined, "n");
const opt: number | undefined = match (bare) { C(opt) => opt, D => 0 };
console.log("opt" in bare, "opt" in full, JSON.stringify(full));
console.log(JSON.stringify(generic), "value" in generic, opt === undefined);
"#;
    let expected = [
        "false true {\"kind\":\"C\",\"opt\":2}",
        "{\"kind\":\"P\",\"label\":\"p\",\"note\":\"n\"} false true",
    ];
    assert_eq!(run(src), expected);
    assert_eq!(
        run_with_tsc_flags(src, &["--exactOptionalPropertyTypes"]),
        expected
    );
}

#[test]
fn runtime_sibling_conditional_trys_each_evaluate_their_operand_in_order() {
    require_toolchain!();
    let out = run_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";
const seen: string[] = [];
function p(s: string): TResult<number, string> {
  seen.push(s);
  return s === "e" ? Result.Err("bad " + s) : Result.Ok(Number(s));
}
function pair(a: number, b: number): number { return a * 10 + b; }
function k1(c: boolean, a: string, b: string): TResult<number, string> {
  const x = [c ? try p(a) : 0, c ? try p(b) : 1];
  return Result.Ok(x[0] * 10 + x[1]);
}
function k2(n: number, a: string, b: string): TResult<number, string> {
  return Result.Ok((n && try p(a)) + (n && try p(b)));
}
function k3(n: number, a: string, b: string): TResult<number, string> {
  return Result.Ok(pair(n && try p(a), n && try p(b)));
}
function k4(c: boolean, a: string, b: string): TResult<string, string> {
  return Result.Ok(`${c ? try p(a) : 0}-${c ? try p(b) : 1}`);
}
function show(r: TResult<unknown, string>): string {
  const line = ("value" in r ? "ok " + r.value : "err " + r.error) + " [" + seen.join(",") + "]";
  seen.length = 0;
  return line;
}
console.log(show(k1(true, "1", "2")), show(k1(true, "e", "2")), show(k1(false, "1", "2")));
console.log(show(k2(1, "3", "4")), show(k2(1, "3", "e")), show(k2(0, "3", "4")));
console.log(show(k3(1, "5", "6")), show(k3(1, "e", "6")));
console.log(show(k4(true, "7", "8")), show(k4(false, "7", "8")));
"#,
    );
    assert_eq!(
        out,
        [
            "ok 12 [1,2] err bad e [e] ok 1 []",
            "ok 7 [3,4] err bad e [3,e] ok 0 []",
            "ok 56 [5,6] err bad e [e]",
            "ok 7-8 [7,8] ok 0-1 []",
        ]
    );
}

#[test]
fn runtime_a_later_declarator_value_runs_after_earlier_declarators_in_their_scope() {
    require_toolchain!();
    let out = run_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";
variant O { A(n: number), B }
const o = O.A(1) as O;
const log: string[] = [];
function t<T>(s: string, v: T): T { log.push(s); return v; }
function r(n: number): TResult<number, string> { log.push("r" + n); return Result.Ok(n); }
function take(): string { const line = log.join(" "); log.length = 0; return line; }
const a = 100;
function f() {
  const a = t("a", 10), b = match (t("m", o)) { A(n) => a + n, B => 0 };
  return b;
}
function g() { var a = 10, b = match (o) { A(n) => a + n, B => 0 }; return b; }
function h(): TResult<number, string> {
  let a = t("a", 1), b = 1 + try r(a), c = t("c", b);
  const d = t("d", 2), e = result { const x = try r(d); return x + c; };
  return Result.Ok(e.kind === "Ok" ? e.value : 0);
}
export const x = t("x", 3), y = match (o) { A(n) => x + n, B => 0 };
console.log(f(), take(), g(), take());
console.log(JSON.stringify(h()), take());
console.log(y, a);
"#,
    );
    assert_eq!(
        out,
        [
            "11 x a m 11 ",
            r#"{"kind":"Ok","value":4} a r1 c d r2"#,
            "4 100",
        ]
    );
}

#[test]
fn runtime_a_result_block_in_an_enum_member_reads_the_members_in_order() {
    require_toolchain!();
    let out = run_with_std(
        r#"
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";
variant O { A(n: number), B }
const o = O.A(1) as O;
const log: string[] = [];
function r(n: number): TResult<number, string> { log.push("r" + n); return Result.Ok(n); }
const P = 100;
enum F {
  P = 7,
  Q = (result { const x = try r(P); return x + 1; }).kind === "Ok" ? P + 1 : 0,
  R = [P].map((p) => match (o) { A(n) => p + n, B => 0 })[0]!,
  S = (log.push("S"), 3),
}
console.log(F.Q, F.R, F.S, log.join(" "));
"#,
    );
    assert_eq!(out, ["8 8 3 r7 S"]);
}

#[test]
fn an_asserted_value_type_checks_without_a_checker_at_compile_time() {
    if !common::tsc_available() {
        return;
    }
    // TASK-596: without the checker, the storage of a `satisfies T`
    // operand is annotated with `T`, so its object-literal arms keep their
    // literal tags, and the operand of `as T` is annotated with nothing
    // from outside the assertion.
    let source = as_module(
        r#"
variant O { A, B }
type Ev = { kind: "click"; x: number } | { kind: "key"; code: string };
declare const o: O;
declare const x: unknown;
export const e = match (o) { A => ({ kind: "click", x: 1 }), B => ({ kind: "key", code: "z" }) } satisfies Ev;
export const q: string = match (o) { A => 1, B => 2 } as unknown as string;
export function h(): number { return match (o) { A => x, B => 0 } as number; }
"#,
    );
    let code = compile(
        &source,
        &Options {
            defer_to_checker: true,
            ..options_with_runtime("./runtime.js")
        },
    )
    .expect("tt compile failed");
    let dir = tmpdir();
    let ts = dir.join("main.ts");
    fs::write(&ts, &code).unwrap();
    let out = common::tsc()
        .arg(&ts)
        .arg("--noEmit")
        .args(TSC_FLAGS)
        .output()
        .expect("failed to run tsc");
    assert!(
        out.status.success(),
        "{}\n---compiled---\n{code}",
        tsc_report(&out)
    );
}
