//// [aValueInALaterDeclaratorSplitsTheDeclarationBeforeItsPrelude1.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
variant O { A(n: number), B }
declare const o: O;
declare function t(s: string): number;
declare function r(n: number): TResult<number, string>;
export function f(): TResult<number, string> {
  const a = t("a"), b = match (o) { A(n) => a + n, B => 0 };
  return Result.Ok(0);
}

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aValueInALaterDeclaratorSplitsTheDeclarationBeforeItsPrelude1.ts]
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
type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare const o: O;
declare function t(s: string): number;
declare function r(n: number): TResult<number, string>;
export function f(): TResult<number, string> {
  const a = t("a");
  let $tt_v0: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v0 = a + n;
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
  const b = $tt_v0;
  return Result.Ok(0);
}
