//// [commentsInsideARebuiltFormStayWhereTheyWereWritten.tt] ////
variant O { A(n: number), B }
declare const obj: { m(a: number, b: number): number } | undefined;
declare function g(a: number, b: number): number;
export function f(o: O, c: boolean) {
  const a = obj?.m(/*C1*/ 1, match (o) { A(n) => n, B => 0 });
  const b = new Array(/*C2*/ 1, match (o) { A(n) => n, B => 0 });
  const d = c ? /*C3*/ 1 : match (o) { A(n) => n, B => 0 };
  const e = c /*C4*/ && match (o) { A(n) => n, B => 0 };
  const h = [/*C5*/ match (o) { A(n) => n, B => 0 } /*C6*/];
  const i = o /*C7*/ |> /*C8*/ (v => v) /*C9*/;
  const j = g(1, /*C10*/ match (o) { A(n) => /*C11*/ n /*C12*/, B => 0 } /*C13*/);
  return [a, b, d, e, h, i, j];
}

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [commentsInsideARebuiltFormStayWhereTheyWereWritten.ts]
import { $tt_ap } from "./tt/runtime.js";
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
type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare const obj: { m(a: number, b: number): number } | undefined;
declare function g(a: number, b: number): number;
export function f(o: O, c: boolean) {
  let $tt_v2: (number) | (undefined);
  if (obj != null) {
    let $tt_v0: number;
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v0 = n;
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
    $tt_v2 = obj?.m(/*C1*/ 1, $tt_v0);
  } else {
    $tt_v2 = undefined;
  }
  
  const a = $tt_v2;
  let $tt_v3: number;
  const $tt_v4: typeof Array = (Array);
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v3 = n;
        break;
      }
      case "B": {
        $tt_v3 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const b = new $tt_v4(/*C2*/ 1, $tt_v3);
  let $tt_v7: number;
  if (c) {
    $tt_v7 = /*C3*/ 1;
  } else {
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v7 = n;
          break;
        }
        case "B": {
          $tt_v7 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  }
  
  const d = $tt_v7;
  let $tt_v10: (number) | (false);
  let $tt_v9: boolean;
  if ($tt_v9 = c) {
    let $tt_v8: number;
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v8 = n;
          break;
        }
        case "B": {
          $tt_v8 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v10 = $tt_v9 && /*C4*/ $tt_v8;
  } else {
    $tt_v10 = $tt_v9;
  }
  
  const e = $tt_v10;
  let $tt_v11: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v11 = n;
        break;
      }
      case "B": {
        $tt_v11 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const h = [/*C5*/ $tt_v11 /*C6*/];
  const i = $tt_ap(o, /*C7*/  /*C8*/ (v => v)) /*C9*/;
  let $tt_v13: number;
  const $tt_v14: typeof g = (g);
  const $tt_v15 = (1);
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        /*C11*/
        $tt_v13 = $tt_v14($tt_v15, /*C10*/ n /*C12*/ /*C13*/);
        break;
      }
      case "B": {
        $tt_v13 = $tt_v14($tt_v15, /*C10*/ 0 /*C13*/);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const j = $tt_v13;
  return [a, b, d, e, h, i, j];
}
