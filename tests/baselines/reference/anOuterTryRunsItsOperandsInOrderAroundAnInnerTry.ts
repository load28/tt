//// [anOuterTryRunsItsOperandsInOrderAroundAnInnerTry.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
const seen: string[] = [];
function half(n: number): TResult<number, string> {
  seen.push(`half(${n})`);
  return n % 2 === 0 ? Result.Ok(n / 2) : Result.Err(`odd ${n}`);
}
const abs = (n: number) => { seen.push(`abs(${n})`); return Math.abs(n); };
function f(k: number): TResult<number, string> {
  const x = try half((k |> abs) + (try half(k)));
  return Result.Ok(x);
}
console.log(JSON.stringify(f(-4)), seen.join(" "));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [anOuterTryRunsItsOperandsInOrderAroundAnInnerTry.ts]
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
const seen: string[] = [];
function half(n: number): TResult<number, string> {
  seen.push(`half(${n})`);
  return n % 2 === 0 ? Result.Ok(n / 2) : Result.Err(`odd ${n}`);
}
const abs = (n: number) => { seen.push(`abs(${n})`); return Math.abs(n); };
function f(k: number): TResult<number, string> {
  let $tt_v2: number;
  const $tt_v4 = (half);
  const $tt_v3 = ((($tt_v, $tt_f) => $tt_f($tt_v))(k, abs));
  const $tt_t1 = half(k);
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  $tt_v2 = $tt_t1.value;
  const $tt_t0 = $tt_v4(($tt_v3) + ($tt_v2));
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const x = $tt_t0.value;
  return Result.Ok(x);
}
console.log(JSON.stringify(f(-4)), seen.join(" "));
