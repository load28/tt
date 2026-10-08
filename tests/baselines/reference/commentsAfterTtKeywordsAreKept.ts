//// [commentsAfterTtKeywordsAreKept.tt] ////
import type { TResult } from "@tt/std";
export variant /*h1*/ V /*h2*/ { A, B }
variant /* a */ W<T> // b
{ A(x: T), B }
declare const v: V;
declare function r(): TResult<number, string>;
export const m = match /*g1*/ (v) { A => 1, B => 2 };
export function f(p = result /*pd*/ { const q = try r(); return q; }): TResult<number, string> {
  const a = try /*i1*/ r();
  const b = try
    // why
    r();
  const w = result /*l1*/ { const q = try r(); return q; };
  const A() = v else { return { kind: "Err", error: "" }; } /*e7*/;
  return { kind: "Ok", value: a + b };
}

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [commentsAfterTtKeywordsAreKept.ts]
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
function $tt_expr<T>(run: () => T): T { return run(); }
import type { TResult } from "./tt/index.js";
export type /*h1*/ V /*h2*/ =
  | { kind: "A" }
  | { kind: "B" };
export const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
type /* a */ W<T> // b
=
  | { kind: "A"; x: T }
  | { kind: "B" };
const W = {
  A: <T>(x: T): W<T> => ({ kind: "A", x }),
  B: { kind: "B" } as const,
};
declare const v: V;
declare function r(): TResult<number, string>;
let $tt_v0: number;
{
  const $tt_m = /*g1*/ v;
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
export const m = $tt_v0;
export function f(p = $tt_expr(() => {
  /*pd*/
  const $tt_t0 = r();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const q = $tt_t0.value; { return { kind: "Ok" as const, value: q }; }
  })): TResult<number, string> {
  const $tt_t1 = /*i1*/ r();
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  const a = $tt_t1.value;
  const $tt_t2 =
    // why
    r();
  if (!("value" in $tt_t2)) {
    return $tt_t2;
  }
  const b = $tt_t2.value;
  let $tt_v2: (import("./tt/index.js").TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v2: /*l1*/ {
    const $tt_t3 = r();
    if (!("value" in $tt_t3)) {
      $tt_v2 = $tt_t3;
      break $tt_v2;
    }
    const q = $tt_t3.value; { $tt_v2 = { kind: "Ok" as const, value: q }; break $tt_v2; }
  }
  const w = $tt_v2;
  const $tt_t4 = v;
  if ($tt_t4.kind !== "A") {
    return { kind: "Err", error: "" };
  }
  
  /*e7*/
  return { kind: "Ok", value: a + b };
}
