//// [aTypeAssertionAfterAPipelineStepAppliesToTheWholePipeline.tt] ////
declare const f: (n: number) => string;
const d = 1 |> String as string;
const e = 1 |> String satisfies string;
const g = 1 |> ((x: number) => x) as number;
const h = 1 |> f as string | undefined;
const k = flow |> f as (n: number) => string;
const m = [1 |> f as string |> .length satisfies number |> String, 2];


//// [aTypeAssertionAfterAPipelineStepAppliesToTheWholePipeline.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
declare const f: (n: number) => string;
const d = String(1) as string;
const e = String(1) satisfies string;
const g = ((x: number) => x)(1) as number;
const h = f(1) as string | undefined;
const k = f as (n: number) => string;
const m = [$tt_ap((f(1) as string).length satisfies number, String), 2];
