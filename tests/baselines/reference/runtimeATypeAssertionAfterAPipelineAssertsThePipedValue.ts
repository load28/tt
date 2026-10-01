//// [runtimeATypeAssertionAfterAPipelineAssertsThePipedValue.tt] ////

const maybe = undefined as ((n: number) => string) | undefined;
const twice = (n: number) => n * 2;
const d = 1 |> String as string;
const e = 2 |> twice satisfies number;
const g = 3 |> ((x: number) => x + 1) as number;
const h = 4 |> twice as number |> String satisfies string |> .length;
const k = flow |> twice as (n: number) => number;
const l = 5 |> (maybe ?? String);
const m = 6 |> maybe ?? String;
console.log(d, e, g, h, k(7), l, m);

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [runtimeATypeAssertionAfterAPipelineAssertsThePipedValue.ts]
import { $tt_ap } from "./tt/runtime.js";

const maybe = undefined as ((n: number) => string) | undefined;
const twice = (n: number) => n * 2;
const d = String(1) as string;
const e = twice(2) satisfies number;
const g = ((x: number) => x + 1)(3) as number;
const h = ($tt_ap(twice(4) as number, String) satisfies string).length;
const k = twice as (n: number) => number;
const l = (maybe ?? String)(5);
const m = (maybe ?? String)(6);
console.log(d, e, g, h, k(7), l, m);

export {};
