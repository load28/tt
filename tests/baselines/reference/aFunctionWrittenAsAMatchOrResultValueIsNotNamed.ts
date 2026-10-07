//// [aFunctionWrittenAsAMatchOrResultValueIsNotNamed.tt] ////
import type { TResult } from "@tt/std";
variant V { A, B }
const v = V.A as V;
const arrow = match (v) { A => () => 1, B => () => 2 };
const klass = match (v) { A => class { x = 1 }, B => class { x = 2 } };
const fn = match (v) { A => function () { return 1; }, B => function () { return 2; } };
const named = match (v) { A => function own() { return 1; }, B => () => 2 };
const asserted: () => number = match (v) { A => (() => 1) as () => number, B => () => 2 };
let assigned: (x: number) => number;
assigned = match (v) { A => (x) => x + 1, B => (x) => x };
function one(): TResult<number, string> { return { kind: "Ok", value: 1 }; }
function made(): TResult<() => number, string> {
  return result { const n = try one(); return () => n; };
}
const ternary = v.kind === "A" ? () => 1 : () => 2;
const fromResult = made();
console.log(JSON.stringify([
  arrow.name, klass.name, fn.name, named.name, asserted.name, assigned.name,
  fromResult.kind === "Ok" ? fromResult.value.name : "err", ternary.name,
]));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aFunctionWrittenAsAMatchOrResultValueIsNotNamed.ts]
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}
import type { TResult } from "./tt/index.js";
type V =
  | { kind: "A" }
  | { kind: "B" };
const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const v = V.A as V;
let $tt_v0: () => number;
{
  const $tt_m = v;
  switch ($tt_m.kind) {
    case "A": {
      const $tt_a0 = { value: (void 0, () => 1) };
      $tt_v0 = $tt_a0.value;
      break;
    }
    case "B": {
      const $tt_a1 = { value: (void 0, () => 2) };
      $tt_v0 = $tt_a1.value;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const arrow = $tt_v0;
let $tt_v1;
{
  const $tt_m = v;
  switch ($tt_m.kind) {
    case "A": {
      $tt_v1 = (void 0, class { x = 1 });
      break;
    }
    case "B": {
      $tt_v1 = (void 0, class { x = 2 });
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const klass = $tt_v1;
let $tt_v2: () => number;
{
  const $tt_m = v;
  switch ($tt_m.kind) {
    case "A": {
      const $tt_a2 = { value: (void 0, function () { return 1; }) };
      $tt_v2 = $tt_a2.value;
      break;
    }
    case "B": {
      const $tt_a3 = { value: (void 0, function () { return 2; }) };
      $tt_v2 = $tt_a3.value;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const fn = $tt_v2;
let $tt_v3: () => number;
{
  const $tt_m = v;
  switch ($tt_m.kind) {
    case "A": {
      const $tt_a4 = { value: function own() { return 1; } };
      $tt_v3 = $tt_a4.value;
      break;
    }
    case "B": {
      const $tt_a5 = { value: (void 0, () => 2) };
      $tt_v3 = $tt_a5.value;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const named = $tt_v3;
let $tt_v4: () => number;
{
  const $tt_m = v;
  switch ($tt_m.kind) {
    case "A": {
      $tt_v4 = (void 0, (() => 1) as () => number);
      break;
    }
    case "B": {
      $tt_v4 = (void 0, () => 2);
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const asserted: () => number = $tt_v4;
let assigned: (x: number) => number;
let $tt_v5: number;
{
  const $tt_m = v;
  switch ($tt_m.kind) {
    case "A": $tt_v5 = 0; break;
    case "B": $tt_v5 = 1; break;
    default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  }
}
assigned = ($tt_v5 === 0 ? (x) => x + 1 : (x) => x);
function one(): TResult<number, string> { return { kind: "Ok", value: 1 }; }
function made(): TResult<() => number, string> {
  let $tt_v6: TResult<() => number, string>;
  $tt_v6: {
    const $tt_t0 = one();
    if (!("value" in $tt_t0)) {
      $tt_v6 = $tt_t0;
      break $tt_v6;
    }
    const n = $tt_t0.value; { $tt_v6 = { kind: "Ok" as const, value: (void 0, () => n) }; break $tt_v6; }
  }
  return $tt_v6;
}
const ternary = v.kind === "A" ? () => 1 : () => 2;
const fromResult = made();
console.log(JSON.stringify([
  arrow.name, klass.name, fn.name, named.name, asserted.name, assigned.name,
  fromResult.kind === "Ok" ? fromResult.value.name : "err", ternary.name,
]));
