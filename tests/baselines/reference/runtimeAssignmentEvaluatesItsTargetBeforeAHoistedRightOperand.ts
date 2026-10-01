//// [runtimeAssignmentEvaluatesItsTargetBeforeAHoistedRightOperand.tt] ////

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

export {};


//// [runtimeAssignmentEvaluatesItsTargetBeforeAHoistedRightOperand.ts]

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
function compound(): R<number> { let $tt_v0: number;
let $tt_v1 = (n);
const $tt_t0 = (n = 100, ok(5));
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
$tt_v0 = $tt_t0.value;
n = $tt_v1 += $tt_v0; return done(n); }
function member(): R<number> { let $tt_v2: number;
const $tt_v3 = (target());
const $tt_t1 = ok(7);
if (!("value" in $tt_t1)) {
  return $tt_t1;
}
$tt_v2 = $tt_t1.value;
$tt_v3.v = $tt_v2; return done(box.v); }
function failed(): R<number> { let $tt_v4: number;
const $tt_v5 = (target());
const $tt_t2 = err();
if (!("value" in $tt_t2)) {
  return $tt_t2;
}
$tt_v4 = $tt_t2.value;
$tt_v5.v = $tt_v4; return done(box.v); }
function computed(): R<number> { let $tt_v6: number;
const $tt_v7 = (target());
const $tt_v8 = (key());
let $tt_v9 = ($tt_v7[$tt_v8]);
const $tt_t3 = (first.v = 50, ok(2));
if (!("value" in $tt_t3)) {
  return $tt_t3;
}
$tt_v6 = $tt_t3.value;
$tt_v7[$tt_v8] = $tt_v9 += $tt_v6; return done(first.v); }
function text(): R<string> { let $tt_v10: string;
let $tt_v11 = (box.s);
const $tt_t4 = (box.s = "q", ok("b"));
if (!("value" in $tt_t4)) {
  return $tt_t4;
}
$tt_v10 = $tt_t4.value;
box.s = $tt_v11 += $tt_v10; return done(box.s); }
class Counter {
  #count = 1;
  add(): R<number> { let $tt_v12: number;
  let $tt_v13 = (this.#count);
  const $tt_t5 = (this.#count = 10, ok(3));
  if (!("value" in $tt_t5)) {
    return $tt_t5;
  }
  $tt_v12 = $tt_t5.value;
  this.#count = $tt_v13 *= $tt_v12; return done(this.#count); }
}
report("compound", compound());
report("member", member());
report("failed", failed());
report("computed", computed());
report("text", text());
report("private", new Counter().add());
let m = 1;
let $tt_v14: number;
let $tt_v15 = (m);
{
  const $tt_m = m;
  switch ($tt_m) {
    case 1: {
      m = 100; $tt_v14 = 5; break;
    }
    default: {
      $tt_v14 = 0;
      break;
    }
  }
}
m = $tt_v15 += $tt_v14;
console.log("match", m);

export {};
