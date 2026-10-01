//// [aValueInALaterDeclaratorSplitsTheDeclarationBeforeItsPrelude3.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
variant O { A(n: number), B }
declare const o: O;
declare function t(s: string): number;
declare function r(n: number): TResult<number, string>;
export function f(): TResult<number, string> {
  let a = t("a"), b = 1 + try r(a);
  return Result.Ok(0);
}

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aValueInALaterDeclaratorSplitsTheDeclarationBeforeItsPrelude3.ts]
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare const o: O;
declare function t(s: string): number;
declare function r(n: number): TResult<number, string>;
export function f(): TResult<number, string> {
  let a = t("a");
  let $tt_v0: number;
  const $tt_t0 = r(a);
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  let b = 1 + $tt_v0;
  return Result.Ok(0);
}
