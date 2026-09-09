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
