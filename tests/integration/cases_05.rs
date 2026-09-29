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
    if !common::tsc_available() {
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
    if !common::tsc_available() {
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
    if !common::tsc_available() {
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
    if !common::tsc_available() {
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
    if !common::tsc_available() {
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
    if !common::tsc_available() {
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
    if !common::tsc_available() {
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
    if !common::tsc_available() {
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
    if !common::tsc_available() {
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
fn run_guard_regions_preserve_all_values_and_short_circuit_effects() {
    if !common::tsc_available() { return; }
    let out = run(r#"
const events: number[] = [];
function mark(n: number) { events.push(n); return n; }
function both(a: boolean, b: boolean) {
  return match (1) {
    1 if (match (mark(1)) { 1 => { const value = a; return value; }, _ => false }) &&
         (match (mark(2)) { 2 => { const value = b; return value; }, _ => false }) => true,
    _ => false
  };
}
function either(a: boolean, b: boolean) {
  return match (1) {
    1 if (match (mark(3)) { 3 => { const value = a; return value; }, _ => false }) ||
         (match (mark(4)) { 4 => { const value = b; return value; }, _ => false }) => true,
    _ => false
  };
}
function choose(a: boolean) {
  return match (1) {
    1 if a ? (match (mark(5)) { 5 => { const value = true; return value; }, _ => false }) :
             (match (mark(6)) { 6 => { const value = false; return value; }, _ => false }) => true,
    _ => false
  };
}
console.log(both(false, true), events.splice(0).join(","));
console.log(both(true, false), events.splice(0).join(","));
console.log(both(true, true), events.splice(0).join(","));
console.log(either(true, false), events.splice(0).join(","));
console.log(either(false, true), events.splice(0).join(","));
console.log(choose(true), events.splice(0).join(","));
console.log(choose(false), events.splice(0).join(","));
"#);
    assert_eq!(out, ["false 1", "false 1,2", "true 1,2", "true 3", "true 3,4", "true 5", "false 6"]);
}

#[test]
fn generated_bindings_never_capture_user_identifiers() {
    if !common::tsc_available() { return; }
    let out = run(r#"
variant O { S(v: number), N }
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const Ok = <T,>(value: T): R<T> => ({ kind: "Ok", value });
const $tt_m = "m";
const $tt_m_1 = "m1";
const $tt_t0 = "t0";
const $tt_ap = "ap";
const $tt_fl = "fl";
const $tt_v = "v";
const $tt_r = "r";
const $tt_k = "k";
const obj = { tag: "o", add(n: number) { return n + this.tag + $tt_r + $tt_k + $tt_v; } };
const key = "add" as const;
const r = match (O.S(1)) { S(v) => v + $tt_m + $tt_m_1, N => "" };
function f(): R<string> { const a = try Ok(2); return Ok(a + $tt_t0); }
const xs = [1].map(x => x |> String);
const ys = [1].map(x => x |> obj.add);
const zs = [1].map(x => x |> obj[key]);
const g = flow |> ((n: number) => n + 1) |> .toFixed(1) |> Number |> obj.add;
console.log(r, JSON.stringify(f()), xs[0], ys[0], zs[0], g(1), $tt_ap, $tt_fl);
"#);
    assert_eq!(
        out,
        [r#"1mm1 {"kind":"Ok","value":"2t0"} 1 1orkv 1orkv 2orkv ap fl"#]
    );
}

#[test]
fn an_unexpected_case_reports_its_own_match_subject() {
    if !common::tsc_available() { return; }
    let out = run(r#"
const pick = (s: string) => s as "a" | "b";
function outer(s: string) {
  return match (match (pick(s)) { "a" => pick("z"), "b" => pick("b") }) { "a" => 1, "b" => 2 };
}
try { outer("a"); } catch (error) { console.log((error as Error).message); }
console.log(outer("b"));
"#);
    assert_eq!(out, [r#"tt match: unexpected literal "z""#, "2"]);
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
fn runtime_let_else_binds_from_an_object_literal_initializer() {
    require_toolchain!();
    let out = run(r#"
function f(n: number) {
  const Some(value: v) = { kind: "Some" as const, value: n } else { return -1; };
  return v;
}
function g(on: boolean) {
  const Some(value) = on ? { kind: "Some" as const, value: "on" } : { kind: "None" as const } else { return "off"; };
  return value;
}
console.log(f(3), g(true), g(false));
"#);
    assert_eq!(out, ["3 on off"]);
}

#[test]
fn runtime_match_suspends_its_generator_from_the_subject_and_guard() {
    require_toolchain!();
    let out = run(r#"
variant S { A(n: number), B }

function* subject(): Generator<string, number, S> {
  const r = match (yield "subject") { A(n) => n, B => 0 };
  return r;
}

function* guard(s: S): Generator<number, string, number> {
  const r = match (s) { A(n) if (yield n) === 1 => `one ${n}`, _ => "other" };
  return r;
}

function* cast(): Generator<number, number, unknown> {
  const r = match ((yield 1) as S) {
    A(n) => match ((yield n) as S) { A(n: m) => n + m, B => n },
    B => 0,
  };
  return r;
}

class Base { start() { return 7; } }
class Derived extends Base {
  scale = 10;
  *run(): Generator<number, number, S> {
    return match (yield super.start()) { A(n) => n * this.scale, B => -1 };
  }
}

async function* later(): AsyncGenerator<number, number, Promise<S>> {
  const r = match (await (yield 1)) { A(n) => n, B => 0 };
  return r;
}

const drive = <Y, R, N>(g: Generator<Y, R, N>, sent: N[]) => {
  const seen: unknown[] = [JSON.stringify(g.next().value)];
  for (const value of sent) seen.push(JSON.stringify(g.next(value).value));
  return seen.join(" ");
};

console.log(drive(subject(), [S.A(4)]), drive(subject(), [S.B]));
console.log(drive(guard(S.A(3)), [1]), drive(guard(S.A(3)), [2]), drive(guard(S.B), []));
console.log(drive(cast(), [S.A(2), S.A(5)]), drive(cast(), [S.A(2), S.B]), drive(cast(), [S.B]));
console.log(drive(new Derived().run(), [S.A(3)]));
const iterator = later();
iterator.next().then((first) =>
  iterator.next(Promise.resolve(S.A(9))).then((last) => console.log(first.value, last.value)),
);
"#);
    assert_eq!(
        out,
        [
            r#""subject" 4 "subject" 0"#,
            r#"3 "one 3" 3 "other" "other""#,
            "1 2 7 1 2 2 1 0",
            "7 30",
            "1 9",
        ]
    );
}

#[test]
fn runtime_values_hoisted_out_of_unbraced_bodies_stay_under_their_parent() {
    require_toolchain!();
    let out = run(r#"
variant S { A(n: number), B }
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const log: string[] = [];
function note(x: unknown) { log.push(String(x)); }
function f(s: S, c: boolean) {
  if (c) return match (s) { A(n) => n, B => 0 };
  return -1;
}
function loops(s: S, xs: number[]) {
  for (const q of xs) note(match (s) { A(n) => n + q, B => q });
  let i = 0;
  while (i++ < 2) note(match (s) { A(n) => n, B => -1 });
  outer: for (const q of match (s) { A(n) => [n, n + 1], B => [] }) { if (q > 5) continue outer; note(q); }
  do note(match (s) { A(n) => -n, B => 0 }); while (false);
  lbl: note(match (s) { A(n) => n * 10, B => 0 });
}
function tries(c: boolean, r: R): R {
  if (c) note(try r); else note("else");
  if (c) note(result { const v = try r; return v + 1; }.kind);
  if (c) for (let k = try r; k < 6; k++) note(k);
  return { kind: "Ok", value: 0 };
}
console.log(f(S.A(3), true), f(S.A(3), false), f(S.B, true));
loops(S.A(5), [1, 2]);
console.log(log.join(","));
log.length = 0;
console.log(tries(true, { kind: "Err", error: "e" }).kind, tries(false, { kind: "Ok", value: 1 }).kind, tries(true, { kind: "Ok", value: 4 }).kind);
console.log(log.join(","));
"#);
    assert_eq!(out, ["3 -1 0", "6,7,5,5,5,-5,50", "Err Ok Ok", "else,4,Ok,4,5"]);
}

#[test]
fn runtime_an_if_let_as_an_unbraced_body_keeps_its_parent_and_its_else() {
    require_toolchain!();
    let out = run(r#"
variant O { Some(value: number), None }
function f(xs: O[]): number {
  let t = 0;
  for (const x of xs) if let Some(value) = x { t += value; } else { break; }
  return t;
}
function g(c: boolean, x: O): number {
  if (c) if let Some(value) = x { return value; } else { return 2; }
  else { return 3; }
}
function h(c: boolean, x: O): number {
  if (c) if let Some(value) = x { return value; }
  else { return 3; }
  return 4;
}
function k(xs: O[]): number {
  let t = 0;
  outer: for (const x of xs) if let Some(value) = x { if (value > 5) continue outer; t += value; }
  return t;
}
console.log(f([O.Some(1), O.Some(2), O.None, O.Some(9)]));
console.log(g(true, O.Some(1)), g(true, O.None), g(false, O.None));
console.log(h(true, O.Some(1)), h(true, O.None), h(false, O.None));
console.log(k([O.Some(1), O.Some(7), O.Some(2)]));
"#);
    assert_eq!(out, ["3", "1 2 3", "1 3 4", "3"]);
}

#[test]
fn runtime_a_var_let_else_as_an_unbraced_body_stays_under_its_parent() {
    require_toolchain!();
    let out = run(r#"
variant O { Some(value: number), None }
function h(c: boolean, o: O): number | undefined {
  const read = () => hv;
  if (c) var Some(value: hv) = o else { return 0; };
  return read();
}
function w(xs: O[]): number {
  const read = () => wv;
  let i = 0;
  while (i < xs.length) var Some(value: wv) = xs[i++] else { break; };
  return read() ?? -1;
}
function e(c: boolean, o: O): number | undefined {
  if (c) return -3; else var Some(value: ev) = o else { return 0; };
  return ev;
}
function l(o: O): number {
  lbl: var Some(value: lv) = o else { return -2; };
  return lv + 1;
}
console.log(h(true, O.Some(5)), h(true, O.None), h(false, O.Some(5)));
console.log(w([O.Some(1), O.Some(2), O.None, O.Some(9)]), w([]));
console.log(e(true, O.None), e(false, O.Some(4)), e(false, O.None));
console.log(l(O.Some(1)), l(O.None));
"#);
    assert_eq!(out, ["5 0 undefined", "2 -1", "-3 4 0", "2 -2"]);
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
fn an_unexpected_value_guard_reports_every_scrutinee_type() {
    require_toolchain!();
    let out = run(r#"
variant V { A, B }
const cyclic: { kind: string; self?: unknown } = { kind: "Z" };
cyclic.self = cyclic;
const values: unknown[] = [
  2n, -5n, Symbol("s"), "zz", 3, NaN, undefined, null, true,
  { toJSON() { throw new Error("toJSON"); } }, cyclic, () => 1, { kind: "Q" },
];
function statement(n: unknown): string {
  return match (n) { 1n => "one", "a" => "a" };
}
function chain(n: unknown, ok: boolean): string {
  return match (n) { 1n if ok => "one", "a" => "a" };
}
type Item = { run: (x: number) => number };
const pair2 = (a: Item, b: Item) => a.run(1) + b.run(1);
function inline(n: unknown): number {
  return pair2(
    match (n) { 1n => ({ run: x => x }), "a" => ({ run: x => x + 1 }) },
    match (n) { 1n => ({ run: x => x }), "a" => ({ run: x => x + 1 }) },
  );
}
function kind(v: V): string {
  return match (v) { A => "a", B => "b" };
}
function pair(a: V, b: V): string {
  return match (a, b) { (A, A) => "aa", (B, _) => "b", (A, B) => "ab" };
}
const report = (run: () => unknown) => {
  try { run(); console.log("returned"); }
  catch (error) { console.log(error instanceof Error ? error.message : "not an Error: " + String(error)); }
};
for (const value of values) {
  report(() => statement(value));
  report(() => chain(value, true));
  report(() => inline(value));
  if (value !== null && value !== undefined) report(() => kind(value as V));
}
report(() => pair(V.A, 7n as unknown as V));
report(() => pair(V.A, { kind: "Nope" } as unknown as V));
"#);
    let unexpected: Vec<&str> = out
        .iter()
        .map(String::as_str)
        .filter(|line| !line.starts_with("tt match: unexpected "))
        .collect();
    assert!(unexpected.is_empty(), "{out:#?}");
    for shown in [
        "2n",
        "-5n",
        "Symbol(s)",
        "\"zz\"",
        "3",
        "NaN",
        "undefined",
        "null",
        "true",
        "object",
        "function",
        "{\"kind\":\"Q\"}",
    ] {
        assert!(
            out.contains(&format!("tt match: unexpected literal {shown}")),
            "{shown}: {out:#?}"
        );
    }
    assert!(out.contains(&"tt match: unexpected case 2n".to_string()), "{out:#?}");
    assert!(
        out.contains(&"tt match: unexpected case {\"kind\":\"Q\"}".to_string()),
        "{out:#?}"
    );
    assert!(
        out.contains(&"tt match: unexpected case [{\"kind\":\"A\"},7n]".to_string()),
        "{out:#?}"
    );
    assert!(
        out.contains(&"tt match: unexpected case [{\"kind\":\"A\"},{\"kind\":\"Nope\"}]".to_string()),
        "{out:#?}"
    );
}

#[test]
fn an_unexpected_value_guard_survives_shadowed_globals() {
    require_toolchain!();
    let out = run(r#"
const String = "shadow";
const JSON = 1;
function pick(n: unknown): number {
  return match (n) { 1n => 1 };
}
for (const value of [2n, Symbol("s"), 3, "a", { a: 1 }]) {
  try { pick(value); } catch (error) { console.log((error as Error).message); }
}
console.log(String, JSON);
"#);
    assert_eq!(
        out,
        [
            "tt match: unexpected literal 2n",
            "tt match: unexpected literal Symbol(s)",
            "tt match: unexpected literal 3",
            "tt match: unexpected literal \"a\"",
            "tt match: unexpected literal {\"a\":1}",
            "shadow 1",
        ]
    );
}

#[test]
fn runtime_guarded_all_wildcard_arm_is_decided_by_its_guard() {
    require_toolchain!();
    let out = run(r#"
variant T { A, B }
type Item = { run: (x: number) => number };
function pair(a: Item, b: Item): number { return a.run(0) * 10 + b.run(0); }
function stmt(a: T, b: T, cond: boolean): number {
  return match (a, b) {
    (A, _) => 1,
    (_, _) if cond => 2,
    _ => 3,
  };
}
function last(a: T, b: T, cond: boolean): number {
  return match (a, b) {
    (A, _) => 1,
    (_, _) if cond => 2,
    (_, _) => 3,
  };
}
function selected(a: T, b: T, cond: boolean): number {
  let seen = 0;
  const consume = (item: Item) => { seen = item.run(0); };
  consume(match (a, b) {
    (A, _) => ({ run: x => x + 1 }),
    (_, _) if cond => ({ run: x => x + 2 }),
    (_, _) => ({ run: x => x + 3 }),
  });
  return seen;
}
function inline(a: T, b: T, cond: boolean): number {
  return pair(
    match (a, b) { (A, _) => ({ run: x => x + 1 }), (_, _) if cond => ({ run: x => x + 2 }), (_, _) => ({ run: x => x + 3 }) },
    match (b, a) { (A, _) => ({ run: x => x + 4 }), (_, _) if !cond => ({ run: x => x + 5 }), _ => ({ run: x => x + 6 }) },
  );
}
function literal(n: number, cond: boolean): string {
  return match (n) {
    1 if cond => "one",
    1 | 2 => "small",
    _ => "other",
  };
}
console.log(stmt(T.A, T.B, false), stmt(T.B, T.A, true), stmt(T.B, T.A, false));
console.log(last(T.A, T.B, false), last(T.B, T.A, true), last(T.B, T.A, false));
console.log(selected(T.A, T.B, false), selected(T.B, T.A, true), selected(T.B, T.A, false));
console.log(inline(T.A, T.B, true), inline(T.B, T.B, true), inline(T.B, T.A, false));
console.log(literal(1, true), literal(1, false), literal(3, true));
"#);
    assert_eq!(out, ["1 2 3", "1 2 3", "1 2 3", "16 26 34", "one small other"]);
}

#[test]
fn a_hoisted_value_inside_a_pipeline_operand_runs_once_in_source_order() {
    require_toolchain!();
    let out = run(r#"
variant E { A(value: number), B }
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const order: string[] = [];
const mark = <T,>(name: string, value: T): T => { order.push(name); return value; };
const subject = (name: string): E => { order.push(name); return E.A(1); };
const add = (value: number) => { order.push("call"); return value + 1; };
const callee = () => { order.push("callee"); return add; };
const make = (n: number) => { order.push("make"); return (value: number) => { order.push("apply"); return value + n; }; };
const step = () => { order.push("step"); return (value: number) => { order.push("apply"); return value * 10; }; };
const okay = (name: string, value: number): R => { order.push(name); return { kind: "Ok", value }; };
const fail = (name: string): R => { order.push(name); return { kind: "Err", error: name }; };
const report = (value: unknown) => { console.log(order.join(","), String(value)); order.length = 0; };
report(callee()(match (subject("head")) { A(value) => value, B => 0 }) |> step());
report(mark("left", 1) + match (subject("right")) { A(value) => value, B => 0 } |> step());
report(match (subject("head")) { A(value) => value, B => 0 } + mark("right", 1) |> step());
report([mark("first", 1), match (subject("second")) { A(value) => value, B => 0 }] |> (pair => pair.map(n => n + 1)));
report(-match (subject("head")) { A(value) => value, B => 0 } |> step());
report((match (subject("head")) { A(value) => value, B => 0 }).toFixed(1) |> Number);
report(`${callee()(match (subject("head")) { A(value) => value, B => 0 })}` |> Number);
report(mark("head", 3) |> make(match (subject("arg")) { A(value) => value, B => 0 }));
report(callee()(match (subject("one")) { A(value) => value, B => 0 }) + callee()(match (subject("two")) { A(value) => value, B => 0 }) |> step());
report(mark("cond", true) && callee()(match (subject("branch")) { A(value) => value, B => 0 }) |> String);
report(mark("cond", false) && callee()(match (subject("skipped")) { A(value) => value, B => 0 }) |> String);
function lifted(ok: boolean): R {
  const value = callee()(try (ok ? okay("try", 4) : fail("err"))) |> step();
  return { kind: "Ok", value };
}
const first = lifted(true);
report(first.kind === "Ok" ? first.value : first.error);
const second = lifted(false);
report(second.kind === "Ok" ? second.value : second.error);
"#);
    assert_eq!(
        out,
        [
            "callee,head,call,step,apply 20",
            "left,right,step,apply 20",
            "head,right,step,apply 20",
            "first,second 2,2",
            "head,step,apply -10",
            "head 1",
            "callee,head,call 2",
            "head,arg,make,apply 4",
            "callee,one,call,callee,two,call,step,apply 40",
            "cond,callee,branch,call 2",
            "cond false",
            "callee,try,call,step,apply 50",
            "callee,err err",
        ]
    );
}

#[test]
fn a_hoisted_value_in_a_member_step_runs_after_the_piped_value_and_its_method() {
    require_toolchain!();
    let out = run(r#"
variant E { A(value: number), B }
type N = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
type R = { kind: "Ok"; value: Box } | { kind: "Err"; error: string };
const order: string[] = [];
const mark = <T,>(name: string, value: T): T => { order.push(name); return value; };
const subject = (name: string): E => { order.push(name); return E.A(1); };
class Box {
  constructor(readonly n: number) {}
  get add() { order.push("get"); return Box.prototype.addTo; }
  addTo(amount: number): Box { order.push("call"); return new Box(this.n + amount); }
}
const factory = {
  get make() { order.push("get"); return (amount: number) => { order.push("make"); return (value: number) => { order.push("apply"); return value + amount; }; }; },
};
const okay = (name: string, value: number): N => { order.push(name); return { kind: "Ok", value }; };
const fail = (name: string): N => { order.push(name); return { kind: "Err", error: name }; };
const report = (value: unknown) => { console.log(order.join(","), String(value)); order.length = 0; };
report(mark("head", new Box(1)) |> .add(match (subject("arg")) { A(value) => value, B => 0 }) |> .n);
report(mark("head", new Box(1)) |> .add(match (subject("one")) { A(value) => value, B => 0 }) |> .add(match (subject("two")) { A(value) => value, B => 0 }) |> .n);
report(mark("head", new Box(1)) |> .add(1).add(match (subject("arg")) { A(value) => value, B => 0 }).n);
report(mark("head", { list: [10, 20] }) |> .list[match (subject("index")) { A(value) => value, B => 0 }]);
report(mark("head", new Box(1) as Box | undefined) |> ?.add(match (subject("arg")) { A(value) => value, B => 0 }) |> String);
report(mark("head", undefined as Box | undefined) |> ?.add(match (subject("skipped")) { A(value) => value, B => 0 }) |> String);
report(mark("head", 2) |> factory.make(match (subject("arg")) { A(value) => value, B => 0 }));
function lifted(ok: boolean): R {
  const value = mark("head", new Box(1)) |> .add(try (ok ? okay("try", 4) : fail("err")));
  return { kind: "Ok", value };
}
const first = lifted(true);
report(first.kind === "Ok" ? first.value.n : first.error);
const second = lifted(false);
report(second.kind === "Ok" ? second.value.n : second.error);
"#);
    assert_eq!(
        out,
        [
            "head,get,arg,call 2",
            "head,get,one,call,get,two,call 3",
            "head,get,call,get,arg,call 3",
            "head,index 20",
            "head,get,arg,call [object Object]",
            "head undefined",
            "head,get,arg,make,apply 3",
            "head,get,try,call 5",
            "head,get,err err",
        ]
    );
}

#[test]
fn a_pipeline_whose_steps_change_the_value_type_compiles_and_runs() {
    require_toolchain!();
    let out = run(r#"
const flag = Math.random() >= 0;
const pick = (value: { kind: "a" } | { kind: "b" }): string => value.kind;
const lengths = match (1) { _ => [1] } |> (p => p.length);
const text: string = match (flag) { true => [1, 2], false => [3] } |> (p => p.length) |> String;
const count: number = match (flag) { true => "xy", false => "z" } |> .length |> (n => [n, n]) |> .length;
const kind = match (flag) { true => ({ kind: "a" }), false => ({ kind: "b" }) } |> pick;
const mapped: string[] = [match (flag) { true => 1, false => 2 }] |> .map(n => n + 1) |> .map(String);
console.log(lengths, text, count, kind, mapped.join(","));
"#);
    assert_eq!(out, ["1 2 2 a 2"]);
}

#[test]
fn a_try_in_a_template_in_a_pipeline_runs_after_the_callee_it_is_an_argument_of() {
    require_toolchain!();
    let out = run(r#"
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
type S = { kind: "Ok"; value: string } | { kind: "Err"; error: string };
const order: string[] = [];
const wrap = (value: number) => { order.push("call"); return `<${value}>`; };
const callee = () => { order.push("callee"); return wrap; };
const suffix = (tail: string) => { order.push("step"); return (value: string) => { order.push("apply"); return value + tail; }; };
const okay = (name: string, value: number): R => { order.push(name); return { kind: "Ok", value }; };
const fail = (name: string): R => { order.push(name); return { kind: "Err", error: name }; };
const report = (value: S) => { console.log(order.join(","), value.kind === "Ok" ? value.value : value.error); order.length = 0; };
function head(ok: boolean): S {
  const value = `${callee()(try (ok ? okay("try", 1) : fail("err")))}!` |> String;
  return { kind: "Ok", value };
}
function step(ok: boolean): S {
  const value = "v" |> suffix(`${callee()(try (ok ? okay("try", 2) : fail("err")))}`);
  return { kind: "Ok", value };
}
function nested(ok: boolean): S {
  const value = callee()(`${callee()(try (ok ? okay("try", 3) : fail("err")))}`.length) |> String;
  return { kind: "Ok", value };
}
report(head(true));
report(head(false));
report(step(true));
report(step(false));
report(nested(true));
report(nested(false));
"#);
    assert_eq!(
        out,
        [
            "callee,try,call <1>!",
            "callee,err err",
            "callee,try,call,step,apply v<2>",
            "callee,err err",
            "callee,callee,try,call,call <3>",
            "callee,callee,err err",
        ]
    );
}

#[test]
fn runtime_assignment_evaluates_its_target_before_a_hoisted_right_operand() {
    require_toolchain!();
    let out = run(r#"
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const log: string[] = [];
const ok = <T,>(value: T): R<T> => { log.push("rhs"); return { kind: "Ok", value }; };
const done = <T,>(value: T): R<T> => ({ kind: "Ok", value });
const err = (): R<number> => { log.push("rhs"); return { kind: "Err", error: "e" }; };
let n = 1;
let box = { v: 0, s: "a" };
const first = box;
const target = () => { log.push("target"); return box; };
const key = (): "v" => { log.push("key"); return "v"; };
const report = (label: string, result: R<unknown>) => {
  console.log(label, JSON.stringify(result), n, first.v, first.s, log.join(","));
  n = 1; box = first; first.v = 0; first.s = "a"; log.length = 0;
};
function compound(): R<number> { n += try (n = 100, ok(5)); return done(n); }
function member(): R<number> { target().v = try ok(7); return done(box.v); }
function failed(): R<number> { target().v = try err(); return done(box.v); }
function computed(): R<number> { target()[key()] += try (first.v = 50, ok(2)); return done(first.v); }
function text(): R<string> { box.s += try (box.s = "q", ok("b")); return done(box.s); }
class Counter {
  #count = 1;
  add(): R<number> { this.#count *= try (this.#count = 10, ok(3)); return done(this.#count); }
}
report("compound", compound());
report("member", member());
report("failed", failed());
report("computed", computed());
report("text", text());
report("private", new Counter().add());
let m = 1;
m += match (m) { 1 => { m = 100; return 5; }, _ => 0 };
console.log("match", m);
"#);
    assert_eq!(
        out,
        [
            r#"compound {"kind":"Ok","value":6} 6 0 a rhs"#,
            r#"member {"kind":"Ok","value":7} 1 7 a target,rhs"#,
            r#"failed {"kind":"Err","error":"e"} 1 0 a target,rhs"#,
            r#"computed {"kind":"Ok","value":2} 1 2 a target,key,rhs"#,
            r#"text {"kind":"Ok","value":"ab"} 1 0 ab rhs"#,
            r#"private {"kind":"Ok","value":3} 1 0 a rhs"#,
            "match 6",
        ]
    );
}

#[test]
fn an_assignment_target_keeps_the_narrowing_typescript_gives_it() {
    require_toolchain!();
    let (valid, diagnostics) = typecheck(
        r#"
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
declare function read(): R<number>;
export function assigned(): R<number> {
  const state: { value?: number } = {};
  state.value = try read();
  return { kind: "Ok", value: state.value.toFixed().length };
}
export function compound(): R<number> {
  const state: { value: number | string } = { value: 1 };
  state.value = 0;
  state.value += try read();
  return { kind: "Ok", value: state.value };
}
export class Holder {
  value;
  constructor(r: R<number>) {
    this.value = result { return try r; };
  }
}
"#,
    );
    assert!(valid, "{diagnostics}");
}

#[test]
fn runtime_a_value_inside_a_let_else_or_if_let_subject_keeps_evaluation_order() {
    require_toolchain!();
    // TASK-544: the callee of the subject's call is read before the value
    // in its argument runs, as in `opt(match ...)` anywhere else.
    let out = run(r#"
type Opt = { kind: "Some"; value: number } | { kind: "None" };
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const trace: string[] = [];
let pick = (n: number): Opt => { trace.push("pick" + n); return n > 1 ? { kind: "Some", value: n } : { kind: "None" }; };
function swap(n: number): number {
  trace.push("swap");
  const first = pick;
  pick = (m) => { trace.push("late" + m); return first(m); };
  return n;
}
function viaMatch(k: number): number {
  if let Some(value: w) = pick(match (k) { 1 => swap(1), _ => swap(3) }) { return w; }
  else if let Some(value: z) = pick(match (k) { 1 => swap(5), _ => 0 }) { return z * 10; }
  return -1;
}
function readOpt(n: number): R<Opt> {
  return n < 0 ? { kind: "Err", error: "neg" } : { kind: "Ok", value: n > 1 ? { kind: "Some", value: n } : { kind: "None" } };
}
function inResult(n: number) {
  return result {
    const Some(value: v) = (try readOpt(n)) else { return -2; };
    if let Some(value: q) = [try readOpt(n + 1)][0] { return v + q; }
    return v;
  };
}
console.log(viaMatch(1), JSON.stringify(trace));
console.log(JSON.stringify([inResult(2), inResult(1), inResult(-1)]));
"#);
    assert_eq!(
        out,
        [
            r#"50 ["swap","pick1","swap","late5","pick5"]"#,
            r#"[{"kind":"Ok","value":5},{"kind":"Ok","value":-2},{"kind":"Err","error":"neg"}]"#,
        ]
    );
}

#[test]
fn runtime_an_optional_member_call_is_skipped_only_when_its_receiver_is_nullish() {
    require_toolchain!();
    // TASK-545: `o?.m(x)` short-circuits when `o` is nullish; a present
    // receiver without `m` throws, as JavaScript's own call does.
    let out = run(r#"
type M = { base?: number; m(v: number): number };
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const trace: string[] = [];
function arg(tag: string): number { trace.push(tag); return 1; }
function read(tag: string): R { trace.push(tag); return { kind: "Ok", value: 2 }; }
const live: M = { base: 7, m(v: number): number { trace.push("this:" + (this === live)); return (this.base ?? 0) + v; } };
const missing = {} as M;
function attempt(f: () => unknown): string {
  try { return String(f()); } catch (e) { return (e as Error).constructor.name; }
}
function viaTry(o: M | null, tag: string): R {
  const called = o?.m(try read(tag));
  return { kind: "Ok", value: called ?? -1 };
}
function each(o: M | null, tag: string): string[] {
  return [
    attempt(() => o?.m(match (arg(tag + ":call")) { 1 => 1, _ => 0 })),
    attempt(() => o?.m?.(match (arg(tag + ":both")) { 1 => 1, _ => 0 })),
    attempt(() => JSON.stringify(viaTry(o, tag + ":try"))),
    attempt(() => o |> ?.m(match (arg(tag + ":pipe")) { 1 => 1, _ => 0 })),
  ];
}
console.log(JSON.stringify([each(live, "live"), each(null, "null"), each(missing, "missing")]));
console.log(JSON.stringify(trace));
"#);
    assert_eq!(
        out,
        [
            r#"[["8","8","{\"kind\":\"Ok\",\"value\":9}","8"],["undefined","undefined","{\"kind\":\"Ok\",\"value\":-1}","undefined"],["TypeError","undefined","TypeError","TypeError"]]"#,
            r#"["live:call","this:true","live:both","this:true","live:try","this:true","live:pipe","this:true","missing:call","missing:try","missing:pipe"]"#,
        ]
    );
}

#[test]
fn runtime_a_prototype_setter_name_is_an_own_property() {
    require_toolchain!();
    let out = run(r#"
variant V { __proto__(x: number), Other }
variant U { __proto__, Other }
variant W { A(__proto__: number, other?: number), B }
const v = V.__proto__(1);
console.log(JSON.stringify(v), Object.keys(V).join(","), Object.getPrototypeOf(V) === Object.prototype);
console.log(JSON.stringify(U.__proto__), Object.getPrototypeOf(U) === Object.prototype);
const w = W.A(5);
console.log(Object.keys(w).join(","), Object.getPrototypeOf(w) === Object.prototype);
console.log(match (v) { __proto__(x) => x, Other => 0 }, match (w) { A(__proto__) => __proto__, B => 0 });
"#);
    assert_eq!(
        out,
        [
            r#"{"kind":"__proto__","x":1} __proto__,Other true"#,
            r#"{"kind":"__proto__"} true"#,
            "kind,__proto__ true",
            "1 5",
        ]
    );
}

#[test]
fn runtime_a_try_statement_in_a_result_block_lowers_the_values_of_its_operand() {
    require_toolchain!();
    // TASK-549: the operand's values and its callee capture run in the
    // statement's prelude inside a `result` block, as they do in a function.
    let out = run(r#"
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const trace: string[] = [];
let r = (n: number): R => { trace.push("r" + n); return n > 15 ? { kind: "Ok", value: n } : { kind: "Err", error: "small" + n }; };
function swap(n: number): number {
  trace.push("swap");
  const first = r;
  r = (m) => { trace.push("late"); return first(m); };
  return n;
}
const id = <T,>(x: T) => x;
const g = (x: R): R => x;
function viaConst(v: number) { return result { const x = try r(match (v) { 1 => swap(10), _ => swap(20) }); return x; }; }
function viaLet(v: number) { return result { let x = try r(match (v) { 1 => 10, _ => 20 }); x += 1; return x; }; }
function propagateOnly(v: number) { return result { try r(match (v) { 1 => 10, _ => 20 }); return 0; }; }
function viaPipeline(v: number) { return result { const x = try (match (v) { 1 => r(10), _ => r(20) } |> id); return x; }; }
function viaCall(v: number) { return result { const x = try id(match (v) { 1 => r(10), _ => r(20) }); return x; }; }
function viaResult(v: number) {
  return result { const x = try r(result { const y = try r(v); return y; } |> g |> (q => q.kind === "Ok" ? q.value : 0)); return x; };
}
const base = r;
console.log(JSON.stringify([viaConst(1), viaConst(2)]), JSON.stringify(trace));
r = base;
console.log(JSON.stringify([viaLet(1), viaLet(2), propagateOnly(1), propagateOnly(2)]));
console.log(JSON.stringify([viaPipeline(1), viaPipeline(2), viaCall(1), viaCall(2), viaResult(20), viaResult(1)]));
"#);
    assert_eq!(
        out,
        [
            r#"[{"kind":"Err","error":"small10"},{"kind":"Ok","value":20}] ["swap","r10","swap","late","r20"]"#,
            r#"[{"kind":"Err","error":"small10"},{"kind":"Ok","value":21},{"kind":"Err","error":"small10"},{"kind":"Ok","value":0}]"#,
            r#"[{"kind":"Err","error":"small10"},{"kind":"Ok","value":20},{"kind":"Err","error":"small10"},{"kind":"Ok","value":20},{"kind":"Ok","value":20},{"kind":"Err","error":"small0"}]"#,
        ]
    );
}

#[test]
fn runtime_a_value_inside_a_region_return_argument_runs_in_the_return_s_prelude() {
    require_toolchain!();
    // TASK-549: a value that a call or an operator consumes inside the
    // argument of a return leaving a `result` block or a match block arm
    // is a value of the return statement, lowered before it with its
    // callee captured first; a returned template keeps its one exit.
    let out = run(r#"
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
const trace: string[] = [];
function ok(n: number): R { return { kind: "Ok", value: n }; }
let s = (n: number): string => { trace.push("s" + n); return "s" + n; };
function swap(n: number): number {
  trace.push("swap");
  const first = s;
  s = (m) => { trace.push("late"); return first(m); };
  return n;
}
function inResult(v: number) {
  return result {
    const q = try ok(1);
    if (v > 5) return s(match (v) { 6 => swap(6), _ => 2 }) + q;
    return [match (v) { 1 => "a", _ => "b" }, match (q) { 1 => "c", _ => "d" }].join("") + q;
  };
}
function inArm(v: number) {
  const x = match (v) {
    1 => { const q = 1; if (q > 0) return s(match (v) { 1 => swap(1), _ => 2 }) + q; return "n"; },
    2 => { return `${match (v) { 2 => "t", _ => "u" }}-`; },
    _ => "x",
  };
  return x;
}
function template(v: number) {
  return result { const q = try ok(1); return `${match (v) { 1 => "a", _ => "b" }}-${q}`; };
}
const base = s;
console.log(JSON.stringify([inResult(6), inResult(1)]), JSON.stringify(trace));
s = base;
trace.length = 0;
console.log(JSON.stringify([inArm(1), inArm(2), inArm(3)]), JSON.stringify(trace));
console.log(JSON.stringify([template(1), template(2)]));
"#);
    assert_eq!(
        out,
        [
            r#"[{"kind":"Ok","value":"s61"},{"kind":"Ok","value":"ac1"}] ["swap","s6"]"#,
            r#"["s11","t-","x"] ["swap","s1"]"#,
            r#"[{"kind":"Ok","value":"a-1"},{"kind":"Ok","value":"b-1"}]"#,
        ]
    );
}

#[test]
fn runtime_a_propagated_call_around_a_match_does_not_share_the_match_s_slot() {
    require_toolchain!();
    // TASK-550: the Result `r(...)` returns is read into the propagation's
    // own temporary; the match's slot keeps holding the number it wrote.
    let out = run(r#"
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function r(n: number): R { return n > 1 ? { kind: "Ok", value: n } : { kind: "Err", error: "small" + n }; }
function sum(v: number): R {
  const y = 1 + try r(match (v) { 1 => 1, _ => 2 });
  return r(y - 1 + 10);
}
function listed(v: number): R {
  const a = [try r(match (v) { 1 => 1, _ => 2 })];
  console.log(try r(match (v) { 1 => 3, _ => 4 }));
  return { kind: "Ok", value: a[0] };
}
function inResult(v: number) {
  return result {
    const y = 1 + try r(match (v) { 1 => 1, _ => 2 });
    const a = [try r(match (v) { 1 => 1, _ => 2 })];
    console.log(try r(match (v) { 1 => 3, _ => 4 }));
    return y + a[0];
  };
}
console.log(JSON.stringify([sum(1), sum(2), listed(1), listed(2), inResult(1), inResult(2)]));
"#);
    assert_eq!(
        out,
        [
            "4",
            "4",
            r#"[{"kind":"Err","error":"small1"},{"kind":"Ok","value":12},{"kind":"Err","error":"small1"},{"kind":"Ok","value":2},{"kind":"Err","error":"small1"},{"kind":"Ok","value":5}]"#,
        ]
    );
}

#[test]
fn runtime_a_member_step_s_simple_key_names_its_member() {
    require_toolchain!();
    // TASK-554: a literal or identifier key is read where the member is,
    // right after the receiver, with the type TypeScript gives it there.
    let out = run(r#"
const trace: string[] = [];
const obj = { m(x: number) { return x * 2; }, n(x: number) { return x * 5; } };
function h(n: number) { trace.push("head"); return n + 1; }
function getFns(): [(n: number) => number, string] { return [(n) => n * 3, "s"]; }
let key: "m" | "n" = "m";
function receiver() { trace.push("receiver"); key = "n"; return obj; }
const c = (flow |> obj["m"])(3);
const d = h(3) |> (() => obj)()["m"];
const e = h(3) |> getFns()[0];
const f = h(3) |> obj[`n`];
const g = h(3) |> receiver()[key];
console.log(c, d, e, f, g, JSON.stringify(trace));
"#);
    assert_eq!(
        out,
        [r#"6 8 12 20 20 ["head","head","head","head","receiver"]"#]
    );
}
