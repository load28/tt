//// [aPatternSubjectWithATryBesideNestedStatementsIsLowered.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
variant V { A(n: number), B }
const r = (n: number): TResult<number, string> => (n > 0 ? Result.Ok(n) : Result.Err(`bad ${n}`));
const w = (n: number): V => (n > 3 ? V.A(n) : V.B);
const g = (a: number, b: TResult<number, string>): V => w(a + (b.kind === "Ok" ? b.value : 0));
function viaIfLet(k: number, c: V): TResult<number, string> {
  if let A(n) = w(try r(k) + (() => { const A(n: m) = c else { return 0; }; return m; })()) { return Result.Ok(n); }
  return Result.Ok(-1);
}
function viaLetElse(k: number): TResult<number, string> {
  const A(n) = g(try r(k), result { const z = try r(k - 1); return z * 10; }) else { return Result.Ok(-1); };
  return Result.Ok(n);
}
console.log(JSON.stringify([viaIfLet(2, V.A(5)), viaIfLet(2, V.B), viaIfLet(0, V.B), viaLetElse(2), viaLetElse(1), viaLetElse(0)]));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aPatternSubjectWithATryBesideNestedStatementsIsLowered.ts]
function $tt_expr<T>(run: () => T): T { return run(); }
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
const w = (n: number): V => (n > 3 ? V.A(n) : V.B);
const g = (a: number, b: TResult<number, string>): V => w(a + (b.kind === "Ok" ? b.value : 0));
function viaIfLet(k: number, c: V): TResult<number, string> {
  {
    let $tt_t0; let $tt_v0: number;
    const $tt_v3 = (w);
    const $tt_t1 = r(k);
    if (!("value" in $tt_t1)) {
      return $tt_t1;
    }
    $tt_v0 = $tt_t1.value;
    $tt_t0 = ($tt_v3($tt_v0 + (() => { const $tt_t2 = c;
    if ($tt_t2.kind !== "A") {
      return 0;
    }
    const { n: m } = $tt_t2; return m; })()));
    if ($tt_t0.kind === "A") {
      const { n } = $tt_t0;
      return Result.Ok(n);
    }
  }
  return Result.Ok(-1);
}
function viaLetElse(k: number): TResult<number, string> {
  let $tt_t3; let $tt_v1: number;
  const $tt_v4 = (g);
  const $tt_t4 = r(k);
  if (!("value" in $tt_t4)) {
    return $tt_t4;
  }
  $tt_v1 = $tt_t4.value;
  $tt_t3 = ($tt_v4($tt_v1, $tt_expr(() => {
    const $tt_t5 = r(k - 1);
    if (!("value" in $tt_t5)) {
      return $tt_t5;
    }
    const z = $tt_t5.value; { return { kind: "Ok" as const, value: z * 10 }; }
    })));
  if ($tt_t3.kind !== "A") {
    return Result.Ok(-1);
  }
  const { n } = $tt_t3;
  return Result.Ok(n);
}
console.log(JSON.stringify([viaIfLet(2, V.A(5)), viaIfLet(2, V.B), viaIfLet(0, V.B), viaLetElse(2), viaLetElse(1), viaLetElse(0)]));
