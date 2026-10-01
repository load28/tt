//// [theRuntimeImportNeverDisplacesADirectiveOrAShebang1.tt] ////
"use client";
declare function f(n: number): number;
declare function g(n: number): number;
export const a = f(4) |> g |> .toFixed(1);


//// [theRuntimeImportNeverDisplacesADirectiveOrAShebang1.ts]
"use client";
declare function f(n: number): number;
declare function g(n: number): number;
export const a = (($tt_v, $tt_f) => $tt_f($tt_v))(f(4), g).toFixed(1);
