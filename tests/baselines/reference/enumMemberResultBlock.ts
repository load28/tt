//// [enumMemberResultBlock.tt] ////
// Runtime companion of TASK-594: a result block in an enum member initializer
// runs inside the enum, where a member's name denotes the member.
import type { TResult } from "@tt/std";
import * as Result from "@tt/std/result";
const P = 100;
const order: string[] = [];
function check(label: string, n: number): TResult<number, string> {
  order.push(label);
  return n < 50 ? Result.Ok(n) : Result.Err(`${label} too large: ${n}`);
}
export enum F {
  P = 7,
  Q = Result.unwrapOr(result { const v = try check("Q", P); return v + 1; }, -1),
  R = Result.unwrapOr(result { const v = try check("R", P * 10); return v; }, -1),
}
console.log(`outer P ${P}`);
console.log(`F.P ${F.P}, F.Q ${F.Q}, F.R ${F.R}`);
console.log(`order ${order.join(" ")}`);

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [enumMemberResultBlock.ts]
function $tt_expr<T>(run: () => T): T { return run(); }
// Runtime companion of TASK-594: a result block in an enum member initializer
// runs inside the enum, where a member's name denotes the member.
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";
const P = 100;
const order: string[] = [];
function check(label: string, n: number): TResult<number, string> {
  order.push(label);
  return n < 50 ? Result.Ok(n) : Result.Err(`${label} too large: ${n}`);
}
export enum F {
  P = 7,
  Q = Result.unwrapOr($tt_expr(() => {
    const $tt_t0 = check("Q", P);
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    const v = $tt_t0.value; { return { kind: "Ok" as const, value: v + 1 }; }
    }), -1),
  R = Result.unwrapOr($tt_expr(() => {
    const $tt_t1 = check("R", P * 10);
    if (!("value" in $tt_t1)) {
      return $tt_t1;
    }
    const v = $tt_t1.value; { return { kind: "Ok" as const, value: v }; }
    }), -1),
}
console.log(`outer P ${P}`);
console.log(`F.P ${F.P}, F.Q ${F.Q}, F.R ${F.R}`);
console.log(`order ${order.join(" ")}`);
