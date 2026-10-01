//// [theRuntimeImportNeverDisplacesADirectiveOrAShebang3.tt] ////
declare const b: string;
"a" + b;
declare function f(n: number): number;
export const a = f(4) |> f |> .toFixed(1);

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [theRuntimeImportNeverDisplacesADirectiveOrAShebang3.ts]
import { $tt_ap } from "./tt/runtime.js";
declare const b: string;
"a" + b;
declare function f(n: number): number;
export const a = $tt_ap(f(4), f).toFixed(1);
