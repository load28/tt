//// [aResultBlockReturningANestedMatchWritesTheInnerMatch.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
variant V { A(n: number), B }
const r = (n: number): TResult<number, string> => (n > 0 ? Result.Ok(n) : Result.Err(`bad ${n}`));
function f(k: number, outer: V, inner: V) {
  const q = result { const z = try r(k); return match (outer) { A(n) => n + z, B => match (inner) { A(n) => n * z, B => -z } }; };
  return JSON.stringify(q);
}
console.log(f(2, V.A(1), V.B), f(2, V.B, V.A(5)), f(3, V.B, V.B), f(0, V.B, V.B));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aResultBlockReturningANestedMatchWritesTheInnerMatch.ts]
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
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const r = (n: number): TResult<number, string> => (n > 0 ? Result.Ok(n) : Result.Err(`bad ${n}`));
function f(k: number, outer: V, inner: V) {
  let $tt_v0: Result.TErr<string> | {
    kind: "Ok";
    value: number;
};
  $tt_v0: {
    const $tt_t0 = r(k);
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    const z = $tt_t0.value; {
      const $tt_m = outer;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v0 = { kind: "Ok" as const, value: n + z };
          break;
        }
        case "B": {
          {
            const $tt_m = inner;
            switch ($tt_m.kind) {
              case "A": {
                const { n } = $tt_m;
                $tt_v0 = { kind: "Ok" as const, value: n * z };
                break;
              }
              case "B": {
                $tt_v0 = { kind: "Ok" as const, value: -z };
                break;
              }
              default: {
                throw new Error("tt match: unexpected case " + $tt_show($tt_m));
              }
            }
          }
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    break $tt_v0;
  }
  const q = $tt_v0;
  return JSON.stringify(q);
}
console.log(f(2, V.A(1), V.B), f(2, V.B, V.A(5)), f(3, V.B, V.B), f(0, V.B, V.B));
