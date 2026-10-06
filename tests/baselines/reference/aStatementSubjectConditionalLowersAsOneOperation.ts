//// [aStatementSubjectConditionalLowersAsOneOperation.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
const L: unknown[] = [];
function r(n: number): TResult<number, string> { L.push(n); return Result.Ok(n); }
const wrap = (n: number) => ({ kind: "Some" as const, value: n });
const none = { kind: "None" as const };
const S = { kind: "Some" as const, value: 3 };
export function letElse(c: boolean): TResult<number, string> {
  const Some(value) = c ? wrap(try r(1)) : none else { return Result.Err("none"); };
  return Result.Ok(value);
}
export function ifLet(n: number) {
  if let Some(value) = match (log(n)) { _ => true } ? match (n) { _ => S } : S { return value; }
  return 0;
}
function log<T>(x: T): T { L.push(x); return x; }
console.log(JSON.stringify(letElse(true)), JSON.stringify(letElse(false)), ifLet(4), L.join());

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aStatementSubjectConditionalLowersAsOneOperation.ts]
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
const L: unknown[] = [];
function r(n: number): TResult<number, string> { L.push(n); return Result.Ok(n); }
const wrap = (n: number) => ({ kind: "Some" as const, value: n });
const none = { kind: "None" as const };
const S = { kind: "Some" as const, value: 3 };
export function letElse(c: boolean): TResult<number, string> {
  let $tt_t0; let $tt_v7: ({
    kind: "Some";
    value: number;
}) | ({
    kind: "None";
});
  if (c) {
    let $tt_v3: number;
    const $tt_v5: typeof wrap = (wrap);
    const $tt_t1 = r(1);
    if (!("value" in $tt_t1)) {
      return $tt_t1;
    }
    $tt_v3 = $tt_t1.value;
    $tt_v7 = $tt_v5($tt_v3);
  } else {
    $tt_v7 = none;
  }
  $tt_t0 = $tt_v7;
  if ($tt_t0.kind !== "Some") {
    return Result.Err("none");
  }
  const { value } = $tt_t0;
  return Result.Ok(value);
}
export function ifLet(n: number) {
  {
    let $tt_t2; let $tt_v4: boolean;
    {
      const $tt_m = log(n);
      switch ($tt_m) {
        default: {
          $tt_v4 = true;
          break;
        }
      }
    }
    let $tt_v2: {
    kind: "Some";
    value: number;
};
    if ($tt_v4) {
      {
        const $tt_m = n;
        switch ($tt_m) {
          default: {
            $tt_v2 = S;
            break;
          }
        }
      }
    } else {
      $tt_v2 = S;
    }
    $tt_t2 = $tt_v2;
    if ($tt_t2.kind === "Some") {
      const { value } = $tt_t2;
      return value;
    }
  }
  return 0;
}
function log<T>(x: T): T { L.push(x); return x; }
console.log(JSON.stringify(letElse(true)), JSON.stringify(letElse(false)), ifLet(4), L.join());
