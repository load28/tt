//// [aCalleeCaptureInsideALogicalOperandReadsTheCallee.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
function R(x: number): TResult<number, string> { return Result.Ok(x); }
function obj(x: unknown) { return x; }
function g(x: unknown) { return x; }
const n = { m: (x: number) => x * 10 };
function t(): TResult<unknown, string> {
  const r = obj(n?.m(try R(1))) || g(try R(2));
  return Result.Ok(r);
}
console.log(JSON.stringify(t()));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aCalleeCaptureInsideALogicalOperandReadsTheCallee.ts]
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
function R(x: number): TResult<number, string> { return Result.Ok(x); }
function obj(x: unknown) { return x; }
function g(x: unknown) { return x; }
const n = { m: (x: number) => x * 10 };
function t(): TResult<unknown, string> {
  let $tt_v6: (number) | (undefined);
  let $tt_v7;
  const $tt_v3: typeof obj = (obj);
  if (n != null) {
    let $tt_v0: number;
    const $tt_t0 = R(1);
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v0 = $tt_t0.value;
    $tt_v6 = n?.m($tt_v0);
  } else {
    $tt_v6 = undefined;
  }
  
  let $tt_v5;
  if ($tt_v5 = $tt_v3($tt_v6)) {
    $tt_v7 = $tt_v5;
  } else {
    let $tt_v1: number;
    const $tt_v4: typeof g = (g);
    const $tt_t1 = R(2);
    if (!("value" in $tt_t1)) {
      return $tt_t1;
    }
    $tt_v1 = $tt_t1.value;
    $tt_v7 = $tt_v5 || $tt_v4($tt_v1);
  }
  
  const r = $tt_v7;
  return Result.Ok(r);
}
console.log(JSON.stringify(t()));
