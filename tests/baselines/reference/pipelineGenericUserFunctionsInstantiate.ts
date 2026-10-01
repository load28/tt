//// [pipelineGenericUserFunctionsInstantiate.tt] ////
const wrap = <T,>(v: T): T[] => [v];
const arr: number[][] = 3 |> wrap |> wrap;

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [pipelineGenericUserFunctionsInstantiate.ts]
import { $tt_ap } from "./tt/runtime.js";
const wrap = <T,>(v: T): T[] => [v];
const arr: number[][] = $tt_ap(wrap(3), wrap);

export {};
