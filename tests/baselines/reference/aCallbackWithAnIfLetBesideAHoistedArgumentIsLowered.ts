//// [aCallbackWithAnIfLetBesideAHoistedArgumentIsLowered.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
variant V { A(n: number), B }
function show(a: string, f: (v: V) => number, b: string) { return `${a} ${f(V.A(7))} ${b}`; }
function run(k: number) {
  return show(k |> String, (v) => { if let A(n) = v { return n; } return 0; }, match (k) { 1 => "one", _ => "other" });
}
function read(): TResult<number, string> { return Result.Ok(4); }
function apply(n: number, f: (v: V) => number): number { return n + f(V.A(10)); }
function arm(v: V): TResult<number, string> {
  const s = match (v) {
    A(n) => n,
    B => apply(try read(), (w) => { if let A(n) = w { return n; } return 0; }),
  };
  return Result.Ok(s);
}
const viaArrow = (k: number) => show("x", (v) => match (v) { A(n) => n + k, B => 0 }, match (k) { 1 => "one", _ => "other" });
console.log(run(1), run(2), JSON.stringify(arm(V.B)), JSON.stringify(arm(V.A(1))), viaArrow(1));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [aCallbackWithAnIfLetBesideAHoistedArgumentIsLowered.ts]
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
function show(a: string, f: (v: V) => number, b: string) { return `${a} ${f(V.A(7))} ${b}`; }
function run(k: number) {
  let $tt_v1: string;
  const $tt_v2: typeof show = (show);
  const $tt_v3: string = ((($tt_v, $tt_f) => $tt_f($tt_v))(k, String));
  const $tt_v4: (v: V) => number = ((v) => { {
    const $tt_t0 = v;
    if ($tt_t0.kind === "A") {
      const { n } = $tt_t0;
      return n;
    }
  } return 0; });
  {
    const $tt_m = k;
    switch ($tt_m) {
      case 1: {
        $tt_v1 = $tt_v2($tt_v3, $tt_v4, "one");
        break;
      }
      default: {
        $tt_v1 = $tt_v2($tt_v3, $tt_v4, "other");
        break;
      }
    }
  }
  return $tt_v1;
}
function read(): TResult<number, string> { return Result.Ok(4); }
function apply(n: number, f: (v: V) => number): number { return n + f(V.A(10)); }
function arm(v: V): TResult<number, string> {
  let $tt_v5: number;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v5 = n;
        break;
      }
      case "B": {
        let $tt_v11: number;
        const $tt_v12: typeof apply = (apply);
        const $tt_t1 = read();
        if (!("value" in $tt_t1)) {
          return $tt_t1;
        }
        $tt_v11 = $tt_t1.value;
        $tt_v5 = ($tt_v12($tt_v11, (w) => { {
          const $tt_t2 = w;
          if ($tt_t2.kind === "A") {
            const { n } = $tt_t2;
            return n;
          }
        } return 0; }));
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const s = $tt_v5;
  return Result.Ok(s);
}
const viaArrow = (k: number) => {
  let $tt_v6: number;
  const $tt_v7: typeof show = (show);
  const $tt_v9: (v: V) => number = ((v) => {
    let $tt_v10: number;
    {
      const $tt_m = v;
      switch ($tt_m.kind) {
        case "A": {
          const { n } = $tt_m;
          $tt_v10 = n + k;
          break;
        }
        case "B": {
          $tt_v10 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    return $tt_v10;
  });
  {
    const $tt_m = k;
    switch ($tt_m) {
      case 1: $tt_v6 = 0; break;
      default: $tt_v6 = 1; break;
    }
  }
  return $tt_v7("x", $tt_v9, ($tt_v6 === 0 ? "one" : "other"));
};
console.log(run(1), run(2), JSON.stringify(arm(V.B)), JSON.stringify(arm(V.A(1))), viaArrow(1));
