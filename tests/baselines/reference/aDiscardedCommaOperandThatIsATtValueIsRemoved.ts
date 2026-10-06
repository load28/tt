//// [aDiscardedCommaOperandThatIsATtValueIsRemoved.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
variant V { A(n: number), B }
const seen: number[] = [];
const r = (n: number): TResult<number, string> => { seen.push(n); return n > 0 ? Result.Ok(n) : Result.Err(`bad ${n}`); };
function f(k: number, v: V): TResult<string, string> {
  const q = match ((try r(k), match (v) { A(n) => n, B => 0 }) as number) { 1 => "one", _ => "other" };
  return Result.Ok(q);
}
console.log(JSON.stringify([f(1, V.A(1)), f(2, V.B), f(0, V.A(1))]), seen.join(","));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aDiscardedCommaOperandThatIsATtValueIsRemoved.ts]
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
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const seen: number[] = [];
const r = (n: number): TResult<number, string> => { seen.push(n); return n > 0 ? Result.Ok(n) : Result.Err(`bad ${n}`); };
function f(k: number, v: V): TResult<string, string> {
  let $tt_v0: string;
  {
    let $tt_m_1; let $tt_v3: number;
    const $tt_t0 = r(k);
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v3 = $tt_t0.value;
    let $tt_v1: number;
    ($tt_v3);
    {
      const $tt_m = v;
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
    $tt_m_1 = ( $tt_v1) as number;
    switch ($tt_m_1) {
      case 1: {
        $tt_v0 = "one";
        break;
      }
      default: {
        $tt_v0 = "other";
        break;
      }
    }
  }
  const q = $tt_v0;
  return Result.Ok(q);
}
console.log(JSON.stringify([f(1, V.A(1)), f(2, V.B), f(0, V.A(1))]), seen.join(","));
