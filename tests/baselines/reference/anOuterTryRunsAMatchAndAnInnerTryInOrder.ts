//// [anOuterTryRunsAMatchAndAnInnerTryInOrder.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
const seen: string[] = [];
const half = (n: number): TResult<number, string> => {
  seen.push(`half(${n})`);
  return n % 2 === 0 ? Result.Ok(n / 2) : Result.Err(`odd ${n}`);
};
const note = (n: number) => { seen.push(`k=${n}`); return n; };
function f(k: number): TResult<number, string> {
  const x = try half(match (note(k)) { 2 => 10, _ => 20 } + (try half(k)));
  return Result.Ok(x);
}
console.log(JSON.stringify([f(2), f(4), f(3)]), seen.join(" "));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [anOuterTryRunsAMatchAndAnInnerTryInOrder.ts]
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
const seen: string[] = [];
const half = (n: number): TResult<number, string> => {
  seen.push(`half(${n})`);
  return n % 2 === 0 ? Result.Ok(n / 2) : Result.Err(`odd ${n}`);
};
const note = (n: number) => { seen.push(`k=${n}`); return n; };
function f(k: number): TResult<number, string> {
  let $tt_v0: number;
  let $tt_v1: number;
  const $tt_v2 = (half);
  {
    const $tt_m = note(k);
    switch ($tt_m) {
      case 2: {
        $tt_v0 = 10;
        break;
      }
      default: {
        $tt_v0 = 20;
        break;
      }
    }
  }
  const $tt_t1 = half(k);
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  $tt_v1 = $tt_t1.value;
  const $tt_t0 = $tt_v2($tt_v0 + ($tt_v1));
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const x = $tt_t0.value;
  return Result.Ok(x);
}
console.log(JSON.stringify([f(2), f(4), f(3)]), seen.join(" "));
