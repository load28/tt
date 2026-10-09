//// [aFlowPipelineHeadBeforeStructuredStepsIsEmitted.tt] ////
import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
variant V { A(n: number), B }
const r = (n: number): TResult<number, string> => (n > 0 ? Result.Ok(n) : Result.Err("none"));
const double = (n: number) => n * 2;
const pick = (x: number): V => (x > 0 ? V.A(x) : V.B);
const chained = (flow |> double |> String)
  |> (f => result { const z = try r(f(4).length); return z; })
  |> (q => (x: number) => { const A(n: m) = match (x) { 1 => pick(q.kind === "Ok" ? q.value : 0), _ => V.B } else { return 0; }; return m; });
console.log(chained(1), chained(2));

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts
//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [aFlowPipelineHeadBeforeStructuredStepsIsEmitted.ts]
import { $tt_fl } from "./tt/runtime.js";
import * as Result from "./tt/result.js";
import type { TResult } from "./tt/index.js";
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const r = (n: number): TResult<number, string> => (n > 0 ? Result.Ok(n) : Result.Err("none"));
const double = (n: number) => n * 2;
const pick = (x: number): V => (x > 0 ? V.A(x) : V.B);
let $tt_v0: (x: number) => number;
do {
  const $tt_v4: (n: number) => string = ((($tt_g, $tt_f) => $tt_fl($tt_g, ($tt_v) => $tt_f($tt_v)))(double, String));
  const $tt_v5: Result.TErr<string> | {
    kind: "Ok";
    value: number;
} = ((f => {
    let $tt_v1: Result.TErr<string> | {
    kind: "Ok";
    value: number;
};
    $tt_v1: {
      const $tt_t0 = r(f(4).length);
      if (!("value" in $tt_t0)) {
        $tt_v1 = $tt_t0;
        break $tt_v1;
      }
      const z = $tt_t0.value; { $tt_v1 = { kind: "Ok" as const, value: z }; break $tt_v1; }
    }
    return $tt_v1;
  }))($tt_v4);
  $tt_v0 = ((q => (x: number) => { let $tt_t1; {
    const $tt_m = x;
    switch ($tt_m) {
      case 1: {
        $tt_t1 = pick(q.kind === "Ok" ? q.value : 0);
        break;
      }
      default: {
        $tt_t1 = V.B;
        break;
      }
    }
  }
  if ($tt_t1.kind !== "A") {
    return 0;
  }
  const { n: m } = $tt_t1; return m; }))($tt_v5);
  break;
} while (false);
const chained = $tt_v0;
console.log(chained(1), chained(2));
