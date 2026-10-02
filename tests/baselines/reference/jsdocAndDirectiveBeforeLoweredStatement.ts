//// [jsdocAndDirectiveBeforeLoweredStatement.tt] ////
// A directive above a JSDoc comment governs the JSDoc's line, where nothing
// fails, so the statement below keeps its error. When the statement's
// lowering writes generated statements and the JSDoc moves down to the
// declaration it documents, the directive moves with it and still governs
// the JSDoc's line rather than the first generated statement.
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
variant S { A(n: number), B }
declare const s: S;
// @ts-ignore -- governs the documentation line below, not the declaration
/** The documented value. */
export const ungoverned: number = match (s) { A(n) => "a" + n, B => 0 };
export function ignored(r: (n: number) => TResult<number, string>): TResult<number, string> {
  // @ts-ignore -- governs the documentation line below, not the declaration
  /** The documented binding. */
  const v = try r("x");
  return Result.Ok(v);
}
export function expected(r: (n: number) => TResult<number, string>): TResult<number, string> {
    // @ts-expect-error -- governs the indented documentation line, so it is unused
    /** The indented documented binding. */
  const w = try r("y");
  return Result.Ok(w);
}
/** A directive below the JSDoc governs the statement's line. */
// @ts-expect-error -- a string where a number is declared
export const governed: number = match (s) { A(n) => "a" + n, B => "b" };

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [jsdocAndDirectiveBeforeLoweredStatement.ts]
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
// A directive above a JSDoc comment governs the JSDoc's line, where nothing
// fails, so the statement below keeps its error. When the statement's
// lowering writes generated statements and the JSDoc moves down to the
// declaration it documents, the directive moves with it and still governs
// the JSDoc's line rather than the first generated statement.
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
type S =
  | { kind: "A"; n: number }
  | { kind: "B" };
const S = {
  A: (n: number): S => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare const s: S;
let $tt_v0: number;
{
  const $tt_m = s;
  switch ($tt_m.kind) {
    case "A": {
      const { n } = $tt_m;
      $tt_v0 = "a" + n;
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
// @ts-ignore -- governs the documentation line below, not the declaration
/** The documented value. */
export const ungoverned: number = $tt_v0;
export function ignored(r: (n: number) => TResult<number, string>): TResult<number, string> {
  const $tt_t0 = r("x");
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  // @ts-ignore -- governs the documentation line below, not the declaration
  /** The documented binding. */
  const v = $tt_t0.value;
  return Result.Ok(v);
}
export function expected(r: (n: number) => TResult<number, string>): TResult<number, string> {
    const $tt_t1 = r("y");
    if (!("value" in $tt_t1)) {
      return $tt_t1;
    }
    // @ts-expect-error -- governs the indented documentation line, so it is unused
    /** The indented documented binding. */
  const w = $tt_t1.value;
  return Result.Ok(w);
}
/** A directive below the JSDoc governs the statement's line. */
// @ts-expect-error -- a string where a number is declared
let $tt_v1: number; { const $tt_m = s; switch ($tt_m.kind) { case "A": { const { n } = $tt_m; $tt_v1 = "a" + n; break; } case "B": { $tt_v1 = "b"; break; } default: { throw new Error("tt match: unexpected case " + $tt_show($tt_m)); } } } export const governed: number = $tt_v1;
