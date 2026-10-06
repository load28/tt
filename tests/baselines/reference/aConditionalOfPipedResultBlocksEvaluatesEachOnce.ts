//// [aConditionalOfPipedResultBlocksEvaluatesEachOnce.tt] ////
import type { TResult } from "@tt/std";
const L: string[] = [];
function r(n: number): TResult<number, string> { L.push("r" + n); return { kind: "Ok", value: n }; }
const f = (x: TResult<number, string>) => x.kind;
export function F() {
  return match ((result { return try r(5); } |> f) ? (result { return try r(6); } |> f) : "none") {
    "Ok" => 1,
    _ => 0,
  };
}
console.log(F(), L.join());

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aConditionalOfPipedResultBlocksEvaluatesEachOnce.ts]
function $tt_expr<T>(run: () => T): T { return run(); }
import type { TResult } from "./tt/index.js";
const L: string[] = [];
function r(n: number): TResult<number, string> { L.push("r" + n); return { kind: "Ok", value: n }; }
const f = (x: TResult<number, string>) => x.kind;
export function F() {
  let $tt_v0: number;
  {
    let $tt_m; let $tt_v4: "Err" | "Ok";
    do {
      const $tt_v10: TResult<number, string> = ($tt_expr(() => {
        const $tt_t0 = r(5);
        if (!("value" in $tt_t0)) {
          return $tt_t0;
        }
        return { kind: "Ok" as const, value: $tt_t0.value };
        }));
      $tt_v4 = f($tt_v10);
      break;
    } while (false);
    let $tt_v3: string;
    if ($tt_v4) {
      do {
        let $tt_v1: "Err" | "Ok";
        const $tt_v9: TResult<number, string> = ($tt_expr(() => {
          const $tt_t1 = r(6);
          if (!("value" in $tt_t1)) {
            return $tt_t1;
          }
          return { kind: "Ok" as const, value: $tt_t1.value };
          }));
        $tt_v1 = f($tt_v9);
        $tt_v3 = $tt_v1; break;
      } while (false);
    } else {
      $tt_v3 = "none";
    }
    $tt_m = $tt_v3;
    switch ($tt_m) {
      case "Ok": {
        $tt_v0 = 1;
        break;
      }
      default: {
        $tt_v0 = 0;
        break;
      }
    }
  }
  return $tt_v0;
}
console.log(F(), L.join());
