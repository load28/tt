//// [tryAlwaysFailing.tt] ////
// Repro from TASK-627
import type { TErr, TResult } from "@tt/std";
import * as Result from "@tt/std/result";
declare function fail(): TErr<string>;
declare function either(): TErr<string> | TErr<number>;
declare function read(): TResult<number, string>;
export function direct(): TResult<number, string> {
  const x = try Result.Err("x");
  return Result.Ok(x + 1);
}
export function declared(): TResult<number, string> {
  const y = try fail();
  return Result.Ok(y);
}
export function joined(): TResult<number, string> {
  const w = (try read()) || (try fail());
  return Result.Ok(w + 1);
}
export function union(): TResult<string, string | number> {
  const u = try either();
  return Result.Ok(u);
}
export function block() {
  return result { const z = try fail(); return z; };
}

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [tryAlwaysFailing.ts]
// Repro from TASK-627
import type { TErr, TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";
declare function fail(): TErr<string>;
declare function either(): TErr<string> | TErr<number>;
declare function read(): TResult<number, string>;
export function direct(): TResult<number, string> {
  const $tt_t0 = Result.Err("x");
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const x = $tt_t0.value;
  return Result.Ok(x + 1);
}
export function declared(): TResult<number, string> {
  const $tt_t1 = fail();
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  const y = $tt_t1.value;
  return Result.Ok(y);
}
export function joined(): TResult<number, string> {
  let $tt_v0: number;
  let $tt_v2;
  const $tt_t2 = read();
  if (!("value" in $tt_t2)) {
    return $tt_t2;
  }
  $tt_v0 = $tt_t2.value;
  if ($tt_v0) {
    $tt_v2 = $tt_v0;
  } else {
    let $tt_v1;
    const $tt_t3 = fail();
    if (!("value" in $tt_t3)) {
      return $tt_t3;
    }
    $tt_v1 = $tt_t3.value;
    $tt_v2 = $tt_v0 || $tt_v1;
  }
  
  const w = $tt_v2;
  return Result.Ok(w + 1);
}
export function union(): TResult<string, string | number> {
  const $tt_t4 = either();
  if (!("value" in $tt_t4)) {
    return $tt_t4;
  }
  const u = $tt_t4.value;
  return Result.Ok(u);
}
export function block() {
  let $tt_v3: (TErr<string>) | ({
    kind: "Ok";
    value: unknown;
});
  $tt_v3: {
    const $tt_t5 = fail();
    if (!("value" in $tt_t5)) {
      $tt_v3 = $tt_t5;
      break $tt_v3;
    }
    const z = $tt_t5.value; { const $tt_a0 = { value: { kind: "Ok" as const, value: z } }; $tt_v3 = $tt_a0.value; break $tt_v3; }
  }
  return $tt_v3;
}
