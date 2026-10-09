//// [aRebuiltOperandInsideACapturedOperandKeepsItsText.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
variant S { A, B }
function R<T>(x: T): TResult<T, string> { return Result.Ok(x); }
function p(): S { return S.A; }
function f(x: unknown) { return x; }
const g = { mm: (x: unknown) => x };
const c = true as boolean;
function t(): TResult<unknown, string> {
  const call = typeof g.mm?.(f(match (p()) { A => 1, B => 2 })) + typeof (try R(3));
  const branch = typeof (c ? (try R(5)) + "x" : 1) + typeof (try R(1));
  return Result.Ok([call, branch]);
}
console.log(JSON.stringify(t()));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aRebuiltOperandInsideACapturedOperandKeepsItsText.ts]
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
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
type S =
  | { kind: "A" }
  | { kind: "B" };
const S = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
function R<T>(x: T): TResult<T, string> { return Result.Ok(x); }
function p(): S { return S.A; }
function f(x: unknown) { return x; }
const g = { mm: (x: unknown) => x };
const c = true as boolean;
function t(): TResult<unknown, string> {
  let $tt_v5;
  let $tt_v1: number;
  const $tt_v3 = (g.mm);
  if ($tt_v3 != null) {
    let $tt_v0: number;
    const $tt_v2: typeof f = (f);
    {
      const $tt_m = p();
      switch ($tt_m.kind) {
        case "A": {
          $tt_v0 = 1;
          break;
        }
        case "B": {
          $tt_v0 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v5 = $tt_v3.call(g, $tt_v2($tt_v0));
  } else {
    $tt_v5 = undefined;
  }
  
  const $tt_v4 = (typeof $tt_v5);
  const $tt_t0 = R(3);
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v1 = $tt_t0.value;
  const call = $tt_v4 + typeof ($tt_v1);
  let $tt_v10: (string) | (number);
  let $tt_v7: number;
  if (c) {
    let $tt_v6: number;
    const $tt_t1 = R(5);
    if (!("value" in $tt_t1)) {
      return $tt_t1;
    }
    $tt_v6 = $tt_t1.value;
    $tt_v10 = ($tt_v6) + "x";
  } else {
    $tt_v10 = 1;
  }
  
  const $tt_v9 = (typeof ($tt_v10));
  const $tt_t2 = R(1);
  if (!("value" in $tt_t2)) {
    return $tt_t2;
  }
  $tt_v7 = $tt_t2.value;
  const branch = $tt_v9 + typeof ($tt_v7);
  return Result.Ok([call, branch]);
}
console.log(JSON.stringify(t()));
