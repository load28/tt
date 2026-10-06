//// [aShorthandPropertyBesideATtValueKeepsItsKey.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
variant V { A, B }
function get(): V { return V.A; }
function read(): TResult<number, string> { return Result.Ok(2); }
export function matched(a: number) {
  return { a, b: match (get()) { A => 1, B => 0 } };
}
export function tried(a: number): TResult<{ a: number; b: number }, string> {
  return Result.Ok({ a, b: try read() });
}
export function nested(a: number) {
  const x = { a, b: result { const q = try read(); return q; } };
  const y = { a, b: 1 |> String };
  return [x, y];
}
const r: { a: number; b: number } = matched(5);
console.log(JSON.stringify(r));
console.log(JSON.stringify(tried(5)));
console.log(JSON.stringify(nested(5)));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aShorthandPropertyBesideATtValueKeepsItsKey.ts]
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
type V =
  | { kind: "A" }
  | { kind: "B" };
const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
function get(): V { return V.A; }
function read(): TResult<number, string> { return Result.Ok(2); }
export function matched(a: number) {
  let $tt_v0: number;
  const $tt_v1 = (a);
  {
    const $tt_m = get();
    switch ($tt_m.kind) {
      case "A": $tt_v0 = 0; break;
      case "B": $tt_v0 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return { a: $tt_v1, b: ($tt_v0 === 0 ? 1 : 0) };
}
export function tried(a: number): TResult<{ a: number; b: number }, string> {
  let $tt_v2: number;
  const $tt_v3: number = (a);
  const $tt_t0 = read();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v2 = $tt_t0.value;
  return Result.Ok({ a: $tt_v3, b: $tt_v2 });
}
export function nested(a: number) {
  let $tt_v5: (Result.TErr<string>) | ({
    kind: "Ok";
    value: number;
});
  const $tt_v6 = (a);
  $tt_v5: {
    const $tt_t1 = read();
    if (!("value" in $tt_t1)) {
      $tt_v5 = $tt_t1;
      break $tt_v5;
    }
    const q = $tt_t1.value; { const $tt_a0 = { value: { kind: "Ok" as const, value: q } }; $tt_v5 = $tt_a0.value; break $tt_v5; }
  }
  const x = { a: $tt_v6, b: $tt_v5 };
  const y = { a, b: String(1) };
  return [x, y];
}
const r: { a: number; b: number } = matched(5);
console.log(JSON.stringify(r));
console.log(JSON.stringify(tried(5)));
console.log(JSON.stringify(nested(5)));
