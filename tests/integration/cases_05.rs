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
    if !have("tsc") || !have("node") { return; }
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
    if !have("tsc") || !have("node") { return; }
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
    if !have("tsc") || !have("node") { return; }
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
