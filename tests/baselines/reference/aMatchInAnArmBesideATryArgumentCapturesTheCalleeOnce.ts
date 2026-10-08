//// [aMatchInAnArmBesideATryArgumentCapturesTheCalleeOnce.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
variant S { A, B }
const s: S = S.A as S;
function R(x: number): TResult<number, string> { return Result.Ok(x); }
function f(...a: unknown[]) { return a.join(","); }
function declared(): TResult<unknown, string> {
  const r = match (s) { A => f(match (s) { A => 1, B => 2 }, try R(1)), B => 2 };
  return Result.Ok(r);
}
function returned(): TResult<unknown, string> {
  return Result.Ok(match (s) { A => f(match (s) { A => 1, B => 2 }, try R(1)), B => 2 });
}
function tested(): TResult<unknown, string> {
  if (match (s) { A => f(match (s) { A => 1, B => 2 }, try R(1)), B => 2 } === "1,1") { return Result.Ok("yes"); }
  return Result.Ok("no");
}
console.log(JSON.stringify([declared(), returned(), tested()]));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aMatchInAnArmBesideATryArgumentCapturesTheCalleeOnce.ts]
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
type S =
  | { kind: "A" }
  | { kind: "B" };
const S = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const s: S = S.A as S;
function R(x: number): TResult<number, string> { return Result.Ok(x); }
function f(...a: unknown[]) { return a.join(","); }
function declared(): TResult<unknown, string> {
  let $tt_v0: (string) | (number);
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        let $tt_v1: number;
        const $tt_v2: typeof f = (f);
        {
          const $tt_m = s;
          switch ($tt_m.kind) {
            case "A": {
              $tt_v1 = 1;
              break;
            }
            case "B": {
              $tt_v1 = 2;
              break;
            }
            default: {
              throw new Error("tt match: unexpected case " + $tt_show($tt_m));
            }
          }
        }
        let $tt_v10: number;
        const $tt_t0 = R(1);
        if (!("value" in $tt_t0)) {
          return $tt_t0;
        }
        $tt_v10 = $tt_t0.value;
        $tt_v0 = $tt_v2($tt_v1, $tt_v10);
        break;
      }
      case "B": {
        $tt_v0 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const r = $tt_v0;
  return Result.Ok(r);
}
function returned(): TResult<unknown, string> {
  let $tt_v3: string | number;
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        let $tt_v4: number;
        const $tt_v6: typeof f = (f);
        {
          const $tt_m = s;
          switch ($tt_m.kind) {
            case "A": {
              $tt_v4 = 1;
              break;
            }
            case "B": {
              $tt_v4 = 2;
              break;
            }
            default: {
              throw new Error("tt match: unexpected case " + $tt_show($tt_m));
            }
          }
        }
        let $tt_v11: number;
        const $tt_t1 = R(1);
        if (!("value" in $tt_t1)) {
          return $tt_t1;
        }
        $tt_v11 = $tt_t1.value;
        $tt_v3 = $tt_v6($tt_v4, $tt_v11);
        break;
      }
      case "B": {
        $tt_v3 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return Result.Ok($tt_v3);
}
function tested(): TResult<unknown, string> {
  let $tt_v7: (string) | (number);
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        let $tt_v8: number;
        const $tt_v9: typeof f = (f);
        {
          const $tt_m = s;
          switch ($tt_m.kind) {
            case "A": {
              $tt_v8 = 1;
              break;
            }
            case "B": {
              $tt_v8 = 2;
              break;
            }
            default: {
              throw new Error("tt match: unexpected case " + $tt_show($tt_m));
            }
          }
        }
        let $tt_v12: number;
        const $tt_t2 = R(1);
        if (!("value" in $tt_t2)) {
          return $tt_t2;
        }
        $tt_v12 = $tt_t2.value;
        $tt_v7 = $tt_v9($tt_v8, $tt_v12);
        break;
      }
      case "B": {
        $tt_v7 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  if ($tt_v7 === "1,1") { return Result.Ok("yes"); }
  return Result.Ok("no");
}
console.log(JSON.stringify([declared(), returned(), tested()]));
