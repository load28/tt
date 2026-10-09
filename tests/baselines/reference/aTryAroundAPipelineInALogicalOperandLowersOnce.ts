//// [aTryAroundAPipelineInALogicalOperandLowersOnce.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
function r(n: number): TResult<number, string> { return Result.Ok(n); }
const f = (x: TResult<number, string>) => x.kind.length;
export function F(c: boolean): TResult<number, string> {
  const x = match (c && (try r(result { return try r(2); } |> f))) { _ => 0 };
  return Result.Ok(x);
}
console.log(JSON.stringify(F(true)));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aTryAroundAPipelineInALogicalOperandLowersOnce.ts]
function $tt_expr<T>(run: () => T): T { return run(); }
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
function r(n: number): TResult<number, string> { return Result.Ok(n); }
const f = (x: TResult<number, string>) => x.kind.length;
export function F(c: boolean): TResult<number, string> {
  let $tt_v0: number;
  {
    let $tt_m; let $tt_v8: (number) | (false);
    let $tt_v3: boolean;
    if ($tt_v3 = c) {
      let $tt_v4: number;
      let $tt_v1: number;
      const $tt_v2: typeof r = (r);
      do {
        const $tt_v7: TResult<number, string> = ($tt_expr(() => {
          const $tt_t1 = r(2);
          if (!("value" in $tt_t1)) {
            return $tt_t1;
          }
          return { kind: "Ok" as const, value: $tt_t1.value };
          }));
        $tt_v1 = f($tt_v7);
        break;
      } while (false);
      const $tt_t0 = $tt_v2($tt_v1);
      if (!("value" in $tt_t0)) {
        return $tt_t0;
      }
      $tt_v4 = $tt_t0.value;
      $tt_v8 = $tt_v3 && $tt_v4;
    } else {
      $tt_v8 = $tt_v3;
    }
    $tt_m = $tt_v8;
    switch ($tt_m) {
      default: {
        $tt_v0 = 0;
        break;
      }
    }
  }
  const x = $tt_v0;
  return Result.Ok(x);
}
console.log(JSON.stringify(F(true)));
