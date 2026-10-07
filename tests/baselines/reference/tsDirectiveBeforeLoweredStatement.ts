//// [tsDirectiveBeforeLoweredStatement.tt] ////
// A `// @ts-expect-error` or `// @ts-ignore` comment governs the next source
// line. When that line holds a tt construct whose lowering writes generated
// statements, the lowering of the line stays on one output line, so the
// directive still reaches every error the line reports: no TS2578 "Unused
// '@ts-expect-error' directive", and no error from the line it governs. A
// JSDoc comment stays on the declaration it documents, after the generated
// statements, so declaration emit and hover keep it.
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
variant S { A(n: number), B }
function matchValue(s: S) {
  // @ts-expect-error -- a string where a number is declared
  const x: number = match (s) { A(n) => "a" + n, B => "b" };
  return x;
}
function matchOperand(s: S) {
  // @ts-ignore
  const y: string = match (s) { A(n) => n, B => 0 } + 1;
  return y;
}
function tryValue(r: TResult<string, string>): TResult<number, string> {
  // @ts-expect-error -- a string where a number is declared
  const z: number = try r;
  return Result.Ok(z);
}
function tryOperand(r: (n: number) => TResult<string, string>): TResult<string, string> {
  // @ts-expect-error -- a string where a number is expected
  const z = try r("1");
  return Result.Ok(z);
}
const block = (r: TResult<string, string>) => {
  // @ts-expect-error -- a string where a number is declared
  const w: number = result { const v = try r; return v; }.kind === "Ok" ? "ok" : 0;
  return w;
};
function letElse(s: S) {
  // @ts-expect-error -- a number where a string is declared
  const A(n: m) = s else { return "none"; }; const t: string = m;
  return t;
}
function laterLine(s: S) {
  // @ts-expect-error -- the declaration, on the first line, is a string
  const u: number = match (s) { A(n) => n, B => 0 }
    + "";
  return u;
}
/** The documented value. */
export const documented = match (S.A(2) as S) { A(n) => n, B => 0 };
/**
 * The documented binding.
 */
// a note after the documentation
const A(n: documentedBinding) = S.A(3) as S else { throw new Error("B"); };
// a plain comment stays above the generated statements
export const plain = match (S.B as S) { A(n) => n, B => -1 };
console.log(documented, documentedBinding, plain, matchValue(S.A(1)), matchOperand(S.B), laterLine(S.A(4)));
console.log(JSON.stringify(tryValue(Result.Ok("s"))), JSON.stringify(tryOperand((n) => Result.Ok(`${n}`))), block(Result.Ok("t")), letElse(S.A(5)), letElse(S.B));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [tsDirectiveBeforeLoweredStatement.ts]
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
// A `// @ts-expect-error` or `// @ts-ignore` comment governs the next source
// line. When that line holds a tt construct whose lowering writes generated
// statements, the lowering of the line stays on one output line, so the
// directive still reaches every error the line reports: no TS2578 "Unused
// '@ts-expect-error' directive", and no error from the line it governs. A
// JSDoc comment stays on the declaration it documents, after the generated
// statements, so declaration emit and hover keep it.
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
type S =
  | { kind: "A"; n: number }
  | { kind: "B" };
const S = {
  A: (n: number): S => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
function matchValue(s: S) {
  // @ts-expect-error -- a string where a number is declared
  let $tt_v0: number; { const $tt_m = s; switch ($tt_m.kind) { case "A": { const { n } = $tt_m; $tt_v0 = "a" + n; break; } case "B": { $tt_v0 = "b"; break; } default: { throw new Error("tt match: unexpected case " + $tt_show($tt_m)); } } } const x: number = $tt_v0;
  return x;
}
function matchOperand(s: S) {
  // @ts-ignore
  let $tt_v1: number; { const $tt_m = s; switch ($tt_m.kind) { case "A": { const { n } = $tt_m; $tt_v1 = n; break; } case "B": { $tt_v1 = 0; break; } default: { throw new Error("tt match: unexpected case " + $tt_show($tt_m)); } } } const y: string = $tt_v1 + 1;
  return y;
}
function tryValue(r: TResult<string, string>): TResult<number, string> {
  // @ts-expect-error -- a string where a number is declared
  const $tt_t0 = r; if (!("value" in $tt_t0)) { return $tt_t0; } const z: number = $tt_t0.value;
  return Result.Ok(z);
}
function tryOperand(r: (n: number) => TResult<string, string>): TResult<string, string> {
  // @ts-expect-error -- a string where a number is expected
  const $tt_t1 = r("1"); if (!("value" in $tt_t1)) { return $tt_t1; } const z = $tt_t1.value;
  return Result.Ok(z);
}
const block = (r: TResult<string, string>) => {
  // @ts-expect-error -- a string where a number is declared
  let $tt_v2: (Result.TErr<string>) | ({ kind: "Ok"; value: string; }); $tt_v2: { const $tt_t2 = r; if (!("value" in $tt_t2)) { $tt_v2 = $tt_t2; break $tt_v2; } const v = $tt_t2.value; { $tt_v2 = { kind: "Ok" as const, value: v }; break $tt_v2; } } const w: number = $tt_v2.kind === "Ok" ? "ok" : 0;
  return w;
};
function letElse(s: S) {
  // @ts-expect-error -- a number where a string is declared
  const $tt_t3 = s; if ($tt_t3.kind !== "A") { return "none"; } const { n: m } = $tt_t3; const t: string = m;
  return t;
}
function laterLine(s: S) {
  // @ts-expect-error -- the declaration, on the first line, is a string
  let $tt_v3: number; { const $tt_m = s; switch ($tt_m.kind) { case "A": { const { n } = $tt_m; $tt_v3 = n; break; } case "B": { $tt_v3 = 0; break; } default: { throw new Error("tt match: unexpected case " + $tt_show($tt_m)); } } } const u: number = $tt_v3
    + "";
  return u;
}
let $tt_v4: number;
{
  const $tt_m = S.A(2) as S;
  switch ($tt_m.kind) {
    case "A": {
      const { n } = $tt_m;
      $tt_v4 = n;
      break;
    }
    case "B": {
      $tt_v4 = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
/** The documented value. */
export const documented = $tt_v4;
const $tt_t4 = S.A(3) as S;
if ($tt_t4.kind !== "A") {
  throw new Error("B");
}
/**
 * The documented binding.
 */
// a note after the documentation
const { n: documentedBinding } = $tt_t4;
// a plain comment stays above the generated statements
let $tt_v5: number;
{
  const $tt_m = S.B as S;
  switch ($tt_m.kind) {
    case "A": {
      const { n } = $tt_m;
      $tt_v5 = n;
      break;
    }
    case "B": {
      $tt_v5 = -1;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
export const plain = $tt_v5;
console.log(documented, documentedBinding, plain, matchValue(S.A(1)), matchOperand(S.B), laterLine(S.A(4)));
console.log(JSON.stringify(tryValue(Result.Ok("s"))), JSON.stringify(tryOperand((n) => Result.Ok(`${n}`))), block(Result.Ok("t")), letElse(S.A(5)), letElse(S.B));
