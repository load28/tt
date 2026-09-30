//// [tryPropagationOrder.tt] ////
// A try evaluates its operand once and, on Err, leaves its Result scope
// before anything written after it runs; sibling trys run left to right.
import type { TResult } from "@tt/std";
import * as Result from "@tt/std/result";
const log: string[] = [];
function step(label: string, ok: boolean): TResult<number, string> {
  log.push(label);
  return ok ? Result.Ok(label.length) : Result.Err(`${label} failed`);
}
function flush(title: string, value: unknown) {
  console.log(`${title}: ${JSON.stringify(value)} after ${log.join(" ")}`);
  log.length = 0;
}
function add(a: number, b: number, c: number) {
  log.push("add");
  return a + b + c;
}
function operands(first: boolean, second: boolean): TResult<number, string> {
  const total = (try step("a", first)) + (try step("bb", second));
  log.push("after");
  return Result.Ok(total);
}
flush("operands ok", operands(true, true));
flush("first fails", operands(false, true));
flush("second fails", operands(true, false));
function args(fail: boolean): TResult<number, string> {
  return Result.Ok(add(try step("x", true), try step("yy", !fail), try step("zzz", true)));
}
flush("arguments ok", args(false));
flush("arguments fail", args(true));
const block = (ok: boolean) => result {
  const a = try step("first", true);
  const b = try step("second", ok);
  log.push("tail");
  return a * b;
};
flush("block ok", block(true));
flush("block fail", block(false));
function statement(ok: boolean): TResult<string, string> {
  try step("check", ok);
  log.push("checked");
  return Result.Ok("done");
}
flush("statement ok", statement(true));
flush("statement fail", statement(false));
function inner(ok: boolean): TResult<number, string> {
  const outcome = result {
    const v = try step("inner", ok);
    return v;
  };
  log.push(`outcome ${outcome.kind}`);
  const w = try outcome;
  log.push("unwrapped");
  return Result.Ok(w);
}
flush("nested scope ok", inner(true));
flush("nested scope fail", inner(false));
function scaled(ok: boolean): TResult<number, string> {
  return Result.Ok(try step("scaled", ok) * 1.5);
}
flush("binds tightly", scaled(true));
function asserted(): TResult<number, string> {
  return Result.Ok(try step(("asserted" as const), true));
}
flush("as const in the operand", asserted());
export {};

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [tryPropagationOrder.ts]
// A try evaluates its operand once and, on Err, leaves its Result scope
// before anything written after it runs; sibling trys run left to right.
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";
const log: string[] = [];
function step(label: string, ok: boolean): TResult<number, string> {
  log.push(label);
  return ok ? Result.Ok(label.length) : Result.Err(`${label} failed`);
}
function flush(title: string, value: unknown) {
  console.log(`${title}: ${JSON.stringify(value)} after ${log.join(" ")}`);
  log.length = 0;
}
function add(a: number, b: number, c: number) {
  log.push("add");
  return a + b + c;
}
function operands(first: boolean, second: boolean): TResult<number, string> {
  let $tt_v0: number;
  let $tt_v1: number;
  const $tt_t0 = step("a", first);
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  const $tt_t1 = step("bb", second);
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  $tt_v1 = $tt_t1.value;
  const total = ($tt_v0) + ($tt_v1);
  log.push("after");
  return Result.Ok(total);
}
flush("operands ok", operands(true, true));
flush("first fails", operands(false, true));
flush("second fails", operands(true, false));
function args(fail: boolean): TResult<number, string> {
  let $tt_v2: number;
  let $tt_v3: number;
  let $tt_v4: number;
  const $tt_v5 = (add);
  const $tt_t2 = step("x", true);
  if (!("value" in $tt_t2)) {
    return $tt_t2;
  }
  $tt_v2 = $tt_t2.value;
  const $tt_t3 = step("yy", !fail);
  if (!("value" in $tt_t3)) {
    return $tt_t3;
  }
  $tt_v3 = $tt_t3.value;
  const $tt_t4 = step("zzz", true);
  if (!("value" in $tt_t4)) {
    return $tt_t4;
  }
  $tt_v4 = $tt_t4.value;
  return Result.Ok($tt_v5($tt_v2, $tt_v3, $tt_v4));
}
flush("arguments ok", args(false));
flush("arguments fail", args(true));
const block = (ok: boolean) => {
  let $tt_v7: (Result.TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v7: {
    const $tt_t5 = step("first", true);
    if (!("value" in $tt_t5)) {
      $tt_v7 = $tt_t5;
      break $tt_v7;
    }
    const a = $tt_t5.value;
  const $tt_t6 = step("second", ok);
  if (!("value" in $tt_t6)) {
    $tt_v7 = $tt_t6;
    break $tt_v7;
  }
  const b = $tt_t6.value;
  log.push("tail");
  {
    const $tt_a0 = { value: { kind: "Ok" as const, value: a * b } };
    $tt_v7 = $tt_a0.value;
    break $tt_v7;
  }
  }
  return $tt_v7;
};
flush("block ok", block(true));
flush("block fail", block(false));
function statement(ok: boolean): TResult<string, string> {
  const $tt_t7 = step("check", ok);
  if (!("value" in $tt_t7)) {
    return $tt_t7;
  }
  log.push("checked");
  return Result.Ok("done");
}
flush("statement ok", statement(true));
flush("statement fail", statement(false));
function inner(ok: boolean): TResult<number, string> {
  let $tt_v8: (Result.TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  $tt_v8: {
    const $tt_t8 = step("inner", ok);
    if (!("value" in $tt_t8)) {
      $tt_v8 = $tt_t8;
      break $tt_v8;
    }
    const v = $tt_t8.value;
    {
      const $tt_a1 = { value: { kind: "Ok" as const, value: v } };
      $tt_v8 = $tt_a1.value;
      break $tt_v8;
    }
  }
  const outcome = $tt_v8;
  log.push(`outcome ${outcome.kind}`);
  const $tt_t9 = outcome;
  if (!("value" in $tt_t9)) {
    return $tt_t9;
  }
  const w = $tt_t9.value;
  log.push("unwrapped");
  return Result.Ok(w);
}
flush("nested scope ok", inner(true));
flush("nested scope fail", inner(false));
function scaled(ok: boolean): TResult<number, string> {
  let $tt_v9: number;
  const $tt_t10 = step("scaled", ok);
  if (!("value" in $tt_t10)) {
    return $tt_t10;
  }
  $tt_v9 = $tt_t10.value;
  return Result.Ok($tt_v9 * 1.5);
}
flush("binds tightly", scaled(true));
function asserted(): TResult<number, string> {
  let $tt_v11: number;
  const $tt_t11 = step(("asserted" as const), true);
  if (!("value" in $tt_t11)) {
    return $tt_t11;
  }
  $tt_v11 = $tt_t11.value;
  return Result.Ok($tt_v11);
}
flush("as const in the operand", asserted());
export {};
