//// [aCapturedOperandWritesTheValuesInsideIt.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
const r = (n: number): TResult<number, string> => (n > 0 ? Result.Ok(n) : Result.Err(`bad ${n}`));
const apply = (f: (w: number) => TResult<number, string>, at: TResult<number, string>) =>
  JSON.stringify(at.kind === "Ok" ? f(at.value) : at);
function f(k: number) {
  return apply((flow |> ((w: number) => (result { const z = try r(w); return z * 2; }))), result { const z = try r(k); return z; });
}
console.log(f(3), f(0));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aCapturedOperandWritesTheValuesInsideIt.ts]
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
const r = (n: number): TResult<number, string> => (n > 0 ? Result.Ok(n) : Result.Err(`bad ${n}`));
const apply = (f: (w: number) => TResult<number, string>, at: TResult<number, string>) =>
  JSON.stringify(at.kind === "Ok" ? f(at.value) : at);
function f(k: number) {
  let $tt_v0: TResult<number, string>;
  const $tt_v1: typeof apply = (apply);
  const $tt_v2: (w: number) => TResult<number, string> = ((((w: number) => {
    let $tt_v3: TResult<number, string>;
    $tt_v3: {
      const $tt_t0 = r(w);
      if (!("value" in $tt_t0)) {
        $tt_v3 = $tt_t0;
        break $tt_v3;
      }
      const z = $tt_t0.value; { $tt_v3 = { kind: "Ok" as const, value: z * 2 }; break $tt_v3; }
    }
    return ($tt_v3);
  })));
  $tt_v0: {
    const $tt_t1 = r(k);
    if (!("value" in $tt_t1)) {
      $tt_v0 = $tt_t1;
      break $tt_v0;
    }
    const z = $tt_t1.value; { $tt_v0 = { kind: "Ok" as const, value: z }; break $tt_v0; }
  }
  return $tt_v1(($tt_v2), $tt_v0);
}
console.log(f(3), f(0));
