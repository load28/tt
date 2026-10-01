//// [flowCompositionKeepsTheFirstStepArity.tt] ////
const add = (a: number, b: number) => a + b;
const f = flow |> add |> ((n: number) => n * 2);
const v: number = f(1, 2);

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [flowCompositionKeepsTheFirstStepArity.ts]
import { $tt_fl } from "./tt/runtime.js";
const add = (a: number, b: number) => a + b;
const f = $tt_fl(add, ((n: number) => n * 2));
const v: number = f(1, 2);

export {};
