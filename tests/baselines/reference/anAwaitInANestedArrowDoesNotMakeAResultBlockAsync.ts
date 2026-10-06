//// [anAwaitInANestedArrowDoesNotMakeAResultBlockAsync.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
function r(f: () => Promise<number>): TResult<() => Promise<number>, string> { return Result.Ok(f); }
const o = { z: 5 };
const F = (k = result { const z = try r(async () => await Promise.resolve(o.z)); return z; }) => k;
const G = (k = result { const z = try r(async function () { return await Promise.resolve(o.z); }); return z; }) => k;
const H = (k = result { const z = try r(async () => { const w = await Promise.resolve(o.z); return w; }); return z; }) => k;
async function main() {
  for (const make of [F, G, H]) {
    const made = make();
    console.log(made.kind, made.kind === "Ok" ? await made.value() : made.error);
  }
}
main();

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [anAwaitInANestedArrowDoesNotMakeAResultBlockAsync.ts]
function $tt_expr<T>(run: () => T): T { return run(); }
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
function r(f: () => Promise<number>): TResult<() => Promise<number>, string> { return Result.Ok(f); }
const o = { z: 5 };
const F = (k = $tt_expr(() => {
  const $tt_t0 = r(async () => await Promise.resolve(o.z));
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const z = $tt_t0.value; { return { kind: "Ok" as const, value: z }; }
  })) => k;
const G = (k = $tt_expr(() => {
  const $tt_t1 = r(async function () { return await Promise.resolve(o.z); });
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  const z = $tt_t1.value; { return { kind: "Ok" as const, value: z }; }
  })) => k;
const H = (k = $tt_expr(() => {
  const $tt_t2 = r(async () => { const w = await Promise.resolve(o.z); return w; });
  if (!("value" in $tt_t2)) {
    return $tt_t2;
  }
  const z = $tt_t2.value; { return { kind: "Ok" as const, value: z }; }
  })) => k;
async function main() {
  for (const make of [F, G, H]) {
    const made = make();
    console.log(made.kind, made.kind === "Ok" ? await made.value() : made.error);
  }
}
main();
