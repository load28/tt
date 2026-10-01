//// [theRuntimeImportNeverDisplacesADirectiveOrAShebang1.tt] ////
"use client";
declare function f(n: number): number;
declare function g(n: number): number;
export const a = f(4) |> g |> .toFixed(1);

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [theRuntimeImportNeverDisplacesADirectiveOrAShebang1.ts]
"use client";
import { $tt_ap } from "./tt/runtime.js";
declare function f(n: number): number;
declare function g(n: number): number;
export const a = $tt_ap(f(4), g).toFixed(1);
