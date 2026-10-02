//// [aUsingForStatementAndAnIfFunctionClauseHostTtValues.tt] ////
import type { TResult } from "@tt/std";
variant O { A(n: number), B }
declare const o: O;
declare function res(): { n: number; [Symbol.dispose](): void };
declare function rr(): TResult<{ n: number; [Symbol.dispose](): void }, string>;
export function f(): TResult<number, string> {
  let t = 0;
  for (using q = res(), p = res(); t < 2; t++) {
    t += match (o) { A(n) => n + q.n + p.n, B => 0 };
  }
  for (using q = try rr(); t < 3; t++) {
    t += q.n;
  }
  return { kind: "Ok", value: t };
}
if (Math.random()) function g() { return match (o) { A(n) => n, B => 0 }; }

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aUsingForStatementAndAnIfFunctionClauseHostTtValues.ts]
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
type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare const o: O;
declare function res(): { n: number; [Symbol.dispose](): void };
declare function rr(): TResult<{ n: number; [Symbol.dispose](): void }, string>;
export function f(): TResult<number, string> {
  let t = 0;
  for (using q = res(), p = res(); t < 2; t++) {
    let $tt_v0: number;
    let $tt_v1 = (t);
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v0 = n + q.n + p.n;
          break;
        }
        case "B": {
          $tt_v0 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    t = $tt_v1 += $tt_v0;
  }
  let $tt_v2;
  const $tt_t0 = rr();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v2 = $tt_t0.value;
  for (using q = $tt_v2; t < 3; t++) {
    t += q.n;
  }
  return { kind: "Ok", value: t };
}
if (Math.random()) function g() { let $tt_v3: number;
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "A": {
      const { n } = $tt_m;
      $tt_v3 = n;
      break;
    }
    case "B": {
      $tt_v3 = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
return $tt_v3; }
