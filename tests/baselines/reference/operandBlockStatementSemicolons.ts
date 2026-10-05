//// [operandBlockStatementSemicolons.tt] ////
// A `;` inside a block inside a tt operand's list belongs to that block's
// statements; only a `;` directly in an argument or array list ends it.
import type { TResult } from "@tt/std";
declare function h(f: () => void): TResult<number, string>;
declare function each(fs: (() => void)[]): TResult<number, string>;
function g(): TResult<number, string> {
  const n = try h(() => { for (;;) { break; } });
  const m = try each([() => { for (let i = 0; i < 2; i++) {} }]);
  try h(function () { for (;;) { break; } });
  return { kind: "Ok", value: n + m };
}
export const r = 1 |> ((v: number) => { for (;;) { break; } return v; });
export { g };
declare const xs: number[];
export const s = xs |> ((ys: number[]) => ys.map((y) => { for (let i = 0; i < y; i++) {} return y; }));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts
//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [operandBlockStatementSemicolons.ts]
import { $tt_ap } from "./tt/runtime.js";
// A `;` inside a block inside a tt operand's list belongs to that block's
// statements; only a `;` directly in an argument or array list ends it.
import type { TResult } from "./tt/index.js";
declare function h(f: () => void): TResult<number, string>;
declare function each(fs: (() => void)[]): TResult<number, string>;
function g(): TResult<number, string> {
  const $tt_t0 = h(() => { for (;;) { break; } });
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const n = $tt_t0.value;
  const $tt_t1 = each([() => { for (let i = 0; i < 2; i++) {} }]);
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  const m = $tt_t1.value;
  const $tt_t2 = h(function () { for (;;) { break; } });
  if (!("value" in $tt_t2)) {
    return $tt_t2;
  }
  return { kind: "Ok", value: n + m };
}
export const r = ((v: number) => { for (;;) { break; } return v; })(1);
export { g };
declare const xs: number[];
export const s = $tt_ap(xs, ((ys: number[]) => ys.map((y) => { for (let i = 0; i < y; i++) {} return y; })));
