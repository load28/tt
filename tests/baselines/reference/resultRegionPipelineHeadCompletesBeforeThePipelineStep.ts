//// [resultRegionPipelineHeadCompletesBeforeThePipelineStep.tt] ////
variant R { Ok(value: number), Err(error: string) }
declare const g: () => R; declare const unwrap: (x: R) => number;
const f = (): number => { const n = result {
const value = try g(); return value * 2;
} |> unwrap; return n; };


//// [resultRegionPipelineHeadCompletesBeforeThePipelineStep.ts]
var $tt_expr: <T>(run: () => T) => T = function (run) { return run(); };
type R =
  | { kind: "Ok"; value: number }
  | { kind: "Err"; error: string };
const R = {
  Ok: (value: number): R => ({ kind: "Ok", value }),
  Err: (error: string): R => ({ kind: "Err", error }),
};
declare const g: () => R; declare const unwrap: (x: R) => number;
const f = (): number => { let $tt_v0: number;
do {
  const $tt_v2: R = ($tt_expr(() => {
    const $tt_t0 = g();
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    const value = $tt_t0.value; { return { kind: "Ok" as const, value: value * 2 }; }
    }));
  $tt_v0 = unwrap($tt_v2);
  break;
} while (false);
const n = $tt_v0; return n; };
