//// [logicalOperationRightNeverRuns.tt] ////
// A logical operation whose right operand can never run has its left
// operand's type, as TypeScript gives it (`checkBinaryLikeExpressionWorker`:
// `||` when the left operand is never falsy, `&&` when it is never truthy,
// `??` when it is never nullish). The branch that runs the right operand is
// written as the operation over the stored left operand, so its value is
// typed `never` when the test narrows that operand to `never`.
// `n && try text()` stays `number | string` where TypeScript gives
// `0 | string`: it takes `number`'s falsy part as `0`
// (`extractDefinitelyFalsyTypes`), which no narrowing of a stored `number`
// produces (`NaN` is falsy too).
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
declare function read(): TResult<number, string>;
declare function text(): TResult<string, string>;
declare const o: { a: number };
declare const n: number;
declare const s: string | null;
export function operations(): TResult<unknown, string> {
  const orTrue = true || try read();
  const orObject = o || try read();
  const nullishObject = o ?? try read();
  const andNull = null && try read();
  const andFalse = false && try read();
  const andNumber = n && try text();
  const orString = s || try read();
  const nullishString = s ?? try read();
  return Result.Ok([orTrue, orObject, nullishObject, andNull, andFalse, andNumber, orString, nullishString]);
}

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [logicalOperationRightNeverRuns.ts]
// A logical operation whose right operand can never run has its left
// operand's type, as TypeScript gives it (`checkBinaryLikeExpressionWorker`:
// `||` when the left operand is never falsy, `&&` when it is never truthy,
// `??` when it is never nullish). The branch that runs the right operand is
// written as the operation over the stored left operand, so its value is
// typed `never` when the test narrows that operand to `never`.
// `n && try text()` stays `number | string` where TypeScript gives
// `0 | string`: it takes `number`'s falsy part as `0`
// (`extractDefinitelyFalsyTypes`), which no narrowing of a stored `number`
// produces (`NaN` is falsy too).
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
declare function read(): TResult<number, string>;
declare function text(): TResult<string, string>;
declare const o: { a: number };
declare const n: number;
declare const s: string | null;
export function operations(): TResult<unknown, string> {
  let $tt_v1: true;
  let $tt_v2: true;
  if ($tt_v2 = true) {
    $tt_v1 = $tt_v2;
  } else {
    let $tt_v0: number;
    const $tt_t0 = read();
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v0 = $tt_t0.value;
    $tt_v1 = $tt_v2 || $tt_v0;
  }
  
  const orTrue = $tt_v1;
  let $tt_v5: {
    a: number;
};
  let $tt_v4: {
    a: number;
};
  if ($tt_v4 = o) {
    $tt_v5 = $tt_v4;
  } else {
    let $tt_v3: number;
    const $tt_t1 = read();
    if (!("value" in $tt_t1)) {
      return $tt_t1;
    }
    $tt_v3 = $tt_t1.value;
    $tt_v5 = $tt_v4 || $tt_v3;
  }
  
  const orObject = $tt_v5;
  let $tt_v8: {
    a: number;
};
  let $tt_v7: {
    a: number;
};
  if (($tt_v7 = o) == null) {
    let $tt_v6: number;
    const $tt_t2 = read();
    if (!("value" in $tt_t2)) {
      return $tt_t2;
    }
    $tt_v6 = $tt_t2.value;
    $tt_v8 = $tt_v7 ?? $tt_v6;
  } else {
    $tt_v8 = $tt_v7;
  }
  
  const nullishObject = $tt_v8;
  let $tt_v10: null;
  let $tt_v11: null;
  if ($tt_v11 = null) {
    let $tt_v9: number;
    const $tt_t3 = read();
    if (!("value" in $tt_t3)) {
      return $tt_t3;
    }
    $tt_v9 = $tt_t3.value;
    $tt_v10 = $tt_v11 && $tt_v9;
  } else {
    $tt_v10 = $tt_v11;
  }
  
  const andNull = $tt_v10;
  let $tt_v13: false;
  let $tt_v14: false;
  if ($tt_v14 = false) {
    let $tt_v12: number;
    const $tt_t4 = read();
    if (!("value" in $tt_t4)) {
      return $tt_t4;
    }
    $tt_v12 = $tt_t4.value;
    $tt_v13 = $tt_v14 && $tt_v12;
  } else {
    $tt_v13 = $tt_v14;
  }
  
  const andFalse = $tt_v13;
  let $tt_v17: (string | 0) | (number);
  let $tt_v16: number;
  if ($tt_v16 = n) {
    let $tt_v15: string;
    const $tt_t5 = text();
    if (!("value" in $tt_t5)) {
      return $tt_t5;
    }
    $tt_v15 = $tt_t5.value;
    $tt_v17 = $tt_v16 && $tt_v15;
  } else {
    $tt_v17 = $tt_v16;
  }
  
  const andNumber = $tt_v17;
  let $tt_v20: string | number;
  let $tt_v19: string | null;
  if ($tt_v19 = s) {
    $tt_v20 = $tt_v19;
  } else {
    let $tt_v18: number;
    const $tt_t6 = read();
    if (!("value" in $tt_t6)) {
      return $tt_t6;
    }
    $tt_v18 = $tt_t6.value;
    $tt_v20 = $tt_v19 || $tt_v18;
  }
  
  const orString = $tt_v20;
  let $tt_v23: (number) | (string);
  let $tt_v22: string | null;
  if (($tt_v22 = s) == null) {
    let $tt_v21: number;
    const $tt_t7 = read();
    if (!("value" in $tt_t7)) {
      return $tt_t7;
    }
    $tt_v21 = $tt_t7.value;
    $tt_v23 = $tt_v22 ?? $tt_v21;
  } else {
    $tt_v23 = $tt_v22;
  }
  
  const nullishString = $tt_v23;
  return Result.Ok([orTrue, orObject, nullishObject, andNull, andFalse, andNumber, orString, nullishString]);
}
