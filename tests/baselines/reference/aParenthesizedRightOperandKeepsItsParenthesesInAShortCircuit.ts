//// [aParenthesizedRightOperandKeepsItsParenthesesInAShortCircuit.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
const r = (n: number): TResult<number, string> => Result.Ok(n);
const g = (x: unknown) => x;
function f(o: { x: number; y: number | null }): TResult<unknown, string> {
  const value = g((o.y ?? (o.x += try r(5))));
  return Result.Ok([value, o.x]);
}
console.log(JSON.stringify(f({ x: 1, y: null })), JSON.stringify(f({ x: 1, y: 7 })));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aParenthesizedRightOperandKeepsItsParenthesesInAShortCircuit.ts]
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
const r = (n: number): TResult<number, string> => Result.Ok(n);
const g = (x: unknown) => x;
function f(o: { x: number; y: number | null }): TResult<unknown, string> {
  let $tt_v4: number;
  const $tt_v3: typeof g = (g);
  let $tt_v2: number | null;
  if (($tt_v2 = o.y) == null) {
    let $tt_v0: number;
    let $tt_v1 = (o.x);
    const $tt_t0 = r(5);
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v0 = $tt_t0.value;
    $tt_v4 = $tt_v2 ?? (o.x = $tt_v1 += $tt_v0);
  } else {
    $tt_v4 = $tt_v2;
  }
  
  const value = $tt_v3(($tt_v4));
  return Result.Ok([value, o.x]);
}
console.log(JSON.stringify(f({ x: 1, y: null })), JSON.stringify(f({ x: 1, y: 7 })));
