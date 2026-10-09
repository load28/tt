//// [aTtValueAsADiscardedCommaOperandLeavesNothingBehind.tt] ////
import type { TResult } from "@tt/std";
variant O { A(n: number), B }
const log: string[] = [];
const r = (n: number): TResult<number, string> => { log.push("r" + n); return { kind: "Ok", value: n }; };
export function f(o: O) {
  match (o) { A(n) => log.push("a" + n), B => 0 }, log.push("second");
  const x = (match (o) { A(n) => n, B => 0 }, 5);
  return x;
}
export function g(): TResult<number, string> {
  const y = (try r(1), try r(2));
  return { kind: "Ok", value: y };
}
console.log(f(O.A(1)), JSON.stringify(g()), log.join());

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aTtValueAsADiscardedCommaOperandLeavesNothingBehind.ts]
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
import type { TResult } from "./tt/index.js";
type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const log: string[] = [];
const r = (n: number): TResult<number, string> => { log.push("r" + n); return { kind: "Ok", value: n }; };
export function f(o: O) {
  let $tt_v0: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v0 = log.push("a" + n);
        break;
      }
      case "B": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
   log.push("second");
  let $tt_v1: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v1 = n;
        break;
      }
      case "B": {
        $tt_v1 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const x = ( 5);
  return x;
}
export function g(): TResult<number, string> {
  let $tt_v2: number;
  let $tt_v3: number;
  const $tt_t0 = r(1);
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v2 = $tt_t0.value;
  const $tt_t1 = r(2);
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  $tt_v3 = $tt_t1.value;
  const y = ( $tt_v3);
  return { kind: "Ok", value: y };
}
console.log(f(O.A(1)), JSON.stringify(g()), log.join());
