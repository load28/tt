//// [aTryOperandMayStartWithNewFunctionOrClass.tt] ////
import type { TResult } from "@tt/std";
declare const rr: TResult<number, string>;
declare class Parser { constructor(s: string); parse(): TResult<number, string>; }
declare class Box { r: TResult<number, string>; }
export function f(s: string): TResult<number, string> {
  const n = try new Parser(s).parse();
  const a = try function () { return rr; }();
  const b = try class { static r = rr; }.r;
  const c = try new Box().r;
  try new Box().r;
  const d = try (new Box()).r;
  return { kind: "Ok", value: n + a + b + c + d };
}

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aTryOperandMayStartWithNewFunctionOrClass.ts]
import type { TResult } from "./tt/index.js";
declare const rr: TResult<number, string>;
declare class Parser { constructor(s: string); parse(): TResult<number, string>; }
declare class Box { r: TResult<number, string>; }
export function f(s: string): TResult<number, string> {
  const $tt_t0 = new Parser(s).parse();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const n = $tt_t0.value;
  let $tt_v0: number;
  const $tt_t1 = function () { return rr; }();
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  $tt_v0 = $tt_t1.value;
  const a = $tt_v0;
  let $tt_v1: number;
  const $tt_t2 = class { static r = rr; }.r;
  if (!("value" in $tt_t2)) {
    return $tt_t2;
  }
  $tt_v1 = $tt_t2.value;
  const b = $tt_v1;
  const $tt_t3 = new Box().r;
  if (!("value" in $tt_t3)) {
    return $tt_t3;
  }
  const c = $tt_t3.value;
  const $tt_t4 = new Box().r;
  if (!("value" in $tt_t4)) {
    return $tt_t4;
  }
  const $tt_t5 = (new Box()).r;
  if (!("value" in $tt_t5)) {
    return $tt_t5;
  }
  const d = $tt_t5.value;
  return { kind: "Ok", value: n + a + b + c + d };
}
