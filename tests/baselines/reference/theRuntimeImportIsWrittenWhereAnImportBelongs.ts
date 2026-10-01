//// [theRuntimeImportIsWrittenWhereAnImportBelongs.tt] ////
declare function f(n: number): number;
declare function g(n: number): number;
export const a = f(4) |> g |> .toFixed(1);


//// [theRuntimeImportIsWrittenWhereAnImportBelongs.ts]
declare function f(n: number): number;
declare function g(n: number): number;
export const a = (($tt_v, $tt_f) => $tt_f($tt_v))(f(4), g).toFixed(1);
