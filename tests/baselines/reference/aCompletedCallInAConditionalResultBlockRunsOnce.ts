//// [aCompletedCallInAConditionalResultBlockRunsOnce.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
const calls: unknown[] = [];
const g = (n: number) => { calls.push(n); return n * 10; };
function r(n: number): TResult<number, string> { calls.push(n); return Result.Ok(n); }
variant V { A(n: number), B }
export function called(c: boolean, v: V) {
  return c ? result { const z = g(match (v) { A(n) => n, B => -1 }); return z + (try r(7)); } : null;
}
export function tried(c: boolean, v: V) {
  return c ? result { const z = try r(match (v) { A(n) => n, B => -1 }); return z + (try r(7)); } : null;
}
console.log(JSON.stringify(called(true, V.B)), JSON.stringify(tried(true, V.A(2))), JSON.stringify(calls));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aCompletedCallInAConditionalResultBlockRunsOnce.ts]
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
const calls: unknown[] = [];
const g = (n: number) => { calls.push(n); return n * 10; };
function r(n: number): TResult<number, string> { calls.push(n); return Result.Ok(n); }
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
export function called(c: boolean, v: V) {
  let $tt_v2: (Result.TErr<string>) | ({
    kind: "Ok";
    value: number;
}) | (null);
  if (c) {
    $tt_y_v0: {
      let $tt_v3: number;
      const $tt_v4: typeof g = (g);
      {
        const $tt_m = v;
        switch ($tt_m.kind) {
          case "A": {
            const { n } = $tt_m;
            $tt_v3 = $tt_v4(n);
            break;
          }
          case "B": {
            $tt_v3 = $tt_v4(-1);
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
      const z = $tt_v3; let $tt_v5: number;
      const $tt_v6: typeof z = (z);
      const $tt_t0 = r(7);
      if (!("value" in $tt_t0)) {
        $tt_v2 = $tt_t0;
        break $tt_y_v0;
      }
      $tt_v5 = $tt_t0.value;
      { $tt_v2 = { kind: "Ok" as const, value: $tt_v6 + ($tt_v5) }; break $tt_y_v0; }
    }
  } else {
    $tt_v2 = null;
  }
  
  return $tt_v2;
}
export function tried(c: boolean, v: V) {
  let $tt_v9: (Result.TErr<string>) | ({
    kind: "Ok";
    value: number;
}) | (null);
  if (c) {
    $tt_y_v7: {
      let $tt_v10: TResult<number, string>;
      const $tt_v11: typeof r = (r);
      {
        const $tt_m = v;
        switch ($tt_m.kind) {
          case "A": {
            const { n } = $tt_m;
            $tt_v10 = $tt_v11(n);
            break;
          }
          case "B": {
            $tt_v10 = $tt_v11(-1);
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
      const $tt_t1 = $tt_v10;
      if (!("value" in $tt_t1)) {
        $tt_v9 = $tt_t1;
        break $tt_y_v7;
      }
      const z = $tt_t1.value; let $tt_v12: number;
      const $tt_v13: typeof z = (z);
      const $tt_t2 = r(7);
      if (!("value" in $tt_t2)) {
        $tt_v9 = $tt_t2;
        break $tt_y_v7;
      }
      $tt_v12 = $tt_t2.value;
      { $tt_v9 = { kind: "Ok" as const, value: $tt_v13 + ($tt_v12) }; break $tt_y_v7; }
    }
  } else {
    $tt_v9 = null;
  }
  
  return $tt_v9;
}
console.log(JSON.stringify(called(true, V.B)), JSON.stringify(tried(true, V.A(2))), JSON.stringify(calls));
